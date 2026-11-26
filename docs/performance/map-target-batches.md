# Batch new SVG targets without replacing retained elements

The [individual dispatch analysis](dispatch-samples.md) found target cloning,
attribute setting and insertion inside Riemann's slow detail wheel. The renewed
Worker trial did not remove that work. This change keeps the current renderer
and changes only its disposable SVG target adapter.

Static, source-free rectangle templates now belong to their document. Templates
contain powers of two up to the existing 128-sibling batch, retaining at most
255 rectangles. A new target group clones the required fragments, assigns each
original path, label and world rectangle, then inserts the complete initial
batch once. A nonempty group keeps the former successor-based placement, so
partial arrivals never replace an unchanged or focused target. Templates retain
no workspace paths, observations or source records. There is no new dependency,
worker, layout, option or kernel/application protocol.

The duplicate `data-world-x/y` attributes were read only by tests. They are
removed; those tests now inspect the actual SVG `x/y/width/height` attributes.
Six dynamic attribute writes replace eight. Coordinates, namespace, accessible
names, pointer/keyboard behavior, roles, opacity, painter order and source
activation remain the original inputs. Cached geometry and compiler/search/
reading objects are unchanged.

## Actual-project comparison

The [numeric record](../../results/map-target-batches-20261126.json) retains
the original captures, source/asset hashes, observations and validation evidence.
Each project runs current → batch → batch → current in fresh Chrome 149
contexts, normal motion, 1440 × 960, without throttling. The native executable,
real APIs, projects and installed caches stay fixed. Only test-browser static
responses change, preserving native security headers. No concurrent build,
test, inference or another sampler runs during capture. Ordinary desktop
background activity is not controlled.

Riemann's two measured observations per implementation:

| Detail drill             | Current, first / last | Batch, first / second |
| ------------------------ | --------------------: | --------------------: |
| Maximum wheel dispatch   |   110.343 / 90.882 ms |    84.562 / 83.727 ms |
| Maximum long task        |          154 / 136 ms |          129 / 130 ms |
| Total long-task duration |          213 / 196 ms |          190 / 193 ms |

The observed wheel median falls from 100.613 to 84.145 ms, about 16.4%; the
worst-task median falls from 145 to 129.5 ms, about 10.7%. Both batch captures
are lower than both current captures for those maxima. This is a small
controlled desktop comparison, not a field percentile or universal guarantee.
The long task includes the wheel and subsequent work; these values are not
additive or physical input-to-paint measurements.

EthLib's initial comparison contains a larger candidate task, so a second
current/batch/batch/current sequence is retained instead of discarding it.
Across all four observations per implementation, wheel maxima have medians
17.403 / 16.745 ms. Longest-task observations are **60, 61, none, 67 ms** for
current and **70, 60, 65, 56 ms** for batch. Their medians are 60.5 / 62.5 ms,
including the no-long-task observation as zero in that diagnostic statistic.
No consistently shorter EthLib worst pause is established. Style recalculation,
asynchronous detail arrival and ordinary desktop variability remain separate;
this is not evidence of universal absence of regressions.

Every capture completes all thirteen windows and retains the same declarations,
drill route and hover/tab cameras. Gestures issue no commands. The settled
837 × 838 map region is pixel-identical across all four Riemann and eight
EthLib captures. Per-dispatch sample exports retain the original inputs rather
than attributing the complete drill's Canvas work to one wheel.

## Public regressions and controls

Literal-path, focus, geometry, saved-layout corruption/restart, large-map
refresh/failure, retained-tab and ordinary source-navigation journeys exercise
the original native entry point. Initial target writes are bounded by six;
the original assets complete their literal-name and keyboard/focus checks,
then fail that bound with eight writes.

A separate control removes only the duplicate attributes, keeping individual
cloning/insertion. It passes the original fading paint, target geometry,
keyboard activation and focused-element checks, then fails the new initial
batch observation. The candidate records 2- and 128-element initial sibling
commits; the control records 1 and 1. Both retain the same 129 painted areas
with 128 fading. This prevents attribute removal alone from satisfying the
batch regression. Partial arrivals remain intentionally separate operations.

The first batch assertion incorrectly expected all root targets in one initial
commit. The real initial one-pixel viewport can expose one root before resize;
that is a legitimate partial arrival. The check is moved to the existing real
128-sibling fading journey, while literal paths retain their own six-write and
focus checks. Product code and measured assets do not change in that correction.
An initial Vitest command also conflicts with its default minimum worker count;
it runs no tests. Explicit minimum/maximum worker settings produce the actual
87-test result. Failed attempts remain distinct from passed gates.

Full TUI and browser results, inspected decoded screens, complete original
publication/artifact comparisons and deployment checks are recorded with their
scope. Installed tools and scripted providers are not live-model, cold-install,
large/local-model, confidentiality or new refinement evidence. This does not
finish the remaining native-input waits, rendering work or migration exits.

The final checks pass 87 unit, 70 architecture/report, 28 generated-corpus and
19 performance cases, plus all 37 required TUI journeys. Both motion settings
complete seven relevant map/source journeys and the registered real-Z3 finite
source/repair/restart journey: sixteen browser cases without skips or flaky
results. Retained-tab two-animation-callback p95 is 36.2 / 29.5 ms against the
unchanged 250 ms fixture bound. This is separate from actual-project latency.
Decoded TUI stop, source recovery, stale input and fresh connection screens are
inspected; sampled composer/spinner fragments remain, without an every-frame
display-authenticity claim.

Both actual projects now serve the measured build's 88 static files with the
identical native executable. All ten complete source/reader/inventory/navigation/
evidence publications and all 104 compiler/search/reading files retain their
original bytes. History and candidates remain unchanged. Selected files reopen
with compiler-cache hits after real host restarts, and both hosts finish idle
without restored model approval. The retained geometry/layout version is unchanged.
