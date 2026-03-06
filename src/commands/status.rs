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
                // short 모드에서 미등록 경로면 아무것도 출력하지 않음
            } else if let Some(a) = alias {
                println!("{} ({})", a.green().bold(), dir);
            } else {
                println!("{} (ccm 미등록 경로)", dir.yellow());
            }
        }
        Err(_) => {
            if !short {
                println!(
                    "{}",
                    "CLAUDE_CONFIG_DIR 미설정 (기본값: ~/.claude, ccm 미관리)".dimmed()
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

    let auth_str = match (auth.oauth, auth.keychain) {
        (true, true)  => "OAuth+Keychain".green(),
        (true, false) => "OAuth".green(),
        (false, true) => "Keychain".green(),
        (false, false) => "미로그인".yellow(),
    };

    let dir_str = if dir_exists {
        account.config_dir.display().to_string().normal()
    } else {
        account.config_dir.display().to_string().red()
    };

    let user_str = match (&auth.display_name, &auth.email) {
        (Some(name), Some(email)) => format!("  {}", format!("{} <{}>", name, email).dimmed()),
        (Some(name), None) => format!("  {}", name.as_str().dimmed()),
        (None, Some(email)) => format!("  {}", email.as_str().dimmed()),
        _ => String::new(),
    };

    println!("{}{:<12} {}  [{}]{}", prefix, alias.bold(), dir_str, auth_str, user_str);
}

fn print_detailed(
    alias: &str,
    account: &config::Account,
    auth: &claude::AuthStatus,
    dir_exists: bool,
    cfg: &config::AccountsConfig,
) {
    let is_default = cfg.default.as_deref() == Some(alias);
    let default_tag = if is_default { " (기본)".cyan().to_string() } else { String::new() };

    println!("{}{}", alias.bold().green(), default_tag);
    println!("  경로    {}{}", account.config_dir.display(), if dir_exists { "" } else { "  ⚠ 없음" });
    if let Some(desc) = &account.description {
        println!("  설명    {}", desc);
    }
    println!("  추가일  {}", &account.added_at[..10]);

    // Auth status
    let oauth_icon = if auth.oauth { "✓".green() } else { "✗".dimmed() };
    let keychain_icon = if auth.keychain { "✓".green() } else { "✗".dimmed() };
    println!("  인증    OAuth {}  Keychain {}", oauth_icon, keychain_icon);

    // Account info from OAuth
    if let Some(name) = &auth.display_name {
        let email_str = auth.email.as_deref().map(|e| format!(" <{}>", e)).unwrap_or_default();
        let sub_str = auth.subscription_type.as_deref().map(|s| format!("  [{}]", s)).unwrap_or_default();
        println!("  계정    {}{}{}", name, email_str, sub_str);
    } else if let Some(email) = &auth.email {
        println!("  계정    {}", email);
    }
}
