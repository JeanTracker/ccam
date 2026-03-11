/// Unit tests for commands::remove::run_inner and run_inner_with_log.
///
/// The only injected dependency is logout_fn (Keychain/claude auth).
/// Confirmation is handled in run() via --yes; cancel behavior is covered
/// by shell integration tests (shell_integration.rs).
use ccam::commands::remove::{run_inner, run_inner_with_log};
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

// --- shared config_dir: keychain preservation ---

/// When two accounts share a config_dir, removing one must NOT call logout for the keychain entry
/// because the remaining account still depends on it.
///
/// Current behavior (BUG): logout_fn is called unconditionally regardless of other accounts
/// sharing the same path. Only directory and keychain deletion should be skipped when another
/// account still references the same config_dir.
#[test]
fn remove_one_of_two_shared_dir_accounts_skips_keychain_logout() {
    let ctx = TestHome::new();
    let shared_dir = ctx.account_dir("shared");
    add_account("account1", shared_dir.clone(), None).unwrap();
    add_account("account2", shared_dir.clone(), None).unwrap();

    let logout_called = std::cell::Cell::new(false);
    let logout_tracking = |_: &std::path::Path| -> anyhow::Result<()> {
        logout_called.set(true);
        Ok(())
    };

    run_inner("account1", None, logout_tracking).unwrap();

    assert!(
        !logout_called.get(),
        "keychain logout must be skipped: 'account2' still shares the same config_dir"
    );
}

/// When the last account referencing a config_dir is removed, logout must be called
/// to clean up the keychain entry.
#[test]
fn remove_last_shared_dir_account_calls_keychain_logout() {
    let ctx = TestHome::new();
    let shared_dir = ctx.account_dir("shared");
    add_account("account1", shared_dir.clone(), None).unwrap();
    add_account("account2", shared_dir.clone(), None).unwrap();

    // Remove first account (shared: logout skipped)
    run_inner("account1", None, logout_noop).unwrap();

    // Remove last account: logout must be called
    let logout_called = std::cell::Cell::new(false);
    let logout_tracking = |_: &std::path::Path| -> anyhow::Result<()> {
        logout_called.set(true);
        Ok(())
    };

    run_inner("account2", None, logout_tracking).unwrap();

    assert!(
        logout_called.get(),
        "keychain logout must be called when the last account referencing the config_dir is removed"
    );
}

/// A sole account (unique config_dir) must always trigger keychain logout on removal.
/// Ensures the shared-path check does not suppress logout for unshared dirs.
#[test]
fn remove_sole_account_for_dir_calls_keychain_logout() {
    let ctx = TestHome::new();
    let unique_dir = ctx.account_dir("unique");
    add_account("unique", unique_dir.clone(), None).unwrap();

    let logout_called = std::cell::Cell::new(false);
    let logout_tracking = |_: &std::path::Path| -> anyhow::Result<()> {
        logout_called.set(true);
        Ok(())
    };

    run_inner("unique", None, logout_tracking).unwrap();

    assert!(
        logout_called.get(),
        "keychain logout must always be called for an account with a unique config_dir"
    );
}

/// Mixed scenario: one shared-dir account and one unique-dir account.
/// Removing the shared-dir account must skip logout; removing the unique-dir account must call it.
#[test]
fn remove_shared_and_unique_dir_accounts_logout_behavior_differs() {
    let ctx = TestHome::new();
    let shared_dir = ctx.account_dir("shared");
    let unique_dir = ctx.account_dir("unique");
    add_account("shared1", shared_dir.clone(), None).unwrap();
    add_account("shared2", shared_dir.clone(), None).unwrap();
    add_account("unique", unique_dir.clone(), None).unwrap();

    // Remove shared1: another account still uses shared_dir → logout must be skipped
    let shared_logout_called = std::cell::Cell::new(false);
    run_inner("shared1", None, |_| {
        shared_logout_called.set(true);
        Ok(())
    })
    .unwrap();
    assert!(
        !shared_logout_called.get(),
        "logout must be skipped for shared1: shared2 still uses the same config_dir"
    );

    // Remove unique: sole account for its dir → logout must be called
    let unique_logout_called = std::cell::Cell::new(false);
    run_inner("unique", None, |_| {
        unique_logout_called.set(true);
        Ok(())
    })
    .unwrap();
    assert!(
        unique_logout_called.get(),
        "logout must be called for unique: it is the only account for its config_dir"
    );
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

// --- shared config_dir: directory preservation ---

/// When two accounts share a config_dir, removing one must NOT delete the directory
/// because the remaining account still depends on it.
///
/// Current behavior (BUG): run_inner deletes the dir unconditionally unless it is ~/.claude.
/// Expected behavior: check all remaining accounts; preserve the dir if any still reference it.
#[test]
fn remove_one_of_two_accounts_sharing_dir_preserves_directory() {
    let ctx = TestHome::new();
    let shared_dir = ctx.account_dir("shared");
    add_account("account1", shared_dir.clone(), None).unwrap();
    add_account("account2", shared_dir.clone(), None).unwrap();
    assert!(shared_dir.exists());

    run_inner("account1", None, logout_noop).unwrap();

    assert!(
        shared_dir.exists(),
        "shared dir must be preserved: 'account2' still references it"
    );
}

/// Once the last account referencing the shared dir is removed, the directory must be deleted.
///
/// Verifies that preservation is not permanent — it lasts only while at least one account
/// still points to the directory.
#[test]
fn remove_last_account_using_shared_dir_deletes_directory() {
    let ctx = TestHome::new();
    let shared_dir = ctx.account_dir("shared");
    add_account("account1", shared_dir.clone(), None).unwrap();
    add_account("account2", shared_dir.clone(), None).unwrap();

    run_inner("account1", None, logout_noop).unwrap();
    // account2 still exists → dir preserved
    assert!(shared_dir.exists());

    run_inner("account2", None, logout_noop).unwrap();
    // no accounts left referencing the dir → must be deleted
    assert!(
        !shared_dir.exists(),
        "shared dir must be deleted after the last account referencing it is removed"
    );
}

/// Three accounts share a dir; removing them one-by-one preserves the dir until the last.
///
/// Verifies that the shared-reference count is always re-evaluated after each removal.
#[test]
fn remove_three_accounts_sharing_dir_one_by_one() {
    let ctx = TestHome::new();
    let shared_dir = ctx.account_dir("shared");
    add_account("a", shared_dir.clone(), None).unwrap();
    add_account("b", shared_dir.clone(), None).unwrap();
    add_account("c", shared_dir.clone(), None).unwrap();

    run_inner("a", None, logout_noop).unwrap();
    assert!(
        shared_dir.exists(),
        "dir must exist: 'b' and 'c' still reference it"
    );

    run_inner("b", None, logout_noop).unwrap();
    assert!(
        shared_dir.exists(),
        "dir must exist: 'c' still references it"
    );

    run_inner("c", None, logout_noop).unwrap();
    assert!(
        !shared_dir.exists(),
        "dir must be deleted: no account references it anymore"
    );
}

/// Accounts that do NOT share a path are unaffected; their directories are still deleted
/// by the existing logic when removed.
///
/// Ensures the shared-dir preservation check does not accidentally protect unrelated dirs.
#[test]
fn remove_account_with_unique_dir_still_deletes_directory() {
    let ctx = TestHome::new();
    let shared_dir = ctx.account_dir("shared");
    let unique_dir = ctx.account_dir("unique");
    add_account("shared1", shared_dir.clone(), None).unwrap();
    add_account("shared2", shared_dir.clone(), None).unwrap();
    add_account("unique", unique_dir.clone(), None).unwrap();

    run_inner("unique", None, logout_noop).unwrap();

    assert!(
        !unique_dir.exists(),
        "unique dir must be deleted: no other account references it"
    );
    assert!(
        shared_dir.exists(),
        "shared dir must be preserved: shared1 and shared2 still use it"
    );
}

/// ~/.claude is always preserved regardless of how many accounts reference it.
/// Confirms the shared-dir logic does not interfere with the ~/.claude exception.
#[test]
fn remove_all_accounts_using_claude_dir_always_preserves_it() {
    let ctx = TestHome::new();
    std::fs::create_dir_all(ctx.claude_dir()).unwrap();
    add_account("base1", ctx.claude_dir(), None).unwrap();
    add_account("base2", ctx.claude_dir(), None).unwrap();

    run_inner("base1", None, logout_noop).unwrap();
    assert!(ctx.claude_dir().exists(), "~/.claude must never be deleted");

    run_inner("base2", None, logout_noop).unwrap();
    assert!(
        ctx.claude_dir().exists(),
        "~/.claude must never be deleted even when no account references it"
    );
}

/// Mixed scenario: shared custom dir and ~/.claude accounts coexist.
/// Removing one of the shared-dir accounts must preserve only the shared dir.
/// ~/.claude is always preserved independently.
#[test]
fn remove_shared_dir_account_alongside_claude_dir_accounts() {
    let ctx = TestHome::new();
    let shared_dir = ctx.account_dir("shared");
    std::fs::create_dir_all(ctx.claude_dir()).unwrap();
    add_account("base", ctx.claude_dir(), None).unwrap();
    add_account("account1", shared_dir.clone(), None).unwrap();
    add_account("account2", shared_dir.clone(), None).unwrap();

    run_inner("account1", None, logout_noop).unwrap();

    assert!(
        shared_dir.exists(),
        "shared dir must be preserved: 'account2' still references it"
    );
    assert!(
        ctx.claude_dir().exists(),
        "~/.claude must always be preserved"
    );
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

// --- output message assertions ---
//
// run_inner_with_log captures all diagnostic messages into a Vec<String>.
// colored output is suppressed to keep assertions simple.

fn collect_messages(
    ctx: &TestHome,
    alias: &str,
    logout_fn: impl Fn(&std::path::Path) -> anyhow::Result<()>,
) -> Vec<String> {
    let _ = ctx; // keep borrow alive (HOME env is set for this TestHome)
    colored::control::set_override(false);
    let messages = std::cell::RefCell::new(Vec::<String>::new());
    run_inner_with_log(alias, None, logout_fn, &mut |msg| {
        messages.borrow_mut().push(msg.to_string());
    })
    .unwrap();
    colored::control::unset_override();
    messages.into_inner()
}

/// When dir is shared, "Skipping Keychain cleanup" must appear
/// and "Cleaning up Keychain entry" must NOT appear.
#[test]
fn output_shared_dir_shows_skip_keychain_message() {
    let ctx = TestHome::new();
    let shared_dir = ctx.account_dir("shared");
    add_account("a", shared_dir.clone(), None).unwrap();
    add_account("b", shared_dir.clone(), None).unwrap();

    let msgs = collect_messages(&ctx, "a", logout_noop);

    let has_skip = msgs.iter().any(|m| m.contains("Skipping Keychain cleanup"));
    let has_cleanup = msgs
        .iter()
        .any(|m| m.contains("Cleaning up Keychain entry"));

    assert!(
        has_skip,
        "expected 'Skipping Keychain cleanup' message; got: {:?}",
        msgs
    );
    assert!(
        !has_cleanup,
        "unexpected 'Cleaning up Keychain entry' when dir is shared; got: {:?}",
        msgs
    );
}

/// When dir is NOT shared, "Cleaning up Keychain entry" must appear
/// and "Skipping Keychain cleanup" must NOT appear.
#[test]
fn output_sole_dir_shows_cleanup_keychain_message() {
    let ctx = TestHome::new();
    add_account("solo", ctx.account_dir("solo"), None).unwrap();

    let msgs = collect_messages(&ctx, "solo", logout_noop);

    let has_cleanup = msgs
        .iter()
        .any(|m| m.contains("Cleaning up Keychain entry"));
    let has_skip = msgs.iter().any(|m| m.contains("Skipping Keychain cleanup"));

    assert!(
        has_cleanup,
        "expected 'Cleaning up Keychain entry' for sole-dir account; got: {:?}",
        msgs
    );
    assert!(
        !has_skip,
        "unexpected 'Skipping Keychain cleanup' for sole-dir account; got: {:?}",
        msgs
    );
}

/// When dir is shared, "Skipping deletion" must appear
/// and "Deleted directory" must NOT appear.
#[test]
fn output_shared_dir_shows_skip_deletion_message() {
    let ctx = TestHome::new();
    let shared_dir = ctx.account_dir("shared");
    add_account("a", shared_dir.clone(), None).unwrap();
    add_account("b", shared_dir.clone(), None).unwrap();

    let msgs = collect_messages(&ctx, "a", logout_noop);

    let has_skip = msgs
        .iter()
        .any(|m| m.contains("Skipping deletion") && m.contains("another account"));
    let has_deleted = msgs.iter().any(|m| m.contains("Deleted directory"));

    assert!(
        has_skip,
        "expected 'Skipping deletion' message; got: {:?}",
        msgs
    );
    assert!(
        !has_deleted,
        "unexpected 'Deleted directory' when dir is shared; got: {:?}",
        msgs
    );
}

/// When dir is NOT shared, "Deleted directory" must appear
/// and "Skipping deletion" must NOT appear.
#[test]
fn output_sole_dir_shows_deleted_directory_message() {
    let ctx = TestHome::new();
    add_account("solo", ctx.account_dir("solo"), None).unwrap();

    let msgs = collect_messages(&ctx, "solo", logout_noop);

    let has_deleted = msgs.iter().any(|m| m.contains("Deleted directory"));
    let has_skip = msgs
        .iter()
        .any(|m| m.contains("Skipping deletion") && m.contains("another account"));

    assert!(
        has_deleted,
        "expected 'Deleted directory' for sole-dir account; got: {:?}",
        msgs
    );
    assert!(
        !has_skip,
        "unexpected 'Skipping deletion' for sole-dir account; got: {:?}",
        msgs
    );
}

/// For ~/.claude, "Skipping deletion of default directory" must appear.
#[test]
fn output_default_dir_shows_skip_default_directory_message() {
    let ctx = TestHome::new();
    std::fs::create_dir_all(ctx.claude_dir()).unwrap();
    add_account("base", ctx.claude_dir(), None).unwrap();

    colored::control::set_override(false);
    let messages = std::cell::RefCell::new(Vec::<String>::new());
    run_inner_with_log("base", None, logout_noop, &mut |msg| {
        messages.borrow_mut().push(msg.to_string());
    })
    .unwrap();
    colored::control::unset_override();
    let msgs = messages.into_inner();

    let has_skip_default = msgs
        .iter()
        .any(|m| m.contains("Skipping deletion of default directory"));

    assert!(
        has_skip_default,
        "expected 'Skipping deletion of default directory' for ~/.claude; got: {:?}",
        msgs
    );
}
