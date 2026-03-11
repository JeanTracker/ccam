# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-03-12

### Added
- `--yes / -y` flag for `remove` command to skip confirmation prompt
- Shell wrappers (bash/zsh/fish) now eval `remove` output to apply `CLAUDE_CONFIG_DIR` changes in the current shell session
- `rm` alias for `remove` in all shell wrappers
- `sorted_accounts()` method on `AccountsConfig` to consolidate sort-by-alias patterns
- SHA256 checksum files generated alongside release tarballs
- Comprehensive test suite: unit and integration tests for `remove`, `use`, `status`, shell integration, shared-path scenarios, and display formatting

### Changed
- `remove`: auto-reassigns default to alphabetically first remaining account after deletion
- `add`: auto-default assignment now lives in `config::add_account()` (previously scattered)
- Account line display unified across `ls`, `use`, and `active` commands (consistent prefix and color)
- Release workflow now gates build on passing tests (`cargo fmt`, `clippy --all-targets`, `cargo test --locked`)
- Release triggered on master push (Cargo.toml version change) instead of tag push
- Fixed `ccm` → `ccam` typo in user-facing hints (`list`, `keychain`)

### Fixed
- `remove`: skip keychain logout and directory cleanup when `config_dir` is shared by another account
- `status` / `active`: resolve active account deterministically when multiple accounts share the same `config_dir` (priority: default account first, then alphabetically first)
- `dir_keychain_service`: normalize trailing slash before hashing so `/path/` and `/path` map to the same keychain service name
- Shell `rm`/`remove` wrapper: restored interactive stdin by replacing command substitution with a temp file, so confirmation prompts work correctly

## [0.3.1] - 2026-03-08

### Changed
- Auto-tag on master merge when Cargo.toml version changes

## [0.3.0] - 2026-03-08

### Added
- Auto-run Claude Code on `ccam add` to complete login immediately after account creation
- Fetch and cache user email and subscription type per account via `claude auth status`
- Display login state and user info in `ccam list` and `ccam active`
- `--no-refresh` flag for shell init to skip keychain refresh on shell startup

### Changed
- `ccam list`: show `*` prefix for default account, `!` for unauthenticated accounts
- `ccam status <alias>`: add `account` line showing email and subscription type
- Removed separate login/logout subcommands; login is handled through Claude Code directly
- All code, comments, and user-facing strings translated to English

### Fixed
- Preserve existing login when adding or switching to an account backed by `~/.claude`

## [0.2.0] - 2026-03-06

### Added
- Symlink shared files across accounts for unified configuration management

### Changed
- Extract `display_info()` to deduplicate account formatting logic

## [0.1.0] - 2026-03-06

### Added
- Initialize ccam project
- Add `accounts.toml` config layer for multi-account management
- Add claude auth integration
- Implement all subcommands: `add`, `remove`, `switch`, `list`, `status`
- Add shell integration for zsh, bash, and fish
- Add CI workflow for PR and master branch
- Add release workflow for automated binary distribution
- Add install script
