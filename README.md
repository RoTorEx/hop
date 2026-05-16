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
    local d
    d="$(jumper "$@")" && [ -n "$d" ] && cd "$d"
}
```

Open a new shell or source your profile, then run:

```bash
j
```

## Usage

```bash
jumper --help
jumper --root /srv
jumper --shell-init
```

All interactive UI is written to stderr. The selected path is the only stdout
output, which keeps shell integration safe and predictable.

## Release Flow

Normal verification:

```bash
make check
```

One-command guarded release:

```bash
make release
```

That command runs checks, bumps version metadata, creates a dedicated release
commit, creates a `vX.Y.Z` tag, and pushes `main` with tags. It asks for an
explicit confirmation prompt before changing release state.

For the first release, when `Cargo.toml` already contains the version to tag:

```bash
make release BUMP=current
```

Prepare the same flow manually:

```bash
make release-bump
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "build: bump version to vX.Y.Z"
make release-tag
make release-publish
```

Use `BUMP=minor` or `BUMP=major` for larger version changes. Pushing a `vX.Y.Z`
tag triggers the GitHub Actions workflow that builds and attaches a Linux
release binary.

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
