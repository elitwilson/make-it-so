use anyhow::Result;
use crate::{config::load_shipwreck_config, models::ExecutionContext, strategy::build::get_build_strategy};

// pub fn run_build(service: String, env: String, version: String, dry_run: bool) -> Result<()> {
//     let (service_config, _config_path, full_config) = load_shipwreck_config()?;
//     // println!("Full config: {:?}", full_config);
    
//     // let strategy_name = service_config
//     //     .deploy_strategy
//     //     .as_deref()
//     //     .ok_or_else(|| anyhow::anyhow!("No deploy_strategy defined in service config"))?;

//     let raw_service_config = service_config
//         .strategy_config
//         .as_ref()
//         .ok_or_else(|| anyhow::anyhow!("Missing strategy_config"))?;


//     let ctx = ExecutionContext::from_config(&service_config, &service, dry_run)?;

//     // let strategy = get_build_strategy(strategy_name, &raw_service_config)?;
//     // strategy.build(&ctx, &full_config)?;

//     Ok(())
// }