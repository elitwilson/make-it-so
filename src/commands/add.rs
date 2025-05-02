use std::{collections::HashMap, fs::File, io::Read};

use crate::{config::load_mis_config, models::RegistryConfig};

pub fn add_plugin(args: HashMap<String, String>, dry_run: bool) -> anyhow::Result<()> {
    let (config, _, _) = load_mis_config().unwrap();
    println!("Registry items: {}", config.registry.iter().count());
    let mut sources: Vec<String> = vec![];

    if let Some(reg) = &config.registry {
        sources = reg.sources.clone();
    }

    if sources.is_empty() {
        println!("No sources found in the registry.");
    } else {
        println!("Sources found in the registry:");
        for source in sources {
            println!("- {}", source);
        }
    }

    println!("Args: {:?}", args);
    println!("Dry run: {}", dry_run);

    Ok(())
}

// fn read_registry_config() -> Result<RegistryConfig, Box<dyn std::error::Error>> {
//     let config_path = "path/to/your/config.toml"; // Update this path
//     let mut file = File::open(config_path)?;
//     let mut contents = String::new();
//     file.read_to_string(&mut contents)?;

//     let config: RegistryConfig = toml::de::from_str(&contents)?;
//     Ok(config)
// }