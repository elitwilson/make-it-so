use std::{collections::HashMap, io::{self, Write}};
use clap::{Parser, Subcommand};

/// Your CLI entrypoint definition
#[derive(Parser)]
#[command(
    name = "make-it-so",
    version,
    about = "A fast CLI that runs TypeScript-powered plugins for your dev workflows.",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init {
        name: Option<String>,
    },
    Run {
        /// The name of the plugin to run (e.g. api, worker)
        plugin: String,
        
        /// Run without actually making changes
        #[arg(long)]
        dry_run: bool,

        /// Any extra args passed to the plugin command
        // #[arg(long, value_parser, num_args=1.., allow_hyphen_values=true)]
        #[arg(trailing_var_arg = true, allow_hyphen_values=true)]
        args: Vec<String>,


    },
    Create {
        #[arg(value_name = "plugin_name")]
        name: String,
    },
    Add {
        #[arg(trailing_var_arg = true, allow_hyphen_values=true)]
        args: Vec<String>,

        #[arg(long)]
        dry_run: bool,
    },    
}

pub fn prompt_user(message: &str) -> anyhow::Result<bool> {
    print!("{} [y/N]: ", message);
    io::stdout().flush()?; // Make sure the prompt shows before user types

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    Ok(matches!(input.as_str(), "y" | "yes"))
}

pub fn parse_cli_args(args: &[String]) -> HashMap<String, String> {    
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

    parsed_args
}