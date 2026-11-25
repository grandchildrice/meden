# Share viewport visibility and omit inert map targets

Baseline: `9fa3ad0`. The [complete record](../../results/map-interaction-20261125.json)
retains phase measurements, trace/profile digests, validation and deployed fidelity.
The [preceding event audit](event-observations-20261125.md)
identified first-detail SVG/Canvas work on the main thread. This change removes
unneeded interaction elements and repeated visibility traversal; it does not
change the compiler, searcher, renderer, geometry format or coordinates.

SVG keeps rectangles whose opacity reaches the original pointer threshold
(`>= 0.5`). Canvas still paints the complete original faded-area sequence.
A focused rectangle remains mounted, accessible and keyboard-operable while
fading; after blur, an otherwise inert target can disappear. The original
128-sibling parent keys preserve retained target identity and painter order.
Foreground declarations, references and main pins keep their existing rules.

SVG, Canvas and prefetch independently subscribe to a viewport. A weakly owned
cache now shares the last two exact projections per immutable `MapScene`,
including the preceding view while subscriptions catch up. Keys compare all
viewport and size values; a new scene has its own entries. Results are read-only.
The bounded cache owns no source/geometry/artifact identity and is not the source
of truth for retained state. The complete scan remains an independent test oracle.
Existing appearance objects were already weakly cached; this change does not
add a second color/name cache or move drawing to a Worker.

## Actual-project diagnostic comparison

A diagnostic asset proxy serves each original bundle set against the same actual
Riemann native gateway, forwarding original bearer credentials and the gateway's
expected Host/Origin. This is not an inert backend or a native-origin-security
proof. Each run uses a fresh headless Chrome 149 at 1440 × 960, normal motion and
no throttling. No build, functional test or other sampler runs concurrently.
Original source data, declaration counts, complete drill actions and every
phase's starting/ending camera transforms agree. Original traces and matched
source-map digests are retained.

| First-detail multi-action phase                         |   Original |    Updated |
| ------------------------------------------------------- | ---------: | ---------: |
| Wheel dispatch observed maximum (two wheel events each) | 113.852 ms | 111.893 ms |
| Long tasks in the complete phase                        |          4 |          2 |
| Total observed long-task time                           |     452 ms |     219 ms |
| Longest task                                            |     178 ms |     156 ms |

This is one sequential diagnostic run per version. The reduced long-task total
is an observation, not a stable speedup estimate or field p95. The worst wheel
still takes about 112 ms; DOM creation and main-thread painting remain work.
Nested rendering/dispatch spans overlap and cannot be summed as exclusive cost.
Input-to-two-rAF remains a proxy, not physical paint or INP.

The initial target-pruning-only prototype did not reduce the worst wheel
(120.448 ms, seven long tasks). Its source path was replaced by the shared-view
implementation above; only diagnostic captures remain. This is why the final
comparison includes actual traversal reuse rather than treating element removal
alone as sufficient evidence of faster interaction.

## Outcome coverage

The native fading journey indexes 128 real Lean files. It uses ordinary wheel
input and real keyboard Tab/Enter, observes actual Canvas drawing at DPR 2,
retains a highlighted accessible focus during fading, and restarts without
model approval. The original deployed renderer reaches the same state but fails
the inert-target budget with 128 hidden SVG rectangles instead of zero. No model
request, paid inference or cold installation is claimed.

The existing retained-area journey still checks original screen geometry,
reference endpoints, constant screen sizes, nonblank paint, camera, chat, jobs,
zero retained geometry rewrites and restart. Canvas may now have more painted
areas than SVG targets: each ordered interactive/focused target must match its
original painted geometry, while omitted targets must belong to inert paint.
The focus oracle matches actual painted bounds rather than assuming paint and
DOM arrays have identical indices.

The 10,001-file journey's former driver used an inert rectangle to aim its first
wheel. It now reveals input-eligible targets with ordinary small wheel steps,
then follows the original file, declaration loading, failure/Retry and restart
journey. Its observable requirements are retained. Initial new-fixture attempts
used unsupported text files and then a collapsed dominant root; the final fixture
uses an actual nested Lean category, without editing host configuration or
relaxing admission.

The visibility-consumer operation budget rejects repeated traversal for cloned
identical views and checks size/scene invalidation against the independent scan.
Generated camera/update traces preserve complete original results and earlier
scenes. Browser measurements are separate from the comparative algorithm gate,
verification authority and the remaining native prepared-input wait.

The initial complete GUI run has 75 passes and one obsolete-driver failure.
After correcting that driver, all three affected journeys pass in a fresh run;
all 76 distinct journeys therefore have passing outcomes, rather than one clean
76-case run. Product runtime sources are unchanged between those runs. The
public TUI gate passes all 35 required installed-tool journeys; four decoded
failure/cancellation/repair/restart screen sequences were inspected. Unit tests
(82), generated corpus tests (25, with 3,000 runs per property) and the comparative
performance gate (18) pass. The original full-scan mutation fails its actual ratio
budget at 1.0015 > 0.35. The comparative gate ran alongside functional browser
recovery; it is not an isolated absolute timing measurement.

All 88 deployed assets match the clean tested build. Both actual projects retain
their complete original source DTO, reader, navigation and evidence observations,
plus exact checksummed compiler/search/rendering objects and graph revision,
before and after host restart. No semantic worker executes. Their file-click to
ready-DOM observations after restart are 12.281 s / 2.203 s for Riemann / EthLib;
these concurrent functional observations are not a paired latency benchmark.
Riemann's complete prepared-input query/validation remains a separate wait.
