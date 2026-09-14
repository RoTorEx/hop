# Navigation and Ranking

## Discovery scope

On Windows, `hop config` automatically enumerates ready fixed local drives and
searches each one for directories containing `.git`. Removable, network, and
unready drives are excluded; the user does not need to supply drive letters.
Discovery skips Windows system/application data folders, ordinary build/cache
folders, links, and inaccessible subdirectories. Linux/macOS keep the home
folder default. An explicit `--root` selects a single directory on any platform.

The config records either a single scan root or `scan_all_drives = true`.
Config format version 4 adds ordered `[[sections]]`, each with a `root` and a
required `items` array. Items are relative descendants at any depth. Versions
1–3 remain readable and `hop config` migrates their active paths. Once version
4 exists, `hop config` preserves it rather than re-adding omitted projects.
If drive enumeration or scanning a selected root fails, the config is not written.

For all-disk configs, ordinary commands check only configured availability.
Single-root version 4 configs also treat unlisted projects as intentional and
warn only about configured items that disappear.

## Default view

The default view uses explicit sections. Sectors and their items retain config
order and are labeled `A` through `Z`, then `AA`, `AB`, and so on. A selector combines the
sector label and one-based position, for example `B2`.

`hop list` renders this same view and exits successfully without prompting for
a selection. It does not select a project or record a jump.

## Frequent view

`--frequent` presents one flat list because retaining directory sectors would
prevent a true global frequency ordering. Positions are one-based plain numbers
(`1`, `2`, and so on); sector letters are intentionally absent in this view.
`hop list --frequent` renders this same ranked view without prompting.

All available configured projects remain visible. Projects are ordered by descending
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

Unconfigured or currently missing projects may retain history but are omitted from
the ranking. If configured and present again, their prior count applies.
`hop config` never rewrites jump history.

The shell bridge records the exact path emitted by Hop. History is not
canonicalized or merged across symlinked or differently spelled paths.

## Protecting tests

- `src/lib.rs`: history round-trip, increment behavior, saturating counters,
  and frequency/path ordering.
- `src/main.rs`: frequent CLI parsing, numeric selection, and post-`cd` shell
  recording.

Discovery tests cover multi-root collection and deduplication, migration of
inactive choices, legacy/current config parsing, explicit ordering and path containment,
Windows disk enumeration, and case-insensitive system/build folder exclusions.
