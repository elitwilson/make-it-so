use std::fs;
use std::path::Path;
use anyhow::{Result, Context};

use crate::models::PluginManifest;

pub fn load_plugin_manifest(path: &Path) -> Result<PluginManifest> {
    let toml_str = fs::read_to_string(path)
        .with_context(|| format!("Failed to read plugin manifest at {}", path.display()))?;

    let manifest: PluginManifest = toml::from_str(&toml_str)
        .with_context(|| "Failed to parse plugin TOML")?;

    Ok(manifest)
}
