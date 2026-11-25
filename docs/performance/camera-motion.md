# Mixed graph zoom and pointer regression

The [numeric record](../../results/camera-motion-20261126.json) records a new
public native-browser regression and a rejected target-construction experiment.
**There is no runtime or deployment change in this slice.** The reported camera
reset is not reproduced on the measured `77eeef6` implementation; this does not
establish that every device, graph and gesture is unaffected.

## Actual compiled graph input

The existing public retained-tab journey builds a real Lean/Lake fixture, opens
its compiled graph, selects a declaration and keeps the original source page.
It now alternates the pointer between the root node and graph background while
delivering 32 actual wheel events: ordinary wheel input and Control-wheel input
equivalent to the graph's pinch handling. No viewport is set by the test.

Animation-frame observations follow the live viewport element, including any
replacement. The test requires a single retained viewport, at least 12 observed
node entries, increasing zoom without a shrinking frame and no truncated trace.
Each wheel probe also checks the pointer anchor transformation within 1.5 CSS
pixels. Both normal and reduced motion pass: 15 node entries, one viewport and
maximum anchor error **0.02547 px** in each run. Observed frame counts are 336
and 399. These are frame/DOM observations, not physical-pixel or field latency
guarantees; the test does not represent every continuous trackpad gesture.

The journey then retains the camera/source through hover, resize, tab changes
and a held real navigation response, without another inspection. Its existing
retained-tab timing budget still runs independently of the mixed-gesture probe.
No inference is approved or requested.

The admitted fault control remounts React Flow on hover **only after the probe
starts**. Real build, selection and source preparation therefore succeed first.
The test observes 33 viewport elements and fails the single-viewport assertion.
An earlier unconditional remount prevented initial selection; that failure is
not counted as a camera-probe control. The mutation exists only in private
test assets; the production graph has no fault switch or additional path.

## Reject numeric SVG parsing

The first-detail map stall remains real. One additional capture of the unchanged
deployed Riemann GUI observes a **104.236 ms** deep-wheel dispatch and a
**149 ms** maximum long task. Native/browser traces, source maps and CPU samples
remain private, with hashes in the record. This is a diagnostic capture, not a
new optimization comparison or a population percentile.

A prototype replaced per-target cloning/numeric attribute writes with detached
SVG parsing in batches of at most 128. Only finite geometry entered markup;
literal names stayed in DOM attribute calls. Literal-path, keyboard, restart,
paint/focus and compiled-graph journeys passed, and JavaScript attribute writes
fell from eight to two per new target. That work count did **not** establish
better performance.

The isolated browser comparison evaluates both actual production `Targets`
implementations using 32 batches of 128 rectangles. It checks complete original
ordered SVG bytes, including quotes, control whitespace and Unicode names,
before reporting timings. After two warmup pairs, nine interleaved pairs have
construction medians **15.7 ms original / 26.6 ms prototype**, a ratio of
**1.694**. Fixture/constructor preparation is outside the construction interval;
DOM insertion is included. Retained updates are recorded separately. The probe
does not measure React, CSS/layout, paint or native preparation.

The parsing prototype is rejected. Its source is retained only as a private
measurement artifact; the original cloning implementation and eight-write
regression remain. There is no second runtime renderer or new dependency.

## Validation

Both motion variants pass the actual compiled-source/retained-tab journey without
skips or flaky cases. The armed fault fails the intended camera assertion after
valid preparation. All 37 required public TUI journeys pass on the exact deployed
native executable. Decoded review, clipped authority, source-failure/recovery and
restart screens were inspected. TypeScript, architecture and formatting checks
pass. Installed-tool/scripted fixtures are not live-model or cold-install
validation. No native/compiler/parser/kernel/wire change is made.

The original migration plan remains active. First-exposure rendering, full-refresh
costs, generic layout/cache orchestration, observation correspondence and remaining
retirement work are not completed by this regression and rejected experiment.
