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
