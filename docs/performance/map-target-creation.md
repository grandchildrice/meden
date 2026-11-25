# Project-map target creation

The [validation record](../../results/map-target-creation-20261126.json) retains
both experiments, exact assets, browser traces and the public regression gates.
This changes browser interaction-target construction; native preparation and the
original source, graph, geometry and evidence owners remain unchanged.

The [event audit](event-latency-current.md) identified the first-detail zoom as
Riemann's largest repeatable interaction stall. A separate instrumented capture
observes 9,174 newly mounted area targets in one update, with 9,208 area paths
painted during that frame. Final settled target counts hide this transient fanout.
The diagnostic hooks add overhead and are excluded from the performance comparison.

Each existing bounded sibling batch now owns one SVG parent and a disposable
rectangle adapter. It clones a fixed shape template, sets the original world
geometry/path/accessible name, and delegates pointer/keyboard activation to that
parent. Original path identity retains each element. Only changed geometry,
accessibility or labels are written afterward. Newly visible siblings are inserted
in original order without replacing or moving an unchanged target. React still
owns the batch lifetime, callbacks and scene. There is no new layout, worker,
dependency, persistent artifact or authority decision.

The former per-target React component is removed. Targets keep their original
rectangle, full name, tab order, click behavior and focused-element lifetime
through fading. Canvas, declarations, reference edges, target pins, screen-pixel
sizes and camera control use their existing implementations. Literal strings use
DOM attribute setters; they are not interpreted as markup.

## Actual comparison

Each project runs control → clone → clone → control in fresh Chrome 149 browser
contexts, at 1440 × 960 with normal motion and no throttling. Both builds use the
same native executable, original project and source/cache objects, with actual
native APIs.
Only static browser assets are substituted; native security headers are retained.
No build, test, inference or another sampler runs during captures. All thirteen
existing interaction windows execute, including real category/file controls and
wheel gestures. Browser traces remain document-thread scoped.

| Deep-detail observation, runs 1 / 2 |   Riemann control |    Riemann clone |  EthLib control |    EthLib clone |
| ----------------------------------- | ----------------: | ---------------: | --------------: | --------------: |
| Maximum wheel dispatch (ms)         | 120.596 / 108.953 | 99.294 / 104.387 | 20.569 / 27.826 | 20.656 / 17.355 |
| Maximum long task (ms)              |         166 / 154 |        144 / 149 |         70 / 60 |         61 / 57 |
| Combined long-task time (ms)        |         225 / 216 |        206 / 208 |       251 / 281 |       239 / 224 |
| Maximum frame interval (ms)         |         150 / 150 |    133.4 / 133.3 |   133.4 / 116.8 |   116.7 / 133.3 |

Riemann's wheel and worst-task observations improve in both clone captures.
EthLib's maximum frame gap does not establish an improvement. These are two
controlled desktop observations per build/project, not field percentiles or a
universal responsiveness guarantee. Focus animation, lazy detail delivery and
settling belong to the complete drill window; its elapsed time is not one event.
EthLib receives eight/seven detail responses in controls and eight/eight with the
clone. Complete final screenshots are pixel-identical across all four captures
of each project; cameras, visible declarations and source-navigation observations
also match. This does not prove every intermediate paint or every input device.

Some native diagnostic rings evict entries during repeated captures. Their spans
are not used for complete worker-execution or native CPU attribution. Browser
traces and request records remain independently complete, and no measured gesture
requests compilation, inference or another command. Native freshness waits are
not optimized or presented as improved by this change.

## Rejected fragment parser

The first prototype creates escaped SVG markup and parses new rectangles in
batches. It passes the initial geometry/focus smoke journeys but increases Riemann's maximum wheel
dispatch to 145.290/150.828 ms and maximum long task to 191/196 ms. Sampled
`createContextualFragment` work replaces the removed property-setting work.
The [prototype patch](../../results/map-target-parser-rejected-20261126.patch)
is retained as experiment evidence; no parser or alternative renderer remains
in the runtime.

## Regression and remaining work

The public literal-path journey checks quotes, angle brackets, ampersands,
Unicode, tabs, newlines and carriage returns. Original SVG names/paths remain
exact, keyboard activation retains the same focused element, and restart retains
navigation without model approval. Newly created targets have eight JavaScript
attribute writes rather than sixteen. The exact original assets must reach the
same geometry/focus checks and then fail that eight-write budget. This is an
actual negative control, not a missing-renderer or startup failure.

The initial 82-case browser run passes 81 cases. The main-target/source-refresh
journey reads a null current source during an idle gap before a queued inspection
starts, while a saved explanation is already visible. Its two refresh checkpoints
now require the complete changed source revision, with the existing operation
timeout; no product or performance budget changes. The original failure and
focused rerun remain separate records.

The candidate passes 85 unit checks, 55 architecture checks, type/schema/format
checks and all 37 required TUI journeys. The corrected source-refresh case passes
its focused rerun; the other 81 browser cases pass in the initial full run, with
no skipped or flaky cases. Serialized retained-tab p95 is 24.2 ms with reduced
motion and 33.2 ms with normal motion, below the unchanged 250 ms budget. Decoded
TUI failure/stop/restart and browser literal-path/fading-focus screens are inspected.

Deployment retains all five complete original publications in each actual
workspace, conversations, candidates and original compiler/search/rendering
objects. All 88 served assets match the measured build. Both hosts finish idle
and unapproved. Functional cache-hit preparation takes 16.323 s in Riemann and
3.148 s in EthLib; these unpaired observations do not establish a native speedup.

Existing public native journeys check fading focus, full painted/interactive
geometry at device-pixel ratio two, lazy descriptions, refresh/failure/retry,
source-line navigation, quiet chat, retained cameras and restart. Serialized tab
performance keeps its existing threshold. Required TUI journeys retain their
separate failure/stop/restart checks; scripted inference and installed tools are
not live-model quality or cold-install validation.

Background painting still traverses thousands of visible areas and the largest
Riemann task remains around 150 ms. Native query/full-content validation, generic
layout/collection and navigation-annotation ownership, and the final retirement
audit remain open in the [migration plan](../architecture/typed-core-and-plugins.md).
No new kernel, filesystem or complete-renderer proof is claimed.
