# Context Routing

Agents should not read the whole project by default.

## AGENTS.md posture

Keep `AGENTS.md` a **router**, not an encyclopedia.

- Put durable, always-applicable rules in `AGENTS.md`.
- Move task-specific instructions into topic docs only when it reduces `AGENTS.md` bloat or clarifies ownership.
- Topic docs are **conditional reads**. Do not instruct agents to read them for every task.

## Always read

1. `AGENTS.md`
2. `.vibe/kernel/PRINCIPLES.md`
3. `.vibe/kernel/AI_WORKFLOW.md`
4. `.vibe/kernel/CONTEXT_ROUTING.md`

## Conditional reads

- Code task: relevant source files and relevant tests.
- UI/UX task: `docs/architecture/*`, `docs/features/*`, relevant UI source.
- Architecture/refactor task: `docs/architecture/*`, relevant source tree.
- Runtime/deploy/env task: `docs/operations.md` (if present), plus relevant config and scripts.
- Task queue task: `TASK.md` (if present).
- Docs task: exact docs being changed.
- Release/version task: `Makefile`, `CHANGELOG.md` (if present), `docs/release.md` (if present), native package config, release notes, `.vibe/kernel/CHANGE_CONVENTIONS.md`.
- Logging/diagnostics task: `docs/logging.md` (if present), plus relevant source/config.
- Test/lint/check task: `docs/testing.md` (if present), plus relevant test/tool config.
- Tooling task: `Makefile`, package scripts, tool configs.
- Parent update task: evidence from exact project paths, then append a proposal to `<KERNEL_SOURCE>/PROPOSALS.md` (parent path is in `.vibe/KERNEL_SOURCE`).

## Avoid by default

- full kernel docs
- full project docs tree
- old reports
- backlog ideas
- unrelated source directories

## Rule priority

1. Current user instruction
2. Hard security/boundary constraints
3. Project truth docs
4. `.vibe/kernel/*.md`
5. General best practices
