use crate::{claude, config, confirm};
use anyhow::Result;
use colored::Colorize;
use std::fs;

pub fn run(alias: &str) -> Result<()> {
    let account = config::get_account(alias)?;

    if !confirm::confirm_yn(&format!("Remove account '{}'?", alias)) {
        eprintln!("Cancelled.");
        return Ok(());
    }

    // Step 1: logout to clean Keychain
    eprintln!("Cleaning up Keychain entry...");
    if let Err(e) = claude::logout(&account.config_dir) {
        eprintln!(
            "{} claude logout failed (Keychain entry may remain): {}",
            "warning:".yellow(),
            e
        );
    }

    // Step 2: remove from accounts.toml
    config::remove_account(alias)?;
    eprintln!("Removed '{}' from accounts.toml.", alias);

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
    Ok(())
}
