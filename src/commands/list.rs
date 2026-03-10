use crate::{claude, config};
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
        println!("Add an account: {}", "ccam add <alias>".cyan());

        // Hint about existing ~/.claude
        let default_claude = config::claude_dir();
        if default_claude.exists() {
            println!();
            println!(
                "{} Existing Claude directory detected: {}",
                "note:".yellow().bold(),
                default_claude.display()
            );
            println!(
                "  {} reuse existing directory (re-login required)",
                "ccam add <alias> --dir ~/.claude".to_string().cyan()
            );
            println!("  {} create a new account", "ccam add <alias>".cyan());
        }
        return Ok(());
    }

    let accounts = cfg.sorted_accounts();

    if names_only {
        for (alias, _) in &accounts {
            println!("{}", alias);
        }
        return Ok(());
    }

    for (alias, account) in &accounts {
        let is_default = cfg.default.as_deref() == Some(*alias);
        let logged_in = claude::auth_status(&account.config_dir).keychain;
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

        if logged_in {
            let prefix = if is_default { "* " } else { "  " };
            println!(
                "{}{} {}{}{}",
                prefix,
                alias_str,
                account.display_name().dimmed(),
                account.sub_tag(),
                desc
            );
        } else {
            println!(
                "{} {} {}{}{}",
                "!".yellow(),
                alias_str,
                account.display_name().dimmed(),
                account.sub_tag(),
                desc
            );
        }
    }
    Ok(())
}
