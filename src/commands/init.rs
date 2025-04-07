use std::path::PathBuf;
use std::{fs, os::unix::fs::PermissionsExt};

use anyhow::Result;

use crate::utils::find_project_root;

// use crate::strategy::deploy::get_deploy_strategy;

fn generate_mis_toml(name: Option<&str>) -> String {
    // If name is None, use the name of the current directory
    let current_dir = std::env::current_dir()
        .expect("Failed to get current directory");
    let dir_name = current_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(".");

    let proj_name = name.unwrap_or_else(|| dir_name);

    format!(
        r#"
name = "{proj_name}"

[strategy_config]
# Add your strategy-specific config here

[environments.dev]
namespace = "dev"
config_path = "devops/dev.yaml"
"#
    )
    .trim_start()
    .to_string()
}


fn make_executable(plugin_path: &PathBuf) -> Result<()> {
    let mut perms = fs::metadata(plugin_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(plugin_path, perms)?;
    Ok(())
}

pub fn run_init(name: Option<&str>) -> Result<()> {
    if let Some(existing_root) = find_project_root() {
        anyhow::bail!(
            "🛑 Already inside a Make It So project (found at {}).\n\
             → You can't re-initialize within an existing project.",
            existing_root.display()
        );
    }

    let current_dir = std::env::current_dir()?;
    let makeitso_dir = current_dir.join(".makeitso");

    if !makeitso_dir.exists() {
        fs::create_dir_all(&makeitso_dir)?;
        println!("📁 Created .makeitso/");
    }

    let config_path = makeitso_dir.join("mis.toml");

    if !config_path.exists() {
        let toml = generate_mis_toml(name);
        fs::write(&config_path, toml)?;
        println!("📝 Created config file: {}", config_path.display());
    } else {
        println!("⚠️  Config already exists: {}", config_path.display());
    }

    // scaffold_plugin_if_needed(&strategy)?;

    println!("✅ Make-It-So service initialized.");
    Ok(())
}
