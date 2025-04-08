pub mod plugins;

use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use toml::Value;
use crate::{models::MakeItSoConfig, utils::find_project_root};

pub fn load_mis_config() -> Result<(MakeItSoConfig, PathBuf, Value)> {
    let project_root = find_project_root()
        .context("Could not determine project root")?;

    let config_path = project_root
        .join(".makeitso")
        .join("mis.toml");

    let contents = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let service_config: MakeItSoConfig = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse TOML from: {}", config_path.display()))?;

    let raw_config_value: Value = contents
        .parse()
        .with_context(|| format!("Failed to parse TOML from: {}", config_path.display()))?;

    Ok((service_config, config_path, raw_config_value))
}