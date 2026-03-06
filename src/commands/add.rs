use crate::config;
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
    if cfg.accounts.len() == 1 && cfg.default.is_none() {
        config::set_default(Some(alias))?;
        println!(
            "[2/2] {} 완료. {} 으로 전환 후 claude 를 실행하세요. {}",
            alias.green().bold(),
            format!("ccam use {}", alias).cyan(),
            "(기본 계정으로 설정됨)".dimmed()
        );
    } else {
        println!(
            "[2/2] {} 완료. {} 으로 전환 후 claude 를 실행하세요.",
            alias.green().bold(),
            format!("ccam use {}", alias).cyan()
        );
    }
    Ok(())
}
