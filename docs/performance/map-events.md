# Event measurements and versioned map geometry

The [numeric record](../../results/map-events-20261124.json) binds real Riemann (175,187 files) and EthLib (2,517 files) GUI observations to the same unchanged native binary and the exact validated browser assets. Captures use Chrome 149.0.7827.53, 1440×960, normal motion, no CPU throttling, local CDP traces and 1 ms CPU sampling. Native stage observations use the authenticated bounded timing query. Raw captures remain local under `target/gui-profile/event-layout-*`. No model approval, inference or project-source edit is performed.

## What each event costs

These are final synchronous EventDispatch p95 values, including nested handlers. Counts and chronological dispatches are in each capture’s `events.csv` and `summary.json`; `events.md` also separates inclusive layout, paint, animation-frame and timer work.

| Action                 | Riemann p95 (ms) | EthLib p95 (ms) |
| ---------------------- | ---------------: | --------------: |
| Project-map wheel      |            4.280 |           6.597 |
| Dependency-graph wheel |            1.156 |           0.873 |
| Hover                  |            0.103 |           0.172 |
| Pan                    |            0.284 |           0.321 |
| Resize                 |            0.855 |           1.181 |
| File/map tab click     |           16.336 |          11.480 |
| Distinct-node click    |           14.036 |           6.735 |
| Explicit Fit View      |            0.998 |           1.111 |

Discrete [Event Timing](https://www.w3.org/TR/event-timing/) entries additionally retain input queue delay and processing at a 16 ms threshold. They exclude continuous wheel/move/resize events; no entry means unavailable, not zero. Two-rAF values are frame proxies, not physical presentation or INP. Trace stages overlap and cannot be summed as exclusive CPU. CPU profiles retain idle/program/GC and unavailable source positions.

## Why deep zoom stalls

The profiler now drills through real category headers and wheel zoom until it reaches a file box, rather than treating an overview wheel as declaration loading. Both versions reach the same original file path. In Riemann, incoming declaration chunks previously rebuilt the complete layout/index on the main thread.

- riemann: longest observed task 509.0 → 380.0 ms; total long-task time 3878.0 → 2030.0 ms; frame-interval p95 100.0 → 66.6 ms. Map reads: 7 → 10.
- ethlib: longest observed task 64.0 → 52.0 ms; total long-task time 277.0 → 154.0 ms; frame-interval p95 16.8 → 16.8 ms. Map reads: 0 → 0.

The 175,187-file paired pure workload compares the current complete-recomputation counterfactual with prepared-geometry overlays on identical input. Nine alternating observations give medians **566.234 → 102.008 ms**, a ratio of 0.180. It is not browser-paint or old-production total latency. Forcing complete layout into the reuse arm fails the ratio gate (1.005 ≥ 0.5). Binary geometry is 6,317,968 bytes; pure decoding took 71.822 ms in that run, excluding storage and input construction.

Geometry now has an explicit algorithm/version/order contract and a bounded disposable cache. New descriptions reuse unchanged rectangle objects and rebuild the sparse symbol overlay. Initial construction avoids repeated ancestor string creation and unused full-world sets/map rewrites. Label updates do not refit the camera. This removes repeated layout computation; it does not remove all full-array/map traversal, React reconciliation or reference resolution. There is no new worker, WASM backend, renderer, kernel rule or dependency.

## Initial navigation and remaining cost

A separate driver captures five complete navigations in one fresh browser context. The native host and project/OS caches remain warm. Actual DOM attributes classify the first computed result and subsequent geometry hits. The values include inventory transfer, parsing/validation, input construction, initial scene and two-rAF; the one-second storage settling interval is outside the readiness measurement.

| Warm navigation median (visits 2–5) |  Original | First cache prototype |     Final |
| ----------------------------------- | --------: | --------------------: | --------: |
| riemann                             | 1219.9 ms |             1481.6 ms | 1366.8 ms |
| ethlib                              | 3157.2 ms |             3126.9 ms | 3107.5 ms |

The first cache prototype increased Riemann startup cost. Reusing common path prefixes and removing unused initial allocations reduces that overhead, but these samples do **not** establish a universal initial-load speedup. Canonical input validation/materialization remains on the main thread. EthLib’s complete-map startup CPU profiles also identify global reference resolution in `projectMap`, including repeated candidate filtering. Persisting rectangles cannot remove that work. These are the next measured browser targets, alongside remaining React reconciliation in deep zoom.

Native preparation is separate. The captured Riemann build log reports Lake build/setup/query work of about 10.3 s and exact verification of 43,477 compiled inputs / 5,992,114,707 bytes in about 5.1 s. Cache-hit file opening still runs this original preparation and complete-input check. The compiler, its 13 captured Rust sources and the executable are unchanged; no compilation-speed or confidentiality improvement is claimed. Returning to a retained tab avoids that work: the separate real Lean journey measured normal/reduced-motion two-rAF p95 of 30.0/31.2 ms with no new inspection commands.

## Validation and reproduction

The full 70-case GUI suite, required public 35-case TUI gate and both serialized motion budgets passed for the cache/overlay split. Decoded TUI failure/cancellation/restart screens were inspected. Equivalent trie construction was then checked against an independent weighted prefix enumeration, unchanged golden coordinates, all 51 unit cases and 3,000 generated corpus cases; four focused GUI journeys passed. Initial allocation cleanup retained those pure checks, the two storage/10,001-file browser journeys and deployed source/graph/reader/inventory/history/evidence fidelity for both projects. Actual restart retains fresh, unapproved model state and the original three public choices. Scripted fixture inference in gates is not live-model or cold-install validation.

```sh
npm run test:performance
MEDEN_PERF_MUTANT=full-layout npm run test:performance # must fail the layout ratio
node tools/gui/profile-interactions.mjs '<CURRENT_GUI_URL>' target/gui-profile/new-events
node tools/gui/profile-map-load.mjs '<CURRENT_GUI_URL>' target/gui-profile/new-map-load
```

Use a current host session URL after restart. Profiling writes local event tables, trace/CPU profiles and the exact original preparation-log revision; it adds no normal-UI annotations or remote telemetry. Capture comparable workloads serially and keep the deliberate animation/settling time separate from event latency.
