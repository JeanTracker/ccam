use ccam::commands::status::format_account_line;
use ccam::config::Account;
use colored::control;
use std::path::PathBuf;

fn make_account(email: Option<&str>, subscription_type: Option<&str>) -> Account {
    Account {
        config_dir: PathBuf::from("/tmp/test"),
        description: None,
        added_at: "2026-01-01T00:00:00Z".to_string(),
        email: email.map(str::to_string),
        subscription_type: subscription_type.map(str::to_string),
    }
}

fn plain(alias: &str, account: &Account, logged_in: bool, is_default: bool) -> String {
    control::set_override(false);
    format_account_line(alias, account, logged_in, is_default)
}

// --- use / active ---

#[test]
fn logged_in_default_has_star_prefix() {
    let account = make_account(Some("user@example.com"), Some("pro"));
    let line = plain("work", &account, true, true);
    assert_eq!(line, "* work user@example.com (pro)");
}

#[test]
fn logged_in_non_default_has_space_prefix() {
    let account = make_account(Some("user@example.com"), Some("pro"));
    let line = plain("work", &account, true, false);
    assert_eq!(line, "  work user@example.com (pro)");
}

#[test]
fn logged_out_non_default_has_bang_prefix() {
    let account = make_account(Some("user@example.com"), Some("pro"));
    let line = plain("work", &account, false, false);
    assert_eq!(line, "! work user@example.com (pro)");
}

#[test]
fn logged_out_default_still_has_bang_prefix() {
    let account = make_account(Some("user@example.com"), Some("pro"));
    let line = plain("work", &account, false, true);
    assert_eq!(line, "! work user@example.com (pro)");
}

// --- email / subscription variants ---

#[test]
fn no_email_shows_empty_display_name() {
    let account = make_account(None, None);
    let line = plain("work", &account, true, false);
    assert_eq!(line, "  work ");
}

#[test]
fn no_subscription_omits_sub_tag() {
    let account = make_account(Some("user@example.com"), None);
    let line = plain("work", &account, true, false);
    assert_eq!(line, "  work user@example.com");
}

// --- ls prefix consistency ---

#[test]
fn ls_default_matches_star_prefix() {
    let account = make_account(Some("a@example.com"), Some("pro"));
    let line = plain("base", &account, true, true);
    assert!(line.starts_with("* "));
}

#[test]
fn ls_non_default_matches_space_prefix() {
    let account = make_account(Some("b@example.com"), Some("pro"));
    let line = plain("work", &account, true, false);
    assert!(line.starts_with("  "));
}

#[test]
fn ls_logged_out_matches_bang_prefix() {
    let account = make_account(Some("c@example.com"), Some("pro"));
    let line = plain("test", &account, false, false);
    assert!(line.starts_with("! "));
}
