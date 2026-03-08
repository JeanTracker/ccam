mod claude;
mod commands;
mod config;
mod confirm;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ccam", about = "Claude Code multi-account manager", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add a new account (login via `claude` after switching with `ccam use`)
    Add {
        alias: String,
        /// Use an existing directory instead of creating a new one
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long, short)]
        description: Option<String>,
    },

    /// List all registered accounts
    List {
        /// Print account names only (for shell completion)
        #[arg(long)]
        names_only: bool,
    },

    /// Remove an account and delete its config directory
    Remove { alias: String },

    /// [Internal] Output `export CLAUDE_CONFIG_DIR=...` for eval
    #[command(name = "__env", hide = true)]
    InternalEnv { alias: String },

    #[command(hide = true)]
    Env { alias: String },

    /// Switch to an account in the current shell (use via: eval "$(ccam use <alias>)")
    Use { alias: String },

    /// Set or get the default account
    Default {
        alias: Option<String>,
        /// Print the current default account name
        #[arg(long)]
        get: bool,
    },

    /// Show the currently active account (based on CLAUDE_CONFIG_DIR)
    Active {
        /// Print only the account alias (for shell prompt integration)
        #[arg(long)]
        short: bool,
    },

    /// Show login status of accounts
    Status { alias: Option<String> },

    #[command(hide = true)]
    Init { shell: String },

    #[command(hide = true)]
    Keychain {
        #[command(subcommand)]
        subcommand: KeychainCommand,
    },
}

#[derive(Subcommand)]
enum KeychainCommand {
    /// List Keychain status for all registered accounts
    List,
    /// Check if the legacy default Keychain entry exists
    StatusDefault,
    /// Remove the legacy default Keychain entry (requires 'yes' confirmation)
    CleanDefault,
    /// Remove Keychain entry for a specific account (requires 'yes' confirmation)
    Remove { alias: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Add {
            alias,
            dir,
            description,
        } => {
            commands::add::run(&alias, dir.as_ref(), description.as_deref())?;
        }

        Command::List { names_only } => {
            commands::list::run(names_only)?;
        }

        Command::Remove { alias } => {
            commands::remove::run(&alias)?;
        }

        Command::InternalEnv { alias } | Command::Env { alias } => {
            commands::env::run(&alias)?;
        }

        Command::Use { alias } => {
            commands::env::run(&alias)?;
        }

        Command::Default { alias, get } => {
            if get {
                if let Some(d) = config::get_default()? {
                    println!("{}", d);
                }
            } else if let Some(a) = alias {
                config::set_default(Some(&a))?;
                eprintln!("Default account set to: {}", a);
            } else {
                match config::get_default()? {
                    Some(d) => eprintln!("Default account: {}", d),
                    None => eprintln!("No default account set."),
                }
            }
        }

        Command::Active { short } => {
            commands::status::run_current(short)?;
        }

        Command::Status { alias } => {
            commands::status::run_status(alias.as_deref())?;
        }

        Command::Init { shell } => {
            commands::init::run(&shell)?;
        }

        Command::Keychain { subcommand } => match subcommand {
            KeychainCommand::List => commands::keychain::run_list()?,
            KeychainCommand::StatusDefault => commands::keychain::run_status_default()?,
            KeychainCommand::CleanDefault => commands::keychain::run_clean_default()?,
            KeychainCommand::Remove { alias } => commands::keychain::run_remove(&alias)?,
        },
    }

    Ok(())
}
