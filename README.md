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

## Other platforms and previous Windows releases

CI and future release assets target macOS only (Apple Silicon and Intel), using
GitHub-hosted macOS runners. Linux/Windows source and existing release assets are
retained, but no new Linux/Windows binaries are built. Linux users must update
from source with the installer; `hop update` expects a release asset and will not
find one for Linux in future macOS-only releases. The Windows download commands
below apply only to older tags containing a Windows asset; use a specific such
tag instead of `latest`, or build from source.

Earlier releases include `hop-windows-x86_64.zip` containing `hop.exe`. Download it
from [GitHub Releases](https://github.com/RoTorEx/hop/releases/latest), or run
these commands in PowerShell once a Windows release is published:

```powershell
$hopBin = Join-Path $env:USERPROFILE '.toolbelt/hop\bin'
New-Item -ItemType Directory -Path $hopBin -Force | Out-Null
$hopArchive = Join-Path $env:TEMP 'hop-windows-x86_64.zip'
Invoke-WebRequest 'https://github.com/RoTorEx/hop/releases/download/v0.3.19/hop-windows-x86_64.zip' -OutFile $hopArchive
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
& "$env:USERPROFILE\.toolbelt/hop\bin\hop.exe" --shell-init | Out-String | Invoke-Expression
```

On Windows, plain `hop config` automatically scans every ready local fixed disk
(`C:\`, `D:\`, and so on) and combines the projects into one list. There is no
need to enter drive letters. It shows which disk is being scanned and skips
system/application folders (`Windows`, `Program Files`, `ProgramData`, `AppData`,
Recycle Bin, recovery data), build outputs, links, and inaccessible subfolders.
Mapped network drives, removable media, and disks that are not ready are excluded.
For a deliberately limited scan, `hop config --root D:\Projects` still works.

For every scan scope, ordinary `hop` uses the saved list and checks only whether
known projects are available. Recursive discovery runs only through `hop config`.
Existing version 1–3 configs remain readable and `hop config` migrates them to
explicit sections. Running `hop config` again scans for newly created projects;
add paths to `excluded` to keep deliberate omissions hidden.

Windows PowerShell 5.1 and PowerShell 7 are supported. Configuration and history
live under `%USERPROFILE%\.toolbelt/hop`; copy mode uses `Set-Clipboard`.
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
hop personal
hop work
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
It also shows average interactive search time and sample counts by view and
outcome for the retained local log. Search time runs from the selection prompt
to a valid selection or cancellation, including time spent on invalid attempts.
Direct selectors, piped input, and copy mode are excluded.

Hop defaults to Personal, regardless of the current directory. Use `hop work`
for Work, `hop work A1` for a direct jump, and `hop list work` to preview.
Selectors and the frequent view use only the selected scope. Navigation loads
the saved config and checks registered paths without recursively scanning.

### Scoped configuration

Use version 5 in `~/.toolbelt/hop/config.toml` to register groups explicitly.
Each group is one flat section. Its root labels the group and anchors its
relative items. Set `jump_root = true` to make the root itself a destination
through selector `0`. A root can be the only destination in a section and can
be a directory without `.git`. Items must contain `.git` to appear.

```toml
version = 5

[[sections]]
root = "/home/alex/dev-tools"
items = ["hop", "shelve"]

[[sections]]
root = "/home/alex/dev-tools/build-service"
jump_root = true
items = ["api", "worker"]

[[sections]]
root = "/home/alex/corporate/company"
scope = "work"
jump_root = true
items = ["first-repo", "second-repo"]
```

Sections default to `scope = "personal"`. In this example, Build Service appears
only in its own section. Put a project or its components in one section. Hop
rejects duplicate destinations and parent/child destinations assigned to
different sections. An ordinary section root can contain other section roots
on disk because users jump only to its listed items.

Run `hop scan` for a progress display and a report of new candidates, discovered
repositories inside registered destinations, and unavailable destinations in
the scanned area. It uses the saved scan root, the home directory on Linux and
macOS when no scope is saved, or ready local fixed drives on Windows. Use
`hop scan --root <dir>` to choose a directory explicitly. The scan leaves
registration to the user and respects absolute paths in the top-level
`excluded` list.

`hop config` points to the editable configuration for version 5. With a missing
config or a version 1–4 config, it retains the legacy bootstrap and refresh
workflow and groups discovered repositories by immediate parent. To migrate an
existing config, back it up, change `version` to `5`, mark the work sections,
and consolidate any destinations that overlap. Add `jump_root = true` only
where a group root should be a destination. Verify with `hop list personal`
and `hop list work`. Use a build that supports version 5 with the new file.

Normal `hop` jump mode requires the config file. If it is missing, hop
prints an alert and exits; run `hop config` first.

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

## Local usage logs

See [usage logging](docs/usage-logging.md) for product events, privacy, paths,
rotation and interpretation limits.
