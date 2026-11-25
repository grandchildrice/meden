# Worker paint experiment and document-thread timings

**Retain the current area painter.** The Worker trial reduces observed total
long-task time, but does not establish a lower worst stall or frame gap. The
[complete comparison](../../results/worker-paint-20261125.json) and
[prototype patch](../../results/worker-paint-prototype-20261125.patch) retain the
experiment. No Worker or alternative painter is installed in the product.
This advances architecture-plan steps 2 and 5; generic backend observations,
native preparation waits and the complete retirement audit remain open.

## Actual comparison

Four fresh-browser Riemann captures run current → Worker → Worker → current,
at 1440 × 960 with Chrome 149, normal motion and no throttling. All 13 windows
run each time, with no concurrent build, test or other sampler. The native host,
compiled source/cache objects, drill coordinates and 26 visible declarations
remain the same. Preparation is a real cache hit. Gestures send no commands or
model requests. Only static responses in the automated browser use the alternate
build, retaining native security headers; native APIs and complete input checks
remain actual. Neither real service is reconfigured or restarted.

| Drill observation           | Current, first / last | Worker, first / second |
| --------------------------- | --------------------: | ---------------------: |
| Maximum wheel dispatch      |  151.638 / 153.238 ms |   132.806 / 137.010 ms |
| Tasks above 50 ms           |                10 / 9 |                  2 / 3 |
| Combined long-task duration |          791 / 699 ms |           286 / 331 ms |
| Maximum long task           |          204 / 210 ms |           213 / 226 ms |
| Maximum frame interval      |      199.9 / 200.0 ms |       199.9 / 216.7 ms |

Combined long-task duration falls by 58.6% in these captures, but worst pauses
remain about 200 ms. Two runs per strategy are diagnostic observations, not a
field percentile or a statistically established regression. Each drill has only
two wheel events. The whole window includes focus animation, description arrival,
automation and settling; its elapsed time is not one handler's latency.

The first trial copies full area objects to a Worker with one frame in flight
and one latest pending frame. Copying adds document-thread work. The compared
prototype instead retains area definitions by scene generation and transfers
numeric drawing buffers. It retains original order, palette, rectangles, opacity,
text and screen sizes. Worker fonts load separately from the document, following
the [CSS Font Loading standard](https://drafts.csswg.org/css-font-loading/).
All nine selected Roboto faces are observed loaded in both candidate captures.

Canvas pixels are nonblank. The settled 837 × 838 map region has zero differing
channels against the first current screenshot in all other captures. Hover/tab
cameras and complete source, graph, reader, inventory, evidence, history and
compiler/search/renderer objects remain exact. This does not prove intermediate
camera/paint combinations, keyboard focus, DPR variants, cancellation/unmount or
Worker-load failure recovery. Browser/GPU peak memory is unmeasured.

CPU samples still show visibility, reference emission, React scene updates,
DOM writes and garbage collection. Across the two candidate drill windows,
visibility self time is 91.975 / 96.722 ms; edge emission is 94.706 / 71.820 ms.
These phase totals are not an exclusive decomposition of the worst task.
Automation hit testing remains separately attributed. Reduce first-detail
SVG/visibility/reference work before adding an asynchronous production pipeline.

## Correct thread attribution

The previous profiler selected every trace interval in a timestamp window,
including simultaneous Worker callbacks. `interactionTraceWindow` now selects
only the document process/thread identified by that window's user-timing marks
and records the attribution. Missing, duplicated, malformed, reversed or
foreign-thread marks fail capture instead of yielding empty measurements.

The candidate traces contain 96 / 102 off-document FunctionCall intervals,
with inclusive durations 535.282 / 568.787 ms. The former collector would include
them in document totals; the corrected one excludes them. They run concurrently
and cannot be added to UI latency or wall time. Current controls have no such
FunctionCall intervals in the drill. Nested document spans also remain inclusive.

Two new Node regressions cover Worker/other-process overlap and invalid markers.
All 35 architecture/tool Node checks pass. All four captures use document-thread
selection. An earlier transfer diagnostic overlaps an extra TypeScript check
and is explicitly excluded from this comparison.
The final product profiler also completes all 13 windows against the actual
EthLib host, recording every document thread and retaining hover/tab cameras
without gesture commands. Both hosts retain their original source publication.

Product sources, schemas and all 88 served assets remain unchanged. The exact
native binary retains its prior 382 Rust checks, 35 public TUI journeys with
decoded failure/restart screens inspected, and 76 GUI journeys. These are prior
identified gates, not newly rerun Worker-production validation. Both hosts end
idle and unapproved. This is not live-model, cold-install or large-model validation.
Raw traces, profiles, screenshots and preparation logs remain private under
`target/native-profile/worker-paint-20261125/`; alternate assets and capture
drivers remain under `target/worker-paint-20261125/`, bound by recorded digests.
The product profiler has no UI timing annotations, Worker toggle or remote export.
