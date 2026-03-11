/// Tests for active/default account resolution when multiple accounts share a config_dir.
///
/// Requires: `ccam::commands::status::resolve_active_account` to be extracted and made public.
///
/// Expected signature:
///   pub fn resolve_active_account<'a>(cfg: &'a AccountsConfig, active_dir: &str) -> Option<&'a str>
///
/// Priority rule (mirrors resolve_default_dir_account):
///   1. default account, if its config_dir matches active_dir
///   2. alphabetically first account whose config_dir matches active_dir
///   3. None if no account matches
use ccam::commands::status::resolve_active_account;
use ccam::config::{add_account, load, set_default};
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
    fn accounts_dir(&self) -> std::path::PathBuf {
        self.tmp.path().join(".claude-accounts")
    }
    fn claude_dir(&self) -> std::path::PathBuf {
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

// ============================================================
// Single account: basic resolution
// ============================================================

/// When CLAUDE_CONFIG_DIR matches exactly one registered account, that account is returned.
#[test]
fn active_returns_sole_matching_account() {
    let ctx = TestHome::new();
    let dir = ctx.accounts_dir().join("work");
    let other = ctx.accounts_dir().join("personal");

    add_account("work", dir.clone(), None).unwrap();
    add_account("personal", other.clone(), None).unwrap();
    set_default(Some("personal")).unwrap(); // default uses a different dir

    let cfg = load().unwrap();
    let active = resolve_active_account(&cfg, &dir.to_string_lossy());
    assert_eq!(
        active,
        Some("work"),
        "sole matching account must be returned"
    );
}

/// When CLAUDE_CONFIG_DIR does not match any registered account, None is returned.
#[test]
fn active_returns_none_when_no_account_matches_dir() {
    let ctx = TestHome::new();
    add_account("work", ctx.accounts_dir().join("work"), None).unwrap();

    let cfg = load().unwrap();
    let active = resolve_active_account(&cfg, "/completely/unregistered/path");
    assert_eq!(active, None);
}

/// When no accounts are registered, None is returned.
#[test]
fn active_returns_none_for_empty_config() {
    let _ctx = TestHome::new();
    let cfg = load().unwrap();
    let active = resolve_active_account(&cfg, "/any/path");
    assert_eq!(active, None);
}

// ============================================================
// Shared path: default-first priority rule
// ============================================================

/// When multiple accounts share a config_dir and the default is one of them,
/// the default account is returned as active.
///
/// Bug in original code: `find_map` over HashMap is non-deterministic; any sharer
/// could be returned depending on iteration order.
#[test]
fn active_shows_default_when_default_shares_the_dir() {
    let ctx = TestHome::new();
    let shared = ctx.accounts_dir().join("shared");

    add_account("alpha", shared.clone(), None).unwrap();
    add_account("beta", shared.clone(), None).unwrap();
    set_default(Some("beta")).unwrap();

    let cfg = load().unwrap();
    let active = resolve_active_account(&cfg, &shared.to_string_lossy());
    assert_eq!(
        active,
        Some("beta"),
        "default account must be shown as active when it shares CLAUDE_CONFIG_DIR"
    );
}

/// When multiple accounts share a config_dir but the default uses a different dir,
/// the alphabetically first account among the sharers is returned.
#[test]
fn active_shows_alphabetical_first_when_default_uses_different_dir() {
    let ctx = TestHome::new();
    let shared = ctx.accounts_dir().join("shared");
    let other = ctx.accounts_dir().join("other");

    add_account("charlie", shared.clone(), None).unwrap();
    add_account("alpha", shared.clone(), None).unwrap(); // alphabetically first
    add_account("default_acct", other.clone(), None).unwrap();
    set_default(Some("default_acct")).unwrap(); // default is on a different dir

    let cfg = load().unwrap();
    let active = resolve_active_account(&cfg, &shared.to_string_lossy());
    assert_eq!(
        active,
        Some("alpha"),
        "alphabetically first sharer must be active when default uses a different dir"
    );
}

/// When no default is set, the alphabetically first account sharing the dir is returned.
#[test]
fn active_shows_alphabetical_first_when_no_default_set() {
    let ctx = TestHome::new();
    let shared = ctx.accounts_dir().join("shared");

    add_account("zebra", shared.clone(), None).unwrap();
    add_account("alpha", shared.clone(), None).unwrap();
    set_default(None).unwrap();

    let cfg = load().unwrap();
    let active = resolve_active_account(&cfg, &shared.to_string_lossy());
    assert_eq!(
        active,
        Some("alpha"),
        "alphabetically first must be returned when no default"
    );
}

// ============================================================
// Active reflects default changes
// ============================================================

/// Changing default among shared-path accounts updates the active display
/// without changing CLAUDE_CONFIG_DIR.
#[test]
fn active_tracks_default_change_for_shared_path() {
    let ctx = TestHome::new();
    let shared = ctx.accounts_dir().join("shared");

    add_account("alpha", shared.clone(), None).unwrap();
    add_account("beta", shared.clone(), None).unwrap();
    // alpha is auto-default (first added)

    let cfg = load().unwrap();
    let active = resolve_active_account(&cfg, &shared.to_string_lossy());
    assert_eq!(active, Some("alpha"), "alpha is default: shown as active");

    set_default(Some("beta")).unwrap();
    let cfg = load().unwrap();
    let active = resolve_active_account(&cfg, &shared.to_string_lossy());
    assert_eq!(
        active,
        Some("beta"),
        "after default changed to beta: beta shown as active"
    );
}

/// Three accounts share a dir; cycling through defaults always returns the correct active.
#[test]
fn active_reflects_default_among_three_shared_dir_accounts() {
    let ctx = TestHome::new();
    let shared = ctx.accounts_dir().join("shared");

    add_account("alice", shared.clone(), None).unwrap();
    add_account("bob", shared.clone(), None).unwrap();
    add_account("carol", shared.clone(), None).unwrap();

    for expected in &["bob", "carol", "alice"] {
        set_default(Some(expected)).unwrap();
        let cfg = load().unwrap();
        let active = resolve_active_account(&cfg, &shared.to_string_lossy());
        assert_eq!(
            active,
            Some(*expected),
            "active must match default '{}' for shared path",
            expected
        );
    }
}

// ============================================================
// Default independence from shared path membership
// ============================================================

/// The stored default is independent of shared-path membership.
/// Active resolution for each path follows its own priority rule.
#[test]
fn default_is_independent_of_shared_path_membership() {
    let ctx = TestHome::new();
    let shared = ctx.accounts_dir().join("shared");
    let other = ctx.accounts_dir().join("other");

    add_account("shared1", shared.clone(), None).unwrap();
    add_account("shared2", shared.clone(), None).unwrap();
    add_account("standalone", other.clone(), None).unwrap();
    set_default(Some("standalone")).unwrap();

    let cfg = load().unwrap();
    assert_eq!(cfg.default.as_deref(), Some("standalone"));

    // Active for shared path: alphabetically first sharer (default not among them)
    let active = resolve_active_account(&cfg, &shared.to_string_lossy());
    assert_eq!(active, Some("shared1"));

    // Active for other path: standalone (default and sole account)
    let active_other = resolve_active_account(&cfg, &other.to_string_lossy());
    assert_eq!(active_other, Some("standalone"));
}

// ============================================================
// Consistency with resolve_default_dir_account for ~/.claude
// ============================================================

/// resolve_active_account called with the ~/.claude path must agree with
/// resolve_default_dir_account, ensuring consistent behavior regardless of
/// whether CLAUDE_CONFIG_DIR is set.
#[test]
fn active_for_claude_dir_path_is_consistent_with_resolve_default_dir() {
    use ccam::commands::status::resolve_default_dir_account;

    let ctx = TestHome::new();
    std::fs::create_dir_all(ctx.claude_dir()).unwrap();

    add_account("alpha", ctx.claude_dir(), None).unwrap();
    add_account("beta", ctx.claude_dir(), None).unwrap();
    set_default(Some("beta")).unwrap();

    let cfg = load().unwrap();
    let via_default_dir = resolve_default_dir_account(&cfg);
    let via_active = resolve_active_account(&cfg, &ctx.claude_dir().to_string_lossy());

    assert_eq!(
        via_active, via_default_dir,
        "resolve_active_account and resolve_default_dir_account must agree for ~/.claude path"
    );
}
