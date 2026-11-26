# Deployed interaction timing, November 26

The [numeric record](../../results/event-audit-20261126.json) measures the
unchanged `e41a847` deployment on the actual EthLib and Riemann workspaces.
It separates synchronous event dispatch, subsequent rendering, request completion
and native preparation. The saved-project-map prototype in the working tree is
not deployed or included in these observations.

Two fresh-browser captures per project run sequentially with normal motion,
Chrome, a 1440 × 960 viewport and no throttling. Every served JavaScript bundle
matches the retained hidden-source-map build. Native executable and raw capture
hashes are recorded. There are no concurrent builds, tests, model requests or
other samplers during capture. Existing project, semantic and OS caches remain;
this is not cold-install or field latency measurement.

Each capture retains all 13 operation windows, chronological event dispatches,
event-type summary CSV/Markdown tables, browser CPU profiles, document-thread traces,
native user-space CPU samples, bounded stage spans and the original preparation
log. Raw captures stay private under
`target/native-profile/event-audit-20261126/`.

## Event costs

Inclusive synchronous EventDispatch p95 in milliseconds, runs 1 / 2. Counts are
delivered events per run, rather than intended automation actions.

| Operation                   |          EthLib |         Riemann | Events per run |
| --------------------------- | --------------: | --------------: | -------------: |
| Dependency-graph wheel      |   1.085 / 0.987 |   0.764 / 1.032 |             24 |
| Hover pointer movement      |   0.056 / 0.098 |   0.071 / 0.071 |             40 |
| Pan pointer movement        |   0.427 / 0.396 |   0.312 / 0.219 |             31 |
| Window resize               |   0.847 / 0.812 |   1.471 / 0.856 |            2–8 |
| Retained file/map tab click | 12.074 / 11.749 | 16.139 / 16.861 |             24 |
| Same-node click             | 19.728 / 18.551 | 33.872 / 30.423 |              8 |
| Different-node click        |  12.254 / 7.151 | 13.095 / 14.550 |              8 |
| Explicit Fit View           |   1.350 / 1.041 |   0.567 / 1.085 |              1 |
| Project-map overview wheel  |   4.369 / 4.704 |   3.038 / 4.063 |             24 |

Resize events can coalesce. Fit View has one event per capture; these values
are individual observations, not meaningful population percentiles. Tab input
to two animation callbacks has p95 **29.8 / 28.6 ms** in EthLib and
**40.1 / 41.7 ms** in Riemann. Different-node selection has p95
**16.3 / 18.8 ms** and **23.3 / 26.0 ms**, respectively.

All 64 node selections retain the graph camera, as do the hover and tab windows.
This does not reproduce or rule out every simultaneous pointer/zoom reset.
Camera-reset correctness remains separate from rendering latency.

## Why deeper zoom stalls

The drill follows reachable category headers, then wheel input until a file and
its declarations are visible. Its complete elapsed time includes deliberate
650 ms category animations, automation, details and settling; that total is not
an event latency.

| Observation, runs 1 / 2     |                EthLib |            Riemann |
| --------------------------- | --------------------: | -----------------: |
| Maximum deep-wheel dispatch |    18.596 / 19.406 ms | 95.593 / 96.686 ms |
| Maximum observed long task  | 63 ms / none observed |       137 / 142 ms |
| Long-task count             |                 4 / 0 |              2 / 2 |
| Total long-task time        |            248 / 0 ms |       196 / 203 ms |
| Frame-interval p95          |        16.8 / 16.8 ms |     33.3 / 33.3 ms |
| Maximum frame interval      |      133.2 / 150.0 ms |   133.2 / 133.3 ms |
| Prepared detail reads       |                 7 / 7 |            10 / 10 |

The largest repeatable synchronous stall is **Riemann's first-detail wheel**.
Inside those original 95.593 / 96.686 ms intervals, sampled self time includes
18.351 / 18.479 ms in native `setAttribute`, 15.629 / 10.259 ms in map SVG/React
code and 13.926 / 19.349 ms in area-painting JavaScript. Creating and inserting
new SVG elements also appears there. A separate 54.262 / 56.060 ms callback
contains 24.680 / 23.524 ms in Canvas `stroke` alone. Nested callback and wheel
durations overlap and must not be added.

Across the whole drill, Riemann spends approximately 126 / 126 ms of sampled
self time in area-painting JavaScript, 161 / 164 ms in `stroke`, 98 / 93 ms in
`fill`, 130 / 151 ms in React DOM and 90 / 75 ms in reference emission. These
are distinct sample categories across multiple actions, not the decomposition
of one long task. Automation's `elementFromPoint` adds 61 / 51 ms and is not
assigned to application rendering.

EthLib has a different remaining cost: **style recalculation as the declaration
layer appears**. Its document-thread `UpdateLayoutTree` takes 500.529 / 415.074 ms
across the whole drill, with individual maxima of 48.494 / 26.119 ms. React DOM
self time is 175 / 180 ms. Reference emission contributes only 13 / 14 ms in this
workload; blaming the former global-reference implementation would misdescribe
the current capture. No long task is reported in the second run, but its 150 ms
frame gap still prevents a claim that rendering is always smooth. This trace
does not identify the precise CSS selector or compositor cause of that gap.

The implementation agrees with the observed work:

- [Map interaction targets](../../apps/web/src/features/native/NativeWorkspaceGraph.tsx)
  and [declaration elements](../../apps/web/src/features/native/MapSceneElements.tsx)
  mount when semantic-zoom thresholds expose detail. DOM construction, property
  updates and style processing occur on the browser's main thread.
- [Area painting](../../apps/web/src/features/native/MapAreaCanvas.tsx) runs in a
  layout effect as the camera changes. Its OffscreenCanvas is owned by that same
  thread; it is not a dedicated rendering worker.
- Incoming details update [map projection](../../apps/web/src/features/native/project-map-model.ts)
  and [reference emission](../../apps/web/src/features/native/project-map-references.ts).
  They remain measurable, but do not explain all of the drawing/DOM cost.

All gesture windows send only read requests, with no commands, compilation or
model requests. Both settled graph and map idle windows have no observed long
tasks. Native status-computation p95 is 1.662 / 1.604 ms for EthLib and
1.670 / 1.485 ms for Riemann. These measurements put first-detail rendering
ahead of status polling as an interaction target.

## Cached file preparation is a separate wait

| Observation, runs 1 / 2                   |          EthLib |           Riemann |
| ----------------------------------------- | --------------: | ----------------: |
| File request to prepared status           | 3.053 / 3.099 s | 20.441 / 11.270 s |
| Prepared artifact cache hit               |       yes / yes |         yes / yes |
| Native analysis extraction                | 0.894 / 0.918 s | 19.356 / 10.415 s |
| Lake query/build/setup stage              |        not used |  10.857 / 3.924 s |
| Complete compiled-input fingerprint stage |        not used |   5.810 / 3.953 s |
| Navigation annotation load                | 0.709 / 0.691 s |   0.067 / 0.065 s |

Riemann validates **43,477 compiled inputs / 5,992,114,707 bytes**. Native samples
show SHA-256 in both the Python worker and Rust host, alongside Lake execution.
A retained compiler result avoids semantic recompilation but still checks its
original complete input binding. The large run-to-run difference is variation
in the same implementation, especially the Lake query; it is not an achieved
speedup. The worker log, original stage observations and CPU samples are retained.

EthLib performs two complete metadata validations, totaling 0.828 / 0.815 s,
and loads navigation annotations once. The overlay itself takes only
0.026 / 0.028 ms. Parent extraction/job intervals contain other listed stages;
do not sum those intervals as independent cost. Request-to-ready additionally
includes delivery and scheduling. It is not first graph visibility: a saved
source presentation can already be visible while current-input checks run.

Returning to an already-open tab is the separate fast path in the event table.
This audit excludes uncached semantic compilation/search, model-turn latency
and process-start discovery. The latter's separate measured path collection
remains in [the collection record](navigation-collection.md).

## Work justified by the measurements

1. Reduce synchronous first-exposure target mounting and area drawing. Retain
   keyboard accessibility, original coordinates, ordered layers and camera
   continuity. Moving Canvas alone cannot remove the independently measured DOM
   and style work; the earlier [Worker trial](worker-paint.md) did not improve
   worst stalls.
2. Isolate EthLib's declaration-layer style invalidation and the original
   main-thread tasks before changing selectors or rendering architecture.
3. Reduce repeated native resource/query/metadata preparation while preserving
   full dependency-content validation, failure and cancellation handling.
   Timestamps alone cannot replace those checks.
4. Keep saved-view readiness separate from current-input validation and reduce
   startup discovery separately. A cached map does not establish current proof
   evidence or make the serial command queue concurrent.

No estimated savings are reported as achieved improvements. No new dependency,
renderer, service configuration or verification rule is introduced by this audit.

## Metric boundaries

EventDispatch includes nested synchronous handlers; later React work and paint
are separate. Input to two animation callbacks is a frame scheduling proxy,
not physical presentation or field INP. Event Timing queue observations cover
reported discrete events only; missing entries are unavailable, not zero.
Wheel, pointer movement and resize do not have that queue measurement.

Native wall spans overlap with their children and must not be added together.
Host thread CPU excludes child workers. Browser samples estimate self time,
including native operations, garbage collection and automation; they do not
provide instruction-level accounting. Each measured action includes a deliberate
600 ms settling tail; whole-action elapsed time is not handler latency.

## Reproduction

All **37 required public TUI journeys** and **three focused native browser
journeys in each motion setting** pass on the exact deployed executable, with
no skips, flaky or unexpected cases. Decoded connection review, clipped approval,
source-failure/recovery, cancellation and historical/restart screens were
inspected, as were both actual-project map screenshots. The browser journeys
cover retained source/camera/explanations without another inspection, small-map
retry and stale-response rejection, continuous cached declarations, large source
files and restart with fresh model approval. Their installed-Lean retained-tab
two-callback p95 is 29.5 ms with reduced motion and 30.7 ms with normal motion,
below the existing 250 ms gate. These are fixture presentation proxies, separate
from actual-project measurements and from live-model/cold-install validation.

Two trace-report unit checks also pass. All four native rings retain complete
capture intervals with zero evictions. Both actual hosts end idle and unapproved,
with original conversations, candidates and current source-publication revisions
unchanged. All seven served JavaScript bundles match the pinned deployment.

Use the current session URL, an idle host and exact matching immutable assets:

```sh
MEDEN_PROFILE_PID='<IDLE_GUI_PID>' MEDEN_GUI_URL='<CURRENT_GUI_URL>' \
  MEDEN_GUI_MOTION=no-preference MEDEN_GUI_ASSETS='<MATCHING_HIDDEN_MAP_BUILD>' \
  node tools/perf/profile-native.mjs /absolute/project selected/File.lean target/native-profile/new-events
```

Run projects sequentially into new output directories. See
[the profiling guide](../../tools/perf/README.md) for local artifacts and sampling
boundaries. This audit does not change application behavior or claim a speedup.
