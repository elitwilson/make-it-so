use crate::git_utils::{commit_changes, create_git_tag, push_changes, push_git_tag, stage_files};
use crate::models::DeploymentContext;
use crate::strategy::build::BuildStrategy;
use crate::strategy::deploy::DeployStrategy;
use crate::strategy::utils::{apply_version_targets, apply_version_targets_with_yq};
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct VersionTarget {
    pub key_path: String,
    pub match_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CampsitesStrategyConfig {
    pub version_targets: Vec<VersionTarget>,
}

pub struct CampsitesBuildStrategy;

impl CampsitesBuildStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl BuildStrategy for CampsitesBuildStrategy {
    fn build(&self, ctx: &DeploymentContext, raw_config: &toml::Value) -> Result<()> {
        println!("🚀 Building service with Campsites strategy");
        if ctx.dry_run {
            println!("🌵 Dry run mode enabled — no changes will be written or pushed.");
            println!("───────────────────────────────────────────────────────────────");
        }

        let strategy_config: CampsitesStrategyConfig =
            toml::from_str(&toml::to_string(raw_config)?)?;

        // Show patch targets
        println!("\n🔧 Version Targets:");
        for target in &strategy_config.version_targets {
            print!("• {}", target.key_path);
            if let Some(name) = &target.match_name {
                print!(" (match_name: {})", name);
            }
            println!();
        }

        // YAML mutation
        apply_version_targets_with_yq(
            &ctx.resolved_config_path,
            &ctx.service_name,
            &strategy_config.version_targets,
            ctx.version,
            ctx.dry_run,
        )?;
        // apply_version_targets(
        //     &ctx.resolved_config_path,
        //     &strategy_config.version_targets,
        //     ctx.version,
        //     ctx.dry_run,
        // )?;

        Ok(())

        // Git actions
        // stage_files(
        //     &[ctx.resolved_config_path.clone()],
        //     &ctx.git_repo_path,
        //     ctx.dry_run,
        // )?;
        // commit_changes(&ctx.git_repo_path, ctx.dry_run)?;
        // let tag = format!("v{}", ctx.version);
        // create_git_tag(&tag, &ctx.git_repo_path, ctx.dry_run)?;
        // push_changes(&ctx.git_repo_path, ctx.dry_run)?;
        // push_git_tag(&tag, &ctx.git_repo_path, ctx.dry_run)?;

        // // Final summary
        // println!("\n✅ Build context:");
        // println!("• Service:   {}", ctx.service_name);
        // println!("• Environment: {}", ctx.env_name);
        // println!("• Namespace:   {}", ctx.namespace);
        // println!("• Version:     {}", ctx.version);
        // println!("• Resolved config: {}", ctx.resolved_config_path.display());
        // println!("• Repo path:        {}", ctx.git_repo_path.display());
        // println!("• Tag to create:    {}", tag);

        // if ctx.dry_run {
        //     println!("\n🧪 [Dry run summary] All steps simulated successfully.");
        // }

        // Ok(())
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
        println!(
            "• Resolved config path: {}",
            ctx.resolved_config_path.display()
        );

        // Later: read file, inject version, shell out to helm, etc.

        Ok(())
    }
}
