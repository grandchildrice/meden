# Current event latency audit

The [numeric record](../../results/event-audit-20261125.json) measures the deployed Riemann and EthLib GUIs after the inventory and publication changes. **Deep zoom still blocks the browser main thread; cache-hit file preparation has a separate native cost.** This audit changes neither the application nor its deployment.

Two fresh-browser captures per project exercise 13 operation windows. Captures run sequentially, with normal motion, Chrome 149.0.7827.53, 1440×960 and no throttling. All seven served JavaScript bundles match the hidden-source-map build byte for byte. The native executable is pinned by SHA-256; its 13 compiler-captured source files are unchanged. Browser traces, CPU profiles, per-event CSV/Markdown tables, native samples and original preparation logs remain private under `target/native-profile/event-audit-20261125/`.

## Event handling

Inclusive synchronous EventDispatch p95 in milliseconds, shown separately for runs 1 / 2. This includes nested handling, rather than the complete asynchronous rendering pipeline.

| Operation                |         Riemann |          EthLib | Delivered events per run |
| ------------------------ | --------------: | --------------: | -----------------------: |
| Overview map wheel       |   3.928 / 4.229 |   6.323 / 5.929 |                       24 |
| Dependency graph wheel   |   0.871 / 0.974 |   0.965 / 0.868 |                       24 |
| Hover pointer movement   |   0.064 / 0.088 |   0.111 / 0.071 |                       40 |
| Pan pointer movement     |   0.231 / 0.186 |   0.343 / 0.472 |                       31 |
| Window resize            |   1.207 / 0.713 |   0.879 / 1.031 |                      2–8 |
| File/map tab click       | 12.308 / 12.031 | 10.093 / 10.025 |                       24 |
| Different-node selection | 13.303 / 13.232 |   8.167 / 6.802 |                        8 |
| Explicit Fit View        |   1.037 / 1.056 |   1.095 / 0.632 |                        1 |

Resize notifications can coalesce; a one-event Fit View observation is not a meaningful population percentile. The complete record also retains repeated same-node selection, the two idle graph windows and map idle. Event Timing at a 16 ms threshold separates input queue delay for reported discrete events. Missing observations are unavailable, not zero; continuous wheel/move/resize does not have that queue measurement.

The input-to-two-animation-frame proxy for tab switching was **32.1 / 32.6 ms** in Riemann and **26.7 / 27.9 ms** in EthLib. Different-node selection was **22.9 / 20.8 ms** and **20.8 / 14.7 ms**, respectively. These are frame proxies, not physical presentation or field INP. All 64 node selections retained their graph cameras. The 56 selections with compiler-provided source lines completed their original-line assertions; eight EthLib selections target `GossipsubParameters.mk.inj`, which has an original file but no reported line, so no line-jump result is claimed for those. The separate hover windows also retained their cameras. This does not test every declaration or simultaneous arbitrary cursor/zoom input.

## Where deep zoom stalls

The drill follows reachable category headers and two additional wheel gestures until it reaches the same file box in each project. It includes the deliberate 650 ms category-focus animation and later lazy declaration responses. Its total action duration includes automation and settling; it must not be reported as handler latency.

| Observation, runs 1 / 2       |              Riemann |             EthLib |
| ----------------------------- | -------------------: | -----------------: |
| Maximum long task             |         226 / 191 ms |         87 / 83 ms |
| Total observed long-task time |        930 / 1033 ms |       757 / 744 ms |
| Long-task count               |                8 / 9 |            13 / 12 |
| Frame-interval p95            |       33.3 / 33.4 ms |     33.3 / 33.3 ms |
| Deep wheel handler maximum    | 101.493 / 117.950 ms | 18.521 / 17.765 ms |
| Prepared map-detail reads     |              10 / 10 |              8 / 8 |

The CPU profiles identify two different costs:

- **Riemann: React/SVG updates during the animated zoom.** Sampled self time in React DOM was 582 / 603 ms across the complete drill; native `setAttribute` work added 160 / 179 ms, and garbage collection 133 / 111 ms. Map rendering contributed 147 / 115 ms. These self-time categories are distinct, but they are not an exclusive wall-time decomposition. Automation's `elementFromPoint` hit testing contributed about 100 ms and is not assigned to application rendering.
- **EthLib: global reference rebuilding when details arrive.** The reference module contributed 356 / 368 ms of sampled self time. The complete heading set contains 42,857 declarations. `projectMap` reuses geometry and sparse file descriptions, but a changed file still rebuilds the combined symbol map and calls `referenceEdges` over all current symbols. Eight small responses can therefore trigger repeated global work on the main thread. React DOM contributed another 193 / 176 ms.

The detail-response p95 was 69.4 / 42.9 ms for Riemann and 4.2 / 2.1 ms for EthLib. These are browser request-completion intervals, including scheduling, not isolated server execution. The corresponding browser tasks remain longer than the responses. Gestures sent no commands, compilation requests or model requests.

## File preparation and initial navigation

| Observation, runs 1 / 2                     |           Riemann |           EthLib |
| ------------------------------------------- | ----------------: | ---------------: |
| Explicit file inspection to prepared status | 13.302 / 12.308 s |  2.109 / 2.132 s |
| Compiler artifact cache hit                 |         yes / yes |        yes / yes |
| Native analysis extraction                  | 12.824 / 11.095 s |  0.897 / 0.895 s |
| Native status computation p95               |  1.581 / 1.551 ms | 1.485 / 1.483 ms |

Riemann's original preparation log separates **6.245 / 4.943 s** of Lake build/setup/input resolution from **2.925 / 2.398 s** of complete-byte validation: 43,477 compiled inputs, 5,992,114,707 bytes. Root/layout selection adds about 1.94 / 1.98 s. Native sampling identifies SHA-256 work in both the Python worker and Rust host, plus Lake execution. A cache hit reuses compiler/search artifacts while still validating their input binding. It does not mean a new file-inspection request is free. Already-open tab navigation is a separate, much faster path.

Five full navigations in a fresh context per project retain host/project/OS caches. Visits 2–5 have a navigation-relative scene/two-rAF median of **1074.8 ms** for Riemann and **672.1 ms** for EthLib. The actual DOM reports cached geometry on those visits; initial inventory transfer, validation and map construction remain. This is not cold installation or a measured optimization comparison.

## Priorities supported by these measurements

1. Reduce work during deep zoom: reuse stable SVG properties where possible and avoid repeatedly resolving the complete declaration set for each incoming detail file. Any incremental resolver must preserve ambiguity, private visibility, original ordered edges and stale-generation rejection. Moving preparation off the main thread is a separate option to measure, rather than an established speedup.
2. Investigate repeated native preparation and hashing before changing file-open behavior. Preserve complete input validation, original bindings and failure/cancellation admission. Replacing those checks with timestamps would change correctness, not merely performance.
3. Reduce unnecessary status-driven React work after the above. Settled polling still renders explorer/MUI components, but it produced no observed long tasks; native status p95 is about 1.5 ms. It is a lower priority than the measured 83–226 ms deep-zoom tasks and 12–13 s file preparation.

No estimated savings are presented as achieved improvements. Sampling, tracing and automation add overhead; native thread-CPU spans exclude child workers, nested spans overlap, and native sample percentages are not wall-time percentages. The second EthLib capture evicted 314 old bounded-ring records; the other captures report zero. Original span IDs, eviction observations and manifests are retained.

## Validation and reproduction

All 35 public TUI journeys and three focused browser journeys in each motion setting passed, with no skips or flaky cases. The browser cases cover retained source/camera with no new inspection, lazy declarations in a 10,001-file map, and continuous map navigation with quiet chat. Serialized retained-tab p95 was 29.6 ms with reduced motion and 35.4 ms with normal motion, below the existing 250 ms budget. Decoded Lake failure and failed/cancelled/historical restart screens were inspected. Actual deployed source/graph/reader/evidence/inventory bytes and conversation/candidate observations remain unchanged; both hosts end idle and unapproved. These installed-tool/scripted-fixture checks are not live-model or cold-install validation.

```sh
MEDEN_PROFILE_PID='<IDLE_GUI_PID>' MEDEN_GUI_URL='<CURRENT_GUI_URL>' \
  MEDEN_GUI_MOTION=no-preference MEDEN_GUI_ASSETS='<MATCHING_ASSET_ROOT>' \
  node tools/perf/profile-native.mjs /absolute/project selected/File.lean target/native-profile/new-capture
node tools/gui/profile-map-load.mjs '<CURRENT_GUI_URL>' target/native-profile/new-map-load
```

Use new output directories, matching served assets and idle hosts. See [the profiling tools](../../tools/perf/README.md) for exact sampling boundaries and local-only artifacts. No normal-UI annotations or telemetry are added.
