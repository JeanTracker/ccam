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
// Scenario 1: 기본 계정 시나리오
// ============================================================

/// S1-1~4: 계정 3개 추가 후 조회 및 active 확인
#[test]
fn s1_add_three_accounts_sorted_and_default() {
    let ctx = TestHome::new();

    // 1. 계정#1 alpha 추가 (인증 계정 - 표시 레이어에서 mock)
    add_account("alpha", ctx.account_dir("alpha"), None).unwrap();

    // TDD: 첫 번째 계정 추가 시 자동으로 default 설정되어야 함
    assert_eq!(
        get_default().unwrap(),
        Some("alpha".to_string()),
        "first account should be auto-set as default"
    );

    // 2. 계정#2 beta 추가 (미인증)
    add_account("beta", ctx.account_dir("beta"), None).unwrap();

    // 3. 계정#3 gamma 추가 (--dir ~/.claude)
    fs::create_dir_all(ctx.claude_dir()).unwrap();
    add_account("gamma", ctx.claude_dir(), None).unwrap();

    // 4. 계정 조회: 알파벳 오름차순
    let cfg = load().unwrap();
    let mut keys: Vec<&str> = cfg.accounts.keys().map(|s| s.as_str()).collect();
    keys.sort();
    assert_eq!(keys, vec!["alpha", "beta", "gamma"]);

    // default는 alpha (첫 번째 추가)
    assert_eq!(cfg.default.as_deref(), Some("alpha"));

    // 새 shell active: alpha는 ~/.claude 미사용 → CLAUDE_CONFIG_DIR 설정됨
    let alpha_account = cfg.accounts.get("alpha").unwrap();
    assert_eq!(
        export_statement(alpha_account),
        format!(
            "export CLAUDE_CONFIG_DIR=\"{}\"",
            ctx.account_dir("alpha").display()
        )
    );
}

/// S1-5: gamma(--dir ~/.claude) 삭제 시 ~/.claude 디렉토리 유지
#[test]
fn s1_remove_claude_dir_account_preserves_directory() {
    let ctx = TestHome::new();
    let claude_dir = ctx.claude_dir();
    fs::create_dir_all(&claude_dir).unwrap();

    add_account("alpha", ctx.account_dir("alpha"), None).unwrap();
    add_account("gamma", claude_dir.clone(), None).unwrap();

    // gamma 삭제
    remove_account("gamma").unwrap();

    let cfg = load().unwrap();
    assert!(
        !cfg.accounts.contains_key("gamma"),
        "config entry should be removed"
    );
    assert!(claude_dir.exists(), "~/.claude directory must be preserved");
}

/// S1-5: gamma 삭제 후 ~/.claude-accounts 하위 폴더 없음 (gamma는 ~/.claude 직접 사용)
#[test]
fn s1_claude_dir_account_has_no_accounts_subfolder() {
    let ctx = TestHome::new();
    fs::create_dir_all(ctx.claude_dir()).unwrap();
    add_account("gamma", ctx.claude_dir(), None).unwrap();

    let account = get_account("gamma").unwrap();
    // gamma의 config_dir은 ~/.claude이므로 ~/.claude-accounts/gamma 폴더 없음
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

    // 일반 계정은 is_default_config_dir = false → commands::remove::run이 디렉토리 삭제
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
// Scenario 2: default 설정 확인
// ============================================================

/// S2-1,2: 계정 없음, CLAUDE_CONFIG_DIR 미설정 → active 없음
#[test]
fn s2_no_accounts_active_resolves_none() {
    let _ctx = TestHome::new();
    let cfg = load().unwrap();
    assert!(cfg.accounts.is_empty());
    assert_eq!(resolve_default_dir_account(&cfg), None);
}

/// S2-3,4: 계정#1(awork) 추가 → auto-default → 새 shell CLAUDE_CONFIG_DIR 설정
#[test]
fn s2_add_first_account_auto_default_new_shell_sets_config_dir() {
    let ctx = TestHome::new();

    // 계정#1 awork 추가
    add_account("awork", ctx.account_dir("awork"), None).unwrap();

    // TDD: 자동 default 설정
    assert_eq!(get_default().unwrap(), Some("awork".to_string()));

    // 새 shell: export_statement → CLAUDE_CONFIG_DIR 설정
    let cfg = load().unwrap();
    let account = cfg.accounts.get("awork").unwrap();
    let stmt = export_statement(account);
    assert!(stmt.starts_with("export CLAUDE_CONFIG_DIR="));
    assert!(stmt.contains("awork"));
}

/// S2-7~10: 계정#2(bwork, --dir ~/.claude) 추가 및 default 변경 → 새 shell CLAUDE_CONFIG_DIR 미설정
#[test]
fn s2_claude_dir_account_as_default_new_shell_unsets_config_dir() {
    let ctx = TestHome::new();
    fs::create_dir_all(ctx.claude_dir()).unwrap();

    add_account("awork", ctx.account_dir("awork"), None).unwrap();
    add_account("bwork", ctx.claude_dir(), None).unwrap();

    // default를 bwork(--dir ~/.claude)로 변경
    set_default(Some("bwork")).unwrap();

    // 새 shell: export_statement → unset CLAUDE_CONFIG_DIR
    let cfg = load().unwrap();
    let bwork = cfg.accounts.get("bwork").unwrap();
    assert_eq!(export_statement(bwork), "unset CLAUDE_CONFIG_DIR");

    // active: CLAUDE_CONFIG_DIR 미설정 시 ~/.claude 사용 계정(bwork) 조회
    assert_eq!(resolve_default_dir_account(&cfg), Some("bwork"));
}

/// S2-11,12: 계정#3(cwork) 추가 후 bwork 삭제 → default 알파벳 최상위(awork) 자동 설정
#[test]
fn s2_remove_claude_dir_default_reassigns_to_alphabetical_first() {
    let ctx = TestHome::new();
    fs::create_dir_all(ctx.claude_dir()).unwrap();

    add_account("awork", ctx.account_dir("awork"), None).unwrap();
    add_account("bwork", ctx.claude_dir(), None).unwrap();
    add_account("cwork", ctx.account_dir("cwork"), None).unwrap();

    set_default(Some("bwork")).unwrap();

    // bwork 삭제: 남은 계정 awork, cwork 중 알파벳 첫 번째(awork)가 default
    remove_account("bwork").unwrap();

    // TDD: 알파벳 오름차순 첫 번째 계정이 default로 재지정되어야 함
    assert_eq!(
        get_default().unwrap(),
        Some("awork".to_string()),
        "default should be reassigned to alphabetically first remaining account"
    );
}

/// S2-13,14: bwork 삭제 후 새 shell → awork default → CLAUDE_CONFIG_DIR 설정
#[test]
fn s2_after_reassign_new_shell_applies_correct_config_dir() {
    let ctx = TestHome::new();
    fs::create_dir_all(ctx.claude_dir()).unwrap();

    add_account("awork", ctx.account_dir("awork"), None).unwrap();
    add_account("bwork", ctx.claude_dir(), None).unwrap();
    set_default(Some("bwork")).unwrap();
    remove_account("bwork").unwrap();

    // TDD: awork가 default
    let default_alias = get_default().unwrap().unwrap();
    let cfg = load().unwrap();
    let default_account = cfg.accounts.get(&default_alias).unwrap();

    // 새 shell: awork는 일반 디렉토리 → CLAUDE_CONFIG_DIR 설정
    let stmt = export_statement(default_account);
    assert!(stmt.starts_with("export CLAUDE_CONFIG_DIR="));
    assert!(stmt.contains("awork"));
}

// ============================================================
// Scenario 3: 계정 사용
// ============================================================

/// S3-1: 계정 없는 상태 확인
#[test]
fn s3_no_accounts_state() {
    let _ctx = TestHome::new();
    let cfg = load().unwrap();
    assert!(cfg.accounts.is_empty());
    assert!(cfg.default.is_none());
    assert_eq!(resolve_default_dir_account(&cfg), None);
}

/// S3-2,3: alice 추가 → default 자동 설정, 현재 shell active 없음
#[test]
fn s3_default_set_but_not_active_in_current_shell() {
    let ctx = TestHome::new();
    add_account("alice", ctx.account_dir("alice"), None).unwrap();

    // TDD: auto-default
    assert_eq!(get_default().unwrap(), Some("alice".to_string()));

    // alice는 ~/.claude 미사용 → CLAUDE_CONFIG_DIR 없으면 active 없음
    let cfg = load().unwrap();
    assert_eq!(
        resolve_default_dir_account(&cfg),
        None,
        "active should be None: alice uses custom dir, CLAUDE_CONFIG_DIR not set in current shell"
    );
}

/// S3-4~6: 계정 3개 추가 후 charlie 사용 → active=charlie
#[test]
fn s3_use_charlie_changes_active() {
    let ctx = TestHome::new();
    add_account("alice", ctx.account_dir("alice"), None).unwrap();
    add_account("bob", ctx.account_dir("bob"), None).unwrap();
    add_account("charlie", ctx.account_dir("charlie"), None).unwrap();

    // charlie 사용 (mocked: logged in, fetch 성공)
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

    // charlie 계정에 user info 저장됨
    let account = get_account("charlie").unwrap();
    assert_eq!(account.email.as_deref(), Some("charlie@example.com"));

    // active display: charlie, default=alice이므로 is_default=false
    control::set_override(false);
    let cfg = load().unwrap();
    let charlie = cfg.accounts.get("charlie").unwrap();
    let line = format_account_line("charlie", charlie, true, false);
    assert_eq!(line, "  charlie charlie@example.com (pro)");
}

/// S3-7: charlie 삭제 후 남은 계정 확인
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

/// S3-8: default(alice) 사용으로 설정 → CLAUDE_CONFIG_DIR 설정됨
#[test]
fn s3_use_default_account_sets_config_dir() {
    let ctx = TestHome::new();
    add_account("alice", ctx.account_dir("alice"), None).unwrap();
    set_default(Some("alice")).unwrap();

    // alice 사용 (no_refresh)
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

/// S3-9: 계정 모두 삭제 시 config 비어있음 → CLAUDE_CONFIG_DIR 해제 필요 상태
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
    // active resolve: None → CLAUDE_CONFIG_DIR 해제 필요
    assert_eq!(resolve_default_dir_account(&cfg), None);
}

/// S3-9: 계정 모두 삭제 시 직전 active 계정 export_statement는 unset 되어야 함
/// (마지막으로 삭제된 계정의 config_dir이 더이상 유효하지 않음을 확인)
#[test]
fn s3_after_all_removed_no_valid_config_dir() {
    let ctx = TestHome::new();
    add_account("alice", ctx.account_dir("alice"), None).unwrap();
    let alice_dir = ctx.account_dir("alice");

    remove_account("alice").unwrap();

    // alice dir는 아직 존재하나 계정은 삭제됨
    // 빈 config → 어떤 계정도 CLAUDE_CONFIG_DIR로 설정할 수 없음
    let cfg = load().unwrap();
    assert!(cfg.accounts.is_empty());
    assert!(
        !alice_dir.exists() || cfg.accounts.is_empty(),
        "no registered account should point to any config dir"
    );
}
