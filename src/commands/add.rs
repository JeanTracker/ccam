use crate::{claude, config};
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;

pub fn run(alias: &str, dir: Option<&PathBuf>, description: Option<&str>) -> Result<()> {
    let config_dir = match dir {
        Some(d) => config::expand_tilde(d),
        None => config::accounts_dir().join(alias),
    };

    eprintln!("[1/2] Preparing directory: {}", config_dir.display());
    let account = config::add_account(alias, config_dir.clone(), description.map(str::to_string))?;
    eprintln!(
        "      {}",
        account.config_dir.display().to_string().dimmed()
    );
    config::ensure_shared_symlinks()?;
    config::setup_account_symlinks(&account.config_dir)?;

    // Check if this account was auto-set as default (first account)
    let cfg = config::load()?;
    let auto_defaulted = cfg.accounts.len() == 1 && cfg.default.as_deref() == Some(alias);

    let default_tag = if auto_defaulted {
        format!("  {}", "(set as default)".dimmed())
    } else {
        String::new()
    };
    eprintln!(
        "[2/2] {} ready.{} Starting claude...",
        alias.green().bold(),
        default_tag,
    );
    claude::run(&account.config_dir)?;

    // Best-effort: capture user info written to Keychain during login
    if let Some(info) = claude::fetch_user_info(&account.config_dir) {
        let _ = config::update_account_user_info(
            alias,
            Some(info.email.clone()),
            Some(info.subscription_type.clone()),
        );
        eprintln!(
            "      {} ({})",
            info.email.dimmed(),
            info.subscription_type.dimmed()
        );
    }
    Ok(())
}
