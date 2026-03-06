use crate::{claude, config};
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;

pub fn run(
    alias: &str,
    dir: Option<&PathBuf>,
    description: Option<&str>,
    no_login: bool,
) -> Result<()> {
    let config_dir = match dir {
        Some(d) => config::expand_tilde(d),
        None => config::accounts_dir().join(alias),
    };

    println!("[1/3] 디렉토리 준비: {}", config_dir.display());
    let account = config::add_account(alias, config_dir.clone(), description.map(str::to_string))?;
    println!(
        "      {}",
        account.config_dir.display().to_string().dimmed()
    );

    if no_login {
        println!("[2/3] {}", "로그인 건너뜀 (--no-login)".yellow());
        println!("      나중에 로그인하려면: ccm login {}", alias);
    } else {
        println!("[2/3] 브라우저에서 Claude 로그인을 진행합니다...");
        if let Err(e) = claude::login(&account.config_dir) {
            // 로그인 실패 시 등록된 계정 롤백
            let _ = config::remove_account(alias);
            // ccm이 새로 만든 디렉토리만 삭제 (--dir로 기존 경로 지정한 경우는 보존)
            if dir.is_none() {
                let _ = std::fs::remove_dir_all(&account.config_dir);
            }
            return Err(e);
        }
    }

    // 첫 번째 계정이면 자동으로 default 설정
    let cfg = config::load()?;
    if cfg.accounts.len() == 1 && cfg.default.is_none() {
        config::set_default(Some(alias))?;
        println!(
            "[3/3] {} 완료. {} 으로 전환하세요. {}",
            alias.green().bold(),
            format!("ccm use {}", alias).cyan(),
            "(기본 계정으로 설정됨)".dimmed()
        );
    } else {
        println!(
            "[3/3] {} 완료. {} 으로 전환하세요.",
            alias.green().bold(),
            format!("ccm use {}", alias).cyan()
        );
    }
    Ok(())
}
