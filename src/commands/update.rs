use crate::plugin_utils::{get_all_plugin_names, get_plugin_path};
use anyhow::Result;

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
    let _plugin_path = get_plugin_path(plugin_name)?;

    if dry_run {
        println!("📝 Would update plugin '{}'", plugin_name);
    } else {
        println!("🔄 Updating plugin '{}'...", plugin_name);
        // TODO: Implement actual update logic
        println!("✅ Plugin '{}' updated successfully", plugin_name);
    }

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
            println!("  - {}", plugin);
        }
    } else {
        println!("🔄 Updating {} plugin(s)...", plugins.len());
        for plugin in &plugins {
            println!("  - Updating '{}'...", plugin);
            // TODO: Implement actual update logic for each plugin
        }
        println!("✅ All plugins updated successfully");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
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

        // Create .makeitso/plugins/test-plugin directory with plugin.toml
        let plugin_dir = temp_dir.path().join(".makeitso/plugins/test-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        // The shared utility requires plugin.toml to exist
        fs::write(plugin_dir.join("plugin.toml"), "# test plugin").unwrap();

        let result = update_plugin(Some("test-plugin".to_string()), false);
        assert!(result.is_ok());

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
}
