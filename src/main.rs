mod cli;
mod config;
mod models;
mod commands;
mod strategy;

use clap::Parser;
use cli::{Cli, Commands};
use commands::{build::run_build, deploy::run_deploy, init::run_init};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Deploy { service, env, version, dry_run } => {
            run_deploy(service, env, version, dry_run)?;
        }

        Commands::Build { service, env, version, dry_run } => {
            run_build(service, env, version, dry_run)?;
        }

        Commands::Init { name, strategy } => {
            run_init(name, strategy)?;
        }
    }

    Ok(())
}
