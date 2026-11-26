# Event latency and rejected SVG consolidation

Riemann's first-detail wheel takes 94–95 ms and overlaps 138–140 ms long tasks;
ordinary dependency-graph wheel dispatch takes under 1 ms in these captures.
Detail creation/painting and cached native preparation are separate bottlenecks.

The [numeric record](../../results/svg-local-20261126.json) also compares a smaller
declaration/reference SVG representation with the deployed `5771462`
implementation. It does not consistently shorten Riemann's first-detail pause
and increases EthLib's style-recalculation intervals. Keep the current renderer. The
[archived prototype](../../results/svg-local-rejected-20261126.patch) is outside
production; no new rendering option, dependency or deployment is introduced.

## Measured event processing

Both actual projects run baseline → candidate → candidate → baseline in fresh
Chrome 149 contexts, normal motion, 1440 × 960, with no throttling. All 88 baseline
assets match the immutable deployment; each variant has exact matching hidden
source maps. The native executable, workspaces and real APIs stay fixed. Only
static browser responses are substituted, retaining the native security headers.
No build, test, inference or second sampler runs during the eight captures.
Existing OS/project caches remain available; ordinary desktop background activity
is not controlled.

Each capture covers thirteen operation windows, including three idle windows.
The private `summary.json` retains every delivered event's dispatch duration and
the original document thread; `events.csv` and `events.md` summarize event types.
The numeric record retains
raw trace/profile hashes, per-window timings, native stages and the comparison.

Current renderer's synchronous dispatch p95, in milliseconds, runs 1 / 2:

| Interaction                |          EthLib |         Riemann |
| -------------------------- | --------------: | --------------: |
| Dependency-graph wheel     |   0.747 / 1.855 |   0.480 / 0.880 |
| Hover pointer movement     |   0.109 / 0.086 |   0.128 / 0.112 |
| Pan pointer movement       |   0.291 / 0.333 |   0.268 / 0.352 |
| Resize                     |   1.163 / 0.745 |   1.132 / 1.683 |
| Retained-tab click         |  12.669 / 9.288 | 17.389 / 16.155 |
| Same-node click            | 19.812 / 19.877 | 27.666 / 31.716 |
| Different-node click       |   9.131 / 7.048 | 16.182 / 19.777 |
| Project-map overview wheel |   4.862 / 5.025 |   3.757 / 2.944 |

Ordinary graph zoom, hover and pan are much cheaper than detail exposure.
Fit View's single click takes 1.121 / 1.118 ms in EthLib and 1.046 / 0.930 ms
in Riemann. Resize delivery coalesces. The drill has only two wheel events per
capture; the maxima below are observations, not population percentiles.

## Why fewer SVG writes did not solve the pause

The prototype removes one inner group per declaration, places its marker and
label in a shared inverse-zoom coordinate system, and uses non-scaling reference
strokes. It therefore replaces repeated label-coordinate/font and reference-width
writes with a declaration transform. Focus selectors exclude flattened markers
so they retain their original appearance. No node positions, source records,
semantic visibility rules or ordered layers are intentionally changed.

| Detail drill, runs 1 / 2         | Riemann current | Riemann prototype |  EthLib current |  EthLib prototype |
| -------------------------------- | --------------: | ----------------: | --------------: | ----------------: |
| Maximum wheel dispatch (ms)      | 95.036 / 94.396 |   97.175 / 88.258 | 18.308 / 18.188 |   19.642 / 17.325 |
| Maximum long task (ms)           |       140 / 138 |         141 / 130 |         61 / 56 |         122 / 120 |
| Total long-task time (ms)        |       201 / 197 |         199 / 190 |        288 / 56 |         541 / 610 |
| Maximum style recalculation (ms) | 33.486 / 33.051 |   33.181 / 30.578 | 46.923 / 38.144 | 110.610 / 110.817 |
| Maximum frame interval (ms)      |   133.4 / 133.4 |     133.4 / 116.8 |   116.7 / 133.4 |     133.3 / 116.7 |

One Riemann prototype run improves, the other does not. EthLib's longer style
intervals repeat in both prototype runs. These measurements reject the combined
change; they do not isolate which individual transform, stroke or selector change
causes the additional work.

The prototype's real native journey at DPR 2 preserves screen-space marker/text
geometry, reference endpoints, roles, focused pixels, keyboard activation and
restart. Two detail-wheel gestures over 400 retained labels and 197 references
produce 800 declaration-transform writes and zero label-coordinate/font or
reference-width writes. The original assets pass the same geometric and actual
zoom checks, then fail the zero-label-write condition with 2,400 writes. The
first added observation failed to change the camera after resize; the driver
was corrected to reacquire the canvas bounds before physical wheel input. That
failed attempt is excluded from passed coverage. A valid work-count reduction
therefore exists, but the actual-project comparison still rejects its adoption.

In the current Riemann renderer, the two approximately 95 ms wheel intervals
contain sampled self time of 15–18 ms in area-painting JavaScript, 13–15 ms in
native `setAttribute`, 12–15 ms in map SVG/React and 9–10 ms in `cloneNode`.
Across the full drill, native Canvas `stroke` accounts for 158–171 ms of sampled
self time. Consolidating declaration attributes leaves that background-paint work
in place. EthLib's first detail wheel instead contains about 9–10 ms of sampled
React DOM work; its later style recalculation is a separate interval.

The wheel callback can be nested inside the wheel event, and a long task can
contain that same event. Their durations must not be added. Full-drill sampled
self time is also distinct from one handler's latency. Two animation callbacks
and frame intervals are scheduling proxies, not physical presentation or INP.

## File preparation is a separate wait

All eight selected-file preparations are compiler-artifact cache hits. Current
EthLib requests reach prepared status in 3.026 / 3.066 s; Riemann takes
15.403 / 12.243 s. Riemann's Lake query stage takes 7.026 / 4.248 s and its
complete compiled-input fingerprint takes 4.392 / 3.860 s. EthLib's navigation
annotation load takes 0.678 / 0.748 s. These stages include their worker/boundary
costs, and parent stages contain children.

Saved views can appear before preparation completes. These are request-to-ready
timings, not first-visible-graph or retained-tab latency. Changing SVG attributes
cannot eliminate the native preparation stages. All native stage rings report
zero evictions during these captures; host-thread CPU still excludes child-worker
CPU and does not establish complete OS execution attribution.

The next rendering work should isolate actual Canvas border/path work and
EthLib style invalidation while preserving camera continuity, geometry, ordered
compositing and keyboard access. Native preparation requires a separate change
that preserves current-content binding and cancellation. The five-step migration
remains in progress; this rejected trial supplies no new proof or inference claim.

## Validation and retained deployment

All 37 required public TUI journeys pass. The original renderer's retained-tab,
screen-space geometry and large-map failure/refresh/restart journeys complete in
both motion settings: six browser cases, no skips, flaky or unexpected cases.
Retained-tab two-animation-callback p95 is 31.9 / 32.4 ms against the 250 ms
fixture gate. This is separate from actual-project latency. The first checker
orchestrator reaches its execution limit; its three reduced-motion cases finish
with a complete successful report, and only the remaining checks are resumed.
That child exit code is recorded as unobserved rather than invented as zero.

Decoded connection, historical/restart, Lake freshness, repair/clipped-review,
cancellation and plain/finite-source failure/recovery screens are inspected.
Some snapshots retain transient spinner/footer/input fragments; this is not a
whole-renderer or display-authenticity claim. Installed real tools and scripted
fixtures do not establish live-model quality or cold installation.

The 62 architecture/report checks, build/type checking, schemas and formatting
pass. Both original production hosts remain idle and unapproved. Their source,
reader, inventory, navigation and evidence publications match all ten original
views exactly, with history/candidates retained and compiler-artifact cache hits.
The final build and both hosts' 88 served assets match the pinned deployment;
the executable and services are unchanged. Raw artifacts stay local, with hashes
and complete validation results in the numeric record.
