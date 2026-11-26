# One complete lexical pass

Lean/Rust navigation previously masked comments and strings twice: once for
counts/imports and again for declaration names, positions and reference
candidates. The concrete source adapter now returns one `Indexed<Symbol>` from
one masking/line pass through the existing generic indexing port. The unused
tuple wrapper and separate production header/symbol entry points are removed.
The finite backend, shared controller and authority kernels are unchanged.

The generic collection records a fixed `navigation.index` span around the pure
indexer call, separately from reading, content validation and hashing. It carries
no filename or source labels. Wall time overlaps its worker/coordinator; summed
worker-thread CPU is not an additional elapsed-time component.

## Actual-source comparison

The [numeric record](../../results/lexical-index-20261126.json) retains the full
interleaved samples, input/executable/source hashes and validation scope. Complete
UTF-8 source bytes are copied from the actual published indexed entries only
after their sizes and SHA-256 values match the original inventory. The corpus
remains local. EthLib contributes all 2,517 indexed files (38,466,384 bytes).
Riemann contributes only three entries published as indexed (14,817 bytes) from
its 175,187-file inventory. Its private syntax cache can contain more records;
this is explicitly not a complete Riemann cache-population benchmark.

The diagnostic compares the independent original two-pass implementation with
the combined adapter. Twelve alternating in-memory samples exclude input I/O,
warmups and output comparison. Four native samples use the same actual confined
file worker and ordinary offline policy on a fresh private complete-text copy,
with the unchanged four-reader/batch/budget implementation. Setup, warmups and
post-run equivalence checks are outside each collection interval. The project
lease is not bypassed. No build, test or other sampler runs during measurement.
OS/tool caches remain warm; both implementations use the development profile.

| Median observation                              |  Original |  Combined |
| ----------------------------------------------- | --------: | --------: |
| EthLib standalone index plus drop               |   7.234 s |   5.411 s |
| EthLib native collection wall                   |   2.694 s |   2.249 s |
| EthLib summed index-thread CPU                  |   7.279 s |   5.521 s |
| Three Riemann files, standalone index plus drop |  3.094 ms |  2.469 ms |
| Three Riemann files, native collection wall     | 40.457 ms | 38.975 ms |

EthLib collection wall falls about 16.5%; its indexing CPU falls about 24.2%.
The small Riemann collection difference has only two observations per method and
does not establish a general loading improvement. Four readers explain why
summed indexing CPU exceeds collection wall time. Every native sample retains
exactly one CPU observation per original file. Lifetime ring eviction is reported
separately; incomplete current spans or unavailable CPU refuse the diagnostic.

These intervals measure source indexing/collection, not directory discovery,
inventory publication, browser rendering or a complete source-opening event.
They do not measure the multi-gigabyte prepared-input validation, first-detail
wheel stalls, compilation, proof, cold installation or model quality. The
[event audit](event-latency-latest-20261126.md) and
[style investigation](attribute-invalidation.md) retain those separate findings.

## Original observations and cache boundaries

The independent two-pass oracle exists only in test/diagnostic builds. It checks
all counts, import order and complete ordered symbol fields, including masked
reference candidates, namespaces, visibility and original line positions.
Keyword-only declarations still affect counts even when they have no named
symbol. Explicit malformed/Unicode/comment/modifier cases and 2,000 seeded
generated cases compare both implementations exactly; every actual-corpus
native result also checks file order, byte count, complete source digest and all
index fields against that original algorithm.

The lexical implementation identity changes honestly. Previously indexed entries
must be rebuilt once rather than relabeled under a new method. Compiler,
counterexample and English-rendering identities stay independent. The native
update journey builds two real executables, prepares real Lake/Plausible/GF
observations, restarts with the combined parser and requires full current-input
validation with byte-identical retained semantic artifacts. Its inventories must
retain every data field except the changed indexer/generation attribution.

## Deployed verification and remaining upgrade wait

Both actual hosts now use native executable `96871bc6c9d8ec85` with the original
88 browser assets. Five full source/reader/inventory/navigation/evidence bodies
per project preserve all original data. Inventory comparison permits only the
new indexer/generation attribution; navigation chunks bind the exact new parent
inventory. These two raw envelope digests are checked independently. Source,
reader and evidence bytes are identical. All 58 Riemann and 46 EthLib retained
compiler/search/render files, including their indices, are byte-identical.
Selected-file preparation reports cache hits; history/candidates stay unchanged
and neither host restores model approval.

The real Riemann upgrade exposes an additional wait: the new syntax-method
namespace must populate the large private cache. Its retained
`navigation.cache.prepare` completion takes **99.833 s**, inside a **107.278 s**
startup job. This is a single deployment observation with both hosts starting;
it is not the paired corpus benchmark or a first-visible-map measurement.
Earlier records have been evicted from its bounded metric ring, so a complete
per-file CPU decomposition or absence of earlier stages is not claimed. EthLib's
complete retained ring shows no compiler/search/render execution. The full
artifact-byte comparisons are independent of ring retention.

This upgrade cost and the separately measured prepared-input validation remain
wait targets. The source-index improvement does not make the whole project open
in 2.249 seconds. Former artifacts are not relabeled to conceal invalidation.

Validation passes 191 application tests, 64 architecture checks, 2,000 seeded
original-algorithm cases, the 37 required public TUI journeys and both motion
settings' map/source browser cases. Five additional real source/update runs
cover prepared Lake, Atlas compilation failure/recovery, declaration kinds and
lexical-source editing; one repeats the update after adding the complete-ring
guard. Nine decoded TUI journeys (21 observations), browser attachments and
source/map screenshots were inspected. Installed tools and scripted providers
are not live-model or cold-install validation.

This is a tested adapter simplification and measured indexing improvement, not
new extraction/refinement or completion of the full typed-core migration.
