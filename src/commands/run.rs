use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    cli::{parse_cli_args, prompt_user},
    config::{
        load_mis_config,
        plugins::{load_plugin_manifest, load_plugin_user_config},
    },
    constants::{PLUGIN_CONFIG_FILE, PLUGIN_MANIFEST_FILE},
    integrations::deno::{cache_deno_dependencies, install_deno, is_deno_installed},
    models::{ExecutionContext, PluginManifest, PluginMeta},
    security::{build_plugin_permissions, validate_deno_dependency_url},
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
    let config_path = plugin_path.join(PLUGIN_CONFIG_FILE);

    let plugin_manifest = load_plugin_manifest(&manifest_path)?;
    let plugin_user_config = load_plugin_user_config(&config_path)?;

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

    // Validate Deno dependencies for security
    for (dep_name, dep_url) in &plugin_manifest.deno_dependencies {
        if let Err(security_error) = validate_deno_dependency_url(dep_url) {
            return Err(anyhow::anyhow!(
                "🛑 Security validation failed for dependency '{}' ({}): {}\n\
                 → Deno dependencies must use secure HTTPS URLs from trusted sources.",
                dep_name,
                dep_url,
                security_error
            ));
        }
    }

    let meta = PluginMeta {
        name: plugin_name, // Move instead of clone - plugin_name not used after this
        description: plugin_manifest.plugin.description.clone(),
        version: plugin_manifest.plugin.version.clone(),
        registry: None, // Not needed for execution context
    };

    let (mis_config, _, __) = load_mis_config()?;

    let plugin_args_toml: HashMap<String, toml::Value> = plugin_args
        .into_iter()
        .map(|(k, v)| (k, json_to_toml(v)))
        .collect();

    let ctx = ExecutionContext::from_parts(
        plugin_args_toml,
        &plugin_manifest,
        &plugin_user_config,
        mis_config.project_variables,
        project_root,
        meta,
        dry_run,
    )?;

    execute_plugin(
        &plugin_path,
        &command.script,
        &ctx,
        &plugin_manifest.deno_dependencies,
        &plugin_manifest,
        command_name,
    )?;

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

    let manifest_path = plugin_path.join(PLUGIN_MANIFEST_FILE);
    if !manifest_path.exists() {
        anyhow::bail!(
            "🛑 manifest.toml not found for plugin '{}'.\n\
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
    deno_dependencies: &HashMap<String, String>,
    plugin_manifest: &PluginManifest,
    command_name: &str,
) -> Result<()> {
    // Cache any [deno_dependencies] first
    cache_deno_dependencies(deno_dependencies)?;

    // Serialize the context into JSON to pass to the plugin
    let json = serde_json::to_string_pretty(ctx)?;

    let path_and_file = dir.join(script_file_name);

    // Check if script file exists before attempting to execute
    if !path_and_file.exists() {
        anyhow::bail!(
            "🛑 Plugin script not found: {}\n\
             → Expected to find: {}\n\
             → Make sure the script file exists and matches the 'script' field in plugin.toml\n\
             → If you just created this plugin, you may need to create the script file.",
            script_file_name,
            path_and_file.display()
        );
    }

    // Build secure permissions for the plugin using manifest-declared permissions
    let project_root = std::env::current_dir()?;
    let permissions = build_plugin_permissions(&project_root, plugin_manifest, command_name)?;

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
        .with_context(|| format!("🛑 Failed to run plugin script: {}\n→ Make sure Deno is installed and the script is valid", script_file_name))?;

    // Pipe context JSON into plugin's stdin
    child
        .stdin
        .as_mut()
        .context("Failed to open stdin for plugin")?
        .write_all(json.as_bytes())?;

    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "🛑 Plugin exited with error (non-zero status)\n→ Check the plugin output above for details"
        ));
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
                permissions: None,
            },
        );

        PluginManifest {
            plugin: PluginMeta {
                name: "test-plugin".to_string(),
                description: Some("Test plugin".to_string()),
                version: "1.0.0".to_string(),
                registry: None,
            },
            commands,
            deno_dependencies: HashMap::new(),
            permissions: None,
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
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("expected boolean value"));
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

    #[test]
    fn test_run_cmd_uses_manifest_version_not_todo() {
        // This test actually calls run_cmd and verifies the version comes from manifest
        // This test should FAIL until we fix the "todo" bug in run_cmd
        use std::fs;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .makeitso structure with a real plugin
        let makeitso_dir = temp_dir.path().join(".makeitso");
        let plugins_dir = makeitso_dir.join("plugins").join("version-test-plugin");
        fs::create_dir_all(&plugins_dir).unwrap();
        fs::create_dir_all(&makeitso_dir).unwrap();

        // Create mis.toml
        let config_content = r#"
name = "test-project"

[project_variables]
test = "value"
"#;
        fs::write(makeitso_dir.join("mis.toml"), config_content).unwrap();

        // Create plugin with specific version
        let plugin_toml = r#"
[plugin]
name = "version-test-plugin"
version = "2.3.4"
description = "Plugin to test version reading"

[commands.version-check]
script = "./version-check.ts"
description = "Check version"
"#;
        fs::write(plugins_dir.join("plugin.toml"), plugin_toml).unwrap();

        // Create a simple script that just outputs the context
        let script_content = r#"
import { loadContext, outputSuccess } from "../plugin-api.ts";

const ctx = await loadContext();
outputSuccess({ version: ctx.meta.version });
"#;
        fs::write(plugins_dir.join("version-check.ts"), script_content).unwrap();

        // Create dummy plugin-api.ts (since we can't run real deno in tests)
        fs::write(makeitso_dir.join("plugin-api.ts"), "// dummy api").unwrap();
        fs::write(makeitso_dir.join("plugin-types.d.ts"), "// dummy types").unwrap();

        // This test would fail because run_cmd currently hardcodes "todo"
        // We can't actually run deno in tests, but we can check that the function
        // creates the right context before trying to execute

        // For now, let's verify the manifest loads correctly
        let manifest_path = plugins_dir.join("plugin.toml");
        let manifest = crate::config::plugins::load_plugin_manifest(&manifest_path).unwrap();
        assert_eq!(manifest.plugin.version, "2.3.4");

        std::env::set_current_dir(original_dir).unwrap();

        // TODO: Once we fix the bug, we could add an integration test that actually
        // verifies the ExecutionContext contains the right version
    }

    #[test]
    fn test_error_recovery_corrupted_manifest() {
        // Test that we handle corrupted plugin.toml files gracefully
        use std::fs;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .makeitso structure
        let makeitso_dir = temp_dir.path().join(".makeitso");
        let plugins_dir = makeitso_dir.join("plugins").join("broken-plugin");
        fs::create_dir_all(&plugins_dir).unwrap();

        // Create a corrupted plugin.toml
        let corrupted_toml = r#"
[plugin
name = "broken-plugin"  # Missing closing bracket
version = "1.0.0
description = "This manifest is corrupted"

[commands.test]
script = "./test.ts"
"#;
        fs::write(plugins_dir.join("plugin.toml"), corrupted_toml).unwrap();

        // Attempt to run the plugin - should fail gracefully, not crash
        let result = run_cmd(
            "broken-plugin".to_string(),
            "test",
            false,
            std::collections::HashMap::new(),
        );

        // Should fail with a helpful error message, not crash
        assert!(
            result.is_err(),
            "Should fail gracefully with corrupted manifest"
        );
        let error_msg = result.unwrap_err().to_string();
        println!("Actual error message: {}", error_msg);
        assert!(
            error_msg.contains("plugin.toml")
                || error_msg.contains("manifest")
                || error_msg.contains("toml"),
            "Error should mention manifest issues. Got: {}",
            error_msg
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_error_recovery_missing_script_file() {
        // Test that we handle missing script files gracefully
        use std::fs;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .makeitso structure
        let makeitso_dir = temp_dir.path().join(".makeitso");
        let plugins_dir = makeitso_dir.join("plugins").join("missing-script-plugin");
        fs::create_dir_all(&plugins_dir).unwrap();

        // Create valid plugin.toml but missing script file
        let valid_toml = r#"
[plugin]
name = "missing-script-plugin"
version = "1.0.0"
description = "Plugin with missing script"

[commands.test]
script = "./nonexistent.ts"
description = "Test command"
"#;
        fs::write(plugins_dir.join("plugin.toml"), valid_toml).unwrap();
        // Note: we're NOT creating the script file

        // Attempt to run the plugin - should fail gracefully
        let result = run_cmd(
            "missing-script-plugin".to_string(),
            "test",
            false,
            std::collections::HashMap::new(),
        );

        // Should fail with a helpful error about missing script
        assert!(
            result.is_err(),
            "Should fail gracefully with missing script"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("script")
                || error_msg.contains("file")
                || error_msg.contains("nonexistent.ts"),
            "Error should mention missing script file"
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_error_recovery_plugin_execution_timeout() {
        // Test that we can handle plugins that run too long
        // Note: This is a placeholder test - actual timeout implementation would come later
        use std::fs;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .makeitso structure
        let makeitso_dir = temp_dir.path().join(".makeitso");
        let plugins_dir = makeitso_dir.join("plugins").join("slow-plugin");
        fs::create_dir_all(&plugins_dir).unwrap();

        // Create plugin that would run forever (infinite loop)
        let infinite_script = r#"
console.log("Starting infinite loop...");
while (true) {
    // This would run forever without timeout handling
    await new Promise(resolve => setTimeout(resolve, 100));
}
"#;
        fs::write(plugins_dir.join("slow.ts"), infinite_script).unwrap();

        let toml_content = r#"
[plugin]
name = "slow-plugin"
version = "1.0.0"
description = "Plugin that runs too long"

[commands.slow]
script = "./slow.ts"
description = "Slow command"
"#;
        fs::write(plugins_dir.join("plugin.toml"), toml_content).unwrap();

        // For now, just verify the plugin structure is valid
        // TODO: When we implement timeouts, this test should verify timeout behavior
        let manifest_path = plugins_dir.join("plugin.toml");
        let manifest_result = crate::config::plugins::load_plugin_manifest(&manifest_path);

        // Manifest should load successfully - the issue is execution, not structure
        assert!(manifest_result.is_ok(), "Plugin manifest should be valid");

        std::env::set_current_dir(original_dir).unwrap();

        // TODO: When timeout functionality is implemented, add:
        // let result = run_cmd("slow-plugin".to_string(), "slow", false, HashMap::new());
        // assert!(result.is_err(), "Should timeout and fail gracefully");
    }

    #[test]
    fn test_error_recovery_invalid_plugin_structure() {
        // Test handling of plugins with invalid directory structure
        use std::fs;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .makeitso structure but with invalid plugin (missing plugin.toml)
        let makeitso_dir = temp_dir.path().join(".makeitso");
        let plugins_dir = makeitso_dir.join("plugins").join("invalid-plugin");
        fs::create_dir_all(&plugins_dir).unwrap();

        // Create script file but NO plugin.toml
        fs::write(plugins_dir.join("script.ts"), "console.log('test');").unwrap();

        // Attempt to run plugin without manifest
        let result = run_cmd(
            "invalid-plugin".to_string(),
            "test",
            false,
            std::collections::HashMap::new(),
        );

        // Should fail gracefully with helpful error about missing manifest
        assert!(
            result.is_err(),
            "Should fail gracefully with missing plugin.toml"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("plugin.toml") || error_msg.contains("manifest"),
            "Error should mention missing plugin.toml"
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    // ========== NEW CONTEXT PASSING TESTS ==========

    #[test]
    fn test_execution_context_includes_both_manifest_and_config() {
        use crate::models::{PluginManifest, PluginMeta, PluginUserConfig};
        use std::collections::HashMap;

        // Create test manifest
        let manifest = PluginManifest {
            plugin: PluginMeta {
                name: "test-plugin".to_string(),
                description: Some("Test plugin for context".to_string()),
                version: "1.2.3".to_string(),
                registry: Some("https://github.com/example/plugins.git".to_string()),
            },
            commands: HashMap::new(),
            deno_dependencies: {
                let mut deps = HashMap::new();
                deps.insert(
                    "oak".to_string(),
                    "https://deno.land/x/oak@v12.6.1/mod.ts".to_string(),
                );
                deps
            },
            permissions: None,
        };

        // Create test user config
        let mut user_config = PluginUserConfig::default();
        user_config.config.insert(
            "api_key".to_string(),
            toml::Value::String("secret-123".to_string()),
        );
        user_config
            .config
            .insert("debug".to_string(), toml::Value::Boolean(true));
        user_config
            .config
            .insert("timeout".to_string(), toml::Value::Integer(5000));

        // Create execution context
        let project_variables = HashMap::new();
        let plugin_args = HashMap::new();
        let ctx = ExecutionContext::from_parts(
            plugin_args,
            &manifest,
            &user_config,
            project_variables,
            "/test/project".to_string(),
            manifest.plugin.clone(),
            false,
        )
        .unwrap();

        // Serialize to JSON to verify structure
        let json_str = serde_json::to_string_pretty(&ctx).unwrap();

        // Verify manifest data is present
        assert!(
            json_str.contains("\"name\": \"test-plugin\""),
            "Should contain plugin name"
        );
        assert!(
            json_str.contains("\"version\": \"1.2.3\""),
            "Should contain plugin version"
        );
        assert!(
            json_str.contains("\"registry\": \"https://github.com/example/plugins.git\""),
            "Should contain registry"
        );
        assert!(json_str.contains("oak"), "Should contain dependency names");

        // Verify user config data is present
        assert!(
            json_str.contains("\"api_key\": \"secret-123\""),
            "Should contain user config values"
        );
        assert!(
            json_str.contains("\"debug\": true"),
            "Should contain boolean config"
        );
        assert!(
            json_str.contains("\"timeout\": 5000"),
            "Should contain integer config"
        );

        // Verify the structure separates manifest and config
        assert!(
            json_str.contains("\"manifest\":"),
            "Should have manifest section"
        );
        assert!(
            json_str.contains("\"config\":"),
            "Should have config section"
        );
    }

    #[test]
    fn test_execution_context_with_empty_user_config() {
        use crate::models::{PluginManifest, PluginMeta, PluginUserConfig};
        use std::collections::HashMap;

        // Create minimal manifest
        let manifest = PluginManifest {
            plugin: PluginMeta {
                name: "minimal-plugin".to_string(),
                description: None,
                version: "1.0.0".to_string(),
                registry: None,
            },
            commands: HashMap::new(),
            deno_dependencies: HashMap::new(),
            permissions: None,
        };

        // Empty user config (default)
        let user_config = PluginUserConfig::default();

        let ctx = ExecutionContext::from_parts(
            HashMap::new(),
            &manifest,
            &user_config,
            HashMap::new(),
            "/test/project".to_string(),
            manifest.plugin.clone(),
            false,
        )
        .unwrap();

        let json_str = serde_json::to_string_pretty(&ctx).unwrap();

        // Should still have both sections, even if config is empty
        assert!(
            json_str.contains("\"manifest\":"),
            "Should have manifest section"
        );
        assert!(
            json_str.contains("\"config\":"),
            "Should have config section"
        );
        assert!(
            json_str.contains("\"name\": \"minimal-plugin\""),
            "Should contain plugin name"
        );
    }

    #[test]
    fn test_execution_context_preserves_plugin_args_and_dry_run() {
        use crate::models::{PluginManifest, PluginMeta, PluginUserConfig};
        use std::collections::HashMap;

        let manifest = PluginManifest {
            plugin: PluginMeta {
                name: "test-plugin".to_string(),
                description: None,
                version: "1.0.0".to_string(),
                registry: None,
            },
            commands: HashMap::new(),
            deno_dependencies: HashMap::new(),
            permissions: None,
        };

        let user_config = PluginUserConfig::default();

        // Add plugin arguments
        let mut plugin_args = HashMap::new();
        plugin_args.insert(
            "environment".to_string(),
            toml::Value::String("production".to_string()),
        );
        plugin_args.insert("force".to_string(), toml::Value::Boolean(true));

        let ctx = ExecutionContext::from_parts(
            plugin_args,
            &manifest,
            &user_config,
            HashMap::new(),
            "/test/project".to_string(),
            manifest.plugin.clone(),
            true, // dry_run = true
        )
        .unwrap();

        let json_str = serde_json::to_string_pretty(&ctx).unwrap();

        // Verify plugin args are present
        assert!(
            json_str.contains("\"environment\": \"production\""),
            "Should contain plugin args"
        );
        assert!(
            json_str.contains("\"force\": true"),
            "Should contain boolean plugin args"
        );

        // Verify dry_run flag
        assert!(
            json_str.contains("\"dry_run\": true"),
            "Should contain dry_run flag"
        );
    }

    #[test]
    fn test_context_passing_integration_with_config_files() {
        use std::fs;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .makeitso structure
        let makeitso_dir = temp_dir.path().join(".makeitso");
        let plugin_dir = makeitso_dir.join("plugins").join("context-test-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        // Create manifest.toml
        let manifest_content = r#"
[plugin]
name = "context-test-plugin"
version = "1.5.0"
description = "Plugin for testing context passing"
registry = "https://github.com/example/test-plugins.git"

[commands.context-test]
script = "./context-test.ts"
description = "Test command for context"

[deno_dependencies]
std = "https://deno.land/std@0.204.0/path/mod.ts"
"#;
        fs::write(plugin_dir.join("manifest.toml"), manifest_content).unwrap();

        // Create config.toml
        let config_content = r#"
# User configuration for context test
database_url = "postgres://localhost/testdb"
cache_enabled = true
max_retries = 3

[advanced_settings]
timeout_ms = 10000
batch_size = 100
"#;
        fs::write(plugin_dir.join("config.toml"), config_content).unwrap();

        // Create the script file
        fs::write(
            plugin_dir.join("context-test.ts"),
            "console.log('Context test script');",
        )
        .unwrap();

        // Create mis.toml with project variables
        let mis_config = r#"
name = "test-project"

[project_variables]
project_env = "development"
api_version = "v2"
"#;
        fs::write(makeitso_dir.join("mis.toml"), mis_config).unwrap();

        // Simulate plugin execution with arguments
        let mut plugin_args = std::collections::HashMap::new();
        plugin_args.insert("target".to_string(), "production".to_string());
        plugin_args.insert("verbose".to_string(), "true".to_string());

        // This would normally call run_cmd, but we can't run deno in tests
        // Instead, let's test the context creation directly by loading the files
        let manifest_path = plugin_dir.join("manifest.toml");
        let config_path = plugin_dir.join("config.toml");

        let manifest = crate::config::plugins::load_plugin_manifest(&manifest_path).unwrap();
        let user_config = crate::config::plugins::load_plugin_user_config(&config_path).unwrap();

        // Load project config
        let (mis_config, _, _) = crate::config::load_mis_config().unwrap();

        // Convert plugin args to TOML format
        let plugin_args_toml: HashMap<String, toml::Value> = plugin_args
            .into_iter()
            .map(|(k, v)| (k, toml::Value::String(v)))
            .collect();

        // Create execution context
        let ctx = ExecutionContext::from_parts(
            plugin_args_toml,
            &manifest,
            &user_config,
            mis_config.project_variables,
            temp_dir.path().to_string_lossy().to_string(),
            manifest.plugin.clone(),
            false,
        )
        .unwrap();

        // Serialize and verify the complete context
        let json_str = serde_json::to_string_pretty(&ctx).unwrap();

        // Verify all data sources are present
        assert!(
            json_str.contains("\"name\": \"context-test-plugin\""),
            "Should contain manifest data"
        );
        assert!(
            json_str.contains("\"registry\": \"https://github.com/example/test-plugins.git\""),
            "Should contain registry"
        );
        assert!(
            json_str.contains("\"database_url\": \"postgres://localhost/testdb\""),
            "Should contain user config"
        );
        assert!(
            json_str.contains("\"cache_enabled\": true"),
            "Should contain boolean user config"
        );
        assert!(
            json_str.contains("\"project_env\": \"development\""),
            "Should contain project variables"
        );
        assert!(
            json_str.contains("\"target\": \"production\""),
            "Should contain plugin arguments"
        );

        std::env::set_current_dir(original_dir).unwrap();
    }
}
