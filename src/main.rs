use anyhow::Result;
use ccam::{commands, config};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "ccam",
    about = "Claude Code multi-account manager",
    version = env!("CARGO_PKG_VERSION"),
    disable_version_flag = true
)]
struct Cli {
    /// Print version
    #[arg(short = 'v', long, action = clap::ArgAction::Version)]
    version: (),
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add a new account
    #[command(
        after_help = "If ~/.claude already exists, import it: ccam add <ALIAS> --dir ~/.claude"
    )]
    Add {
        /// Alias to identify the account
        #[arg(value_name = "ALIAS")]
        alias: String,
        /// Use an existing directory instead of creating a new one
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
        /// Short description for the account
        #[arg(long, short, value_name = "TEXT")]
        description: Option<String>,
    },

    /// List all registered accounts
    #[command(visible_alias = "ls")]
    List {
        #[arg(long, hide = true)]
        names_only: bool,
    },

    /// Remove an account and delete its config directory
    #[command(visible_alias = "rm")]
    Remove {
        /// Alias of the account to remove
        #[arg(value_name = "ALIAS")]
        alias: String,
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// [Internal] Output `export CLAUDE_CONFIG_DIR=...` for eval
    #[command(name = "__env", hide = true)]
    InternalEnv {
        #[arg(value_name = "ALIAS")]
        alias: String,
        /// Skip fetching user info (used during shell init to avoid startup delay)
        #[arg(long)]
        no_refresh: bool,
    },

    /// Switch to an account in the current shell
    Use {
        /// Alias of the account to switch to
        #[arg(value_name = "ALIAS")]
        alias: String,
    },

    /// Set or get the default account
    #[command(after_help = "To print the current default: ccam default --get")]
    Default {
        /// Account alias to set as default
        #[arg(value_name = "ALIAS", required_unless_present = "get")]
        alias: Option<String>,
        /// Print the current default account name
        #[arg(long, short)]
        get: bool,
    },

    /// Show the currently active account
    Active,

    /// Show details for an account
    Status {
        /// Alias of the account to inspect
        #[arg(value_name = "ALIAS")]
        alias: String,
    },

    #[command(hide = true)]
    Init {
        #[arg(value_name = "SHELL")]
        shell: String,
    },

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
    Remove {
        #[arg(value_name = "ALIAS")]
        alias: String,
    },
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

        Command::Remove { alias, yes } => {
            commands::remove::run(&alias, yes)?;
        }

        Command::InternalEnv { alias, no_refresh } => {
            commands::env::run(&alias, no_refresh)?;
        }
        Command::Use { alias } => {
            commands::env::run(&alias, false)?;
        }

        Command::Default { alias, get } => {
            if get {
                if let Some(d) = config::get_default()? {
                    println!("{}", d);
                }
            } else if let Some(a) = alias {
                config::set_default(Some(&a))?;
                eprintln!("Default account set to: {}", a);
            }
        }

        Command::Active => {
            commands::status::run_current()?;
        }

        Command::Status { alias } => {
            commands::status::run_status(&alias)?;
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
