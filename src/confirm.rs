use std::io::{self, Write};

/// Prompts "message [y/N]" — returns true only if user enters 'y' or 'Y'.
pub fn confirm_yn(message: &str) -> bool {
    eprint!("{} [y/N]: ", message);
    io::stderr().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    matches!(input.trim(), "y" | "Y")
}

/// Prompts "message (type 'yes' to confirm)" — returns true only if user types "yes".
pub fn confirm_yes(message: &str) -> bool {
    eprintln!("{}", message);
    eprint!("Continue? (type 'yes' to confirm): ");
    io::stderr().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    input.trim() == "yes"
}
