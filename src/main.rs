//! Make It So CLI
//! Or: How I Learned to Stop Worrying and Wrap My Simple CI/CD Workflow In A Rust CLI Instead of a Makefile
//!
//! A silly, hilarious extravagance in personal CLI tooling that is delightfully excessive yet hopefully useful.
//!

mod cli;
mod commands;
mod config;
mod constants;
mod git_utils;
mod integrations;
mod models;
mod utils;
mod validation;

use anyhow::anyhow;
use clap::Parser;
use cli::{Cli, Commands};
use commands::{create::create_plugin, init::run_init, run::run_cmd, add::add_plugin, help::show_help};

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

            let parsed_args = cli::parse_cli_args(&args);

            // Run the command
            run_cmd(plugin_name, command_name, dry_run, parsed_args)?;
        }

        Commands::Create { name } => {
            create_plugin(&name)?;
        }

        Commands::Add {
            plugins,
            dry_run,
            registry,
            force,
        } => {
            add_plugin(plugins, dry_run, registry, force)?;
        }

        Commands::Info { plugin_command } => {
            show_help(&plugin_command)?;
        }
    }

    Ok(())
}