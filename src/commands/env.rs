use crate::{claude, commands::status::format_account_line, config};
use anyhow::Result;
use colored::control;

/// Internal command: outputs `export CLAUDE_CONFIG_DIR="..."` to stdout.
/// Shell function wraps `ccm use` by eval-ing this output.
/// User-facing messages must go to stderr only.
pub fn run(alias: &str, no_refresh: bool) -> Result<()> {
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
    if no_refresh {
        return Ok(());
    }
    // stderr: user-facing messages (not captured by eval)
    let logged_in = claude::auth_status(&account.config_dir).keychain;
    if logged_in {
        // Refresh stored user info on each switch (picks up logins done outside ccm)
        if let Some(info) = claude::fetch_user_info(&account.config_dir) {
            let _ = config::update_account_user_info(
                alias,
                Some(info.email),
                Some(info.subscription_type),
            );
        }
    }
    // Reload to reflect any updated info before display
    let cfg = config::load()?;
    let account = cfg.accounts.get(alias).cloned().unwrap_or(account);
    let is_default = cfg.default.as_deref() == Some(alias);
    // stdout is captured by the shell wrapper (eval), so colored disables colors globally.
    // Force colors on since stderr is displayed in the terminal.
    control::set_override(true);
    eprintln!(
        "{}",
        format_account_line(alias, &account, logged_in, is_default)
    );
    Ok(())
}
