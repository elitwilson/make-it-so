use toml::Value as TomlValue;
use serde_json::Value as JsonValue;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct MakeItSoConfig {
    pub name: Option<String>,

    #[serde(rename = "project_variables", default)]
    pub project_variables: HashMap<String, TomlValue>,

    #[serde(default)]
    pub registry: Option<RegistryConfig>,
}

#[derive(Debug, Deserialize)]
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
#[derive(Serialize)]
pub struct ExecutionContext {
    pub plugin_args: HashMap<String, TomlValue>,
    pub config: JsonValue, // <-- plugin-specific config
    pub project_variables: JsonValue, // <-- project-scoped variables
    pub project_root: String,
    pub meta: PluginMeta,
    pub dry_run: bool,

    #[serde(skip_serializing)]
    pub log: Option<()>, // ignored during serialization
}

#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMeta,
    pub commands: HashMap<String, PluginCommand>,

    #[serde(default)]
    pub deno_dependencies: HashMap<String, String>, // name -> URL

    #[serde(default)]
    pub user_config: Option<TomlValue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PluginMeta {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct PluginCommand {
    pub script: String,
}

impl ExecutionContext {
    pub fn from_parts(
        args: HashMap<String, TomlValue>,
        plugin_user_config: Option<TomlValue>,
        project_variables: HashMap<String, TomlValue>,
        project_root: String,
        meta: PluginMeta,
        dry_run: bool,
    ) -> anyhow::Result<Self> {
        // 1) plugin config (TOML) → JSON
        let plugin_toml = plugin_user_config
            .unwrap_or_else(|| TomlValue::Table(toml::map::Map::new()));
        let plugin_config_json: JsonValue = toml_to_json(plugin_toml);

        // 2) project vars (flat map) → TOML table → JSON
        let mut vars_table = toml::map::Map::new();
        for (k, v) in project_variables {
            vars_table.insert(k, v);
        }
        let project_vars_json: JsonValue = toml_to_json(TomlValue::Table(vars_table));

        Ok(Self {
            plugin_args: args,
            config: plugin_config_json,
            project_variables: project_vars_json,
            project_root,
            meta,
            dry_run,
            log: None,
        })
    }
}



// ToDo: Move this to a utility module
fn toml_to_json(val: TomlValue) -> JsonValue {
    serde_json::to_value(val).expect("Failed to convert TOML to JSON")
}
