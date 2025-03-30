//! Shipwreck CLI
//! Or: How I Learned to Stop Worrying and Wrap My Simple CI/CD Workflow In A Rust CLI Instead of a Makefile
//! 
//! A silly, hilarious extravagance in personal CLI tooling that is delightfully excessive yet likely to reduce some pain in the long run.
//!

mod cli;
mod config;
mod models;
mod commands;
mod strategy;
mod git_utils;
mod integrations;
mod constants;

use clap::Parser;
use cli::{Cli, Commands};
use commands::{init::run_init, run::run_cmd};
use anyhow::anyhow;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => {
            let name_ref = name.as_deref();
            run_init(name_ref)?;
        },

        Commands::Run { plugin, dry_run } => {
            let parts: Vec<&str> = plugin.split(':').collect();
            if parts.len() != 2 {
                return Err(anyhow!("Invalid plugin format. Use <plugin_name>:<command_name>"));
            }

            let plugin_name = parts[0].to_string();
            println!("Plugin name: {}", plugin_name);

            let command_name = parts[1];

            // Check if the plugin exists
            let plugin_dir = std::path::PathBuf::from(".makeitso")
                .join("plugins")
                .join(&plugin_name);
            
            if !plugin_dir.exists() {
                return Err(anyhow!("Plugin '{}' not found in .makeitso/plugins", plugin_name));
            }

            // Check if the command exists in the plugin manifest
            // ToDo: Implement this check
            
            // Run the command
            run_cmd(plugin_name, command_name, dry_run)?;
        }
    }

    Ok(())
}