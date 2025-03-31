use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    config::{load_mis_config, plugins::load_plugin_manifest},
    constants::PLUGIN_MANIFEST_FILE,
    integrations::deno::cache_deno_dependencies,
    models::{ExecutionContext, PluginManifest, PluginMeta},
};
use anyhow::{Context, Result};

pub fn run_cmd(
    plugin_name: String,
    command_name: &str,
    dry_run: bool,
    args: HashMap<String, String>,
) -> Result<()> {
    let plugin_path = PathBuf::from(".makeitso/plugins").join(&plugin_name);
    let manifest_path = plugin_path.join(PLUGIN_MANIFEST_FILE);
    let plugin_manifest = load_plugin_manifest(&manifest_path)?;

    let env_arg = args
        .get("env")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing required argument: --env"))?;

    let version_arg = args
        .get("version")
        .cloned()
        .unwrap_or_else(|| "0.0.0".to_string()); // default fallback or bail

    let mut plugin_args = HashMap::new();

    plugin_args.insert("env".to_string(), serde_json::Value::String(env_arg));
    plugin_args.insert(
        "version".to_string(),
        serde_json::Value::String(version_arg),
    );

    if dry_run {
        plugin_args.insert("dry_run".to_string(), serde_json::Value::Bool(true));
    }

    let project_root = std::env::current_dir()?.to_string_lossy().to_string();
    // let env_vars = std::env::vars().collect::<HashMap<_, _>>();
    let meta = PluginMeta {
        plugin_name: plugin_name.clone(),
        plugin_description: plugin_manifest.plugin.plugin_description.clone(),
        plugin_version: "todo".to_string(), // figure out how to get this
    };

    let ctx = ExecutionContext::from_parts(
        plugin_args,
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
