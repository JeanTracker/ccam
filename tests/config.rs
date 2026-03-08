use ccam::config::{
    add_account, expand_tilde, get_account, get_default, load, remove_account, set_default,
};
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
        let guard = HOME_LOCK.lock().unwrap();
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
