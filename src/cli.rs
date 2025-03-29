use std::io::{self, Write};

use clap::{Parser, Subcommand};

/// Your CLI entrypoint definition
#[derive(Parser)]
#[command(
    name = "shipwreck",
    version,
    about = "Rusty CI/CD tool for building & deploying services",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init {
        /// The name of the project
        name: String,

        /// The name of the strategy to use (e.g. ecs, k8s)
        #[arg(long)]
        strategy: String,
    },
    /// Deploy a specific service to a specific environment with a version
    Deploy {
        /// The service to deploy (e.g. api, worker)
        service: String,

        /// The environment to deploy to (e.g. dev, staging, prod)
        #[arg(long)]
        env: String,

        /// The version to deploy (e.g. 1.2.3)
        #[arg(long)]
        version: String,

        /// Run without actually making changes
        #[arg(long)]
        dry_run: bool,
    },
    Build {
        /// The service to build (e.g. api, worker)
        service: String,

        /// The environment to build for (e.g. dev, staging, prod)
        #[arg(long)]
        env: String,

        /// The version to build (e.g. 1.2.3)
        #[arg(long)]
        version: String,

        /// Run without actually making changes
        #[arg(long)]
        dry_run: bool,
    }
}

pub fn prompt_user(message: &str) -> anyhow::Result<bool> {
    print!("{} [y/N]: ", message);
    io::stdout().flush()?; // Make sure the prompt shows before user types

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    Ok(matches!(input.as_str(), "y" | "yes"))
}
