# Current event costs after snapshot typing

The [complete measurements](../../results/snapshot-events-20261125.json)
identify two remaining delays: **first-detail project-map rendering blocks the
browser, and a newly requested cached Riemann inspection still validates about
6 GB of compiled inputs.** Normal graph hover, drag and zoom handlers are much
smaller. No product optimization is included in this audit.

These are actual deployed EthLib and Riemann GUIs running `ae9d29a`'s validated
native executable. One browser capture per project runs sequentially at
1440 × 960 in Chrome 149, normal motion, without throttling or concurrent builds,
tests, inference or another sampler. All seven served JavaScript bundles match
the existing hidden-source-map build byte for byte. The record pins the native
executable and original trace/profile digests. Raw captures, all-event CSV and
Markdown tables remain local under `target/native-profile/snapshot-events-20261125/`.

## Handling each interaction

Inclusive synchronous EventDispatch times in milliseconds. p95 refers only to
the recorded events in this single desktop capture.

| Action                      | Events per project | Riemann p95 | EthLib p95 |
| --------------------------- | -----------------: | ----------: | ---------: |
| Project-map overview wheel  |                 24 |       3.292 |      4.878 |
| Dependency-graph wheel      |                 24 |       0.560 |      0.978 |
| Hover pointer movement      |                 40 |       0.079 |      0.112 |
| Drag pointer movement       |                 31 |       0.323 |      0.340 |
| Retained file/map tab click |                 24 |      16.208 |      9.536 |
| Repeat selected-node click  |                  8 |      28.122 |     17.781 |
| Select a different node     |                  8 |      14.613 |      8.213 |

Window resizing delivers six / four coalesced events, with observed maximum
dispatch 1.146 / 1.398 ms. Explicit Fit View delivers one click, 1.179 / 1.041 ms;
one event is not a population percentile. The complete record covers all
13 windows, including idle graph/status polling, settled graph, map idle and
deep zoom. It retains every delivered event type, render-stage distribution and
reported discrete-event input-queue observation. Missing queue observations
are unavailable, not zero.

Click-to-two-animation-frame p95 for retained tabs is **39.6 / 27.6 ms**.
For different-node selection it is **22.2 / 10.1 ms**. These are presentation
proxies, not physical display completion or field INP. Hover, tab switching
and all 32 node selections preserve their graph cameras. The profiler checks
original source lines when compiler metadata reports one; some generated EthLib
declarations have no reported line. This does not test every possible combined
pointer/zoom gesture. None of these ordinary operation windows has a long task.

## Why first-detail zoom remains uneven

The separate drill follows real category controls, their deliberate focus
animations and two wheel events into a file. Its maximum wheel dispatch is
**109.610 ms in Riemann / 15.518 ms in EthLib**. Report these as observed maxima,
not stable percentiles. Across that whole multi-action window, Riemann has four
long tasks, maximum **156 ms**, and EthLib has four, maximum **63 ms**. Maximum
frame intervals reach 149.9 / 133.3 ms. The total action window includes settling
and automation; it is not event-handler latency.

Matching sampled CPU frames within the actual 109.610 ms Riemann wheel interval
include 27.900 ms in native `setAttribute`, 10.661 ms in React DOM `setProp`,
7.472 ms in `createElementNS`, 7.486 ms in area appearance generation and
6.372 ms in Canvas `fill`. A separate 60.433 ms callback includes 25.628 ms in
Canvas `stroke`. The original correlated trace intervals are retained; nested
FunctionCall/EventDispatch intervals cannot be added together.

The source explains those costs:

- [SVG interaction elements](../../apps/web/src/features/native/NativeWorkspaceGraph.tsx)
  mount as newly visible areas become operable and declarations become readable.
  Their DOM property updates run on the browser main thread.
- [Area painting](../../apps/web/src/features/native/MapAreaCanvas.tsx) runs in a
  main-thread layout effect. `OffscreenCanvas` here does not create a dedicated
  rendering worker. Every draw traverses visible areas and issues rectangle
  fill/stroke operations; first exposure also populates appearance entries.
- [Visibility traversal](../../apps/web/src/features/native/project-map-model.ts)
  already shares the exact scene/camera result between consumers, but revealing
  a dense branch still enumerates its children. Sharing this result does not
  remove the first traversal or drawing work.
- A changed file description also rebuilds the combined symbol projection in
  `projectMap`. Current reference candidates are retained when possible, but
  [ordered edge emission](../../apps/web/src/features/native/project-map-references.ts)
  still visits current symbols. Over the whole drill, sampled self time is
  84.843 ms in Riemann edge emission, and 41.652 ms in EthLib `projectMap` plus
  15.754 ms in edge emission. These phase totals are separate from the specific
  slow wheel interval.

The drill reads 10 / 8 already prepared map descriptions. It sends no commands,
compilation or model requests. Thus the synchronous drawing stall cannot be
attributed to waiting for Lean or an LLM. The graph and map idle windows record
no long tasks; native status computation p95 is only 1.322 / 1.360 ms.

## Why a cached file can still take seconds

In the serialized captures, file click to prepared status takes **12.286 s in
Riemann / 2.063 s in EthLib**, despite complete compiler/search/reading cache
hits. The native extraction spans are **11.198 s / 0.884 s**. No compiler,
graph extractor, counterexample search or meaning-rendering worker runs.

Riemann still performs the original offline Lake query/build check (**4.254 s**)
and complete compiled-input fingerprint (**3.918 s**). These are child spans of
extraction; do not add them to the extraction duration. The validator reads
43,477 compiled inputs / 5,992,114,707 bytes. Opening an already retained tab
uses a different path and does not repeat that preparation.

The separate [deployment record](snapshot-contexts.md) retains the actual
first preparation after the compiler/search definition change: 211.438 s in
Riemann / 51.061 s in EthLib. Riemann spends 121.573 s in bounded counterexample
search, 33.767 s in graph extraction and 10.515 s in meaning rendering; EthLib's
search and rendering take 19.403 s and 16.569 s. Those concurrent functional
observations are not a benchmark comparison with this serialized capture.
New bounded-search outcomes include actual timeouts, retained as inconclusive
observations rather than old verdicts. The original complete newly prepared
publication is reused after restart.

## Optimization order supported by the evidence

1. Bound first-detail work during map navigation: reduce newly mounted SVG work,
   batch or move area painting away from synchronous input, and avoid repeating
   whole-symbol projection for each small description response. Preserve original
   geometry, keyboard targets, ordered edges, source selection and camera.
2. Address repeated native preparation separately. Retain complete original
   dependency/content validation and failure/cancellation admission. Replacing
   content checks with modification times would change correctness, not just speed.
3. Keep expensive search/reading artifacts reusable and make future definition
   updates avoid unnecessary invalidation. Actual new methods must retain their
   own observations; old outputs cannot be stamped as new. Inspect bounded-search
   execution before changing its budget.

No estimated savings are claimed as achieved. These measurements do not include
chat/model-turn latency or cold installation. Profiling/automation add overhead,
sampled self time is not an exclusive wall-time decomposition, and native
current-thread CPU excludes child processes. Neither ring loses records in this
capture. The same exact executable already passed 382 Rust checks, all 35 public
TUI journeys with decoded screens inspected, and all 76 GUI journeys; this audit
adds no product-source change.

To repeat one capture against an idle existing host:

```sh
MEDEN_GUI_FILE='selected/File.lean' MEDEN_GUI_MOTION=no-preference \
  MEDEN_GUI_ASSETS='/matching/hidden-source-map-build' \
  node tools/gui/profile-interactions.mjs '<CURRENT_GUI_URL>' target/native-profile/new-events
```

Use fresh destinations and matching served assets, and run projects sequentially.
See [the profiling tools](../../tools/perf/README.md) for native sampling and
precise measurement boundaries. No normal-UI counters or remote telemetry are added.
