# Deployed event latency, November 25

The [complete numeric record](../../results/event-latency-current-20261125.json)
measures the currently deployed `ce0c88f` executable and unchanged browser assets.
**The largest repeatable interaction stall is first-detail project-map zoom;
requesting a cached file has a separate native input-validation wait.** This
measurement changes no product implementation, deployment or verification rule.

Two fresh-browser captures per project run sequentially, in Riemann/EthLib order,
with Chrome 149.0.7827.53, normal motion, a 1440 × 960 viewport and no throttling.
All seven JavaScript bundles match a hidden-source-map build byte for byte.
Each capture includes document-thread CDP events, sampled browser CPU, native
99 Hz user-space CPU samples, authenticated stage spans and original build logs.
No builds, tests, inference or another sampler run concurrently with captures.
Existing project, semantic and OS caches remain; these are diagnostic desktop
observations, not cold-install or field percentiles. Raw captures, per-event CSV,
profiles and screenshots remain local under
`target/native-profile/event-latency-current-20261125/`.

## Operation costs

Inclusive synchronous EventDispatch p95 in milliseconds, runs 1 / 2. This measures
the handler, including nested synchronous work, rather than later asynchronous
rendering. The full record covers all 13 windows and all delivered event types.

| Action                      |         Riemann |          EthLib | Events per run |
| --------------------------- | --------------: | --------------: | -------------: |
| Dependency-graph wheel      |   0.723 / 0.748 |   1.583 / 0.678 |             24 |
| Hover pointer movement      |   0.099 / 0.089 |   0.185 / 0.090 |             40 |
| Pan pointer movement        |   0.277 / 0.210 |   0.373 / 0.406 |             31 |
| Window resize               |   0.979 / 1.050 |   0.631 / 1.402 |            2–4 |
| Retained file/map tab click | 14.387 / 15.200 | 10.649 / 12.771 |             24 |
| Repeated same-node click    | 28.859 / 57.498 | 26.044 / 30.570 |              8 |
| Different-node click        | 14.615 / 16.727 |  8.359 / 10.162 |              8 |
| Explicit Fit View           |   0.687 / 1.249 |   0.942 / 1.521 |              1 |
| Project-map overview wheel  |   3.540 / 4.068 |   4.095 / 2.740 |             24 |

Resize deliveries coalesce; the single Fit View event is an individual observation,
not a meaningful population percentile. Reported discrete Event Timing entries
retain queue delay separately; missing entries are unavailable, not zero.

Tab click to two animation callbacks p95 is **42.5 / 40.6 ms** in Riemann and
**28.4 / 34.4 ms** in EthLib. Different-node selection is **24.7 / 25.4 ms** and
**16.2 / 14.8 ms**, respectively. These are scheduling/presentation proxies,
not physical paint or field INP. Riemann's second same-node window also contains
a 59 ms long task; a blanket claim that ordinary operations never stall would
be incorrect. Its 57.498 ms click needs separate popup/component attribution.
The whole-window CPU profile contains automation query work too, which must not
be assigned to the application's click handler.

All 64 node selections preserve their graph cameras, as do the hover and tab
windows. Original source lines are checked when compiler metadata supplies one;
these checks do not invent positions for generated declarations without a line.
The captures do not reproduce or rule out every simultaneous pointer/zoom reset.
Camera-reset correctness remains separate from event-speed measurement.

## The detail boundary blocks rendering

The drill follows real category controls and two additional wheel gestures until
a file and its declarations become visible. It includes intentional 650 ms focus
animations, lazy cached details, automation and settling. Its total elapsed time
is not a handler latency.

| Observation, runs 1 / 2       |              Riemann |             EthLib |
| ----------------------------- | -------------------: | -----------------: |
| Maximum deep-wheel dispatch   | 119.193 / 117.183 ms | 14.038 / 15.737 ms |
| Maximum long task             |         164 / 161 ms |         50 / 80 ms |
| Long-task count               |                2 / 3 |              1 / 3 |
| Total observed long-task time |         223 / 271 ms |        50 / 202 ms |

Riemann's actual slow-wheel interval includes **36.498 / 23.602 ms** of sampled
native `setAttribute` self time, **27.000 / 26.752 ms** in React DOM, and
**17.107 / 13.990 ms** in area-painting JavaScript. Additional work creates new
SVG elements and computes their appearance. A separate 53.850 / 55.063 ms
callback includes **22.415 / 25.753 ms** in Canvas `stroke` alone. These are
source-mapped point-sample estimates inside the original trace intervals,
not exclusive instruction-level accounting; nested callbacks must not be added
to their enclosing wheel event.

The implementation explains the boundary:

- [Map interaction geometry](../../apps/web/src/features/native/NativeWorkspaceGraph.tsx)
  and [SVG cards/declarations](../../apps/web/src/features/native/MapSceneElements.tsx)
  appear as detail thresholds are crossed. Mounting them and assigning DOM
  properties run synchronously on the main thread.
- [Area drawing](../../apps/web/src/features/native/MapAreaCanvas.tsx) runs in a
  layout effect and repaints visible areas for each camera update. Its
  `OffscreenCanvas` is owned by that same thread; the name does not imply a
  dedicated rendering worker. First exposure also computes cached appearances.
- Small detail responses still trigger
  [combined symbol projection](../../apps/web/src/features/native/project-map-model.ts)
  and [ordered reference emission](../../apps/web/src/features/native/project-map-references.ts).
  Across the complete drill, sampled self time is **92.980 / 79.038 ms** in
  Riemann reference emission. EthLib spends **46.353 / 49.292 ms** in map
  projection and **8.243 / 16.062 ms** in reference emission. These totals span
  the whole multi-action drill, not the single slow wheel event.

No gesture window sends a command, compilation request or model request.
Graph/map idle windows contain no long tasks. Native status-computation p95
is only **1.455 / 1.536 ms** in Riemann and **1.531 / 1.610 ms** in EthLib.
The measured deep-zoom stall is therefore browser work; accelerating Lean or
inference will not resolve that synchronous stall.

## Cached file preparation is another path

| Observation, runs 1 / 2                    |           Riemann |          EthLib |
| ------------------------------------------ | ----------------: | --------------: |
| File request to prepared status            | 18.401 / 13.330 s | 3.073 / 3.074 s |
| Complete compiler/search/reading cache hit |         yes / yes |       yes / yes |
| Native analysis extraction                 | 17.396 / 12.114 s | 0.924 / 0.864 s |
| Offline Lake query/build check             |   9.716 / 4.913 s |        not used |
| Complete compiled-input fingerprint stage  |   4.833 / 4.608 s |        not used |

Riemann's validator checks **43,477 files / 5,992,114,707 bytes**. Its original
worker log reports 3.366 / 3.156 s for byte validation; the larger fingerprint
stage additionally includes preparation/dispatch and other work. Native samples
identify SHA-256 in the Python worker and Rust host, plus Lake execution. Host
thread-CPU excludes those workers; sampling percentages do not decompose wall time.
Root selection adds 1.484 / 1.375 s inside extraction. All these spans overlap
with enclosing stages and must not be summed as independent costs.

EthLib performs two complete metadata validations of roughly 0.4–0.5 s each,
plus cache/source preparation. Its native job lasts 2.030 / 2.043 s; browser
request-to-ready also includes delivery and scheduling. No semantic compiler,
graph extraction, counterexample search or meaning-rendering worker executes
in any of these four cache-hit preparations. Retained-tab navigation is the
separate fast path measured above. Saved presentation can be visible before
current-input validation finishes; prepared status is not first graph visibility.

The 5 s difference between Riemann repetitions is an observed variation in the
same implementation, particularly the Lake query. It is not an achieved speedup
or a controlled comparison of alternative implementations.

## Work supported by the measurements

1. Reduce first-exposure SVG mounting and synchronous area drawing at detail
   thresholds. Preserve keyboard targets, original coordinates, ordered layers
   and continuous camera position. Moving Canvas alone cannot remove the
   independently observed DOM cost; the previous
   [Worker experiment](worker-paint.md) did not improve worst stalls.
2. Bound combined symbol/reference work when each small description arrives,
   while retaining original ambiguity, privacy, ordering and generation checks.
3. Investigate repeated native query/resource/metadata preparation separately,
   preserving full dependency-content validation and failure/cancellation
   admission. Do not replace content checks with modification-time assumptions.
4. Profile the occasional same-node popup/component stall with correlated events
   before attributing all whole-window query time to application rendering.
   Settled status polling is a lower priority in these captures.

No predicted savings are presented as achieved. The audit excludes chat/model
turn latency and initial uncached compilation/search. All native rings retain
the complete capture with zero evictions. Sample attribution reconstructs and
stably sorts cumulative CDP sample timestamps before integrating nonnegative
intervals clipped to the original profile/event bounds; it excludes unsampled
trailing time and remains an approximation.

## Reproduction and validation

Use an idle host, its current session URL and exact matching immutable assets:

```sh
MEDEN_PROFILE_PID='<IDLE_GUI_PID>' MEDEN_GUI_URL='<CURRENT_GUI_URL>' \
  MEDEN_GUI_MOTION=no-preference MEDEN_GUI_ASSETS='<MATCHING_HIDDEN_MAP_BUILD>' \
  node tools/perf/profile-native.mjs /absolute/project selected/File.lean target/native-profile/new-events
```

Run projects sequentially and use new output directories. See
[the profiling guide](../../tools/perf/README.md) for metric boundaries and local
artifacts. Executable, asset, source-map and raw-capture hashes are retained in
the numeric record. No normal-UI annotations or remote telemetry are added.

On the same exact deployed executable, all **36 required public TUI journeys**
and **three focused native browser journeys in each motion setting** pass with
no skips or flaky cases. These cover retained tabs/source/camera, small-map
failure/retry/stale-response rejection, continuous cached declarations, large
source files and restart without renewed model approval. Serialized retained-tab
two-callback p95 is 39.8 ms with reduced motion and 31.0 ms with ordinary motion,
below the unchanged 250 ms gate. Two trace-report unit checks also pass.
Decoded TUI failure/recovery/historical-restart and fresh-review screens, both
actual-project maps and the retained source/graph screenshot were inspected.
Both actual hosts finish idle and unapproved, retaining the exact complete
original source publication, conversations and candidates. These installed-tool
and scripted-provider checks do not establish live-model quality, cold
installation or new verification evidence.
