use crate::{claude, config, confirm};
use anyhow::Result;
use colored::Colorize;
use std::fs;

pub fn run(alias: &str, purge: bool) -> Result<()> {
    let account = config::get_account(alias)?;

    // Determine confirmation level
    if purge {
        println!("{}", "[경고] 이 작업은 되돌릴 수 없습니다.".red().bold());
        println!(
            "  '{}' 계정을 Keychain에서 로그아웃하고,\n  accounts.toml에서 제거하며,\n  디렉토리({})를 삭제합니다.",
            alias,
            account.config_dir.display()
        );
        if !confirm::confirm_yes("") {
            println!("취소되었습니다.");
            return Ok(());
        }
    } else if !confirm::confirm_yn(&format!("'{}' 계정을 제거하시겠습니까?", alias)) {
        println!("취소되었습니다.");
        return Ok(());
    }

    // Step 1: logout to clean Keychain
    println!("Keychain 항목 정리 중...");
    if let Err(e) = claude::logout(&account.config_dir) {
        eprintln!(
            "{} claude logout 실패 (Keychain 항목이 남아있을 수 있음): {}",
            "경고:".yellow(),
            e
        );
    }

    // Step 2: remove from accounts.toml
    config::remove_account(alias)?;
    println!("accounts.toml에서 '{}' 제거 완료.", alias);

    // Step 3: delete directory if --purge
    if purge && account.config_dir.exists() {
        fs::remove_dir_all(&account.config_dir)?;
        println!("디렉토리 삭제 완료: {}", account.config_dir.display());
    }

    println!("{}", format!("'{}' 계정이 제거되었습니다.", alias).green());
    Ok(())
}
