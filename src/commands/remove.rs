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
    run_inner_with_log(alias, active_dir, logout_fn, &mut |msg| {
        eprintln!("{}", msg)
    })
}

/// Like `run_inner` but accepts a `log` callback for all diagnostic messages.
/// This enables tests to capture and assert on exact output without spawning subprocesses.
pub fn run_inner_with_log(
    alias: &str,
    active_dir: Option<&str>,
    logout_fn: impl Fn(&Path) -> Result<()>,
    log: &mut dyn FnMut(&str),
) -> Result<Option<String>> {
    let account = config::get_account(alias)?;

    // Determine whether this account is active before removal.
    // Active means: CLAUDE_CONFIG_DIR points to this dir, or
    // CLAUDE_CONFIG_DIR is unset and the account uses ~/.claude.
    let is_active = match active_dir {
        Some(dir) => account.config_dir == Path::new(dir),
        None => claude::is_default_config_dir(&account.config_dir),
    };

    // Check before removal whether any OTHER account shares the same config_dir.
    // If so, the Keychain entry and directory belong to those accounts too and must not be removed.
    let cfg_before = config::load()?;
    let dir_is_shared = cfg_before
        .accounts
        .iter()
        .any(|(k, v)| k != alias && v.config_dir == account.config_dir);

    // Step 1: logout to clean Keychain (skip when another account still uses the same dir)
    if dir_is_shared {
        log(&format!(
            "{} Skipping Keychain cleanup: another account shares the same config dir.",
            "note:".yellow()
        ));
    } else {
        log("Cleaning up Keychain entry...");
        if let Err(e) = logout_fn(&account.config_dir) {
            log(&format!(
                "{} claude logout failed (Keychain entry may remain): {}",
                "warning:".yellow(),
                e
            ));
        }
    }

    // Step 2: remove from accounts.toml
    let was_default = config::get_default()?.as_deref() == Some(alias);
    config::remove_account(alias)?;
    log(&format!("Removed '{}' from accounts.toml.", alias));

    if was_default {
        match config::get_default()? {
            Some(new_default) => log(&format!(
                "{} default reassigned to '{}'",
                "note:".yellow(),
                new_default.cyan()
            )),
            None => log(&format!(
                "{} no accounts remain; default cleared",
                "note:".yellow()
            )),
        }
    }

    // Step 3: delete config directory.
    // Skip for ~/.claude (always preserved) and when another account still references the dir.
    if claude::is_default_config_dir(&account.config_dir) {
        log(&format!(
            "{} Skipping deletion of default directory: {}",
            "note:".yellow(),
            account.config_dir.display()
        ));
    } else if dir_is_shared {
        log(&format!(
            "{} Skipping deletion: another account still references this directory.",
            "note:".yellow()
        ));
    } else if account.config_dir.exists() {
        fs::remove_dir_all(&account.config_dir)?;
        log(&format!(
            "Deleted directory: {}",
            account.config_dir.display()
        ));
    }

    log(&format!("Account '{}' removed.", alias).green().to_string());

    // Step 4: if this account was active, return the eval statement for the shell wrapper.
    if !is_active {
        return Ok(None);
    }

    let cfg = config::load()?;
    let eval_stmt = match cfg.default.as_deref() {
        Some(new_default) => match cfg.accounts.get(new_default) {
            Some(acc) => {
                log(&format!(
                    "Switching current session to '{}'...",
                    new_default
                ));
                export_statement(acc)
            }
            None => "unset CLAUDE_CONFIG_DIR".to_string(),
        },
        None => {
            log("Unsetting CLAUDE_CONFIG_DIR in current session.");
            "unset CLAUDE_CONFIG_DIR".to_string()
        }
    };
    Ok(Some(eval_stmt))
}
