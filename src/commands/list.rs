use crate::{claude, config};
use anyhow::Result;
use colored::Colorize;

pub fn run(names_only: bool) -> Result<()> {
    let cfg = config::load()?;

    if cfg.accounts.is_empty() {
        if names_only {
            return Ok(());
        }
        println!("등록된 계정이 없습니다.");
        println!();
        println!("계정 추가: {}", "ccm add <별칭>".cyan());

        // Hint about existing ~/.claude
        let default_claude = dirs::home_dir().map(|h| h.join(".claude"));
        if let Some(path) = default_claude {
            if path.exists() {
                println!();
                println!(
                    "{} 기존 Claude 디렉토리({})가 감지되었습니다.",
                    "참고:".yellow().bold(),
                    path.display()
                );
                println!(
                    "  {} 기존 디렉토리를 재활용 (재로그인 필요)",
                    format!("ccm add <별칭> --dir ~/.claude").cyan()
                );
                println!(
                    "  {} 새 계정 추가",
                    "ccm add <별칭>".cyan()
                );
            }
        }
        return Ok(());
    }

    if names_only {
        let mut names: Vec<&str> = cfg.accounts.keys().map(|s| s.as_str()).collect();
        names.sort();
        for name in names {
            println!("{}", name);
        }
        return Ok(());
    }

    let mut accounts: Vec<(&String, &config::Account)> = cfg.accounts.iter().collect();
    accounts.sort_by_key(|(k, _)| k.as_str());

    for (alias, account) in &accounts {
        let is_default = cfg.default.as_deref() == Some(alias.as_str());
        let alias_str = if is_default {
            alias.cyan().bold().to_string()
        } else {
            alias.bold().to_string()
        };
        let default_tag = if is_default {
            format!(" {}", "(default)".truecolor(100, 150, 160))
        } else {
            String::new()
        };

        let auth = claude::auth_status(&account.config_dir);
        let account_str = match (&auth.display_name, &auth.email) {
            (Some(name), Some(email)) => format!("{} <{}>", name, email),
            (Some(name), None) => name.clone(),
            (None, Some(email)) => email.clone(),
            (None, None) => account.config_dir.display().to_string(),
        };

        let desc = account
            .description
            .as_deref()
            .map(|d| format!("  {}", d.dimmed()))
            .unwrap_or_default();

        println!(
            "  {}{} {}{}",
            alias_str,
            default_tag,
            account_str.dimmed(),
            desc
        );
    }
    Ok(())
}
