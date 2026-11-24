# Sparse project-map updates and event costs

The [numeric record](../../results/map-updates-20261124.json) captures real Riemann and EthLib events before and after separating the complete immutable map index from lazy file observations. Native binary, compiler inputs, world coordinates and approval rules are unchanged. Raw traces and original source-bearing inventories stay local.

## Each interaction

Synchronous inclusive EventDispatch p95 in milliseconds, Chrome 149.0.7827.53, 1440×960, normal motion, no throttling. These diagnostic sample percentiles include automation and CPU sampling overhead. The record retains counts, before/after distributions and input/two-rAF proxies separately; two-rAF is not physical presentation or field INP.

| Action                 | Riemann handler p95 | EthLib handler p95 |
| ---------------------- | ------------------: | -----------------: |
| Project-map wheel      |               4.451 |              2.974 |
| Dependency-graph wheel |               0.836 |              1.159 |
| Hover                  |               0.076 |              0.080 |
| Pan                    |               0.345 |              0.370 |
| Resize                 |               0.906 |              0.807 |
| File/map tab click     |              14.246 |             10.401 |
| Distinct-node click    |              12.986 |              8.018 |
| Fit View               |               0.951 |              0.959 |

Discrete Event Timing additionally observes input queues at a 16 ms threshold. Continuous wheel/move/resize events do not provide that queue metric, and an absent entry is unavailable rather than zero. Each capture has all 13 phases, chronological dispatches, rendering spans, sampled stacks and exact preparation logs. Stages overlap and must not be added as exclusive CPU.

## Why lazy zoom stalled

The former lazy-update path copied the complete inventory array and rebuilt its complete path/hierarchy index after incoming declaration chunks. In Riemann that inventory has 175,187 files. Geometry caching had already removed repeated treemap layout, but retained these allocations and main-thread scans. The prepared index is now shared for the original inventory; only loaded file descriptions and symbols are replaced. Metadata updates keep world coordinates and do not fit the camera. Earlier scenes remain unchanged. A replacement inventory cannot reuse former observations, and deferred completions recheck both refresh epoch and original publication.

The actual browser drill reaches the same file path in each version:

| Project | Longest task before → after | Total long tasks before → after | Frame interval p95 before → after | Map chunk reads before → after |
| ------- | --------------------------: | ------------------------------: | --------------------------------: | -----------------------------: |
| riemann |            362.0 → 359.0 ms |              1892.0 → 1204.0 ms |                    50.0 → 33.4 ms |                         9 → 10 |
| ethlib  |              55.0 → 57.0 ms |                218.0 → 273.0 ms |                    16.8 → 16.8 ms |                          0 → 0 |

These are sequential diagnostic captures, not a universal browser latency guarantee. Request counts and declarations loaded during the gesture can vary with scheduling. Remaining React reconciliation and reference resolution are retained in the sampled stacks. Updates still traverse loaded descriptions and rebuild loaded reference links when inputs change; global ambiguity makes blindly retaining unrelated old edges incorrect. Target-only highlighting reuses unchanged edges.

The longest Riemann stall is essentially unchanged. Sampled self time in React's
`getHostSibling` remains 204.7 → 210.9 ms during the drill, identifying DOM
reconciliation as a remaining target. The observations support less repeated
index work and fewer blocked frames, not elimination of every large stall.
EthLib has no lazy map reads in this drill and shows no corresponding benefit;
its long-task total increased in this capture.

## Isolated update budget

Nine alternating same-process observations use the same 175,187-file corpus and already prepared geometry. Complete output is compared before timing. Reconstructing the complete index took a median **150.353 ms**; a prepared-index update for one changed file took **0.031 ms**. This boundary excludes layout, rendering, native compilation and cumulative loaded-graph work. Forcing complete reconstruction into the sparse arm fails the <0.25 ratio assertion (0.954).

The complete private EthLib publication still produces exactly 212,283 ordered reference edges with original SHA-256 `e9cd425a9ebf8d032482e139a8a97cddceac63b52e1aa5a7eaaf381195f5a5f8`. No dependency resolver fallback or truncated graph is introduced.

## Startup and remaining waits

Five current navigations per project use a fresh browser context and warm native/project/OS caches. Actual DOM attributes identify computed versus cached geometry. Visits 2–5 have these input/two-rAF medians; the historical baseline is the prior capture of the identical baseline assets, not a new paired startup run.

| Project | Prior assets | Current assets |
| ------- | -----------: | -------------: |
| riemann |    1306.9 ms |      1279.6 ms |
| ethlib  |    1844.8 ms |      1901.2 ms |

Initial inventory transfer, JSON validation and materialization remain main-thread work. Native Lake build/query and exact compiled-input verification also remain: the prior Riemann log measured about 10.3 s and 5.1 s respectively. This frontend update does not accelerate compilation or weaken those input checks. Returning to a retained tab remains a distinct path with no new inspection; the current full browser gate exercises it. Broader orchestration/preparation and layout-retirement work in architecture-plan steps 4 and 5 is still open.

## Validation and reproduction

All 57 unit cases, 15 performance cases, 3,000 generated map/reference/update cases per corpus, the expected-failing full-index mutation, and the complete original EthLib graph pass. The public 35-journey TUI gate passes with decoded Lake freshness-failure and historical/cancel/restart screens inspected. All 70 browser journeys pass with zero skips, flaky or unexpected cases, including the real 10,001-file map and retained-tab/restart paths. Actual deployment checks preserve original source/graph/reader/inventory/history/evidence bindings, camera, selected source line and quiet chat; both model bindings remain unapproved. These are installed-tool/scripted-inference checks, not live-model or cold-install evidence.

```sh
npm run test:fuzz
npm run test:performance
MEDEN_PERF_MUTANT=full-index npm run test:performance # must fail
node tools/gui/profile-interactions.mjs '<CURRENT_GUI_URL>' target/gui-profile/new-events
node tools/gui/profile-map-load.mjs '<CURRENT_GUI_URL>' target/gui-profile/new-navigation
```

Use a current session URL and exact matching asset source maps; profile idle hosts serially. The [profiling tools](../../tools/perf/README.md) document observation boundaries and raw outputs. No profiler runs continuously in the product or sends remote analytics.
