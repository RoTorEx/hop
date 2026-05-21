# Project Changelog

Tracks real product and release progress.

## [Unreleased]

## [0.1.4] - 2026-05-21

### Changed

- Standardized the Makefile release flow and Cargo build artifact path against
  the refreshed Vibecoding Kernel rules.
- Moved help and version output to stderr so stale shell wrappers cannot treat
  that text as a jump path.

### Added

- Added `-v` as a version alias.

## [0.1.3] - 2026-05-21

### Added

- Added `make version` as the standard repo command for checking the CLI
  version.

### Fixed

- Fixed the `j` shell wrapper so help, version, shell-init, and update commands
  run directly instead of being treated as jump paths.

## [0.1.2] - 2026-05-21

### Added

- Added `jumper update` to update the current executable from the latest GitHub
  release.
- Added Linux aarch64 release binaries for updater support on ARM hosts.

## [0.1.1] - 2026-05-21

### Added

- Added direct target selection with `jumper <target>` and shell wrapper support
  for `j <target>`.
- Added `--copy-path` to copy the selected project path instead of jumping.

## [0.1.0] - 2026-05-16

### Added

- Packaged the raw navigator as a Cargo-based Rust CLI.
- Added a GitHub installer that builds from source into `~/.x-cli-jumper`.
- Added release bump, tag, publish, and GitHub Actions binary build workflow.
- Added a plain one-command release flow that follows the project release rules.
