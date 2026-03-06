# ccam — Claude Code Multi-Account Manager

Claude Code subscriptions come with usage limits that vary by plan. The higher the plan, the more you can use — but for most individuals, subscribing to the most expensive tier just to avoid hitting limits isn't realistic. A practical alternative is to use multiple accounts and spread usage across them.

However, Claude Code has no built-in way to switch between accounts — you would have to log out, log back in with a different account, and repeat every time you want to switch.

ccam keeps each account's session isolated so you can switch between them with a single command, instantly, without ever logging out.

## Requirements

- macOS
- [Claude Code](https://claude.ai/code) installed (`claude` binary in PATH)
- Rust (for building from source)

## Installation

```bash
git clone https://github.com/yourname/ccam.git
cd ccam
cargo install --path .
```

## Shell Integration

Shell integration is required for `ccam use` to take effect in the current shell session.

**zsh** — add to `~/.zshrc`:
```zsh
eval "$(ccam init zsh)"
```

**bash** — add to `~/.bashrc`:
```bash
eval "$(ccam init bash)"
```

**fish** — add to `~/.config/fish/config.fish`:
```fish
ccam init fish | source
```

## Usage

### Add an account

```bash
ccam add personal                         # Creates ~/.claude-accounts/personal and logs in
ccam add work --description "Work account"
ccam add main --dir ~/.claude             # Reuse an existing directory
ccam add staging --no-login               # Create directory only, login later
```

The first account added is automatically set as the default.

### Switch accounts

```bash
ccam use personal      # Switch in the current shell session
```

When a new terminal opens, the default account is applied automatically via the shell integration.

### List accounts

```bash
ccam list
```

```
  personal   hyojoong <hyojoong@gmail.com>
* work        jean <jean@company.com>        (default)
```

### Active account

```bash
ccam active            # Show the active account in the current session
ccam active --short    # Print only the alias (useful for shell prompt integration)
```

### Auth status

```bash
ccam status            # Summary of all accounts
ccam status work       # Detailed info for a specific account
```

```
work (default)
  path    /Users/username/.claude-accounts/work
  added   2026-03-05
  auth    OAuth ✓  Keychain ✓
  user    Jean <jean@company.com>  [stripe_subscription]
```

### Default account

```bash
ccam default work      # Set default account
ccam default           # Show current default
ccam default --unset   # Remove default
```

### Login / Logout

```bash
ccam login personal    # Browser OAuth login
ccam logout personal   # Logout (removes Keychain token)
```

### Remove an account

```bash
ccam remove personal           # Unregister account
ccam remove personal --purge   # Unregister and delete config directory
```

### Keychain management

```bash
ccam keychain list             # Keychain status for all accounts
ccam keychain status-default   # Check legacy default Keychain entry
ccam keychain clean-default    # Remove legacy default Keychain entry
ccam keychain remove personal  # Remove Keychain entry for a specific account
```

## How it works

Claude Code uses `CLAUDE_CONFIG_DIR` as its config directory when set, and looks up its auth token in macOS Keychain using a key derived from a SHA256 hash of that path.

`ccam use <alias>` prints an `export` statement that the shell integration evaluates in the current shell:

```
ccam use work
→ outputs: export CLAUDE_CONFIG_DIR="/Users/username/.claude-accounts/work"
→ shell function evals the output → applies to current shell
→ claude uses the Keychain token for the work account
```

## Config file

**`~/.claude-accounts/accounts.toml`**

```toml
default = "work"

[accounts.work]
config_dir = "/Users/username/.claude-accounts/work"
description = "Work account"
added_at = "2026-03-05T09:00:00Z"

[accounts.personal]
config_dir = "/Users/username/.claude-accounts/personal"
added_at = "2026-03-05T10:00:00Z"
```

Tokens and credentials are never stored here — only paths and metadata.

## Existing Claude Code users

If you have been using Claude Code without `CLAUDE_CONFIG_DIR`, your existing login is preserved. When no ccam default is set, the shell integration does not modify `CLAUDE_CONFIG_DIR`, so Claude Code continues to use its built-in default.

To bring your existing `~/.claude` directory into ccam:

```bash
ccam add main --dir ~/.claude   # Re-login required
ccam keychain clean-default      # Optionally clean up the legacy Keychain entry
```

## License

MIT
