use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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
    pub args: HashMap<String, String>,
    // pub plugin_manifest: Option<PluginManifest>,
    // pub service_name: &'a str, // Deprecated... get rid of me
    pub dry_run: bool,
    pub plugin_config: toml::Value,
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

#[derive(Debug, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub description: Option<String>,
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
    pub fn from_config(
        config:  MakeItSoConfig,
        args: HashMap<String, String>,
        plugin_user_config: Option<toml::Value>,
        dry_run: bool,
    ) -> Result<Self> {
        let plugin_config = plugin_user_config.unwrap_or_else(|| {
            toml::Value::Table(toml::map::Map::new())
        });

        Ok(Self {
            args,
            dry_run,
            plugin_config,
        })
    }
}

