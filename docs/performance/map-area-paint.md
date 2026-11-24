# Project-map area paint and retained targets

The [validation record](../../results/map-area-paint-20261125.json) retains
capture identities, public gates and geometry observations. This changes only
browser presentation. Native preparation and the original compiler inputs are
unchanged.

After [retaining reference indices](map-reference-updates.md), Riemann's deep
zoom still spent substantial time updating SVG properties and recalculating
styles across newly revealed files. Updating fewer React properties with an
inherited CSS variable or a changing stylesheet merely moved work into browser
style calculation. Those alternatives were rejected. A fresh original-asset control also retained
EthLib pauses under the current host, so the earlier two zero-long-task runs
cannot establish a universal zero-task baseline. Hidden editor work was checked
separately with an original-response browser regression.

Area backgrounds and headings now have one retained OffscreenCanvas painter,
directly linked to the visible Canvas. It uses the original
visible-area sequence, palette, opacity, world rectangles and camera. Corners,
strokes and headings retain their screen-pixel sizes; the backing surface follows
the actual device-pixel ratio. Resize and font readiness redraw the retained
view. Drawing stays on the window thread: there is no worker, message protocol
or per-frame bitmap-copy/transfer loop. Weak per-area appearance entries avoid repeatedly hashing names, and
ordinary parent status updates do not redraw an unchanged layer.

SVG keeps one transparent rectangular target per area, its complete accessible
name, keyboard order and original focus action. Its world bounds do not change
on zoom. Painted corners remain rounded; hit regions use the full rectangle.
Area targets retain their existing bounded sibling parents. Foreground
references, declarations and main-theorem pins keep their SVG geometry, source
bindings and painter order. The background never intercepts pointer input.
Keyboard focus is painted after actual focus/input observations; zoom does not
query focus styles for every frame. No new layout, compiler, authority decision,
worker, dependency or persistent cache format is introduced.

A DOM Canvas prototype still forced style calculation during font work.
[Chromium's implementation](https://chromium.googlesource.com/chromium/src/third_party/+/ab3a7379d1d7660e1f9b480b9fc07019e92c7f3c/blink/renderer/modules/canvas/canvas2d/canvas_rendering_context_2d.cc)
updates document styles before its same-font fast path. The
[standard OffscreenCanvas interface](https://html.spec.whatwg.org/multipage/canvas.html#the-offscreencanvas-interface)
separates drawing from a DOM element. Intermediate copy and ImageBitmap-transfer
prototypes added their own costs; neither remains in the runtime. Actual capture
results distinguish these experiments from the final served renderer.

Hidden file workspaces also stop parent-driven presentation updates. Their
source/read-model/job owners remain active above the retained component. The
file DOM, camera, selection and requirement draft survive; activation renders
the latest data and callbacks. This removes hidden input autosizing and graph
projection while the user browses the map. It does not pause jobs or accept stale
source/analysis bindings.

The former per-area SVG background/text painter is removed. There is one runtime
area painter and one interaction path, rather than selectable renderer modes or
a compatibility fallback. DOM structure is disposable presentation, separate
from versioned world geometry and revision-bound source observations.

## Conformance and regression

A public native-browser journey observes actual Canvas calls and pixels without
production instrumentation hooks. It checks every painted area's order, bounds,
opacity, radius, stroke and heading against retained interaction geometry. It also
checks declaration label/marker sizes, reference endpoints and main pins at the
initial camera, after eight real wheel gestures and after resize. The run uses
device-pixel ratio 2, checks painted keyboard focus, navigates a main pin and
reopens the actual native host without restored model approval.

A MutationObserver counts geometry/style writes to retained area elements during
those gestures. Its budget is zero. Running the same journey against the original
immutable deployed assets must fail that budget while retaining valid geometry;
a missing Canvas alone is not the negative control. A second original DOM-Canvas
control retains valid geometry and zero area writes, but must fail the new
zero DOM font/text/image-copy budget. The earlier control observes 94 retained
area writes; the DOM-Canvas control observes 9 font settings and 18 text draws
during eight wheel gestures. The native test launcher has
a test-only asset-directory input for this comparison. The profiler locates the
original rectangular bounds whether a target is an SVG group or a rectangle;
it never forces clicks through overlapping areas.

The retained-tab journey holds an actual Gamma navigation response while real
Alpha/Beta Lake modules compile and open. It first selects an actual declaration
and observes its requirement input. Applying the original response while the
editors are hidden reveals over 100 real reference lines. The old renderer reads
hidden textarea styles twice; the retained presentation must read none. This is
a functioning public editor and transfer, not an inert measurement fixture.

Zoom leaves chat, preparation job bindings and source outcomes unchanged and
sends no command or model request. Existing public small-map transfer
failure/retry, late-body, large-map lazy loading and TUI failure/restart checks
remain required. These are installed-tool/browser observations, not a renderer
proof, physical-display timing, live-model quality or cold-install validation.

## Measurements and limits

Repeated served captures and all thirteen operation windows are retained in the
validation record. The action windows include deliberate focus animation and
automation/settling. Sampled self time, nested browser rendering spans, long tasks
and input-to-two-frame observations are separate measures; they must not be
summed into an exclusive decomposition or called field INP.

Two normal-motion served captures per project/version use Chrome 149 at
1440×960 without throttling. All served JavaScript bytes match their hidden
source-map build; all 88 final assets match the complete browser gate. The
original served baseline is the preceding `0b6218e` deployment. Its complete
served bytes and identical native binary were checked before the experiments.
The same category/file targets, camera endpoints, area counts and visible
declarations remain: 26 in Riemann and 383 in EthLib. Scheduling can change detail
read counts; the complete captures retain those observations.

| Deep-zoom observation, runs 1 / 2 | Riemann before | Riemann after | EthLib before |  EthLib after |
| --------------------------------- | -------------: | ------------: | ------------: | ------------: |
| Tasks above 50 ms                 |          7 / 7 |         5 / 4 |         0 / 0 |         4 / 0 |
| Total long-task time (ms)         |      944 / 961 |     506 / 452 |         0 / 0 |       254 / 0 |
| Maximum long task (ms)            |      266 / 253 |     169 / 170 |   none / none |     70 / none |
| Frame-interval p95 (ms)           |    33.3 / 33.3 |   16.8 / 16.8 |   16.8 / 16.8 |   16.8 / 16.8 |
| Maximum frame gap (ms)            |  266.7 / 266.7 | 166.7 / 183.3 | 133.3 / 116.7 | 133.4 / 116.6 |
| Input-to-two-frame p95 (ms)       |  115.6 / 104.7 |   61.4 / 50.1 |   45.3 / 37.8 |   68.6 / 84.0 |
| Style recalculation total (ms)    |  406.4 / 399.1 | 139.6 / 137.0 | 421.9 / 436.4 | 462.1 / 376.9 |

Riemann's combined observed long-task time falls by 49.7% in these two runs.
This is a measured workload comparison, not a universal latency guarantee.
EthLib's earlier zero-task baseline is not stationary: a later same-native
original-asset diagnostic records five long tasks. That diagnostic uses static
asset substitution and is kept separately, rather than counted as another served
baseline. No overall EthLib speedup or universal zero-task behavior is claimed.

All thirteen operation windows and event dispatch percentiles are available in
[the CSV](../../results/map-area-events-20261125.csv) and the validation JSON.
Window/frame/long-task values repeat across a window's event rows; do not sum
them across those rows. No dispatch observed or a missing measurement is not
zero handler cost.

Startup observations in a fresh browser context retain host/project/OS caches:
ethlib: first scene/two-rAF 810.5 ms; visits 2–5 median 706.5 ms; riemann: first scene/two-rAF 1150.3 ms; visits 2–5 median 1071.4 ms. Visits 2–5 actually report cached geometry. These navigation-relative
DOM/two-frame proxies include inventory transfer and initial rendering, not
physical first paint or cold installation. There is no matching served startup
counterfactual, so no startup speedup is claimed. Total uncompressed JavaScript
changes from 2,415,736 to 2,418,192 bytes
(+2456 bytes); CSS changes by
79 bytes.
Browser/GPU peak memory and internal presentation buffers are not measured.

All 81 unit cases and 74 final browser journeys pass, together with four
normal-motion scenarios and both serialized retained-tab gates (navigation-reduce: 25.8 ms; navigation-no-preference: 27.3 ms;
250 ms scenario budget). The unchanged native binary retains the earlier
passing 35 public TUI journeys, decoded failure/stop/restart screens, schema and
Rust-format checks; unchanged pure algorithms retain 25 fuzz cases and 18
performance checks. The full browser gate is repeated after the final surface
and hidden-view changes. There are no skips or flaky successful gate results.
Expected negative controls are retained separately.

Both deployed workspaces retain their complete original source/graph/reader,
inventory, history, candidate/evidence and connection observations, and end idle
and unapproved. Native executable and thirteen compiler-captured sources remain
unchanged. Cache-hit fidelity opens still take riemann 15.316 s / ethlib 2.110 s;
these are unpaired validation observations, not a native preparation improvement.

The renderer reduces repeated area-property work. It does not remove new SVG
target placement, reference projection, rasterization, garbage collection or
native compilation/cache validation. Long tasks and frame gaps remain possible.
Broader typed session/backend/job identities and the complete retirement audit
also remain open.
