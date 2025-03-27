use std::path::PathBuf;

use crate::models::DeploymentContext;
use crate::strategy::campsites::CampsitesBuildStrategy;
use crate::strategy::plugin::PluginBuildStrategy;
use anyhow::{Result, anyhow};

pub trait BuildStrategy {
    fn build(&self, ctx: &DeploymentContext) -> Result<()>;
}

pub fn get_build_strategy(name: &str) -> Result<Box<dyn BuildStrategy>> {
    match name {
        "campsites" => Ok(Box::new(CampsitesBuildStrategy)),
        _ => {
            let plugin_path = PathBuf::from(format!(".shipwreck/{}.js", name));
            if plugin_path.exists() {
                Ok(Box::new(PluginBuildStrategy::new(plugin_path)) as Box<dyn BuildStrategy>)
            } else {
                Err(anyhow!("Unknown build strategy: {}", name))
            }
        }
    }
}
