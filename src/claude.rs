use anyhow::{Result, bail};
use std::path::Path;
use std::process::Command;

fn find_claude() -> Result<String> {
    let output = Command::new("which").arg("claude").output();
    if let Ok(o) = output
        && o.status.success()
    {
        let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }
    bail!("'claude' binary not found in PATH. Install Claude Code first.");
}

/// Authentication status for a Claude account.
pub struct AuthStatus {
    /// Whether a Keychain credential entry exists for this config directory.
    /// Claude Code manages its own OAuth flow; ccam only checks Keychain presence.
    pub keychain: bool,
}

/// Returns the authentication status for the given config directory.
pub fn auth_status(config_dir: &Path) -> AuthStatus {
    AuthStatus {
        keychain: has_keychain_api_key(config_dir),
    }
}

fn has_keychain_api_key(config_dir: &Path) -> bool {
    let service = keychain_service(config_dir);
    Command::new("security")
        .args(["find-generic-password", "-s", &service])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Returns true if config_dir is the default Claude Code directory (~/.claude).
/// When this is the case, CLAUDE_CONFIG_DIR must NOT be set, otherwise Claude Code
/// uses a hash-based keychain key that differs from the default "Claude Code-credentials".
pub fn is_default_config_dir(config_dir: &Path) -> bool {
    let default = dirs::home_dir().map(|h| h.join(".claude"));
    default.as_deref() == Some(config_dir)
}

/// Returns the macOS Keychain service name for the given config directory.
/// Uses the default key for ~/.claude, and a hash-based key for all other paths.
pub fn keychain_service(config_dir: &Path) -> String {
    if is_default_config_dir(config_dir) {
        "Claude Code-credentials".to_string()
    } else {
        dir_keychain_service(config_dir)
    }
}

/// Run `claude` interactively for the given config directory.
/// Used on first account setup so the user can complete theme selection and login.
pub fn run(config_dir: &Path) -> Result<()> {
    let claude = find_claude()?;
    let mut cmd = Command::new(&claude);
    if !is_default_config_dir(config_dir) {
        cmd.env("CLAUDE_CONFIG_DIR", config_dir);
    }
    cmd.status()?;
    Ok(())
}

/// Run `claude auth logout` with CLAUDE_CONFIG_DIR set to config_dir.
pub fn logout(config_dir: &Path) -> Result<()> {
    let claude = find_claude()?;
    let status = Command::new(&claude)
        .args(["auth", "logout"])
        .env("CLAUDE_CONFIG_DIR", config_dir)
        .status()?;
    if !status.success() {
        bail!("claude logout failed");
    }
    Ok(())
}

/// User information returned by `claude auth status`.
pub struct UserInfo {
    pub email: String,
    pub subscription_type: String,
}

/// Runs `claude auth status` and parses user info from its JSON output.
/// Returns `None` if not logged in or the command fails.
pub fn fetch_user_info(config_dir: &Path) -> Option<UserInfo> {
    let claude = find_claude().ok()?;
    let mut cmd = Command::new(&claude);
    cmd.args(["auth", "status", "--json"]);
    if !is_default_config_dir(config_dir) {
        cmd.env("CLAUDE_CONFIG_DIR", config_dir);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    if json.get("loggedIn").and_then(|v| v.as_bool()) != Some(true) {
        return None;
    }
    let email = json.get("email")?.as_str()?.to_string();
    let subscription_type = json
        .get("subscriptionType")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    Some(UserInfo {
        email,
        subscription_type,
    })
}

pub fn dir_keychain_service(config_dir: &Path) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let path_str = config_dir.to_string_lossy();
    let hash = Sha256::digest(path_str.as_bytes());
    let mut hex = String::with_capacity(8);
    for b in &hash[..4] {
        write!(hex, "{:02x}", b).unwrap();
    }
    format!("Claude Code-credentials-{}", hex)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -- is_default_config_dir / keychain_service --

    #[test]
    fn test_is_default_config_dir() {
        let home = dirs::home_dir().unwrap();
        assert!(is_default_config_dir(&home.join(".claude")));
        assert!(!is_default_config_dir(
            &home.join(".claude-accounts/personal")
        ));
        assert!(!is_default_config_dir(Path::new("/some/other/path")));
    }

    #[test]
    fn test_keychain_service_default_dir() {
        let home = dirs::home_dir().unwrap();
        let service = keychain_service(&home.join(".claude"));
        assert_eq!(service, "Claude Code-credentials");
    }

    #[test]
    fn test_keychain_service_custom_dir() {
        let service = keychain_service(Path::new("/Users/1112298/.claude-accounts/personal"));
        assert_eq!(service, "Claude Code-credentials-c0cf3b6b");
    }

    // -- dir_keychain_service --

    #[test]
    fn test_keychain_service_known_hash() {
        // SHA256("/Users/1112298/.claude-accounts/personal")[:4] → c0cf3b6b
        let service = dir_keychain_service(Path::new("/Users/1112298/.claude-accounts/personal"));
        assert_eq!(service, "Claude Code-credentials-c0cf3b6b");
    }

    #[test]
    fn test_keychain_service_known_hash_company() {
        // SHA256("/Users/1112298/.claude-accounts/company")[:4] → 1b5ba2bc
        let service = dir_keychain_service(Path::new("/Users/1112298/.claude-accounts/company"));
        assert_eq!(service, "Claude Code-credentials-1b5ba2bc");
    }

    #[test]
    fn test_keychain_service_format() {
        let service = dir_keychain_service(Path::new("/some/path"));
        assert!(service.starts_with("Claude Code-credentials-"));
        assert_eq!(service.len(), "Claude Code-credentials-".len() + 8);
    }

    // -- auth_status --

    #[test]
    fn test_auth_status_no_keychain() {
        let tmp = TempDir::new().unwrap();
        let status = auth_status(tmp.path());
        assert!(!status.keychain);
    }
}
