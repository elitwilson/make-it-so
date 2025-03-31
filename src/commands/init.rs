use anyhow::Context;
use std::path::PathBuf;
use std::{fs, os::unix::fs::PermissionsExt};

use anyhow::Result;

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

// pub fn scaffold_plugin_if_needed(strategy: &str) -> Result<()> {
//     // Skip if it's a built-in strategy
//     if get_deploy_strategy(strategy).is_ok() {
//         println!(
//             "🧠 '{}' is a built-in strategy — skipping plugin scaffold.",
//             strategy
//         );
//         return Ok(());
//     }

//     let plugin_path = PathBuf::from(format!(".makeitso/{}.js", strategy));
//     if plugin_path.exists() {
//         println!("⚠️  Plugin already exists: {}", plugin_path.display());
//         return Ok(());
//     }

//     // Create plugin stub
//     let js_stub = r#"#!/usr/bin/env node

// process.stdin.setEncoding("utf8");
// process.stdin.on("data", (data) => {
// const ctx = JSON.parse(data);

// const { service_name, env_name, version, dry_run } = ctx;

// console.log(`🚀 Deploying ${service_name} to ${env_name} (version: ${version})`);

// if (dry_run) {
//   console.log("🚫 Dry run: skipping actual deploy.");
//   return;
// }

// // TODO: Replace this with your real deploy logic (Azure CLI, etc)
// });
// "#;

//     fs::write(&plugin_path, js_stub.trim_start())
//         .with_context(|| format!("Failed to write JS plugin to {}", plugin_path.display()))?;

//     make_executable(&plugin_path)?;

//     println!("🛠️  Created JS plugin: {}", plugin_path.display());
//     Ok(())
// }

fn make_executable(plugin_path: &PathBuf) -> Result<()> {
    let mut perms = fs::metadata(plugin_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(plugin_path, perms)?;
    Ok(())
}

pub fn run_init(name: Option<&str>) -> Result<()> {
    let mis_dir = PathBuf::from(".makeitso");
    if !mis_dir.exists() {
        fs::create_dir_all(&mis_dir)?;
        println!("📁 Created .makeitso/");
    }

    let config_path = mis_dir.join("mis.toml");

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
