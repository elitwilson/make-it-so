use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn create_git_tag(tag: &str, repo_path: &Path, dry_run: bool) -> Result<()> {
    println!("🔖 Creating git tag: {}", tag);

    if dry_run {
        println!("🌵 [dry run] Skipping git tag creation.");
        return Ok(());
    }

    let status = Command::new("git")
        .arg("tag")
        .arg(tag)
        .current_dir(repo_path)
        .status()?;

    if !status.success() {
        return Err(anyhow!("❌ Failed to create git tag '{}'", tag));
    }

    println!("✅ Tag '{}' created successfully.", tag);
    Ok(())
}

pub fn push_git_tag(tag: &str, repo_path: &Path, dry_run: bool) -> Result<()> {
    println!("📤 Pushing git tag: {}", tag);

    if dry_run {
        println!("🌵 [dry run] Would run: git push origin {}", tag);
        return Ok(());
    }

    let status = Command::new("git")
        .arg("push")
        .arg("origin")
        .arg(tag)
        .current_dir(repo_path)
        .status()?;

    if !status.success() {
        return Err(anyhow!("❌ Failed to push git tag '{}'", tag));
    }

    println!("✅ Tag '{}' pushed successfully.", tag);
    Ok(())
}

pub fn commit_changes(repo_path: &Path, dry_run: bool) -> Result<()> {
    println!("📝 Committing changes...");

    if dry_run {
        println!("🌵 [dry run] Would run: git add . && git commit -m 'Automated commit'");
        return Ok(());
    }

    // Stage all changes
    let add_status = Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(repo_path)
        .status()?;

    if !add_status.success() {
        return Err(anyhow!("❌ Failed to stage changes."));
    }

    // Try to commit
    let commit_output = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("Automated commit")
        .current_dir(repo_path)
        .output()?;

    let stdout = String::from_utf8_lossy(&commit_output.stdout);
    let stderr: std::borrow::Cow<'_, str> = String::from_utf8_lossy(&commit_output.stderr);

    if !commit_output.status.success() {
        if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
            println!("⚠️ Nothing to commit. Working tree clean.");
            return Ok(());
        } else {
            return Err(anyhow!("❌ Failed to commit changes:\n{}", stderr));
        }
    }

    println!("✅ Changes committed successfully.");
    Ok(())
}

pub fn push_changes(repo_path: &Path, dry_run: bool) -> Result<()> {
    println!("🚀 Pushing committed changes...");

    if dry_run {
        println!("🌵 [dry run] Would run: git push");
        return Ok(());
    }

    let status = Command::new("git")
        .arg("push")
        .current_dir(repo_path)
        .status()?;

    if !status.success() {
        return Err(anyhow!("❌ Failed to push changes."));
    }

    println!("✅ Changes pushed successfully.");
    Ok(())
}

pub fn stage_files(files: &[PathBuf], repo_path: &Path, dry_run: bool) -> Result<()> {
    for file in files {
        if dry_run {
            println!("🌵 [dry run] Would run: git add {}", file.display());
            continue;
        }

        let status = Command::new("git")
            .arg("add")
            .arg(file)
            .current_dir(repo_path)
            .status()?;

        if !status.success() {
            return Err(anyhow!("❌ Failed to stage file: {}", file.display()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_create_git_tag_dry_run() {
        let path = Path::new(".");
        let result = create_git_tag("v0.0.0-test", path, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_push_git_tag_dry_run() {
        let path = Path::new(".");
        let result = push_git_tag("v0.0.0-test", path, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_commit_changes_dry_run() {
        let path = Path::new(".");
        let result = commit_changes(path, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_push_changes_dry_run() {
        let path = Path::new(".");
        let result = push_changes(path, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stage_files_dry_run() {
        let path = Path::new(".");
        let files = vec![PathBuf::from("test.txt")];
        let result = stage_files(&files, path, true);
        assert!(result.is_ok());
    }
}
