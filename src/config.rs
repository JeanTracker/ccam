use anyhow::{Context, Result, bail};
use chrono::Utc;
use colored::Colorize;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_type: Option<String>,
}

impl Account {
    /// Returns the email if known, otherwise an empty string.
    pub fn display_name(&self) -> &str {
        self.email.as_deref().unwrap_or("")
    }

    /// Returns a formatted, colored subscription suffix like ` (pro)`, or empty string.
    pub fn sub_tag(&self) -> String {
        self.subscription_type
            .as_deref()
            .map(|s| format!(" ({})", s.truecolor(100, 150, 160)))
            .unwrap_or_default()
    }
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

pub fn claude_dir() -> PathBuf {
    home_dir().expect("home dir not found").join(".claude")
}

pub fn shared_dir() -> PathBuf {
    accounts_dir().join("shared")
}

/// Items shared across all accounts (settings.json, CLAUDE.md, plugins/)
pub const SHARED_ITEMS: &[&str] = &["settings.json", "CLAUDE.md", "plugins"];

fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

fn create_symlink_if_needed(link: &Path, target: &Path) -> Result<()> {
    if is_symlink(link) || link.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).with_context(|| {
        format!(
            "failed to create symlink {} -> {}",
            link.display(),
            target.display()
        )
    })?;
    Ok(())
}

/// Sets up symlinks in ~/.claude-accounts/shared/ pointing to ~/.claude/.
/// Creates ~/.claude/ if it does not exist.
pub fn ensure_shared_symlinks() -> Result<()> {
    let claude = claude_dir();
    let shared = shared_dir();
    fs::create_dir_all(&claude)?;
    fs::create_dir_all(&shared)?;
    for name in SHARED_ITEMS {
        create_symlink_if_needed(&shared.join(name), &claude.join(name))?;
    }
    Ok(())
}

/// Sets up symlinks in the account directory pointing to ../shared/.
/// Skipped when account_dir is ~/.claude itself (--dir ~/.claude).
pub fn setup_account_symlinks(account_dir: &Path) -> Result<()> {
    let claude = claude_dir();
    let canon_account = account_dir
        .canonicalize()
        .unwrap_or_else(|_| account_dir.to_path_buf());
    let canon_claude = claude.canonicalize().unwrap_or_else(|_| claude.clone());
    if canon_account == canon_claude {
        return Ok(());
    }
    for name in SHARED_ITEMS {
        let account_path = account_dir.join(name);
        if is_symlink(&account_path) {
            continue;
        }
        // If a real file/dir exists, move it to ~/.claude/ then remove the original
        if account_path.exists() {
            let claude_target = claude.join(name);
            if !claude_target.exists() {
                fs::rename(&account_path, &claude_target)
                    .with_context(|| format!("failed to move {}", account_path.display()))?;
            } else if account_path.is_dir() {
                fs::remove_dir_all(&account_path)?;
            } else {
                fs::remove_file(&account_path)?;
            }
        }
        create_symlink_if_needed(&account_path, &PathBuf::from("../shared").join(name))?;
    }
    Ok(())
}

pub fn accounts_file() -> PathBuf {
    accounts_dir().join("accounts.toml")
}

pub fn load() -> Result<AccountsConfig> {
    let path = accounts_file();
    if !path.exists() {
        return Ok(AccountsConfig::default());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
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
        email: None,
        subscription_type: None,
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

pub fn update_account_user_info(
    alias: &str,
    email: Option<String>,
    subscription_type: Option<String>,
) -> Result<()> {
    let mut cfg = load()?;
    let account = cfg
        .accounts
        .get_mut(alias)
        .with_context(|| format!("account '{}' not found", alias))?;
    account.email = email;
    account.subscription_type = subscription_type;
    save(&cfg)
}

pub fn get_default() -> Result<Option<String>> {
    Ok(load()?.default)
}

pub fn expand_tilde(path: &Path) -> PathBuf {
    if let Ok(stripped) = path.strip_prefix("~") {
        home_dir().expect("home dir not found").join(stripped)
    } else {
        path.to_path_buf()
    }
}
