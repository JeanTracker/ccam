use crate::config;
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
        if let Some(path) = default_claude
            && path.exists()
        {
            println!();
            println!(
                "{} 기존 Claude 디렉토리({})가 감지되었습니다.",
                "참고:".yellow().bold(),
                path.display()
            );
            println!(
                "  {} 기존 디렉토리를 재활용 (재로그인 필요)",
                "ccm add <별칭> --dir ~/.claude".to_string().cyan()
            );
            println!("  {} 새 계정 추가", "ccm add <별칭>".cyan());
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
        let prefix = if is_default { "* " } else { "  " };
        let alias_str = if is_default {
            alias.cyan().bold().to_string()
        } else {
            alias.bold().to_string()
        };

        let desc = account
            .description
            .as_deref()
            .map(|d| format!("  {}", d.dimmed()))
            .unwrap_or_default();

        println!(
            "{}{} {}{}{}",
            prefix,
            alias_str,
            account.display_name().dimmed(),
            account.sub_tag(),
            desc
        );
    }
    Ok(())
}
