# Deployed event timings after navigation cache extraction

The [numeric record](../../results/event-latency-latest-20261126.json) measures
`d09eb9e`, the currently deployed native executable, with its immutable browser
assets. EthLib and Riemann each have two sequential captures in fresh Chrome
149 browsers, at 1440 × 960 with normal motion and no throttling. All seven
JavaScript bundles match a separately built hidden-source-map copy byte for
byte. In-progress backend-policy changes are outside this deployed measurement.
There are no concurrent builds, tests, inference or other samplers during capture.
Existing project, compiler, search and OS caches remain available.

Each capture covers thirteen operation windows: initial and settled graph idle,
graph zoom, hover, pan, resize, retained-tab switching, Fit View, same-node and
different-node selection, map overview zoom, map detail drill and settled map
idle. Chronological dispatch durations for **every delivered event** are in each
private `summary.json`'s `dispatch_events`; `events.csv` and `events.md` contain
event-type summaries, alongside Chrome traces, CPU profiles,
native samples, stage spans and streamed preparation logs. The numeric record
retains their hashes under `target/native-profile/event-audit-latest-20261126/`.
This is an audit; application behavior and deployment are unchanged.

## Synchronous event handlers

Inclusive EventDispatch p95 in milliseconds, runs 1 / 2. Counts are actual
delivered events per capture. Deep-drill and Fit View have too few observations
for a meaningful population percentile and are reported separately.

| Operation                   |          EthLib |         Riemann | Events per capture |
| --------------------------- | --------------: | --------------: | -----------------: |
| Dependency-graph wheel      |   1.222 / 1.543 |   0.902 / 1.521 |                 24 |
| Hover pointer movement      |   0.057 / 0.089 |   0.059 / 0.059 |                 40 |
| Pan pointer movement        |   0.334 / 0.366 |   0.297 / 0.361 |                 31 |
| Window resize               |   1.102 / 1.428 |   1.180 / 0.889 |                2–5 |
| Retained file/map tab click | 13.618 / 13.061 | 17.141 / 18.862 |                 24 |
| Same-node click             | 17.710 / 19.698 | 29.470 / 41.684 |                  8 |
| Different-node click        |   9.095 / 7.652 | 14.875 / 24.062 |                  8 |
| Project-map overview wheel  |   5.781 / 5.063 |   3.386 / 3.034 |                 24 |

Fit View's single click takes 0.705 / 1.132 ms in EthLib and 1.078 / 1.122 ms
in Riemann. Resize delivery coalesces. Ordinary hover, pan and graph zoom do not
explain the reproducible deep-zoom stall.

Tab input to two animation callbacks has p95 29.0 / 33.8 ms in EthLib and
43.6 / 44.2 ms in Riemann. Different-node selection has p95 16.4 / 15.1 ms and
25.6 / 32.1 ms, respectively. All 64 node selections preserve their original
camera; hover and tab windows also preserve it. This does not rule out every
possible mixed-input camera bug.

## Detail exposure is the remaining rendering bottleneck

| Observation, runs 1 / 2              |                EthLib |            Riemann |
| ------------------------------------ | --------------------: | -----------------: |
| Maximum deep-wheel dispatch          |    17.627 / 16.710 ms | 92.092 / 91.118 ms |
| Maximum observed long task           | 61 ms / none observed |       137 / 136 ms |
| Long-task count in drill             |                 4 / 0 |              2 / 2 |
| Total drill long-task time           |            224 / 0 ms |       198 / 195 ms |
| Maximum style-recalculation interval |    41.376 / 29.123 ms | 33.437 / 33.490 ms |

Riemann's first-detail wheel is repeatably slow. Within those exact 92.092 /
91.118 ms intervals, sampled self time includes 21.546 / 20.509 ms in native
`setAttribute`, 21.330 / 18.303 ms in area-painting JavaScript, 7.478 / 13.072 ms
in map SVG/React code and 7.468 / 11.781 ms in `cloneNode`. A later callback takes
55.981 / 54.498 ms, including 23.468 / 25.628 ms of native Canvas `stroke`.
The callback nested inside the wheel is the same work and must not be added to
its parent's duration.

Across the complete Riemann drill, sampled `stroke` self time totals about
167 / 167 ms, React DOM 145 / 171 ms and area-painting JavaScript 133 / 130 ms.
Reference-edge emission accounts for 80 / 83 ms across multiple updates. These
are whole-window sample categories, not an exclusive decomposition of one event.
Automation's hit testing and idle time remain separate from application work.

EthLib's drill has lower synchronous wheel cost, but style recalculation can
still take 29–41 ms in one interval. React DOM contributes 169 / 173 ms of
sampled self time across the whole drill. A second run without reported long
tasks does not establish uniformly smooth presentation or identify the CSS
selector responsible for invalidation.

These observations match the implementation: visible declaration and reference
SVG elements are created as semantic zoom exposes them in
[NativeWorkspaceGraph](../../apps/web/src/features/native/NativeWorkspaceGraph.tsx)
and [MapSceneElements](../../apps/web/src/features/native/MapSceneElements.tsx).
[MapAreaCanvas](../../apps/web/src/features/native/MapAreaCanvas.tsx) paints in a
layout effect after camera updates. Its OffscreenCanvas still runs on the main
thread. Using that API alone does not move painting to a Worker.

Riemann's second selection run also contains 59 / 60 ms long tasks in the
same-node / different-node windows; the other selection captures contain none.
Selection therefore deserves secondary investigation, but the repeatable
91–92 ms detail wheel is a stronger first target.

## A cached file still waits for current-input validation

| Observation, runs 1 / 2                   |          EthLib |           Riemann |
| ----------------------------------------- | --------------: | ----------------: |
| File request to prepared status           | 3.055 / 3.050 s | 12.310 / 11.266 s |
| Compiler artifact cache hit               |       yes / yes |         yes / yes |
| Native analysis extraction                | 0.881 / 0.911 s | 11.093 / 10.625 s |
| Lake query/build/setup                    |        not used |   4.273 / 4.206 s |
| Complete compiled-input fingerprint       |        not used |   4.220 / 3.815 s |
| Navigation annotation load                | 0.712 / 0.746 s |   0.065 / 0.063 s |
| Metadata revalidation, two calls combined | 0.810 / 0.832 s |   0.685 / 0.769 s |

Riemann's original log confirms **43,477 compiled inputs / 5,992,114,707 bytes**
are validated. The fingerprint stage includes the complete worker/result
boundary, not just time spent hashing inside its Python worker. Native samples
show Lake and SHA-256 work. Host thread CPU excludes child workers and cannot
be used as their CPU accounting.

EthLib instead spends about 0.7 s loading navigation annotations. That span
includes backend/resource selection and cached-artifact revalidation and reads;
it is not a disk-only measurement. Applying the
loaded overlay takes only 0.025 / 0.025 ms; changing that overlay alone cannot
remove the measured load wait. A new browser/file request is distinct from
returning to a retained open tab. Saved source/graph presentation may already be
visible while validation runs: request-to-prepared time is not time to first
visible graph. Parent stages include nested children; do not sum table rows.

Settled graph and map idle windows have no observed long tasks and no commands.
Native status computation p95 is 1.586 / 1.634 ms in EthLib and 1.425 / 1.548 ms
in Riemann. Polling is not the leading measured cause of these stalls.

## Next work supported by the measurements

1. Reduce synchronous SVG creation/property work and Canvas drawing when details
   first appear. Preserve camera continuity, coordinates, ordered layers,
   readable labels and keyboard targets. Measure both dispatch and subsequent
   rendering; optimizing just the wheel handler can hide a later stall.
2. Isolate EthLib's declaration-layer style invalidation. Retain the original
   trace interval and compare actual rendering before changing CSS or the renderer.
3. Reduce redundant preparation around Lake query, complete input validation and
   annotation loading. Preserve content checks, cancellation and failure handling;
   metadata hints alone cannot establish current proof evidence.
4. Inspect intermittent node-selection work after the repeatable detail stall.
   Keep retained-tab recovery separate from new-file preparation.

The prior Worker and paint-cache trials are not assumed to solve these costs;
see the [rejected Worker trial](worker-paint.md). This audit does not establish
an achieved speedup or estimate savings from an untested replacement.

## Reproduction and limits

Use an idle host's current session URL and exact matching hidden maps:

```sh
MEDEN_PROFILE_PID='<GUI_PID>' MEDEN_GUI_URL='<CURRENT_GUI_URL>' \
  MEDEN_GUI_MOTION=no-preference MEDEN_GUI_ASSETS='<MATCHING_HIDDEN_MAP_BUILD>' \
  node tools/perf/profile-native.mjs /absolute/project selected/File.lean target/native-profile/new-events
```

Run captures sequentially into new directories. Native metrics have zero ring
evictions in all four captures. Both hosts finish idle and unapproved, with
original history, candidates and source publications unchanged. Raw artifacts
remain local; session URLs are not included in the committed record.

Synchronous dispatch excludes later asynchronous rendering. Two animation
callbacks are a scheduling proxy, not physical presentation or field INP.
Discrete Event Timing entries retain their observed queue/processing durations;
missing entries and continuous-event queue measurements remain unavailable.
Browser/native sampling perturbs timing. This workload excludes cold startup,
uncached compilation/search, model turns and cold installation. See the
[profiling guide](../../tools/perf/README.md) for all thirteen windows and exact
measurement boundaries.

All **37 required public TUI journeys** and **three focused public native browser
journeys in each motion setting** pass, with no skips, flaky or unexpected cases.
The browser gate serves the immutable deployed build rather than rebuilding the
in-progress working tree. Decoded connection review, historical/restart,
source-failure/recovery, cancellation, repair and clipped-review screens were
inspected, along with both actual-project map screenshots. Retained-tab fixture
p95 is 33.1 ms with reduced motion and 39.2 ms with normal motion, below its
250 ms gate. Two trace-report unit cases pass. These fixture checks are distinct
from the actual-project measurements, live inference and cold installation.
The numeric record retains validation and screen-review artifact hashes.
