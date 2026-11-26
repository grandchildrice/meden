# Attribute samples to individual input events

The local profiler already retains each dispatch duration, but its CPU hotspots
describe an entire operation window. A window can include many inputs,
animation callbacks, lazy data arrival and automation. Those totals cannot
explain one particular slow wheel by themselves.

`node tools/perf/dispatch-samples.mjs CAPTURE_DIRECTORY NEW_OUTPUT_DIRECTORY`
now produces a separate `samples.json` and `samples.csv` from the original
capture. Every dispatch retains its document process/thread, chronological
offset and original inclusive duration. Observed leaf samples are counted
inside its half-open timestamp interval. Repeated inputs and events with no
observed samples remain separate rows. Nested dispatches can share samples;
their counts and durations must not be added as exclusive costs.

This is offline postprocessing, with no application instrumentation, browser
launch, model request or remote export. It binds the original summary, trace,
profile and used source-map digests. The original marker/thread attribution and
complete dispatch vector must match. Lossy completion, changed observations,
unknown sample nodes and changed recorded maps refuse output before publication.
An existing output directory is never replaced. Maps absent from the original
capture's recorded set are not used; those locations keep their script name.

All 52 original profiles in the comparison below contain nonmonotonic deltas.
The tool reconstructs each timestamp together with its original node before
sorting the points. It does not reinterpret negative deltas as CPU durations.
Counts and shares are sampled observations, not exact milliseconds or CPU
coverage; no samples does not establish no work. Temporal overlap alone cannot
prove profile thread origin. The original document CDP session/file attribution
and clock alignment remain capture-adapter assumptions.

## Renew the Worker comparison after intervening optimizations

The [earlier Worker trial](worker-paint.md) predates retained visibility,
reference indexing and cheaper SVG target construction. A new comparison uses
the exact archived Worker prototype over the current source. It keeps the
original native executable, Riemann project and real APIs fixed. Four fresh
Chrome 149 contexts run current → Worker → Worker → current, at 1440 × 960,
normal motion and without throttling. Only static responses in the test browser
change; native security headers remain. There are no concurrent builds, tests,
inference or other samplers during capture. Installed project/OS caches remain.

All 88 current static assets match the deployed distribution. Each comparison
build has its own exact hidden source maps. Every capture completes all thirteen
interaction windows, reaches the same declarations and retains hover/tab
cameras. Gesture windows send no commands. Both Worker captures observe all nine
selected fonts loaded. The settled 837 × 838 map region has zero differing
channels across all four captures; this does not establish intermediate
camera/paint atomicity, keyboard/failure behavior or production readiness of
the asynchronous prototype.

| Detail drill, runs 1 / 2 |            Current |             Worker |
| ------------------------ | -----------------: | -----------------: |
| Maximum wheel dispatch   | 84.611 / 91.481 ms | 79.690 / 85.147 ms |
| Maximum long task        |       125 / 136 ms |       124 / 139 ms |
| Total long-task duration |       184 / 197 ms |       124 / 139 ms |
| Maximum frame interval   |   116.7 / 133.4 ms |   116.7 / 133.2 ms |

The Worker reduces total long-task duration, but worst-task and frame-gap
observations do not establish a consistent improvement. Each drill contains
only two wheel dispatches; these are controlled diagnostic observations, not
field percentiles. The candidate remains outside production. The original
painter, camera, graph data, native preparation and authority boundaries stay
unchanged. This is a scoped rejection on Riemann, not a new EthLib comparison.

The first comparison coordinator used an obsolete summary field and stopped
after its first successful browser capture. The corrected coordinator validates
and retains that original complete capture before running the remaining three;
it does not substitute a second control observation. Raw artifacts and their
hashes are retained in the [numeric record](../../results/dispatch-samples-20261126.json).

## What the individual slow wheel shows

The first current wheel takes 84.611 ms and contains 90 observed leaf samples.
Its largest entries include 12 `cloneNode` samples, 10 `setAttribute` samples
and 10 in `MapAreaTargets.update`. The other current capture and both Worker
captures also retain target-construction/attribute work in their slow wheel.
These counts point toward synchronous SVG construction; they are not precise
exclusive CPU times or a complete causal decomposition.

Canvas `stroke` still contributes substantial sampled time over the complete
current drill. Moving it off the document thread does not remove the separately
observed SVG work. This distinction supports investigating target creation and
insertion next, while preserving literal paths, target/focus lifetime, painter
order, original geometry and camera continuity. No unmeasured saving is claimed.

The 70 architecture/report checks include six new sampling/export cases and a
512-case independent point-count oracle. Actual offline postprocessing retains
all original dispatches across 52 windows. Required TUI and native browser
failure/restart, geometry and retained-tab gates run separately against the
unchanged production implementation; their decoded evidence is recorded.
Sampled cancellation output still contains a checking spinner and some composer
cells retain decoder fragments; no settled-every-frame rendering claim follows.
Scripted providers and installed tools do not establish live-model, cold-install,
large/local-model, confidentiality or complete-renderer assurance.
