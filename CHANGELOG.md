# Changelog

All notable changes to this project will be documented in this file.

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
