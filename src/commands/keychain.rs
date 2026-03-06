use crate::{confirm, config};
use anyhow::{bail, Result};
use colored::Colorize;
use std::process::Command;

// Claude Code Keychain service name (determined by reverse engineering / docs)
// When CLAUDE_CONFIG_DIR is not set, Claude uses this fixed service+account pair.
const CLAUDE_DEFAULT_SERVICE: &str = "claude";
const CLAUDE_DEFAULT_ACCOUNT: &str = "claude_api_key";

pub fn run_list() -> Result<()> {
    let cfg = config::load()?;
    if cfg.accounts.is_empty() {
        println!("등록된 계정이 없습니다.");
        return Ok(());
    }

    let mut accounts: Vec<(&String, &config::Account)> = cfg.accounts.iter().collect();
    accounts.sort_by_key(|(k, _)| k.as_str());

    for (alias, account) in &accounts {
        let auth = crate::claude::auth_status(&account.config_dir);
        let oauth = if auth.oauth { "OAuth ✓".green() } else { "OAuth ✗".dimmed() };
        let keychain = if auth.keychain { "Keychain ✓".green() } else { "Keychain ✗".dimmed() };
        println!("{:<12} {}  {}", alias.bold(), oauth, keychain);
    }
    Ok(())
}

pub fn run_status_default() -> Result<()> {
    let exists = check_keychain_entry(CLAUDE_DEFAULT_SERVICE, CLAUDE_DEFAULT_ACCOUNT);
    if exists {
        println!(
            "{} 고정 기본 키 Keychain 항목이 {}.",
            "참고:".yellow(),
            "존재합니다".green()
        );
        println!("(CLAUDE_CONFIG_DIR 미설정 환경에서 사용되는 항목)");
        println!();
        println!("제거하려면: {}", "ccm keychain clean-default".cyan());
    } else {
        println!("고정 기본 키 Keychain 항목이 {}.", "없습니다".dimmed());
    }
    Ok(())
}

pub fn run_clean_default() -> Result<()> {
    let exists = check_keychain_entry(CLAUDE_DEFAULT_SERVICE, CLAUDE_DEFAULT_ACCOUNT);
    if !exists {
        println!("고정 기본 키 Keychain 항목이 없습니다. 이미 정리되었거나 사용된 적이 없습니다.");
        return Ok(());
    }

    println!("{}", "[경고] 이 작업은 되돌릴 수 없습니다.".red().bold());
    println!("CLAUDE_CONFIG_DIR 미설정 환경의 Claude 로그인 토큰을 Keychain에서 제거합니다.");
    println!("제거 후에는 ccm 없이 claude를 실행하면 재로그인이 필요합니다.");
    println!();

    if !confirm::confirm_yes("") {
        println!("취소되었습니다.");
        return Ok(());
    }

    delete_keychain_entry(CLAUDE_DEFAULT_SERVICE, CLAUDE_DEFAULT_ACCOUNT)?;
    println!("{}", "고정 기본 키 Keychain 항목이 제거되었습니다.".green());
    Ok(())
}

pub fn run_remove(alias: &str) -> Result<()> {
    let account = config::get_account(alias)?;

    println!("{}", "[경고] 이 작업은 되돌릴 수 없습니다.".red().bold());
    println!(
        "'{}' 계정의 Keychain 토큰을 제거합니다. 이후 재로그인이 필요합니다.",
        alias
    );
    println!("  경로: {}", account.config_dir.display());
    println!();

    if !confirm::confirm_yes("") {
        println!("취소되었습니다.");
        return Ok(());
    }

    // Use claude logout to properly remove the keychain entry for this path
    crate::claude::logout(&account.config_dir)?;
    println!(
        "{}",
        format!("'{}' Keychain 항목이 제거되었습니다.", alias).green()
    );
    Ok(())
}

fn check_keychain_entry(service: &str, account: &str) -> bool {
    Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            service,
            "-a",
            account,
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}


fn delete_keychain_entry(service: &str, account: &str) -> Result<()> {
    let status = Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            service,
            "-a",
            account,
        ])
        .status()?;
    if !status.success() {
        bail!("Keychain 항목 제거 실패 (이미 없거나 권한 문제)");
    }
    Ok(())
}
