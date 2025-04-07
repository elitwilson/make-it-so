use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    config::plugins::load_plugin_manifest,
    constants::PLUGIN_MANIFEST_FILE,
    integrations::deno::cache_deno_dependencies,
    models::{ExecutionContext, PluginManifest, PluginMeta}, utils::find_project_root,
};
use anyhow::{Context, Result};

pub fn run_cmd(
    plugin_name: String,
    command_name: &str,
    dry_run: bool,
    plugin_raw_args: HashMap<String, String>,
) -> Result<()> {
    let plugin_path = validate_plugin_exists(&plugin_name)?;
    let manifest_path = plugin_path.join(PLUGIN_MANIFEST_FILE);    
    let plugin_manifest = load_plugin_manifest(&manifest_path)?;

    // let mut plugin_args = HashMap::new();
    let mut plugin_args: serde_json::Map<String, serde_json::Value> = plugin_raw_args
        .into_iter()
        .map(|(k, v)| {
            let value = match v.as_str() {
                "true" => serde_json::Value::Bool(true),
                "false" => serde_json::Value::Bool(false),
                _ => serde_json::Value::String(v),
            };
            (k, value)
        })
        .collect();

    if dry_run {
        plugin_args.insert("dry_run".to_string(), serde_json::Value::Bool(true));
    }

    let project_root = std::env::current_dir()?.to_string_lossy().to_string();
    // let env_vars = std::env::vars().collect::<HashMap<_, _>>();
    let meta = PluginMeta {
        name: plugin_name.clone(),
        description: plugin_manifest.plugin.description.clone(),
        version: "todo".to_string(), // figure out how to get this
    };

    let ctx = ExecutionContext::from_parts(
        plugin_args.into_iter().collect::<HashMap<_, _>>(),
        plugin_manifest.user_config.clone(),
        project_root,
        meta,
        dry_run,
    )?;

    let command = plugin_manifest
        .commands
        .get(command_name)
        .with_context(|| {
            format!(
                "Command '{}' not found in plugin '{}'",
                command_name, plugin_name
            )
        })?;

    execute_plugin(&plugin_path, &command.script, &ctx, &plugin_manifest)?;

    Ok(())
}

fn validate_plugin_exists(plugin_name: &str) -> Result<PathBuf> {
    let root = find_project_root()
        .ok_or_else(|| anyhow::anyhow!("Failed to find project root"))?;

    if !root.exists() {
        anyhow::bail!(
            "🛑 You're not inside a Make It So project.\n\
             → Make sure you're in the project root (where .makeitso/ lives).\n\
             → If you haven't set it up yet, run `mis init`."
        );
    }

    let plugin_path = root.join(".makeitso/plugins").join(plugin_name);
    println!("Plugin path: {}", plugin_path.display());

    if !plugin_path.exists() {
        anyhow::bail!(
            "🛑 Plugin '{}' not found in .makeitso/plugins.\n\
             → Did you run `mis create plugin {}`?",
            plugin_name,
            plugin_name
        );
    }

    let manifest_path = plugin_path.join("plugin.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "🛑 plugin.toml not found for plugin '{}'.\n\
             → Expected to find: {}\n\
             → Did something delete it?",
            plugin_name,
            manifest_path.display()
        );
    }

    Ok(plugin_path)
}

pub fn execute_plugin(
    dir: &PathBuf,
    script_file_name: &str,
    ctx: &ExecutionContext,
    plugin_config: &PluginManifest,
) -> Result<()> {
    // Cache any [deno_dependencies] first
    cache_deno_dependencies(&plugin_config.deno_dependencies)?;

    // Serialize the context into JSON to pass to the plugin
    let json = serde_json::to_string_pretty(ctx)?;

    let path_and_file = dir.join(script_file_name);

    // Spawn the plugin with Deno
    let mut child = Command::new("deno")
        .arg("run")
        .arg("--allow-all") // !!This needs to be scoped down later!!
        .arg(path_and_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to run plugin: {}", script_file_name))?;

    // Pipe context JSON into plugin’s stdin
    child
        .stdin
        .as_mut()
        .context("Failed to open stdin for plugin")?
        .write_all(json.as_bytes())?;

    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Plugin exited with non-zero status"));
    }

    Ok(())
}
