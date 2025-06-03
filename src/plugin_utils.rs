use crate::utils::find_project_root;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Check if a plugin exists in the current project
pub fn plugin_exists_in_project(plugin_name: &str) -> bool {
    let plugin_path = Path::new(".makeitso/plugins").join(plugin_name);
    plugin_path.exists() && plugin_path.is_dir()
}

/// Get the path to a plugin directory, ensuring it exists
pub fn get_plugin_path(plugin_name: &str) -> Result<PathBuf> {
    let root = find_project_root().ok_or_else(|| anyhow::anyhow!("Failed to find project root"))?;

    if !root.exists() {
        anyhow::bail!(
            "🛑 You're not inside a Make It So project.\n\
             → Make sure you're in the project root (where .makeitso/ lives).\n\
             → If you haven't set it up yet, run `mis init`."
        );
    }

    let plugin_path = root.join(".makeitso/plugins").join(plugin_name);

    if !plugin_path.exists() {
        anyhow::bail!(
            "🛑 Plugin '{}' not found in .makeitso/plugins.\n\
             → Available plugins: {}\n\
             → To install a plugin, run `mis add {}`\n\
             → To create a plugin, run `mis create {}`",
            plugin_name,
            list_available_plugins()?,
            plugin_name,
            plugin_name
        );
    }

    // Check for plugin.toml to ensure it's a valid plugin
    let manifest_path = plugin_path.join("plugin.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "🛑 plugin.toml not found for plugin '{}'.\n\
             → Expected to find: {}\n\
             → The plugin may be corrupted.",
            plugin_name,
            manifest_path.display()
        );
    }

    Ok(plugin_path)
}

/// Get the plugins directory path, creating it if needed for write operations
pub fn get_plugins_dir(create_if_missing: bool) -> Result<PathBuf> {
    let root = find_project_root().ok_or_else(|| anyhow::anyhow!("Failed to find project root"))?;

    if !root.exists() {
        anyhow::bail!(
            "🛑 You're not inside a Make It So project.\n\
             → Make sure you're in the project root (where .makeitso/ lives).\n\
             → If you haven't set it up yet, run `mis init`."
        );
    }

    let plugins_dir = root.join(".makeitso/plugins");

    if !plugins_dir.exists() {
        if create_if_missing {
            fs::create_dir_all(&plugins_dir)?;
        } else {
            anyhow::bail!(
                "🛑 No plugins directory found (.makeitso/plugins).\n\
                 → Make sure you're in a Make It So project directory.\n\
                 → If you haven't set it up yet, run `mis init`."
            );
        }
    }

    Ok(plugins_dir)
}

/// List all available plugins in the project
pub fn list_available_plugins() -> Result<String> {
    let plugins_dir = match get_plugins_dir(false) {
        Ok(dir) => dir,
        Err(_) => return Ok("none".to_string()),
    };

    list_plugins_in_directory(&plugins_dir)
}

/// List plugins in a specific directory (helper for testing and flexibility)
pub fn list_plugins_in_directory(plugins_dir: &Path) -> Result<String> {
    let mut plugins = Vec::new();

    if plugins_dir.exists() {
        for entry in fs::read_dir(plugins_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    plugins.push(name.to_string());
                }
            }
        }
    }

    if plugins.is_empty() {
        Ok("none".to_string())
    } else {
        plugins.sort();
        Ok(plugins.join(", "))
    }
}

/// Ensure a plugin doesn't already exist (for add command)
pub fn ensure_plugin_does_not_exist(plugin_name: &str, force: bool) -> Result<()> {
    if plugin_exists_in_project(plugin_name) && !force {
        anyhow::bail!(
            "🛑 Plugin '{}' already exists in your project.\n\
             → Use --force to overwrite the existing plugin.\n\
             → Or choose a different plugin name.",
            plugin_name
        );
    }
    Ok(())
}

/// Get all plugin names in the current project
pub fn get_all_plugin_names() -> Result<Vec<String>> {
    let plugins_dir = get_plugins_dir(false)?;
    let mut plugins = Vec::new();

    for entry in fs::read_dir(plugins_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                plugins.push(name.to_string());
            }
        }
    }

    plugins.sort();
    Ok(plugins)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn run_test_in_temp_dir<F>(test_fn: F)
    where
        F: FnOnce() + std::panic::UnwindSafe,
    {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        // Set up isolated test environment
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Run the test with proper panic recovery
        let result = std::panic::catch_unwind(|| test_fn());

        // Always restore original directory, even if test panicked
        std::env::set_current_dir(original_dir).unwrap();

        // Re-panic if the test failed
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    #[test]
    fn test_plugin_exists_in_project_returns_true_when_plugin_exists() {
        run_test_in_temp_dir(|| {
            // Create .makeitso/plugins/test-plugin directory
            let plugin_dir = Path::new(".makeitso/plugins/test-plugin");
            fs::create_dir_all(&plugin_dir).unwrap();

            let result = plugin_exists_in_project("test-plugin");
            assert!(result);
        });
    }

    #[test]
    fn test_plugin_exists_in_project_returns_false_when_plugin_missing() {
        run_test_in_temp_dir(|| {
            let result = plugin_exists_in_project("nonexistent-plugin");
            assert!(!result);
        });
    }

    #[test]
    fn test_get_plugin_path_fails_when_no_project_root() {
        run_test_in_temp_dir(|| {
            let result = get_plugin_path("test-plugin");
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Failed to find project root")
            );
        });
    }

    #[test]
    fn test_get_plugin_path_fails_when_plugin_not_found() {
        run_test_in_temp_dir(|| {
            // Create .makeitso directory
            fs::create_dir_all(".makeitso/plugins").unwrap();

            let result = get_plugin_path("nonexistent-plugin");
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Plugin 'nonexistent-plugin' not found")
            );
        });
    }

    #[test]
    fn test_get_plugin_path_succeeds_when_plugin_exists() {
        run_test_in_temp_dir(|| {
            // Create .makeitso/plugins/test-plugin directory with plugin.toml
            let plugin_dir = Path::new(".makeitso/plugins/test-plugin");
            fs::create_dir_all(&plugin_dir).unwrap();
            fs::write(plugin_dir.join("plugin.toml"), "# test plugin").unwrap();

            let result = get_plugin_path("test-plugin");
            assert!(result.is_ok());
            assert!(result.unwrap().ends_with("test-plugin"));
        });
    }

    #[test]
    fn test_list_plugins_in_directory_with_empty_directory() {
        run_test_in_temp_dir(|| {
            let plugins_dir = Path::new("plugins");
            fs::create_dir_all(&plugins_dir).unwrap();

            let result = list_plugins_in_directory(&plugins_dir).unwrap();
            assert_eq!(result, "none");
        });
    }

    #[test]
    fn test_list_plugins_in_directory_with_multiple_plugins() {
        run_test_in_temp_dir(|| {
            let plugins_dir = Path::new("plugins");
            fs::create_dir_all(&plugins_dir.join("plugin-c")).unwrap();
            fs::create_dir_all(&plugins_dir.join("plugin-a")).unwrap();
            fs::create_dir_all(&plugins_dir.join("plugin-b")).unwrap();

            let result = list_plugins_in_directory(&plugins_dir).unwrap();
            // Should be sorted alphabetically
            assert_eq!(result, "plugin-a, plugin-b, plugin-c");
        });
    }

    #[test]
    fn test_ensure_plugin_does_not_exist_fails_when_plugin_exists_without_force() {
        run_test_in_temp_dir(|| {
            // Create .makeitso/plugins/test-plugin directory
            fs::create_dir_all(".makeitso/plugins/test-plugin").unwrap();

            let result = ensure_plugin_does_not_exist("test-plugin", false);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Plugin 'test-plugin' already exists")
            );
        });
    }

    #[test]
    fn test_ensure_plugin_does_not_exist_succeeds_when_plugin_exists_with_force() {
        run_test_in_temp_dir(|| {
            // Create .makeitso/plugins/test-plugin directory
            fs::create_dir_all(".makeitso/plugins/test-plugin").unwrap();

            let result = ensure_plugin_does_not_exist("test-plugin", true);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_get_all_plugin_names() {
        run_test_in_temp_dir(|| {
            // Create .makeitso/plugins with multiple plugin directories
            fs::create_dir_all(".makeitso/plugins/plugin-c").unwrap();
            fs::create_dir_all(".makeitso/plugins/plugin-a").unwrap();
            fs::create_dir_all(".makeitso/plugins/plugin-b").unwrap();

            let result = get_all_plugin_names().unwrap();
            // Should be sorted alphabetically
            assert_eq!(result, vec!["plugin-a", "plugin-b", "plugin-c"]);
        });
    }
}
