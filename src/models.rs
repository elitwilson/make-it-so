use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

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
pub struct ExecutionContext<'a> {
    pub service_name: &'a str,
    pub dry_run: bool,
    // pub resolved_config_path: PathBuf,
    // pub git_repo_path: PathBuf,
    // pub raw_env_config: &'a crate::models::EnvConfig,
}

#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMeta,
    pub commands: HashMap<String, PluginCommand>,

    #[serde(default)]
    pub deno_dependencies: HashMap<String, String>, // name -> URL
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

impl<'a> ExecutionContext<'a> {
    pub fn from_config(
        config: &'a MakeItSoConfig,
        service_file_name: &'a str,
        dry_run: bool,
    ) -> Result<Self> {

        // let config_path = env_config
        //     .config_path
        //     .as_ref()
        //     .with_context(|| format!("No config_path defined for env '{}'", env))?;

        // let resolved_config_path = PathBuf::from(config_path)
        //     .canonicalize()
        //     .with_context(|| format!("Failed to resolve path: {}", config_path))?;

        Ok(Self {
            service_name: config.name.as_deref().unwrap_or(service_file_name),
            dry_run,
            // resolved_config_path,
            // namespace: env_config
            //     .namespace
            //     .as_deref()
            //     .unwrap_or("default_namespace"),
            // git_repo_path: resolved_repo_path,
            // raw_env_config: env_config,
        })
    }
}
