# Hop Business Truth

Hop is a local-first project navigator for developers who move between Git
working directories from bash or zsh. It discovers projects, lets the user hide
irrelevant entries, presents selectable views, and delegates the final directory
change to its shell integration.

## Actors and concepts

- The user owns the project configuration and all locally recorded jump data.
- A project is a discovered directory containing `.git`.
- An active project is eligible for normal and frequent navigation views.
- A sector groups projects that share a parent directory.
- A successful jump is a directory change completed by the installed shell
  integration, not merely a selection or copied path.

## Core flows and invariants

- `hop config` refreshes discovered projects while preserving the user's
  `active` choices for paths that still exist.
- The default view groups active projects into sectors and addresses them with
  selectors such as `A1`.
- The frequent view globally ranks the same active projects by successful jump
  count and addresses them with numeric positions such as `1`.
- UI and diagnostics go to stderr. A selected path is the only stdout emitted
  in jump mode, so shell integration remains safe.
- Configuration, jump history, and navigation stay local. Hop sends no
  telemetry.
- Copying a path, selecting a path without an active shell integration, a
  failed `cd`, and the `hop ~` shortcut do not count as project jumps.

## Non-goals

- Hop does not synchronize configuration or history between machines.
- Frequent ranking is not a recommendation engine and does not use recency,
  decay, or remote activity.
- Selectors are positions in the current view, not permanent project IDs.

## Detailed truth map

- [Navigation and ranking](business/navigation.md) — views, selector semantics,
  counting, persistence, and ordering.
- `src/lib.rs` — discovery, configuration, history, grouping, and ranking.
- `src/main.rs` — CLI routing, terminal rendering, and shell integration.
- `docs/architecture/overview.md` — technical flow and system boundaries.
