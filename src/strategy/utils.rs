use anyhow::{Context, Result, anyhow};
use serde_yaml::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
// use super::campsites::VersionTarget;

// pub fn patch_yaml_key_value(
//     path: &Path,
//     key: &str,
//     new_value: &str,
//     dry_run: bool,
// ) -> Result<()> {
//     println!("Path: {}", path.display());

//     let file_contents = fs::read(path)
//         .with_context(|| format!("Failed to read file: {}", path.display()))?;
//     let doc = yaml::from_slice(&file_contents)?;

//     println!("File Contents: {:#?}", file_contents);
//     println!("Doc: {:#?}", doc);

//     if dry_run {
//         println!("🌵 [dry run] Would patch key: {}", key);
//         return Ok(());
//     }
    
//     Ok(())
// }

pub fn is_deno_installed() -> bool {
    Command::new("deno")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

// pub fn apply_version_targets(
//     path: &Path,
//     targets: &[VersionTarget],
//     new_version: &str,
//     dry_run: bool,
// ) -> Result<Value> {
//     if dry_run {
//         println!(
//             "🌵 [dry run] Would read and patch YAML file: {}",
//             path.display()
//         );
//         return Ok(Value::Null);
//     }

//     let contents = fs::read_to_string(path)
//         .with_context(|| format!("Failed to read YAML file: {}", path.display()))?;

//     // First, try serde_yaml normally
//     let mut doc: Value = match serde_yaml::from_str(&contents) {
//         Ok(value) => value,
//         Err(err) => {
//             println!("⚠️ Standard YAML parse failed. Trying yq fallback...");
//             return load_yaml_with_yq(path);
//         }
//     };

//     if doc.is_null() {
//         println!("⚠️ YAML file is empty or invalid. Skipping mutation.");
//         return Ok(doc);
//     }

//     for target in targets {
//         patch_single_target(&mut doc, target, new_version)?;
//     }

//     let new_yaml = serde_yaml::to_string(&doc)?;

//     fs::write(path, new_yaml)
//         .with_context(|| format!("Failed to write updated YAML: {}", path.display()))?;
//     println!("✅ Updated version(s) in {}", path.display());

//     Ok(doc)
// }

pub fn load_yaml_with_yq(path: &Path) -> Result<serde_yaml::Value> {
    println!("📣 Attempting to run `yq` fallback...");

    let output = Command::new("yq")
        .args(["eval", "explode(.)", path.to_str().unwrap()])
        .output()
        .context("Failed to run yq")?;

    if !output.status.success() {
        return Err(anyhow!(
            "❌ `yq` fallback failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let yaml_str = String::from_utf8_lossy(&output.stdout);
    let doc: Value =
        serde_yaml::from_str(&yaml_str).context("❌ Failed to parse YAML from yq output")?;

    println!("✅ Successfully loaded YAML via yq fallback (read-only)");
    Ok(doc)
}

// fn patch_single_target(doc: &mut Value, target: &VersionTarget, new_version: &str) -> Result<()> {
//     let key = &target.key_path;

//     // println!("🔍 Patching key: {}", target.key_path);
//     // println!("🔍 match_name: {:?}", target.match_name);
//     // println!("🔍 YAML before:\n{:#?}", serde_yaml::to_string(doc)?);

//     if let Some(match_name) = &target.match_name {
//         let val = doc.get(key);
//         println!("📦 raw value at `{}`:\n{:#?}", key, val);

//         // Handle array-of-maps matching, e.g., proxiedAppEnv -> [{ name: VERSION, value: ... }]
//         if let Some(items) = doc.get_mut(key).and_then(Value::as_sequence_mut) {
//             for item in items {
//                 if let Some(obj) = item.as_mapping_mut() {
//                     let name = obj.get(&Value::String("name".into()));
//                     if name == Some(&Value::String(match_name.clone())) {
//                         obj.insert(
//                             Value::String("value".into()),
//                             Value::String(new_version.into()),
//                         );
//                     }
//                 }
//             }
//         } else {
//             println!("⚠️ Expected an array at '{}'", key);
//         }
//     } else {
//         // Handle direct key update, e.g., proxiedAppImageTag: "..."
//         if let Some(map) = doc.as_mapping_mut() {
//             map.insert(
//                 Value::String(key.clone()),
//                 Value::String(new_version.into()),
//             );
//         } else {
//             println!("⚠️ Expected a top-level mapping to insert '{}'", key);
//         }
//     }

//     Ok(())
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::fs::{self};
//     use tempfile::tempdir;

//     #[test]
//     fn it_replaces_a_simple_top_level_value() {
//         let input_yaml = r#"
// proxiedAppImageTag: old-version
// "#;

//         let mut yaml: Value = serde_yaml::from_str(input_yaml).unwrap();

//         let target = VersionTarget {
//             key_path: "proxiedAppImageTag".to_string(),
//             match_name: None,
//         };

//         patch_single_target(&mut yaml, &target, "1.2.3").unwrap();

//         assert_eq!(
//             yaml.get("proxiedAppImageTag").unwrap().as_str(),
//             Some("1.2.3")
//         );
//     }

//     #[test]
//     fn it_replaces_a_value_in_an_array_of_maps_matching_by_name() {
//         let input_yaml = r#"
// proxiedAppEnv:
//   - name: VERSION
//     value: old-version
//   - name: SOMETHING_ELSE
//     value: untouched
// "#;

//         let mut yaml: Value = serde_yaml::from_str(input_yaml).unwrap();

//         let target = VersionTarget {
//             key_path: "proxiedAppEnv".to_string(),
//             match_name: Some("VERSION".to_string()),
//         };

//         patch_single_target(&mut yaml, &target, "1.2.3").unwrap();

//         // Verify "VERSION" got updated
//         let items = yaml.get("proxiedAppEnv").unwrap().as_sequence().unwrap();

//         let version_entry = items
//             .iter()
//             .find(|item| item.get("name") == Some(&Value::String("VERSION".into())))
//             .unwrap();

//         assert_eq!(version_entry.get("value").unwrap().as_str(), Some("1.2.3"));

//         // Verify "SOMETHING_ELSE" is untouched
//         let other_entry = items
//             .iter()
//             .find(|item| item.get("name") == Some(&Value::String("SOMETHING_ELSE".into())))
//             .unwrap();

//         assert_eq!(
//             other_entry.get("value").unwrap().as_str(),
//             Some("untouched")
//         );
//     }

//     #[test]
//     fn it_uses_yq_fallback_on_serde_failure() {
//         let dir = tempdir().unwrap();
//         let file_path = dir.path().join("anchors.yaml");

//         let bad_yaml = r#"
//     foo: &bar
//       nested: true
//     baz: *bar
//     "#;
//         fs::write(&file_path, bad_yaml).unwrap();

//         let result = apply_version_targets(&file_path, &[], "1.2.3", false);

//         match result {
//             Ok(doc) => {
//                 println!("✅ Fallback succeeded. Parsed doc: {:#?}", doc);
//             }
//             Err(e) => {
//                 panic!("❌ Fallback failed unexpectedly: {e}");
//             }
//         }
//     }
// }
