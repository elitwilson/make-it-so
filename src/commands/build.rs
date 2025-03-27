use anyhow::Result;
use crate::{config::load_shipwreck_service_config, models::DeploymentContext, strategy::build::get_build_strategy};

pub fn run_build(service: String, env: String, version: String, dry_run: bool) -> Result<()> {
    let (config, _config_path) = load_shipwreck_service_config(&service)?;
    
    let strategy_name = config
        .deploy_strategy
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("No deploy_strategy defined in service config"))?;

    println!("Using deploy strategy: {}", strategy_name);

    let ctx = DeploymentContext::from_config(&config, &service, &env, &version, dry_run)?;

    let strategy = get_build_strategy(strategy_name)?;
    strategy.build(&ctx)?;

    Ok(())
}