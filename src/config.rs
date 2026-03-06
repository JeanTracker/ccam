use anyhow::{bail, Context, Result};
use chrono::Utc;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub config_dir: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub added_at: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AccountsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default)]
    pub accounts: HashMap<String, Account>,
}

pub fn accounts_dir() -> PathBuf {
    home_dir()
        .expect("home dir not found")
        .join(".claude-accounts")
}

pub fn accounts_file() -> PathBuf {
    accounts_dir().join("accounts.toml")
}

pub fn load() -> Result<AccountsConfig> {
    let path = accounts_file();
    if !path.exists() {
        return Ok(AccountsConfig::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| "failed to parse accounts.toml")
}

pub fn save(config: &AccountsConfig) -> Result<()> {
    let dir = accounts_dir();
    fs::create_dir_all(&dir)?;
    let path = accounts_file();
    let content = toml::to_string_pretty(config)?;
    fs::write(&path, content)?;
    Ok(())
}

pub fn add_account(
    alias: &str,
    config_dir: PathBuf,
    description: Option<String>,
) -> Result<Account> {
    let mut cfg = load()?;
    if cfg.accounts.contains_key(alias) {
        bail!("account '{}' already exists", alias);
    }

    let expanded = expand_tilde(&config_dir);
    fs::create_dir_all(&expanded)
        .with_context(|| format!("failed to create directory {}", expanded.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&expanded, fs::Permissions::from_mode(0o700))?;
    }

    let account = Account {
        config_dir: expanded,
        description,
        added_at: Utc::now().to_rfc3339(),
    };
    cfg.accounts.insert(alias.to_string(), account.clone());
    save(&cfg)?;
    Ok(account)
}

pub fn remove_account(alias: &str) -> Result<Account> {
    let mut cfg = load()?;
    let account = cfg
        .accounts
        .remove(alias)
        .with_context(|| format!("account '{}' not found", alias))?;
    if cfg.default.as_deref() == Some(alias) {
        // Auto-assign another account as default if one exists
        cfg.default = cfg.accounts.keys().next().cloned();
    }
    save(&cfg)?;
    Ok(account)
}

pub fn get_account(alias: &str) -> Result<Account> {
    let cfg = load()?;
    cfg.accounts
        .get(alias)
        .cloned()
        .with_context(|| format!("account '{}' not found", alias))
}

pub fn set_default(alias: Option<&str>) -> Result<()> {
    let mut cfg = load()?;
    match alias {
        Some(a) => {
            if !cfg.accounts.contains_key(a) {
                bail!("account '{}' not found", a);
            }
            cfg.default = Some(a.to_string());
        }
        None => cfg.default = None,
    }
    save(&cfg)
}

pub fn get_default() -> Result<Option<String>> {
    Ok(load()?.default)
}

pub fn expand_tilde(path: &Path) -> PathBuf {
    if let Ok(stripped) = path.strip_prefix("~") {
        home_dir()
            .expect("home dir not found")
            .join(stripped)
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // HOME을 변경하는 테스트는 직렬 실행 필요 (전역 상태 충돌 방지)
    static HOME_LOCK: Mutex<()> = Mutex::new(());

    struct TestHome {
        tmp: tempfile::TempDir,
        old_home: Option<String>,
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl TestHome {
        fn new() -> Self {
            let guard = HOME_LOCK.lock().unwrap();
            let tmp = tempfile::TempDir::new().unwrap();
            let old_home = std::env::var("HOME").ok();
            // SAFETY: serialized by HOME_LOCK mutex, no other threads modify HOME
            unsafe { std::env::set_var("HOME", tmp.path()); }
            Self { tmp, old_home, _guard: guard }
        }

        fn path(&self) -> &std::path::Path {
            self.tmp.path()
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            match &self.old_home {
                Some(h) => unsafe { std::env::set_var("HOME", h) },
                None => unsafe { std::env::remove_var("HOME") },
            }
        }
    }

    #[test]
    fn test_expand_tilde_replaces_prefix() {
        let _ctx = TestHome::new();
        let expanded = expand_tilde(Path::new("~/foo/bar"));
        assert!(!expanded.starts_with("~"));
        assert!(expanded.ends_with("foo/bar"));
    }

    #[test]
    fn test_expand_tilde_no_tilde_unchanged() {
        let path = Path::new("/absolute/path");
        assert_eq!(expand_tilde(path), path);
    }

    #[test]
    fn test_add_account_creates_entry() {
        let ctx = TestHome::new();
        let config_dir = ctx.path().join("accounts").join("work");
        add_account("work", config_dir.clone(), None).unwrap();

        let cfg = load().unwrap();
        assert!(cfg.accounts.contains_key("work"));
        assert!(config_dir.exists());
    }

    #[test]
    fn test_add_account_duplicate_fails() {
        let ctx = TestHome::new();
        let config_dir = ctx.path().join("accounts").join("work");
        add_account("work", config_dir.clone(), None).unwrap();
        let result = add_account("work", config_dir, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_account_deletes_entry() {
        let ctx = TestHome::new();
        let config_dir = ctx.path().join("accounts").join("work");
        add_account("work", config_dir, None).unwrap();
        remove_account("work").unwrap();

        let cfg = load().unwrap();
        assert!(!cfg.accounts.contains_key("work"));
    }

    #[test]
    fn test_remove_default_account_reassigns_default() {
        let ctx = TestHome::new();
        add_account("work", ctx.path().join("accounts").join("work"), None).unwrap();
        add_account("personal", ctx.path().join("accounts").join("personal"), None).unwrap();
        set_default(Some("work")).unwrap();
        remove_account("work").unwrap();

        // Remaining account should become default
        assert_eq!(get_default().unwrap(), Some("personal".to_string()));
    }

    #[test]
    fn test_remove_last_account_clears_default() {
        let ctx = TestHome::new();
        let config_dir = ctx.path().join("accounts").join("work");
        add_account("work", config_dir, None).unwrap();
        set_default(Some("work")).unwrap();
        remove_account("work").unwrap();

        assert_eq!(get_default().unwrap(), None);
    }

    #[test]
    fn test_remove_nonexistent_fails() {
        let _ctx = TestHome::new();
        assert!(remove_account("ghost").is_err());
    }

    #[test]
    fn test_get_account_returns_correct() {
        let ctx = TestHome::new();
        let config_dir = ctx.path().join("accounts").join("work");
        add_account("work", config_dir.clone(), Some("회사".to_string())).unwrap();

        let account = get_account("work").unwrap();
        assert_eq!(account.config_dir, config_dir);
        assert_eq!(account.description.as_deref(), Some("회사"));
    }

    #[test]
    fn test_get_account_missing_fails() {
        let _ctx = TestHome::new();
        assert!(get_account("ghost").is_err());
    }

    #[test]
    fn test_set_and_get_default() {
        let ctx = TestHome::new();
        let config_dir = ctx.path().join("accounts").join("work");
        add_account("work", config_dir, None).unwrap();

        set_default(Some("work")).unwrap();
        assert_eq!(get_default().unwrap(), Some("work".to_string()));
    }

    #[test]
    fn test_set_default_nonexistent_fails() {
        let _ctx = TestHome::new();
        assert!(set_default(Some("ghost")).is_err());
    }

    #[test]
    fn test_load_empty_when_no_file() {
        let _ctx = TestHome::new();
        let cfg = load().unwrap();
        assert!(cfg.accounts.is_empty());
        assert!(cfg.default.is_none());
    }
}
