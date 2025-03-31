use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, hash::Hash, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct MakeItSoConfig {
    pub name: Option<String>,
    // pub deploy_strategy: Option<String>,
    // pub git_repo_path: Option<String>,

    #[serde(rename = "plugins")]
    pub plugins: Option<toml::Value>,

    pub environments: HashMap<String, EnvConfig>,

    #[serde(rename = "strategy_config")]
    pub strategy_config: Option<toml::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnvConfig {
    pub namespace: Option<String>,

    #[serde(rename = "config_path")]
    pub config_path: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}
#[derive(Serialize)]
pub struct ExecutionContext {
    pub plugin_args: HashMap<String, Value>,
    pub config: Value, // <-- plugin-specific config
    pub project_root: String,
    // pub env: HashMap<String, String>,
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
    pub user_config: Option<toml::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PluginMeta {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
}

#[derive(Serialize)]
struct PluginContext {
    args: HashMap<String, Value>,
    config: Value, // 👈 untyped JSON blob—plugin owns this
    project_root: String,
    env: HashMap<String, String>,
    meta: PluginMeta,
    #[serde(skip_serializing)]
    log: Option<()>,
}

#[derive(Debug, Deserialize)]
pub struct PluginCommand {
    pub description: Option<String>,
    pub script: String,
    pub entrypoint: String,

    #[serde(default)]
    pub options: HashMap<String, PluginOption>,
}

#[derive(Debug, Deserialize)]
pub struct PluginOption {
    #[serde(rename = "type")]
    pub type_: String, // "string", "bool", "number"
    pub required: Option<bool>,
    pub default: Option<toml::Value>,
    pub description: Option<String>,
}

impl ExecutionContext {
    pub fn from_parts(
        args: HashMap<String, Value>,
        plugin_user_config: Option<toml::Value>,
        project_root: String,
        meta: PluginMeta,
        dry_run: bool,
    ) -> Result<Self> {
        let plugin_config_toml = plugin_user_config.unwrap_or_else(|| {
            toml::Value::Table(toml::map::Map::new())
        });

        let plugin_config_json = toml_to_json(plugin_config_toml);

        Ok(Self {
            plugin_args: args,
            config: plugin_config_json,
            project_root,
            meta,
            dry_run,
            log: None,
        })
    }
}


// ToDo: Move this to a utility module
fn toml_to_json(val: toml::Value) -> serde_json::Value {
    let s = toml::to_string(&val).expect("Failed to stringify TOML");
    toml::from_str::<serde_json::Value>(&s).expect("Failed to parse TOML as JSON")
}
