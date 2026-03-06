# Changelog

All notable changes to this project will be documented in this file.

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
