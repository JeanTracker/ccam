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
    let account_info = match (&auth.display_name, &auth.email) {
        (Some(name), Some(email)) => format!("{} <{}>", name, email),
        (Some(name), None) => name.clone(),
        (None, Some(email)) => email.clone(),
        (None, None) => String::new(),
    };
    if account_info.is_empty() {
        eprintln!("Switched to account: {}", alias.bold());
    } else {
        eprintln!(
            "Switched to account: {}  {}",
            alias.bold(),
            account_info.dimmed()
        );
    }
    Ok(())
}
