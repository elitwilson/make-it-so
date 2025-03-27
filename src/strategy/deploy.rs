use anyhow::{Result, anyhow};
use crate::models::DeploymentContext;
use crate::strategy::campsites::CampsitesDeployStrategy;

pub trait DeployStrategy {
  fn deploy(&self, ctx: &DeploymentContext) -> Result<()>;
}

pub fn get_deploy_strategy(name: &str) -> Result<Box<dyn DeployStrategy>> {
    match name {
        "campsites" => Ok(Box::new(CampsitesDeployStrategy)),
        _ => Err(anyhow!("Unknown deploy strategy: {}", name)),
    }
}
