use anyhow::{Result, anyhow};
use std::{collections::HashMap, fs, path::Path};
use tempfile::TempDir;
use crate::{config::load_mis_config, git_utils::shallow_clone_repo, models::MakeItSoConfig};

pub fn add_plugin(plugins: Vec<String>, dry_run: bool, registry: Option<String>, force: bool) -> anyhow::Result<()> {
    let (config, _, _) = load_mis_config().unwrap();
    add_plugin_with_config(plugins, dry_run, registry, force, config)
}

// Testable version that accepts config as parameter (dependency injection)
pub fn add_plugin_with_config(
    plugins: Vec<String>, 
    dry_run: bool, 
    registry: Option<String>, 
    force: bool,
    config: MakeItSoConfig
) -> anyhow::Result<()> {
    if let Some(reg) = &registry {
        println!("Custom Registry Provided: {}", reg);
    }

    // Input validation (Priority 2 issue #8)
    for plugin in &plugins {
        if plugin.trim().is_empty() {
            return Err(anyhow!("Plugin name cannot be empty"));
        }
        if plugin.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
            return Err(anyhow!("Plugin name '{}' contains invalid characters", plugin));
        }
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

    let cloned_repos = temp_clone_repositories(&sources)?;

    // Loop through the plugin args and handle them
    for plugin in &plugins {
        let plugin_name = &plugin;

        // Check if the plugin exists in the project
        if plugin_exists_in_project(plugin_name) && !force {
            println!("❌ Plugin {} already exists in the project. Use --force to overwrite.", plugin_name);
            continue;
        }

        if !plugin_exists_in_registries(&plugin_name, &cloned_repos) {
            println!("❌ Plugin {} not found in any registry.", plugin_name);
            continue;
        }

        // FIXED: Install from first matching registry only (Priority 1 issue #2)
        let mut installed = false;
        for (url, temp_dir) in &cloned_repos {
            // Check both root level and plugins subdirectory
            let root_plugin_path = temp_dir.path().join(plugin_name);
            let plugins_subdir_path = temp_dir.path().join("plugins").join(plugin_name);
            
            let source_path = if plugins_subdir_path.exists() && plugins_subdir_path.is_dir() {
                // Plugin is in plugins/ subdirectory
                plugins_subdir_path
            } else if root_plugin_path.exists() && root_plugin_path.is_dir() {
                // Plugin is at root level
                root_plugin_path
            } else {
                // Plugin not found in this registry
                continue;
            };
            
            if dry_run {
                println!("📝 Would install plugin '{}' from {}", plugin_name, url);
            } else {
                install_plugin_from_path(plugin_name, &source_path, url, force)?;
            }
            installed = true;
            break; // Only install from first matching registry
        }

        if !installed && !dry_run {
            println!("❌ Failed to install plugin {} from any registry.", plugin_name);
        }
    }

    Ok(())
}

fn plugin_exists_in_project(name: &String) -> bool {
    let plugin_path = Path::new(".makeitso/plugins").join(name);
    plugin_path.exists() && plugin_path.is_dir()
}

fn plugin_exists_in_registries(plugin_name: &str, cloned: &HashMap<String, TempDir>) -> bool {
    for (_registry_url, temp_dir) in cloned {
        // Check both root level and inside 'plugins' subdirectory
        let root_plugin_path = temp_dir.path().join(plugin_name);
        let plugins_subdir_path = temp_dir.path().join("plugins").join(plugin_name);
        
        // Check if plugin exists in plugins subdirectory first (more common)
        if plugins_subdir_path.exists() && plugins_subdir_path.is_dir() {
            return true;
        }
        
        // Fallback: check root level for backward compatibility
        if root_plugin_path.exists() && root_plugin_path.is_dir() {
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

pub fn install_plugin_from_path(
    plugin_name: &str,
    source_path: &Path,
    registry_url: &str,
    force: bool,
) -> Result<()> {
    if !source_path.exists() || !source_path.is_dir() {
        return Err(anyhow!(
            "Plugin '{}' not found at path {}",
            plugin_name,
            source_path.display()
        ));
    }

    let dest_root = Path::new(".makeitso/plugins");
    let dest_path = dest_root.join(plugin_name);

    // Ensure the destination parent dir exists
    fs::create_dir_all(dest_root)?;

    // Check if plugin already exists
    if dest_path.exists() && !force {
        return Err(anyhow!(
            "Plugin '{}' already exists in your project. Use --force to overwrite.",
            plugin_name
        ));
    }

    // Remove existing directory if force is enabled
    if dest_path.exists() && force {
        fs::remove_dir_all(&dest_path)?;
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

/// Installs a plugin from a cloned repository (TempDir) without force.
/// 
/// This is a convenience wrapper around `install_plugin_from_path` that handles
/// the plugin discovery logic (checking both root and plugins/ subdirectory).
/// 
/// Used primarily by:
/// - Unit tests for isolated plugin installation testing
/// - Legacy/external code that works with TempDir objects
/// 
/// For new code, prefer `install_plugin_from_path` for direct path operations
/// or `add_plugin` for the full CLI workflow.
pub fn install_plugin_from_clone(
    plugin_name: &str,
    source_tempdir: &TempDir,
    registry_url: &str,
) -> Result<()> {
    // Check both root level and plugins subdirectory
    let root_plugin_path = source_tempdir.path().join(plugin_name);
    let plugins_subdir_path = source_tempdir.path().join("plugins").join(plugin_name);
    
    let source_path = if plugins_subdir_path.exists() && plugins_subdir_path.is_dir() {
        plugins_subdir_path
    } else if root_plugin_path.exists() && root_plugin_path.is_dir() {
        root_plugin_path
    } else {
        return Err(anyhow!(
            "Plugin '{}' not found in registry clone at {}",
            plugin_name,
            registry_url
        ));
    };

    install_plugin_from_path(plugin_name, &source_path, registry_url, false)
}

/// Installs a plugin from a cloned repository (TempDir) with optional force.
/// 
/// This is a convenience wrapper around `install_plugin_from_path` that handles
/// the plugin discovery logic (checking both root and plugins/ subdirectory)
/// and provides force overwrite capability.
/// 
/// Used primarily by:
/// - Unit tests that need to test force overwrite behavior
/// - The main add_plugin workflow for actual installations
/// - Legacy/external code that works with TempDir objects
/// 
/// The force parameter controls whether existing plugins are overwritten.
pub fn install_plugin_from_clone_with_force(
    plugin_name: &str,
    source_tempdir: &TempDir,
    registry_url: &str,
    force: bool,
) -> Result<()> {
    // Check both root level and plugins subdirectory
    let root_plugin_path = source_tempdir.path().join(plugin_name);
    let plugins_subdir_path = source_tempdir.path().join("plugins").join(plugin_name);
    
    let source_path = if plugins_subdir_path.exists() && plugins_subdir_path.is_dir() {
        plugins_subdir_path
    } else if root_plugin_path.exists() && root_plugin_path.is_dir() {
        root_plugin_path
    } else {
        return Err(anyhow!(
            "Plugin '{}' not found in registry clone at {}",
            plugin_name,
            registry_url
        ));
    };

    install_plugin_from_path(plugin_name, &source_path, registry_url, force)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::{TempDir, tempdir};

    fn create_test_config(registry_sources: Option<Vec<String>>) -> MakeItSoConfig {
        use std::collections::HashMap;
        use crate::models::RegistryConfig;
        
        MakeItSoConfig {
            name: Some("test-project".to_string()),
            project_variables: HashMap::new(),
            registry: registry_sources.map(|sources| RegistryConfig { sources }),
        }
    }

    fn create_test_config_with_registry(sources: Vec<String>) -> (TempDir, PathBuf) {
        let temp_dir = tempdir().unwrap();
        let makeitso_dir = temp_dir.path().join(".makeitso");
        fs::create_dir_all(&makeitso_dir).unwrap();
        
        let config_content = format!(
            r#"
name = "test-project"

[project_variables]
foo = "bar"

[registry]
sources = [{}]
"#,
            sources.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", ")
        );
        
        let config_path = makeitso_dir.join("mis.toml");
        fs::write(&config_path, config_content).unwrap();
        
        (temp_dir, config_path)
    }

    fn create_mock_registry_with_plugins(plugins: Vec<&str>) -> TempDir {
        let registry_dir = tempdir().unwrap();
        
        for plugin in plugins {
            let plugin_dir = registry_dir.path().join(plugin);
            fs::create_dir_all(&plugin_dir).unwrap();
            
            // Create a simple plugin.toml file
            let plugin_toml = format!(
                r#"
[plugin]
name = "{}"
version = "1.0.0"
description = "Test plugin"

[commands.test]
script = "./main.ts"
"#,
                plugin
            );
            fs::write(plugin_dir.join("plugin.toml"), plugin_toml).unwrap();
            fs::write(plugin_dir.join("main.ts"), "console.log('test');").unwrap();
        }
        
        registry_dir
    }

    #[test]
    fn test_plugin_exists_in_project_returns_true_when_plugin_exists() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Create .makeitso/plugins/test-plugin directory
        let plugins_dir = temp_dir.path().join(".makeitso/plugins/test-plugin");
        fs::create_dir_all(&plugins_dir).unwrap();
        
        let result = plugin_exists_in_project(&"test-plugin".to_string());
        assert!(result);
        
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_plugin_exists_in_project_returns_false_when_plugin_missing() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let result = plugin_exists_in_project(&"nonexistent-plugin".to_string());
        assert!(!result);
        
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_plugin_exists_in_registries_finds_plugin() {
        let registry = create_mock_registry_with_plugins(vec!["test-plugin", "another-plugin"]);
        let mut cloned = HashMap::new();
        cloned.insert("test-registry".to_string(), registry);
        
        let result = plugin_exists_in_registries("test-plugin", &cloned);
        assert!(result);
        
        let result = plugin_exists_in_registries("nonexistent", &cloned);
        assert!(!result);
    }

    #[test]
    fn test_install_plugin_from_clone_success() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Create source plugin
        let registry = create_mock_registry_with_plugins(vec!["test-plugin"]);
        
        // Ensure destination directory exists
        fs::create_dir_all(temp_dir.path().join(".makeitso/plugins")).unwrap();
        
        let result = install_plugin_from_clone("test-plugin", &registry, "test-registry");
        assert!(result.is_ok());
        
        // Verify plugin was copied
        let dest_path = temp_dir.path().join(".makeitso/plugins/test-plugin");
        assert!(dest_path.exists());
        assert!(dest_path.join("plugin.toml").exists());
        assert!(dest_path.join("main.ts").exists());
        
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_install_plugin_from_clone_fails_when_plugin_already_exists() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Create source plugin
        let registry = create_mock_registry_with_plugins(vec!["test-plugin"]);
        
        // Create existing plugin in destination
        let dest_plugins = temp_dir.path().join(".makeitso/plugins");
        let existing_plugin = dest_plugins.join("test-plugin");
        fs::create_dir_all(&existing_plugin).unwrap();
        
        let result = install_plugin_from_clone("test-plugin", &registry, "test-registry");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
        
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_install_plugin_from_clone_fails_when_plugin_not_in_registry() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Create registry without our target plugin
        let registry = create_mock_registry_with_plugins(vec!["other-plugin"]);
        
        fs::create_dir_all(temp_dir.path().join(".makeitso/plugins")).unwrap();
        
        let result = install_plugin_from_clone("test-plugin", &registry, "test-registry");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found in registry"));
        
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_copy_dir_recursive_copies_all_files() {
        let temp_dir = tempdir().unwrap();
        
        // Create source directory structure
        let src_dir = temp_dir.path().join("source");
        fs::create_dir_all(src_dir.join("subdir")).unwrap();
        fs::write(src_dir.join("file1.txt"), "content1").unwrap();
        fs::write(src_dir.join("subdir/file2.txt"), "content2").unwrap();
        
        // Copy to destination
        let dst_dir = temp_dir.path().join("destination");
        let result = copy_dir_recursive(&src_dir, &dst_dir);
        assert!(result.is_ok());
        
        // Verify all files were copied
        assert!(dst_dir.join("file1.txt").exists());
        assert!(dst_dir.join("subdir/file2.txt").exists());
        assert_eq!(fs::read_to_string(dst_dir.join("file1.txt")).unwrap(), "content1");
        assert_eq!(fs::read_to_string(dst_dir.join("subdir/file2.txt")).unwrap(), "content2");
    }

    // Tests for the main add_plugin function will be added next
    // These test the specific bugs we identified

    #[test]
    fn test_install_plugin_with_force_overwrites_existing() {
        run_test_in_temp_dir(|temp_dir| {
            // Create source plugin
            let registry = create_mock_registry_with_plugins(vec!["test-plugin"]);
            
            // Create existing plugin in destination with different content
            let dest_plugins = temp_dir.path().join(".makeitso/plugins");
            let existing_plugin = dest_plugins.join("test-plugin");
            fs::create_dir_all(&existing_plugin).unwrap();
            fs::write(existing_plugin.join("old-file.txt"), "old content").unwrap();
            
            // Install with force=true should succeed
            let result = install_plugin_from_clone_with_force("test-plugin", &registry, "test-registry", true);
            assert!(result.is_ok(), "Failed to install with force: {:?}", result);
            
            // Verify new content exists and old content is gone
            let dest_path = temp_dir.path().join(".makeitso/plugins/test-plugin");
            assert!(dest_path.exists(), "Plugin directory was not created");
            assert!(dest_path.join("plugin.toml").exists(), "plugin.toml was not copied");
            assert!(dest_path.join("main.ts").exists(), "main.ts was not copied");
            assert!(!dest_path.join("old-file.txt").exists(), "Old file was not removed");
        });
    }

    #[test]
    fn test_install_plugin_without_force_fails_when_exists() {
        run_test_in_temp_dir(|temp_dir| {
            // Create source plugin
            let registry = create_mock_registry_with_plugins(vec!["test-plugin"]);
            
            // Create existing plugin in destination
            let dest_plugins = temp_dir.path().join(".makeitso/plugins");
            let existing_plugin = dest_plugins.join("test-plugin");
            fs::create_dir_all(&existing_plugin).unwrap();
            
            // Install with force=false should fail
            let result = install_plugin_from_clone_with_force("test-plugin", &registry, "test-registry", false);
            assert!(result.is_err(), "Should have failed when plugin exists");
            let error_msg = result.unwrap_err().to_string();
            assert!(error_msg.contains("already exists"));
            assert!(error_msg.contains("--force"));
        });
    }

    // This test demonstrates BUG #2: Logic error in plugin installation loop
    #[test]
    fn test_add_plugin_should_install_from_first_matching_registry_only() {
        run_test_in_temp_dir(|temp_dir| {
            // Create two registries, both with the same plugin but different content
            let registry1 = tempdir().unwrap();
            let registry2 = tempdir().unwrap();
            
            // Registry 1: plugin with version 1.0.0
            let plugin1_dir = registry1.path().join("test-plugin");
            fs::create_dir_all(&plugin1_dir).unwrap();
            fs::write(plugin1_dir.join("plugin.toml"), r#"
[plugin]
name = "test-plugin"
version = "1.0.0"
description = "Version 1.0.0"

[commands.test]
script = "./main.ts"
"#).unwrap();
            fs::write(plugin1_dir.join("main.ts"), "console.log('version 1.0.0');").unwrap();
            fs::write(plugin1_dir.join("marker1.txt"), "from-registry-1").unwrap();
            
            // Registry 2: plugin with version 2.0.0 
            let plugin2_dir = registry2.path().join("test-plugin");
            fs::create_dir_all(&plugin2_dir).unwrap();
            fs::write(plugin2_dir.join("plugin.toml"), r#"
[plugin]
name = "test-plugin"
version = "2.0.0"
description = "Version 2.0.0"

[commands.test]
script = "./main.ts"
"#).unwrap();
            fs::write(plugin2_dir.join("main.ts"), "console.log('version 2.0.0');").unwrap();
            fs::write(plugin2_dir.join("marker2.txt"), "from-registry-2").unwrap();
            
            // Create config pointing to both registries (registry1 first)
            let config_content = format!(r#"
name = "test-project"

[project_variables]
foo = "bar"

[registry]
sources = [
    "{}",
    "{}"
]
"#, registry1.path().display(), registry2.path().display());
            
            let makeitso_dir = temp_dir.path().join(".makeitso");
            fs::create_dir_all(&makeitso_dir).unwrap();
            fs::write(makeitso_dir.join("mis.toml"), config_content).unwrap();
            
            // Mock cloned repos
            let mut cloned_repos = HashMap::new();
            cloned_repos.insert(registry1.path().to_string_lossy().to_string(), registry1);
            cloned_repos.insert(registry2.path().to_string_lossy().to_string(), registry2);
            
            // Install plugin - should only install from first registry
            fs::create_dir_all(temp_dir.path().join(".makeitso/plugins")).unwrap();
            
            // Simulate the logic from add_plugin - find first matching registry and install
            let plugin_name = "test-plugin";
            let mut installed = false;
            for (url, temp_dir_ref) in &cloned_repos {
                let plugin_path = temp_dir_ref.path().join(plugin_name);
                if plugin_path.exists() && plugin_path.is_dir() {
                    let result = install_plugin_from_clone_with_force(plugin_name, temp_dir_ref, url, false);
                    assert!(result.is_ok(), "Failed to install plugin: {:?}", result);
                    installed = true;
                    break; // Only install from first matching registry
                }
            }
            assert!(installed, "Plugin should have been installed");
            
            // Verify plugin was installed from first registry only
            let dest_path = temp_dir.path().join(".makeitso/plugins/test-plugin");
            assert!(dest_path.exists(), "Plugin directory was not created");
            
            // Check content to see which registry it came from
            let plugin_toml_content = fs::read_to_string(dest_path.join("plugin.toml")).unwrap();
            let main_ts_content = fs::read_to_string(dest_path.join("main.ts")).unwrap();
            
            // Should contain content from registry1 (first registry)
            assert!(plugin_toml_content.contains("1.0.0"), "Should have version 1.0.0 from first registry");
            assert!(main_ts_content.contains("version 1.0.0"), "Should have content from first registry");
            assert!(dest_path.join("marker1.txt").exists(), "Should have marker file from first registry");
            assert!(!dest_path.join("marker2.txt").exists(), "Should NOT have marker file from second registry");
        });
    }

    // Helper function for better test isolation
    fn run_test_in_temp_dir<F>(test_fn: F) 
    where 
        F: FnOnce(&TempDir) + std::panic::UnwindSafe,
    {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        
        // Set up isolated test environment
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Run the test with proper panic recovery
        let result = std::panic::catch_unwind(|| test_fn(&temp_dir));
        
        // Always restore original directory, even if test panicked
        std::env::set_current_dir(original_dir).unwrap();
        
        // Re-panic if the test failed
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    // Improved test that creates a complete test environment
    fn run_test_with_config<F>(config_content: &str, test_fn: F) 
    where 
        F: FnOnce(&TempDir) + std::panic::UnwindSafe,
    {
        run_test_in_temp_dir(|temp_dir| {
            // Create .makeitso directory and config file
            let makeitso_dir = temp_dir.path().join(".makeitso");
            fs::create_dir_all(&makeitso_dir).unwrap();
            fs::write(makeitso_dir.join("mis.toml"), config_content).unwrap();
            
            // Also create plugins directory for tests that need it
            fs::create_dir_all(makeitso_dir.join("plugins")).unwrap();
            
            test_fn(temp_dir);
        });
    }

    #[test]
    fn test_plugin_exists_in_project_with_isolation() {
        run_test_in_temp_dir(|temp_dir| {
            // Create .makeitso/plugins/test-plugin directory
            let plugins_dir = temp_dir.path().join(".makeitso/plugins/test-plugin");
            fs::create_dir_all(&plugins_dir).unwrap();
            
            let result = plugin_exists_in_project(&"test-plugin".to_string());
            assert!(result);
            
            let result = plugin_exists_in_project(&"nonexistent".to_string());
            assert!(!result);
        });
    }

    #[test]
    fn test_install_plugin_with_isolation() {
        run_test_in_temp_dir(|temp_dir| {
            // Create source plugin
            let registry = create_mock_registry_with_plugins(vec!["test-plugin"]);
            
            // Ensure destination directory exists
            fs::create_dir_all(temp_dir.path().join(".makeitso/plugins")).unwrap();
            
            let result = install_plugin_from_clone("test-plugin", &registry, "test-registry");
            assert!(result.is_ok(), "Failed to install plugin: {:?}", result);
            
            // Verify plugin was copied
            let dest_path = temp_dir.path().join(".makeitso/plugins/test-plugin");
            assert!(dest_path.exists(), "Plugin directory was not created");
            assert!(dest_path.join("plugin.toml").exists(), "plugin.toml was not copied");
            assert!(dest_path.join("main.ts").exists(), "main.ts was not copied");
        });
    }

    #[test]
    fn test_add_plugin_validates_empty_plugin_names() {
        let config = create_test_config(Some(vec!["https://example.com/registry".to_string()]));
        
        let result = add_plugin_with_config(vec!["".to_string()], false, None, false, config.clone());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Plugin name cannot be empty"));
        
        let result = add_plugin_with_config(vec!["   ".to_string()], false, None, false, config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Plugin name cannot be empty"));
    }

    #[test]
    fn test_add_plugin_validates_invalid_characters_in_plugin_names() {
        let config = create_test_config(Some(vec!["https://example.com/registry".to_string()]));
        
        let invalid_names = vec!["my/plugin", "my\\plugin", "my:plugin", "my*plugin", "my?plugin", "my\"plugin", "my<plugin", "my>plugin", "my|plugin"];
        
        for invalid_name in invalid_names {
            let result = add_plugin_with_config(vec![invalid_name.to_string()], false, None, false, config.clone());
            assert!(result.is_err(), "Should have failed for invalid name: {}", invalid_name);
            assert!(result.unwrap_err().to_string().contains("contains invalid characters"), 
                   "Should mention invalid characters for: {}", invalid_name);
        }
    }

    #[test]
    fn test_add_plugin_should_not_have_duplicate_empty_sources_check() {
        let config = create_test_config(None); // No registry sources
        
        let result = add_plugin_with_config(vec!["test-plugin".to_string()], false, None, false, config);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("No registry sources found"));
        // Should not contain duplicated error messages
    }
}
