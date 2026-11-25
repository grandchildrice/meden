# Display complete saved graphs before input revalidation

Baseline: `663367f`. This advances the prepared-view and latency exits in the
[typed-core migration](../architecture/typed-core-and-plugins.md).

A saved file opening previously waited for the complete current Lean input
observation before publishing its graph. Cached compilation/search/rendering
avoided semantic workers, but Lake's current configuration, build query and
complete dependency fingerprint remained in front of the first graph.

The application now distinguishes a complete saved presentation from a prepared
backend observation. `SavedSource` owns only the original plain source and
frozen publication bytes. It cannot return a backend snapshot; `Sources.file`
and `Sources.current` expose only completed preparation. Saved and prepared
views share the existing bounded retention owner, content identity and eviction
rules. Equal complete bytes promote without replacing the graph component or
camera. There is no second execution or evidence controller.

Lean's optional saved-view adapter reads an existing checksummed compiler,
search and rendering chain. It checks their original bodies, parent identities,
selected method definitions and exact selected source bytes. It starts no
compiler, search or rendering worker. Missing, incomplete, corrupt or changed
saved data falls through to ordinary preparation. The existing full pipeline
still checks current metadata, builds/resolves dependencies and checks complete
physical compiler inputs before publishing a prepared backend observation.

The UI shows the complete original graph with a compact **Checking saved
analysis** disclosure while that operation continues. Source selection and zoom
remain available; meaning submission remains unavailable. A failed or cancelled
preparation retires every alias of that saved observation. Original source and
the lexical reference graph remain readable beside the actual failure and
Retry. This is saved historical presentation, not a claim of current proof or
verification evidence.

The common view boundary works with any projection implementation; the native
composition in this slice remains Lean-specific. Kernel transitions, approval,
file policy, job admission, accepted meaning and evidence rules are unchanged.
The first updated opening prepares a new compiler-bound chain because the
captured compiler cache implementation changes. Search and rendering method
definitions remain unchanged; their artifacts bind the new compiler parent.
Original immutable artifacts are retained rather than relabeled.

## Regression journey

Both reduced and normal motion use actual offline Lean/Lake. The browser first
compiles a valid two-file project and restarts. A trusted host fixture holds only
the real prepared-Lake query launch; release invokes the original Bubblewrap
with unchanged arguments. The saved original declaration is visible before
release, while native status is still preparing and no backend input is exposed.
Its source line, quiet chat and camera survive completion with identical bytes
and the same graph DOM element. The real input query and fingerprint still run;
no semantic worker or model request occurs on this saved visit.

A second held visit changes only an imported definition after the saved graph
appears. Release produces a real Lean rejection. The old saved reference is
removed and returns HTTP 409; source/lexical inspection and failure logs remain.
Visible Retry creates another failed job; opening the valid dependency recovers
through actual preparation and cached restart without model approval. There is
no configuration edit that rescues the failed theorem.

The original sealed executable completes the prerequisite cold compilation and
then fails the new declaration-visibility condition while the query is held.
An initial pilot used an overly broad graph selector: it also matched the
intentionally retained lexical fallback after failure. Narrowing that assertion
to the compiled graph fixed the test without changing product behavior.

## Measurement scope

Actual Riemann and EthLib comparisons use separate old/new sealed executable and
asset bundles, a fresh host restart and browser for every trial, warm persistent
analysis caches, a 1440 × 960 viewport and no throttling. Two repetitions per
motion setting produce four observations per project/version. Measurements run
without another build, gate or sampler.

File click to the first visible original declaration plus two animation frames
is measured separately from the native completed-preparation outcome. This is
an automation visibility proxy, not a physical-presentation timestamp or INP.
Native timing records retain input-query/fingerprint and saved-view phases.
Complete final source/publication bytes, history and candidates must match;
each file click issues exactly one file inspection and no model request.
Fresh host restart prevents an earlier in-memory prepared file from appearing
as an artificial zero-latency baseline. Warm disk caches do not establish
cold-install, uncached compiler, live-model or local-inference performance.

## Validation

All 293 Rust unit/doc checks, 45 architecture checks, 12 source/Lake checks and
36 required public TUI journeys pass. Eight decoded TUI journeys and the saved
view/failure browser captures were inspected. Clippy, generated-schema
consistency, Rust/web formatting and Python lint pass.

The first complete browser run passed 78 of 79 cases. The existing large-source
IDE case read `sources.current` as soon as a compiled graph appeared. With the
new deliberately read-only saved view, that backend reference is correctly
absent until revalidation completes. The test now also waits for native idle
and a prepared current observation before asserting its complete backend source
and dependency edge. Its focused rerun passes against the same sealed binary;
TypeScript and the changed test's formatting also pass. These are 79 distinct
successful cases across the original 78 successes and the corrected case, not
a second whole-suite run or an automatic retry. The original failure is retained.
Both native executable-update conformance cases retain their original complete
compiler/search/rendering artifacts, and both saved-view motion cases still
observe actual input queries/fingerprints without semantic workers or inference.

## Actual-project observations

The [machine-readable result](../../results/saved-graph-display-20261125.json)
retains all individual trials, per-event timing aggregates, original artifact
bindings, validation provenance and hashes of the raw captures/drivers.

| Warm opening, median of four per version | Before | After | Change |
| --- | ---: | ---: | ---: |
| Riemann: first visible original graph node | 12.331 s | 1.351 s | 89.0% less display wait |
| EthLib: first visible original graph node | 1.991 s | 1.354 s | 32.0% less display wait |
| Riemann: completed input preparation | 12.838 s | 11.931 s | Separate completion observation |
| EthLib: completed input preparation | 2.019 s | 2.393 s | 0.374 s more completion wait |

Every new first-graph observation occurs while native preparation is still
running, with no prepared current backend. Full completion retains the exact
new source/publication and original history/candidates. No compilation, search
or rendering worker runs in these sixteen old/new warm trials. All final
reader, inventory, navigation and evidence views also match their retained
originals; both hosts finish idle and unapproved after the last measured restart.

| Native phase median | Riemann before | Riemann after | EthLib before | EthLib after |
| --- | ---: | ---: | ---: | ---: |
| Saved-view load and checks | — | 267 ms | — | 288 ms |
| Source/layout discovery | 1,284 ms | 1,289 ms | — | — |
| Lake build/input query | 4,200 ms | 3,793 ms | — | — |
| Complete Lake input fingerprint | 4,596 ms | 3,678 ms | — | — |
| Compiled-cache validation | 196 ms | 189 ms | 479 ms | 495 ms |
| Full canonical extraction/revalidation | 11,286 ms | 10,134 ms | 901 ms | 879 ms |
| Final native view materialization | 2.19 ms | 2.13 ms | 4.22 ms | 3.93 ms |

Native wall intervals overlap/nest; adding this table would double-count work.
Current-thread CPU observations exclude child-process CPU. The unchanged Lake
query/fingerprint algorithms still dominate Riemann's full completion. Their
observed old/new variation is not attributed to this display change. Saved
loading introduces real work ahead of that unchanged pipeline, especially
visible in EthLib's completion time. The tradeoff is earlier inspection, not
a claim of uniformly faster total preparation. Eliminating repeated saved/cache
reads remains a further optimization opportunity. Native materialization is
separate from browser polling/transfer/layout and the two-frame visibility proxy.

Initial preparation under the newly bound compiler method takes 197.558 s for
Riemann and 51.359 s for EthLib. These functional first-method observations are
separate from the warm comparison. Existing source, formal graph, reader,
inventory, navigation, evidence and physical inputs remain exact, except new
owned graph/publication identities and one recorded Riemann search diagnostic.
`PIW.AdelClassFin.DictGen.natCast_cancel` changes from wall-clock budget exhausted
to unsupported `MedenPlausible.Testable`; neither observation supplies a
counterexample verdict or proof. The complete before/after diagnostic is retained.
EthLib's search observations remain exact. The new complete artifact chain is
reused byte-for-byte on subsequent cached and restarted visits, including those
observations; the original immutable artifacts remain intact.
