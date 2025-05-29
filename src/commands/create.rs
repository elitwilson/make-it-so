use std::fs;

use crate::utils::find_project_root;

// Template files that will be used for scaffolding plugins
const PLUGIN_TEMPLATE: &str = include_str!("../../templates/mis-plugin-bootstrap.ts");
const MANIFEST_TEMPLATE: &str = include_str!("../../templates/plugin-manifest.toml");

pub fn create_plugin(name: &str) -> anyhow::Result<()> {
    let root_dir = find_project_root()
        .ok_or_else(|| anyhow::anyhow!("Failed to find project root"))?;

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

    // Write scaffold files - no longer creating local types.d.ts since we use shared files
    fs::write(plugin_dir.join(format!("{}.ts", name)), scaffold_ts(name))?;
    fs::write(plugin_dir.join("plugin.toml"), scaffold_toml(name))?;

    println!("✅ Created plugin '{}'", name);

    Ok(())
}

fn scaffold_ts(name: &str) -> String {
    // Use the template file and replace "examples" placeholder with actual plugin name
    PLUGIN_TEMPLATE.replace("examples", name)
}

fn scaffold_toml(name: &str) -> String {
    // Use the template file and replace "examples" placeholder with actual plugin name
    MANIFEST_TEMPLATE.replace("examples", name)
}
