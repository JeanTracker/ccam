use crate::{config, confirm};
use anyhow::{Result, bail};
use colored::Colorize;
use std::process::Command;

// Claude Code Keychain service name (determined by reverse engineering / docs)
// When CLAUDE_CONFIG_DIR is not set, Claude uses this fixed service+account pair.
const CLAUDE_DEFAULT_SERVICE: &str = "claude";
const CLAUDE_DEFAULT_ACCOUNT: &str = "claude_api_key";

pub fn run_list() -> Result<()> {
    let cfg = config::load()?;
    if cfg.accounts.is_empty() {
        println!("No accounts registered.");
        return Ok(());
    }

    let mut accounts: Vec<(&String, &config::Account)> = cfg.accounts.iter().collect();
    accounts.sort_by_key(|(k, _)| k.as_str());

    for (alias, account) in &accounts {
        let auth = crate::claude::auth_status(&account.config_dir);
        let keychain = if auth.keychain {
            "Keychain ✓".green()
        } else {
            "Keychain ✗".dimmed()
        };
        println!("{:<12} {}", alias.bold(), keychain);
    }
    Ok(())
}

pub fn run_status_default() -> Result<()> {
    let exists = check_keychain_entry(CLAUDE_DEFAULT_SERVICE, CLAUDE_DEFAULT_ACCOUNT);
    if exists {
        println!(
            "{} Default Keychain entry {}.",
            "note:".yellow(),
            "exists".green()
        );
        println!("(Used when CLAUDE_CONFIG_DIR is not set)");
        println!();
        println!("To remove: {}", "ccm keychain clean-default".cyan());
    } else {
        println!("Default Keychain entry {}.", "not found".dimmed());
    }
    Ok(())
}

pub fn run_clean_default() -> Result<()> {
    let exists = check_keychain_entry(CLAUDE_DEFAULT_SERVICE, CLAUDE_DEFAULT_ACCOUNT);
    if !exists {
        eprintln!("Default Keychain entry not found. Already cleaned or never used.");
        return Ok(());
    }

    eprintln!("{}", "[warning] This action cannot be undone.".red().bold());
    eprintln!("Removes the Claude login token for the default (no CLAUDE_CONFIG_DIR) environment.");
    eprintln!("After removal, running claude without ccm will require re-login.");
    eprintln!();

    if !confirm::confirm_yes("") {
        eprintln!("Cancelled.");
        return Ok(());
    }

    delete_keychain_entry(CLAUDE_DEFAULT_SERVICE, CLAUDE_DEFAULT_ACCOUNT)?;
    eprintln!("{}", "Default Keychain entry removed.".green());
    Ok(())
}

pub fn run_remove(alias: &str) -> Result<()> {
    let account = config::get_account(alias)?;

    eprintln!("{}", "[warning] This action cannot be undone.".red().bold());
    eprintln!(
        "Removes the Keychain token for '{}'. Re-login will be required.",
        alias
    );
    eprintln!("  path: {}", account.config_dir.display());
    eprintln!();

    if !confirm::confirm_yes("") {
        eprintln!("Cancelled.");
        return Ok(());
    }

    // Use claude logout to properly remove the keychain entry for this path
    crate::claude::logout(&account.config_dir)?;
    eprintln!(
        "{}",
        format!("Keychain entry for '{}' removed.", alias).green()
    );
    Ok(())
}

fn check_keychain_entry(service: &str, account: &str) -> bool {
    Command::new("security")
        .args(["find-generic-password", "-s", service, "-a", account])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn delete_keychain_entry(service: &str, account: &str) -> Result<()> {
    let status = Command::new("security")
        .args(["delete-generic-password", "-s", service, "-a", account])
        .status()?;
    if !status.success() {
        bail!("Failed to remove Keychain entry (already gone or permission denied)");
    }
    Ok(())
}
