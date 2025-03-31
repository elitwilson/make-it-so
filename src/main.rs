//! Shipwreck CLI
//! Or: How I Learned to Stop Worrying and Wrap My Simple CI/CD Workflow In A Rust CLI Instead of a Makefile
//!
//! A silly, hilarious extravagance in personal CLI tooling that is delightfully excessive yet likely to reduce some pain in the long run.
//!

mod cli;
mod commands;
mod config;
mod constants;
mod git_utils;
mod integrations;
mod models;

use std::collections::HashMap;

use anyhow::{Context, anyhow};
use clap::Parser;
use cli::{Cli, Commands};
use commands::{init::run_init, run::run_cmd};
use config::plugins::load_plugin_manifest;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => {
            let name_ref = name.as_deref();
            run_init(name_ref)?;
        }

        Commands::Run {
            plugin,
            args,
            dry_run,
        } => {
            let parts: Vec<&str> = plugin.split(':').collect();
            if parts.len() != 2 {
                return Err(anyhow!(
                    "Invalid plugin format. Use <plugin_name>:<command_name>"
                ));
            }

            let command_name = parts[1];

            let plugin_name = parts[0].to_string();
            // Check if the plugin exists
            let plugin_dir = std::path::PathBuf::from(".makeitso")
                .join("plugins")
                .join(&plugin_name);

            // Extract plugin information
            let plugin_manifest_path = plugin_dir.join("plugin.toml");
            let plugin_manifest = load_plugin_manifest(&plugin_manifest_path)?;

            let command = plugin_manifest
                .commands
                .get(command_name)
                .with_context(|| {
                    format!(
                        "Command '{}' not found in plugin '{}'",
                        command_name, plugin_name
                    )
                })?;

            if !plugin_dir.exists() {
                return Err(anyhow!(
                    "Plugin '{}' not found in .makeitso/plugins",
                    plugin_name
                ));
            }

            let mut parsed_args = HashMap::new();
            let mut iter = args.iter();

            while let Some(key) = iter.next() {
                if key.starts_with("--") {
                    if let Some(value) = iter.next() {
                        let key_clean = key.trim_start_matches("--").to_string();
                        parsed_args.insert(key_clean, value.to_string());
                    }
                }
            }

            // Check if the command exists in the plugin manifest
            // ToDo: Implement this check

            // Run the command
            run_cmd(plugin_name, command_name, dry_run, parsed_args)?;
        }

        Commands::Create {} => {}
    }

    Ok(())
}
