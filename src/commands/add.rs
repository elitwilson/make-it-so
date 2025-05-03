use anyhow::{Result, anyhow};
use std::{collections::HashMap, fs, path::Path};
use tempfile::TempDir;
use crate::{config::load_mis_config, git_utils::shallow_clone_repo};

pub fn add_plugin(plugins: Vec<String>, dry_run: bool, registry: Option<String>) -> anyhow::Result<()> {
    let (config, _, _) = load_mis_config().unwrap();
    println!("Registry items: {}", config.registry.iter().count());

    if let Some(reg) = &registry {
        println!("Custom Registry Provided: {}", reg);
    }

    // Get the registry sources from the config
    let sources: Vec<String> = if let Some(reg_override) = &registry {
        vec![reg_override.clone()]
    } else if let Some(reg) = &config.registry {
        reg.sources.clone()
    } else {
        vec![]
    };
    
    if sources.is_empty() {
        return Err(anyhow!(
            "No registry sources found. Add a [registry] section to mis.toml or pass --registry <url>."
        ));
    }

    if sources.is_empty() {
        return Err(anyhow!("No sources found in the registry section of mis.toml and no registry provided via --registry flag."));
    }

    let cloned_repos = temp_clone_repositories(&sources)?;

    // Loop through the plugin args and handle them
    for plugin in &plugins {
        let plugin_name = &plugin;

        // Check if the plugin exists in the project
        if plugin_exists_in_project(plugin_name) {
            println!("❌ Plugin {} already exists in the project.", plugin_name);
            continue;
        }

        if !plugin_exists_in_registries(&plugin_name, &cloned_repos) {
            println!("❌ Plugin {} not found in any registry.", plugin_name);
            continue;
        }

        // If the plugin exists in the registries, add it to the project
        for (url, temp_dir) in &cloned_repos {
            if dry_run {
                println!(
                    "📝 Would install plugin '{}' from {}",
                    plugin_name, url
                );
            } else {
                install_plugin_from_clone(plugin_name, temp_dir, url)?;
            }
        }        
    }

    println!("Args: {:?}", plugins.iter().collect::<Vec<_>>());
    println!("Dry run: {}", dry_run);

    Ok(())
}

fn plugin_exists_in_project(name: &String) -> bool {
    let plugin_path = Path::new(".makeitso/plugins").join(name);
    plugin_path.exists() && plugin_path.is_dir()
}

fn plugin_exists_in_registries(plugin_name: &str, cloned: &HashMap<String, TempDir>) -> bool {
    for (_registry_url, temp_dir) in cloned {
        let plugin_path = temp_dir.path().join(plugin_name);
        if plugin_path.exists() && plugin_path.is_dir() {
            return true;
        }
    }

    false
}

fn temp_clone_repositories(registries: &[String]) -> Result<HashMap<String, TempDir>> {
    let mut registry_map = HashMap::new();

    for registry_url in registries {
        let tmp_dir = TempDir::new()?;
        let tmp_path = tmp_dir.path().to_string_lossy().to_string();

        if let Err(e) = shallow_clone_repo(registry_url.clone(), tmp_path) {
            return Err(anyhow!("❌ Failed to clone {}: {}", registry_url, e));
        }

        registry_map.insert(registry_url.clone(), tmp_dir); // keep ownership of TempDir
    }

    Ok(registry_map)
}

pub fn install_plugin_from_clone(
    plugin_name: &str,
    source_tempdir: &TempDir,
    registry_url: &str,
) -> Result<()> {
    let source_path = source_tempdir.path().join(plugin_name);
    if !source_path.exists() || !source_path.is_dir() {
        return Err(anyhow!(
            "Plugin '{}' not found in registry clone at {}",
            plugin_name,
            registry_url
        ));
    }

    let dest_root = Path::new(".makeitso/plugins");
    let dest_path = dest_root.join(plugin_name);

    // Ensure the destination parent dir exists
    fs::create_dir_all(dest_root)?;

    // Check if plugin already exists
    if dest_path.exists() {
        return Err(anyhow!(
            "Plugin '{}' already exists in your project. Use --force to overwrite.",
            plugin_name
        ));
    }

    // Copy directory
    copy_dir_recursive(&source_path, &dest_path)?;

    println!(
        "✅ Installed plugin '{}' from {} → {}",
        plugin_name,
        registry_url,
        dest_path.display()
    );

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let target_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &target_path)?;
        } else {
            fs::copy(&entry_path, &target_path)?;
        }
    }

    Ok(())
}
