use crate::{claude, commands::env::export_statement, config, confirm};
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn run(alias: &str, yes: bool) -> Result<()> {
    if !yes && !confirm::confirm_yn(&format!("Remove account '{}'?", alias)) {
        eprintln!("Cancelled.");
        return Ok(());
    }
    let active_dir = std::env::var("CLAUDE_CONFIG_DIR").ok();
    if let Some(stmt) = run_inner(alias, active_dir.as_deref(), claude::logout)? {
        println!("{}", stmt);
    }
    Ok(())
}

/// Core removal logic with injectable side effects for testability.
///
/// `logout_fn` is the only injected dependency — it wraps the Keychain/claude
/// auth call so tests do not require a real claude installation.
///
/// Returns `Ok(Some(eval_stmt))` when the removed account was active in the
/// current shell — the caller should print this to stdout so the shell wrapper
/// can eval it to update `CLAUDE_CONFIG_DIR`.
/// Returns `Ok(None)` when the account was not active.
pub fn run_inner(
    alias: &str,
    active_dir: Option<&str>,
    logout_fn: impl Fn(&Path) -> Result<()>,
) -> Result<Option<String>> {
    let account = config::get_account(alias)?;

    // Determine whether this account is active before removal.
    // Active means: CLAUDE_CONFIG_DIR points to this dir, or
    // CLAUDE_CONFIG_DIR is unset and the account uses ~/.claude.
    let is_active = match active_dir {
        Some(dir) => account.config_dir == Path::new(dir),
        None => claude::is_default_config_dir(&account.config_dir),
    };

    // Step 1: logout to clean Keychain
    eprintln!("Cleaning up Keychain entry...");
    if let Err(e) = logout_fn(&account.config_dir) {
        eprintln!(
            "{} claude logout failed (Keychain entry may remain): {}",
            "warning:".yellow(),
            e
        );
    }

    // Step 2: remove from accounts.toml
    let was_default = config::get_default()?.as_deref() == Some(alias);
    config::remove_account(alias)?;
    eprintln!("Removed '{}' from accounts.toml.", alias);

    if was_default {
        match config::get_default()? {
            Some(new_default) => eprintln!(
                "{} default reassigned to '{}'",
                "note:".yellow(),
                new_default.cyan()
            ),
            None => eprintln!("{} no accounts remain; default cleared", "note:".yellow()),
        }
    }

    // Step 3: delete config directory (skip if it's the default ~/.claude)
    if claude::is_default_config_dir(&account.config_dir) {
        eprintln!(
            "{} Skipping deletion of default directory: {}",
            "note:".yellow(),
            account.config_dir.display()
        );
    } else if account.config_dir.exists() {
        fs::remove_dir_all(&account.config_dir)?;
        eprintln!("Deleted directory: {}", account.config_dir.display());
    }

    eprintln!("{}", format!("Account '{}' removed.", alias).green());

    // Step 4: if this account was active, return the eval statement for the shell wrapper.
    if !is_active {
        return Ok(None);
    }

    let cfg = config::load()?;
    let eval_stmt = match cfg.default.as_deref() {
        Some(new_default) => match cfg.accounts.get(new_default) {
            Some(acc) => {
                eprintln!("Switching current session to '{}'...", new_default);
                export_statement(acc)
            }
            None => "unset CLAUDE_CONFIG_DIR".to_string(),
        },
        None => {
            eprintln!("Unsetting CLAUDE_CONFIG_DIR in current session.");
            "unset CLAUDE_CONFIG_DIR".to_string()
        }
    };
    Ok(Some(eval_stmt))
}
