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
`~/.x-cli-hop/bin/hop`, writes the shell bridge to
`~/.x-cli-hop/init.zsh`, stores a supplied private repo update token at
`~/.x-cli-hop/gh-token` with file mode `0600`, and adds one plain line to the
active bash/zsh profile:

```bash
source "$HOME/.x-cli-hop/init.zsh"
```

The bridge is required because a child process cannot change its parent shell's
working directory. It adds `~/.x-cli-hop/bin` to PATH without duplicates,
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

## Windows (x64)

Releases include `hop-windows-x86_64.zip` containing `hop.exe`. Download it
from [GitHub Releases](https://github.com/RoTorEx/hop/releases/latest), or run
these commands in PowerShell once a Windows release is published:

```powershell
$hopBin = Join-Path $env:USERPROFILE '.x-cli-hop\bin'
New-Item -ItemType Directory -Path $hopBin -Force | Out-Null
$hopArchive = Join-Path $env:TEMP 'hop-windows-x86_64.zip'
Invoke-WebRequest 'https://github.com/RoTorEx/hop/releases/latest/download/hop-windows-x86_64.zip' -OutFile $hopArchive
Expand-Archive -LiteralPath $hopArchive -DestinationPath $hopBin -Force
Remove-Item -LiteralPath $hopArchive
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
& "$hopBin\hop.exe" --shell-init | Out-String | Invoke-Expression
hop config
hop
```

To enable directory changes in every PowerShell session, add these lines to
`$PROFILE` (create the profile if needed):

```powershell
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
& "$env:USERPROFILE\.x-cli-hop\bin\hop.exe" --shell-init | Out-String | Invoke-Expression
```

On Windows, plain `hop config` automatically scans every ready local fixed disk
(`C:\`, `D:\`, and so on) and combines the projects into one list. There is no
need to enter drive letters. It shows which disk is being scanned and skips
system/application folders (`Windows`, `Program Files`, `ProgramData`, `AppData`,
Recycle Bin, recovery data), build outputs, links, and inaccessible subfolders.
Mapped network drives, removable media, and disks that are not ready are excluded.
For a deliberately limited scan, `hop config --root D:\Projects` still works.

After an all-disk scan, ordinary `hop` uses the saved list and checks only whether
known projects are available. It does not scan all disks again on every jump.
Run `hop config` after adding projects or connecting another local disk. Existing
configs remain readable; run plain `hop config` once to switch to all-disk discovery.

Windows PowerShell 5.1 and PowerShell 7 are supported. Configuration and history
live under `%USERPROFILE%\.x-cli-hop`; copy mode uses `Set-Clipboard`.
The raw executable also works from CMD, but directory changes require the
PowerShell bridge. To update on Windows, repeat the download/extract commands
when `hop.exe` has exited; `hop update` remains Linux/macOS-only.

To build from source on Windows, install Rust with the MSVC toolchain and
Visual Studio C++ Build Tools, then run in PowerShell:

```powershell
$env:CARGO_TARGET_DIR = Join-Path $env:USERPROFILE 'construction_side\hop\target'
cargo test --locked --all-targets
cargo build --release --locked
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
& "$env:CARGO_TARGET_DIR\release\hop.exe" --shell-init | Out-String | Invoke-Expression
```

## Usage

```bash
hop --help
hop ~
hop config
hop -v
hop A1
hop --copy-path A1
hop list
hop list --frequent
hop --frequent
hop --frequent 1
hop update
hop --root /srv
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

`hop ~` jumps directly to the hop home directory, `~/.x-cli-hop`.

`hop --frequent` shows all active projects in one list, ordered by the number
of successful jumps. This view uses numeric selectors, so `hop --frequent 1`
jumps to the current most-used project. Equal counts fall back to alphanumeric
path order. Copying a path and failed directory changes do not increase the
count.

Jump counts are kept locally in `~/.x-cli-hop/history.toml`, separately from
the manually editable project config. The installed shell bridge records a jump
only after it changes directory successfully.

`hop config` scans `$HOME` on Linux/macOS or all ready local fixed disks on
Windows, and creates or updates
`~/.x-cli-hop/config.toml`. The config records the scan scope, preserves
existing `active = true` or `active = false` values for projects that are still
present, adds newly discovered projects as `active = true`, and removes projects
that are no longer discovered under that root. Projects are written in
alphanumeric path order. Edit `active = false` to hide a project from normal
`hop` results. Pass `--root <dir>` to refresh from a different scan root.
Passing `--root` to normal jump mode still performs an ad hoc scan instead of
using the config.

For a single-root config, every non-config command checks whether the configured
project tree still matches the current folders. An all-disk config checks only
the availability of known projects to keep navigation fast. If projects were added or removed, hop prints a
stderr warning and keeps running; run `hop config` again to refresh the
config. If the config uses a custom scan root, the warning prints the matching
`hop config --root <dir>` command.

Normal `hop` jump mode requires the config file. If it is missing, hop
prints an alert and exits; run `hop config` first.

`hop update` replaces `~/.x-cli-hop/bin/hop` and refreshes `init.zsh`
from the latest Linux or macOS release for the current CPU architecture. It
requires `curl` or `wget`, plus `tar`. If `~/.x-cli-hop/gh-token` exists,
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
attaches Linux and macOS x86_64 and aarch64 release binaries, plus a Windows
x86_64 ZIP. Pushes to `main`, pull requests, and manual workflow runs build the
same artifacts without publishing a release. Native targets run Rust tests;
Windows also checks directory changes and history in PowerShell 5.1 and 7.

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
