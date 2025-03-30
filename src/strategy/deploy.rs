use anyhow::{Result, anyhow};
use crate::models::ExecutionContext;
use crate::strategy::campsites::CampsitesDeployStrategy;

pub trait DeployStrategy {
  fn deploy(&self, ctx: &ExecutionContext) -> Result<()>;
}

pub fn get_deploy_strategy(name: &str) -> Result<Box<dyn DeployStrategy>> {
    match name {
        "campsites" => Ok(Box::new(CampsitesDeployStrategy)),
        _ => Err(anyhow!("Unknown deploy strategy: {}", name)),
    }
}
