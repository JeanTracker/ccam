use ccam::config::{
    Account, add_account, expand_tilde, get_account, get_default, load, remove_account, set_default,
};
use colored::control;
use std::path::Path;
use std::sync::Mutex;

// Tests that mutate HOME must run serially to avoid global state conflicts
static HOME_LOCK: Mutex<()> = Mutex::new(());

struct TestHome {
    tmp: tempfile::TempDir,
    old_home: Option<String>,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl TestHome {
    fn new() -> Self {
        let guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::TempDir::new().unwrap();
        let old_home = std::env::var("HOME").ok();
        // SAFETY: serialized by HOME_LOCK mutex, no other threads modify HOME
        unsafe {
            std::env::set_var("HOME", tmp.path());
        }
        Self {
            tmp,
            old_home,
            _guard: guard,
        }
    }

    fn path(&self) -> &Path {
        self.tmp.path()
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        match &self.old_home {
            Some(h) => unsafe { std::env::set_var("HOME", h) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }
}

#[test]
fn test_expand_tilde_replaces_prefix() {
    let _ctx = TestHome::new();
    let expanded = expand_tilde(Path::new("~/foo/bar"));
    assert!(!expanded.starts_with("~"));
    assert!(expanded.ends_with("foo/bar"));
}

#[test]
fn test_expand_tilde_no_tilde_unchanged() {
    let path = Path::new("/absolute/path");
    assert_eq!(expand_tilde(path), path);
}

#[test]
fn test_add_account_creates_entry() {
    let ctx = TestHome::new();
    let config_dir = ctx.path().join("accounts").join("work");
    add_account("work", config_dir.clone(), None).unwrap();

    let cfg = load().unwrap();
    assert!(cfg.accounts.contains_key("work"));
    assert!(config_dir.exists());
}

#[test]
fn test_add_account_duplicate_fails() {
    let ctx = TestHome::new();
    let config_dir = ctx.path().join("accounts").join("work");
    add_account("work", config_dir.clone(), None).unwrap();
    let result = add_account("work", config_dir, None);
    assert!(result.is_err());
}

#[test]
fn test_remove_account_deletes_entry() {
    let ctx = TestHome::new();
    let config_dir = ctx.path().join("accounts").join("work");
    add_account("work", config_dir, None).unwrap();
    remove_account("work").unwrap();

    let cfg = load().unwrap();
    assert!(!cfg.accounts.contains_key("work"));
}

#[test]
fn test_remove_default_account_reassigns_default() {
    let ctx = TestHome::new();
    add_account("work", ctx.path().join("accounts").join("work"), None).unwrap();
    add_account(
        "personal",
        ctx.path().join("accounts").join("personal"),
        None,
    )
    .unwrap();
    set_default(Some("work")).unwrap();
    remove_account("work").unwrap();

    // Remaining account should become default
    assert_eq!(get_default().unwrap(), Some("personal".to_string()));
}

#[test]
fn test_remove_last_account_clears_default() {
    let ctx = TestHome::new();
    let config_dir = ctx.path().join("accounts").join("work");
    add_account("work", config_dir, None).unwrap();
    set_default(Some("work")).unwrap();
    remove_account("work").unwrap();

    assert_eq!(get_default().unwrap(), None);
}

#[test]
fn test_remove_nonexistent_fails() {
    let _ctx = TestHome::new();
    assert!(remove_account("ghost").is_err());
}

#[test]
fn test_get_account_returns_correct() {
    let ctx = TestHome::new();
    let config_dir = ctx.path().join("accounts").join("work");
    add_account("work", config_dir.clone(), Some("company".to_string())).unwrap();

    let account = get_account("work").unwrap();
    assert_eq!(account.config_dir, config_dir);
    assert_eq!(account.description.as_deref(), Some("company"));
}

#[test]
fn test_get_account_missing_fails() {
    let _ctx = TestHome::new();
    assert!(get_account("ghost").is_err());
}

#[test]
fn test_set_and_get_default() {
    let ctx = TestHome::new();
    let config_dir = ctx.path().join("accounts").join("work");
    add_account("work", config_dir, None).unwrap();

    set_default(Some("work")).unwrap();
    assert_eq!(get_default().unwrap(), Some("work".to_string()));
}

#[test]
fn test_set_default_nonexistent_fails() {
    let _ctx = TestHome::new();
    assert!(set_default(Some("ghost")).is_err());
}

#[test]
fn test_load_empty_when_no_file() {
    let _ctx = TestHome::new();
    let cfg = load().unwrap();
    assert!(cfg.accounts.is_empty());
    assert!(cfg.default.is_none());
}

// --- shared --dir path: config layer ---

/// Adding two accounts with the same `config_dir` must either be explicitly rejected,
/// or must store byte-identical path values so keychain service derivation is consistent.
///
/// Currently `add_account` only checks alias uniqueness, so two aliases can share a dir.
/// This test documents that behavior and fails if the contract breaks unexpectedly.
#[test]
fn test_add_account_same_dir_for_different_aliases_is_permitted() {
    let ctx = TestHome::new();
    let shared_dir = ctx.path().join("accounts").join("shared-dir");

    add_account("account1", shared_dir.clone(), None).unwrap();
    // account2 reuses account1's config_dir via --dir; must not panic or corrupt state
    let result = add_account("account2", shared_dir.clone(), None);
    assert!(
        result.is_ok(),
        "adding a second alias for an existing config_dir should not crash; \
         path-uniqueness enforcement (if added) must return a clear error instead"
    );
}

/// When two accounts are registered for the same `config_dir`, the stored path strings
/// must be byte-for-byte identical so that `keychain_service` returns the same value for both.
///
/// A mismatch (e.g. one stored with tilde, one expanded) would cause one account to appear
/// authenticated and the other not, even though they share the same keychain entry.
#[test]
fn test_add_account_same_dir_stores_identical_path_strings() {
    let ctx = TestHome::new();
    let shared_dir = ctx.path().join("accounts").join("shared-dir");

    add_account("account1", shared_dir.clone(), None).unwrap();
    add_account("account2", shared_dir.clone(), None).unwrap();

    let cfg = load().unwrap();
    let path1 = &cfg.accounts["account1"].config_dir;
    let path2 = &cfg.accounts["account2"].config_dir;

    assert_eq!(
        path1, path2,
        "both accounts must store identical config_dir strings; \
         any difference causes keychain_service to produce different hashes"
    );
}

/// When two accounts share a `config_dir`, their derived keychain service names must match.
///
/// This is the end-to-end assertion: if path strings are identical, service names must be
/// identical, so `auth_status` returns the same result for both accounts.
#[test]
fn test_two_accounts_same_dir_produce_identical_keychain_service() {
    let ctx = TestHome::new();
    let shared_dir = ctx.path().join("accounts").join("shared-dir");

    add_account("account1", shared_dir.clone(), None).unwrap();
    add_account("account2", shared_dir.clone(), None).unwrap();

    let cfg = load().unwrap();
    let svc1 = ccam::claude::keychain_service(&cfg.accounts["account1"].config_dir);
    let svc2 = ccam::claude::keychain_service(&cfg.accounts["account2"].config_dir);

    assert_eq!(
        svc1, svc2,
        "accounts sharing config_dir must derive the same keychain service name; \
         if account1 is authenticated, account2 must also appear authenticated"
    );
}

// --- Account methods ---

fn make_account(email: Option<&str>, subscription_type: Option<&str>) -> Account {
    Account {
        config_dir: std::path::PathBuf::from("/tmp/test"),
        description: None,
        added_at: "2026-01-01T00:00:00Z".to_string(),
        email: email.map(str::to_string),
        subscription_type: subscription_type.map(str::to_string),
    }
}

#[test]
fn test_display_name_returns_email() {
    let account = make_account(Some("user@example.com"), None);
    assert_eq!(account.display_name(), "user@example.com");
}

#[test]
fn test_display_name_returns_empty_when_no_email() {
    let account = make_account(None, None);
    assert_eq!(account.display_name(), "");
}

#[test]
fn test_sub_tag_with_subscription() {
    control::set_override(false);
    let account = make_account(None, Some("pro"));
    assert_eq!(account.sub_tag(), " (pro)");
}

#[test]
fn test_sub_tag_empty_when_no_subscription() {
    let account = make_account(None, None);
    assert_eq!(account.sub_tag(), "");
}
