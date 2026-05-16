# AI Workflow

## Default cycle

1. Inspect the smallest relevant context.
2. State a short plan only when the task is non-trivial.
3. Make minimal, reversible changes.
4. Run the relevant checks.
5. Update docs only when durable behavior, commands, architecture, or workflow changed.
6. Commit and push `main` to `origin` unless the user says otherwise.
7. Report changed files and checks; include risks/docs/parent proposals only when relevant.

## Task queue (optional)

If the repo uses a root task file (`TASK.md`), treat it as the user-owned task queue:

- Read it at the start of the session when the user says “follow TASK.md” (do not assume it exists).
- Process tasks **in order**, one by one.
- When a task is completed, remove its section from `TASK.md` (keep the file as the current remaining queue).
- Do not turn `TASK.md` into a backlog or roadmap; move ideas to `docs/ideas/` instead.

## Agent rules

- Do not perform unrelated refactors.
- Do not invent new architecture unless the task requires it.
- Prefer existing project patterns.
- Ask for approval before weakening boundaries or changing parent rules.
- Ask for explicit approval before creating tags, publishing artifacts, or performing a release.
- If a project has a release bump command that supports `patch`/`minor`/`major`, interpret approvals as:
  - “release approved” with no bump-level specified => default to `patch`
  - `minor`/`major` approval is one-shot (applies only to the next release); subsequent releases default back to `patch` unless the user explicitly requests `minor`/`major` again
- Always commit and push to `origin` at the end of a directive unless the user says otherwise.
- Default branch policy: assume the repo uses `main`. If the repo clearly uses a different default branch (e.g., `master`), ask before pushing.
- After any commit: push to the default branch (`git push origin main`) unless the user says otherwise.
- After creating a release tag: push commits and tags (`git push origin main --follow-tags`) unless the user says otherwise.
- Never use history-rewriting or destructive Git commands by default:
  - forbidden: `git push --force`, `git push --force-with-lease`
  - forbidden: `git reset --hard` (use `git revert` or a new corrective commit instead)
- Do not touch files outside the task scope; treat unexpected/untracked files as suspicious and report them, but do not delete them or include them in commits unless explicitly requested.
- Do not create speculative future features in stable docs.
- Put raw ideas into `docs/ideas/`.
- Keep output compact by default: do not restate the prompt; prefer 1 outcome sentence + a small list of facts (changed files, checks, next step).
- Expand only for architecture, security, migration, release, or high-risk changes.
