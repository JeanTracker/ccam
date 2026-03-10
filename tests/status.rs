use ccam::commands::status::resolve_default_dir_account;
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

    /// Returns the path that is_default_config_dir considers "~/.claude"
    fn claude_dir(&self) -> std::path::PathBuf {
        self.tmp.path().join(".claude")
    }

    fn accounts_dir(&self) -> std::path::PathBuf {
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

// --- resolve_default_dir_account ---

#[test]
fn resolve_returns_default_when_it_uses_claude_dir() {
    let ctx = TestHome::new();
    add_account("base", ctx.claude_dir(), None).unwrap();
    add_account("work", ctx.accounts_dir().join("work"), None).unwrap();
    set_default(Some("base")).unwrap();

    let cfg = load().unwrap();
    assert_eq!(resolve_default_dir_account(&cfg), Some("base"));
}

#[test]
fn resolve_skips_default_when_it_does_not_use_claude_dir() {
    let ctx = TestHome::new();
    add_account("alpha", ctx.claude_dir(), None).unwrap();
    add_account("work", ctx.accounts_dir().join("work"), None).unwrap();
    set_default(Some("work")).unwrap(); // default doesn't use ~/.claude

    let cfg = load().unwrap();
    // falls back to alphabetical: "alpha" uses ~/.claude
    assert_eq!(resolve_default_dir_account(&cfg), Some("alpha"));
}

#[test]
fn resolve_returns_first_alphabetically_when_no_default() {
    let ctx = TestHome::new();
    // Both "zebra" and "alpha" share ~/.claude (unusual but tests priority logic)
    add_account("zebra", ctx.claude_dir(), None).unwrap();
    add_account("alpha", ctx.claude_dir(), None).unwrap();
    set_default(None).unwrap(); // explicitly clear default to test alphabetical fallback

    let cfg = load().unwrap();
    assert_eq!(resolve_default_dir_account(&cfg), Some("alpha"));
}

#[test]
fn resolve_returns_none_when_no_account_uses_claude_dir() {
    let ctx = TestHome::new();
    add_account("work", ctx.accounts_dir().join("work"), None).unwrap();
    add_account("personal", ctx.accounts_dir().join("personal"), None).unwrap();

    let cfg = load().unwrap();
    assert_eq!(resolve_default_dir_account(&cfg), None);
}

#[test]
fn resolve_returns_none_for_empty_config() {
    let _ctx = TestHome::new();
    let cfg = load().unwrap();
    assert_eq!(resolve_default_dir_account(&cfg), None);
}

// --- list sort order ---

#[test]
fn list_accounts_sorted_alphabetically() {
    let ctx = TestHome::new();
    add_account("zebra", ctx.accounts_dir().join("zebra"), None).unwrap();
    add_account("alpha", ctx.accounts_dir().join("alpha"), None).unwrap();
    add_account("middle", ctx.accounts_dir().join("middle"), None).unwrap();

    let cfg = load().unwrap();
    let mut keys: Vec<&str> = cfg.accounts.keys().map(|s| s.as_str()).collect();
    keys.sort();
    assert_eq!(keys, vec!["alpha", "middle", "zebra"]);
}
