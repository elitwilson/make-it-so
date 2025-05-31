use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::models::PluginManifest;

pub fn load_plugin_manifest(path: &Path) -> Result<PluginManifest> {
    let toml_str = fs::read_to_string(path)
        .with_context(|| format!("Failed to read plugin manifest at {}", path.display()))?;

    let manifest: PluginManifest = toml::from_str(&toml_str).with_context(|| {
        format!(
            "🛑 Corrupted plugin.toml found at {}\n\
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
