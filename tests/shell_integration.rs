/// Shell integration tests for the ccam wrapper functions.
///
/// Each test spawns an isolated bash subprocess with HOME pointed at a
/// temporary directory, so the real ~/  is never touched and no global
/// state is mutated in the parent process. No HOME_LOCK mutex is needed.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_ccam");
const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn verbose() -> bool {
    std::env::var("CCAM_TEST_VERBOSE").is_ok()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct TestEnv {
    tmp: tempfile::TempDir,
}

impl TestEnv {
    fn new() -> Self {
        let env = Self {
            tmp: tempfile::TempDir::new().unwrap(),
        };
        // Place a no-op claude stub so ccam's logout/auth calls don't hang.
        let stub_dir = env.stub_bin_dir();
        fs::create_dir_all(&stub_dir).unwrap();
        let stub = stub_dir.join("claude");
        fs::write(&stub, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
        }
        env
    }

    fn home(&self) -> &Path {
        self.tmp.path()
    }

    fn stub_bin_dir(&self) -> PathBuf {
        self.tmp.path().join("bin")
    }

    fn accounts_dir(&self) -> PathBuf {
        self.tmp.path().join(".claude-accounts")
    }

    fn account_dir(&self, name: &str) -> PathBuf {
        self.accounts_dir().join(name)
    }

    fn claude_dir(&self) -> PathBuf {
        self.tmp.path().join(".claude")
    }

    /// Write accounts.toml directly — no need to touch the parent HOME.
    fn write_accounts_toml(&self, content: &str) {
        let dir = self.accounts_dir();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("accounts.toml"), content).unwrap();
    }

    /// Run a bash script in a subprocess with HOME isolated to the temp dir.
    /// `active_dir` pre-sets CLAUDE_CONFIG_DIR before the wrapper is sourced.
    /// Returns (stdout, stderr, exit_success).
    fn run_bash(&self, active_dir: Option<&str>, script: &str) -> (String, String, bool) {
        let wrapper = PathBuf::from(MANIFEST_DIR).join("shell/ccam.bash");
        let binary_dir = PathBuf::from(BINARY).parent().unwrap().to_path_buf();

        // PATH order: stub bin (fake claude) → ccam binary dir → system
        let set_active = match active_dir {
            Some(dir) => format!("export CLAUDE_CONFIG_DIR=\"{}\"\n", dir),
            None => "unset CLAUDE_CONFIG_DIR\n".to_string(),
        };
        let full_script = format!(
            "export PATH=\"{}:{}:$PATH\"\nsource \"{}\"\n{}{}",
            self.stub_bin_dir().display(),
            binary_dir.display(),
            wrapper.display(),
            set_active,
            script,
        );

        let output = Command::new("bash")
            .args(["-c", &full_script])
            .env("HOME", self.home())
            .env_remove("CLAUDE_CONFIG_DIR")
            .output()
            .expect("bash not found");

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let ok = output.status.success();

        if verbose() {
            eprintln!("\n[script]\n{}", full_script);
            eprintln!("[stdout] {:?}", stdout);
            eprintln!("[stderr] {}", stderr);
            eprintln!("[exit]   {}", output.status);
        }

        (stdout, stderr, ok)
    }

    /// Like run_bash but does not touch CLAUDE_CONFIG_DIR at all.
    /// Use this to test session init behavior (the init block inside ccam.bash).
    fn run_bash_init(&self, script: &str) -> (String, String, bool) {
        let wrapper = PathBuf::from(MANIFEST_DIR).join("shell/ccam.bash");
        let binary_dir = PathBuf::from(BINARY).parent().unwrap().to_path_buf();

        let full_script = format!(
            "export PATH=\"{}:{}:$PATH\"\nsource \"{}\"\n{}",
            self.stub_bin_dir().display(),
            binary_dir.display(),
            wrapper.display(),
            script,
        );

        let output = Command::new("bash")
            .args(["-c", &full_script])
            .env("HOME", self.home())
            .env_remove("CLAUDE_CONFIG_DIR")
            .output()
            .expect("bash not found");

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let ok = output.status.success();

        if verbose() {
            eprintln!("\n[script]\n{}", full_script);
            eprintln!("[stdout] {:?}", stdout);
            eprintln!("[stderr] {}", stderr);
            eprintln!("[exit]   {}", output.status);
        }

        (stdout, stderr, ok)
    }
}

/// Minimal accounts.toml with two custom-dir accounts.
fn toml_two_accounts(home: &Path) -> String {
    let work = home.join(".claude-accounts/work").display().to_string();
    let personal = home.join(".claude-accounts/personal").display().to_string();
    format!(
        "default = \"work\"\n\
         [accounts.work]\n\
         config_dir = \"{work}\"\n\
         added_at = \"2026-01-01T00:00:00+00:00\"\n\
         [accounts.personal]\n\
         config_dir = \"{personal}\"\n\
         added_at = \"2026-01-01T00:00:00+00:00\"\n"
    )
}

/// Minimal accounts.toml with one custom-dir account.
fn toml_one_account(home: &Path, name: &str) -> String {
    let dir = home
        .join(".claude-accounts")
        .join(name)
        .display()
        .to_string();
    format!(
        "default = \"{name}\"\n\
         [accounts.{name}]\n\
         config_dir = \"{dir}\"\n\
         added_at = \"2026-01-01T00:00:00+00:00\"\n"
    )
}

/// Minimal accounts.toml with one ~/.claude account.
fn toml_claude_dir_account(home: &Path, name: &str) -> String {
    let dir = home.join(".claude").display().to_string();
    format!(
        "default = \"{name}\"\n\
         [accounts.{name}]\n\
         config_dir = \"{dir}\"\n\
         added_at = \"2026-01-01T00:00:00+00:00\"\n"
    )
}

/// Minimal accounts.toml with one custom-dir account and one ~/.claude account.
fn toml_custom_and_claude_dir(home: &Path) -> String {
    let work = home.join(".claude-accounts/work").display().to_string();
    let base = home.join(".claude").display().to_string();
    format!(
        "default = \"work\"\n\
         [accounts.work]\n\
         config_dir = \"{work}\"\n\
         added_at = \"2026-01-01T00:00:00+00:00\"\n\
         [accounts.base]\n\
         config_dir = \"{base}\"\n\
         added_at = \"2026-01-01T00:00:00+00:00\"\n"
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Removing the active account switches CLAUDE_CONFIG_DIR to the default.
#[test]
fn remove_active_account_switches_to_default() {
    let env = TestEnv::new();
    fs::create_dir_all(env.account_dir("work")).unwrap();
    fs::create_dir_all(env.account_dir("personal")).unwrap();
    env.write_accounts_toml(&toml_two_accounts(env.home()));

    let active = env.account_dir("work").display().to_string();
    let personal = env.account_dir("personal").display().to_string();

    let (stdout, _stderr, ok) = env.run_bash(
        Some(&active),
        "ccam remove work --yes 2>/dev/null\necho \"RESULT:$CLAUDE_CONFIG_DIR\"",
    );

    assert!(ok);
    assert!(
        stdout.contains(&format!("RESULT:{}", personal)),
        "expected CLAUDE_CONFIG_DIR to switch to personal, got: {}",
        stdout
    );
}

/// Removing the only active account unsets CLAUDE_CONFIG_DIR.
#[test]
fn remove_last_active_account_unsets_config_dir() {
    let env = TestEnv::new();
    fs::create_dir_all(env.account_dir("work")).unwrap();
    env.write_accounts_toml(&toml_one_account(env.home(), "work"));

    let active = env.account_dir("work").display().to_string();

    let (stdout, _stderr, ok) = env.run_bash(
        Some(&active),
        "ccam remove work --yes 2>/dev/null\necho \"RESULT:$CLAUDE_CONFIG_DIR\"",
    );

    assert!(ok);
    assert_eq!(
        stdout.trim(),
        "RESULT:",
        "CLAUDE_CONFIG_DIR should be unset, got: {}",
        stdout
    );
}

/// Removing an inactive account leaves CLAUDE_CONFIG_DIR unchanged.
#[test]
fn remove_inactive_account_leaves_config_dir_unchanged() {
    let env = TestEnv::new();
    fs::create_dir_all(env.account_dir("work")).unwrap();
    fs::create_dir_all(env.account_dir("personal")).unwrap();
    env.write_accounts_toml(&toml_two_accounts(env.home()));

    // personal is active, removing work (inactive)
    let active = env.account_dir("personal").display().to_string();

    let (stdout, _stderr, ok) = env.run_bash(
        Some(&active),
        "ccam remove work --yes 2>/dev/null\necho \"RESULT:$CLAUDE_CONFIG_DIR\"",
    );

    assert!(ok);
    assert!(
        stdout.contains(&format!("RESULT:{}", active)),
        "CLAUDE_CONFIG_DIR should remain unchanged, got: {}",
        stdout
    );
}

/// Removing the active ~/.claude account unsets CLAUDE_CONFIG_DIR.
#[test]
fn remove_active_claude_dir_account_unsets_config_dir() {
    let env = TestEnv::new();
    fs::create_dir_all(env.claude_dir()).unwrap();
    let claude_dir = env.claude_dir().display().to_string();
    let toml = format!(
        "default = \"base\"\n\
         [accounts.base]\n\
         config_dir = \"{claude_dir}\"\n\
         added_at = \"2026-01-01T00:00:00+00:00\"\n"
    );
    env.write_accounts_toml(&toml);

    // CLAUDE_CONFIG_DIR unset → ~/.claude is active
    let (stdout, _stderr, ok) = env.run_bash(
        None,
        "ccam remove base --yes 2>/dev/null\necho \"RESULT:$CLAUDE_CONFIG_DIR\"",
    );

    assert!(ok);
    assert_eq!(
        stdout.trim(),
        "RESULT:",
        "CLAUDE_CONFIG_DIR should be unset, got: {}",
        stdout
    );
}

/// --yes skips the confirmation prompt; without it the remove would block.
#[test]
fn remove_yes_flag_skips_prompt() {
    let env = TestEnv::new();
    fs::create_dir_all(env.account_dir("work")).unwrap();
    env.write_accounts_toml(&toml_one_account(env.home(), "work"));

    // Pass empty stdin so a real prompt would fail (read returns empty → 'N')
    // --yes must override this
    let (_, _, ok) = env.run_bash(None, "echo '' | ccam remove work --yes 2>/dev/null");

    assert!(ok, "remove --yes should succeed without interactive input");
}

// ---------------------------------------------------------------------------
// rm alias: shell wrapper must handle `ccam rm` identically to `ccam remove`
// ---------------------------------------------------------------------------

/// `ccam rm` (alias) on the active account switches CLAUDE_CONFIG_DIR.
#[test]
fn rm_alias_active_account_switches_to_default() {
    let env = TestEnv::new();
    fs::create_dir_all(env.account_dir("work")).unwrap();
    fs::create_dir_all(env.account_dir("personal")).unwrap();
    env.write_accounts_toml(&toml_two_accounts(env.home()));

    let active = env.account_dir("work").display().to_string();
    let personal = env.account_dir("personal").display().to_string();

    let (stdout, _stderr, ok) = env.run_bash(
        Some(&active),
        "ccam rm work --yes 2>/dev/null\necho \"RESULT:$CLAUDE_CONFIG_DIR\"",
    );

    assert!(ok);
    assert!(
        stdout.contains(&format!("RESULT:{}", personal)),
        "expected CLAUDE_CONFIG_DIR to switch to personal via rm alias, got: {}",
        stdout
    );
}

/// `ccam rm` (alias) on the only active account unsets CLAUDE_CONFIG_DIR.
#[test]
fn rm_alias_last_active_account_unsets_config_dir() {
    let env = TestEnv::new();
    fs::create_dir_all(env.account_dir("work")).unwrap();
    env.write_accounts_toml(&toml_one_account(env.home(), "work"));

    let active = env.account_dir("work").display().to_string();

    let (stdout, _stderr, ok) = env.run_bash(
        Some(&active),
        "ccam rm work --yes 2>/dev/null\necho \"RESULT:$CLAUDE_CONFIG_DIR\"",
    );

    assert!(ok);
    assert_eq!(
        stdout.trim(),
        "RESULT:",
        "CLAUDE_CONFIG_DIR should be unset via rm alias, got: {}",
        stdout
    );
}

/// `ccam rm` (alias) on an inactive account leaves CLAUDE_CONFIG_DIR unchanged.
#[test]
fn rm_alias_inactive_account_leaves_config_dir_unchanged() {
    let env = TestEnv::new();
    fs::create_dir_all(env.account_dir("work")).unwrap();
    fs::create_dir_all(env.account_dir("personal")).unwrap();
    env.write_accounts_toml(&toml_two_accounts(env.home()));

    let active = env.account_dir("personal").display().to_string();

    let (stdout, _stderr, ok) = env.run_bash(
        Some(&active),
        "ccam rm work --yes 2>/dev/null\necho \"RESULT:$CLAUDE_CONFIG_DIR\"",
    );

    assert!(ok);
    assert!(
        stdout.contains(&format!("RESULT:{}", active)),
        "CLAUDE_CONFIG_DIR should remain unchanged via rm alias, got: {}",
        stdout
    );
}

// ---------------------------------------------------------------------------
// use: shell wrapper evals `ccam __env <alias>` to set CLAUDE_CONFIG_DIR
// ---------------------------------------------------------------------------

/// `ccam use work` sets CLAUDE_CONFIG_DIR to work's config dir.
#[test]
fn use_sets_config_dir_for_custom_account() {
    let env = TestEnv::new();
    fs::create_dir_all(env.account_dir("work")).unwrap();
    env.write_accounts_toml(&toml_one_account(env.home(), "work"));

    let expected = env.account_dir("work").display().to_string();

    let (stdout, _stderr, ok) = env.run_bash(
        None,
        "ccam use work 2>/dev/null\necho \"RESULT:$CLAUDE_CONFIG_DIR\"",
    );

    assert!(ok);
    assert!(
        stdout.contains(&format!("RESULT:{}", expected)),
        "expected CLAUDE_CONFIG_DIR={}, got: {}",
        expected,
        stdout
    );
}

/// `ccam use base` where base uses ~/.claude unsets CLAUDE_CONFIG_DIR.
#[test]
fn use_unsets_config_dir_for_claude_dir_account() {
    let env = TestEnv::new();
    fs::create_dir_all(env.claude_dir()).unwrap();
    fs::create_dir_all(env.account_dir("work")).unwrap();
    env.write_accounts_toml(&toml_custom_and_claude_dir(env.home()));

    // Start with work active, switch to base (uses ~/.claude)
    let active = env.account_dir("work").display().to_string();

    let (stdout, _stderr, ok) = env.run_bash(
        Some(&active),
        "ccam use base 2>/dev/null\necho \"RESULT:$CLAUDE_CONFIG_DIR\"",
    );

    assert!(ok);
    assert_eq!(
        stdout.trim(),
        "RESULT:",
        "CLAUDE_CONFIG_DIR should be unset after switching to ~/.claude account, got: {}",
        stdout
    );
}

/// `ccam use personal` switches CLAUDE_CONFIG_DIR from work to personal.
#[test]
fn use_switches_between_accounts() {
    let env = TestEnv::new();
    fs::create_dir_all(env.account_dir("work")).unwrap();
    fs::create_dir_all(env.account_dir("personal")).unwrap();
    env.write_accounts_toml(&toml_two_accounts(env.home()));

    let active = env.account_dir("work").display().to_string();
    let expected = env.account_dir("personal").display().to_string();

    let (stdout, _stderr, ok) = env.run_bash(
        Some(&active),
        "ccam use personal 2>/dev/null\necho \"RESULT:$CLAUDE_CONFIG_DIR\"",
    );

    assert!(ok);
    assert!(
        stdout.contains(&format!("RESULT:{}", expected)),
        "expected CLAUDE_CONFIG_DIR to switch to personal, got: {}",
        stdout
    );
}

// ---------------------------------------------------------------------------
// session init: ccam.bash init block applies default account on shell startup
// ---------------------------------------------------------------------------

/// Sourcing ccam.bash sets CLAUDE_CONFIG_DIR to the default account's dir.
#[test]
fn session_init_sets_config_dir_from_default() {
    let env = TestEnv::new();
    fs::create_dir_all(env.account_dir("work")).unwrap();
    env.write_accounts_toml(&toml_one_account(env.home(), "work"));

    let expected = env.account_dir("work").display().to_string();

    let (stdout, _stderr, ok) = env.run_bash_init("echo \"RESULT:$CLAUDE_CONFIG_DIR\"");

    assert!(ok);
    assert!(
        stdout.contains(&format!("RESULT:{}", expected)),
        "expected CLAUDE_CONFIG_DIR set by init block, got: {}",
        stdout
    );
}

/// Sourcing ccam.bash with a ~/.claude default leaves CLAUDE_CONFIG_DIR unset.
#[test]
fn session_init_unsets_config_dir_for_claude_dir_default() {
    let env = TestEnv::new();
    fs::create_dir_all(env.claude_dir()).unwrap();
    env.write_accounts_toml(&toml_claude_dir_account(env.home(), "base"));

    let (stdout, _stderr, ok) = env.run_bash_init("echo \"RESULT:$CLAUDE_CONFIG_DIR\"");

    assert!(ok);
    assert_eq!(
        stdout.trim(),
        "RESULT:",
        "CLAUDE_CONFIG_DIR should be unset for ~/.claude default, got: {}",
        stdout
    );
}

/// Sourcing ccam.bash with no default does not set CLAUDE_CONFIG_DIR.
#[test]
fn session_init_no_default_leaves_config_dir_unset() {
    let env = TestEnv::new();
    // No accounts.toml → no default
    let (stdout, _stderr, ok) = env.run_bash_init("echo \"RESULT:$CLAUDE_CONFIG_DIR\"");

    assert!(ok);
    assert_eq!(
        stdout.trim(),
        "RESULT:",
        "CLAUDE_CONFIG_DIR should remain unset with no default, got: {}",
        stdout
    );
}
