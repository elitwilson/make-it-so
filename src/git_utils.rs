use anyhow::anyhow;
use std::process::Command;

pub fn shallow_clone_repo(repo_uri: String, target_dir: String) -> anyhow::Result<()> {
    let output = Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(&repo_uri)
        .arg(&target_dir)
        .output()?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to clone repository: {}", error_message));
    }

    Ok(())
}