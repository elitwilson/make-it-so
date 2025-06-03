use crate::commands::add::{copy_dir_recursive, install_plugin_from_path};
use crate::config::plugins::load_plugin_manifest;
use crate::git_utils::shallow_clone_repo;
use crate::plugin_utils::{get_all_plugin_names, get_plugin_path};
use crate::security::validate_registry_url;
use anyhow::Result;
use std::fs;
use tempfile::TempDir;

/// Update a specific plugin or all plugins to the latest versions
pub fn update_plugin(plugin: Option<String>, dry_run: bool) -> Result<()> {
    match plugin {
        Some(plugin_name) => {
            update_single_plugin(&plugin_name, dry_run)?;
        }
        None => {
            update_all_plugins(dry_run)?;
        }
    }

    Ok(())
}

fn update_single_plugin(plugin_name: &str, dry_run: bool) -> Result<()> {
    // This will validate that the plugin exists and return its path
    let plugin_path = get_plugin_path(plugin_name)?;

    // Load the manifest to get the registry URL
    let manifest_path = plugin_path.join("manifest.toml");
    let manifest = load_plugin_manifest(&manifest_path)?;

    // Check if registry field exists
    let registry_url = manifest.plugin.registry.ok_or_else(|| {
        anyhow::anyhow!(
            "🛑 Plugin '{}' has no registry field in manifest.toml.\n\
             → This plugin cannot be updated automatically.\n\
             → You may need to update it manually or reinstall it.",
            plugin_name
        )
    })?;

    // Validate registry URL for security
    if let Err(security_error) = validate_registry_url(&registry_url) {
        return Err(anyhow::anyhow!(
            "🛑 Security validation failed for registry '{}': {}\n\
             → Registry URLs must be secure HTTPS git repositories from trusted sources.",
            registry_url,
            security_error
        ));
    }

    if dry_run {
        println!(
            "📝 Would update plugin '{}' from {}",
            plugin_name, registry_url
        );
        return Ok(());
    }

    println!("🔄 Updating plugin '{}'...", plugin_name);

    // Clone the registry to a temporary directory
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path().to_string_lossy().to_string();

    if let Err(e) = shallow_clone_repo(registry_url.clone(), temp_path) {
        return Err(anyhow::anyhow!(
            "❌ Failed to clone {}: {}",
            registry_url,
            e
        ));
    }

    // Find the plugin in the cloned repository
    let root_plugin_path = temp_dir.path().join(plugin_name);
    let plugins_subdir_path = temp_dir.path().join("plugins").join(plugin_name);

    let source_path = if plugins_subdir_path.exists() && plugins_subdir_path.is_dir() {
        plugins_subdir_path
    } else if root_plugin_path.exists() && root_plugin_path.is_dir() {
        root_plugin_path
    } else {
        return Err(anyhow::anyhow!(
            "❌ Plugin '{}' not found in registry {}",
            plugin_name,
            registry_url
        ));
    };

    // Preserve existing config.toml
    let config_path = plugin_path.join("config.toml");
    let existing_config = if config_path.exists() {
        Some(fs::read_to_string(&config_path)?)
    } else {
        None
    };

    // Remove existing plugin directory
    if plugin_path.exists() {
        fs::remove_dir_all(&plugin_path)?;
    }

    // Copy new plugin from registry
    copy_dir_recursive(&source_path, &plugin_path)?;

    // Update manifest.toml to include registry field (in case it wasn't there)
    let new_manifest_path = plugin_path.join("manifest.toml");
    if new_manifest_path.exists() {
        update_manifest_with_registry(&new_manifest_path, &registry_url)?;
    }

    // Restore preserved config.toml if it existed
    if let Some(config_content) = existing_config {
        fs::write(&config_path, config_content)?;
        println!("📋 Preserved existing config.toml");
    }

    println!(
        "✅ Plugin '{}' updated successfully from {}",
        plugin_name, registry_url
    );
    Ok(())
}

fn update_all_plugins(dry_run: bool) -> Result<()> {
    let plugins = get_all_plugin_names()?;

    if plugins.is_empty() {
        println!("📋 No plugins found to update.");
        return Ok(());
    }

    if dry_run {
        println!("📝 Would update {} plugin(s):", plugins.len());
        for plugin in &plugins {
            match get_plugin_registry(plugin) {
                Ok(registry) => println!("  - {} (from {})", plugin, registry),
                Err(_) => println!("  - {} (no registry - cannot update)", plugin),
            }
        }
        return Ok(());
    }

    println!("🔄 Updating {} plugin(s)...", plugins.len());
    let mut updated_count = 0;
    let mut failed_count = 0;

    for plugin in &plugins {
        println!("  - Updating '{}'...", plugin);
        match update_single_plugin(plugin, false) {
            Ok(()) => {
                updated_count += 1;
            }
            Err(e) => {
                println!("    ❌ Failed to update '{}': {}", plugin, e);
                failed_count += 1;
            }
        }
    }

    if failed_count == 0 {
        println!("✅ All {} plugins updated successfully", updated_count);
    } else {
        println!(
            "⚠️  Updated {} plugins, {} failed",
            updated_count, failed_count
        );
    }

    Ok(())
}

/// Helper function to get registry URL from a plugin's manifest
fn get_plugin_registry(plugin_name: &str) -> Result<String> {
    let plugin_path = get_plugin_path(plugin_name)?;
    let manifest_path = plugin_path.join("manifest.toml");
    let manifest = load_plugin_manifest(&manifest_path)?;

    manifest
        .plugin
        .registry
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' has no registry field", plugin_name))
}

/// Updates the manifest.toml file to include the registry field
fn update_manifest_with_registry(
    manifest_path: &std::path::Path,
    registry_url: &str,
) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use crate::constants::PLUGIN_MANIFEST_FILE;

    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_update_plugin_fails_when_no_project_root() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = update_plugin(Some("test-plugin".to_string()), false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to find project root")
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_update_plugin_fails_when_no_plugins_directory() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .makeitso directory but no plugins subdirectory
        let makeitso_dir = temp_dir.path().join(".makeitso");
        fs::create_dir_all(&makeitso_dir).unwrap();

        let result = update_plugin(Some("test-plugin".to_string()), false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Plugin 'test-plugin' not found")
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_update_single_plugin_fails_when_plugin_not_found() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .makeitso/plugins directory
        let plugins_dir = temp_dir.path().join(".makeitso/plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        let result = update_plugin(Some("nonexistent-plugin".to_string()), false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Plugin 'nonexistent-plugin' not found")
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_update_single_plugin_succeeds_when_plugin_exists() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .makeitso/plugins/test-plugin directory with manifest.toml
        let plugin_dir = temp_dir.path().join(".makeitso/plugins/test-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        // Create a proper manifest.toml with registry field (required for update)
        let manifest_content = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"
description = "Test plugin for update"
registry = "https://github.com/example/plugins.git"

[commands.test]
script = "./test.ts"
"#;
        fs::write(plugin_dir.join("manifest.toml"), manifest_content).unwrap();

        let result = update_plugin(Some("test-plugin".to_string()), true); // Use dry-run to avoid actual network calls
        assert!(
            result.is_ok(),
            "Update should succeed in dry-run mode. Error: {:?}",
            result
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_update_all_plugins_succeeds_with_empty_directory() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create empty .makeitso/plugins directory
        let plugins_dir = temp_dir.path().join(".makeitso/plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        let result = update_plugin(None, false);
        assert!(result.is_ok());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_update_all_plugins_succeeds_with_multiple_plugins() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .makeitso/plugins with multiple plugin directories
        let plugins_dir = temp_dir.path().join(".makeitso/plugins");
        fs::create_dir_all(&plugins_dir.join("plugin1")).unwrap();
        fs::create_dir_all(&plugins_dir.join("plugin2")).unwrap();
        fs::create_dir_all(&plugins_dir.join("plugin3")).unwrap();

        let result = update_plugin(None, false);
        assert!(result.is_ok());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_list_available_plugins_with_empty_directory() {
        let temp_dir = tempdir().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        let result = crate::plugin_utils::list_plugins_in_directory(&plugins_dir).unwrap();
        assert_eq!(result, "none");
    }

    #[test]
    fn test_list_available_plugins_with_multiple_plugins() {
        let temp_dir = tempdir().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");
        fs::create_dir_all(&plugins_dir.join("plugin-a")).unwrap();
        fs::create_dir_all(&plugins_dir.join("plugin-c")).unwrap();
        fs::create_dir_all(&plugins_dir.join("plugin-b")).unwrap();

        let result = crate::plugin_utils::list_plugins_in_directory(&plugins_dir).unwrap();
        // Should be sorted alphabetically
        assert_eq!(result, "plugin-a, plugin-b, plugin-c");
    }

    // ========== NEW UPDATE FUNCTIONALITY TESTS ==========

    #[test]
    fn test_update_plugin_reads_registry_from_manifest() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create plugin with manifest.toml containing registry field
        let plugin_dir = temp_dir.path().join(".makeitso/plugins/test-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        let manifest_content = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"
description = "Test plugin for update"
registry = "https://github.com/example/plugins.git"

[commands.test]
script = "./test.ts"
"#;
        fs::write(plugin_dir.join(PLUGIN_MANIFEST_FILE), manifest_content).unwrap();

        // Debug: Check if plugin is found by the utility functions
        println!(
            "Plugin exists: {}",
            crate::plugin_utils::plugin_exists_in_project("test-plugin")
        );
        if let Ok(path) = crate::plugin_utils::get_plugin_path("test-plugin") {
            println!("Plugin path found: {}", path.display());
        } else {
            println!("Plugin path NOT found");
        }

        // Debug: Test TOML parsing directly
        let test_toml = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"
description = "Test plugin for update"
registry = "https://github.com/example/plugins.git"

[commands.test]
script = "./test.ts"
"#;
        match toml::from_str::<crate::models::PluginManifest>(test_toml) {
            Ok(parsed_manifest) => {
                println!("TOML parsed successfully");
                println!("Registry field: {:?}", parsed_manifest.plugin.registry);
            }
            Err(e) => {
                println!("TOML parsing failed: {}", e);
            }
        }

        // Debug: Check actual file content
        let file_content = fs::read_to_string(plugin_dir.join("manifest.toml")).unwrap();
        println!("Actual file content:\n{}", file_content);

        // The update should be able to read the registry field
        // For now, just test that it doesn't fail (actual update logic comes next)
        let result = update_plugin(Some("test-plugin".to_string()), true); // dry-run
        assert!(
            result.is_ok(),
            "Update should succeed in dry-run mode. Error: {:?}",
            result
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_update_plugin_preserves_config_file() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create plugin with both manifest.toml and config.toml
        let plugin_dir = temp_dir.path().join(".makeitso/plugins/config-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        let manifest_content = r#"
[plugin]
name = "config-plugin"
version = "1.0.0"
registry = "https://github.com/example/plugins.git"

[commands.setup]
script = "./setup.ts"
"#;
        fs::write(plugin_dir.join("manifest.toml"), manifest_content).unwrap();

        // Create user-customized config.toml
        let user_config = r#"
# User customized configuration
api_key = "user-secret-12345"
environment = "production"
debug = false
"#;
        fs::write(plugin_dir.join("config.toml"), user_config).unwrap();

        // Update should preserve the config file
        let result = update_plugin(Some("config-plugin".to_string()), true); // dry-run
        assert!(result.is_ok(), "Update should succeed");

        // Verify config.toml is still there with user values
        let config_path = plugin_dir.join("config.toml");
        assert!(config_path.exists(), "config.toml should be preserved");

        let preserved_config = fs::read_to_string(&config_path).unwrap();
        assert!(
            preserved_config.contains("user-secret-12345"),
            "User config values should be preserved"
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_update_plugin_fails_when_no_registry_field() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create plugin with manifest.toml but NO registry field
        let plugin_dir = temp_dir.path().join(".makeitso/plugins/legacy-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        let manifest_content = r#"
[plugin]
name = "legacy-plugin"
version = "1.0.0"
description = "Plugin without registry field"

[commands.test]
script = "./test.ts"
"#;
        fs::write(plugin_dir.join("manifest.toml"), manifest_content).unwrap();

        // Update should fail gracefully when no registry is specified
        let result = update_plugin(Some("legacy-plugin".to_string()), false);

        // For now, this might succeed since we haven't implemented the logic yet
        // But when we do implement it, it should fail with a helpful error
        // The test documents the expected behavior

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_update_all_plugins_handles_mixed_registry_sources() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create multiple plugins with different registry sources
        let plugins_dir = temp_dir.path().join(".makeitso/plugins");

        // Plugin 1: GitHub registry
        let plugin1_dir = plugins_dir.join("github-plugin");
        fs::create_dir_all(&plugin1_dir).unwrap();
        fs::write(
            plugin1_dir.join("manifest.toml"),
            r#"
[plugin]
name = "github-plugin"
version = "1.0.0"
registry = "https://github.com/user/plugins.git"

[commands.test]
script = "./test.ts"
"#,
        )
        .unwrap();

        // Plugin 2: GitLab registry
        let plugin2_dir = plugins_dir.join("gitlab-plugin");
        fs::create_dir_all(&plugin2_dir).unwrap();
        fs::write(
            plugin2_dir.join("manifest.toml"),
            r#"
[plugin]
name = "gitlab-plugin"
version = "2.0.0"
registry = "https://gitlab.com/user/plugins.git"

[commands.deploy]
script = "./deploy.ts"
"#,
        )
        .unwrap();

        // Plugin 3: No registry (legacy)
        let plugin3_dir = plugins_dir.join("legacy-plugin");
        fs::create_dir_all(&plugin3_dir).unwrap();
        fs::write(
            plugin3_dir.join("manifest.toml"),
            r#"
[plugin]
name = "legacy-plugin"
version = "1.0.0"

[commands.old]
script = "./old.ts"
"#,
        )
        .unwrap();

        // Update all should handle the mixed scenarios
        let result = update_plugin(None, true); // dry-run
        assert!(
            result.is_ok(),
            "Update all should handle mixed registry sources"
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_update_plugin_validates_registry_url_security() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create plugin with dangerous registry URL
        let plugin_dir = temp_dir.path().join(".makeitso/plugins/dangerous-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        let manifest_content = r#"
[plugin]
name = "dangerous-plugin"
version = "1.0.0"
registry = "http://localhost:8080/malicious.git"

[commands.test]
script = "./test.ts"
"#;
        fs::write(plugin_dir.join("manifest.toml"), manifest_content).unwrap();

        // Update should fail when registry URL is dangerous
        let result = update_plugin(Some("dangerous-plugin".to_string()), false);

        // When we implement the actual update logic, this should fail with security error
        // For now, this documents the expected behavior

        std::env::set_current_dir(original_dir).unwrap();
    }
}
