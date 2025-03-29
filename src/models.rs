use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct ServiceConfig {
    pub name: Option<String>,
    pub deploy_strategy: Option<String>,
    pub git_repo_path: Option<String>,

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
pub struct DeploymentContext<'a> {
    pub service_name: &'a str,
    pub namespace: &'a str,
    pub env_name: &'a str,
    pub version: &'a str,
    pub dry_run: bool,
    pub resolved_config_path: PathBuf,
    pub git_repo_path: PathBuf,
    pub raw_env_config: &'a crate::models::EnvConfig,
}

impl<'a> DeploymentContext<'a> {
    pub fn from_config(
        config: &'a ServiceConfig,
        service_file_name: &'a str,
        env: &'a str,
        version: &'a str,
        dry_run: bool,
    ) -> Result<Self> {
        let env_config = config
            .environments
            .get(env)
            .with_context(|| format!("No environment config found for '{}'", env))?;

        let config_path = env_config
            .config_path
            .as_ref()
            .with_context(|| format!("No config_path defined for env '{}'", env))?;

        let resolved_config_path = PathBuf::from(config_path)
            .canonicalize()
            .with_context(|| format!("Failed to resolve path: {}", config_path))?;

        let repo_path = config
            .git_repo_path
            .as_ref()
            .with_context(|| format!("No git_repo_path defined for env '{}'", env))?;

        let resolved_repo_path = PathBuf::from(repo_path)
            .join(format!("{}.toml", service_file_name))
            .parent()
            .unwrap()
            .canonicalize()
            .with_context(|| format!("Failed to resolve path: {}", config_path))?;

        Ok(Self {
            service_name: config.name.as_deref().unwrap_or(service_file_name),
            env_name: env,
            version,
            dry_run,
            namespace: env_config
                .namespace
                .as_deref()
                .unwrap_or("default_namespace"),
            resolved_config_path,
            git_repo_path: resolved_repo_path,
            raw_env_config: env_config,
        })
    }
}
