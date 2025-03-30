use std::path::PathBuf;

use crate::integrations::deno::install_deno;
use crate::{cli::prompt_user, models::ExecutionContext};
use crate::strategy::campsites::CampsitesBuildStrategy;
use crate::strategy::plugin::PluginBuildStrategy;
use anyhow::{Result, anyhow};

use super::utils::is_deno_installed;

pub trait BuildStrategy {
    fn build(&self, ctx: &ExecutionContext, raw_config: &toml::Value) -> Result<()>;
}

pub fn get_build_strategy(name: &str, config: &toml::Value) -> Result<Box<dyn BuildStrategy>> {
    match name {
        "campsites" => Ok(Box::new(CampsitesBuildStrategy::new())),
        "deno" => {
            if !is_deno_installed() {
                println!("❌ Deno is not installed.");
            
                if prompt_user("Would you like to install Deno now?")? {
                    install_deno()?;
                    println!("Deno installation is not implemented yet.");
                } else {
                    return Err(anyhow!("Deno is required to run this strategy."));
                }
            }

            let plugin_entrypoint = config
                .get("plugin_entrypoint")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    anyhow!("Missing `plugin_entrypoint` in strategy_config for plugin strategy")
                })?;

            let plugin_path = PathBuf::from(".makeitso/plugins").join(plugin_entrypoint);

            if !plugin_path.exists() {
                return Err(anyhow!(
                    "Plugin script `{plugin_entrypoint}` not found at {}",
                    plugin_path.display()
                ));
            }

            // Optional: Validate extension
            if let Some(ext) = plugin_path.extension() {
                if ext != "js" && ext != "ts" {
                    return Err(anyhow!("Plugin must be a .js or .ts file"));
                }
            }

            Ok(Box::new(PluginBuildStrategy::new(plugin_path)))
        }
        other => Err(anyhow!("Unknown strategy `{other}`")),
    }
}
