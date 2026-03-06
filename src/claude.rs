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
    /// OAuth login via claude.ai browser flow (.claude.json oauthAccount)
    pub oauth: bool,
    /// API key stored in macOS Keychain
    pub keychain: bool,
    /// Display name from oauthAccount (OAuth only)
    pub display_name: Option<String>,
    /// Email from oauthAccount (OAuth only)
    pub email: Option<String>,
    /// Subscription type from oauthAccount, e.g. "pro", "free" (OAuth only)
    pub subscription_type: Option<String>,
}

impl AuthStatus {
    /// Returns a formatted account info string like "Name <email>" or just "email".
    /// Returns None if neither display_name nor email is available.
    pub fn display_info(&self) -> Option<String> {
        match (&self.display_name, &self.email) {
            (Some(name), Some(email)) => Some(format!("{} <{}>", name, email)),
            (Some(name), None) => Some(name.clone()),
            (None, Some(email)) => Some(email.clone()),
            (None, None) => None,
        }
    }
}

/// Returns detailed authentication status for the given config directory.
pub fn auth_status(config_dir: &Path) -> AuthStatus {
    let keychain = has_keychain_api_key(config_dir);
    let claude_json = config_dir.join(".claude.json");

    let Ok(content) = std::fs::read_to_string(&claude_json) else {
        return AuthStatus {
            oauth: false,
            keychain,
            display_name: None,
            email: None,
            subscription_type: None,
        };
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return AuthStatus {
            oauth: false,
            keychain,
            display_name: None,
            email: None,
            subscription_type: None,
        };
    };

    let oauth_account = value.get("oauthAccount");
    let oauth = oauth_account.is_some_and(|v| !v.is_null());

    let (display_name, email, subscription_type) =
        if let Some(oa) = oauth_account.and_then(|v| v.as_object()) {
            let display_name = oa
                .get("displayName")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let email = oa
                .get("emailAddress")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let subscription_type = oa
                .get("billingType")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            (display_name, email, subscription_type)
        } else {
            (None, None, None)
        };

    AuthStatus {
        oauth,
        keychain,
        display_name,
        email,
        subscription_type,
    }
}

/// Check if an account is logged in via OAuth or API key.
pub fn is_logged_in(config_dir: &Path) -> bool {
    let s = auth_status(config_dir);
    s.oauth || s.keychain
}

fn has_keychain_api_key(config_dir: &Path) -> bool {
    // Claude Code stores credentials as "Claude Code-credentials-<sha256[:8]>" for custom dirs,
    // or "Claude Code-credentials" for the default (no CLAUDE_CONFIG_DIR).
    let service = dir_keychain_service(config_dir);
    Command::new("security")
        .args(["find-generic-password", "-s", &service])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
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

/// Run `claude auth login` with CLAUDE_CONFIG_DIR set to config_dir.
/// The process is interactive (inherits stdin/stdout/stderr).
/// Returns an error if the process fails OR if Keychain verification shows no token was saved.
pub fn login(config_dir: &Path) -> Result<()> {
    let claude = find_claude()?;
    let status = Command::new(&claude)
        .args(["auth", "login"])
        .env("CLAUDE_CONFIG_DIR", config_dir)
        .status()?;
    if !status.success() {
        bail!("claude login failed");
    }
    if !is_logged_in(config_dir) {
        bail!("로그인이 완료되지 않았습니다. /login 명령으로 로그인 후 종료해 주세요.");
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_claude_json(dir: &std::path::Path, content: &str) {
        fs::write(dir.join(".claude.json"), content).unwrap();
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

    // -- auth_status: OAuth --

    #[test]
    fn test_auth_status_no_file() {
        let tmp = TempDir::new().unwrap();
        let status = auth_status(tmp.path());
        assert!(!status.oauth);
        assert!(status.display_name.is_none());
        assert!(status.email.is_none());
    }

    #[test]
    fn test_auth_status_invalid_json() {
        let tmp = TempDir::new().unwrap();
        write_claude_json(tmp.path(), "not json");
        let status = auth_status(tmp.path());
        assert!(!status.oauth);
    }

    #[test]
    fn test_auth_status_null_oauth_account() {
        let tmp = TempDir::new().unwrap();
        write_claude_json(tmp.path(), r#"{"oauthAccount": null}"#);
        let status = auth_status(tmp.path());
        assert!(!status.oauth);
    }

    #[test]
    fn test_auth_status_no_oauth_account_key() {
        let tmp = TempDir::new().unwrap();
        write_claude_json(tmp.path(), r#"{"numStartups": 1}"#);
        let status = auth_status(tmp.path());
        assert!(!status.oauth);
    }

    #[test]
    fn test_auth_status_oauth_present() {
        let tmp = TempDir::new().unwrap();
        write_claude_json(
            tmp.path(),
            r#"{
            "oauthAccount": {
                "displayName": "hyojoong",
                "emailAddress": "hyojoong@gmail.com",
                "billingType": "stripe_subscription"
            }
        }"#,
        );
        let status = auth_status(tmp.path());
        assert!(status.oauth);
        assert_eq!(status.display_name.as_deref(), Some("hyojoong"));
        assert_eq!(status.email.as_deref(), Some("hyojoong@gmail.com"));
        assert_eq!(
            status.subscription_type.as_deref(),
            Some("stripe_subscription")
        );
    }

    #[test]
    fn test_auth_status_partial_fields() {
        let tmp = TempDir::new().unwrap();
        write_claude_json(
            tmp.path(),
            r#"{"oauthAccount": {"emailAddress": "only@email.com"}}"#,
        );
        let status = auth_status(tmp.path());
        assert!(status.oauth);
        assert!(status.display_name.is_none());
        assert_eq!(status.email.as_deref(), Some("only@email.com"));
    }

    // -- is_logged_in --

    #[test]
    fn test_is_logged_in_false_when_no_auth() {
        let tmp = TempDir::new().unwrap();
        // 빈 디렉토리 → Keychain 없음, .claude.json 없음
        assert!(!is_logged_in(tmp.path()));
    }

    #[test]
    fn test_is_logged_in_true_when_oauth() {
        let tmp = TempDir::new().unwrap();
        write_claude_json(
            tmp.path(),
            r#"{"oauthAccount": {"emailAddress": "test@example.com"}}"#,
        );
        assert!(is_logged_in(tmp.path()));
    }
}
