use crate::{claude, config};
use anyhow::{Context, Result};
use colored::Colorize;
use std::env;

pub fn format_account_line(
    alias: &str,
    account: &config::Account,
    logged_in: bool,
    is_default: bool,
) -> String {
    let alias_str = if is_default {
        alias.cyan().bold().to_string()
    } else {
        alias.bold().to_string()
    };
    let prefix = if !logged_in {
        "! "
    } else if is_default {
        "* "
    } else {
        "  "
    };
    format!(
        "{}{} {}{}",
        prefix,
        alias_str,
        account.display_name().dimmed(),
        account.sub_tag()
    )
}

/// Finds the account alias that maps to ~/.claude when CLAUDE_CONFIG_DIR is unset.
/// Priority: 1) default account if it uses ~/.claude  2) first alphabetically
pub fn resolve_default_dir_account(cfg: &config::AccountsConfig) -> Option<&str> {
    let uses_default_dir = |v: &config::Account| claude::is_default_config_dir(&v.config_dir);
    cfg.default
        .as_deref()
        .filter(|d| cfg.accounts.get(*d).is_some_and(uses_default_dir))
        .or_else(|| {
            cfg.sorted_accounts()
                .into_iter()
                .find(|(_, v)| uses_default_dir(v))
                .map(|(k, _)| k)
        })
}

/// Finds the account alias whose config_dir matches the given CLAUDE_CONFIG_DIR value.
/// Priority: 1) default account if its config_dir matches  2) first alphabetically
pub fn resolve_active_account<'a>(
    cfg: &'a config::AccountsConfig,
    active_dir: &str,
) -> Option<&'a str> {
    let matches = |v: &config::Account| v.config_dir.to_string_lossy() == active_dir;
    cfg.default
        .as_deref()
        .filter(|d| cfg.accounts.get(*d).is_some_and(matches))
        .or_else(|| {
            cfg.sorted_accounts()
                .into_iter()
                .find(|(_, v)| matches(v))
                .map(|(k, _)| k)
        })
}

pub fn run_current() -> Result<()> {
    match env::var("CLAUDE_CONFIG_DIR") {
        Ok(dir) => {
            let cfg = config::load()?;
            let alias = resolve_active_account(&cfg, &dir);
            if let Some(a) = alias {
                let account = cfg.accounts.get(a).unwrap();
                let logged_in = claude::auth_status(&account.config_dir).keychain;
                let is_default = cfg.default.as_deref() == Some(a);
                println!("{}", format_account_line(a, account, logged_in, is_default));
            } else {
                println!("{} (not registered in ccm)", dir.yellow());
            }
        }
        Err(_) => {
            // CLAUDE_CONFIG_DIR not set: active dir is ~/.claude
            let cfg = config::load()?;
            if let Some(a) = resolve_default_dir_account(&cfg) {
                let account = cfg.accounts.get(a).unwrap();
                let logged_in = claude::auth_status(&account.config_dir).keychain;
                let is_default = cfg.default.as_deref() == Some(a);
                println!("{}", format_account_line(a, account, logged_in, is_default));
            } else {
                println!(
                    "{}",
                    "CLAUDE_CONFIG_DIR not set (default: ~/.claude, unmanaged by ccm)".dimmed()
                );
            }
        }
    }
    Ok(())
}

pub fn run_status(alias: &str) -> Result<()> {
    let cfg = config::load()?;
    let (key, account) = cfg
        .accounts
        .get_key_value(alias)
        .with_context(|| format!("account '{}' not found", alias))?;
    let auth = claude::auth_status(&account.config_dir);
    let dir_exists = account.config_dir.exists();
    print_detailed(key, account, &auth, dir_exists, &cfg);
    Ok(())
}

fn print_detailed(
    alias: &str,
    account: &config::Account,
    auth: &claude::AuthStatus,
    dir_exists: bool,
    cfg: &config::AccountsConfig,
) {
    let is_default = cfg.default.as_deref() == Some(alias);
    let default_tag = if is_default {
        " (default)".cyan().to_string()
    } else {
        String::new()
    };

    println!("{}{}", alias.bold(), default_tag);
    println!(
        "  path     {}{}",
        account.config_dir.display(),
        if dir_exists { "" } else { "  ⚠ missing" }
    );
    if let Some(desc) = &account.description {
        println!("  desc     {}", desc);
    }
    println!("  added    {}", &account.added_at[..10]);

    // Auth status
    let keychain_icon = if auth.keychain {
        "✓".green()
    } else {
        "✗".dimmed()
    };
    println!("  auth     Keychain {}", keychain_icon);
    if let Some(email) = &account.email {
        let sub = account.subscription_type.as_deref().unwrap_or("unknown");
        println!("  account  {} ({})", email, sub);
    }
}
