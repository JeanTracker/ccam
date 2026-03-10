/// Composite workflow tests covering multi-command sequences.
use ccam::claude::UserInfo;
use ccam::commands::env::run_inner;
use ccam::commands::status::{format_account_line, resolve_default_dir_account};
use ccam::config::{add_account, get_account, get_default, load, remove_account, set_default};
use colored::control;
use std::path::PathBuf;
use std::sync::Mutex;

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
        unsafe { std::env::set_var("HOME", tmp.path()) }
        Self {
            tmp,
            old_home,
            _guard: guard,
        }
    }
    fn claude_dir(&self) -> PathBuf {
        self.tmp.path().join(".claude")
    }
    fn account_dir(&self, name: &str) -> PathBuf {
        self.tmp.path().join(".claude-accounts").join(name)
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

// --- add → ls → remove ---

#[test]
fn add_multiple_ls_sorted_then_remove() {
    let ctx = TestHome::new();
    add_account("zebra", ctx.account_dir("zebra"), None).unwrap();
    add_account("alpha", ctx.account_dir("alpha"), None).unwrap();
    add_account("middle", ctx.account_dir("middle"), None).unwrap();

    let cfg = load().unwrap();
    let mut keys: Vec<&str> = cfg.accounts.keys().map(|s| s.as_str()).collect();
    keys.sort();
    assert_eq!(keys, vec!["alpha", "middle", "zebra"]);

    remove_account("middle").unwrap();
    let cfg = load().unwrap();
    assert!(!cfg.accounts.contains_key("middle"));
    assert!(cfg.accounts.contains_key("alpha"));
    assert!(cfg.accounts.contains_key("zebra"));
}

// --- add → default set → default --get ---

#[test]
fn add_set_default_get_default() {
    let ctx = TestHome::new();
    add_account("work", ctx.account_dir("work"), None).unwrap();
    add_account("personal", ctx.account_dir("personal"), None).unwrap();

    set_default(Some("work")).unwrap();
    assert_eq!(get_default().unwrap(), Some("work".to_string()));

    set_default(Some("personal")).unwrap();
    assert_eq!(get_default().unwrap(), Some("personal".to_string()));
}

// --- add → default set → remove default → auto-reassign ---

#[test]
fn remove_default_auto_reassigns_remaining() {
    let ctx = TestHome::new();
    add_account("alpha", ctx.account_dir("alpha"), None).unwrap();
    add_account("beta", ctx.account_dir("beta"), None).unwrap();
    set_default(Some("alpha")).unwrap();

    remove_account("alpha").unwrap();

    // default reassigned to the only remaining account
    assert_eq!(get_default().unwrap(), Some("beta".to_string()));
    assert!(!load().unwrap().accounts.contains_key("alpha"));
}

// --- add → use (mocked) → active display ---

#[test]
fn use_refresh_then_active_shows_updated_info() {
    let ctx = TestHome::new();
    add_account("work", ctx.account_dir("work"), None).unwrap();
    set_default(Some("work")).unwrap();

    // Simulate: use work (logged in, fetch succeeds)
    run_inner(
        "work",
        false,
        |_| true,
        |_| {
            Some(UserInfo {
                email: "work@example.com".to_string(),
                subscription_type: "pro".to_string(),
            })
        },
    )
    .unwrap();

    // Verify config updated
    let account = get_account("work").unwrap();
    assert_eq!(account.email.as_deref(), Some("work@example.com"));

    // Verify active display reflects updated info
    control::set_override(false);
    let cfg = load().unwrap();
    let account = cfg.accounts.get("work").unwrap();
    let line = format_account_line("work", account, true, true);
    assert_eq!(line, "* work work@example.com (pro)");
}

// --- add --dir ~/.claude → active fallback resolves ---

#[test]
fn add_claude_dir_then_active_resolves() {
    let ctx = TestHome::new();
    add_account("base", ctx.claude_dir(), None).unwrap();
    add_account("work", ctx.account_dir("work"), None).unwrap();
    set_default(Some("base")).unwrap();

    let cfg = load().unwrap();
    assert_eq!(resolve_default_dir_account(&cfg), Some("base"));
}

#[test]
fn add_claude_dir_no_default_active_resolves_alphabetically() {
    let ctx = TestHome::new();
    add_account("zebra", ctx.claude_dir(), None).unwrap();
    add_account("alpha", ctx.claude_dir(), None).unwrap();
    set_default(None).unwrap(); // explicitly clear default to test alphabetical fallback

    let cfg = load().unwrap();
    assert_eq!(resolve_default_dir_account(&cfg), Some("alpha"));
}

// --- add → use switch → ls prefix ---

#[test]
fn ls_prefix_reflects_default_after_set() {
    let ctx = TestHome::new();
    add_account("work", ctx.account_dir("work"), None).unwrap();
    add_account("personal", ctx.account_dir("personal"), None).unwrap();
    set_default(Some("work")).unwrap();

    control::set_override(false);
    let cfg = load().unwrap();

    let work = cfg.accounts.get("work").unwrap();
    let personal = cfg.accounts.get("personal").unwrap();

    let work_line = format_account_line("work", work, false, true);
    let personal_line = format_account_line("personal", personal, false, false);

    // default account: ! prefix (logged out) but alias is the default
    assert!(work_line.starts_with("! work"));
    assert!(personal_line.starts_with("! personal"));

    // switching default changes the marker
    set_default(Some("personal")).unwrap();
    let cfg = load().unwrap();
    let is_work_default = cfg.default.as_deref() == Some("work");
    let is_personal_default = cfg.default.as_deref() == Some("personal");
    assert!(!is_work_default);
    assert!(is_personal_default);
}
