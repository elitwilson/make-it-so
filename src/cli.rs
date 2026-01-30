use clap::{Parser, Subcommand};
use std::{
    collections::HashMap,
    io::{self, Write},
};

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
    /// Initialize this directory as a new .makeitso project
    Init { name: Option<String> },
    /// Execute a plugin command
    Run {
        /// The name of the plugin to run (e.g. api, worker)
        plugin: String,

        /// Run without actually making changes
        #[arg(long)]
        dry_run: bool,

        /// Any extra args passed to the plugin command
        // #[arg(long, value_parser, num_args=1.., allow_hyphen_values=true)]
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Create a new plugin from template
    Create {
        #[arg(value_name = "plugin_name")]
        name: String,
    },
    /// Install plugins from registries
    Add {
        plugins: Vec<String>,

        #[arg(long)]
        dry_run: bool,

        #[arg(long)]
        registry: Option<String>,

        #[arg(long)]
        force: bool,
    },
    /// Update a specific plugin or all plugins to the latest versions
    Update {
        plugin: Option<String>,

        #[arg(long)]
        dry_run: bool,
    },
    /// Show detailed help for a plugin command
    Info {
        /// Plugin and command to show information for (e.g. my-plugin:deploy)
        plugin_command: Option<String>,
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

/// Check if we should inject "run" command for implicit plugin execution
/// Returns true if the first argument looks like a plugin:command and isn't already "run"
pub fn should_inject_run_command(args: &[String]) -> bool {
    if args.len() < 2 {
        return false;
    }

    let first_arg = &args[1];

    // Don't inject if it's a flag
    if first_arg.starts_with("--") || first_arg.starts_with('-') {
        return false;
    }

    // Don't inject if it's already an explicit subcommand
    let known_subcommands = ["init", "run", "create", "add", "update", "info"];
    if known_subcommands.contains(&first_arg.as_str()) {
        return false;
    }

    // Inject if it looks like a plugin:command
    first_arg.contains(':')
}

/// Transform args to inject "run" command if needed
/// Example: ["mis", "claude:init", "--flag"] → ["mis", "run", "claude:init", "--flag"]
pub fn transform_args_for_implicit_run(args: &[String]) -> Vec<String> {
    if should_inject_run_command(args) {
        let mut new_args = vec![args[0].clone(), "run".to_string()];
        new_args.extend_from_slice(&args[1..]);
        new_args
    } else {
        args.to_vec()
    }
}

pub fn parse_cli_args(args: &[String]) -> HashMap<String, String> {
    let mut parsed_args = HashMap::new();
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        if arg.starts_with("--") {
            // Handle --key=value format
            if let Some(eq_pos) = arg.find('=') {
                let key = arg[2..eq_pos].to_string();
                let value = arg[eq_pos + 1..].to_string();
                parsed_args.insert(key, value);
            } else {
                // Handle --key value format or boolean flags
                let key = arg[2..].to_string();

                // Check if next argument exists and is not a flag
                if let Some(next_arg) = iter.peek() {
                    if !next_arg.starts_with("--") {
                        // Next argument is a value
                        let value = iter.next().unwrap().to_string();
                        parsed_args.insert(key, value);
                    } else {
                        // Next argument is another flag, treat current as boolean
                        parsed_args.insert(key, "true".to_string());
                    }
                } else {
                    // No more arguments, treat as boolean flag
                    parsed_args.insert(key, "true".to_string());
                }
            }
        }
        // Ignore non-flag arguments (positional arguments)
    }

    parsed_args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cli_args_basic_key_value_pairs() {
        let args = vec![
            "--name".to_string(),
            "test".to_string(),
            "--count".to_string(),
            "5".to_string(),
        ];
        let result = parse_cli_args(&args);

        assert_eq!(result.get("name"), Some(&"test".to_string()));
        assert_eq!(result.get("count"), Some(&"5".to_string()));
    }

    #[test]
    fn test_parse_cli_args_boolean_flags_without_values() {
        let args = vec![
            "--verbose".to_string(),
            "--force".to_string(),
            "--name".to_string(),
            "test".to_string(),
        ];
        let result = parse_cli_args(&args);

        // Now handles boolean flags properly
        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
        assert_eq!(result.get("force"), Some(&"true".to_string()));
        assert_eq!(result.get("name"), Some(&"test".to_string()));
    }

    #[test]
    fn test_parse_cli_args_quoted_values_with_spaces() {
        let args = vec![
            "--message".to_string(),
            "hello world".to_string(),
            "--path".to_string(),
            "/path/with spaces/file.txt".to_string(),
        ];
        let result = parse_cli_args(&args);

        assert_eq!(result.get("message"), Some(&"hello world".to_string()));
        assert_eq!(
            result.get("path"),
            Some(&"/path/with spaces/file.txt".to_string())
        );
    }

    #[test]
    fn test_parse_cli_args_equals_format() {
        // This format is now supported
        let args = vec!["--name=test".to_string(), "--count=5".to_string()];
        let result = parse_cli_args(&args);

        // Now handles --key=value format
        assert_eq!(result.get("name"), Some(&"test".to_string()));
        assert_eq!(result.get("count"), Some(&"5".to_string()));
    }

    #[test]
    fn test_parse_cli_args_mixed_formats() {
        let args = vec![
            "--name".to_string(),
            "test".to_string(),
            "--verbose".to_string(),
            "--count=5".to_string(),
            "--force".to_string(),
        ];
        let result = parse_cli_args(&args);

        assert_eq!(result.get("name"), Some(&"test".to_string()));
        // Now handles all formats correctly:
        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
        assert_eq!(result.get("count"), Some(&"5".to_string()));
        assert_eq!(result.get("force"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_cli_args_empty_values() {
        let args = vec![
            "--name".to_string(),
            "".to_string(),
            "--count".to_string(),
            "5".to_string(),
        ];
        let result = parse_cli_args(&args);

        assert_eq!(result.get("name"), Some(&"".to_string()));
        assert_eq!(result.get("count"), Some(&"5".to_string()));
    }

    #[test]
    fn test_parse_cli_args_special_characters() {
        let args = vec![
            "--url".to_string(),
            "https://example.com/path?param=value&other=123".to_string(),
            "--regex".to_string(),
            "^[a-zA-Z0-9]+$".to_string(),
        ];
        let result = parse_cli_args(&args);

        assert_eq!(
            result.get("url"),
            Some(&"https://example.com/path?param=value&other=123".to_string())
        );
        assert_eq!(result.get("regex"), Some(&"^[a-zA-Z0-9]+$".to_string()));
    }

    #[test]
    fn test_parse_cli_args_orphaned_flags() {
        let args = vec![
            "--name".to_string(),
            "test".to_string(),
            "--orphaned".to_string(),
            // No value for --orphaned, followed by another flag
            "--count".to_string(),
            "5".to_string(),
        ];
        let result = parse_cli_args(&args);

        assert_eq!(result.get("name"), Some(&"test".to_string()));
        // Now correctly treats --orphaned as a boolean flag
        assert_eq!(result.get("orphaned"), Some(&"true".to_string()));
        assert_eq!(result.get("count"), Some(&"5".to_string())); // No longer consumed
    }

    #[test]
    fn test_parse_cli_args_non_flag_arguments() {
        let args = vec![
            "positional".to_string(),
            "--name".to_string(),
            "test".to_string(),
            "another-positional".to_string(),
        ];
        let result = parse_cli_args(&args);

        // Current implementation ignores non-flag arguments
        assert_eq!(result.get("name"), Some(&"test".to_string()));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_parse_cli_args_numeric_values() {
        let args = vec![
            "--count".to_string(),
            "42".to_string(),
            "--price".to_string(),
            "19.99".to_string(),
            "--negative".to_string(),
            "-5".to_string(),
        ];
        let result = parse_cli_args(&args);

        assert_eq!(result.get("count"), Some(&"42".to_string()));
        assert_eq!(result.get("price"), Some(&"19.99".to_string()));
        assert_eq!(result.get("negative"), Some(&"-5".to_string()));
    }

    // Tests for implicit run command feature
    #[test]
    fn test_should_inject_run_for_plugin_command() {
        // Should inject "run" for plugin:command syntax
        assert!(should_inject_run_command(&["mis".to_string(), "claude:init".to_string()]));
        assert!(should_inject_run_command(&["mis".to_string(), "git-utils:status".to_string()]));
    }

    #[test]
    fn test_should_not_inject_run_for_explicit_subcommands() {
        // Should NOT inject for explicit subcommands
        assert!(!should_inject_run_command(&["mis".to_string(), "init".to_string()]));
        assert!(!should_inject_run_command(&["mis".to_string(), "create".to_string(), "foo".to_string()]));
        assert!(!should_inject_run_command(&["mis".to_string(), "add".to_string(), "plugin".to_string()]));
        assert!(!should_inject_run_command(&["mis".to_string(), "run".to_string(), "plugin:cmd".to_string()]));
        assert!(!should_inject_run_command(&["mis".to_string(), "info".to_string()]));
        assert!(!should_inject_run_command(&["mis".to_string(), "update".to_string()]));
    }

    #[test]
    fn test_should_not_inject_run_for_flags() {
        // Should NOT inject for flags
        assert!(!should_inject_run_command(&["mis".to_string(), "--help".to_string()]));
        assert!(!should_inject_run_command(&["mis".to_string(), "--version".to_string()]));
    }

    #[test]
    fn test_transform_args_injects_run() {
        let args = vec!["mis".to_string(), "claude:init".to_string(), "--dry-run".to_string()];
        let result = transform_args_for_implicit_run(&args);

        assert_eq!(result, vec![
            "mis".to_string(),
            "run".to_string(),
            "claude:init".to_string(),
            "--dry-run".to_string()
        ]);
    }

    #[test]
    fn test_transform_args_preserves_explicit_run() {
        let args = vec!["mis".to_string(), "run".to_string(), "claude:init".to_string()];
        let result = transform_args_for_implicit_run(&args);

        // Should be unchanged
        assert_eq!(result, args);
    }

    #[test]
    fn test_transform_args_preserves_other_subcommands() {
        let args = vec!["mis".to_string(), "init".to_string()];
        let result = transform_args_for_implicit_run(&args);

        // Should be unchanged
        assert_eq!(result, args);
    }
}
