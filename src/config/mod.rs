use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use toml::Value;
use crate::models::ServiceConfig;

pub fn load_shipwreck_config() -> Result<(ServiceConfig, PathBuf, Value)> {
  let shipwreck_base_path = ".shipwreck";

  let config_path = PathBuf::from(shipwreck_base_path).join(format!("shipwreck.toml"));

  // Read file contents
  let contents = fs::read_to_string(&config_path)
    .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

  // Parse TOML
  let service_config: ServiceConfig = toml::from_str(&contents)
    .with_context(|| format!("Failed to parse TOML from: {}", config_path.display()))?;

  let raw_config_value: Value = contents
    .parse()
    .with_context(|| format!("Failed to parse TOML from: {}", config_path.display()))?;

  Ok((service_config, config_path, raw_config_value))
}