# jumper Architecture

`jumper` is a local-first shell companion for quickly moving between Git projects
on developer machines, VMs, and VPS hosts.

## Runtime Flow

1. The binary uses `~/.x-cli-jumper/config.toml` when it exists and no ad hoc
   `--root` scan is requested.
2. Without a config file, or when `--root <dir>` is passed to jump mode, the
   binary scans a root directory (`$HOME` by default).
3. A directory below the scan root is treated as a project when it contains
   `.git`.
4. Known noisy directories such as `node_modules`, `target`, virtualenvs, caches,
   and hidden directories are skipped.
5. Projects are grouped by their parent folder into lettered sectors.
6. The interactive UI is written to stderr.
7. Jump mode writes the selected path as the only stdout output.
8. Copy mode writes no stdout and sends the selected path to the system
   clipboard with an available platform clipboard command.
9. The shell wrapper captures stdout and runs `cd` in the caller shell when jump
   mode returns a path.

`jumper config` refreshes the config file by scanning `$HOME` or an explicit
`--root <dir>`, merging newly discovered projects into the existing file, and
preserving manually edited `active = true` or `active = false` values.

The binary never changes directory itself because child processes cannot change
the parent shell's working directory.

## Boundaries

- The CLI reads local directory metadata only.
- `jumper config` writes local project selection state to
  `~/.x-cli-jumper/config.toml`.
- The CLI does not write logs or telemetry. `--copy-path` explicitly writes the
  selected path to the system clipboard.
- Network access is limited to the optional installer, GitHub release flow, and
  explicit `jumper update` command.
- Installation writes one binary to `~/.x-cli-jumper/jumper` and may update
  bash/zsh profile files with an idempotent marked block.
- For authenticated GitHub installs, the installer reads `GH_INSTALLER_TOKEN`
  and stores it at `~/.x-cli-jumper/gh-token` with mode `0600` for later
  updates.
- `jumper update` downloads the latest matching Linux release archive from
  GitHub Releases, using `~/.x-cli-jumper/gh-token` when present, and replaces
  the current executable.
