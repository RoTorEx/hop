# jumper Architecture

`jumper` is a local-first shell companion for quickly moving between Git projects
on developer machines, VMs, and VPS hosts.

## Runtime Flow

1. The binary scans a root directory, `$HOME` by default.
2. A directory below the scan root is treated as a project when it contains
   `.git`.
3. Known noisy directories such as `node_modules`, `target`, virtualenvs, caches,
   and hidden directories are skipped.
4. Projects are grouped by their parent folder into lettered sectors.
5. The interactive UI is written to stderr.
6. Jump mode writes the selected path as the only stdout output.
7. Copy mode writes no stdout and sends the selected path to the system
   clipboard with an available platform clipboard command.
8. The shell wrapper captures stdout and runs `cd` in the caller shell when jump
   mode returns a path.

The binary never changes directory itself because child processes cannot change
the parent shell's working directory.

## Boundaries

- The CLI reads local directory metadata only.
- The CLI does not write local state, config, logs, or telemetry, except when
  `--copy-path` explicitly writes the selected path to the system clipboard.
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
