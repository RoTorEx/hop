# jumper

Tiny interactive project navigator for shells on local machines, VMs, and VPS
hosts. It scans for Git projects, lets you pick one, and prints only the chosen
path so a shell wrapper can `cd` there cleanly.

## Quickstart

```bash
make install
make check
```

Run locally:

```bash
make run
```

Install from GitHub on a VM/VPS:

```bash
curl -fsSL https://raw.githubusercontent.com/RoTorEx/jumper/main/scripts/install.sh | sh
```

If the source repository requires GitHub authentication, pass the installer
token as `GH_INSTALLER_TOKEN`:

```bash
GH_INSTALLER_TOKEN="$(gh auth token)" sh -c 'curl -fsSL -H "Authorization: Bearer $GH_INSTALLER_TOKEN" https://raw.githubusercontent.com/RoTorEx/jumper/main/scripts/install.sh | GH_INSTALLER_TOKEN="$GH_INSTALLER_TOKEN" sh'
```

Pin a release or branch:

```bash
curl -fsSL https://raw.githubusercontent.com/RoTorEx/jumper/main/scripts/install.sh | sh -s -- --ref vX.Y.Z
```

The installer builds with Cargo, copies the binary to
`~/.x-cli-jumper/bin/jumper`, writes the shell bridge to
`~/.x-cli-jumper/init.zsh`, stores a supplied private repo update token at
`~/.x-cli-jumper/gh-token` with file mode `0600`, and adds one plain line to
bash/zsh profile files:

```bash
source "$HOME/.x-cli-jumper/init.zsh"
```

The bridge is required because a child process cannot change its parent shell's
working directory. It adds `~/.x-cli-jumper/bin` to PATH without duplicates,
delegates all argument parsing to the Rust CLI, uses the absolute installed
binary path, validates the selected directory, and contains the only `cd`
needed by the integration. `jumper update` refreshes both the binary and this
bridge.

Open a new shell or source your profile, then run:

```bash
jumper
jumper ~
jumper A1
jumper b1
jumper --copy-path A1
```

## Usage

```bash
jumper --help
jumper ~
jumper config
jumper -v
jumper A1
jumper --copy-path A1
jumper update
jumper --root /srv
```

Interactive UI, help, and version output are written to stderr. The installed
shell integration makes `jumper` change the current shell directory in jump
mode. Sector labels are case-insensitive, so `jumper B1` and `jumper b1` are
equivalent. The underlying binary prints the selected path as its only stdout
output, which keeps shell integration safe and predictable. Copy mode writes no
stdout and copies the selected path with `pbcopy`, `wl-copy`, `xclip`, or `xsel`.

`jumper ~` jumps directly to the jumper home directory, `~/.x-cli-jumper`.

`jumper config` scans `$HOME` and creates or updates
`~/.x-cli-jumper/config.toml`. Existing `active = true` or `active = false`
values are preserved, and newly discovered projects default to `active = true`.
Projects are written in alphanumeric path order. Edit `active = false` to hide a
project from normal `jumper` results. Pass `--root <dir>` to refresh
from a different scan root. Passing `--root` to normal jump mode still performs
an ad hoc scan instead of using the config.

Normal `jumper` jump mode requires the config file. If it is missing, jumper
prints an alert and exits; run `jumper config` first.

`jumper update` replaces `~/.x-cli-jumper/bin/jumper` and refreshes `init.zsh`
from the latest Linux or macOS release for the current CPU architecture. It
requires `curl` or `wget`, plus `tar`. If `~/.x-cli-jumper/gh-token` exists,
updates use that token for GitHub authentication.

## Release Flow

Normal verification:

```bash
make check
make version
```

One-command guarded release:

```bash
make release
make release-push
```

`make release` prompts for the exact `MAJOR.MINOR.PATCH` version, runs checks,
updates release metadata, creates a dedicated release commit, and creates a
`vX.Y.Z` tag. `make release-push` pushes `main` and tags.

Pushing a `vX.Y.Z` tag triggers the GitHub Actions workflow that builds and
attaches Linux and macOS x86_64 and aarch64 release binaries.

## Kernel sync (sanity check)

```bash
make vibe-kernel-set
make vibe-pull
```

`make vibe-kernel-set` prompts for the parent kernel path and updates `.vibe/KERNEL_SOURCE` (local-only; gitignored).

Confirm these exist after pulling:

- `.vibe/kernel/PRINCIPLES.md`
- `.githooks/pre-commit`
- `TASK.md`
- `CHANGELOG.md`
- `AGENTS.md` contains the `VIBE:KERNEL_ROUTING` markers

## Docs map

- Keep the repo root minimal. Prefer putting project docs under `docs/` rather than adding many root markdown files.
- `AGENTS.md` — agent router.
- `TASK.md` — task queue (agents process and remove completed tasks).
- `CHANGELOG.md` — release progress (if this project releases).
- `.vibe/kernel/*.md` — local copies of Vibecoding Kernel instructions (do not edit).
- `.githooks/` — optional git hooks managed by the kernel (lint gates).
- `docs/architecture/` — design truth (agents choose scope; keep schemas/diagrams/boundaries up to date).
- `docs/contracts/` — stable contracts.
- `docs/features/` — accepted feature notes.
- `docs/ideas/` — raw ideas, not roadmap.
- `docs/reports/` — reports and audits (read only when relevant).

## Commands

See `Makefile`.
