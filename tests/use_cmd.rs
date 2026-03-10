use ccam::claude::UserInfo;
use ccam::commands::env::{export_statement, run_inner};
use ccam::config::{Account, add_account, get_account, set_default};
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
        let guard = HOME_LOCK.lock().unwrap();
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
    fn accounts_dir(&self) -> PathBuf {
        self.tmp.path().join(".claude-accounts")
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

fn make_account(config_dir: PathBuf) -> Account {
    Account {
        config_dir,
        description: None,
        added_at: "2026-01-01T00:00:00Z".to_string(),
        email: None,
        subscription_type: None,
    }
}

// --- export_statement ---

#[test]
fn export_statement_unsets_for_default_claude_dir() {
    let ctx = TestHome::new();
    let account = make_account(ctx.claude_dir());
    assert_eq!(export_statement(&account), "unset CLAUDE_CONFIG_DIR");
}

#[test]
fn export_statement_exports_for_custom_dir() {
    let ctx = TestHome::new();
    let dir = ctx.accounts_dir().join("work");
    let account = make_account(dir.clone());
    assert_eq!(
        export_statement(&account),
        format!("export CLAUDE_CONFIG_DIR=\"{}\"", dir.display())
    );
}

// --- run_inner: no_refresh ---

#[test]
fn no_refresh_skips_fetch_and_config_update() {
    let ctx = TestHome::new();
    add_account("work", ctx.accounts_dir().join("work"), None).unwrap();

    let fetch_called = std::cell::Cell::new(false);
    run_inner(
        "work",
        true,
        |_| true,
        |_| {
            fetch_called.set(true);
            None
        },
    )
    .unwrap();

    assert!(
        !fetch_called.get(),
        "fetch should not be called with no_refresh=true"
    );
    let account = get_account("work").unwrap();
    assert!(account.email.is_none());
}

// --- run_inner: refresh with mocked auth + fetch ---

#[test]
fn refresh_updates_config_when_logged_in() {
    let ctx = TestHome::new();
    add_account("work", ctx.accounts_dir().join("work"), None).unwrap();

    run_inner(
        "work",
        false,
        |_| true, // mock: logged in
        |_| {
            Some(UserInfo {
                email: "test@example.com".to_string(),
                subscription_type: "pro".to_string(),
            })
        },
    )
    .unwrap();

    let account = get_account("work").unwrap();
    assert_eq!(account.email.as_deref(), Some("test@example.com"));
    assert_eq!(account.subscription_type.as_deref(), Some("pro"));
}

#[test]
fn refresh_skips_update_when_logged_out() {
    let ctx = TestHome::new();
    add_account("work", ctx.accounts_dir().join("work"), None).unwrap();

    run_inner(
        "work",
        false,
        |_| false,
        |_| panic!("fetch should not be called when logged out"),
    )
    .unwrap();

    let account = get_account("work").unwrap();
    assert!(account.email.is_none());
}

#[test]
fn refresh_keeps_old_info_when_fetch_fails() {
    let ctx = TestHome::new();
    add_account("work", ctx.accounts_dir().join("work"), None).unwrap();

    // First run: populate info
    run_inner(
        "work",
        false,
        |_| true,
        |_| {
            Some(UserInfo {
                email: "old@example.com".to_string(),
                subscription_type: "pro".to_string(),
            })
        },
    )
    .unwrap();

    // Second run: fetch fails → config unchanged
    run_inner("work", false, |_| true, |_| None).unwrap();

    let account = get_account("work").unwrap();
    assert_eq!(account.email.as_deref(), Some("old@example.com"));
}

// --- default marker ---

#[test]
fn default_account_reflected_after_set_default() {
    let ctx = TestHome::new();
    add_account("work", ctx.accounts_dir().join("work"), None).unwrap();
    set_default(Some("work")).unwrap();

    // run_inner completes without error and account is still default
    run_inner("work", false, |_| false, |_| None).unwrap();

    let cfg = ccam::config::load().unwrap();
    assert_eq!(cfg.default.as_deref(), Some("work"));
}
