# ccam — Claude Code Multi-Account Manager

Claude Code subscriptions come with usage limits that vary by plan. The higher the plan, the more you can use — but for most individuals, subscribing to the most expensive tier just to avoid hitting limits isn't realistic. A practical alternative is to use multiple accounts and spread usage across them.

However, Claude Code only supports one account at a time. Even with multiple terminal windows open, they all share the same session. To switch accounts, you have to log out and log back in — every time.

ccam removes that friction. Each terminal session can run a different Claude Code account simultaneously, making it seamless to distribute usage across accounts and get the most out of your subscriptions.

## Overview

### Account Switching Model

**Default Account** (`ccam default <alias>`) — the account applied automatically whenever a new terminal session opens. Set it once, and every new terminal starts with that account active. Managed via shell integration.

**Session Switch** (`ccam use <alias>`) — temporarily switches the account in the current shell session only. Useful when you want to use a different account without changing the default. The switch does not affect other terminal windows, and reverts to the default when you open a new session.

```
 New terminal           Current terminal        New terminal
 (default applies)      (ccam use)              (default applies)
┌───────────────────┐  ┌───────────────────┐  ┌───────────────────┐
│                   │  │                   │  │                   │
│  $ claude         │  │  $ ccam use user2 │  │  $ claude         │
│    account: user1 │  │  $ claude         │  │    account: user1 │
│    (default)      │  │    account: user2 │  │    (default)      │
│                   │  │    (this session  │  │                   │
└───────────────────┘  │     only)         │  └───────────────────┘
                       └───────────────────┘
```

### Shared files across accounts

Giving each account its own `CLAUDE_CONFIG_DIR` isolates credentials, but it also separates files like `settings.json`, `CLAUDE.md`, and `plugins/` — meaning changes in one account don't carry over to others.

ccam solves this by symlinking those files through a shared directory (`~/.claude-accounts/shared/`), which itself points to `~/.claude/`. Every account directory gets symlinks for the shared items, so all accounts read and write the same files.

**Shared items:** `settings.json`, `CLAUDE.md`, `plugins/`

```
~/.claude/
├── settings.json          # ← single source of truth (shared across all accounts)
├── CLAUDE.md
└── plugins/

~/.claude-accounts/
├── accounts.toml          # account registry (paths, metadata; no credentials)
├── shared/
│   ├── settings.json ──→  ~/.claude/settings.json   (symlink)
│   ├── CLAUDE.md ──────→  ~/.claude/CLAUDE.md        (symlink)
│   └── plugins/ ───────→  ~/.claude/plugins/         (symlink)
├── account1/              # CLAUDE_CONFIG_DIR for account1
│   ├── settings.json ──→  ../shared/settings.json    (symlink)
│   ├── CLAUDE.md ──────→  ../shared/CLAUDE.md        (symlink)
│   ├── plugins/ ───────→  ../shared/plugins/         (symlink)
│   └── ...                # auth state, project history (per-account)
└── account2/
    ├── settings.json ──→  ../shared/settings.json    (symlink)
    ├── CLAUDE.md ──────→  ../shared/CLAUDE.md        (symlink)
    ├── plugins/ ───────→  ../shared/plugins/         (symlink)
    └── ...
```

## Requirements

- macOS
- [Claude Code](https://claude.ai/code) installed (`claude` binary in PATH)

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/JeanTracker/ccam/master/install.sh | sh
```

Installs to `~/.local/bin`.

**Build from source** (requires Rust):
```bash
cargo install --git https://github.com/JeanTracker/ccam
```

## Shell Integration

Shell integration is required for `ccam use` to take effect in the current shell session.

**zsh** — add to `~/.zshrc`:
```zsh
export PATH="$HOME/.local/bin:$PATH"
eval "$(ccam init zsh)"
```

**bash** — add to `~/.bashrc`:
```bash
export PATH="$HOME/.local/bin:$PATH"
eval "$(ccam init bash)"
```

**fish** — add to `~/.config/fish/config.fish`:
```fish
fish_add_path "$HOME/.local/bin"
ccam init fish | source
```

## Usage

### Add an account

```bash
ccam add alice                             # Create ~/.claude-accounts/alice
ccam add bob --description "Secondary"    # With a description
ccam add main --dir ~/.claude             # Reuse an existing directory
```

The first account added is automatically set as the default.

`ccam add` launches Claude Code immediately so you can complete the login flow right away.

### Switch accounts

```bash
ccam use account1      # Switch in the current shell session
```

When a new terminal opens, the default account is applied automatically via the shell integration.

### List accounts

```bash
ccam list   # alias: ls
* account1 user1@example.com (pro)
  account2 user2@example.com (pro)
! account3
```

`*` marks the default account. `!` indicates not logged in.

### Active account

```bash
ccam active            # Show the active account in the current session
```

### Auth status

```bash
ccam status account1    # Detailed info for a specific account
account1 (default)
  path     /Users/username/.claude-accounts/account1
  added    2026-03-05
  auth     Keychain ✓
  account  user1@example.com (pro)
```

### Default account

```bash
ccam default account1    # Set default account
ccam default --get       # Show current default
```

### Remove an account

```bash
ccam remove account1      # Unregister account and delete config directory
ccam remove account1 -y   # Skip confirmation prompt
ccam rm account1          # alias: rm
```


## How it works

Claude Code uses `CLAUDE_CONFIG_DIR` as its config directory when set, and looks up its auth token in macOS Keychain using a key derived from a SHA256 hash of that path.

`ccam use <alias>` prints an `export` statement that the shell integration evaluates in the current shell:

```
ccam use account1
→ outputs: export CLAUDE_CONFIG_DIR="/Users/username/.claude-accounts/account1"
→ shell function evals the output → applies to current shell
→ claude uses the Keychain token for account1
```

## Config file

**`~/.claude-accounts/accounts.toml`**

```toml
default = "account1"

[accounts.account1]
config_dir = "/Users/username/.claude-accounts/account1"
added_at = "2026-03-05T09:00:00Z"
email = "user1@example.com"
subscription_type = "pro"

[accounts.account2]
config_dir = "/Users/username/.claude-accounts/account2"
added_at = "2026-03-05T10:00:00Z"
email = "user2@example.com"
subscription_type = "pro"
```

Tokens and credentials are never stored here — only paths and metadata.

## Existing Claude Code users

If you have been using Claude Code without `CLAUDE_CONFIG_DIR`, your existing login is preserved. When no ccam default is set, the shell integration does not modify `CLAUDE_CONFIG_DIR`, so Claude Code continues to use its built-in default.

To bring your existing `~/.claude` directory into ccam:

```bash
ccam add main --dir ~/.claude
```
