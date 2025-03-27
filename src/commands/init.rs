use anyhow::Context;
use std::path::PathBuf;
use std::{fs, os::unix::fs::PermissionsExt};

use anyhow::Result;

use crate::strategy::deploy::get_deploy_strategy;

fn generate_service_toml(service: &str, strategy: &str) -> String {
    format!(
        r#"
name = "{service}"
deploy_strategy = "{strategy}"

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

pub fn scaffold_plugin_if_needed(strategy: &str) -> Result<()> {
    // Skip if it's a built-in strategy
    if get_deploy_strategy(strategy).is_ok() {
        println!(
            "🧠 '{}' is a built-in strategy — skipping plugin scaffold.",
            strategy
        );
        return Ok(());
    }

    let plugin_path = PathBuf::from(format!(".shipwreck/{}.js", strategy));
    if plugin_path.exists() {
        println!("⚠️  Plugin already exists: {}", plugin_path.display());
        return Ok(());
    }

    // Create plugin stub
    let js_stub = r#"#!/usr/bin/env node

process.stdin.setEncoding("utf8");
process.stdin.on("data", (data) => {
const ctx = JSON.parse(data);

const { service_name, env_name, version, dry_run } = ctx;

console.log(`🚀 Deploying ${service_name} to ${env_name} (version: ${version})`);

if (dry_run) {
  console.log("🚫 Dry run: skipping actual deploy.");
  return;
}

// TODO: Replace this with your real deploy logic (Azure CLI, etc)
});
"#;

    fs::write(&plugin_path, js_stub.trim_start())
        .with_context(|| format!("Failed to write JS plugin to {}", plugin_path.display()))?;

    make_executable(&plugin_path)?;

    println!("🛠️  Created JS plugin: {}", plugin_path.display());
    Ok(())
}

fn make_executable(plugin_path: &PathBuf) -> Result<()> {
    let mut perms = fs::metadata(plugin_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(plugin_path, perms)?;
    Ok(())
}

// Very naive PascalCase conversion
fn pascal_case(input: &str) -> String {
    input
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<String>()
}

pub fn run_init(service: String, strategy: String) -> Result<()> {
    let shipwreck_dir = PathBuf::from(".shipwreck");
    if !shipwreck_dir.exists() {
        fs::create_dir_all(&shipwreck_dir)?;
        println!("📁 Created .shipwreck/");
    }

    let config_path = shipwreck_dir.join(format!("{}.toml", service));
    if !config_path.exists() {
        let toml = generate_service_toml(&service, &strategy);
        fs::write(&config_path, toml)?;
        println!("📝 Created config file: {}", config_path.display());
    } else {
        println!("⚠️  Config already exists: {}", config_path.display());
    }

    scaffold_plugin_if_needed(&strategy)?;

    if get_deploy_strategy(&strategy).is_err() {
        println!("🛠 Not a built-in strategy, scaffolding plugin...");
        // scaffold code
    }

    println!("✅ Shipwreck service '{}' initialized.", service);
    Ok(())
}
