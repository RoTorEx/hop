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

Pin a release or branch:

```bash
curl -fsSL https://raw.githubusercontent.com/RoTorEx/jumper/main/scripts/install.sh | sh -s -- --ref vX.Y.Z
```

The installer builds with Cargo, copies the binary to
`~/.x-cli-jumper/jumper`, and adds this integration to bash/zsh profile files:

```bash
export PATH="$HOME/.x-cli-jumper:$PATH"

j() {
    local arg
    for arg in "$@"; do
        case "$arg" in
            -h|--help|-v|-V|--version|--shell-init|update)
                jumper "$@"
                return
                ;;
        esac
    done

    local d
    d="$(jumper "$@")" && [ -n "$d" ] && cd "$d"
}
```

Open a new shell or source your profile, then run:

```bash
j
j A1
j --copy-path A1
```

## Usage

```bash
jumper --help
jumper -v
jumper A1
jumper --copy-path A1
jumper update
jumper --root /srv
jumper --shell-init
```

Interactive UI, help, and version output are written to stderr. Jump mode prints
the selected path as the only stdout output, which keeps shell integration safe
and predictable. Copy mode writes no stdout and copies the selected path with
`pbcopy`, `wl-copy`, `xclip`, or `xsel`.

`jumper update` replaces the current executable with the latest Linux binary for
the current CPU architecture from GitHub Releases. It requires `curl` or `wget`,
plus `tar`.

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
attaches Linux x86_64 and aarch64 release binaries.

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
