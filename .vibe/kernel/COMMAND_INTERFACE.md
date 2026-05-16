# Command Interface

The kernel defines command names and semantics, not implementation.

Native tool configuration remains local:

- `package.json`
- `pyproject.toml`
- `Cargo.toml`
- Docker files
- CI files

## Recommended targets

Keep the Makefile command surface stable and predictable across child projects.

Canonical target set (keep the same names, semantics, and *rough* order; omit only when truly not applicable):

1. `make install` — install dependencies
2. `make deps-update` — update dependencies (only if a real update flow exists)
3. `make build` — build artifact
4. `make watch` — development watch mode
5. `make typecheck` — type checking
6. `make lint` — linting if real lint exists
7. `make fmt` — formatting if real formatter exists
8. `make test` — tests if real tests exist
9. `make check` — main verification command (the “verify this repo” entrypoint)
10. `make run` — local run
11. `make release-bump` — bump version if release flow exists
12. `make release-tag` — create release tag if release flow exists
13. `make release-publish` — publish/deploy if release flow exists
14. `make vibe-kernel-path` — print the current kernel source path (`.vibe/KERNEL_SOURCE`)
15. `make vibe-kernel-set` — set/update `.vibe/KERNEL_SOURCE` (interactive prompt, or pass `KERNEL=/abs/path/to/vibecoding-kernel`)
16. `make vibe-pull` — refresh `.vibe/kernel/*.md` from the parent kernel

## Rules

- Do not create fake passing targets.
- Treat `Makefile` as the repo’s **service command surface**: routine actions (install, lint, fmt, test, build, run, dependency updates) should be runnable via `make ...`.
- Prefer invoking `make <target>` in docs/instructions instead of raw tool commands (`npm`, `uv`, `cargo`, etc.). Make wraps the native tooling for the current repo.
- Do not force release targets on projects that do not release.
- Do not perform version bumps, tags, publishing, or deployments without explicit user approval.
- When release targets exist, use the Makefile release interface (`make release-bump`, `make release-tag`, `make release-publish`) instead of ad-hoc commands.
- `make check` should be the stable “verify this repo” command.
- Makefile should wrap native tools, not replace them.

## Git hook gates (optional)

The kernel can provide optional lint gates via git hooks.

`make vibe-pull` installs/updates kernel-managed hook scripts under `.githooks/` without deleting other hook files:

- it only overwrites hook scripts that contain the `VIBE:KERNEL_MANAGED_HOOK` sentinel;
- otherwise it leaves existing hook scripts unchanged and prints a warning.

To avoid breaking existing hook setups, `make vibe-pull` enables hooks in an append-only way:

- if `core.hooksPath` is unset and the repo does not already have non-sample hooks in `.git/hooks`, it sets `core.hooksPath` to `.githooks`;
- in all other cases it leaves git hook configuration unchanged and prints a warning.

By default the kernel-managed lint gate runs `make lint` on `git commit` and `git push` (set `VIBE_SKIP_LINT_HOOKS=1` to bypass in an emergency).

If `make lint` is still a skeleton placeholder (prints `TODO: implement lint only if real lint exists.`), the hook will **not** block commits/pushes.
