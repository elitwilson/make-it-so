pub mod plugins;

use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use toml::Value;
use crate::models::MakeItSoConfig;

pub fn load_shipwreck_config() -> Result<(MakeItSoConfig, PathBuf, Value)> {
  let shipwreck_base_path = ".makeitso";

  let config_path = PathBuf::from(shipwreck_base_path).join(format!("mis.toml"));

  // Read file contents
  let contents = fs::read_to_string(&config_path)
    .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

  // Parse TOML
  let service_config: MakeItSoConfig = toml::from_str(&contents)
    .with_context(|| format!("Failed to parse TOML from: {}", config_path.display()))?;

  let raw_config_value: Value = contents
    .parse()
    .with_context(|| format!("Failed to parse TOML from: {}", config_path.display()))?;

  Ok((service_config, config_path, raw_config_value))
}