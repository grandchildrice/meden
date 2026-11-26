# Keep the camera when graph publications reissue declaration IDs

The native file viewer's shared graph component now retains its supplied user
camera and chosen center
when an updated graph gives the same source declaration a new ID. It resolves
an exact current ID/source/name first, then a unique source/name match. An ID
reused for another name cannot take over the selection. Ambiguous or removed
centers invalidate their retained camera; an automatic refit does not become a
new user gesture. The viewer still selects source locations from the current
declaration after an update.

Previously, local center state held only the old ID. Its disappearance selected
the default root, including after the user had centered another declaration.
Separately, the initial-viewport memo could retain its original `undefined`
value while React Flow remounted for a new root ID. A real wheel gesture from
zoom **0.290 to 0.410** was then undone to **0.290** by the update.

Center choice now retains its current ID and original source/name together.
The transient camera also records its current declaration ID. Camera restoration
is reevaluated for new declarations/root IDs, and programmatic adjustments only
update an already valid retained context. These are presentation identities, not
new source bindings, proof evidence or authority decisions. The L4SB layout,
ordered graph data, compiler/search/renderer ports and native publications are
unchanged. There is no new runtime mode or renderer.

## Outcome regressions

Six browser cases compile the actual shared `GraphView` with the installed React
Flow and application styles. Valid input revisions update node/edge IDs and
source lines. The driver uses actual pointer/wheel/double-click input; it never
sets a viewport. It checks the main center, an explicitly chosen dependency,
initial fitting without a user gesture, ambiguous source names, reused IDs and
an update scheduled two animation frames after a delivered wheel event.

All twenty sampled frames after that wheel-triggered update retain the user's
camera, and source selection uses the new line. The untouched original component
fails five cases after valid visible graph/gesture prerequisites, while its
initial-fit case passes. The tests also check that an ambiguous update cannot
turn its automatic refit into a user camera for a later larger graph.

This is isolated component evidence through its public input interface, not an
actual backend publication, physical presentation or live-model test. The test
harness initially had a collapsed graph container and an overly broad node/edge
selector; those attempts are excluded. The admitted driver requires a visible
root and uses actual React Flow node elements. Native/public failure, refresh,
retained-tab and restart checks remain separate.

## Reject latency shortcuts that do not help the actual workload

The [numeric record](../../results/graph-publication-camera-20261126.json) also
retains private-artifact hashes and observations for the following rejected
experiments. These do not establish a first-detail rendering speedup.

A separate instrumented Riemann capture retains an actual maximum background
frame with **12,511 areas**. Its ordered rectangle/fill/stroke/text calls are
replayed using the installed font. Fresh surfaces isolate each comparison; draw
and forced pixel readback are reported separately. This is not browser
presentation latency or a native worker benchmark.

| Same-frame replay                                    | Median draw plus readback |
| ---------------------------------------------------- | ------------------------: |
| Original stroke                                      |                  44.40 ms |
| Filled border ring                                   |                  62.45 ms |
| Original, separate state/path trial                  |                  43.60 ms |
| Repeated stroke settings hoisted                     |                  43.40 ms |
| A new `Path2D` for every area                        |                  48.55 ms |
| Already prepared screen paths; construction excluded |                  36.40 ms |

The border-ring output also differs at DPR 1 and 2. Hoisting settings does not
produce a meaningful gain. Local-size caching has only a small replay difference,
and both local/world-coordinate paths have rasterization differences. Prebuilt
paths give an upper bound that excludes their construction; they do not prove
faster first exposure.

An actual world-path-cache prototype is therefore compared with the original
painter in current/prototype/prototype/current order, using the same Riemann
host, APIs, matching hidden maps and thirteen operation windows in each fresh
browser. It preserves the reached route/declaration count, but does not shorten
the initial-detail pause:

| Detail drill, runs 1 / 2 |           Original |   World-path cache |
| ------------------------ | -----------------: | -----------------: |
| Maximum wheel dispatch   | 90.915 / 97.035 ms | 97.889 / 97.763 ms |
| Maximum long task        |       135 / 142 ms |       142 / 139 ms |

The [prototype patch](../../results/world-path-rejected-20261126.patch) stays
outside the runtime. Its full promotion/conformance gate is not claimed; the
comparison already rejects it for the targeted pause. Event spans and containing
long tasks overlap and must not be summed. The older SVG consolidation is also
[rejected](svg-local-transforms.md).

## Preserve complete input validation and the current reader bound

A separate actual confined Riemann preparation comparison tests four versus
eight hash readers. One warmup per variant is excluded; eight measured
invocations use current/prototype/prototype/current/current/prototype/prototype/
current order. There are no concurrent builds/tests/samplers in these comparisons;
ordinary desktop background activity and existing caches are not controlled.

Every invocation reads **43,477 files / 5,992,114,707 bytes**, returns the exact
same full-input fingerprint and uses the same public Python and compute-library
identity. No metadata-only shortcut, partial graph or evidence admission occurs.
The first diagnostic invocation omitted the now-required source-root field and
was refused before preparation; it is excluded from the valid comparison.

| Median, four measured invocations per variant | Four readers | Eight readers |
| --------------------------------------------- | -----------: | ------------: |
| Complete preparation                          |      6.855 s |       7.529 s |
| Metadata/content worker                       |      3.335 s |       4.022 s |

Eight readers are slower in this workload. Production retains four readers,
256-job batches, all byte reads and the original fingerprint format. The existing
independent byte/order oracle now exercises two, four and eight readers across
batch boundaries, forced out-of-order completion, Unicode paths and a multi-chunk
file. This adds coverage, not an eight-reader runtime option.

Initial map detail exposure and complete new-file preparation remain separate
performance work under the [migration plan](../architecture/migration-status.md).
The camera fix does not claim to eliminate their measured stalls or close any
kernel refinement gap.

## Validation and deployment

All 37 required public TUI journeys and 29 input-worker protocol/filesystem cases
pass. Both motion settings complete six component cases and four native journeys
(source-publication failure/retry/restart, retained tabs, changed compiled source,
and the large map): twenty browser cases without skips, flaky or unexpected
results. Three existing recorded-workspace cases also pass, covering pointer/
resize camera retention, dense-node hover and prototype-named declarations. These
are distinct contexts using the same renderer. The untouched component is the
five-failure negative control, not counted as passing coverage.

Decoded TUI connection, history/restart, freshness, repair/clipped approval,
cancellation and plain/finite-source recovery screens are inspected. Some sampled
input/footer/spinner fragments remain; there is no whole-renderer authenticity
claim. The 62 architecture/report checks, build/types, schemas, formatting and
Python lint pass. Installed tools and scripted fixtures are separate from
live-model and cold-install validation.

Both actual workspaces now serve the new browser assets with the identical
native executable. All 88 served assets per host match the reviewed build and
its hidden-map copy. Source, reader, inventory, navigation and evidence views
match all ten original publications before/after deployment; history/candidates
are retained, selected sources reuse their compiler artifacts, and hosts finish
idle and unapproved. The copied preflight initially required an old startup span
that had been evicted from Riemann's bounded metric ring. That diagnostic now
records observation availability explicitly; original data/cache checks are
unchanged, and missing spans are not treated as zero work.
