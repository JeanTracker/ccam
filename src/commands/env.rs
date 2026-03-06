use crate::{claude, config};
use anyhow::Result;
use colored::Colorize;

/// Internal command: outputs `export CLAUDE_CONFIG_DIR="..."` to stdout.
/// Shell function wraps `ccm use` by eval-ing this output.
/// User-facing messages must go to stderr only.
pub fn run(alias: &str) -> Result<()> {
    let account = config::get_account(alias)?;
    // stdout: only the export statement for eval
    println!(
        "export CLAUDE_CONFIG_DIR=\"{}\"",
        account.config_dir.display()
    );
    // stderr: user-facing messages (not captured by eval)
    let auth = claude::auth_status(&account.config_dir);
    match auth.display_info() {
        Some(info) => eprintln!("Switched to account: {}  {}", alias.bold(), info.dimmed()),
        None => eprintln!("Switched to account: {}", alias.bold()),
    }
    Ok(())
}
