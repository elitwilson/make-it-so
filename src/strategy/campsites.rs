use crate::git_utils::{commit_changes, create_git_tag, push_changes, push_git_tag, stage_files};
use crate::models::ExecutionContext;
use crate::strategy::build::BuildStrategy;
use crate::strategy::deploy::DeployStrategy;
use crate::strategy::utils::{apply_version_targets};
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
    fn build(&self, ctx: &ExecutionContext, raw_config: &toml::Value) -> Result<()> {
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
        // NOT WORKING FOR REAL HELM CASES
        // apply_version_targets(
        //     &ctx.resolved_config_path,
        //     &strategy_config.version_targets,
        //     ctx.version,
        //     ctx.dry_run,
        // )?;

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
    fn deploy(&self, ctx: &ExecutionContext) -> Result<()> {
        println!("🚀 Deploying service with Campsites strategy");

        println!("• Dry run: {}", ctx.dry_run);
        // println!("• Environment: {}", ctx.env_name);
        // println!("• Namespace: {}", ctx.namespace);
        // println!("• Version: {}", ctx.version);
        // println!(
        //     "• Resolved config path: {}",
        //     ctx.resolved_config_path.display()
        // );

        // Later: read file, inject version, shell out to helm, etc.

        Ok(())
    }
}
