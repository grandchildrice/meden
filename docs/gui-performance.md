# Graph interaction measurements

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
