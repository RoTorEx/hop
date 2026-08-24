# Navigation and Ranking

## Default view

The default view groups active projects by parent directory. Sectors are ordered
by their full parent paths and labeled `A` through `Z`, then `AA`, `AB`, and so
on. Projects inside a sector are ordered by path. A selector combines the
sector label and one-based position, for example `B2`.

`hop list` renders this same view and exits successfully without prompting for
a selection. It does not select a project or record a jump.

## Frequent view

`--frequent` presents one flat list because retaining directory sectors would
prevent a true global frequency ordering. Positions are one-based plain numbers
(`1`, `2`, and so on); sector letters are intentionally absent in this view.
`hop list --frequent` renders this same ranked view without prompting.

All active projects remain visible. Projects are ordered by descending
successful jump count. Equal counts are resolved by alphanumeric full-path
order, which makes a new or empty history deterministic. Counts are unsigned
64-bit values and saturate at their maximum instead of wrapping.

## Counting and persistence

The installed shell bridge increments a project only after `cd` succeeds. It
does not increment for `--copy-path`, raw executable selection without shell
integration, failed directory changes, or `hop ~`.

Counts are stored separately from the user-edited project config in
`~/.x-cli-hop/history.toml`. The file is local state, is written with owner-only
permissions on Unix, and is replaced atomically so an interrupted write does
not leave a partial history. Missing history means every project has zero
jumps. Invalid or unsupported history is reported rather than overwritten.

Inactive or currently missing projects may retain history but are omitted from
the ranking. If they become active and present again, their prior count applies.
`hop config` never rewrites jump history.

The shell bridge records the exact path emitted by Hop. History is not
canonicalized or merged across symlinked or differently spelled paths.

## Protecting tests

- `src/lib.rs`: history round-trip, increment behavior, saturating counters,
  and frequency/path ordering.
- `src/main.rs`: frequent CLI parsing, numeric selection, and post-`cd` shell
  recording.
