pub mod deploy;
pub mod campsites;
pub mod build;
pub mod plugin;


// pub mod campsites;

// use crate::models::ServiceConfig;
// use anyhow::Result;
// pub use self::campsites::CampsitesDeployStrategy;

// pub trait DeployStrategy {
//   fn deploy(&self, config: &ServiceConfig, env: &str, version: &str, dry_run: bool) -> Result<()>;
// }

// pub fn get_deploy_strategy(name: &str) -> Box<dyn DeployStrategy> {
//   match name {
//     "campsites" => Box::new(CampsitesDeployStrategy),
//     _ => panic!("Unknown deploy strategy: {}", name),
//   }
// }