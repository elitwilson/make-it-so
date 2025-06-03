use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::models::{PluginManifest, PluginUserConfig};

pub fn load_plugin_manifest(path: &Path) -> Result<PluginManifest> {
    let toml_str = fs::read_to_string(path)
        .with_context(|| format!("Failed to read plugin manifest at {}", path.display()))?;

    let manifest: PluginManifest = toml::from_str(&toml_str).with_context(|| {
        format!(
            "🛑 Corrupted manifest.toml found at {}\n\
                 → The TOML syntax is invalid. Common issues:\n\
                 → • Missing closing brackets: [plugin\n\
                 → • Missing quotes: version = 1.0.0 (should be \"1.0.0\")\n\
                 → • Invalid characters or formatting\n\
                 → Fix the syntax errors and try again.",
            path.display()
        )
    })?;

    Ok(manifest)
}

pub fn load_plugin_user_config(path: &Path) -> Result<PluginUserConfig> {
    if !path.exists() {
        // config.toml is optional - return empty config if it doesn't exist
        return Ok(PluginUserConfig::default());
    }

    let toml_str = fs::read_to_string(path)
        .with_context(|| format!("Failed to read plugin config at {}", path.display()))?;

    let config: PluginUserConfig = toml::from_str(&toml_str).with_context(|| {
        format!(
            "🛑 Corrupted config.toml found at {}\n\
                 → The TOML syntax is invalid. Check for syntax errors and try again.",
            path.display()
        )
    })?;

    Ok(config)
}
