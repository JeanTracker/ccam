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

// --- shared config_dir: use command behavior ---

/// Accounts sharing a config_dir produce the same export statement when switched to.
/// CLAUDE_CONFIG_DIR alone cannot distinguish which of the sharing accounts is active.
#[test]
fn use_shared_dir_accounts_produce_identical_export_statement() {
    let ctx = TestHome::new();
    let shared_dir = ctx.accounts_dir().join("shared");

    add_account("account1", shared_dir.clone(), None).unwrap();
    add_account("account2", shared_dir.clone(), None).unwrap();

    let cfg = ccam::config::load().unwrap();
    let stmt1 = export_statement(&cfg.accounts["account1"]);
    let stmt2 = export_statement(&cfg.accounts["account2"]);

    assert_eq!(
        stmt1, stmt2,
        "shared-path accounts must produce identical export statements"
    );
    assert!(
        stmt1.contains(&shared_dir.to_string_lossy().to_string()),
        "export statement must contain the shared dir path"
    );
}

/// Switching to account A updates only A's user info, not B's,
/// even when A and B share the same config_dir.
#[test]
fn use_shared_dir_updates_only_selected_account_info() {
    let ctx = TestHome::new();
    let shared_dir = ctx.accounts_dir().join("shared");

    add_account("account1", shared_dir.clone(), None).unwrap();
    add_account("account2", shared_dir.clone(), None).unwrap();

    // switch to account1 and receive user info
    run_inner(
        "account1",
        false,
        |_| true,
        |_| {
            Some(UserInfo {
                email: "user1@example.com".to_string(),
                subscription_type: "pro".to_string(),
            })
        },
    )
    .unwrap();

    let account1 = get_account("account1").unwrap();
    assert_eq!(account1.email.as_deref(), Some("user1@example.com"));

    // account2 was never switched to; its info must remain unchanged
    let account2 = get_account("account2").unwrap();
    assert!(
        account2.email.is_none(),
        "account2 must not have its info changed: only account1 was switched to"
    );
}

/// Sequential switches between shared-path accounts update each account's info independently.
#[test]
fn use_shared_dir_sequential_switch_updates_each_account_independently() {
    let ctx = TestHome::new();
    let shared_dir = ctx.accounts_dir().join("shared");

    add_account("account1", shared_dir.clone(), None).unwrap();
    add_account("account2", shared_dir.clone(), None).unwrap();

    run_inner(
        "account1",
        false,
        |_| true,
        |_| {
            Some(UserInfo {
                email: "user1@example.com".to_string(),
                subscription_type: "pro".to_string(),
            })
        },
    )
    .unwrap();

    run_inner(
        "account2",
        false,
        |_| true,
        |_| {
            Some(UserInfo {
                email: "user2@example.com".to_string(),
                subscription_type: "free".to_string(),
            })
        },
    )
    .unwrap();

    let a1 = get_account("account1").unwrap();
    let a2 = get_account("account2").unwrap();
    assert_eq!(a1.email.as_deref(), Some("user1@example.com"));
    assert_eq!(a2.email.as_deref(), Some("user2@example.com"));
    assert_eq!(a1.subscription_type.as_deref(), Some("pro"));
    assert_eq!(a2.subscription_type.as_deref(), Some("free"));
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
