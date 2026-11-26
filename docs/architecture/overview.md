# Hop Architecture

`hop` is a local-first shell companion for quickly moving between Git projects
on developer machines, VMs, and VPS hosts.

## Runtime Flow

1. Jump mode loads both version 5 files under `~/.hop/routing/`.
   Without the routing directory, it reads the legacy `config.toml`.
2. Explicit `hop scan` walks the selected filesystem scope and reports candidates.
   Legacy `hop config` also scans when creating or refreshing a legacy config.
3. Navigation checks only saved project availability. Help, version, shell init,
   and update do not check the project tree.
4. A directory below a scan root is treated as a project when it contains
   `.git`.
5. Known noisy directories such as `construction_side`, `node_modules`,
   `target`, virtualenvs, caches, and hidden directories are skipped.
6. Sections sort by full root path and items by relative path, using natural
   alphanumeric order. Version 5 filters explicit sections by Personal, Work or
   Archive, and validates ownership across all scopes before selection.
7. The interactive UI is written to stderr.
8. Jump mode writes the selected path as the only stdout output.
9. The `~` target is a shortcut for the hop home directory,
   `~/.hop`.
10. Copy mode writes no stdout and sends the selected path to the system
   clipboard with an available platform clipboard command.
11. Profile files contain only one integration line:
    `source "$HOME/.hop/init.zsh"`. The bridge idempotently adds
    `~/.hop/bin` to PATH, calls the absolute installed binary, captures
    its stdout, validates the returned directory, and runs `cd` in the caller
    shell. The Rust CLI owns all argument parsing.
12. If the raw executable runs from a terminal without the bridge, it reports
    that it cannot change its parent shell instead of silently printing a path.
13. `--frequent` loads local jump history, globally sorts configured projects by
    descending count and alphanumeric path, and uses one-based numeric
    selectors instead of sector labels.
14. After a project `cd` succeeds, the shell bridge invokes the executable's
    internal recorder. The recorder atomically updates
    `~/.hop/history.toml`, keyed by a destination name derived from the
    registered paths; copy mode, failed changes, and `hop ~` do not record a
    jump.
15. `list` renders the default grouped view, or the ranked view when combined
    with `--frequent`, and exits without reading a selection or writing a path
    to stdout.

Each routing file stores flat sections for one scope. A section root can be a display
container or a registered destination. Parsing validates registered path
ownership even across scopes. Navigation filters available destinations,
assigns consecutive letters, and resolves the selector against that view.
The frequent view uses the same destinations and the history store.

`hop scan` walks local directories and sends progress and its report to stderr,
preserving the shell integration's stdout protocol. It does not write the
project config. Legacy bootstrap and refresh through `hop config` group
discovered repositories by immediate parent and save version 4.

The binary never changes directory itself because child processes cannot change
the parent shell's working directory; the installed `hop` shell wrapper
provides that behavior.

## Boundaries

The paths below are relative to the default `~/.hop` home. Executables in
`~/.hop`, `~/.toolbelt/hop` or a custom installation with a
`.hop-install` marker use their own home. Other binaries use `~/.hop`, with a
fallback to the legacy Toolbelt home if only it exists.

- The CLI reads local directory metadata only.
- `hop config` writes local project selection state to
  `~/.hop/config.toml`.
- The CLI writes bounded local usage logs and sends no telemetry. It writes local successful-jump
  counts to `~/.hop/history.toml`; `--copy-path` explicitly writes the
  selected path to the system clipboard.
- Network access is limited to the optional installer, GitHub release flow, and
  explicit `hop update` command.
- Installation writes one binary to `~/.hop/bin/hop`, writes the
  shell bridge to `~/.hop/init.zsh`, and updates only the active
  bash/zsh profile with one idempotent source line. Unsupported shells are left
  unchanged. The installer removes the known root-level binary and legacy
  Hop profile entries without touching config, tokens, or caches.
- `make install-local` installs the current checkout through the same installer
  path without downloading a GitHub source archive.
- For authenticated GitHub installs, the installer reads `GH_INSTALLER_TOKEN`
  and stores it at `~/.hop/gh-token` with mode `0600` for later
  updates.
- `hop update` downloads the latest matching release archive from GitHub
  Releases, using `~/.hop/gh-token` when present, and refreshes the
  current executable and root-level generated shell bridge through atomic file
  replacements. The updater recognizes both legacy root-level and `bin/`
  executable layouts.

## Windows

Windows x86_64 releases contain `hop.exe` in a ZIP archive. The Windows binary
uses `USERPROFILE` for local state and emits a PowerShell function from
`--shell-init`. The bridge adds the binary directory to the session's
`PATH`. The function uses the absolute executable path, changes the
caller directory with `Set-Location -LiteralPath`, and records successful
project jumps. Native output is decoded as UTF-8 to preserve Unicode paths.
Copy mode calls PowerShell `Set-Clipboard`. Windows updates are manual ZIP
replacement after the executable exits; the Unix self-updater is unchanged.

Windows default discovery queries PowerShell's
[`System.IO.DriveInfo`](https://learn.microsoft.com/en-us/dotnet/api/system.io.driveinfo) for fixed,
ready drive roots, validates the returned drive-letter paths, and combines
per-root scans. No Win32 unsafe code or new dependency is needed. Config format
3 introduced `scan_all_drives = true`; format 4 adds explicit sections while
retaining that scope. Full-drive scanning is available through `hop scan` or
legacy config bootstrap and refresh.
