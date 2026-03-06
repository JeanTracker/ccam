use crate::{claude, config, confirm};
use anyhow::Result;
use colored::Colorize;

pub fn run_login(alias: &str) -> Result<()> {
    let account = config::get_account(alias)?;
    println!(
        "브라우저에서 '{}' 계정 로그인을 진행합니다...",
        alias.bold()
    );
    claude::login(&account.config_dir)?;
    println!("{}", "로그인 완료.".green());
    Ok(())
}

pub fn run_logout(alias: &str) -> Result<()> {
    let account = config::get_account(alias)?;
    if !confirm::confirm_yn(&format!(
        "'{}' 계정을 로그아웃하시겠습니까? (Keychain 토큰이 제거됩니다)",
        alias
    )) {
        println!("취소되었습니다.");
        return Ok(());
    }
    claude::logout(&account.config_dir)?;
    println!("{}", format!("'{}' 로그아웃 완료.", alias).green());
    Ok(())
}
