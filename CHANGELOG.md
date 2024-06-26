# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

[unreleased]: https://github.com/Cantido/tomate/compare/v0.1.1...HEAD

## [0.2.0] - 2024-04-06

### Added

- The library is now documented

### Changed

- Now uses the [`chrono`](https://docs.rs/chrono) library in the public API

### Fixed

- `%s` (started time in RFC 3339 format) token in the `--format` argument. This was in the docs but not implemented.
- `%e` (end time in RFC 3339 format) token in the `--format` argument. This was in the docs but not implemented.

### Removed

- CLI-specific functions were moved from the core lib and into the CLI executable

[0.2.0]: https://github.com/Cantido/tomate/compare/v0.1.1..v0.2.0

## [0.1.1] - 2024-04-05

### Fixed

- Fixed a subcommand option to clap

[0.1.1]: https://github.com/Cantido/tomate/compare/v0.1.0..v0.1.1

## [0.1.0] - 2024-04-05

### Added

- Starting Pomodoro, short break, and long break timers
- Saving history of past Pomodoros
- Loading config from TOML file

[0.1.0]: https://github.com/Cantido/tomate/releases/tag/v0.1.0
