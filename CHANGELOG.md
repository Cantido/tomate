# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

[unreleased]: https://github.com/Cantido/tomate/compare/v0.4.0...HEAD

## [0.4.0] - 2025-05-19

**BREAKING** - The expected names of hook executables have changed.

### Changed

- The hook executables now have much clearer names. The hook directory has not changed (`${XDG_CONFIG_DIR}/tomate/hooks`). Rename your hook files if you have any. Tomate how looks for files with these names:
    - `pomodoro-start`
    - `pomodoro-end`
    - `shortbreak-start`
    - `shortbreak-end`
    - `longbreak-start`
    - `longbreak-end`


[0.4.0]: https://github.com/Cantido/tomate/compare/v0.3.0..v0.4.0

## [0.3.0] - 2024-12-15

**BREAKING** - This tool was re-licensed to `AGPL-3.0-or-later`.

### Added

- Tomate now sets a systemd timer that invokes the `tomate timer check` command when it expires.
  This means that tomate will execute hooks as soon as a timer ends, instead of when you run `tomate finish`.

### Changed

- Panics are now a lot more friendly thanks to the [`human_panic`](https://github.com/rust-cli/human-panic) package.
- History entries now record the actual duration of the Pomodoro timer, not just the duration that the timer was set to.

### Fixed

- The `-c <path>`/`--config=<path>` argument is now correctly handled.

[0.3.0]: https://github.com/Cantido/tomate/compare/v0.2.0..v0.3.0

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
