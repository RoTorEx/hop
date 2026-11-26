# Hop

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

Build and install the current checkout on this machine:

```bash
make install-local
```

Install from GitHub on a VM/VPS:

```bash
curl -fsSL https://raw.githubusercontent.com/RoTorEx/hop/main/scripts/install.sh | sh
```

If the source repository requires GitHub authentication, pass the installer
token as `GH_INSTALLER_TOKEN`:

```bash
GH_INSTALLER_TOKEN="$(gh auth token)" sh -c 'curl -fsSL -H "Authorization: Bearer $GH_INSTALLER_TOKEN" https://raw.githubusercontent.com/RoTorEx/hop/main/scripts/install.sh | GH_INSTALLER_TOKEN="$GH_INSTALLER_TOKEN" sh'
```

Pin a release or branch:

```bash
curl -fsSL https://raw.githubusercontent.com/RoTorEx/hop/main/scripts/install.sh | sh -s -- --ref vX.Y.Z
```

The installer builds with Cargo, copies the binary to
`~/.toolbelt/hop/bin/hop`, writes the shell bridge to
`~/.toolbelt/hop/init.zsh`, stores a supplied private repo update token at
`~/.toolbelt/hop/gh-token` with file mode `0600`, and adds one plain line to the
active bash/zsh profile:

```bash
source "$HOME/.toolbelt/hop/init.zsh"
```

The bridge is required because a child process cannot change its parent shell's
working directory. It adds `~/.toolbelt/hop/bin` to PATH without duplicates,
delegates all argument parsing to the Rust CLI, uses the absolute installed
binary path, validates the selected directory, and contains the only `cd`
needed by the integration. `hop update` refreshes both the binary and this
bridge.

Open a new shell or source your profile, then run:

```bash
hop
hop ~
hop A1
hop b1
hop --copy-path A1
```

## Windows

The [latest GitHub release](https://github.com/RoTorEx/hop/releases/latest) includes the `hop-windows-x86_64.zip` archive for 64-bit Windows. The archive contains `hop.exe`. Run the following in Windows PowerShell 5.1 or PowerShell 7.

```powershell
$hopBin = Join-Path $env:USERPROFILE '.toolbelt\hop\bin'
$hopArchive = Join-Path ([IO.Path]::GetTempPath()) ('hop-' + [guid]::NewGuid() + '.zip')
$hopUrl = 'https://github.com/RoTorEx/hop/releases/latest/download/hop-windows-x86_64.zip'
New-Item -ItemType Directory -Path $hopBin -Force | Out-Null
try {
    Invoke-WebRequest -UseBasicParsing -Uri $hopUrl -OutFile $hopArchive
    Expand-Archive -LiteralPath $hopArchive -DestinationPath $hopBin -Force
} finally {
    if (Test-Path -LiteralPath $hopArchive) { Remove-Item -LiteralPath $hopArchive }
}
```

Save this bridge in `$PROFILE.CurrentUserAllHosts` and open a new PowerShell window. Run `hop config` to set up an empty installation or write the [version 5 routing files](#scoped-configuration) to register specific projects.

```powershell
$hopExe = Join-Path $env:USERPROFILE '.toolbelt\hop\bin\hop.exe'
if (Test-Path -LiteralPath $hopExe) {
    & $hopExe --shell-init | Out-String | Invoke-Expression
}
```

Run `hop` for the selector, `hop list` to preview it, or `hop <project-name>` to change directory directly. Configuration and history live in `%USERPROFILE%\.toolbelt\hop`. Copy mode uses `Set-Clipboard`. To update, close running `hop.exe` processes and repeat the ZIP installation.

Without saved routing, Windows can scan ready local fixed disks. Use `hop scan --root C:/Coding` to check one workspace and `hop scan` for the configured scope. Discovery skips system and application directories, build outputs, links and inaccessible subfolders.

To build from source on Windows, install Rust with the MSVC toolchain and
Visual Studio C++ Build Tools. Run the native build and the PowerShell smoke test.

```powershell
$env:CARGO_TARGET_DIR = Join-Path $env:USERPROFILE 'construction_side\hop\target'
cargo test --locked --all-targets
cargo build --release --locked
.\scripts\test-windows.ps1 -Binary "$env:CARGO_TARGET_DIR\release\hop.exe"
```

Linux users can install from source with the Unix installer.

## Usage

```bash
hop --help
hop ~
hop --scope work
hop A1 --scope work
hop list --scope work
hop scan
hop config
hop -v
hop A1
hop --copy-path A1
hop list
hop list --frequent
hop stats
hop --frequent
hop --frequent 1
hop update
hop config --root /srv
```

Interactive UI, help, and version output are written to stderr. The installed
shell integration makes `hop` change the current shell directory in jump
mode. Sector labels are case-insensitive, so `hop B1` and `hop b1` are
equivalent. The underlying binary prints the selected path as its only stdout
output, which keeps shell integration safe and predictable. Copy mode writes no
stdout and copies the selected path with `pbcopy`, `wl-copy`, `xclip`, or `xsel`.

`hop list` prints the same grouped project view as `hop` and exits without an
interactive prompt. Add `--frequent` to print the same ranked view as
`hop --frequent`. List mode does not select a project or change directories.

`hop ~` jumps directly to the hop home directory, `~/.toolbelt/hop`.

`hop --frequent` shows all available configured projects in one list, ordered by the number
of successful jumps. This view uses numeric selectors, so `hop --frequent 1`
jumps to the current most-used project. Equal counts fall back to alphanumeric
path order. Copying a path and failed directory changes do not increase the
count.

Jump counts are kept locally in `~/.toolbelt/hop/history.toml`, separately from
the manually editable project config. The installed shell bridge records a jump
only after it changes directory successfully. `hop stats` prints the total and
per-destination counts, including past destinations that are no longer available.

Every destination has a name, and `hop <name>` jumps to it from any scope:
`hop voxray`, or `hop Aveni/ai-workspace` when a folder name is shared. A script
or an agent resolves a project the same way without the shell bridge:
`~/.toolbelt/hop/bin/hop voxray` prints the path. The lookup keeps working after
the project moves, because the name does not change. Text shaped like a selector
(`A1`) is always a selector.

A count belongs to the destination's name; the path shown beside it is the
registered location, or the last known one for a retired destination. The name is the destination's folder name. Destinations
that share a folder name are told apart by the shortest trailing path, such as
`Aveni/ai-workspace` and `Thira/ai-workspace`. Moving a project and updating
its registered path therefore keeps its count and its place in the frequent
view. Renaming the folder starts a new count. Path-keyed history written by
earlier versions is converted on the next recorded jump; counts of old paths
that spell a registered name are added to that destination.

It also shows average interactive search time and sample counts by view and
outcome for the retained local log. Search time runs from the selection prompt
to a valid selection or cancellation, including time spent on invalid attempts.
Direct selectors, piped input, and copy mode are excluded.

Hop defaults to Personal, regardless of the current directory. Use
`hop --scope work` to choose a Work destination, `hop A1 --scope work` to jump
directly, and `hop list --scope work` to preview the view. `--scope archive`
selects retired material the same way.
Selectors and the frequent view use only the selected scope. Navigation loads
the saved config and checks registered paths without recursively scanning.

### Scoped configuration

Save Personal destinations in `~/.toolbelt/hop/routing/personal.toml` and Work
destinations in `~/.toolbelt/hop/routing/work.toml`. Both files use version 5.
Use a file containing only `version = 5` for an empty scope. The filename
determines scope. An optional `routing/archive.toml` in the same format defines
the Archive scope for retired material; without the file that scope is empty. Shared scan settings live in the optional `config.toml`
under the Hop home directory.

Each group is one flat section. Its root labels the group and anchors its
relative items. Set `jump_root = true` to make the root itself a destination
through selector `0`. A root can be the only destination in a section and can
be a directory without `.git`. Items must contain `.git` to appear.

`routing/personal.toml`:

```toml
version = 5

[[sections]]
root = "/home/alex/dev-tools"
items = ["hop", "shelve"]

[[sections]]
root = "/home/alex/dev-tools/build-service"
jump_root = true
items = ["api", "worker"]
```

`routing/work.toml`:

```toml
version = 5

[[sections]]
root = "/home/alex/corporate/company"
jump_root = true
items = ["first-repo", "second-repo"]
```

In this example, Build Service appears
only in its own section. Put a project or its components in one section. Hop
rejects duplicate destinations and parent/child destinations assigned to
different sections. An ordinary section root can contain other section roots
on disk because users jump only to its listed items.

### Register or move a project

1. Open the routing file for the scope. `hop config` validates the routing
   files and prints their location.
2. Add a repository path relative to the chosen section's `root` in its
   `items`. To register a separate project group, add one section with its
   root and any component paths. If a destination moves between sections or
   scopes, move its registration between the appropriate files. If a whole group moves
   together, change its `root` and keep the relative items.
3. Run `hop list` or `hop list --scope work` to verify the destination and its
   selector. Selectors are positions in the active view.

`hop scan` is optional for finding unregistered checkouts and unavailable
entries, for example after moving several repositories. It compares Git
repositories under a filesystem root against every scope, reports the
differences, and leaves the config unchanged. Git metadata can show that a
checkout exists and where it lives. The user chooses where it belongs.
Use `hop scan --root <dir>` to narrow the walk. Otherwise it uses the saved scan
root, the home directory on Linux and macOS when no root is saved, or ready
local fixed drives on Windows. The report respects the top-level `excluded`
list of absolute paths.

When the routing directory is absent, Hop reads an existing `config.toml`
as before. Without routing files, `hop config` retains bootstrap and refresh
for version 1–4 configs. To migrate, split sections between the two routing
files, consolidate overlapping destinations, and keep shared discovery
settings in `config.toml`. Routing files take precedence for destinations.

`hop update` replaces `~/.toolbelt/hop/bin/hop` and refreshes `init.zsh`
from the latest Linux or macOS release for the current CPU architecture. It
requires `curl` or `wget`, plus `tar`. If `~/.toolbelt/hop/gh-token` exists,
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

Pushing a `vX.Y.Z` tag builds and attaches Windows x86_64 and macOS x86_64
and aarch64 release binaries. Pushes to `main`, pull requests, and manual
workflow runs build the same artifacts for verification. Each target runs
native Rust tests. Windows also checks scoped navigation, directory changes
and history in Windows PowerShell 5.1 and PowerShell 7.

## Kernel sync (sanity check)

```bash
make vibe-kernel-set
make vibe-pull
```

`make vibe-kernel-set` prompts for the parent kernel path and updates `.vibe/KERNEL_SOURCE` (local-only; gitignored).

Confirm these exist after pulling:

- `.vibe/kernel/PRINCIPLES.md`
- `.githooks/pre-commit`
- `CHANGELOG.md`
- `AGENTS.md` contains the `VIBE:KERNEL_ROUTING` markers

## Docs map

- Keep the repo root minimal. Prefer putting project docs under `docs/` rather than adding many root markdown files.
- `AGENTS.md` — agent router.
- `CHANGELOG.md` — release progress (if this project releases).
- `.vibe/kernel/*.md` — local copies of Vibecoding Kernel instructions (do not edit).
- `.githooks/` — optional git hooks managed by the kernel (lint gates).
- `BUSINESS.md` and `business/` — product purpose and navigation rules.
- `docs/architecture/` — design truth (agents choose scope; keep schemas/diagrams/boundaries up to date).
- `docs/contracts/` — stable contracts.
- `docs/features/` — accepted feature notes.
- `docs/ideas/` — raw ideas, not roadmap.
- `docs/reports/` — reports and audits (read only when relevant).

## Commands

See `Makefile`.

## Local usage logs

See [usage logging](docs/usage-logging.md) for product events, privacy, paths,
rotation and interpretation limits.
