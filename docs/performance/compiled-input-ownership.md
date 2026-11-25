# Lean-owned compiled inputs

Baseline: `fadc440`. This advances the typed ownership and materialized-view exits
in the [migration](../architecture/typed-core-and-plugins.md).

A display graph no longer carries executable compiled-module cache references.
Lean's snapshot owns either source-only data or one complete compilation. A
compilation contains its original graph, original source/module collection and
exactly one input mode: frozen content-bound modules or a prepared-Lake input
fingerprint. Source collection returns a distinct `Collected` value; it cannot
appear to be a completed compiler observation before compilation succeeds.

The compilation field is private. Native host composition reads its graph,
collection and fingerprint through immutable accessors. Backend-local search and
reading projection may decorate the graph, while execution bindings remain owned
by the original compilation. Frozen module identifiers have a private canonical
64-character lowercase hexadecimal type, separate from prepared input revisions.
Their original content framing and complete physical byte checks remain.

The retired independent graph/collection options and graph-contained module field
have no compatibility decoder. Mixed or incomplete input modes, unknown keys,
old field placements and malformed module identities are refused. Compiler and
search definitions bind the changed decoded schema and actual implementation;
new observations are prepared once rather than relabeling old artifacts. Source
body/digest framing and neutral source-view DTOs remain unchanged. Lake's graph
revision now explicitly binds the prepared fingerprint beside its original
collection instead of embedding that input in the collection's data structure.

Source selection still uses the original frozen Lake configuration, exact source
bytes and complete metadata observations. Native cache lookup, search, assessment
and repair retain original source/configuration/resource/dependency checks. Cache
restoration still compares its actual input mode and fingerprint; frozen search
still checks complete retained module bytes before and after execution. No
approval, file-policy, job or evidence controller is added or weakened.

Neutral source/view retention tests now supply their own source projection rather
than constructing incomplete Lean compilation fixtures. Type examples prevent
replacement of the private compilation. Codec tests exercise source-only, frozen
and prepared modes, absence of required compilation fields, mixed modes, retired
field locations and malformed module digests. Actual native cache journeys check
prepared-Lake ownership and frozen-module ownership, including recovery after
retained module bytes are corrupted. The same existing public repair, failure,
cancellation and restart journeys remain required.

The actual installed Lean/Lake dependency probe uses the sealed worker and the
new compilation owner. Changing only an imported `Base.lean` preserves selected
source/configuration/resource observations but is refused by the original prepared
fingerprint. No model or solver request is part of that diagnostic.

All 289 Rust unit/doc checks, 45 architecture checks, 12 source/Lake checks,
36 required public TUI journeys and 77 browser journeys pass. The browser gate
has no skipped or flaky outcomes. Eight decoded TUI journeys and the relevant
browser captures were inspected. Clippy, schema consistency, formatting and
Python lint pass; all 88 browser assets remain identical. The first lint run
caught two long evaluation-driver lines after updating its backend data access;
a local graph binding removes that duplication. Successful preceding gates were
retained, and the corrected lint and remaining journeys use the same sealed
executable. The separate evaluation denominator unit test also passes; it does
not exercise actual dataset/model quality.

Validation and deployed observations are recorded in the
[machine-readable result](../../results/compiled-input-ownership-20261125.json).
This is ownership/adapter work. It does not prove truthful filesystem observation,
atomic input snapshots, physical worker dispatch or native plugin implementations.
No core/kernel transition, runtime worker protocol, frontend source or browser
asset change is part of this slice. Native heterogeneous selection, remaining
full-input waits, browser stalls and retirement work remain open.

## Deployed observations

The actual Riemann `DictGen.lean` now owns a Lake input; EthLib gossip
`Parameters.lean` owns frozen modules. Complete formal source/graph, reader,
inventory, navigation, evidence, history and candidates retain their original
values, apart from owned graph/publication identities and the recorded search
diagnostic below. EthLib's original frozen-module identifier and complete
file/hash map remain exact. Neither graph contains execution resources and
neither collection retains an independent prepared-input option. Original
immutable artifacts are retained without relabeling.

Node selection reaches the original source line, preserves the map/file cameras
and leaves chat unchanged. Saved and restarted openings reuse the exact new
artifact chain and complete source view, including its new search result, with
no compilation, search or rendering worker. Both services finish idle without
restored model approval.

| File click to observed ready | First new-method preparation | Saved analysis | After actual restart |
| --------------------------- | ---------------------------: | -------------: | -------------------: |
| Riemann                     |                    198.725 s |       11.040 s |             12.263 s |
| EthLib                      |                     53.363 s |        1.979 s |              2.199 s |

These individual functional observations use 1440 × 960, normal motion and no
throttling, without another build, gate or sampler. Readiness is the observed
native outcome; visible graph/source and camera assertions follow it. This is
not a physical-presentation, percentile or old/new performance comparison.
Native full-input revalidation still accounts for substantial saved-opening
work; this type/ownership migration does not remove that wait.

One new Riemann search diagnostic differs: `PIW.AdelClassFin.mGen_of` remains
budget exhausted, with the per-declaration wall-clock diagnostic replacing the
file-budget diagnostic. Neither result supplies a counterexample verdict.
Complete before/after values are recorded. EthLib's search observations remain
exact. Actual-project fidelity probes send no model request and do not restore
verification or inference authority.
