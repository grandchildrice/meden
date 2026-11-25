# Ordered sibling bounds for project maps

Baseline: `09af785`. Large directory queries now skip whole off-screen sibling
ranges instead of enqueuing every file. This is a presentation index, with no
compiler, authority, persistent geometry format or new dependency change.

A packed bounding tree covers contiguous blocks of 32 original siblings for
branches larger than 64 items. It emits intersecting items in their original
order, with closed rectangle and point boundaries. Smaller lists use direct
intersection checks. The original breadth-first opacity/detail rules still run
against current descriptions; the breadcrumb uses the same index for its center
point. The existing two-view cache still shares exact results among consumers.

The index belongs to the original immutable geometry/inventory. Description and
tracked-target updates reuse it; a new inventory constructs its own index.
Coordinates, seed, weights, ordering, colors, SVG parents, keyboard targets,
source bindings and world/cache revisions remain unchanged. The retired sibling
walk remains only in the test oracle as a measured counterfactual. There is one
runtime query path. Overlapping bounds or a fully visible directory can still
require linear work; this is not a universal logarithmic-time or zero-stall claim.

Generated rectangle cases cover original references/order, repeated rectangles,
edge/corner contacts and point queries. A work-count check ensures a narrow view
need not read every one of 30,000 items. Dense detail/target traces compare current
and previous scenes with an independent complete scan through index replacement.
All those generated corpora run with 3,000 cases, alongside existing world,
reference and cone-layout conformance. The new dense corpus needs the same
60-second corpus deadline as the existing large generators; its first 5-second
run timed out without an observed counterexample and is retained separately.

The 100,000-file performance gate uses six distinct cameras, exceeding the
retained two-view cache, and nine alternating pairs against the retired walk.
Its median ratio must stay below 0.02. Both restoring the retired walk and
removing the prepared index fail the actual ratio assertion. These are algorithm
measurements after preparation, separate from browser drawing and source checks.

The normal-motion browser driver originally sent wheel input 53.211 ms after
starting a 650 ms category fit. The installed d3 wheel handler interrupts that
transition, leaving the center at another category. It now waits for the actual
settled camera and verifies the intended breadcrumb before anchoring further
wheel input. Original geometry, file, keyboard, camera and quiet-chat assertions
remain. This changes the driver rather than overriding user input in the product.
An interrupted recovery with an empty report is not counted as a complete gate.

This remains part of the typed-core/plugin migration. Generic internal compiler
observations, native preparation waits and the complete retirement audit remain
open. No new Lean refinement, cold-install, live-model or large-model claim is made.

## Measured outcome and validation

The [complete record](../../results/map-sibling-bounds-20261125.json) binds the
actual source, binary, assets, raw captures and test results. On original complete
inventories and selected navigation overlays, nine interleaved algorithm pairs
retain exact geometry, visible items and breadcrumbs. Ten explicit cameras exceed
the two-view cache. Median time for 100 combined visibility/breadcrumb queries:

| Project                | Original |  Indexed |
| ---------------------- | -------: | -------: |
| Riemann, 175,187 files | 5.303 ms | 0.539 ms |
| EthLib, 2,517 files    | 0.408 ms | 0.682 ms |

Riemann improves by about 90% in this algorithm workload. EthLib has about
0.275 ms extra work per 100 queries; no algorithm speedup is claimed there.
Independent preparation medians are 569.426 / 558.379 ms for Riemann and
6.875 / 4.444 ms for EthLib, before/after. These noisy diagnostic preparation
observations are not evidence of a startup optimization.

Four real-browser captures per project run original → indexed → indexed →
original, with all 13 windows, normal motion, 1440 × 960 and no throttling.
The exact native host, source/cache checks and project data remain actual;
static response substitution selects each version with native security headers.
No test, build or other sampler runs concurrently with these captures.

| Deep-zoom observation        |  Riemann original |   Riemann indexed | EthLib original | EthLib indexed |
| ---------------------------- | ----------------: | ----------------: | --------------: | -------------: |
| Maximum wheel dispatch (ms)  | 101.038 / 110.348 | 111.465 / 103.830 | 16.287 / 16.622 | 7.694 / 18.125 |
| Maximum long task (ms)       |         143 / 155 |         156 / 148 |         59 / 59 |        59 / 65 |
| Combined long-task time (ms) |         277 / 322 |         217 / 210 |        289 / 59 |       59 / 293 |

Riemann's combined long-task time falls by 28.7% in these two captures per
version. Its maximum stall remains about 150 ms. EthLib's variable observations
do not establish a stable improvement. The entire drill includes animation,
description arrival, automation and settling; it is not one handler duration.
Only two wheel events occur per drill. All original drill steps, settled camera,
area coordinates and 26 / 358 visible declarations compare exactly. The selected
837 × 838 screenshot region has zero differing channels in every comparison.
No gesture command/model request occurs; hover/tab cameras remain unchanged.
Native span rings report no eviction during these captures. Remaining drawing,
reference projection and new SVG targets are separate costs.

Five navigations per project/version retain native/project/OS caches. Visits
2–5 actually report cached geometry. Their navigation-relative scene/two-frame
medians are 1130.4 → 1178.1 ms for Riemann and 767.8 → 742.5 ms for EthLib.
Static routing affects browser HTTP caching and adds overhead; these are neither
physical presentation nor field startup percentiles, and no startup speedup is
claimed. Browser/GPU peak memory is not measured.

All 85 unit checks, 28 fuzz checks (3,000 cases per generated property), 19
performance checks and 35 architecture/tool Node checks pass. The full 76-case
browser gate passes; after the driver correction, all four relevant normal-motion
journeys and the large-map reduced-motion journey pass again. The required public
TUI gate passes all 35 journeys; six complete decoded review/failure/repair/stop/
restart screens were inspected. Format, type and Python lint checks pass. The
original native executable and Rust/refinement sources remain unchanged; no new
kernel proof or Rust gate is claimed for this frontend-only change.

Both actual services run all 88 gated public assets, byte-identical to the matching
hidden-source-map build. After deployment and an actual restart, complete source,
graph, reader, inventory, navigation, history and evidence match their originals,
as do compiler/search/renderer cache chains. Each file is inspected once; returning
to its retained tab performs no new inspection. Nodes retain camera/chat and jump
to the original source line. Both hosts end idle and unapproved. The interrupted
validation and stopped-service restoration are recorded separately.
