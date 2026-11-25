# Repeated area paint: measure before adding a cache

**Keep the current renderer.** The [scalar-frame cache trial](../../results/map-repaint-cache-rejected-20261126.patch)
removes repeated paint calls but does not establish shorter worst pauses in the
actual projects. Its [numeric record](../../results/map-repaint-cache-20261126.json)
retains the comparison and rejection. No application cache, painter, protocol,
backend or authority change is deployed.

A separate instrumented Riemann capture observes 107 background draws during the
real detail drill. Thirteen repeat the preceding rectangle/fill/stroke/text
commands under the same backing dimensions, transform and font-notification
revision. They account for 11,543 of 149,221 area draws; the largest duplicate
contains 9,208 areas. These are observed command streams, not a proof of physical
pixel equivalence. Instrumentation changes timing and is excluded from the
controlled comparison.

The trial retains only scalar paint inputs, path strings and a renderer token,
rather than source/semantic records. It skips a draw only when all represented
geometry, opacity, palette inputs, camera, dimensions, DPR, font and focus match.
A new paint invalidates the old stamp before touching the surface. Resize,
font notifications and context-restoration notifications invalidate it too.
The prototype's isolated browser journeys preserve foreground reference updates,
focused pixels, a real surface reset and notification-driven repaint. The
original assets reach the same foreground/pixel checks and fail the zero-extra-
paint condition. These checks establish the local behavior, not its performance
value or universal GPU/font recovery.

## Current event processing

The current renderer's two runs give these maximum document-thread event-dispatch
spans. Each cell lists run 1 / run 2 in milliseconds; these are not input-to-pixel
latencies. Work triggered after an event is reported separately as long tasks and
frame intervals below. All thirteen interaction windows, including three idle
windows, are retained in the numeric record.

| Interaction              |    Riemann (ms) |     EthLib (ms) |
| ------------------------ | --------------: | --------------: |
| Dependency graph wheel   |   1.886 / 2.052 |   1.489 / 2.218 |
| Hover                    |   0.422 / 0.332 |   0.958 / 1.106 |
| Pan                      |   0.788 / 0.674 |   0.749 / 1.254 |
| Resize                   |   0.938 / 0.993 |   4.783 / 0.984 |
| Tab switch               | 17.141 / 17.140 | 11.746 / 14.958 |
| Fit view                 |   1.486 / 1.828 |   1.260 / 1.150 |
| Node selection           | 31.920 / 29.907 | 21.657 / 17.608 |
| Different node selection | 21.432 / 13.969 | 17.336 / 14.783 |
| Project map wheel        |   3.892 / 4.272 |   5.508 / 4.968 |
| Project map detail drill | 97.920 / 98.100 | 21.470 / 16.814 |

The bottleneck is concentrated in detail exposure, rather than ordinary hover,
pan or wheel dispatch. Duplicate background work exists, but the cache comparison
below does not establish it as the cause of the worst pauses. Newly exposed
geometry and first paint remain priorities for the next trace analysis. These
browser observations do not identify complete native-worker CPU cost.

## Actual controlled observations

Each project runs current → cache → cache → current in fresh Chrome 149 contexts,
normal motion, 1440 × 960 and no throttling. The same native executable, workspace
and real APIs remain active. Only browser static responses are substituted,
retaining native security headers. No build, test, inference or another sampler
runs during these eight captures. All thirteen existing interaction windows run.

| Deep-drill observation, runs 1 / 2 | Riemann current |   Riemann cache |  EthLib current |    EthLib cache |
| ---------------------------------- | --------------: | --------------: | --------------: | --------------: |
| Maximum wheel dispatch (ms)        | 97.920 / 98.100 | 98.085 / 93.793 | 21.470 / 16.814 | 11.197 / 17.449 |
| Maximum long task (ms)             |       142 / 143 |       143 / 139 |       none / 61 |       none / 64 |
| Combined long-task time (ms)       |       202 / 203 |       202 / 253 |         0 / 292 |         0 / 244 |
| Maximum frame interval (ms)        |   133.3 / 133.3 |   133.4 / 133.3 |     150 / 116.7 |   116.8 / 133.3 |

Riemann's worst pause remains about 140 ms; combined long-task time rises to 253 ms in one cache run.
EthLib does not show a consistent worst-frame improvement. Two repetitions are
desktop diagnostics, not field percentiles. Callback and event spans overlap;
the full drill includes focus animation, detail arrival and settling. Native
rings may evict during repeated captures and are not used for complete native
execution/CPU attribution.

The added snapshot, comparison and invalidation code therefore stays outside the
runtime. Removing duplicate work alone is insufficient evidence of an end-to-end
improvement. Newly exposed geometry and its first paint remain substantial.
Native query/full-input validation and the remaining generic layout/collection,
navigation-annotation and retirement work still require completion under the
[original plan](../architecture/typed-core-and-plugins.md).

## Opt-in diagnostic

The existing profiler now supports `MEDEN_GUI_PAINT_OBSERVATIONS=1`:

```sh
MEDEN_GUI_FILE='selected/File.lean' MEDEN_GUI_MOTION=no-preference \
MEDEN_GUI_ASSETS='/matching/hidden-map/assets' \
MEDEN_GUI_PAINT_OBSERVATIONS=1 \
node tools/gui/profile-interactions.mjs "$MEDEN_GUI_URL" target/new-paint-diagnostic
```

It writes `area-paints.json` with phase, area count, font epoch and exact preceding-
command agreement. Complete command strings exist only transiently in that local
browser; they are not included in the output. The observer covers this painter's
rectangle/fill/stroke/text operations, clear arguments, backing dimensions and
transform. It is not an arbitrary Canvas equivalence checker. Its 10,000-frame
limit marks truncation and fails capture rather than reporting an empty or
complete result. The native profiling wrapper includes this optional file in its
checksummed manifest.

Run this separately from comparative timing captures. Normal profiling leaves
Canvas methods untouched; the normal application imports none of this diagnostic
code. The rejected prototype was type/unit/architecture/schema/format checked,
with three focused browser journeys and its real old-assets negative control.
The required 37 public TUI journeys complete on the unchanged executable; the
full prototype browser suite is interrupted after rejection; its partial results
are not counted as completed coverage. Detached test children are cleaned up
without touching the production services. Scripted/installed-tool checks are not
live-model quality, cold-install or new verification evidence.
