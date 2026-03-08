use crate::{claude, config};
use anyhow::{Context, Result};
use colored::Colorize;
use std::env;

pub fn run_current() -> Result<()> {
    match env::var("CLAUDE_CONFIG_DIR") {
        Ok(dir) => {
            let cfg = config::load()?;
            let alias = cfg.accounts.iter().find_map(|(k, v)| {
                if v.config_dir.to_string_lossy() == dir {
                    Some(k.as_str())
                } else {
                    None
                }
            });
            if let Some(a) = alias {
                let account = cfg.accounts.get(a);
                let logged_in = account
                    .map(|ac| claude::auth_status(&ac.config_dir).keychain)
                    .unwrap_or(false);
                let name = account.map(|ac| ac.display_name()).unwrap_or("");
                let sub_tag = account.map(|ac| ac.sub_tag()).unwrap_or_default();
                if logged_in {
                    println!("* {} {}{}", a.green().bold(), name.dimmed(), sub_tag);
                } else {
                    println!(
                        "{} {} {}{}",
                        "!".yellow(),
                        a.green().bold(),
                        name.dimmed(),
                        sub_tag
                    );
                }
            } else {
                println!("{} (not registered in ccm)", dir.yellow());
            }
        }
        Err(_) => {
            println!(
                "{}",
                "CLAUDE_CONFIG_DIR not set (default: ~/.claude, unmanaged by ccm)".dimmed()
            );
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

    println!("{}{}", alias.bold().green(), default_tag);
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
