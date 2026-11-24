# Stable SVG placement for project maps

The [numeric record](../../results/map-placement-20261124.json) compares the served sparse-index map with stable bounded SVG area parents. It retains real Riemann and EthLib events, exact matching source maps, original preparation logs and asset hashes. The native executable, compiler sources, graph/source/history/evidence bindings and world geometry are unchanged.

## Placement work

The former area list shared its SVG parent with the subsequent reference, declaration and main-target lists. React placement can search later uncommitted siblings before inserting a new area. A large visible directory produced substantial sampled time in `getHostSibling`. The renderer now keeps those four layers separate and groups areas by their original parent and blocks of at most 128 original siblings.

Group membership is prepared once, independently of visibility, camera and lazy descriptions. Flattening groups retains the exact original painter order. Panning does not put a retained file under a different parent; only visible areas are rendered. Geometry/cache format and authority are unchanged. The obsolete flat runtime projection is replaced, with no alternate renderer, worker or dependency. Batch-root, reference and declaration lists can still grow; this is not a general linear-time rendering guarantee.

## Actual deep zoom

Each pair clicks real category headers and zooms to the same original file. Chrome 149.0.7827.53, 1440×960, normal motion, no throttling, tracing and 1 ms CPU sampling. Captures are sequential diagnostic observations, not field latency guarantees.

| Project | Longest task before → after | Total long tasks before → after | Frame interval p95 before → after | Map reads before → after |
| ------- | --------------------------: | ------------------------------: | --------------------------------: | -----------------------: |
| riemann |            359.0 → 231.0 ms |               1204.0 → 957.0 ms |                    33.4 → 33.4 ms |                  10 → 10 |
| ethlib  |              57.0 → 58.0 ms |                273.0 → 327.0 ms |                    16.8 → 16.8 ms |                    0 → 0 |

Riemann sampled sibling-search self time was 210.916 ms before; after, no samples were attributed to that function. An absent sample is unavailable at this resolution, not zero work. DOM construction/property updates, reference lookup and GC remain visible in the profiles. Read counts and observed declarations may differ with scheduling, so task totals are not exclusive-stage causal estimates.

Before deployment, a candidate-asset diagnostic reached the same file and observed a 276.0 ms maximum task. That capture intercepted static assets locally while using the actual existing native API. Its runtime bytes match the subsequently gated assets; final results above use the actual served deployment.

## Interaction handlers

Inclusive synchronous EventDispatch p95 in milliseconds. Individual counts/distributions and separate input/two-rAF observations are in the record; all 13 phase traces and per-event CSVs stay local with committed digests. Two-rAF is not physical presentation or field INP.

| Action                 | Riemann | EthLib |
| ---------------------- | ------: | -----: |
| Project-map wheel      |   4.451 |  6.604 |
| Dependency-graph wheel |   1.038 |  0.936 |
| Hover                  |   0.099 |  0.140 |
| Pan                    |   0.269 |  0.341 |
| Resize                 |   0.997 |  0.754 |
| File/map tab click     |  16.523 | 11.099 |
| Distinct-node click    |  14.343 |  9.653 |
| Fit View               |   1.287 |  1.133 |

## Startup and preparation

Five full navigations use a fresh context and warm host/project/OS caches. Actual DOM attributes classify computed and stored geometry. The comparison uses the immediately prior baseline capture; it is not an interleaved startup experiment. Visits 2–5 input/two-rAF medians:

| Project | Prior assets | Current assets |
| ------- | -----------: | -------------: |
| riemann |    1279.6 ms |      1276.5 ms |
| ethlib  |    1901.2 ms |      1890.3 ms |

Initial inventory parsing/materialization and native preparation remain separate costs. The original preparation-log revisions and exact file/byte counts are retained. This rendering change does not replace compilation or complete-input validation with metadata checks. Broader preparation/job and remaining layout/retirement audits in architecture-plan steps 4 and 5 remain open.

## Validation

All 58 unit cases, 15 algorithm performance cases and 3,000 generated cases per map/reference/update corpus pass. Generated maps compare grouped output with the complete visible painter order, unique stable parents and the 128-file bound; a 400-file directory crosses batch boundaries through several cameras. The actual 10,001-file browser journey now selects a file through the keyboard and requires retained focus after lazy declarations arrive, in addition to camera/chat/restart checks.

The full 70-browser gate passes with no skips, flaky or unexpected cases. The
large-map journey additionally passes with normal-motion animation enabled,
including its keyboard-focus assertion. The public 35-journey TUI gate passes;
decoded Lake freshness failure and historical/cancel/restart screens were
inspected. Actual deployment checks compare original
source/graph/reader/inventory/history/evidence and model bindings, preserve the
selected source line and camera, keep chat quiet and end idle/unapproved. These
are installed-tool and scripted-inference checks, not actual-model or cold-install
validation.

```sh
npm run test:fuzz
npm run test:performance
node tools/gui/profile-interactions.mjs '<CURRENT_GUI_URL>' target/gui-profile/new-placement
node tools/gui/profile-map-load.mjs '<CURRENT_GUI_URL>' target/gui-profile/new-navigation
```

Use a current session URL, exact matching asset maps and idle hosts. The [profiling tools](../../tools/perf/README.md) document scope and raw outputs.
