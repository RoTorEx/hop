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
6. The selected path is the only stdout output.
7. The shell wrapper captures stdout and runs `cd` in the caller shell.

The binary never changes directory itself because child processes cannot change
the parent shell's working directory.

## Boundaries

- The CLI reads local directory metadata only.
- The CLI does not write local state, config, logs, or telemetry.
- Network access is limited to the optional installer and GitHub release flow.
- Installation writes one binary to `~/.x-cli-jumper/jumper` and may update
  bash/zsh profile files with an idempotent marked block.
