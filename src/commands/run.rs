use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    cli::{parse_cli_args, prompt_user},
    config::{load_mis_config, plugins::load_plugin_manifest},
    constants::PLUGIN_MANIFEST_FILE,
    integrations::deno::{cache_deno_dependencies, install_deno, is_deno_installed},
    models::{ExecutionContext, PluginManifest, PluginMeta},
    security::build_plugin_permissions,
    utils::find_project_root,
    validation::validate_plugin_args,
};
use anyhow::{Context, Result};

pub fn run_cmd(
    plugin_name: String,
    command_name: &str,
    dry_run: bool,
    plugin_raw_args: HashMap<String, String>,
) -> Result<()> {
    let plugin_path = validate_plugin_exists(&plugin_name)?;
    let manifest_path = plugin_path.join(PLUGIN_MANIFEST_FILE);
    let plugin_manifest = load_plugin_manifest(&manifest_path)?;

    if !is_deno_installed() {
        let should_install = prompt_user("Deno is not installed. Would you like to install it?")?;
        if !should_install {
            anyhow::bail!("Deno is required to run plugins. Please install it and try again.");
        }

        // Install Deno
        install_deno()?; // or prompt/abort if you want confirmation
    }

    // Parse raw arguments with improved logic that preserves spaces and handles empty values
    let mut raw_args = Vec::new();
    for (k, v) in plugin_raw_args {
        raw_args.push(format!("--{}", k));
        if !v.is_empty() {
            raw_args.push(v);
        }
    }

    let parsed_args = parse_cli_args(&raw_args);

    // Get the command definition for validation
    let command = plugin_manifest
        .commands
        .get(command_name)
        .with_context(|| {
            format!(
                "Command '{}' not found in plugin '{}'",
                command_name, plugin_name
            )
        })?;

    // Validate arguments against the plugin manifest
    let validated_args = validate_plugin_args(
        &parsed_args,
        command.args.as_ref(),
        &plugin_name,
        command_name,
    )?;

    // Convert validated args to the format expected by ExecutionContext
    let mut plugin_args: serde_json::Map<String, serde_json::Value> = validated_args
        .into_iter()
        .map(|(k, v)| {
            let value = match v.as_str() {
                "true" => serde_json::Value::Bool(true),
                "false" => serde_json::Value::Bool(false),
                _ => serde_json::Value::String(v),
            };
            (k, value)
        })
        .collect();

    if dry_run {
        plugin_args.insert("dry_run".to_string(), serde_json::Value::Bool(true));
    }

    let project_root = std::env::current_dir()?.to_string_lossy().to_string();
    // let env_vars = std::env::vars().collect::<HashMap<_, _>>();
    let meta = PluginMeta {
        name: plugin_name.clone(),
        description: plugin_manifest.plugin.description.clone(),
        version: "todo".to_string(), // figure out how to get this
    };

    let (mis_config, _, __) = load_mis_config()?;

    let plugin_args_toml: HashMap<String, toml::Value> = plugin_args
        .into_iter()
        .map(|(k, v)| (k, json_to_toml(v)))
        .collect();

    let ctx = ExecutionContext::from_parts(
        plugin_args_toml,
        plugin_manifest.user_config.clone(),
        mis_config.project_variables,
        project_root,
        meta,
        dry_run,
    )?;

    execute_plugin(&plugin_path, &command.script, &ctx, &plugin_manifest)?;

    Ok(())
}

fn json_to_toml(value: serde_json::Value) -> toml::Value {
    toml::Value::try_from(value).expect("Failed to convert plugin arg from JSON to TOML")
}

fn validate_plugin_exists(plugin_name: &str) -> Result<PathBuf> {
    let root = find_project_root().ok_or_else(|| anyhow::anyhow!("Failed to find project root"))?;

    if !root.exists() {
        anyhow::bail!(
            "🛑 You're not inside a Make It So project.\n\
             → Make sure you're in the project root (where .makeitso/ lives).\n\
             → If you haven't set it up yet, run `mis init`."
        );
    }

    let plugin_path = root.join(".makeitso/plugins").join(plugin_name);
    println!("Plugin path: {}", plugin_path.display());

    if !plugin_path.exists() {
        anyhow::bail!(
            "🛑 Plugin '{}' not found in .makeitso/plugins.\n\
             → Did you run `mis create plugin {}`?",
            plugin_name,
            plugin_name
        );
    }

    let manifest_path = plugin_path.join("plugin.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "🛑 plugin.toml not found for plugin '{}'.\n\
             → Expected to find: {}\n\
             → Did something delete it?",
            plugin_name,
            manifest_path.display()
        );
    }

    Ok(plugin_path)
}

pub fn execute_plugin(
    dir: &PathBuf,
    script_file_name: &str,
    ctx: &ExecutionContext,
    plugin_config: &PluginManifest,
) -> Result<()> {
    // Cache any [deno_dependencies] first
    cache_deno_dependencies(&plugin_config.deno_dependencies)?;

    // Serialize the context into JSON to pass to the plugin
    let json = serde_json::to_string_pretty(ctx)?;

    let path_and_file = dir.join(script_file_name);

    // Build secure permissions for the plugin
    let project_root = std::env::current_dir()?;
    let permissions = build_plugin_permissions(&project_root)?;

    // Build Deno command arguments
    let mut deno_args = vec!["run".to_string()];
    deno_args.extend(permissions.to_deno_args());
    deno_args.push(path_and_file.to_string_lossy().to_string());

    // Spawn the plugin with Deno using secure permissions
    let mut child = Command::new("deno")
        .args(&deno_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to run plugin: {}", script_file_name))?;

    // Pipe context JSON into plugin's stdin
    child
        .stdin
        .as_mut()
        .context("Failed to open stdin for plugin")?
        .write_all(json.as_bytes())?;

    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Plugin exited with non-zero status"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ArgDefinition, ArgType, CommandArgs, PluginCommand, PluginManifest, PluginMeta,
    };
    use std::collections::HashMap;

    fn create_test_plugin_manifest() -> PluginManifest {
        let mut commands = HashMap::new();

        let mut required = HashMap::new();
        required.insert(
            "environment".to_string(),
            ArgDefinition {
                description: "Target environment".to_string(),
                arg_type: ArgType::String,
                default_value: None,
            },
        );

        let mut optional = HashMap::new();
        optional.insert(
            "verbose".to_string(),
            ArgDefinition {
                description: "Enable verbose output".to_string(),
                arg_type: ArgType::Boolean,
                default_value: Some("false".to_string()),
            },
        );
        optional.insert(
            "count".to_string(),
            ArgDefinition {
                description: "Number of items".to_string(),
                arg_type: ArgType::Integer,
                default_value: Some("1".to_string()),
            },
        );

        commands.insert(
            "deploy".to_string(),
            PluginCommand {
                script: "./deploy.ts".to_string(),
                description: Some("Deploy application".to_string()),
                instructions: None,
                args: Some(CommandArgs { required, optional }),
            },
        );

        PluginManifest {
            plugin: PluginMeta {
                name: "test-plugin".to_string(),
                description: Some("Test plugin".to_string()),
                version: "1.0.0".to_string(),
            },
            commands,
            deno_dependencies: HashMap::new(),
            user_config: None,
        }
    }

    #[test]
    fn test_argument_reconstruction_basic() {
        // Test the complex argument reconstruction logic in run_cmd
        let plugin_raw_args: HashMap<String, String> = [
            ("environment".to_string(), "staging".to_string()),
            ("verbose".to_string(), "true".to_string()),
        ]
        .iter()
        .cloned()
        .collect();

        // Simulate the reconstruction logic from run_cmd
        let raw_args: Vec<String> = plugin_raw_args
            .into_iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    format!("--{}", k)
                } else {
                    vec![format!("--{}", k), v].join(" ")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let parsed_args = parse_cli_args(&raw_args);

        assert_eq!(parsed_args.get("environment"), Some(&"staging".to_string()));
        assert_eq!(parsed_args.get("verbose"), Some(&"true".to_string()));
    }

    #[test]
    fn test_argument_reconstruction_with_spaces() {
        // Test edge case: values with spaces
        let plugin_raw_args: HashMap<String, String> = [
            ("message".to_string(), "hello world".to_string()),
            ("path".to_string(), "/path/with spaces/file.txt".to_string()),
        ]
        .iter()
        .cloned()
        .collect();

        // This demonstrates the bug in the current reconstruction logic
        let raw_args: Vec<String> = plugin_raw_args
            .into_iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    format!("--{}", k)
                } else {
                    vec![format!("--{}", k), v].join(" ")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let parsed_args = parse_cli_args(&raw_args);

        // This will fail because spaces break the reconstruction
        // "hello world" becomes ["hello", "world"] after split_whitespace
        assert_ne!(parsed_args.get("message"), Some(&"hello world".to_string()));
        assert_eq!(parsed_args.get("message"), Some(&"hello".to_string()));
    }

    #[test]
    fn test_argument_reconstruction_empty_values() {
        // Test edge case: empty values
        let plugin_raw_args: HashMap<String, String> = [
            ("flag".to_string(), "".to_string()),
            ("name".to_string(), "test".to_string()),
        ]
        .iter()
        .cloned()
        .collect();

        let raw_args: Vec<String> = plugin_raw_args
            .into_iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    format!("--{}", k)
                } else {
                    vec![format!("--{}", k), v].join(" ")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let parsed_args = parse_cli_args(&raw_args);

        // Now correctly handles empty values as boolean flags
        assert_eq!(parsed_args.get("name"), Some(&"test".to_string()));
        assert_eq!(parsed_args.get("flag"), Some(&"true".to_string())); // Now correctly handled
    }

    #[test]
    fn test_argument_reconstruction_special_characters() {
        // Test edge case: special characters in values
        let plugin_raw_args: HashMap<String, String> = [
            (
                "url".to_string(),
                "https://example.com/path?param=value&other=123".to_string(),
            ),
            ("regex".to_string(), "^[a-zA-Z0-9]+$".to_string()),
        ]
        .iter()
        .cloned()
        .collect();

        let raw_args: Vec<String> = plugin_raw_args
            .into_iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    format!("--{}", k)
                } else {
                    vec![format!("--{}", k), v].join(" ")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let parsed_args = parse_cli_args(&raw_args);

        assert_eq!(
            parsed_args.get("url"),
            Some(&"https://example.com/path?param=value&other=123".to_string())
        );
        assert_eq!(
            parsed_args.get("regex"),
            Some(&"^[a-zA-Z0-9]+$".to_string())
        );
    }

    #[test]
    fn test_improved_argument_reconstruction() {
        // Test the better approach to argument reconstruction
        let plugin_raw_args: HashMap<String, String> = [
            ("message".to_string(), "hello world".to_string()),
            ("flag".to_string(), "".to_string()),
            ("count".to_string(), "5".to_string()),
        ]
        .iter()
        .cloned()
        .collect();

        // Improved reconstruction that preserves spaces and handles empty values
        let mut raw_args = Vec::new();
        for (k, v) in plugin_raw_args {
            raw_args.push(format!("--{}", k));
            if !v.is_empty() {
                raw_args.push(v);
            }
        }

        let parsed_args = parse_cli_args(&raw_args);

        assert_eq!(parsed_args.get("message"), Some(&"hello world".to_string()));
        assert_eq!(parsed_args.get("flag"), Some(&"true".to_string()));
        assert_eq!(parsed_args.get("count"), Some(&"5".to_string()));
    }

    #[test]
    fn test_validation_with_edge_case_arguments() {
        let manifest = create_test_plugin_manifest();
        let command = manifest.commands.get("deploy").unwrap();

        // Test with arguments that have special characters
        let mut provided_args = HashMap::new();
        provided_args.insert("environment".to_string(), "staging-us-west-2".to_string());
        provided_args.insert("verbose".to_string(), "true".to_string());

        let result = validate_plugin_args(
            &provided_args,
            command.args.as_ref(),
            "test-plugin",
            "deploy",
        );

        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(
            validated.get("environment"),
            Some(&"staging-us-west-2".to_string())
        );
        assert_eq!(validated.get("verbose"), Some(&"true".to_string()));
        assert_eq!(validated.get("count"), Some(&"1".to_string())); // default value
    }

    #[test]
    fn test_validation_with_boolean_edge_cases() {
        let manifest = create_test_plugin_manifest();
        let command = manifest.commands.get("deploy").unwrap();

        // Test various boolean representations
        let test_cases = vec![
            ("true", "true"),
            ("false", "false"),
            ("1", "true"),
            ("0", "false"),
            ("yes", "true"),
            ("no", "false"),
            ("on", "true"),
            ("off", "false"),
        ];

        for (input, expected) in test_cases {
            let mut provided_args = HashMap::new();
            provided_args.insert("environment".to_string(), "test".to_string());
            provided_args.insert("verbose".to_string(), input.to_string());

            let result = validate_plugin_args(
                &provided_args,
                command.args.as_ref(),
                "test-plugin",
                "deploy",
            );

            assert!(result.is_ok(), "Failed for input: {}", input);
            let validated = result.unwrap();
            assert_eq!(
                validated.get("verbose"),
                Some(&expected.to_string()),
                "Failed for input: {}, expected: {}",
                input,
                expected
            );
        }
    }

    #[test]
    fn test_validation_with_invalid_boolean() {
        let manifest = create_test_plugin_manifest();
        let command = manifest.commands.get("deploy").unwrap();

        let mut provided_args = HashMap::new();
        provided_args.insert("environment".to_string(), "test".to_string());
        provided_args.insert("verbose".to_string(), "invalid-boolean".to_string());

        let result = validate_plugin_args(
            &provided_args,
            command.args.as_ref(),
            "test-plugin",
            "deploy",
        );

        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("expected boolean value"));
    }

    #[test]
    fn test_validation_with_integer_edge_cases() {
        let manifest = create_test_plugin_manifest();
        let command = manifest.commands.get("deploy").unwrap();

        let test_cases = vec![
            ("0", true),
            ("42", true),
            ("-5", true),
            ("999999", true),
            ("3.14", false), // float should fail for integer
            ("abc", false),  // string should fail for integer
            ("", false),     // empty should fail for integer
        ];

        for (input, should_succeed) in test_cases {
            let mut provided_args = HashMap::new();
            provided_args.insert("environment".to_string(), "test".to_string());
            provided_args.insert("count".to_string(), input.to_string());

            let result = validate_plugin_args(
                &provided_args,
                command.args.as_ref(),
                "test-plugin",
                "deploy",
            );

            if should_succeed {
                assert!(result.is_ok(), "Should succeed for input: {}", input);
            } else {
                assert!(result.is_err(), "Should fail for input: {}", input);
            }
        }
    }

    #[test]
    fn test_full_pipeline_integration() {
        // Test the complete pipeline: raw args -> reconstruction -> parsing -> validation
        let manifest = create_test_plugin_manifest();
        let command = manifest.commands.get("deploy").unwrap();

        // Simulate what would come from the CLI
        let plugin_raw_args: HashMap<String, String> = [
            ("environment".to_string(), "staging-us-west-2".to_string()),
            ("verbose".to_string(), "".to_string()), // Empty value = boolean flag
            ("count".to_string(), "5".to_string()),
        ]
        .iter()
        .cloned()
        .collect();

        // Use the improved reconstruction logic
        let mut raw_args = Vec::new();
        for (k, v) in plugin_raw_args {
            raw_args.push(format!("--{}", k));
            if !v.is_empty() {
                raw_args.push(v);
            }
        }

        // Parse with the unified parser that handles all edge cases
        let parsed_args = parse_cli_args(&raw_args);

        // Validate
        let result =
            validate_plugin_args(&parsed_args, command.args.as_ref(), "test-plugin", "deploy");

        assert!(result.is_ok());
        let validated = result.unwrap();

        // Check all arguments are correctly processed
        assert_eq!(
            validated.get("environment"),
            Some(&"staging-us-west-2".to_string())
        );
        assert_eq!(validated.get("verbose"), Some(&"true".to_string())); // Empty value became boolean
        assert_eq!(validated.get("count"), Some(&"5".to_string()));
    }

    #[test]
    fn test_complex_real_world_scenario() {
        // Test a complex real-world scenario with mixed argument types
        let manifest = create_test_plugin_manifest();
        let command = manifest.commands.get("deploy").unwrap();

        // Simulate complex CLI input with various edge cases
        let plugin_raw_args: HashMap<String, String> = [
            (
                "environment".to_string(),
                "production-eu-central-1".to_string(),
            ),
            ("verbose".to_string(), "".to_string()), // Boolean flag
            ("count".to_string(), "10".to_string()),
        ]
        .iter()
        .cloned()
        .collect();

        // Test the improved pipeline
        let mut raw_args = Vec::new();
        for (k, v) in plugin_raw_args {
            raw_args.push(format!("--{}", k));
            if !v.is_empty() {
                raw_args.push(v);
            }
        }

        let parsed_args = parse_cli_args(&raw_args);
        let validated =
            validate_plugin_args(&parsed_args, command.args.as_ref(), "test-plugin", "deploy")
                .unwrap();

        // Verify all edge cases are handled correctly
        assert_eq!(validated.len(), 3); // All 3 arguments present
        assert_eq!(
            validated.get("environment"),
            Some(&"production-eu-central-1".to_string())
        );
        assert_eq!(validated.get("verbose"), Some(&"true".to_string()));
        assert_eq!(validated.get("count"), Some(&"10".to_string()));
    }
}
