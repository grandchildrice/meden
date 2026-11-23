# Graph interaction measurements

## Latest deployed event measurements (November 23)

Two fresh sequential captures of the current pinned Riemann GUI reproduce the
same prepared-file wait and separate first selection from repeat selection.
The [complete measurement record](../results/event-latency-final-20261123.json)
retains both distributions, native stages, sampled source locations and verified
digests for 30 raw artifacts. The executable and all seven served JavaScript
bundles match the immutable deployment; unfinished model-transport worktree
changes are excluded. No application update, restart or inference was needed.
Chrome 149 used normal motion, 1440 × 960, no throttling, 175,187 indexed files
and a prepared 15-declaration file. Validation ran after profiling.

| Operation | Dispatch p95, runs 1 / 2 | Input/two-rAF p95, runs 1 / 2 |
| --- | ---: | ---: |
| Project-map wheel, 24 events | 4.210 / 4.275 ms | 18.6 / 18.2 ms |
| Dependency-graph wheel, 24 events | 0.777 / 0.952 ms | 16.6 / 23.3 ms |
| Hover mouse movement, 40 events | 0.104 / 0.067 ms | 24.1 / 20.5 ms |
| Drag mouse movement, 31 events | 0.193 / 0.281 ms | 20.3 / 21.5 ms |
| Resize, 3 / 4 delivered events | 1.794 / 0.974 ms | 24.9 / 20.9 ms |
| File/map tab click, 24 events | 15.463 / 14.175 ms | 40.0 / 36.5 ms |
| Fit View and node selection, 9 clicks | 34.114 / 29.549 ms | 71.9 / 59.7 ms |

Dispatch is synchronous browser event processing. The two-animation-frame
measurement also includes scheduling; it is neither physical presentation nor
field INP. Resize events are coalesced. There are no observed 50 ms long tasks
in these measured gesture or idle windows. This does not establish smoothness
for other workloads or exclude work outside those windows.

Chronological click traces resolve the selection tail more precisely: the
explicit Fit View costs 0.991 / 1.038 ms, the first node selection costs
34.114 / 29.549 ms, and the seven following selections of that same node cost
0.066–0.126 ms. Repeating a selected node is a different workload from changing
between declarations. Popper scroll-parent traversal, React rendering and
XYFlow node remeasurement appear in profiles. Automated selector/hit-testing
queries appear too; their sampled time must not all be attributed to the app.
Distinct-node selection remains a separate follow-up measurement.

Opening the file takes **15.730 / 15.714 seconds**, with a full analysis-cache
hit in both runs. The nested `atlas.project.prepare` stage takes
**10.214 / 9.941 seconds**; source-view materialization takes only
0.430 / 0.433 milliseconds. The last completed build log separates 4.844 seconds
of build/setup/input resolution from 4.499 seconds validating every byte in
43,477 files / 5,992,114,707 bytes. Python's SHA-256 routine accounts for
26.39 / 26.72 percent of sampled native host/worker self CPU, which is not a
wall-time fraction. These observations make exact-input preparation the first
wait-time optimization target. Layout discovery and cache admission also cost
time; nested spans overlap and cannot be added as exclusive costs.

Hover, resizing and retained-tab round trips preserve the exact graph transform.
Every gesture phase issues only state requests, with no new inspection, build
or inference. Idle polling still transfers 89,688 bytes roughly three times a
second, and native state projection p95 is 8.605 / 8.323 ms. Revision-aware
updates merit measurement, though these idle windows contain no long task.
The native ring evicts 539 / 541 older records during capture; the newly retained
record counts equal those deltas. Raw traces and profiles remain local under
`target/native-profile/event-latency-final-{1,2}`. These warm-cache diagnostic
captures are not cold-install or live-model validation.

The same pinned executable passed all 34 required public TUI journeys after
profiling. Decoded Lake failure/restart, meaning-history, provider-limit/retry
and GUI/TUI continuation screens, plus the captured browser screenshot, were
inspected. The gate's binary and summary digest are retained in the record.

## Batched Lake preparation (November 23)

Prepared analysis now requests the ordinary root build and the source, compiled
output and compiler-setup facets in one `lake --json query`. All selected roots
share Lake's build state. Separate build/setup-file/per-root query processes
were removed; exact setup and complete-byte input validation remain mandatory.
The worker rejects incomplete, extra, oversized and nonzero query output before
returning a fingerprint, streams actual compiler stderr, and reports resolution
and byte-validation durations separately.

The [measurement record](../results/lake-query-batching-20261123.json) compares
the same Riemann file, 175,187-file inventory and 15-declaration graph. Chrome 149
used normal motion at 1440 × 960 without throttling. Two warm post-update captures
both hit the complete analysis cache:

| Stage                          | Before, runs 1 / 2 | After, runs 1 / 2 |
| ------------------------------ | -----------------: | ----------------: |
| Explorer open to prepared view |  35.529 / 40.231 s | 15.605 / 15.790 s |
| Build/query/input validation   |  30.500 / 34.855 s | 10.066 / 10.300 s |
| Source-view materialization    |   0.640 / 0.385 ms |  0.413 / 0.424 ms |

The final cached log confirms one query and a full read of **43,477 files /
5,992,114,707 bytes**. Build/setup/input resolution took 5.135 seconds and byte
validation 4.524 seconds in that log. Twelve configuration observations were
reused in each warm capture; no configuration evaluator or fresh graph/search
extraction ran. Nested native stages overlap and cannot be added as exclusive
costs. Raw captures and artifact digests are retained locally.

These observations show a shorter prepare interval for this workload; they do
not establish a universal speedup or isolate every end-to-end difference to one
script edit. The native update also deploys compiler/renderer bindings already
on main but absent from the preceding executable. The asset update includes
the explicit-refresh wake-up fix described below. Exact source/build hashes are
recorded, and profiling begins after unrelated validation jobs finish.

The changed compiler/resource identities require a new observation artifact.
That **initial cache fill took 197.448 seconds**, including 134.974 seconds of
actual graph extraction and bounded counterexample observations, a second exact
fingerprint, rendering and persistence. It is reported separately from warm
cache validation and is not a cold installation. Parallel hashing was not
adopted: an independent read-only diagnostic on the same cached input inventory
measured serial per-file SHA-256 at 4.12–4.66 seconds and four threads at
1.78–1.80 seconds. Its aggregate format differs from production and is only a
diagnosis of hashing cost.

Faster background work exposed a real navigation failure: a directory request
could start and finish within one React batch, leaving the visible busy boolean
unchanged and never waking the pending file read. Command activity now retains
generation and pending count. The real 35-module browser journey requires one
requested-file inspection on every explicit refresh, including configuration
failure/recovery, dependency edits and restart. Cached logs require one query and
completed byte validation. Held-compiler tests cover streamed output, timeout,
cancel/retry and UTF-8 diagnostic truncation. Retained-tab budgets pass in both
motion modes. Fifteen worker-protocol fixtures, 32 TypeScript tests, all 262 Rust
tests, warning-free Clippy and all 34 required public TUI journeys passed; decoded
failure/retry/restart screens and actual browser screenshots were inspected.
The worker-mutation fixture now makes only its own temporary copy writable so
the original pinned executable can stay read-only. These checks add no new
refinement, inference-quality or confidentiality claim.

Warm opens still take about 16 seconds. Exact input reads, initial extraction
and independent search caching remain optimization/migration work.

## Indexed map navigation (November 23)

The deployed map now reuses its branch-pruned visible areas for declaration
legend and chunk-selection decisions. The legend appears only for rendered,
nonempty symbol sets. The center breadcrumb traverses containing branches and
derives directly from the current camera; its separate location state and
whole-inventory move-end scan are deleted. A canvas overlay keeps the breadcrumb
from resizing the viewport during normal-motion focus.

Two post-deployment Riemann captures use the same 175,187 files, 15-declaration
file, native executable digest, Chrome 149, 1440 × 960 viewport and normal motion
as the preceding two captures. The
[comparison record](../results/map-navigation-pruning-20261123.json) retains
raw-artifact digests, per-event distributions and unchanged-backend attribution.

| Operation                                  | Dispatch p95 before, runs 1 / 2 | Dispatch p95 after, runs 1 / 2 |
| ------------------------------------------ | ------------------------------: | -----------------------------: |
| Project-map wheel, 24 events               |               10.900 / 9.050 ms |               2.697 / 2.519 ms |
| File/map tab click, 24 events              |              16.800 / 23.225 ms |             10.803 / 19.297 ms |
| Node sequence including Fit View, 9 clicks |              34.064 / 29.695 ms |             35.499 / 30.836 ms |

The wheel handler reduction is about 72–75% in these captures. The input/two-rAF
proxy did **not** improve consistently: map-wheel p95 was 20.3 / 17.3 ms before
and 24.6 / 24.6 ms after. These are diagnostic event dispatch and scheduling
observations, not physical paint or field INP; do not claim a 75% end-to-end
rendering improvement. Tab click/two-rAF p95 was 32.9 / 42.9 ms after, versus
37.7 / 47.0 ms before. CPU samples no longer show the legend's whole-area scan
among the map-wheel hotspots.

Hover, resize and retained-tab round trips preserve the graph transform in both
captures; no interaction phase issues a non-state request. Neither settled
graph idle nor map idle contains a long task. The first post-restart graph-idle
window contains 174 / 434 ms tasks, with `projectMap` and Zod parsing sampled;
the second capture does not reproduce them. This supports a remaining initial
materialization cost, not a claim that every state poll rebuilds the map. One
tab phase contains an 86 ms task and one selection phase a 53 ms task. Selection
and initial materialization remain follow-up work.

Prepared-file opens took 35.529 / 40.231 seconds after deployment, with nested
build/input resolution and fingerprinting taking 30.500 / 34.855 seconds.
Both analysis caches hit, twelve configuration observations were reused and
no Lake configuration evaluator ran. This dominant native wait remains variable;
the frontend change does not optimize or explain its variation. Hosts restarted
for the new immutable assets, so the before/after cache/OS conditions are not a
controlled cold-start comparison. The actual native executable bytes did not
change. Riemann and EthLib now serve the same pinned frontend build.

The new browser regression caught a stale `File15.lean` breadcrumb while
`File0.lean` occupied the center. It now requires the correct settled breadcrumb,
visible legend and unchanged canvas height, along with the existing real
10,001-file declaration/selection, invalid-source, public-refresh and restart
journey. The actual screenshot and decoded TUI failure/retry/restart screens
were inspected. Three normal-motion browser journeys and the reduced-motion
retained-tab budget passed, as did 32 TypeScript tests, 3,000 generated
inventory/camera cases, schema/type/architecture checks and all 34 required TUI
journeys. The expanded algorithm budget covers visibility, legend and breadcrumb
at overview, category and fitted-file cameras; the full-scan mutation fails its
ratio assertion. These are presentation/runtime checks, not refinement,
live-model or cold-install evidence.

## Per-event and native-stage audit (November 23, evening)

Two sequential captures of the pinned Riemann GUI cover 175,187 indexed files
and the same prepared 15-declaration graph. Both opens hit the complete analysis
cache. The [measurement record](../results/event-stage-audit-20261123.json)
retains distributions, verified raw-artifact digests, native stages and sampled
source attribution. Chrome 149 used normal motion at 1440 × 960 without
throttling. No application runtime or deployment changed for this audit.

| Operation                                  | Dispatch p95, run 1 / 2 | Input/two-rAF p95, run 1 / 2 |
| ------------------------------------------ | ----------------------: | ---------------------------: |
| Project-map wheel, 24 events               |       10.900 / 9.050 ms |               20.3 / 17.3 ms |
| Dependency-graph wheel, 24 events          |        0.904 / 1.051 ms |               25.8 / 17.8 ms |
| Hover mouse movement, 40 events            |        0.080 / 0.100 ms |               25.9 / 24.1 ms |
| Drag mouse movement, 31 events             |        0.312 / 0.279 ms |               21.2 / 21.6 ms |
| Resize, 5 / 3 delivered events             |        1.867 / 1.918 ms |               23.1 / 20.3 ms |
| File/map tab click, 24 events              |      16.800 / 23.225 ms |               37.7 / 47.0 ms |
| Node sequence including Fit View, 9 clicks |      34.064 / 29.695 ms |               74.2 / 37.4 ms |

The proxy now separates event types: hover/drag use pointermove observations
for the corresponding mouse movements, and resize uses window resize. It is
not physical presentation or field INP. Node selection includes one explicit
Fit View and eight selections of a reachable node; it is not a distribution
over arbitrary declarations. Its first run contained one 52 ms long task.
Neither initial/settled graph idle nor map idle contained a long task. The
hover, resize and tab-return phases preserve the retained graph transform in
both runs. These checks do not rule out a reset during other gestures or layouts.
Every interaction phase issued only state polls, with no new inspection,
preparation or inference request. Settled state responses contained 89,419 bytes.

The profiler now selects source maps explicitly with `MEDEN_GUI_ASSETS`.
All seven generated JavaScript bundles were compared byte-for-byte with the
immutable deployment before using a separate hidden-source-map build. This
avoids attributing a running bundle through a newer mutable build. Map digests
are retained; the deployed assets were not replaced.

The main remaining costs are distinct:

1. **Opening a prepared file still validates its compiled inputs.** The two
   opens took 29.040 / 24.897 seconds. The nested actual build/input-resolution
   and fingerprint stage took 23.847 / 19.979 seconds, including validation of
   43,477 compiled inputs. Layout discovery took 1.846 / 1.807 seconds; twelve
   Lake configuration observations were reused with zero evaluator executions.
   Exact cache-validation spans totaled 1.211 / 1.161 seconds. Source-view
   materialization took 0.412 / 0.409 milliseconds. These overlapping stages
   cannot be added as exclusive costs. Reusing precisely bound preparation
   observations with explicit invalidation is the first optimization target.
2. **Map legend and breadcrumb decisions bypass visibility pruning.** Wheel
   CPU samples resolve to `NativeWorkspaceGraph.tsx:468`,
   `project-map-model.ts:252/256` and `WorkspaceMap`. The legend's
   `scene.areas.some` runs on camera updates, and `onMoveEnd` separately walks
   all areas for the breadcrumb. Tab reactivation also samples this work.
   Both decisions should use the existing indexed visible-area query.
3. **Selection and tab activation have a rendering tail.** DOM queries,
   Popper scroll-parent traversal and React work appear in the selection
   samples. This identifies a narrower follow-up around selection/tooltips;
   it does not assign all click latency to one function. Measure that path
   independently after removing map-wide scans.

Native stage summaries exclude span IDs present before the capture. There was
no eviction in either run. Host-thread CPU excludes workers, and nested spans
overlap. Raw captures remain local under
`target/native-profile/event-audit-riemann-{2,3}`. These are controlled desktop
observations using warm project/OS caches, not cold-install measurements or
confidence intervals.

The same measured native executable passed all 34 required public TUI journeys.
Decoded Lake inventory-refusal/history, provider-limit/retry and GUI/TUI
continuation/fresh-review screens were inspected, along with the actual map
screenshot. All 30 TypeScript tests, architecture boundaries, script syntax and
format checks passed. This audit adds no live-model or cold-install claim.

## Frozen Lake configuration reuse (November 23)

The host now retains deterministic configuration evaluation separately from
compiler/search observations. Keys bind the complete frozen input map and
absences, command/options, validated Lean/library manifest and implementation.
Reads check the full identity and output hash and use the same result parser.
Failed evaluations are not saved; the 4096 replacement slots retain outputs up
to 1 MiB. A collision or larger output requires fresh evaluation. The actual
Lake build and compiled-input fingerprint remain mandatory on a reopened file.

Three sequential Riemann captures use the same native executable, 175,187-file
inventory and 15-declaration file. All three hit the existing complete analysis
cache. The [measurement record](../results/lake-configuration-cache-20261123.json)
retains native/browser artifacts, checked digests and validation evidence.

| Stage                                     | Initial configuration cache fill | Repeated open 1 | Repeated open 2 |
| ----------------------------------------- | -------------------------------: | --------------: | --------------: |
| File request to prepared view             |                         59.748 s |        35.658 s |        24.817 s |
| Actual configuration evaluations          |                               12 |               0 |               0 |
| Reused configuration observations         |                                0 |              12 |              12 |
| Layout discovery                          |                         20.783 s |         2.042 s |         1.933 s |
| Build/input resolution and fingerprinting |                         35.046 s |        29.896 s |        19.277 s |

These are nested diagnostic spans, not additive exclusive costs. Project/OS
caches also warm, so the entire elapsed-time change cannot be attributed to
configuration reuse. The initial capture used the preceding navigation asset
build; repeated captures used the final typed navigation build. Asset digests
are retained. No graph-engine improvement or cold-install result is claimed.
The remaining build/input validation is still the largest measured wait.

The public GUI regression checks real configuration failure on two attempts,
dependency edits, compiler rejection/retry and restart. A delayed-status fixture
also reproduces an older retained-view map after refresh: the next requested
file must still dispatch. Navigation distinguishes an explicit read, a retained
tab and an already prepared/requested graph, avoiding both a lost refresh and
a second inspection after explicit preparation. Existing tab returns retain
source/graph/camera without inspections. The three real-Lean browser journeys,
34 required TUI journeys, 45 application unit tests, two doc tests and 30 TypeScript
tests passed; decoded failure/restart screens and the browser screenshot
were inspected. These checks add no new proof authority or live-model claim.

## Production recheck (November 23)

A third capture of the same pinned Riemann host confirms that preparation,
map navigation and selection have separate costs. It uses a fresh browser
page, the existing project/compiler caches and normal motion. The
[recheck record](../results/event-measurements-recheck-20261123.json) retains
the distributions, latest native trace, environment and verified artifact
digests. Application code, project sources and the running services were not
changed for this measurement.

| Operation                         | Events | Handler p95 | Phase input/two-rAF p95 |
| --------------------------------- | -----: | ----------: | ----------------------: |
| Project-map wheel                 |     24 |   10.721 ms |                 26.7 ms |
| Dependency-graph wheel            |     24 |    0.962 ms |                 17.1 ms |
| Hover mouse movement              |     40 |    0.116 ms |                 29.4 ms |
| Drag mouse movement               |     31 |    0.400 ms |                 37.1 ms |
| Resize                            |      6 |    1.894 ms |        Not instrumented |
| File/map tab click                |     24 |   24.676 ms |                 55.5 ms |
| Node sequence, including Fit View |      9 |   29.863 ms |                 76.6 ms |

The phase proxy combines instrumented input types, so it is not the latency
of each handler or a physical paint measurement. The drag proxy contains only
two instrumented samples. Node clicks repeatedly select a reachable node and
include explicit Fit View; the sequence contained one 50 ms long task.
None of the gesture or tab phases issued non-state API requests, and none of
the initial/settled graph or map idle windows contained a long task.
State polling still transfers 89,687 bytes per response in this capture;
response-end p95 was 25.454 ms in settled graph idle and 14.688 ms in map idle.
That is recurring network/serialization work, but these idle windows do not
show a corresponding repeated long rendering task.

The prepared-cache open took **68.762 seconds**, compared with 40.04 / 41.20
seconds in the preceding captures. Native extraction took 67.529 seconds:

- Layout discovery: 26.013 seconds, including twelve Lake configuration calls
  totaling 24.013 seconds.
- Build/input resolution and fingerprinting: 38.746 seconds. The visible build
  log reported validation of 43,477 compiled inputs.
- Exact cache-validation spans: 1.269 seconds.
- Source-view materialization: 0.427 milliseconds.

These stages overlap and must not be added as exclusive costs. Native sampling
collected 4,106 samples, with Python/OpenSSL SHA-256 and actual Lake execution
visible. This confirms expensive revalidation before cached display; it does
not assign all preparation time to hashing. Timing variation on the same
binary does not establish a code regression or identify its cause.

The next optimization order remains: reuse evaluated Lake configuration and
prepared input observations with explicit invalidation, remove full-inventory
legend/breadcrumb scans during navigation, then reduce tab/selection layout
work. Cache reuse must preserve the original source/configuration/resource
bindings. Raw captures remain local under
`target/native-profile/riemann-events-recheck`; repeat with the commands below.
The required TUI gate passed all 34 journeys on the current main validation
executable, with decoded Lake-failure/restart, provider-stop/retry and fresh
processor-review screens inspected. This is a separate executable from the
pinned production host measured above; neither gate establishes cold-install
or live-model performance.

## Current event audit (November 23)

Two sequential captures of the pinned running Riemann GUI cover 175,187 files
and the prepared 15-declaration `DictGen.lean` graph. Chrome 149 used normal
motion at 1440 × 960, without throttling. Both opens reported a persisted-cache
hit. The [complete event record](../results/event-measurements-20261123.json)
retains event counts, distributions, rendering costs, native spans and raw
artifact digests. No application runtime or user source was changed for this
audit; the profiler now separates initial and settled idle windows.

| Operation                                             | Handler p95, capture 1 | Handler p95, capture 2 |
| ----------------------------------------------------- | ---------------------: | ---------------------: |
| Project-map wheel, 24 events                          |              10.324 ms |              10.857 ms |
| Dependency-graph wheel, 24 events                     |               1.000 ms |               1.048 ms |
| Hover mouse movement, 40 events                       |               0.108 ms |               0.103 ms |
| Drag mouse movement, 31 events                        |               0.253 ms |               0.276 ms |
| Resize, 4 / 3 delivered events                        |               0.942 ms |               0.960 ms |
| File/map tab click, 24 events                         |              21.808 ms |              20.696 ms |
| Node selection, including explicit Fit View, 9 clicks |              33.120 ms |              32.079 ms |

Handler time excludes subsequent rendering. The tab phase's input/two-rAF p95
was 44.5 / 46.3 ms; the node-selection phase's was 77.4 / 63.8 ms. Those phase
proxies combine instrumented pointer/click events and are not physical paint or
field INP. The node sequence repeatedly selects a reachable node and includes
one explicit Fit View; it is not a percentile across arbitrary declarations.
One selection phase contained a 53 ms long task. Resize events were coalesced.
No measured gesture or tab phase issued a non-state API request.

The largest delay remains opening a prepared file: **40.04 / 41.20 seconds**.
Within those intervals, twelve Lake configuration executions totaled
17.23 / 17.12 seconds, and prepared build/input resolution and fingerprinting
took 17.87 / 19.03 seconds. Exact cache-validation spans totaled
1.14 / 1.20 seconds; source-view materialization took 0.418 / 0.397 ms.
These are nested native stages and must not be added as exclusive costs.
The prepare interval includes real Lake commands and sandbox work as well as
hashing; it cannot all be attributed to file hashing. Native CPU sampling
confirmed 2,922 / 2,956 samples, with hashing in Python/OpenSSL and actual Lake
execution visible. Some Lean-library addresses remain unresolved.

The map-wheel CPU call tree identifies another concrete cost:
`WorkspaceMap` calls `declarationDetail` through its full `scene.areas.some`
scan to decide whether to show the legend. That decision runs during camera
updates even though viewport visibility already has a branch-pruning index.
The declaration-detail function alone accounted for 150.8 ms of sampled self
time across the first wheel sequence. The breadcrumb's `onMoveEnd` also scans
all areas; it should reuse an indexed viewport query, with its own regression.

Optimize in this order:

1. Reuse evaluated Lake configuration and prepared immutable input observations,
   with explicit source/configuration/resource invalidation. Preserve exact
   freshness checks; a cache hit must remain bound to its original inputs.
2. Derive legend and breadcrumb decisions from indexed visible areas, avoiding
   whole-inventory work during navigation.
3. Reduce retained-tab reactivation and source/popover layout work, measuring
   the rendering tail separately from the click handler.

Neither capture reproduced the historical roughly 384 ms map materialization
in its initial idle window. The additional settled graph and map idle windows
contained no long tasks. A new inventory/revision can still require expensive
layout; these observations do **not** establish that every 350 ms state poll
rebuilds it. Initial overview arrival and revision changes need their own
capture before making that attribution.

Raw recordings are local under
`target/native-profile/riemann-events-{current,settled}`. Use the existing
reproduction commands below; the second capture adds `graph-idle-settled` and
`project-map-idle` phases. These are diagnostic observations on one desktop,
not confidence intervals, cold-install results or live-model measurements.

The November 22, 2026 audit uses the actual Riemann workspace: 175,187 indexed
files and the compiled 15-declaration `DictGen.lean` graph. Chrome 149 ran at
1440 × 960 without CPU or network throttling, with reduced motion enabled.
The [measurement record](../results/gui-interactions-20261122.json) retains the
event distributions, sampled CPU attribution and intermediate changes.

## Observed event costs

These are renderer `EventDispatch` p95 durations, in milliseconds, from one
controlled interaction sequence per revision. They are not field INP or a
hardware-independent guarantee.

| Operation                                             | Before |  Final |
| ----------------------------------------------------- | -----: | -----: |
| Project-map wheel, 24 events                          | 42.809 | 10.554 |
| File/map tab click, 24 events                         | 47.509 | 25.809 |
| Dependency-graph wheel, 24 events                     |  0.762 |  1.031 |
| Hover mouse movement, 40 events                       |  0.073 |  0.104 |
| Graph drag mouse movement, 31 events                  |  0.286 |  0.415 |
| Resize, 6–8 delivered events                          |  1.133 |  1.461 |
| Node-selection sequence, including one Fit View click | 66.353 | 40.223 |

Small differences in the already cheap pointer/graph-wheel handlers are not
evidence of an improvement. The expensive work was the map traversal and work
triggered after selection. In the node-selection phase, the longest style
recalculation fell from 240.611 ms to 9.313 ms. The initial run recorded a
248 ms long task there; the final action window recorded none above 50 ms.

The input-to-two-animation-frames proxy fell from 66.5 to 28.9 ms p95 for map
zoom and from 93.4 to 60.1 ms for tab switching. This proxy includes browser
scheduling, not actual physical presentation. Wheel-end/debounce callbacks can
finish outside the action window. Raw traces retain the surrounding work;
absence of a long task in one window does not establish universal smoothness.

## Causes and changes

- **Automatic fitting reclaimed the camera.** A second fit controller observed
  every canvas resize. A regression reproduced a zoom of 0.168469 falling to
  0.0948569 after an incidental resize. Removing that controller leaves initial
  fitting and the explicit Fit View control. Hover and resizing preserve the
  user's transform. The passive review legend also intercepted pointer events
  over nodes and wheel gestures; it now passes them through.
- **Visibility calculation scanned all 175,187 files on every move.** The map
  now builds a parent/children index with its stable coordinates and skips
  branches outside the viewport or below the opacity cutoff. The visibility
  result is checked against an independent full-scan oracle across 24 views.
  The minimap uses the root index too. Hidden maps retain their camera without
  processing parent-driven status updates; activation receives current data.
  Previously, `visibleMapAreas` consumed about 114 ms of sampled self time even
  during a three-second idle period on the file tab. It no longer appears among
  that period's leading CPU samples after the change.
- **Selecting a loaded line rebuilt the source page.** The reader moved its
  200-line page to `selectedLine - 5` on each selection, even for a line already
  loaded. That replaced tokens/DOM and forced style work during Popper's position
  reads. The page now remains intact for selections within it, while the reader
  scrolls/highlights the target. Out-of-page selection still loads its page.
- **Reduced motion created unintended transitions.** Giving every element a
  nonzero transition duration activated default `transition-property: all` on
  changing SVG coordinates and font sizes. The initial map-wheel sequence
  generated 1,813 transition starts; the final sequence generated zero. A zero
  transition duration removes this work. Normal-motion hover remains tested.

There is still work to investigate. Map legend and breadcrumb decisions retain
large-array checks, and tab reactivation includes real layout/paint work.
State polling transfers roughly 178 KB per response at the existing interval;
the dominant sampled cost was the map work it triggered. Initial prepared-cache
validation remained about 53–56 seconds and is outside the warm interaction
comparison. This audit does not establish its Lean/Lake or sandbox-preparation
CPU breakdown, and does not claim first-open compilation is instant.

## Reproduce and inspect

Use installed development dependencies and an already prepared file in a running
native GUI. The explorer open validates that file's cache before timing begins.
The profiler does not send chat, request inference or edit project sources.

```sh
npm run typecheck
npm run build --workspace @meden/web -- --sourcemap
node tools/gui/profile-interactions.mjs '<GUI_URL>' target/gui-profile/review
```

`MEDEN_GUI_CHROME` selects another installed Chrome executable. The output
contains `summary.json`, a Chrome `trace.json`, and one `.cpuprofile` per action.
CPU attribution resolves the built bundles' local source maps when present.
Session fragments are redacted from exported traces. Profiling runs locally and
adds no always-on instrumentation or dependency to the production renderer.

The browser MCP was unavailable; the tool uses installed Playwright and Chrome's
[Tracing](https://chromedevtools.github.io/devtools-protocol/tot/Tracing/) and
[Profiler](https://chromedevtools.github.io/devtools-protocol/tot/Profiler/)
protocols. The first capture includes profiler boundaries in frame intervals;
later captures exclude them. Frame maxima are therefore not used for a before/
after claim. Raw profiles and traces for this audit live in
`target/gui-profile/riemann-{baseline,after,final,motion-fix}`.

Required public TUI journeys and decoded failure/restart screens are recorded
with the same native binary. Browser regressions exercise pointer movement,
wheel routing through the legend, resizing, explicit fit, a 10,001-file map,
retained views and selection without source-page replacement. Compiler/model
fixtures do not establish cold installation, live-model quality or renderer
refinement.

## Native preparation and routine gates (November 23)

`tools/perf/perf.sh` provides a pinned host-only perf tool. Same-user user-space
CPU sampling succeeded with paranoid=2 and kptr_restrict=1, without changing
kernel permissions. The production host now retains 4096 completed local spans
and an eviction count, with static stage names, numeric correlation and parent
IDs, monotonic offsets, wall time and current-thread CPU time. The authenticated
`/api/metrics` query has the same session/loopback checks as state. There is no
source/prompt/credential label or default remote export. Native samples include
inherited worker CPU; host thread CPU does not. Wall minus thread CPU includes
child execution and scheduling, not just I/O wait. Nested spans overlap.

The same actual Riemann file/cache-validation journey took 64.98 s in the initial
unoptimized development host, 46.40 s with SHA-256 dependency optimization, and
39.33 s in a subsequent normal-motion capture. These are diagnostic observations,
not repeated before/after confidence intervals: OS caches and concurrent test
load can differ. The first capture collected 4765 native samples. Initial capture
finalization exposed two tool bugs (SIGINT exit 130 and a not-yet-open report
stream); both were fixed, the retained recordings decoded independently, and the
final attach run completed with 2899 confirmed samples and an artifact manifest.
Some installed Lean shared-library addresses have no usable symbol names; the
profiles do not establish function attribution for those addresses.

The first instrumented file trace exposes the remaining work:

| Stage                                 | Wall time | Host thread CPU | Interpretation                                      |
| ------------------------------------- | --------: | --------------: | --------------------------------------------------- |
| Analysis extraction                   |  45.775 s |         1.521 s | Overall preparation interval                        |
| Select roots/discover Lake layout     |  20.441 s |         1.181 s | Includes 12 Lake configuration executions           |
| Lake configuration executions         |  18.575 s |         0.484 s | Includes their process/sandbox intervals            |
| Prepared Lake build/input fingerprint |  23.391 s |         0.159 s | Worker checks 43477 compiled input files            |
| Cached observation validation         |   0.988 s |         0.066 s | Exact resource, metadata and selected-source checks |

These rows are nested, so their durations cannot be added. Resource selection
and validation take 0.870 s across this host snapshot; 14 sandbox preparations
total 7.136 s, including startup and nested compiler work. This identifies repeated
Lake configuration and the compiled-input fingerprint as the next application
optimization targets. No hash/freshness check was removed: only `sha2` is optimized
in development builds. The separate profiling binary retains debug symbols and
avoids replacing a user's running executable.

CI now runs a 3000-case shrinkable differential inventory/camera corpus and nine
paired visibility measurements on 175187 generated files. Optimized median cost
must stay below 35% of an independent full scan in the same runner/process; a
full-scan negative mutation must fail that budget. Absolute layout timings remain
diagnostic. The native retained-tab journey records first preparation from an
empty analysis store, then ten actual tab-click/two-rAF samples. Serialized runs
in reduced/normal motion require warm p95 below 250 ms, no new inspection or
preparation and stable retained graph/source/camera. This is a scenario target,
not physical paint/INP, cold installation or model-quality evidence. Separate
output directories preserve both motion-mode reports. The diagnostic CDP profiler
also includes a 600 ms gesture tail plus eight frames, with profiler boundaries
excluded consistently.

See [tools/perf/README.md](../tools/perf/README.md) for reproduction and seed/path
replay, and [the native result record](../results/native-profiling-20261123.json)
for raw-artifact references and completed checks. Shared CI hardware is not a
fixed physical CPU: the algorithm gate uses paired relative timings and records
its environment; the browser budget uses an explicit serialized workload and a
broad scenario deadline. Neither is a universal performance claim.

## Source projection follow-up (November 23)

The native client now receives an application-owned prepared source/graph page,
instead of raw Atlas/package records and a second browser lexical index. Original
backend inputs remain retained for explicit analysis; the projection carries no
accepted-contract proof receipt. Its `analysis.materialize_view` span took
1.767 ms on the actual 15-declaration Riemann page. A separate 4,000-declaration
lexical regression completed in 89.6 ms in an unoptimized development test;
that single diagnostic is not a performance budget.

The updated production binary and browser assets were pinned together. Two real
native/browser captures distinguish the first open after an adapter update from
a new host session reusing the resulting persisted cache:

| Operation                                               | Observation |
| ------------------------------------------------------- | ----------: |
| First preparation after renderer cache identity changed |    229.97 s |
| Preparation with a confirmed persisted-cache hit        |     40.09 s |
| Project-map wheel handler p95                           |   10.533 ms |
| Dependency-graph wheel handler p95                      |    1.008 ms |
| Hover pointer movement p95                              |    0.065 ms |
| Drag pointer movement p95                               |    0.169 ms |
| Resize handler p95                                      |    1.316 ms |
| Tab click handler p95                                   |   19.267 ms |
| Node-selection click handler p95                        |   28.274 ms |

These are single-run diagnostic observations on the same 175,187-file project,
Chrome 149 at 1440 × 960, normal motion, without throttling. Handler durations
exclude later rendering. The warm trace's input/two-rAF proxies were 16.9 ms for
graph wheel, 39.2 ms for tabs and 58.0 ms for node selection. The separate final
serialized native regression recorded warm-tab p95 of 29.2 ms against its 250 ms
scenario budget. These are different controlled sequences, not interchangeable
field percentiles.

The retained artifacts explain the unusually slow first open: source revision,
compiler resources and prepared input fingerprint were unchanged, while the
renderer binding differed. The renderer hashes its adapter source, which changed
in the preceding application extraction. The combined cache consequently reran
graph extraction and counterexample observations before rendering. Search has a
120-second file wall budget, with at most ten seconds per declaration; a timeout
is explicitly inconclusive. Compiler/search artifacts should be cached separately
from renderer output so a renderer update does not repeat unrelated search.

Even a valid persisted cache still spends 19.059 s selecting roots/layout,
including twelve Lake configuration calls totaling 16.939 s, followed by
17.105 s preparing/fingerprinting compiled inputs. These intervals are nested.
The exact input and sandbox checks remain in place. Deduplicating configuration
preparation is still necessary; source-view materialization is not the bottleneck.

The trace also retains a remaining UI pause: the initial idle/poll phase recorded
two long tasks, the largest 394 ms. Its sampled project-map layout function used
230.9 ms of self time. The production minified function body matches `projectMap`
in `project-map-model.ts`; this follow-up did not include browser source maps.
This is whole-inventory layout work outside a wheel handler and must not be hidden
by reporting cheap pointer events. Layout materialization/reuse is still needed.
State polling transferred about 89.7 KB per response in this capture.

See [the source-projection record](../results/source-projection-20261123.json)
for binary/artifact digests, both traces, residual costs and completed checks.
The first raw report retains the old `setup_cache_validation_ms` name despite
its cache miss; the profiler now emits `setup_preparation_ms` and an explicit
`setup_cache_hit` value to prevent that ambiguity.

## Independent compiler and renderer caches

The combined cache identified above has been replaced. Compiler ports return raw
compiler/search observations; the application retains them before selecting the
renderer. The raw artifact includes original source text, module/configuration
observations, any prepared Lake input fingerprint, compiler resources, structural
English and search outcomes. Existing exact source, metadata and resource checks
still run before reuse. Lake still prepares and fingerprints its compiled inputs;
this change does not remove its roughly forty-second validation cost.

Renderer readings are a separate artifact keyed by the immutable raw artifact
and selected renderer fingerprint. They contain only declaration IDs, sentences,
renderer attribution and rendering errors. Publication refuses missing, duplicate
or foreign declarations and changes to any compiler/search field. The completed
graph revision also binds the reading artifact, so returning a new reading for
the same source cannot leave an old browser memoized view. A failed renderer
starts from retained structural English, rather than attributing a previous
renderer's sentence to a different implementation.

Compiler/search data are persisted before rendering, and the original is held in
an immutable consuming application value. A rendering interruption therefore
does not require repeating successful compiler/search work. Cancellation gates
raw retention and final rendering-cache admission. These cache observations neither
accept a contract nor carry an admitted verification receipt. Filesystem and
physical stop correspondence remain adapter obligations.

The new cache is under host-owned `compiled-views`, with separate bounded indices
and checksummed objects for observations and renderings. The obsolete combined
cache loader was removed. An old combined artifact cannot reconstruct its lost
structural reading, so first use prepares the new raw artifact once; no legacy
compatibility path is retained.

The public renderer-update regression copies actual prepared Informath resources,
changes a host resource with a matching manifest, and opens the same source after
restart. It checks preserved counterexamples and formal inputs, a different view
revision, renderer execution, and no compiler/search scripts in authenticated
native spans. Another restart reuses the complete view without inference approval.
An actual malformed source then fails, and the public retry succeeds after a
source correction. The attached `renderer-cache-preparation` timings are
diagnostic installed-tool measurements, not cold installation, live inference or
physical presentation. Unit checks also reject changes to checker results,
source, refutation and expressions, corrupted/misbound overlays and partial or
unbounded readings. A compile-fail example checks that the original observation
cannot be mutated through its public accessor.

On the actual one-theorem installed-Lean/GF fixture, the final run recorded
30.43 s for initial preparation, 2.30 s after the renderer resource update, and
1.77 s for a complete-cache reopen. These are single diagnostic file-request to
prepared-DOM intervals, with concurrent functional validation and no hardware
budget claim. The no-recompilation claim comes from the independent native spans
and preserved observations, not a timing threshold. They do not predict Riemann's
larger Lake input-validation cost. See
[the cache split record](../results/renderer-cache-20261123.json).
