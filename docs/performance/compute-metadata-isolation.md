# Isolate input validation from project computation

The preceding [prepared-input comparison](lake-path-observations.md) exposed a
second cost: changing the input-validation Python forced a new graph, meaning
rendering and counterexample search, including for the frozen-source compiler.
The source of coupling was physical as well as diagnostic. Every project Lean
metaprogram could read the entire mounted analysis library, including that
Python file. Simply dropping it from a digest would omit an observable input.

The common compute library now contains all actual Lean modules and compute
scripts, including search resources. Its identity and compiled manifest bind
that complete set. Validation rejects unknown files/directories, symbolic links
and special entries. The metadata Python and its wrapper are absent from this
library and its manifest. The compiler and search descriptors also capture the
resource-selection and analysis-mount implementation. Analysis profiles also
omit the generic `/meden-worker` executable, whose embedded helper would expose
the same dependency through another path. The outer host launcher still closes
inherited capabilities before entering Bubblewrap; ordinary file/checker workers
retain their existing executable. There is one compute environment; splitting
roles does not permit each to resolve the same project under a different library.

Preparation first runs the actual offline Lake query in that compute environment.
Only complete, successful, bounded output is passed to a separate worker. That
worker receives exactly two trusted metadata files in a temporary read-only
library, a read-only workspace, isolated Python imports and the original root
bindings. It executes no project code or subprocess. It still decodes every
original setup record, resolves every input and hashes all its bytes with the
original ordered `meden.prepared-inputs.v2` fingerprint. Current results must
match before any cached compiler observation can be reused. No old observation
is relabeled, no weaker metadata-only cache key is introduced, and existing
accepted-contract/evidence controllers are unchanged.

`atlas.project.query` and `atlas.project.fingerprint` separately measure Lake
execution and metadata/content validation. Older traces combine them under
`atlas.project.prepare`. Both intervals include worker setup and child execution;
thread CPU excludes children, and nested spans must not be summed as exclusive
cost. A second confined worker has a startup cost; this separation alone does
not claim faster unchanged-input preparation.

## Controlled preparation measurements

The installed Riemann workload validates **43,477 files / 5,992,114,707 bytes**.
Two resource/input warmups are excluded, followed by eight interleaved actual
offline invocations, four per immutable executable. No build, functional test
or browser sampler runs concurrently; the existing GUI hosts remain idle and
unapproved. OS/project caches are retained. Every original fingerprint, file
count and byte count agrees, including the independently retained fingerprint
from the preceding implementation.

| Measurement                                                | Median wall time |
| ---------------------------------------------------------- | ---------------: |
| Combined query/validation implementation                   |          6.207 s |
| Separated query/validation implementation                  |          6.454 s |
| Actual Lake query stage in the separated implementation    |          3.326 s |
| Metadata/content worker in the separated implementation    |          3.033 s |
| Query decoding/path resolution inside that metadata worker |          0.556 s |
| Full-content hashing inside that metadata worker           |          2.246 s |

The separated implementation measures about **0.247 s / 4.0% more** in this
preparation comparison. The improvement is avoiding unrelated semantic recomputation after
a helper-only update, not reducing this steady preparation cost. The last two
rows are included in the metadata worker; medians across separate observations
do not add exactly to the complete preparation median. These are installed-tool
wall intervals, not browser paint, inference, cold installation or I/O-wait
counters. A concurrent functional preflight is retained separately and excluded
from this comparison.

## Update and failure coverage

The native regression builds two actual executables differing only in a comment
in the embedded metadata helper. It compiles a real Lean project whose Lake
configuration, compile-time metaprogram and imported initializer reject helper
and generic-worker visibility. After updating the executable and restarting, the complete source
view and every retained compiler/search/render object remain byte-identical.
Native spans still show the new query and full-input validator, while graph
extraction, search and rendering are absent. A real false theorem retains the
original fixed-seed counterexample. No model request or approval occurs.

The test builder uses a private target and cleans the workspace packages before
each variant. Reusing another source snapshot's Cargo dep-info initially returned
the previous executable; the test rejects equal executable hashes and verifies
the selected helper bytes are embedded. Another initial fixture run correctly
refused a manifest created after source capture. The final journey starts from
an actual public-CLI build, as the existing prepared-project journey does. These
failures were not bypassed by editing host configuration or weakening admission.

The prior monolithic executable fails the isolation oracle after valid public
GUI startup: the actual Lake configuration can see the helper. Separate native
coverage retains dependency-edit invalidation, real compilation failure, Retry
and restart; a failed query cannot dispatch the metadata validator. Python
protocol/filesystem tests retain complete-result/EOF bounds, original input
identity, full-byte mutation checks, deterministic parallel hashing, the
independent 3,000-case path oracle and depth-independent observation budget.
Exit-status admission now belongs to the host query stage, rather than a Python
subprocess fixture.

This is tested execution/cache separation, not a Lean refinement of Lake,
Python, the filesystem, mounting or the full session loop. Public toolchain/OS
and arbitrary project-filesystem attribution remain adapter obligations; these
checks do not prove observational equivalence of every possible Lean I/O program.
Prepared-snapshot
ownership, remaining session/job typing and the full retirement audit remain
open. The new compute namespace is prepared once; former monolithic observations
are not migrated into it. Cold installation, paid inference and large local-model
validation are separate obligations.

See [the complete validation record](../../results/compute-metadata-isolation-20261125.json)
for original samples, actual worker spans, positive/negative controls and public
failure/retry/restart evidence.
