use std::fs;

use crate::utils::find_project_root;

// Template files that will be used for scaffolding plugins
const PLUGIN_TEMPLATE: &str = include_str!("../../templates/mis-plugin-bootstrap.ts");
const MANIFEST_TEMPLATE: &str = include_str!("../../templates/plugin-manifest.toml");
const CONFIG_TEMPLATE: &str = include_str!("../../templates/config.toml");

pub fn create_plugin(name: &str) -> anyhow::Result<()> {
    let root_dir =
        find_project_root().ok_or_else(|| anyhow::anyhow!("Failed to find project root"))?;

    let makeitso_dir = root_dir.join(".makeitso");

    if !makeitso_dir.exists() {
        anyhow::bail!(
            "🛑 No Make It So project found in this directory.\n→ Run `mis init` first to initialize your project."
        );
    }

    let plugin_dir = makeitso_dir.join("plugins").join(name);

    if plugin_dir.exists() {
        anyhow::bail!("Plugin '{}' already exists", name);
    }

    fs::create_dir_all(&plugin_dir)?;

    // Write scaffold files using new split config structure
    fs::write(plugin_dir.join(format!("{}.ts", name)), scaffold_ts(name))?;
    fs::write(plugin_dir.join("manifest.toml"), scaffold_manifest(name))?;
    fs::write(plugin_dir.join("config.toml"), scaffold_config())?;

    println!(
        "✅ Created plugin '{}' with new split config structure",
        name
    );
    println!("   → manifest.toml: Plugin metadata and commands");
    println!("   → config.toml: User-editable configuration");
    println!("   → {}.ts: Plugin script", name);

    Ok(())
}

fn scaffold_ts(name: &str) -> String {
    // Use the template file and replace "examples" placeholder with actual plugin name
    PLUGIN_TEMPLATE.replace("examples", name)
}

fn scaffold_manifest(name: &str) -> String {
    // Use the template file and replace "examples" placeholder with actual plugin name
    MANIFEST_TEMPLATE.replace("examples", name)
}

fn scaffold_config() -> String {
    // Use the config template as-is (it's already generic)
    CONFIG_TEMPLATE.to_string()
}
