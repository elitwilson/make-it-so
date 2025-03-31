use crate::models::ExecutionContext;
use crate::strategy::deploy::get_deploy_strategy;
use crate::config::load_shipwreck_config;
use anyhow::Result;

// pub fn run_deploy(service: String, env: String, version: String, dry_run: bool) -> Result<()> {
//     let (config, _config_path, _raw_config) = load_shipwreck_config()?;
    
//     // let strategy_name = config
//     //     .deploy_strategy
//     //     .as_deref()
//     //     .ok_or_else(|| anyhow::anyhow!("No deploy_strategy defined in service config"))?;

//     let ctx = ExecutionContext::from_config(&config, &service, dry_run)?;

//     // let strategy = get_deploy_strategy(strategy_name)?;
//     // strategy.deploy(&ctx)?;

//     Ok(())
// }
