# Navigation and Ranking

## Scopes and discovery

Personal, Work, and Archive are explicit saved scopes. Personal is the default
from any working directory. The `--scope` flag selects one scope for jumps and
lists, including the frequent view. Archive holds retired material that should
stay reachable without appearing in the daily views; its routing file is
optional, and a missing file means an empty scope.

The version 5 files under `routing/` record flat sections with a root,
relative items, and an optional `jump_root = true`. The filename determines
scope. Items are available when they contain `.git`. A section root with
`jump_root` is available when the directory exists. Other section roots only
organize destinations. A destination can appear
once across all sections and scopes. A destination's ancestor and descendant
may coexist in one section, for example a service group root and its components.
Hop rejects that overlap across sections, including resolved aliases for
available paths. A folder may physically contain different groups when the
containing section does not register the nested group's path or its ancestor.

`hop scan` finds repositories recursively, reports progress, and leaves the
config unchanged. It reports a repository under a registered destination as
inside that destination. Other unregistered repositories are candidates rather
than immediate menu entries. A moved project appears as an unavailable saved
path and a newly discovered checkout until someone updates its registered
location. Hop cannot infer a move from two paths alone.

Scanning uses an explicit `--root`, the saved scope, the user home on Unix, or
all ready local fixed drives on Windows. It skips symlinks and known hidden,
system, cache, and build directories, and skips inaccessible subdirectories.
Discovery finds nested Git repositories.

When routing files are present, `config.toml` supplies shared scan settings,
and `hop config` validates both routing files and displays their directory.
When the routing directory is absent, Hop reads the single `config.toml`.
Version 1–4 configs are Personal and retain legacy bootstrap and refresh
through `hop config`. A single version 5 config may specify scope per section.

## Default view

Version 5 shows available items and sections with a configured, available
root jump. Unavailable destinations remain in the config for recovery.
Legacy version 4 sections require at least one available Git project.
Labels are assigned consecutively after filtering, so hidden sections occupy no selectors.
Sectors are sorted alphanumerically by their
full root paths, and items within each sector by their relative paths, including
all descendant components. Both config output and navigation apply these sorts
even if the input config is manually reordered. Numeric runs use natural order
(for example, `project-2` precedes `project-10`). Sectors are labeled `A`
through `Z`, then `AA`, `AB`, and so on. A selector combines the
sector label and one-based position, for example `B2`. Position `0` selects
the sector's exact root directory: `hop A0` or interactive `A0` jumps there,
and `hop --copy-path A0` copies it. Labels are case-insensitive and may contain
multiple letters, such as `AA0`. Project positions still start at `1`.
In version 5, root selection is available only with `jump_root = true`. The root need not contain `.git`;
the shell bridge validates the directory before changing into it.

In legacy configs, when a project also roots a visible child sector, that sector's implicit `0`
represents the project; omit its duplicate item in the parent sector. Resolve
deeper sectors first and hide parent sectors left with no items. If a child
sector has no available items, retain its Git root as a normal parent item so
the project remains reachable. Saved selections are preserved across refreshes.

`hop list` renders this same view and exits successfully without prompting for
a selection. It does not select a project or record a jump.

In both interactive views, Esc immediately cancels the selection, including
after partially entering a selector. Cancellation emits no selected path and
leaves the shell directory and jump history unchanged. Enter submits a selector;
empty Enter, Ctrl+C, and Ctrl+D also cancel. Backspace edits the selector and
Ctrl+U clears it. Terminal input settings are restored when the prompt ends.
Piped input keeps the existing line-based interface.

## Frequent view

`--frequent` presents one flat list because retaining directory sectors would
prevent a true global frequency ordering. Positions are one-based plain numbers
(`1`, `2`, and so on); sector letters are intentionally absent in this view.
`hop list --frequent` renders this same ranked view without prompting.

All available destinations in the selected scope remain visible. Destinations are ordered by descending
successful jump count. Equal counts are resolved by alphanumeric full-path
order, which makes a new or empty history deterministic. Counts are unsigned
64-bit values and saturate at their maximum instead of wrapping.

## Counting and persistence

The installed shell bridge increments a project only after `cd` succeeds. It
does not increment for `--copy-path`, raw executable selection without shell
integration, failed directory changes, or `hop ~`.

Counts are stored separately from the user-edited project config in
the active Hop home's `history.toml`. The file is local state, is written with owner-only
permissions on Unix, and is replaced atomically so an interrupted write does
not leave a partial history. Missing history means every project has zero
jumps. Invalid or unsupported history is reported rather than overwritten.

Unconfigured or currently missing projects may retain history but are omitted from
the ranking. If configured and present again, their prior count applies.
`hop config` never rewrites jump history.

History version 2 keys each count by destination name and stores a path as an
attribute: the registered location, or the last known one once the destination
is no longer registered. The owner reorganizes project folders
often, and path-keyed counts split one project across every place it had lived.

- The name is the destination's folder name. Destinations sharing a folder name
  use the shortest trailing path that distinguishes them, for example
  `Aveni/ai-workspace`. Names are derived from all registered destinations in
  every scope, whether or not they currently exist, so they are not a second
  list to maintain and do not change with mounts or the active view.
- Moving a project and updating its registered path keeps the count. Renaming
  the folder starts a new count.
- When a second destination with the same folder name is registered, both
  names grow a parent folder; an existing count follows through its recorded
  path while that path stays registered.
- Version 1 history is read as before and rewritten as version 2 by the next
  recorded jump. An old path adopts the registered name spelled by its trailing
  folders, so counts from earlier layouts merge into the current destination;
  other old paths keep a count under their folder name.

A destination name is also a target: `hop <name>` selects the destination in
any scope, exactly or by one case-insensitive match. Instructions and scripts
can therefore refer to a project by name and stay valid when it moves. Text
shaped like a selector is never treated as a name, and an all-digit target keeps
its meaning as a position in the frequent view.

The shell bridge records the exact path emitted by Hop. Paths are not
canonicalized, so a symlinked spelling of an unregistered path is named by its
own folder.

## Usage statistics

`hop stats` reports the total and, for each destination, the count, name and
last path from jump history, including destinations absent from the current
configuration or filesystem. Concurrent shell jumps serialize history updates.

Interactive search time measures the interval from the first selection prompt to
a valid selection or cancellation. Invalid selections keep the same timer
running. Default and frequent views are reported separately, as are valid
selections and cancellations. Direct selectors, piped input and copy mode are
excluded. A valid selection can be timed even if the shell never completes the
jump. `hop stats` shows the sample count and mean duration for retained events.

## Protecting tests

- `src/lib.rs`: history round-trip, increment behavior, saturating counters,
  frequency/path ordering, destination naming, and version 1 conversion.
- `tests/navigation.rs`: the optional Archive scope and a count that follows a
  moved destination.
- `src/main.rs`: frequent CLI parsing, numeric selection, and post-`cd` shell
  recording.

Discovery tests cover multi-root collection and deduplication, migration of
inactive choices, legacy/current config parsing, explicit ordering and path containment,
Windows disk enumeration, and case-insensitive system/build folder exclusions.
