use anyhow::{Context, Result};
use std::process::Command;

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