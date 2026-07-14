# jumper Architecture

`jumper` is a local-first shell companion for quickly moving between Git projects
on developer machines, VMs, and VPS hosts.

## Runtime Flow

1. Normal jump mode requires `~/.x-cli-jumper/config.toml`; when it is missing,
   the binary fails with an alert to run `jumper config`.
2. Passing `--root <dir>` to jump mode performs an explicit ad hoc scan instead
   of using the config.
3. A directory below a scan root is treated as a project when it contains
   `.git`.
4. Known noisy directories such as `node_modules`, `target`, virtualenvs, caches,
   and hidden directories are skipped.
5. Projects are grouped by their parent folder into lettered sectors.
6. The interactive UI is written to stderr.
7. Jump mode writes the selected path as the only stdout output.
8. The `~` target is a shortcut for the jumper home directory,
   `~/.x-cli-jumper`.
9. Copy mode writes no stdout and sends the selected path to the system
   clipboard with an available platform clipboard command.
10. Profile files contain a normal PATH export for `~/.x-cli-jumper` and a
    managed source line for `~/.x-cli-jumper/init.zsh`. The installer preserves
    an existing canonical export so it can live alongside the user's other CLI
    paths. The bridge calls the absolute installed binary, captures its stdout,
    validates the returned directory, and runs `cd` in the caller shell. The
    Rust CLI owns all argument parsing.
11. If the raw executable runs from a terminal without the bridge, it reports
    that it cannot change its parent shell instead of silently printing a path.

`jumper config` refreshes the config file by scanning `$HOME` or an explicit
`--root <dir>`, merging newly discovered projects into the existing file, and
preserving manually edited `active = true` or `active = false` values.

The binary never changes directory itself because child processes cannot change
the parent shell's working directory; the installed `jumper` shell wrapper
provides that behavior.

## Boundaries

- The CLI reads local directory metadata only.
- `jumper config` writes local project selection state to
  `~/.x-cli-jumper/config.toml`.
- The CLI does not write logs or telemetry. `--copy-path` explicitly writes the
  selected path to the system clipboard.
- Network access is limited to the optional installer, GitHub release flow, and
  explicit `jumper update` command.
- Installation writes one binary to `~/.x-cli-jumper/jumper`, writes the shell
  bridge to `~/.x-cli-jumper/init.zsh`, and updates bash/zsh profile files with
  an idempotent PATH export and marked source block.
- For authenticated GitHub installs, the installer reads `GH_INSTALLER_TOKEN`
  and stores it at `~/.x-cli-jumper/gh-token` with mode `0600` for later
  updates.
- `jumper update` downloads the latest matching release archive from GitHub
  Releases, using `~/.x-cli-jumper/gh-token` when present, and refreshes the
  current executable and generated shell bridge through atomic file replacements.
