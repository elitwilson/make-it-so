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
    fn build(&self, ctx: &DeploymentContext, raw_config: &toml::Value) -> Result<()> {
        // 💾 Step 1: Cache Deno deps (if defined)
        cache_deno_dependencies(raw_config)?;

        let json = serde_json::to_string(ctx)?;
        let mut child = Command::new(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("Failed to run plugin: {}", self.path.display()))?;

        child
            .stdin
            .as_mut()
            .context("Failed to open stdin for plugin")?
            .write_all(json.as_bytes())?;

        let status = child.wait()?;
        if !status.success() {
            return Err(anyhow!("Plugin exited with non-zero status"));
        }

        Ok(())
    }
}