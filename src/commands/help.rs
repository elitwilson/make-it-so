use crate::{
    config::plugins::load_plugin_manifest, constants::PLUGIN_MANIFEST_FILE, models::ArgType,
    utils::find_project_root,
};
use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;

pub fn show_help(plugin_command: &str) -> Result<()> {
    // Parse plugin:command format
    let parts: Vec<&str> = plugin_command.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid format. Use: mis info <plugin_name>:<command_name>\n\
             Example: mis info my-plugin:deploy"
        ));
    }

    let plugin_name = parts[0];
    let command_name = parts[1];

    // Validate plugin exists
    let plugin_path = validate_plugin_exists(plugin_name)?;
    let manifest_path = plugin_path.join(PLUGIN_MANIFEST_FILE);
    let plugin_manifest = load_plugin_manifest(&manifest_path)?;

    // Get the specific command
    let command = plugin_manifest
        .commands
        .get(command_name)
        .with_context(|| {
            let available_commands: Vec<String> =
                plugin_manifest.commands.keys().map(|k| k.clone()).collect();
            format!(
                "Command '{}' not found in plugin '{}'.\n\
                 Available commands: {}",
                command_name,
                plugin_name,
                available_commands.join(", ")
            )
        })?;

    // Display help information
    println!("📖 Help for {}:{}\n", plugin_name, command_name);

    // Plugin information
    println!(
        "🔌 Plugin: {} (v{})",
        plugin_manifest.plugin.name, plugin_manifest.plugin.version
    );
    if let Some(desc) = &plugin_manifest.plugin.description {
        println!("   {}", desc);
    }
    println!();

    // Command information
    if let Some(desc) = &command.description {
        println!("📝 Command: {}", desc);
    } else {
        println!("📝 Command: {}", command_name);
    }
    println!("   Script: {}", command.script);
    println!();

    // Usage line
    print!("⚡ Usage: mis run {}:{}", plugin_name, command_name);

    if let Some(args) = &command.args {
        // Add required args to usage
        for arg_name in args.required.keys() {
            print!(" --{} <value>", arg_name);
        }

        // Add optional args to usage
        for arg_name in args.optional.keys() {
            print!(" [--{} <value>]", arg_name);
        }
    } else {
        print!(" [arguments...]");
    }
    println!("\n");

    // Arguments section
    if let Some(args) = &command.args {
        if !args.required.is_empty() || !args.optional.is_empty() {
            println!("📋 Arguments:");

            // Required arguments
            if !args.required.is_empty() {
                println!("\n  🔴 Required:");
                for (name, def) in &args.required {
                    println!(
                        "    --{:15} {} ({})",
                        name,
                        def.description,
                        format_arg_type(&def.arg_type)
                    );
                }
            }

            // Optional arguments
            if !args.optional.is_empty() {
                println!("\n  🟡 Optional:");
                for (name, def) in &args.optional {
                    let default_info = def
                        .default_value
                        .as_ref()
                        .map(|d| format!(" [default: {}]", d))
                        .unwrap_or_default();
                    println!(
                        "    --{:15} {} ({}){}",
                        name,
                        def.description,
                        format_arg_type(&def.arg_type),
                        default_info
                    );
                }
            }
            println!();
        }
    } else {
        println!("ℹ️  This command accepts any arguments (no validation defined).\n");
    }

    // Examples section
    println!("💡 Examples:");
    if let Some(args) = &command.args {
        if !args.required.is_empty() {
            // Generate example with required args
            print!("   mis run {}:{}", plugin_name, command_name);
            for (name, def) in &args.required {
                let example_value = generate_example_value(&def.arg_type);
                print!(" --{} {}", name, example_value);
            }
            println!();
        }

        if !args.optional.is_empty() {
            // Generate example with optional args
            print!("   mis run {}:{}", plugin_name, command_name);
            for (name, def) in &args.required {
                let example_value = generate_example_value(&def.arg_type);
                print!(" --{} {}", name, example_value);
            }
            // Add one optional arg as example
            if let Some((name, def)) = args.optional.iter().next() {
                let example_value = generate_example_value(&def.arg_type);
                print!(" --{} {}", name, example_value);
            }
            println!();
        }
    }

    // Show dry run example
    println!(
        "   mis run {}:{} --dry-run  # Preview without executing",
        plugin_name, command_name
    );
    println!();

    // Plugin configuration hint - Note: config is now in config.toml
    println!("Plugin configuration can be customized in config.toml");
    println!();

    // Dependencies information
    if !plugin_manifest.deno_dependencies.is_empty() {
        println!("📦 External Dependencies:");
        for (name, url) in &plugin_manifest.deno_dependencies {
            println!("   {} → {}", name, url);
        }
        println!();
    }

    // Custom instructions
    if let Some(instructions) = &command.instructions {
        println!("📋 Instructions:");
        // Split by lines and indent each line
        for line in instructions.lines() {
            println!("   {}", line);
        }
        println!();
    }

    Ok(())
}

pub fn show_all_plugins() -> Result<()> {
    let root = find_project_root().ok_or_else(|| anyhow::anyhow!("Failed to find project root"))?;

    if !root.exists() {
        anyhow::bail!(
            "🛑 You're not inside a Make It So project.\n\
             → Make sure you're in the project root (where .makeitso/ lives).\n\
             → If you haven't set it up yet, run `mis init`."
        );
    }

    let plugins_dir = root.join(".makeitso/plugins");

    if !plugins_dir.exists() {
        println!("📋 Available Plugins and Commands\n");
        println!("🛑 No plugins directory found (.makeitso/plugins).");
        println!("→ Create your first plugin with: mis create <plugin_name>");
        return Ok(());
    }

    let mut plugins = Vec::new();
    for entry in std::fs::read_dir(&plugins_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                let plugin_path = entry.path();
                let manifest_path = plugin_path.join(PLUGIN_MANIFEST_FILE);

                if manifest_path.exists() {
                    match load_plugin_manifest(&manifest_path) {
                        Ok(manifest) => {
                            plugins.push((name.to_string(), manifest));
                        }
                        Err(_) => {
                            println!("⚠️  Warning: Failed to load manifest for plugin '{}'", name);
                        }
                    }
                } else {
                    println!("⚠️  Warning: Plugin '{}' missing manifest.toml", name);
                }
            }
        }
    }

    if plugins.is_empty() {
        println!("📋 Available Plugins and Commands\n");
        println!("🛑 No valid plugins found in .makeitso/plugins.");
        println!("→ Create your first plugin with: mis create <plugin_name>");
        return Ok(());
    }

    // Sort plugins by name
    plugins.sort_by(|a, b| a.0.cmp(&b.0));

    println!("📋 Available Plugins and Commands\n");

    for (plugin_name, manifest) in &plugins {
        println!("🔌 {}", plugin_name);
        if let Some(desc) = &manifest.plugin.description {
            println!("   {}", desc);
        }
        println!("   Version: {}", manifest.plugin.version);

        if manifest.commands.is_empty() {
            println!("   └─ No commands defined");
        } else {
            let mut commands: Vec<_> = manifest.commands.iter().collect();
            commands.sort_by_key(|(name, _)| *name);

            for (i, (cmd_name, cmd)) in commands.iter().enumerate() {
                let is_last = i == commands.len() - 1;
                let prefix = if is_last { "   └─" } else { "   ├─" };

                if let Some(desc) = &cmd.description {
                    println!("{} {} - {}", prefix, cmd_name, desc);
                } else {
                    println!("{} {}", prefix, cmd_name);
                }
            }
        }
        println!();
    }

    println!("💡 Usage:");
    println!("   mis run <plugin>:<command>     # Run a command");
    println!("   mis info <plugin>:<command>    # Get detailed help for a command");
    println!("   mis create <plugin>            # Create a new plugin");
    println!();

    Ok(())
}

fn validate_plugin_exists(plugin_name: &str) -> Result<PathBuf> {
    let root = find_project_root().ok_or_else(|| anyhow::anyhow!("Failed to find project root"))?;

    if !root.exists() {
        anyhow::bail!(
            "🛑 You're not inside a Make It So project.\n\
             → Make sure you're in the project root (where .makeitso/ lives).\n\
             → If you haven't set it up yet, run `mis init`."
        );
    }

    let plugin_path = root.join(".makeitso/plugins").join(plugin_name);

    if !plugin_path.exists() {
        anyhow::bail!(
            "🛑 Plugin '{}' not found in .makeitso/plugins.\n\
             → Available plugins: {}\n\
             → To install a plugin, run `mis add {}`\n\
             → To create a plugin, run `mis create {}`",
            plugin_name,
            list_available_plugins()?,
            plugin_name,
            plugin_name
        );
    }

    let manifest_path = plugin_path.join(PLUGIN_MANIFEST_FILE);
    if !manifest_path.exists() {
        anyhow::bail!(
            "🛑 manifest.toml not found for plugin '{}'.\n\
             → Expected to find: {}\n\
             → The plugin may be corrupted.",
            plugin_name,
            manifest_path.display()
        );
    }

    Ok(plugin_path)
}

fn list_available_plugins() -> Result<String> {
    let root = find_project_root().ok_or_else(|| anyhow::anyhow!("Failed to find project root"))?;

    let plugins_dir = root.join(".makeitso/plugins");

    if !plugins_dir.exists() {
        return Ok("none".to_string());
    }

    let mut plugins = Vec::new();
    for entry in std::fs::read_dir(plugins_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                plugins.push(name.to_string());
            }
        }
    }

    if plugins.is_empty() {
        Ok("none".to_string())
    } else {
        plugins.sort();
        Ok(plugins.join(", "))
    }
}

fn format_arg_type(arg_type: &ArgType) -> &'static str {
    match arg_type {
        ArgType::String => "string",
        ArgType::Boolean => "boolean",
        ArgType::Integer => "integer",
        ArgType::Float => "float",
    }
}

fn generate_example_value(arg_type: &ArgType) -> &'static str {
    match arg_type {
        ArgType::String => "\"value\"",
        ArgType::Boolean => "true",
        ArgType::Integer => "5",
        ArgType::Float => "3.14",
    }
}
