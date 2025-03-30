use anyhow::{Context, Result};
use toml::Value;
use std::{collections::HashMap, process::Command};

pub fn install_deno() -> Result<()> {
  println!("⬇️ Installing Deno...");

  #[cfg(target_os = "macos")]
  let shell_command = "curl -fsSL https://deno.land/install.sh | sh";

  #[cfg(target_os = "linux")]
  let shell_command = "curl -fsSL https://deno.land/install.sh | sh";

  #[cfg(target_os = "windows")]
  let shell_command = "iwr https://deno.land/install.ps1 -useb | iex";

  let status = if cfg!(windows) {
      Command::new("powershell")
          .args(["-Command", shell_command])
          .status()
          .context("Failed to launch PowerShell to install Deno")?
  } else {
      Command::new("sh")
          .arg("-c")
          .arg(shell_command)
          .status()
          .context("Failed to launch shell to install Deno")?
  };

  if !status.success() {
      return Err(anyhow::anyhow!("Deno installation failed"));
  }

  println!("✅ Deno installed. You may need to restart your shell.");
  Ok(())
}

pub fn cache_deno_dependencies(deps: &HashMap<String, String>) -> Result<()> {
    if deps.is_empty() {
        println!("📦 No Deno dependencies defined — skipping cache.");
        return Ok(());
    }

    println!("📦 Caching Deno dependencies...");
    for url in deps.values() {
        println!("• {}", url);
    }

    let status = Command::new("deno")
        .arg("cache")
        .args(deps.values())
        .status()
        .context("Failed to run `deno cache`")?;

    if !status.success() {
        return Err(anyhow::anyhow!("Deno cache failed"));
    }

    println!("✅ Dependencies cached.");
    Ok(())
}

// pub fn cache_deno_dependencies(config: &Value) -> Result<()> {
//     println!("{}", config);

//     let deps_table = config
//         .get("deno_dependencies")
//         .and_then(Value::as_table);

//     let Some(table) = deps_table else {
//         println!("📦 No Deno dependencies defined — skipping cache.");
//         return Ok(());
//     };

//     let urls: Vec<&str> = table
//         .values()
//         .filter_map(Value::as_str)
//         .collect();

//     if urls.is_empty() {
//         println!("📦 No valid Deno dependencies found.");
//         return Ok(());
//     }

//     println!("📦 Caching Deno dependencies...");
//     for url in &urls {
//         println!("• {}", url);
//     }

//     let status = Command::new("deno")
//         .arg("cache")
//         .args(&urls)
//         .status()
//         .context("Failed to run `deno cache`")?;

//     if !status.success() {
//         return Err(anyhow::anyhow!("Deno cache failed"));
//     }

//     println!("✅ Dependencies cached.");
//     Ok(())
// }
