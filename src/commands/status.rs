use crate::{claude, config};
use anyhow::Result;
use colored::Colorize;
use std::env;

pub fn run_current(short: bool) -> Result<()> {
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
            if short {
                if let Some(a) = alias {
                    println!("{}", a);
                }
                // In short mode, print nothing for unregistered paths
            } else if let Some(a) = alias {
                let account = cfg.accounts.get(a);
                let name = account.map(|ac| ac.display_name()).unwrap_or("");
                let sub_tag = account.map(|ac| ac.sub_tag()).unwrap_or_default();
                println!("* {} {}{}", a.green().bold(), name.dimmed(), sub_tag);
            } else {
                println!("{} (not registered in ccm)", dir.yellow());
            }
        }
        Err(_) => {
            if !short {
                println!(
                    "{}",
                    "CLAUDE_CONFIG_DIR not set (default: ~/.claude, unmanaged by ccm)".dimmed()
                );
            }
        }
    }
    Ok(())
}

pub fn run_status(alias: Option<&str>) -> Result<()> {
    let cfg = config::load()?;

    let accounts: Vec<(&String, &config::Account)> = if let Some(a) = alias {
        match cfg.accounts.get_key_value(a) {
            Some((k, v)) => vec![(k, v)],
            None => anyhow::bail!("account '{}' not found", a),
        }
    } else {
        let mut v: Vec<_> = cfg.accounts.iter().collect();
        v.sort_by_key(|(k, _)| k.as_str());
        v
    };

    let single = accounts.len() == 1 && alias.is_some();

    for (alias, account) in &accounts {
        let auth = claude::auth_status(&account.config_dir);
        let dir_exists = account.config_dir.exists();

        if single {
            print_detailed(alias, account, &auth, dir_exists, &cfg);
        } else {
            print_summary(alias, account, &auth, dir_exists, &cfg);
        }
    }
    Ok(())
}

fn print_summary(
    alias: &str,
    account: &config::Account,
    auth: &claude::AuthStatus,
    dir_exists: bool,
    cfg: &config::AccountsConfig,
) {
    let is_default = cfg.default.as_deref() == Some(alias);
    let prefix = if is_default { "* " } else { "  " };

    let auth_str = if auth.keychain {
        "logged in".green()
    } else {
        "not logged in".yellow()
    };

    let display = if dir_exists {
        account.display_name().normal()
    } else {
        account.display_name().red()
    };

    let sub_tag = account.sub_tag();

    println!(
        "{}{:<12} {}{}  [{}]",
        prefix,
        alias.bold(),
        display,
        sub_tag,
        auth_str,
    );
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
