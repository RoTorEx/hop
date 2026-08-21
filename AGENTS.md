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

This project uses committed local copies of the Vibecoding Kernel.

- Always read `.vibe/kernel/OPERATING.md` before normal project work.
- For version, tag, publish, or release work, also read
  `.vibe/kernel/RELEASE.md`.
- For user corrections, instruction conflicts, reusable agent-workflow lessons,
  kernel proposals, or a newly pulled kernel version, also read
  `.vibe/kernel/EVOLUTION.md`.
- For product behavior or domain logic, read `BUSINESS.md` and only the relevant
  module from `business/*`.
- Do not edit `.vibe/kernel/*` manually or read the parent kernel during normal
  work. Refresh the local copy with `make vibe-pull`.

<!-- VIBE:KERNEL_ROUTING_END -->
