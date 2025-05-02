use std::path::{Path, PathBuf};

use tempfile::tempdir;

use crate::{config::load_mis_config, git_utils::shallow_clone_repo};

pub fn add_plugin(plugins: Vec<String>, dry_run: bool, registry: Option<String>) -> anyhow::Result<()> {
    let (config, _, _) = load_mis_config().unwrap();
    println!("Registry items: {}", config.registry.iter().count());

    if let Some(reg) = &registry {
        println!("Custom Registry Provided: {}", reg);
    }

    // Get the registry sources from the config
    let mut sources: &[String] = &[];
    if let Some(reg) = &config.registry {
        sources = &reg.sources;
    }

    if sources.is_empty() {
        println!("No sources found in the registry section of mis.toml.");
    } else {
        println!("Sources found in the registry:");
        for source in sources {
            println!("- {}", source);
        }
    }

    println!("Args: {:?}", plugins.iter().collect::<Vec<_>>());
    println!("Dry run: {}", dry_run);

    Ok(())
}

fn plugin_exists_in_project(name: String) -> bool {
    let plugin_path = Path::new(".makeitso/plugins").join(name);
    plugin_path.exists() && plugin_path.is_dir()
}

fn plugin_exists_in_registries(name: &str, registries: &[String]) -> bool {
    for registry_url in registries {
        let tmp_dir = match tempdir() {
            Ok(dir) => dir,
            Err(_) => continue,
        };

        let tmp_path = tmp_dir.path().to_string_lossy().to_string();

        if shallow_clone_repo(registry_url.clone(), tmp_path.clone()).is_err() {
            continue;
        }

        let plugin_path: PathBuf = tmp_dir.path().join(name);
        if plugin_path.exists() && plugin_path.is_dir() {
            return true;
        }
    }

    false
}

// fn read_registry_config() -> Result<RegistryConfig, Box<dyn std::error::Error>> {
//     let config_path = "path/to/your/config.toml"; // Update this path
//     let mut file = File::open(config_path)?;
//     let mut contents = String::new();
//     file.read_to_string(&mut contents)?;

//     let config: RegistryConfig = toml::de::from_str(&contents)?;
//     Ok(config)
// }