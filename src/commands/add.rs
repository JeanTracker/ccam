use crate::{claude, config};
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;

pub fn run(alias: &str, dir: Option<&PathBuf>, description: Option<&str>) -> Result<()> {
    let config_dir = match dir {
        Some(d) => config::expand_tilde(d),
        None => config::accounts_dir().join(alias),
    };

    println!("[1/2] 디렉토리 준비: {}", config_dir.display());
    let account = config::add_account(alias, config_dir.clone(), description.map(str::to_string))?;
    println!(
        "      {}",
        account.config_dir.display().to_string().dimmed()
    );
    config::ensure_shared_symlinks()?;
    config::setup_account_symlinks(&account.config_dir)?;

    // 첫 번째 계정이면 자동으로 default 설정
    let cfg = config::load()?;
    let is_first = cfg.accounts.len() == 1 && cfg.default.is_none();
    if is_first {
        config::set_default(Some(alias))?;
    }

    let default_tag = if is_first {
        format!("  {}", "(기본 계정으로 설정됨)".dimmed())
    } else {
        String::new()
    };
    println!(
        "[2/2] {} 완료.{} claude 를 시작합니다...",
        alias.green().bold(),
        default_tag,
    );
    claude::run(&account.config_dir)?;

    // Best-effort: capture user info written to Keychain during login
    if let Some(info) = claude::fetch_user_info(&account.config_dir) {
        let _ = config::update_account_user_info(
            alias,
            Some(info.email.clone()),
            Some(info.subscription_type.clone()),
        );
        eprintln!(
            "      {} ({})",
            info.email.dimmed(),
            info.subscription_type.dimmed()
        );
    }
    Ok(())
}
