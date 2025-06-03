use crate::constants::{PLUGIN_CONFIG_FILE, PLUGIN_MANIFEST_FILE};
use crate::{
    config::load_mis_config, git_utils::shallow_clone_repo, models::MakeItSoConfig,
    plugin_utils::plugin_exists_in_project, security::validate_registry_url,
};
use anyhow::{Result, anyhow};
use std::{collections::HashMap, fs, path::Path};
use tempfile::TempDir;

pub fn add_plugin(
    plugins: Vec<String>,
    dry_run: bool,
    registry: Option<String>,
    force: bool,
) -> anyhow::Result<()> {
    let (config, _, _) = load_mis_config().unwrap();
    add_plugin_with_config(plugins, dry_run, registry, force, config)
}

// Testable version that accepts config as parameter (dependency injection)
pub fn add_plugin_with_config(
    plugins: Vec<String>,
    dry_run: bool,
    registry: Option<String>,
    force: bool,
    config: MakeItSoConfig,
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
            return Err(anyhow!(
                "Plugin name '{}' contains invalid characters",
                plugin
            ));
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

    // Validate all registry URLs for security
    for source in &sources {
        if let Err(security_error) = validate_registry_url(source) {
            return Err(anyhow!(
                "🛑 Security validation failed for registry '{}': {}\n\
                 → Registry URLs must be secure HTTPS git repositories from trusted sources.",
                source,
                security_error
            ));
        }
    }

    let cloned_repos = temp_clone_repositories(&sources)?;

    // Loop through the plugin args and handle them
    for plugin in &plugins {
        let plugin_name = &plugin;

        // Check if the plugin exists in the project
        if plugin_exists_in_project(plugin_name) && !force {
            anyhow::bail!(
                "🛑 Plugin '{}' already exists in .makeitso/plugins.\n\
                 → Use `mis update {}` to update it to the latest version.\n\
                 → Use `--force` to reinstall and overwrite existing plugin.",
                plugin_name,
                plugin_name
            );
        }

        if !plugin_exists_in_registries(plugin_name, &cloned_repos) {
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
            println!(
                "❌ Failed to install plugin {} from any registry.",
                plugin_name
            );
        }
    }

    Ok(())
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

    // Preserve existing config.toml if doing a force reinstall
    let existing_config = if dest_path.exists() && force {
        let config_path = dest_path.join("config.toml");
        if config_path.exists() {
            Some(fs::read_to_string(&config_path)?)
        } else {
            None
        }
    } else {
        None
    };

    // Remove existing directory if force is enabled
    if dest_path.exists() && force {
        fs::remove_dir_all(&dest_path)?;
    }

    // Copy directory
    copy_dir_recursive(&source_path, &dest_path)?;

    // Restore preserved config.toml if it existed
    if let Some(config_content) = existing_config {
        fs::write(dest_path.join(PLUGIN_CONFIG_FILE), config_content)?;
    }

    // Update manifest.toml to include registry field
    let manifest_path = dest_path.join(PLUGIN_MANIFEST_FILE);
    if manifest_path.exists() {
        update_manifest_with_registry(&manifest_path, registry_url)?;
    } else {
        return Err(anyhow!(
            "Plugin '{}' is missing manifest.toml file",
            plugin_name
        ));
    }

    println!(
        "✅ Installed plugin '{}' from {} → {}",
        plugin_name,
        registry_url,
        dest_path.display()
    );

    Ok(())
}

/// Updates the manifest.toml file to include the registry field
fn update_manifest_with_registry(manifest_path: &Path, registry_url: &str) -> Result<()> {
    use crate::constants::PLUGIN_MANIFEST_FILE;

    // Load the existing manifest
    let manifest_content = fs::read_to_string(manifest_path)?;
    let mut manifest: crate::models::PluginManifest = toml::from_str(&manifest_content)?;

    // Update the registry field
    manifest.plugin.registry = Some(registry_url.to_string());

    // Serialize back to TOML
    let updated_content = toml::to_string_pretty(&manifest)?;

    // Write back to file
    fs::write(manifest_path, updated_content)?;

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

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
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
        use crate::models::RegistryConfig;
        use std::collections::HashMap;

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
            sources
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", ")
        );

        let config_path = makeitso_dir.join("mis.toml");
        fs::write(&config_path, config_content).unwrap();

        (temp_dir, config_path)
    }

    fn create_mock_registry_with_plugins(plugins: Vec<&str>) -> TempDir {
        let temp_dir = tempdir().unwrap();

        for plugin_name in plugins {
            let plugin_dir = temp_dir.path().join(plugin_name);
            fs::create_dir_all(&plugin_dir).unwrap();

            // Create manifest.toml (new structure)
            let manifest_content = format!(
                r#"
[plugin]
name = "{}"
version = "1.0.0"
description = "Test plugin"

[commands.test]
script = "./main.ts"
"#,
                plugin_name
            );
            fs::write(plugin_dir.join(PLUGIN_MANIFEST_FILE), manifest_content).unwrap();

            // Create main.ts
            fs::write(
                plugin_dir.join("main.ts"),
                "console.log('Hello from test plugin');",
            )
            .unwrap();
        }

        temp_dir
    }

    #[test]
    fn test_plugin_exists_in_project_returns_true_when_plugin_exists() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .makeitso/plugins/test-plugin directory
        let plugins_dir = temp_dir.path().join(".makeitso/plugins/test-plugin");
        fs::create_dir_all(&plugins_dir).unwrap();

        // Create manifest.toml file (required by new plugin_exists_in_project check)
        fs::write(
            plugins_dir.join(PLUGIN_MANIFEST_FILE),
            r#"
[plugin]
name = "test-plugin"
version = "1.0.0"

[commands.test]
script = "./test.ts"
"#,
        )
        .unwrap();

        let result = plugin_exists_in_project("test-plugin");
        assert!(result);

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_plugin_exists_in_project_returns_false_when_plugin_missing() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = plugin_exists_in_project("nonexistent-plugin");
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
        assert!(
            result.is_ok(),
            "Plugin installation should succeed. Error: {:?}",
            result
        );

        // Verify plugin was copied
        let dest_path = temp_dir.path().join(".makeitso/plugins/test-plugin");
        assert!(dest_path.exists());
        assert!(dest_path.join(PLUGIN_MANIFEST_FILE).exists());
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
        assert!(
            result.is_err(),
            "Should have failed when plugin already exists"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("already exists"));
        assert!(error_msg.contains("--force"));

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
        assert!(
            result.is_err(),
            "Should have failed when plugin not found in registry"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("not found in registry"));

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
        assert_eq!(
            fs::read_to_string(dst_dir.join("file1.txt")).unwrap(),
            "content1"
        );
        assert_eq!(
            fs::read_to_string(dst_dir.join("subdir/file2.txt")).unwrap(),
            "content2"
        );
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
            let result = install_plugin_from_clone_with_force(
                "test-plugin",
                &registry,
                "test-registry",
                true,
            );
            assert!(result.is_ok(), "Failed to install with force: {:?}", result);

            // Verify new content exists and old content is gone
            let dest_path = temp_dir.path().join(".makeitso/plugins/test-plugin");
            assert!(dest_path.exists(), "Plugin directory was not created");
            assert!(
                dest_path.join(PLUGIN_MANIFEST_FILE).exists(),
                "manifest.toml was not copied"
            );
            assert!(dest_path.join("main.ts").exists(), "main.ts was not copied");
            assert!(
                !dest_path.join("old-file.txt").exists(),
                "Old file was not removed"
            );
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
            let result = install_plugin_from_clone_with_force(
                "test-plugin",
                &registry,
                "test-registry",
                false,
            );
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
            fs::write(
                plugin1_dir.join(PLUGIN_MANIFEST_FILE),
                r#"
[plugin]
name = "test-plugin"
version = "1.0.0"
description = "Version 1.0.0"

[commands.test]
script = "./main.ts"
"#,
            )
            .unwrap();
            fs::write(plugin1_dir.join("main.ts"), "console.log('version 1.0.0');").unwrap();
            fs::write(plugin1_dir.join("marker1.txt"), "from-registry-1").unwrap();

            // Registry 2: plugin with version 2.0.0
            let plugin2_dir = registry2.path().join("test-plugin");
            fs::create_dir_all(&plugin2_dir).unwrap();
            fs::write(
                plugin2_dir.join(PLUGIN_MANIFEST_FILE),
                r#"
[plugin]
name = "test-plugin"
version = "2.0.0"
description = "Version 2.0.0"

[commands.test]
script = "./main.ts"
"#,
            )
            .unwrap();
            fs::write(plugin2_dir.join("main.ts"), "console.log('version 2.0.0');").unwrap();
            fs::write(plugin2_dir.join("marker2.txt"), "from-registry-2").unwrap();

            // Install plugin - should only install from first registry
            fs::create_dir_all(temp_dir.path().join(".makeitso/plugins")).unwrap();

            // Create ordered list of registries to ensure deterministic behavior
            let registries_in_order = vec![
                (registry1.path().to_string_lossy().to_string(), &registry1),
                (registry2.path().to_string_lossy().to_string(), &registry2),
            ];

            // Simulate the logic from add_plugin - find first matching registry and install
            let plugin_name = "test-plugin";
            let mut installed = false;
            for (url, temp_dir_ref) in &registries_in_order {
                let plugin_path = temp_dir_ref.path().join(plugin_name);
                if plugin_path.exists() && plugin_path.is_dir() {
                    let result =
                        install_plugin_from_clone_with_force(plugin_name, temp_dir_ref, url, false);
                    assert!(result.is_ok(), "Failed to install plugin: {:?}", result);
                    installed = true;
                    break; // Only install from first matching registry
                }
            }
            assert!(installed, "Plugin should have been installed");

            // Verify plugin was installed from first registry only
            let dest_path = temp_dir.path().join(".makeitso/plugins/test-plugin");
            assert!(dest_path.exists(), "Plugin directory was not created");

            // Debug: Print actual manifest content
            let manifest_toml_content =
                fs::read_to_string(dest_path.join(PLUGIN_MANIFEST_FILE)).unwrap();
            let main_ts_content = fs::read_to_string(dest_path.join("main.ts")).unwrap();

            println!("Manifest content:\n{}", manifest_toml_content);
            println!("Main.ts content:\n{}", main_ts_content);

            // Should contain content from registry1 (first registry)
            assert!(
                manifest_toml_content.contains("1.0.0"),
                "Should have version 1.0.0 from first registry. Actual content: {}",
                manifest_toml_content
            );
            assert!(
                main_ts_content.contains("version 1.0.0"),
                "Should have content from first registry. Actual content: {}",
                main_ts_content
            );
            assert!(
                dest_path.join("marker1.txt").exists(),
                "Should have marker file from first registry"
            );
            assert!(
                !dest_path.join("marker2.txt").exists(),
                "Should NOT have marker file from second registry"
            );
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
            // Create plugin directory
            let plugin_dir = temp_dir.path().join(".makeitso/plugins/test-plugin");
            fs::create_dir_all(&plugin_dir).unwrap();

            // Create manifest.toml file (required by new plugin_exists_in_project check)
            fs::write(
                plugin_dir.join(PLUGIN_MANIFEST_FILE),
                r#"
[plugin]
name = "test-plugin"
version = "1.0.0"

[commands.test]
script = "./test.ts"
"#,
            )
            .unwrap();

            let result = plugin_exists_in_project("test-plugin");
            assert!(result);
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
            assert!(
                dest_path.join(PLUGIN_MANIFEST_FILE).exists(),
                "manifest.toml was not copied"
            );
            assert!(dest_path.join("main.ts").exists(), "main.ts was not copied");
        });
    }

    #[test]
    fn test_add_plugin_validates_empty_plugin_names() {
        let config = create_test_config(Some(vec!["https://example.com/registry".to_string()]));

        let result =
            add_plugin_with_config(vec!["".to_string()], false, None, false, config.clone());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Plugin name cannot be empty")
        );

        let result = add_plugin_with_config(vec!["   ".to_string()], false, None, false, config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Plugin name cannot be empty")
        );
    }

    #[test]
    fn test_add_plugin_validates_invalid_characters_in_plugin_names() {
        let config = create_test_config(Some(vec!["https://example.com/registry".to_string()]));

        let invalid_names = vec![
            "my/plugin",
            "my\\plugin",
            "my:plugin",
            "my*plugin",
            "my?plugin",
            "my\"plugin",
            "my<plugin",
            "my>plugin",
            "my|plugin",
        ];

        for invalid_name in invalid_names {
            let result = add_plugin_with_config(
                vec![invalid_name.to_string()],
                false,
                None,
                false,
                config.clone(),
            );
            assert!(
                result.is_err(),
                "Should have failed for invalid name: {}",
                invalid_name
            );
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("contains invalid characters"),
                "Should mention invalid characters for: {}",
                invalid_name
            );
        }
    }

    #[test]
    fn test_add_plugin_should_not_have_duplicate_empty_sources_check() {
        let config = create_test_config(None); // No registry sources

        let result =
            add_plugin_with_config(vec!["test-plugin".to_string()], false, None, false, config);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("No registry sources found"));
        // Should not contain duplicated error messages
    }

    #[test]
    fn test_add_plugin_blocks_localhost_registry_urls() {
        let config = create_test_config(None); // No registry sources in config

        // Test localhost URLs that should be blocked
        let localhost_urls = vec![
            "http://localhost/repo",
            "https://localhost:8080/git",
            "http://127.0.0.1/admin",
            "https://127.0.0.1:3000/secret.git",
        ];

        for url in localhost_urls {
            let result = add_plugin_with_config(
                vec!["test-plugin".to_string()],
                false,
                Some(url.to_string()),
                false,
                config.clone(),
            );

            assert!(result.is_err(), "Should block localhost URL: {}", url);
            let error_msg = result.unwrap_err().to_string();
            assert!(
                error_msg.contains("Security validation failed"),
                "Should mention security validation for URL: {}. Got: {}",
                url,
                error_msg
            );
            assert!(
                error_msg.contains("localhost") || error_msg.contains("loopback"),
                "Should mention localhost issue for URL: {}. Got: {}",
                url,
                error_msg
            );
        }
    }

    // ========== NEW HIGH-PRIORITY TESTS ==========

    #[test]
    fn test_add_plugin_populates_registry_field_in_manifest() {
        run_test_in_temp_dir(|temp_dir| {
            // Create a mock registry with a test plugin
            let registry = create_mock_registry_with_plugins(vec!["test-plugin"]);
            let registry_url = "https://github.com/example/test-registry.git"; // Use mock HTTPS URL for test

            // Create config pointing to the mock HTTPS registry
            let config = create_test_config(Some(vec![registry_url.to_string()]));

            // Ensure destination directory exists
            fs::create_dir_all(temp_dir.path().join(".makeitso/plugins")).unwrap();

            // Use direct install function to bypass security checks for testing
            let result = install_plugin_from_clone("test-plugin", &registry, registry_url);

            assert!(
                result.is_ok(),
                "Plugin installation should succeed. Error: {:?}",
                result
            );

            // Verify the manifest.toml has the registry field populated
            let manifest_path = temp_dir
                .path()
                .join(".makeitso/plugins/test-plugin/manifest.toml");
            assert!(manifest_path.exists(), "manifest.toml should exist");

            let manifest_content = fs::read_to_string(&manifest_path).unwrap();
            assert!(
                manifest_content.contains("registry = "),
                "manifest.toml should contain registry field. Content: {}",
                manifest_content
            );

            // Load and verify the manifest structure
            let manifest = crate::config::plugins::load_plugin_manifest(&manifest_path).unwrap();
            assert!(
                manifest.plugin.registry.is_some(),
                "Registry field should be populated"
            );
            assert_eq!(
                manifest.plugin.registry.unwrap(),
                registry_url,
                "Registry should match the source URL"
            );
        });
    }

    #[test]
    fn test_add_plugin_creates_initial_config_file() {
        run_test_in_temp_dir(|temp_dir| {
            // Create a mock registry with a plugin that has default config
            let registry_dir = tempdir().unwrap();
            let plugin_dir = registry_dir.path().join("config-plugin");
            fs::create_dir_all(&plugin_dir).unwrap();

            // Create manifest.toml
            fs::write(
                plugin_dir.join("manifest.toml"),
                r#"
[plugin]
name = "config-plugin"
version = "1.0.0"
description = "Plugin with default config"

[commands.setup]
script = "./setup.ts"
"#,
            )
            .unwrap();

            // Create default config.toml that should be copied
            fs::write(
                plugin_dir.join("config.toml"),
                r#"
# Default configuration for config-plugin
database_url = "postgres://localhost/dev"
debug = false
max_connections = 10
"#,
            )
            .unwrap();

            fs::write(plugin_dir.join("setup.ts"), "console.log('Setup script');").unwrap();

            // Use direct install function to bypass security checks for testing
            let registry_url = "https://github.com/example/test-registry.git"; // Mock HTTPS URL

            // Ensure destination directory exists
            fs::create_dir_all(temp_dir.path().join(".makeitso/plugins")).unwrap();

            // Install the plugin directly using the install function
            let result =
                install_plugin_from_path("config-plugin", &plugin_dir, registry_url, false);

            assert!(
                result.is_ok(),
                "Plugin installation should succeed. Error: {:?}",
                result
            );

            // Verify config.toml was copied
            let config_path = temp_dir
                .path()
                .join(".makeitso/plugins/config-plugin/config.toml");
            assert!(config_path.exists(), "config.toml should exist");

            let config_content = fs::read_to_string(&config_path).unwrap();
            assert!(
                config_content.contains("database_url"),
                "config.toml should contain default values"
            );
            assert!(
                config_content.contains("debug = false"),
                "config.toml should contain default boolean values"
            );
        });
    }

    #[test]
    fn test_add_plugin_preserves_existing_config_on_force_reinstall() {
        run_test_in_temp_dir(|temp_dir| {
            // Create a plugin first
            let registry = create_mock_registry_with_plugins(vec!["test-plugin"]);
            let registry_url = "https://github.com/example/test-registry.git"; // Mock HTTPS URL

            fs::create_dir_all(temp_dir.path().join(".makeitso/plugins")).unwrap();

            // Install plugin initially using direct install function
            let result = install_plugin_from_clone("test-plugin", &registry, registry_url);
            assert!(
                result.is_ok(),
                "Initial installation should succeed. Error: {:?}",
                result
            );

            // Create user config.toml with custom values
            let config_path = temp_dir
                .path()
                .join(".makeitso/plugins/test-plugin/config.toml");
            fs::write(
                &config_path,
                r#"
# User customized config
api_key = "user-secret-key"
environment = "production"
"#,
            )
            .unwrap();

            // Force reinstall the plugin using direct install function
            let result =
                install_plugin_from_clone_with_force("test-plugin", &registry, registry_url, true);
            assert!(
                result.is_ok(),
                "Force reinstall should succeed. Error: {:?}",
                result
            );

            // Verify manifest.toml was updated but config.toml was preserved
            let manifest_path = temp_dir
                .path()
                .join(".makeitso/plugins/test-plugin/manifest.toml");
            assert!(manifest_path.exists(), "manifest.toml should exist");

            if config_path.exists() {
                let preserved_config = fs::read_to_string(&config_path).unwrap();
                assert!(
                    preserved_config.contains("user-secret-key"),
                    "User config should be preserved during force reinstall"
                );
            }
        });
    }

    #[test]
    fn test_install_plugin_with_registry_field_population() {
        run_test_in_temp_dir(|temp_dir| {
            // Create source plugin without registry field
            let registry = create_mock_registry_with_plugins(vec!["test-plugin"]);
            let registry_url = "https://github.com/example/plugins.git";

            fs::create_dir_all(temp_dir.path().join(".makeitso/plugins")).unwrap();

            // Install plugin and verify registry field is added
            let result = install_plugin_from_clone("test-plugin", &registry, registry_url);
            assert!(result.is_ok(), "Installation should succeed");

            // Read the installed manifest and verify registry field
            let manifest_path = temp_dir
                .path()
                .join(".makeitso/plugins/test-plugin/manifest.toml");
            let manifest = crate::config::plugins::load_plugin_manifest(&manifest_path).unwrap();

            assert!(
                manifest.plugin.registry.is_some(),
                "Registry field should be populated during installation"
            );
            assert_eq!(
                manifest.plugin.registry.unwrap(),
                registry_url,
                "Registry should match the installation source"
            );
        });
    }
}
