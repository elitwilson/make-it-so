use super::build::BuildStrategy;
use crate::{integrations::deno::cache_deno_dependencies, models::DeploymentContext};
use anyhow::{Context, Result, anyhow};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

pub struct PluginBuildStrategy {
    pub path: PathBuf,
}

impl PluginBuildStrategy {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl BuildStrategy for PluginBuildStrategy {
    fn build(&self, ctx: &DeploymentContext, raw_service_config: &toml::Value) -> Result<()> {
        // Cache any [deno_dependencies] first
        cache_deno_dependencies(raw_service_config)?;

        // Serialize the context into JSON to pass to the plugin
        let json = serde_json::to_string_pretty(ctx)?;

        // Spawn the plugin with Deno
        let mut child = Command::new("deno")
            .arg("run")
            .arg("--allow-all") // you can scope this down later
            .arg(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("Failed to run plugin: {}", self.path.display()))?;

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
    // fn build(&self, ctx: &DeploymentContext, full_config: &toml::Value) -> Result<()> {
    //     println!("full config: {:?}", full_config);

    //     // 💾 Step 1: Cache Deno deps (if defined)
    //     cache_deno_dependencies(full_config)?;

    //     let json = serde_json::to_string(ctx)?;
    //     let mut child = Command::new(&self.path)
    //         .stdin(Stdio::piped())
    //         .stdout(Stdio::inherit())
    //         .stderr(Stdio::inherit())
    //         .spawn()
    //         .with_context(|| format!("Failed to run plugin: {}", self.path.display()))?;

    //     child
    //         .stdin
    //         .as_mut()
    //         .context("Failed to open stdin for plugin")?
    //         .write_all(json.as_bytes())?;

    //     let status = child.wait()?;
    //     if !status.success() {
    //         return Err(anyhow!("Plugin exited with non-zero status"));
    //     }

    //     Ok(())
    // }
}
