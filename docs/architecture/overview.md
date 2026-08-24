# Hop Architecture

`hop` is a local-first shell companion for quickly moving between Git projects
on developer machines, VMs, and VPS hosts.

## Runtime Flow

1. Normal jump mode requires `~/.x-cli-hop/config.toml`; when it is missing,
   the binary fails with an alert to run `hop config`.
2. Passing `--root <dir>` to jump mode performs an explicit ad hoc scan instead
   of using the config.
3. Every non-config command compares the configured project tree with the
   current folders under the config's scan root. If projects were added or
   removed, it prints a stderr warning and continues; `hop config` is the
   repair command.
4. A directory below a scan root is treated as a project when it contains
   `.git`.
5. Known noisy directories such as `node_modules`, `target`, virtualenvs, caches,
   and hidden directories are skipped.
6. Projects are grouped by their parent folder into lettered sectors.
7. The interactive UI is written to stderr.
8. Jump mode writes the selected path as the only stdout output.
9. The `~` target is a shortcut for the hop home directory,
   `~/.x-cli-hop`.
10. Copy mode writes no stdout and sends the selected path to the system
   clipboard with an available platform clipboard command.
11. Profile files contain only one integration line:
    `source "$HOME/.x-cli-hop/init.zsh"`. The bridge idempotently adds
    `~/.x-cli-hop/bin` to PATH, calls the absolute installed binary, captures
    its stdout, validates the returned directory, and runs `cd` in the caller
    shell. The Rust CLI owns all argument parsing.
12. If the raw executable runs from a terminal without the bridge, it reports
    that it cannot change its parent shell instead of silently printing a path.
13. `--frequent` loads local jump history, globally sorts active projects by
    descending count and alphanumeric path, and uses one-based numeric
    selectors instead of sector labels.
14. After a project `cd` succeeds, the shell bridge invokes the executable's
    internal recorder. The recorder atomically updates
    `~/.x-cli-hop/history.toml`; copy mode, failed changes, and `hop ~` do not
    record a jump.
15. `list` renders the default grouped view, or the ranked view when combined
    with `--frequent`, and exits without reading a selection or writing a path
    to stdout.

`hop config` refreshes the config file by scanning `$HOME` or an explicit
`--root <dir>`, recording that scan root, preserving manually edited
`active = true` or `active = false` values for projects that are still present,
adding newly discovered projects, and removing paths that are no longer
discovered under that root.

The binary never changes directory itself because child processes cannot change
the parent shell's working directory; the installed `hop` shell wrapper
provides that behavior.

## Boundaries

- The CLI reads local directory metadata only.
- `hop config` writes local project selection state to
  `~/.x-cli-hop/config.toml`.
- The CLI does not write logs or telemetry. It writes local successful-jump
  counts to `~/.x-cli-hop/history.toml`; `--copy-path` explicitly writes the
  selected path to the system clipboard.
- Network access is limited to the optional installer, GitHub release flow, and
  explicit `hop update` command.
- Installation writes one binary to `~/.x-cli-hop/bin/hop`, writes the
  shell bridge to `~/.x-cli-hop/init.zsh`, and updates only the active
  bash/zsh profile with one idempotent source line. Unsupported shells are left
  unchanged. The installer removes the known root-level binary and legacy
  Hop profile entries without touching config, tokens, or caches.
- `make install-local` installs the current checkout through the same installer
  path without downloading a GitHub source archive.
- For authenticated GitHub installs, the installer reads `GH_INSTALLER_TOKEN`
  and stores it at `~/.x-cli-hop/gh-token` with mode `0600` for later
  updates.
- `hop update` downloads the latest matching release archive from GitHub
  Releases, using `~/.x-cli-hop/gh-token` when present, and refreshes the
  current executable and root-level generated shell bridge through atomic file
  replacements. The updater recognizes both legacy root-level and `bin/`
  executable layouts.
