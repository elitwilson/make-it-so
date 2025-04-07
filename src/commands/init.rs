use std::path::PathBuf;
use std::{fs, os::unix::fs::PermissionsExt};

use anyhow::Result;

use crate::cli::prompt_user;
use crate::integrations::deno::{install_deno, is_deno_installed};
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
name = "make-it-so-cli"

# 👇 Project-scoped settings available to all plugins
[project_variables]
# — User-created (optional) env variables accessible by plugins through the ctx object:
foo = "bar"            # ← EXAMPLE of a project-scoped variable
"#
    )
    .trim_start()
    .to_string()
}

pub fn run_init(name: Option<&str>) -> Result<()> {
    if !is_deno_installed() {
        let should_install = prompt_user("Deno is not installed. Would you like to install it?")?;
        if !should_install {
            anyhow::bail!("Deno is required for Make It So. Please install it and try again.");
        }
        
        // Install Deno
        install_deno()?; // or prompt/abort if you want confirmation
    }    

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
