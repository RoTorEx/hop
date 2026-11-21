# Hop Business Truth

Hop is a local-first project navigator for developers who move between Git
working directories from bash, zsh, or Windows PowerShell. It discovers projects,
lets the user select relevant entries, presents selectable views, and delegates the final directory
change to its shell integration.

## Actors and concepts

- The user owns the project configuration and all locally recorded jump data.
- Discovery finds candidate Git repositories by their `.git` metadata.
- Users assign saved destinations to flat sections in Personal or Work. Each
  section can register relative items and optionally its root via selector `0`.
- A successful jump is a directory change completed by the installed shell
  integration, not merely a selection or copied path.

## Core flows and invariants

- Personal is the default scope. Jumps and lists accept `--scope personal`
  or `--scope work` to select saved sections explicitly. Navigation checks registered paths without recursive discovery.
- A destination has one section. A registered root or item and its descendants
  belong to the same section. Container folders may organize groups on disk.
- `hop scan` shows progress and reports candidates without assigning them to
  sections or replacing configured paths. `hop config` retains the legacy
  bootstrap and refresh workflow for version 1–4 configs.
- The default view sorts sections by full root path and their items by relative path, and addresses them with
  selectors such as `A1`.
- The frequent view globally ranks configured projects and section roots by successful jump
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

## Toolbelt installation

The default installation, configuration and jump history live together under
`~/.toolbelt/hop`. Discovery includes nested Git repositories and skips symbolic
links and dependency/build directories. Each discovered path is listed once.
The normal list keeps root selector `0` implicit; it adds no Root row.

## Local product usage evidence

Record meaningful workflow outcomes locally, with bounded storage and no source
content. [Usage logging](docs/usage-logging.md) owns the event and retention
contract. Successful API calls, prepared artifacts and user-reported outcomes
must not be presented as broader business success.
