# AGENTS.md

This project uses local copies of the Vibecoding Kernel instructions under `.vibe/kernel/`.

This file should be a router, not an encyclopedia.

Do not read the parent kernel repo outside this repository during normal work.

## Rule priority

1. Current user instruction
2. Hard safety/security/boundary constraints
3. Project truth docs
4. `.vibe/kernel/*.md`
5. General best practices

## Kernel updates

Do not edit `.vibe/kernel/*` manually.
Run `make vibe-pull` to refresh the copied kernel instructions.

Agents must not edit parent kernel files from this child project.

Exception:

- Agents may append proposals to parent `<KERNEL_SOURCE>/PROPOSALS.md`.
- To find the parent path, read `.vibe/KERNEL_SOURCE`.

## Kernel routing

<!-- VIBE:KERNEL_ROUTING_START -->

This project uses **local copies** of the Vibecoding Kernel instructions under `.vibe/kernel/`.

- Read the **full local files** (no summaries).
- Do **not** read the parent kernel repo outside this repository during normal work.
- Do **not** edit `.vibe/kernel/*` manually.
- If present, `.githooks/*` is managed by the kernel; refresh it via `make vibe-pull`.
- If a kernel rule should change, append a proposal to `<KERNEL_SOURCE>/PROPOSALS.md` (find the parent path in `.vibe/KERNEL_SOURCE`).
- If present, `TASK.md` is the repo task queue (process tasks in order; remove completed task sections).
- If present, `CHANGELOG.md` tracks release progress (update on releases, if the repo releases).

Routing:

- Required first read:
  - `.vibe/kernel/PRINCIPLES.md`
  - `.vibe/kernel/AI_WORKFLOW.md`
  - `.vibe/kernel/CONTEXT_ROUTING.md`

- Conditional reads:
  - `.vibe/kernel/SETUP.md` — when bootstrapping, standardizing, repairing, or auditing Vibecoding setup.
  - `.vibe/kernel/DOCS_CONVENTIONS.md` — when editing documentation.
  - `.vibe/kernel/COMMAND_INTERFACE.md` — when editing commands/scripts/Makefile/tooling.
  - `.vibe/kernel/examples/DIST_ARTIFACTS.md` — when editing generated `dist/` output, bundle/package artifacts, or distribution build paths.
  - `.vibe/kernel/examples/RUST_PROJECTS.md` — when editing Rust/Cargo commands, Cargo config, or build/check/test/lint behavior.
  - `.vibe/kernel/CHANGE_CONVENTIONS.md` — when preparing commits/reports/tags/releases.
  - `.vibe/kernel/SECURITY_BOUNDARIES.md` — when touching secrets/env/network/logging/storage/auth/deployment/safety.

- Other kernel files:
  - `.vibe/kernel/*.md` — only when `CONTEXT_ROUTING.md` routes you there or the task explicitly requires it.

<!-- VIBE:KERNEL_ROUTING_END -->
