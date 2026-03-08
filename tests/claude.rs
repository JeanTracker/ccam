use ccam::claude::{auth_status, dir_keychain_service, is_default_config_dir, keychain_service};
use std::path::Path;
use tempfile::TempDir;

// -- is_default_config_dir / keychain_service --

#[test]
fn test_is_default_config_dir() {
    let home = dirs::home_dir().unwrap();
    assert!(is_default_config_dir(&home.join(".claude")));
    assert!(!is_default_config_dir(
        &home.join(".claude-accounts/personal")
    ));
    assert!(!is_default_config_dir(Path::new("/some/other/path")));
}

#[test]
fn test_keychain_service_default_dir() {
    let home = dirs::home_dir().unwrap();
    let service = keychain_service(&home.join(".claude"));
    assert_eq!(service, "Claude Code-credentials");
}

#[test]
fn test_keychain_service_custom_dir() {
    let service = keychain_service(Path::new("/Users/1112298/.claude-accounts/personal"));
    assert_eq!(service, "Claude Code-credentials-c0cf3b6b");
}

// -- dir_keychain_service --

#[test]
fn test_keychain_service_known_hash() {
    // SHA256("/Users/1112298/.claude-accounts/personal")[:4] → c0cf3b6b
    let service = dir_keychain_service(Path::new("/Users/1112298/.claude-accounts/personal"));
    assert_eq!(service, "Claude Code-credentials-c0cf3b6b");
}

#[test]
fn test_keychain_service_known_hash_company() {
    // SHA256("/Users/1112298/.claude-accounts/company")[:4] → 1b5ba2bc
    let service = dir_keychain_service(Path::new("/Users/1112298/.claude-accounts/company"));
    assert_eq!(service, "Claude Code-credentials-1b5ba2bc");
}

#[test]
fn test_keychain_service_format() {
    let service = dir_keychain_service(Path::new("/some/path"));
    assert!(service.starts_with("Claude Code-credentials-"));
    assert_eq!(service.len(), "Claude Code-credentials-".len() + 8);
}

// -- auth_status --

#[test]
fn test_auth_status_no_keychain() {
    let tmp = TempDir::new().unwrap();
    let status = auth_status(tmp.path());
    assert!(!status.keychain);
}
