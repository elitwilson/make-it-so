use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use toml::Value as TomlValue;

#[derive(Debug, Deserialize, Clone)]
pub struct MakeItSoConfig {
    pub name: Option<String>,

    #[serde(rename = "project_variables", default)]
    pub project_variables: HashMap<String, TomlValue>,

    #[serde(default)]
    pub registry: Option<RegistryConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegistryConfig {
    pub sources: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnvConfig {
    pub namespace: Option<String>,

    #[serde(rename = "config_path")]
    pub config_path: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, TomlValue>,
}

/// Security permissions that can be declared in manifest.toml
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct SecurityPermissions {
    /// File paths that can be read (relative to project root or absolute)
    #[serde(default)]
    pub file_read: Vec<String>,

    /// File paths that can be written to (relative to project root or absolute)
    #[serde(default)]
    pub file_write: Vec<String>,

    /// Whether environment variable access is allowed (None = inherit, Some(true/false) = override)
    #[serde(default)]
    pub env_access: Option<bool>,

    /// Network domains/IPs that can be accessed
    #[serde(default)]
    pub network: Vec<String>,

    /// Commands that can be executed
    #[serde(default)]
    pub run_commands: Vec<String>,
}

#[derive(Serialize)]
pub struct ExecutionContext {
    pub plugin_args: HashMap<String, TomlValue>,
    pub manifest: JsonValue,          // <-- plugin manifest data
    pub config: JsonValue,            // <-- user-editable config
    pub project_variables: JsonValue, // <-- project-scoped variables
    pub project_root: String,
    pub meta: PluginMeta,
    pub dry_run: bool,
    // #[serde(skip_serializing)]
    // pub log: Option<()>, // ignored during serialization
}

/// Plugin manifest (manifest.toml) - defines plugin structure and metadata
#[derive(Debug, Deserialize, Serialize)]
pub struct PluginManifest {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub commands: HashMap<String, PluginCommand>,
    #[serde(default)]
    pub deno_dependencies: HashMap<String, String>,
    #[serde(default)]
    pub permissions: Option<SecurityPermissions>,
}

/// User configuration (config.toml) - user-editable project-specific config
#[derive(Debug, Deserialize, Default, Clone)]
pub struct PluginUserConfig {
    #[serde(flatten)]
    pub config: HashMap<String, TomlValue>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PluginMeta {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    #[serde(default)]
    pub registry: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PluginCommand {
    pub script: String,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub instructions: Option<String>,

    #[serde(default)]
    pub args: Option<CommandArgs>,

    /// Command-specific security permissions (extends plugin permissions)
    #[serde(default)]
    pub permissions: Option<SecurityPermissions>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommandArgs {
    #[serde(default)]
    pub required: HashMap<String, ArgDefinition>,

    #[serde(default)]
    pub optional: HashMap<String, ArgDefinition>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ArgDefinition {
    pub description: String,

    #[serde(default)]
    pub arg_type: ArgType,

    #[serde(default)]
    pub default_value: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ArgType {
    #[default]
    String,
    Boolean,
    Integer,
    Float,
}

impl ExecutionContext {
    pub fn from_parts(
        args: HashMap<String, TomlValue>,
        plugin_manifest: &PluginManifest,
        plugin_user_config: &PluginUserConfig,
        project_variables: HashMap<String, TomlValue>,
        project_root: String,
        meta: PluginMeta,
        dry_run: bool,
    ) -> anyhow::Result<Self> {
        // Convert manifest data to JSON (excluding sensitive internal data)
        // ToDo: Revisit this and check if we can move instead of clone
        let manifest_data = ManifestData {
            plugin: plugin_manifest.plugin.clone(),
            commands: plugin_manifest.commands.keys().cloned().collect(),
            deno_dependencies: plugin_manifest.deno_dependencies.clone(),
            registry: plugin_manifest.plugin.registry.clone(),
        };
        let manifest_json: JsonValue = serde_json::to_value(manifest_data)?;

        // Convert user config to JSON
        let user_config_json: JsonValue = toml_to_json(TomlValue::Table(
            plugin_user_config.config.clone().into_iter().collect(),
        ));

        // Convert project vars to JSON
        let mut vars_table = toml::map::Map::new();
        for (k, v) in project_variables {
            vars_table.insert(k, v);
        }
        let project_vars_json: JsonValue = toml_to_json(TomlValue::Table(vars_table));

        Ok(Self {
            plugin_args: args,
            manifest: manifest_json,
            config: user_config_json,
            project_variables: project_vars_json,
            project_root,
            meta,
            dry_run,
        })
    }
}

/// Subset of manifest data exposed to plugins (excludes sensitive permissions data)
#[derive(Debug, Serialize)]
struct ManifestData {
    pub plugin: PluginMeta,
    pub commands: Vec<String>, // Just command names, not full definitions
    pub deno_dependencies: HashMap<String, String>,
    pub registry: Option<String>,
}

// ToDo: Move this to a utility module
fn toml_to_json(val: TomlValue) -> JsonValue {
    serde_json::to_value(val).expect("Failed to convert TOML to JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manifest_parsing_correct_structure() {
        let toml_content = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"
description = "A test plugin"

[permissions]
run_commands = ["git", "docker"]
file_read = ["./config", "./data"]
file_write = ["./output"]
network = ["api.github.com"]
env_access = true

[commands.deploy]
script = "./deploy.ts"
description = "Deploy the application"

[commands.deploy.permissions]
run_commands = ["kubectl"]
network = ["k8s.example.com"]

[deno_dependencies]
oak = "https://deno.land/x/oak@v12.6.1/mod.ts"
"#;

        let manifest: Result<PluginManifest, _> = toml::from_str(toml_content);
        assert!(
            manifest.is_ok(),
            "Well-formed TOML should parse successfully"
        );

        let manifest = manifest.unwrap();
        assert_eq!(manifest.plugin.name, "test-plugin");
        assert_eq!(manifest.plugin.version, "1.0.0");

        // Check plugin-level permissions
        assert!(manifest.permissions.is_some());
        let perms = manifest.permissions.unwrap();
        assert_eq!(perms.run_commands, vec!["git", "docker"]);
        assert_eq!(perms.file_read, vec!["./config", "./data"]);
        assert_eq!(perms.file_write, vec!["./output"]);
        assert_eq!(perms.network, vec!["api.github.com"]);
        assert_eq!(perms.env_access, Some(true));

        // Check command exists
        assert!(manifest.commands.contains_key("deploy"));
        let deploy_cmd = &manifest.commands["deploy"];
        assert_eq!(deploy_cmd.script, "./deploy.ts");

        // Check command-level permissions
        assert!(deploy_cmd.permissions.is_some());
        let cmd_perms = deploy_cmd.permissions.as_ref().unwrap();
        assert_eq!(cmd_perms.run_commands, vec!["kubectl"]);
        assert_eq!(cmd_perms.network, vec!["k8s.example.com"]);

        // Check deno dependencies
        assert!(manifest.deno_dependencies.contains_key("oak"));
    }

    #[test]
    fn test_plugin_manifest_parsing_incorrect_permission_structure() {
        // Test the common mistake: [plugin.permissions] instead of [permissions]
        let incorrect_toml = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"

[plugin.permissions]  # This is WRONG
run_commands = ["git"]

[commands.test]
script = "./test.ts"
"#;

        let manifest: Result<PluginManifest, _> = toml::from_str(incorrect_toml);
        assert!(
            manifest.is_ok(),
            "Should parse but permissions will be None"
        );

        let manifest = manifest.unwrap();
        assert!(
            manifest.permissions.is_none(),
            "Permissions should be None with incorrect structure"
        );
    }

    #[test]
    fn test_plugin_manifest_parsing_missing_permissions() {
        let minimal_toml = r#"
[plugin]
name = "minimal-plugin"
version = "1.0.0"

[commands.test]
script = "./test.ts"
"#;

        let manifest: Result<PluginManifest, _> = toml::from_str(minimal_toml);
        assert!(manifest.is_ok(), "Minimal TOML should parse");

        let manifest = manifest.unwrap();
        assert!(
            manifest.permissions.is_none(),
            "No permissions should be None"
        );
        assert!(
            manifest.deno_dependencies.is_empty(),
            "Deno dependencies should be empty"
        );
        assert!(
            manifest.plugin.registry.is_none(),
            "Registry should be None"
        );
    }

    #[test]
    fn test_security_permissions_default_values() {
        let perms = SecurityPermissions::default();
        assert!(perms.file_read.is_empty());
        assert!(perms.file_write.is_empty());
        assert!(perms.env_access.is_none());
        assert!(perms.network.is_empty());
        assert!(perms.run_commands.is_empty());
    }

    #[test]
    fn test_permission_parsing_type_errors() {
        // Test wrong types in permission arrays
        let bad_toml_configs = vec![
            // String instead of array
            r#"
[plugin]
name = "test"
version = "1.0.0"

[permissions]
run_commands = "git"  # Should be ["git"]

[commands.test]
script = "./test.ts"
"#,
            // Number in array
            r#"
[plugin]
name = "test" 
version = "1.0.0"

[permissions]
run_commands = ["git", 123]

[commands.test]
script = "./test.ts"
"#,
            // Boolean instead of array
            r#"
[plugin]
name = "test"
version = "1.0.0"

[permissions]
file_read = true

[commands.test]
script = "./test.ts"
"#,
        ];

        for bad_config in bad_toml_configs {
            let result: Result<PluginManifest, _> = toml::from_str(bad_config);
            // Should either fail to parse or parse with defaults
            if let Ok(manifest) = result {
                if let Some(perms) = manifest.permissions {
                    // If it parsed, arrays should be empty or valid
                    assert!(perms.run_commands.iter().all(|s| !s.is_empty()));
                    assert!(perms.file_read.iter().all(|s| !s.is_empty()));
                }
            }
        }
    }

    #[test]
    fn test_command_permissions_parsing() {
        let toml_with_command_perms = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"

[commands.basic]
script = "./basic.ts"

[commands.advanced]
script = "./advanced.ts"

[commands.advanced.permissions]
run_commands = ["docker", "kubectl"]
network = ["docker.io", "k8s.example.com"]
file_write = ["./deploy-output"]
"#;

        let manifest: Result<PluginManifest, _> = toml::from_str(toml_with_command_perms);
        assert!(
            manifest.is_ok(),
            "Command permissions should parse correctly"
        );

        let manifest = manifest.unwrap();

        // Basic command should have no permissions
        let basic_cmd = &manifest.commands["basic"];
        assert!(basic_cmd.permissions.is_none());

        // Advanced command should have permissions
        let advanced_cmd = &manifest.commands["advanced"];
        assert!(advanced_cmd.permissions.is_some());

        let cmd_perms = advanced_cmd.permissions.as_ref().unwrap();
        assert_eq!(cmd_perms.run_commands, vec!["docker", "kubectl"]);
        assert_eq!(cmd_perms.network, vec!["docker.io", "k8s.example.com"]);
        assert_eq!(cmd_perms.file_write, vec!["./deploy-output"]);
        assert!(cmd_perms.file_read.is_empty());
        assert!(cmd_perms.env_access.is_none());
    }

    #[test]
    fn test_env_access_permission_parsing() {
        let toml_configs = vec![
            // Explicit true
            (
                r#"
[plugin]
name = "test"
version = "1.0.0"

[permissions]
env_access = true

[commands.test]
script = "./test.ts"
"#,
                Some(true),
            ),
            // Explicit false
            (
                r#"
[plugin]
name = "test"
version = "1.0.0"

[permissions]
env_access = false

[commands.test]
script = "./test.ts"
"#,
                Some(false),
            ),
            // Not specified (should be None)
            (
                r#"
[plugin]
name = "test"
version = "1.0.0"

[permissions]
run_commands = ["git"]

[commands.test]
script = "./test.ts"
"#,
                None,
            ),
        ];

        for (config, expected) in toml_configs {
            let manifest: Result<PluginManifest, _> = toml::from_str(config);
            assert!(manifest.is_ok(), "Config should parse successfully");

            let manifest = manifest.unwrap();
            assert!(manifest.permissions.is_some());

            let perms = manifest.permissions.unwrap();
            assert_eq!(
                perms.env_access, expected,
                "env_access should match expected value"
            );
        }
    }

    #[test]
    fn test_malformed_toml_handling() {
        let malformed_configs = vec![
            // Missing closing bracket
            r#"
[plugin
name = "broken"
version = "1.0.0"
"#,
            // Invalid TOML syntax
            r#"
[plugin]
name = broken-without-quotes
version = "1.0.0"
"#,
            // Duplicate keys
            r#"
[plugin]
name = "test"
name = "duplicate"
version = "1.0.0"
"#,
        ];

        for bad_config in malformed_configs {
            let result: Result<PluginManifest, _> = toml::from_str(bad_config);
            assert!(result.is_err(), "Malformed TOML should fail to parse");
        }
    }

    #[test]
    fn test_empty_arrays_in_permissions() {
        let toml_with_empty_arrays = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"

[permissions]
run_commands = []
file_read = []
file_write = []
network = []

[commands.test]
script = "./test.ts"
"#;

        let manifest: Result<PluginManifest, _> = toml::from_str(toml_with_empty_arrays);
        assert!(manifest.is_ok(), "Empty arrays should parse correctly");

        let manifest = manifest.unwrap();
        assert!(manifest.permissions.is_some());

        let perms = manifest.permissions.unwrap();
        assert!(perms.run_commands.is_empty());
        assert!(perms.file_read.is_empty());
        assert!(perms.file_write.is_empty());
        assert!(perms.network.is_empty());
        assert!(perms.env_access.is_none());
    }

    #[test]
    fn test_mixed_permission_sources() {
        // Test a realistic scenario with plugin and command permissions
        let complex_toml = r#"
[plugin]
name = "complex-plugin"
version = "2.0.0"
description = "A plugin with complex permissions"

[permissions]
run_commands = ["git", "npm"]
file_read = ["./package.json", "./tsconfig.json"]
env_access = true

[commands.build]
script = "./build.ts"
description = "Build the project"

[commands.build.permissions]
run_commands = ["node", "tsc"]
file_write = ["./dist", "./build"]

[commands.deploy]
script = "./deploy.ts"
description = "Deploy to production"

[commands.deploy.permissions]
run_commands = ["docker", "kubectl"]
network = ["docker.io", "k8s.prod.com"]
file_read = ["./secrets"]
env_access = false  # Override plugin setting

[deno_dependencies]
oak = "https://deno.land/x/oak@v12.6.1/mod.ts"
std = "https://deno.land/std@0.204.0/path/mod.ts"
"#;

        let manifest: Result<PluginManifest, _> = toml::from_str(complex_toml);
        assert!(manifest.is_ok(), "Complex TOML should parse successfully");

        let manifest = manifest.unwrap();

        // Verify plugin-level permissions
        assert!(manifest.permissions.is_some());
        let plugin_perms = manifest.permissions.unwrap();
        assert_eq!(plugin_perms.run_commands, vec!["git", "npm"]);
        assert_eq!(
            plugin_perms.file_read,
            vec!["./package.json", "./tsconfig.json"]
        );
        assert_eq!(plugin_perms.env_access, Some(true));

        // Verify build command permissions
        let build_cmd = &manifest.commands["build"];
        assert!(build_cmd.permissions.is_some());
        let build_perms = build_cmd.permissions.as_ref().unwrap();
        assert_eq!(build_perms.run_commands, vec!["node", "tsc"]);
        assert_eq!(build_perms.file_write, vec!["./dist", "./build"]);
        assert!(build_perms.env_access.is_none()); // Not overridden

        // Verify deploy command permissions
        let deploy_cmd = &manifest.commands["deploy"];
        assert!(deploy_cmd.permissions.is_some());
        let deploy_perms = deploy_cmd.permissions.as_ref().unwrap();
        assert_eq!(deploy_perms.run_commands, vec!["docker", "kubectl"]);
        assert_eq!(deploy_perms.network, vec!["docker.io", "k8s.prod.com"]);
        assert_eq!(deploy_perms.file_read, vec!["./secrets"]);
        assert_eq!(deploy_perms.env_access, Some(false)); // Overrides plugin setting

        // Verify deno dependencies
        assert_eq!(manifest.deno_dependencies.len(), 2);
        assert_eq!(
            manifest.deno_dependencies["oak"],
            "https://deno.land/x/oak@v12.6.1/mod.ts"
        );
    }

    #[test]
    fn test_plugin_manifest_parsing_with_registry_field() {
        let toml_with_registry = r#"
[plugin]
name = "registry-plugin"
version = "1.5.0"
description = "Plugin with registry field"
registry = "https://github.com/user/registry-plugins.git"

[commands.test]
script = "./test.ts"
description = "Test command"

[deno_dependencies]
std = "https://deno.land/std@0.204.0/path/mod.ts"
"#;

        let manifest: Result<PluginManifest, _> = toml::from_str(toml_with_registry);
        assert!(
            manifest.is_ok(),
            "Manifest with registry should parse successfully"
        );

        let manifest = manifest.unwrap();
        assert_eq!(manifest.plugin.name, "registry-plugin");
        assert_eq!(manifest.plugin.version, "1.5.0");

        // Check that registry field is populated
        assert!(
            manifest.plugin.registry.is_some(),
            "Registry field should be Some"
        );
        assert_eq!(
            manifest.plugin.registry.unwrap(),
            "https://github.com/user/registry-plugins.git",
            "Registry should match the value in TOML"
        );

        // Check other fields work as expected
        assert!(manifest.commands.contains_key("test"));
        assert!(!manifest.deno_dependencies.is_empty());
    }
}
