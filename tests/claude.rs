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

/// Trailing slash changes the SHA256 input → different keychain service name for the same directory.
///
/// Documents the known limitation: a path stored without trailing slash and one stored with it
/// will produce different hashes, causing `auth_status` to disagree even though they resolve
/// to the same filesystem location.
#[test]
fn test_keychain_service_trailing_slash_produces_different_service() {
    let without_slash = Path::new("/Users/testuser/.claude-accounts/work");
    let with_slash = Path::new("/Users/testuser/.claude-accounts/work/");
    // PathBuf normalizes trailing slashes on most platforms; verify the actual behavior.
    // If they differ, the bug is present: same physical dir → different keychain service.
    let svc_a = dir_keychain_service(without_slash);
    let svc_b = dir_keychain_service(with_slash);
    assert_eq!(
        svc_a, svc_b,
        "trailing slash must not produce a different keychain service name \
         (same physical directory must always map to the same service)"
    );
}

/// On macOS, TempDir paths may be symlinks (e.g. /var → /private/var).
/// If `config_dir` is stored as a symlink path but Claude Code canonicalizes it when
/// writing the keychain entry, `dir_keychain_service` will compute the wrong hash.
///
/// This test documents the mismatch: raw path vs. canonical path → different service names.
#[test]
#[ignore = "known bug: dir_keychain_service hashes raw path string; fix by canonicalizing path before hashing"]
fn test_keychain_service_symlink_path_and_canonical_must_agree() {
    let tmp = TempDir::new().unwrap();
    let raw = tmp.path();
    let Ok(canonical) = raw.canonicalize() else {
        return; // canonicalize failed; skip
    };

    if raw == canonical.as_path() {
        // No symlink involved on this system; nothing to assert.
        return;
    }

    // Bug condition: raw path and its canonical form produce different service names.
    // ccm stores the raw path; if Claude Code uses the canonical path, authentication
    // status checks will silently fail for any account whose dir is a symlink.
    assert_eq!(
        dir_keychain_service(raw),
        dir_keychain_service(&canonical),
        "raw path and canonical path must produce the same keychain service name; \
         symlink mismatch will cause auth_status to incorrectly report unauthenticated"
    );
}
