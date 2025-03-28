use anyhow::{Context, Result};
use serde_yaml::Value;
use std::fs;
use std::path::Path;

use super::campsites::VersionTarget;

pub fn apply_version_targets(
    path: &Path,
    targets: &[VersionTarget],
    new_version: &str,
    dry_run: bool,
) -> Result<Value> {
    if dry_run {
        println!("🌵 [dry run] Would read and patch YAML file: {}", path.display());
        return Ok(Value::Null);
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read YAML file: {}", path.display()))?;

    let mut doc: Value = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse YAML: {}", path.display()))?;

    if doc.is_null() {
        println!("⚠️ YAML file is empty or invalid. Skipping mutation.");
        return Ok(doc);
    }

    for target in targets {
        patch_single_target(&mut doc, target, new_version)?;
    }

    let new_yaml = serde_yaml::to_string(&doc)?;
    fs::write(path, new_yaml)
        .with_context(|| format!("Failed to write updated YAML: {}", path.display()))?;
    println!("✅ Updated version(s) in {}", path.display());

    Ok(doc)
}

fn patch_single_target(doc: &mut Value, target: &VersionTarget, new_version: &str) -> Result<()> {
    let key = &target.key_path;

    println!("🔍 Patching key: {}", target.key_path);
    println!("🔍 match_name: {:?}", target.match_name);
    println!("🔍 YAML before:\n{}", serde_yaml::to_string(doc)?);

    if let Some(match_name) = &target.match_name {
        // Handle array-of-maps matching, e.g., proxiedAppEnv -> [{ name: VERSION, value: ... }]
        if let Some(items) = doc.get_mut(key).and_then(Value::as_sequence_mut) {
            for item in items {
                if let Some(obj) = item.as_mapping_mut() {
                    let name = obj.get(&Value::String("name".into()));
                    if name == Some(&Value::String(match_name.clone())) {
                        obj.insert(
                            Value::String("value".into()),
                            Value::String(new_version.into()),
                        );
                    }
                }
            }
        } else {
            println!("⚠️ Expected an array at '{}'", key);
        }
    } else {
        // Handle direct key update, e.g., proxiedAppImageTag: "..."
        if let Some(map) = doc.as_mapping_mut() {
            map.insert(
                Value::String(key.clone()),
                Value::String(new_version.into()),
            );
        } else {
            println!("⚠️ Expected a top-level mapping to insert '{}'", key);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    #[test]
    fn it_replaces_a_simple_top_level_value() {
        let input_yaml = r#"
proxiedAppImageTag: old-version
"#;

        let mut yaml: Value = serde_yaml::from_str(input_yaml).unwrap();

        let target = VersionTarget {
            key_path: "proxiedAppImageTag".to_string(),
            match_name: None,
        };

        patch_single_target(&mut yaml, &target, "1.2.3").unwrap();

        assert_eq!(
            yaml.get("proxiedAppImageTag").unwrap().as_str(),
            Some("1.2.3")
        );
    }

    #[test]
    fn it_replaces_a_value_in_an_array_of_maps_matching_by_name() {
        let input_yaml = r#"
proxiedAppEnv:
  - name: VERSION
    value: old-version
  - name: SOMETHING_ELSE
    value: untouched
"#;

        let mut yaml: Value = serde_yaml::from_str(input_yaml).unwrap();

        let target = VersionTarget {
            key_path: "proxiedAppEnv".to_string(),
            match_name: Some("VERSION".to_string()),
        };

        patch_single_target(&mut yaml, &target, "1.2.3").unwrap();

        // Verify "VERSION" got updated
        let items = yaml.get("proxiedAppEnv").unwrap().as_sequence().unwrap();

        let version_entry = items
            .iter()
            .find(|item| item.get("name") == Some(&Value::String("VERSION".into())))
            .unwrap();

        assert_eq!(version_entry.get("value").unwrap().as_str(), Some("1.2.3"));

        // Verify "SOMETHING_ELSE" is untouched
        let other_entry = items
            .iter()
            .find(|item| item.get("name") == Some(&Value::String("SOMETHING_ELSE".into())))
            .unwrap();

        assert_eq!(
            other_entry.get("value").unwrap().as_str(),
            Some("untouched")
        );
    }
}
