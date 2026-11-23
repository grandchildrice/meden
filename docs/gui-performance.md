# Graph interaction measurements

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
