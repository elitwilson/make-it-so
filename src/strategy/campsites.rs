use crate::models::{DeploymentContext, ServiceConfig};
use crate::strategy::deploy::DeployStrategy;
use crate::strategy::build::BuildStrategy;
use anyhow::Result;

pub struct CampsitesBuildStrategy;

impl CampsitesBuildStrategy {
  pub fn new() -> Self {
    Self
  }
}

impl BuildStrategy for CampsitesBuildStrategy {
  fn build(&self, ctx: &DeploymentContext) -> Result<()> {
    println!("🚀 Deploying service with Campsites strategy");

    println!("• Service: {}", ctx.service_name);
    println!("• Environment: {}", ctx.env_name);
    println!("• Namespace: {}", ctx.namespace);
    println!("• Version: {}", ctx.version);
    println!("• Dry run: {}", ctx.dry_run);
    println!("• Resolved config path: {}", ctx.resolved_config_path.display());
    println!("• Repo path: {}", ctx.git_repo_path.display());

    let tag = format!("v{}", ctx.version);

    println!();
    println!("🔖 Git tag to create: {}", tag);
    println!("📤 Push command: git push origin {}", tag);

    // let status = Command::new("git")
    //   .arg(["tag", &tag])
    //   .current_dir(ctx.repo_path)
    //   .status()?;

    Ok(())
  }
}

pub struct CampsitesDeployStrategy;

impl CampsitesDeployStrategy {
  pub fn new() -> Self {
    Self
  }
}

impl DeployStrategy for CampsitesDeployStrategy {
  fn deploy(&self, ctx: &DeploymentContext) -> Result<()> {
    println!("🚀 Deploying service with Campsites strategy");

    println!("• Service: {}", ctx.service_name);
    println!("• Environment: {}", ctx.env_name);
    println!("• Namespace: {}", ctx.namespace);
    println!("• Version: {}", ctx.version);
    println!("• Dry run: {}", ctx.dry_run);
    println!("• Resolved config path: {}", ctx.resolved_config_path.display());

    // Later: read file, inject version, shell out to helm, etc.

    Ok(())
  }
}