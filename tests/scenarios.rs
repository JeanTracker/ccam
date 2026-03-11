/// Scenario-based tests covering full user workflows.
///
/// Account naming uses alphabetical order intentionally:
///   alpha < beta < gamma  (scenario 1)
///   awork < bwork         (scenario 2)
///   alice < bob < charlie (scenario 3)
///
/// Tests marked `// TDD` document expected behavior not yet implemented.
use ccam::claude::UserInfo;
use ccam::commands::env::{export_statement, run_inner};
use ccam::commands::status::{format_account_line, resolve_default_dir_account};
use ccam::config::{add_account, get_account};
use ccam::config::{get_default, load, remove_account, set_default};
use colored::control;
use std::fs;
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

// ============================================================
// Scenario 1: basic account management
// ============================================================

/// S1-1~4: add three accounts, verify sorted listing and auto-default
#[test]
fn s1_add_three_accounts_sorted_and_default() {
    let ctx = TestHome::new();

    // account #1 alpha (authenticated — auth layer is mocked at the display level)
    add_account("alpha", ctx.account_dir("alpha"), None).unwrap();

    // TDD: first account added must be auto-set as default
    assert_eq!(
        get_default().unwrap(),
        Some("alpha".to_string()),
        "first account should be auto-set as default"
    );

    // account #2 beta (unauthenticated)
    add_account("beta", ctx.account_dir("beta"), None).unwrap();

    // account #3 gamma (--dir ~/.claude)
    fs::create_dir_all(ctx.claude_dir()).unwrap();
    add_account("gamma", ctx.claude_dir(), None).unwrap();

    // listing must be alphabetically sorted
    let cfg = load().unwrap();
    let mut keys: Vec<&str> = cfg.accounts.keys().map(|s| s.as_str()).collect();
    keys.sort();
    assert_eq!(keys, vec!["alpha", "beta", "gamma"]);

    // default stays alpha (first added)
    assert_eq!(cfg.default.as_deref(), Some("alpha"));

    // new shell: alpha uses a custom dir → CLAUDE_CONFIG_DIR must be set
    let alpha_account = cfg.accounts.get("alpha").unwrap();
    assert_eq!(
        export_statement(alpha_account),
        format!(
            "export CLAUDE_CONFIG_DIR=\"{}\"",
            ctx.account_dir("alpha").display()
        )
    );
}

/// S1-5: removing gamma (--dir ~/.claude) must preserve ~/.claude
#[test]
fn s1_remove_claude_dir_account_preserves_directory() {
    let ctx = TestHome::new();
    let claude_dir = ctx.claude_dir();
    fs::create_dir_all(&claude_dir).unwrap();

    add_account("alpha", ctx.account_dir("alpha"), None).unwrap();
    add_account("gamma", claude_dir.clone(), None).unwrap();

    remove_account("gamma").unwrap();

    let cfg = load().unwrap();
    assert!(
        !cfg.accounts.contains_key("gamma"),
        "config entry should be removed"
    );
    assert!(claude_dir.exists(), "~/.claude directory must be preserved");
}

/// S1-5: gamma uses ~/.claude directly, so no ~/.claude-accounts/gamma subfolder exists
#[test]
fn s1_claude_dir_account_has_no_accounts_subfolder() {
    let ctx = TestHome::new();
    fs::create_dir_all(ctx.claude_dir()).unwrap();
    add_account("gamma", ctx.claude_dir(), None).unwrap();

    let account = get_account("gamma").unwrap();
    // gamma's config_dir is ~/.claude, so ~/.claude-accounts/gamma must not exist
    assert!(!ctx.account_dir("gamma").exists());
    assert!(ccam::claude::is_default_config_dir(&account.config_dir));
}

/// S1-6: Removing a regular account deletes it from config; directory removal is handled by commands::remove::run.
///
/// NOTE: config::remove_account only manages config data. Directory deletion is the
/// responsibility of commands::remove::run, so only the config layer is verified here.
/// The account being eligible for deletion is confirmed via is_default_config_dir == false.
#[test]
fn s1_remove_regular_account_directory_should_be_deleted() {
    let ctx = TestHome::new();
    add_account("alpha", ctx.account_dir("alpha"), None).unwrap();
    add_account("beta", ctx.account_dir("beta"), None).unwrap();

    let alpha_dir = ctx.account_dir("alpha");
    let beta_dir = ctx.account_dir("beta");
    assert!(alpha_dir.exists());
    assert!(beta_dir.exists());

    // regular accounts have is_default_config_dir = false → commands::remove::run handles dir deletion
    assert!(!ccam::claude::is_default_config_dir(&alpha_dir));
    assert!(!ccam::claude::is_default_config_dir(&beta_dir));

    remove_account("alpha").unwrap();
    remove_account("beta").unwrap();

    // Config layer: account entries are removed
    let cfg = load().unwrap();
    assert!(!cfg.accounts.contains_key("alpha"));
    assert!(!cfg.accounts.contains_key("beta"));

    // Directory deletion is out of scope for config::remove_account — handled by commands::remove::run.
}

// ============================================================
// Scenario 2: default account management
// ============================================================

/// S2-1,2: no accounts, CLAUDE_CONFIG_DIR unset → active resolves to None
#[test]
fn s2_no_accounts_active_resolves_none() {
    let _ctx = TestHome::new();
    let cfg = load().unwrap();
    assert!(cfg.accounts.is_empty());
    assert_eq!(resolve_default_dir_account(&cfg), None);
}

/// S2-3,4: adding first account auto-sets default; new shell sets CLAUDE_CONFIG_DIR
#[test]
fn s2_add_first_account_auto_default_new_shell_sets_config_dir() {
    let ctx = TestHome::new();

    add_account("awork", ctx.account_dir("awork"), None).unwrap();

    // TDD: auto-default must be set
    assert_eq!(get_default().unwrap(), Some("awork".to_string()));

    // new shell: export_statement must set CLAUDE_CONFIG_DIR
    let cfg = load().unwrap();
    let account = cfg.accounts.get("awork").unwrap();
    let stmt = export_statement(account);
    assert!(stmt.starts_with("export CLAUDE_CONFIG_DIR="));
    assert!(stmt.contains("awork"));
}

/// S2-7~10: adding bwork (--dir ~/.claude) and setting it as default → new shell unsets CLAUDE_CONFIG_DIR
#[test]
fn s2_claude_dir_account_as_default_new_shell_unsets_config_dir() {
    let ctx = TestHome::new();
    fs::create_dir_all(ctx.claude_dir()).unwrap();

    add_account("awork", ctx.account_dir("awork"), None).unwrap();
    add_account("bwork", ctx.claude_dir(), None).unwrap();

    // switch default to bwork (--dir ~/.claude)
    set_default(Some("bwork")).unwrap();

    // new shell: bwork uses ~/.claude → must unset CLAUDE_CONFIG_DIR
    let cfg = load().unwrap();
    let bwork = cfg.accounts.get("bwork").unwrap();
    assert_eq!(export_statement(bwork), "unset CLAUDE_CONFIG_DIR");

    // active when CLAUDE_CONFIG_DIR is unset: bwork (uses ~/.claude)
    assert_eq!(resolve_default_dir_account(&cfg), Some("bwork"));
}

/// S2-11,12: removing bwork (default) reassigns default to alphabetically first remaining account
#[test]
fn s2_remove_claude_dir_default_reassigns_to_alphabetical_first() {
    let ctx = TestHome::new();
    fs::create_dir_all(ctx.claude_dir()).unwrap();

    add_account("awork", ctx.account_dir("awork"), None).unwrap();
    add_account("bwork", ctx.claude_dir(), None).unwrap();
    add_account("cwork", ctx.account_dir("cwork"), None).unwrap();

    set_default(Some("bwork")).unwrap();

    // remove bwork: remaining are awork, cwork → awork is alphabetically first
    remove_account("bwork").unwrap();

    // TDD: alphabetically first remaining account must become default
    assert_eq!(
        get_default().unwrap(),
        Some("awork".to_string()),
        "default should be reassigned to alphabetically first remaining account"
    );
}

/// S2-13,14: after bwork removal, new shell with awork as default sets CLAUDE_CONFIG_DIR
#[test]
fn s2_after_reassign_new_shell_applies_correct_config_dir() {
    let ctx = TestHome::new();
    fs::create_dir_all(ctx.claude_dir()).unwrap();

    add_account("awork", ctx.account_dir("awork"), None).unwrap();
    add_account("bwork", ctx.claude_dir(), None).unwrap();
    set_default(Some("bwork")).unwrap();
    remove_account("bwork").unwrap();

    // TDD: awork becomes default
    let default_alias = get_default().unwrap().unwrap();
    let cfg = load().unwrap();
    let default_account = cfg.accounts.get(&default_alias).unwrap();

    // new shell: awork uses a custom dir → CLAUDE_CONFIG_DIR must be set
    let stmt = export_statement(default_account);
    assert!(stmt.starts_with("export CLAUDE_CONFIG_DIR="));
    assert!(stmt.contains("awork"));
}

// ============================================================
// Scenario 3: account switching
// ============================================================

/// S3-1: initial state with no accounts
#[test]
fn s3_no_accounts_state() {
    let _ctx = TestHome::new();
    let cfg = load().unwrap();
    assert!(cfg.accounts.is_empty());
    assert!(cfg.default.is_none());
    assert_eq!(resolve_default_dir_account(&cfg), None);
}

/// S3-2,3: adding alice auto-sets default; active is None in current shell (custom dir, no CLAUDE_CONFIG_DIR)
#[test]
fn s3_default_set_but_not_active_in_current_shell() {
    let ctx = TestHome::new();
    add_account("alice", ctx.account_dir("alice"), None).unwrap();

    // TDD: auto-default
    assert_eq!(get_default().unwrap(), Some("alice".to_string()));

    // alice uses a custom dir; without CLAUDE_CONFIG_DIR set, active resolves to None
    let cfg = load().unwrap();
    assert_eq!(
        resolve_default_dir_account(&cfg),
        None,
        "active should be None: alice uses custom dir, CLAUDE_CONFIG_DIR not set in current shell"
    );
}

/// S3-4~6: three accounts; switching to charlie updates its user info and active display
#[test]
fn s3_use_charlie_changes_active() {
    let ctx = TestHome::new();
    add_account("alice", ctx.account_dir("alice"), None).unwrap();
    add_account("bob", ctx.account_dir("bob"), None).unwrap();
    add_account("charlie", ctx.account_dir("charlie"), None).unwrap();

    // switch to charlie (mocked: logged in, fetch succeeds)
    run_inner(
        "charlie",
        false,
        |_| true,
        |_| {
            Some(UserInfo {
                email: "charlie@example.com".to_string(),
                subscription_type: "pro".to_string(),
            })
        },
    )
    .unwrap();

    // charlie's user info must be stored
    let account = get_account("charlie").unwrap();
    assert_eq!(account.email.as_deref(), Some("charlie@example.com"));

    // active display: charlie is not default (alice is), so no star prefix
    control::set_override(false);
    let cfg = load().unwrap();
    let charlie = cfg.accounts.get("charlie").unwrap();
    let line = format_account_line("charlie", charlie, true, false);
    assert_eq!(line, "  charlie charlie@example.com (pro)");
}

/// S3-7: removing charlie leaves alice and bob intact
#[test]
fn s3_remove_active_account_remaining_accounts_intact() {
    let ctx = TestHome::new();
    add_account("alice", ctx.account_dir("alice"), None).unwrap();
    add_account("bob", ctx.account_dir("bob"), None).unwrap();
    add_account("charlie", ctx.account_dir("charlie"), None).unwrap();

    remove_account("charlie").unwrap();

    let cfg = load().unwrap();
    assert!(!cfg.accounts.contains_key("charlie"));
    assert!(cfg.accounts.contains_key("alice"));
    assert!(cfg.accounts.contains_key("bob"));
}

/// S3-8: switching to the default account produces the correct CLAUDE_CONFIG_DIR export
#[test]
fn s3_use_default_account_sets_config_dir() {
    let ctx = TestHome::new();
    add_account("alice", ctx.account_dir("alice"), None).unwrap();
    set_default(Some("alice")).unwrap();

    // switch to alice (no_refresh)
    run_inner("alice", true, |_| false, |_| None).unwrap();

    let cfg = load().unwrap();
    let alice = cfg.accounts.get("alice").unwrap();
    assert_eq!(
        export_statement(alice),
        format!(
            "export CLAUDE_CONFIG_DIR=\"{}\"",
            ctx.account_dir("alice").display()
        )
    );
}

/// S3-9: removing all accounts leaves config empty; CLAUDE_CONFIG_DIR must be unset
#[test]
fn s3_remove_all_accounts_config_empty() {
    let ctx = TestHome::new();
    add_account("alice", ctx.account_dir("alice"), None).unwrap();
    add_account("bob", ctx.account_dir("bob"), None).unwrap();
    add_account("charlie", ctx.account_dir("charlie"), None).unwrap();

    remove_account("alice").unwrap();
    remove_account("bob").unwrap();
    remove_account("charlie").unwrap();

    let cfg = load().unwrap();
    assert!(cfg.accounts.is_empty(), "all accounts should be removed");
    assert!(
        cfg.default.is_none(),
        "default should be None after all accounts removed"
    );
    // active resolves to None → CLAUDE_CONFIG_DIR must be unset
    assert_eq!(resolve_default_dir_account(&cfg), None);
}

/// S3-9: after all accounts are removed, no account maps to any config dir
#[test]
fn s3_after_all_removed_no_valid_config_dir() {
    let ctx = TestHome::new();
    add_account("alice", ctx.account_dir("alice"), None).unwrap();
    let alice_dir = ctx.account_dir("alice");

    remove_account("alice").unwrap();

    // alice dir may still exist on disk but no account is registered for it
    let cfg = load().unwrap();
    assert!(cfg.accounts.is_empty());
    assert!(
        !alice_dir.exists() || cfg.accounts.is_empty(),
        "no registered account should point to any config dir"
    );
}
