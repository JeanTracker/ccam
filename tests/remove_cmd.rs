/// Unit tests for commands::remove::run_inner.
///
/// The only injected dependency is logout_fn (Keychain/claude auth).
/// Confirmation is handled in run() via --yes; cancel behavior is covered
/// by shell integration tests (shell_integration.rs).
use ccam::commands::remove::run_inner;
use ccam::config::{add_account, get_default, load, set_default};
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

    fn account_dir(&self, name: &str) -> PathBuf {
        self.tmp.path().join(".claude-accounts").join(name)
    }

    fn claude_dir(&self) -> PathBuf {
        self.tmp.path().join(".claude")
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

fn logout_noop(_: &std::path::Path) -> anyhow::Result<()> {
    Ok(())
}

// --- inactive account ---

#[test]
fn remove_inactive_account_returns_no_eval_stmt() {
    let ctx = TestHome::new();
    add_account("work", ctx.account_dir("work"), None).unwrap();
    add_account("personal", ctx.account_dir("personal"), None).unwrap();

    let active = ctx.account_dir("personal").to_string_lossy().into_owned();
    let result = run_inner("work", Some(&active), logout_noop).unwrap();

    assert!(result.is_none(), "no eval stmt needed for inactive account");
    assert!(!load().unwrap().accounts.contains_key("work"));
}

#[test]
fn remove_inactive_account_deletes_directory() {
    let ctx = TestHome::new();
    let work_dir = ctx.account_dir("work");
    add_account("work", work_dir.clone(), None).unwrap();
    assert!(work_dir.exists());

    run_inner("work", None, logout_noop).unwrap();

    assert!(
        !work_dir.exists(),
        "custom dir should be deleted after removal"
    );
}

// --- active custom-dir account ---

#[test]
fn remove_active_custom_dir_switches_to_default() {
    let ctx = TestHome::new();
    add_account("work", ctx.account_dir("work"), None).unwrap();
    add_account("personal", ctx.account_dir("personal"), None).unwrap();
    set_default(Some("personal")).unwrap();

    let active = ctx.account_dir("work").to_string_lossy().into_owned();
    let result = run_inner("work", Some(&active), logout_noop).unwrap();

    let stmt = result.expect("eval stmt required when active account is deleted");
    assert!(
        stmt.contains("personal"),
        "eval stmt should switch to default 'personal', got: {}",
        stmt
    );
    assert!(stmt.starts_with("export CLAUDE_CONFIG_DIR="));
}

#[test]
fn remove_active_custom_dir_no_accounts_left_unsets() {
    let ctx = TestHome::new();
    add_account("work", ctx.account_dir("work"), None).unwrap();

    let active = ctx.account_dir("work").to_string_lossy().into_owned();
    let result = run_inner("work", Some(&active), logout_noop).unwrap();

    assert_eq!(
        result.expect("eval stmt required"),
        "unset CLAUDE_CONFIG_DIR"
    );
}

// --- active ~/.claude account (CLAUDE_CONFIG_DIR unset) ---

#[test]
fn remove_active_claude_dir_account_switches_to_default() {
    let ctx = TestHome::new();
    std::fs::create_dir_all(ctx.claude_dir()).unwrap();
    add_account("base", ctx.claude_dir(), None).unwrap();
    add_account("work", ctx.account_dir("work"), None).unwrap();
    set_default(Some("work")).unwrap();

    let result = run_inner("base", None, logout_noop).unwrap();

    let stmt = result.expect("eval stmt required");
    assert!(
        stmt.contains("work"),
        "eval stmt should switch to 'work', got: {}",
        stmt
    );
}

#[test]
fn remove_active_claude_dir_no_accounts_left_unsets() {
    let ctx = TestHome::new();
    std::fs::create_dir_all(ctx.claude_dir()).unwrap();
    add_account("base", ctx.claude_dir(), None).unwrap();

    let result = run_inner("base", None, logout_noop).unwrap();

    assert_eq!(
        result.expect("eval stmt required"),
        "unset CLAUDE_CONFIG_DIR"
    );
    assert!(ctx.claude_dir().exists(), "~/.claude must not be deleted");
}

// --- default reassignment ---

#[test]
fn remove_default_account_reassigns_to_alphabetical_first() {
    let ctx = TestHome::new();
    add_account("beta", ctx.account_dir("beta"), None).unwrap();
    add_account("alpha", ctx.account_dir("alpha"), None).unwrap();
    set_default(Some("beta")).unwrap();

    run_inner("beta", None, logout_noop).unwrap();

    assert_eq!(get_default().unwrap(), Some("alpha".to_string()));
}

#[test]
fn remove_nonexistent_account_returns_error() {
    let _ctx = TestHome::new();
    assert!(run_inner("ghost", None, logout_noop).is_err());
}
