use crate::config;
use anyhow::Result;
use colored::Colorize;

pub fn run(names_only: bool) -> Result<()> {
    let cfg = config::load()?;

    if cfg.accounts.is_empty() {
        if names_only {
            return Ok(());
        }
        println!("No accounts registered.");
        println!();
        println!("Add an account: {}", "ccm add <alias>".cyan());

        // Hint about existing ~/.claude
        let default_claude = dirs::home_dir().map(|h| h.join(".claude"));
        if let Some(path) = default_claude
            && path.exists()
        {
            println!();
            println!(
                "{} Existing Claude directory detected: {}",
                "note:".yellow().bold(),
                path.display()
            );
            println!(
                "  {} reuse existing directory (re-login required)",
                "ccm add <alias> --dir ~/.claude".to_string().cyan()
            );
            println!("  {} create a new account", "ccm add <alias>".cyan());
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
