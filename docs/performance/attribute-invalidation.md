# Attribute invalidation and per-dispatch export

The local interaction profiler now exports every observed dispatch to
`dispatch-events.csv`: one row per input, original document process/thread,
window start, chronological offset and inclusive duration. Repeated wheel and
pointer events remain separate. Existing `events.csv` and `events.md` summarize
each operation/event type; their individual observations already lived in
`summary.json`. Earlier reports describing those summary CSVs as chronological
have been corrected without altering their original measurements.

`MEDEN_GUI_STYLE_INVALIDATIONS=1` separately enables Chrome's advertised
invalidation category. The profiler counts instant style/layout observations
within the same marked document process/thread as the dispatch measurements.
It retains reason, attribute/pseudo and node description, excluding stack URLs
from the summary. Normal captures leave this field `null`. Counts describe
observations, not exclusive costs; no records does not establish no style work.
The full trace retains timestamps. Extra tracing is excluded from latency
comparisons.

The diagnostic flag requests a bounded 512 MiB Chrome trace buffer. Raw trace
and completion status are saved before admission. A lossy or unknown completion
is refused before producing the report, and missing/ambiguous phase marks still
fail independently. An initial native-wrapper capture lacked final phase marks;
a subsequent 128 MiB capture explicitly reported data loss and was rejected by
the new guard. Neither is admitted measurement. The final native-wrapper runs
complete without data loss. Normal captures retain their existing buffer settings.

## Actual EthLib causes

A fresh normal-motion Chrome 149 capture uses the deployed native host, original
browser assets and exact matching hidden source maps. It exercises all thirteen
public interaction windows, actual source/cache validation and map navigation.
The detail window reaches 358 declarations and records these style causes:

| Node/attribute                  | Observations |
| ------------------------------- | -----------: |
| Reference line `stroke-width`   |        1,131 |
| Symbol marker `transform`       |          914 |
| Declaration label `font-size`   |          914 |
| Reference line `stroke-opacity` |          944 |
| Main-theorem pin `transform`    |          671 |
| Declaration group `opacity`     |          735 |
| Inserted reference lines        |          506 |
| Inserted declaration groups     |          383 |

This window contains no observed scheduled selector-set invalidations. It does
contain presentation-attribute updates and inserted SVG subtrees. That narrows
the next investigation to geometry/attribute publication and the renderer; it
does not identify a universally expensive CSS selector or assign milliseconds
to these counts. Style and layout notifications can describe the same update.

## Reject isolated non-scaling reference strokes

An isolated prototype removes the reference component's `zoom` input and uses
SVG `vector-effect="non-scaling-stroke"` with a fixed width of one. The earlier
[combined SVG trial](svg-local-transforms.md) had tested that together with other
changes; this comparison isolates just reference width handling.

Both real workspaces run current/prototype/prototype/current in fresh 1440 × 960
Chrome contexts with normal motion and no throttling. Static asset substitution
preserves the actual native APIs and security headers. The executable, workspace,
source selection and hidden-map attribution stay fixed. Each capture completes
all thirteen windows, reuses the compiler artifacts and emits no post-setup
commands. Existing caches and ordinary desktop background activity remain.

| Detail window, runs 1 / 2      |            Current |          Prototype |
| ------------------------------ | -----------------: | -----------------: |
| EthLib maximum wheel dispatch  | 17.747 / 23.516 ms | 13.074 / 17.252 ms |
| EthLib maximum style interval  | 51.848 / 52.788 ms | 37.831 / 47.283 ms |
| EthLib maximum long task       |         72 / 84 ms |         58 / 63 ms |
| Riemann maximum wheel dispatch | 90.491 / 92.502 ms | 95.126 / 98.845 ms |
| Riemann maximum style interval | 34.198 / 30.493 ms | 33.467 / 33.834 ms |
| Riemann maximum long task      |       136 / 135 ms |       140 / 144 ms |

A 53 ms report-unit check ran while the first EthLib prototype capture's driver
was active. That capture is disclosed but excluded from improvement estimates.
No build, test or second sampler runs during the four Riemann captures. Their
targeted stall is worse with the prototype, so it is rejected. Smaller attribute
counts alone would not establish a faster interaction.

The [archived patch](../../results/reference-stroke-rejected-20261126.patch)
stays outside production. Draft geometric/retained-width regression additions
were restored too; they were not run and are not claimed as candidate coverage.
Full promotion/conformance validation is not claimed for a rejected prototype.

## Reject grouping separate background shapes

A second replay groups rounded shapes only while their expanded screen bounds
occupy separate spatial cells, with a barrier at labels. It preserves each
captured color/alpha and uses the installed font on fresh surfaces. The captured
Riemann maximum frame contains 12,511 areas. Across 24 interleaved replays,
median draw plus forced readback rises from **43.00 to 57.35 ms**. The prototype
reduces 25,022 fill/stroke calls to only 24,986 at DPR 1: overlap barriers and
different styles leave almost all calls intact.

The detail frame also differs in 296 channels at DPR 1 (maximum delta 6) and
1,270 channels at DPR 2 (maximum delta 4). Overview frames match. This is a
same-frame replay, not native interaction or physical presentation latency. The
grouping prototype is rejected; no grouping engine, cache or runtime switch is
introduced.

## Validation and remaining work

The [numeric record](../../results/attribute-invalidation-20261126.json) retains
original capture/trace/source hashes, per-window causes and the rejected
comparisons. Per-dispatch export refuses missing document attribution and invalid
timing rather than inventing a zero. Independent report tests cover repeated and
out-of-order inputs, CSV quoting, original capture preservation, malformed
timings and foreign-thread/out-of-window invalidation exclusion.

The actual public native profiling entry completes all thirteen windows on both
projects, with 96 / 1,431 native samples. Python's independent CSV reader matches
all **1,261 EthLib and 1,400 Riemann dispatch rows** against the original marked
document-thread events in their raw traces. Each native manifest hashes both
the per-dispatch CSV and trace completion status. The EthLib capture exercises
the diagnostic flag; the Riemann capture leaves it unset and retains `null`
invalidation observations.

All 37 required public TUI journeys and three native browser journeys in each
motion setting pass, without skipped/flaky/unexpected browser cases. The browser
checks cover retained tabs, screen-space geometry/keyboard/resize/restart and the
large-map loading/failure/restart path. Decoded TUI connection, history/restart,
freshness, source failure/recovery, cancellation and repair/clipped-review screens
are inspected. Existing sampled input/footer/spinner fragments remain; no
whole-renderer display claim is made. The four report cases are included in the
64 architecture/report checks; builds, generated schemas and formatting pass.

All ten original source/reader/inventory/navigation/evidence publications match
after capture; history and candidates remain intact and selected files hit their
compiler caches. Both hosts finish idle and unapproved. Their unchanged executable
and all 88 assets per host match the reviewed distribution and hidden-map copy.

The production executable, browser renderer and wire/kernel/controller behavior
remain unchanged. Riemann's first-detail pause and complete new-file preparation
remain work under the [migration plan](../architecture/migration-status.md).
These measurements are installed-tool/browser observations, not field INP,
physical presentation, live inference, cold installation or new refinement.
