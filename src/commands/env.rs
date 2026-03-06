use crate::{claude, config};
use anyhow::Result;
use colored::Colorize;

/// Internal command: outputs `export CLAUDE_CONFIG_DIR="..."` to stdout.
/// Shell function wraps `ccm use` by eval-ing this output.
/// User-facing messages must go to stderr only.
pub fn run(alias: &str) -> Result<()> {
    let account = config::get_account(alias)?;
    // stdout: only the export statement for eval
    // If the account uses the default ~/.claude directory, unset CLAUDE_CONFIG_DIR so Claude Code
    // uses its built-in default keychain key ("Claude Code-credentials" without hash suffix).
    if claude::is_default_config_dir(&account.config_dir) {
        println!("unset CLAUDE_CONFIG_DIR");
    } else {
        println!(
            "export CLAUDE_CONFIG_DIR=\"{}\"",
            account.config_dir.display()
        );
    }
    // stderr: user-facing messages (not captured by eval)
    if claude::auth_status(&account.config_dir).keychain {
        // Refresh stored user info on each switch (picks up logins done outside ccm)
        if let Some(info) = claude::fetch_user_info(&account.config_dir) {
            let _ = config::update_account_user_info(
                alias,
                Some(info.email.clone()),
                Some(info.subscription_type.clone()),
            );
            eprintln!(
                "* {} {} ({})",
                alias.green().bold(),
                info.email.dimmed(),
                info.subscription_type
            );
        } else {
            eprintln!("* {}", alias.green().bold());
        }
    } else {
        eprintln!("* {}", alias.green().bold());
        eprintln!(
            "  {} 로그인이 필요합니다. {} 를 실행하세요.",
            "⚠".yellow(),
            "claude".cyan()
        );
    }
    Ok(())
}
