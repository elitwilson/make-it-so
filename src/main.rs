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

use std::collections::HashMap;

use anyhow::anyhow;
use clap::Parser;
use cli::{parse_cli_args, Cli, Commands};
use commands::{create::create_plugin, init::run_init, run::run_cmd, add::add_plugin};

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

            let mut parsed_args = HashMap::new();
            let mut iter = args.iter();

            // ToDo: Refactor to use the parse_cli_args function
            while let Some(key) = iter.next() {
                if key.starts_with("--") {
                    if let Some(value) = iter.next() {
                        let key_clean = key.trim_start_matches("--").to_string();
                        parsed_args.insert(key_clean, value.to_string());
                    }
                }
            }

            // Run the command
            run_cmd(plugin_name, command_name, dry_run, parsed_args)?;
        }

        Commands::Create { name } => {
            create_plugin(&name)?;
        }

        Commands::Add {
            args,
            dry_run,
        } => {
            let parsed_args = parse_cli_args(&args);
            for (k, v) in &parsed_args {
                println!("{}: {}", k, v);
            }

            add_plugin(parsed_args, dry_run)?;
        }
    }

    Ok(())
}
