# Project Changelog

Tracks real product and release progress.

## [Unreleased]

### Changed

- Reduced shell profile setup to one ordinary source line; the sourced bridge
  now owns idempotent PATH setup and same-shell directory changes.
- Moved the installed executable to `~/.x-cli-jumper/bin/jumper` and added a
  migration that removes only known legacy profile entries and the old
  root-level binary while preserving config, tokens, and caches.
- Updated self-update path discovery to support both the legacy root-level
  executable and the new `bin/` layout.
- Limited automatic profile edits to the active bash or zsh profile instead of
  touching multiple profiles or writing bash/zsh syntax into `.profile`.

## [0.2.5] - 2026-07-14

### Changed

- Reduced managed shell profile integration to one sourced bridge file and
  moved all CLI argument handling back into the Rust executable.
- Made the shell bridge call the absolute installed binary, keep PATH updates
  idempotent, validate destinations, and refresh during `jumper update`.
- Added a narrow installer migration for unmarked PATH and `j()` snippets from
  early Jumper installations.
- Added an explicit `~/.x-cli-jumper` PATH export that can live alongside the
  other installed CLI tools while the small shell bridge remains separately
  managed.

### Fixed

- Raw interactive executable use now explains why it cannot change the parent
  shell instead of silently leaving the user in the original directory.

## [0.2.4] - 2026-07-12

### Changed

- Removed the `j` shorthand and made `jumper` the only installed shell command.
  New shell integration removes legacy `j` aliases and functions.

## [0.2.3] - 2026-07-12

### Fixed

- Made both installed shell commands, `j` and `jumper`, change the current
  directory in jump mode while preserving direct dispatch for CLI commands.
- Preserved successful exit status for shell-wrapped modes that intentionally
  produce no destination path.

## [0.2.2] - 2026-06-17

### Added

- Added macOS x86_64 and aarch64 release artifacts and `jumper update` support.

## [0.2.1] - 2026-06-17

### Fixed

- Fixed generated shell integration so stale `jumper()` shell functions and
  `j` aliases from older installs no longer intercept `jumper config` or
  wrapped `j` commands.

## [0.2.0] - 2026-06-14

### Added

- Added `jumper config` to maintain `~/.x-cli-jumper/config.toml` and let users
  hide projects by editing `active = false`.
- Added `jumper ~` as a shortcut to the jumper home directory.

### Changed

- Config projects are now written in alphanumeric path order.
- Normal jump mode now requires `~/.x-cli-jumper/config.toml` instead of
  falling back to a full `$HOME` scan.

## [0.1.5] - 2026-05-22

### Added

- Added `GH_INSTALLER_TOKEN` support for authenticated installs and token-backed
  `jumper update` downloads.

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
