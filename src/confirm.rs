use std::io::{self, Write};

/// Prompts "message [y/N]" — returns true only if user enters 'y' or 'Y'.
pub fn confirm_yn(message: &str) -> bool {
    print!("{} [y/N]: ", message);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    matches!(input.trim(), "y" | "Y")
}

/// Prompts "message (type 'yes' to confirm)" — returns true only if user types "yes".
pub fn confirm_yes(message: &str) -> bool {
    println!("{}", message);
    print!("계속하시겠습니까? (yes/N): ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    input.trim() == "yes"
}
