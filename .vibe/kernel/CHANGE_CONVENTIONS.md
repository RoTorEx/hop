# Change Conventions

## Commits

Use compact conventional commits:

```text
<type>(optional-scope): imperative summary
```

Common types:

- `feat`
- `fix`
- `docs`
- `refactor`
- `test`
- `build`
- `chore`
- `ui`
- `style`

## Atomicity

- One completed slice per commit.
- Do not mix unrelated changes.
- Keep release/version bump commits separate when possible.

## Changelog (if present)

If the repo uses `CHANGELOG.md`:

- Keep new notable changes under `## [Unreleased]` as they land.
- On a release bump, move the Unreleased entries under a new version header like:
  - `## [X.Y.Z] - YYYY-MM-DD`
  leaving `## [Unreleased]` empty for new work.
- Update the changelog deterministically (no speculation; only what actually shipped).

## Tags and releases

Default tag format:

```text
vX.Y.Z
```

Use release/changelog flow only when the project actually releases.

Recommended flow (language-agnostic; Makefile wraps native tooling):

1. Commit all normal work first (clean working tree).
2. Run the repo release bump command (must update version + changelog/release notes deterministically):
   - `make release-bump` (defaults to `patch` unless user explicitly requests `minor`/`major`)
3. Commit the bump as a dedicated release commit (keep it separate from feature work):
   - `build: bump version to vX.Y.Z` (or equivalent)
4. Tag the release commit:
   - `make release-tag` (creates `vX.Y.Z`)
5. Push the release commit and tag:
   - `git push origin main --follow-tags`
6. Publish/deploy only if the project actually publishes:
   - `make release-publish`

Notes:
- Ask for explicit user approval before any version bump, tagging, or publishing.
- Never use destructive Git overrides: no `git push --force*`, no `git reset --hard`.
- If the repo supports `patch`/`minor`/`major`, treat `minor`/`major` approvals as one-shot; subsequent releases default back to `patch` unless requested again.

## Reports

For agent work, prefer a minimal report:

- Changed
- Checks

Include only when relevant:

- Risks
- Docs (only if durable docs changed)
- Parent proposal

Do not create giant reports for small changes.
