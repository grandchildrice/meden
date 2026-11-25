# Batch upcoming configuration reads without changing their namespace

Baseline: `e9cef51`. The preceding
[event measurements](generic-compiler-observations.md) identify layout discovery
as one contributor to Riemann's saved-analysis wait. Directory discovery and
individual configuration snapshots repeatedly start the same confined worker.

The source-selection adapter now observes the upcoming dependencies' parent
directories and configuration files in bounded batches. Each original request
still goes through the existing policy, canonical-resource and complete-output
checks. The original per-file, per-response, aggregate and directory budgets
remain. No successful check is retained across selection operations, and the
prepared compiler still builds/resolves inputs and hashes every original byte.

Prefetched file values stay in a private reader buffer. Each logical read
consumes its value into the original metadata map in the original package
evaluation order. Frozen Lake evaluation therefore receives the same file
collection as before; an upcoming sibling's configuration does not become
visible early. The directory helper accepts only needed directory paths rather
than fabricated content observations. Incomplete/invalid batches produce no
source selection. Metadata is still independently revalidated before reuse.

The superseded per-request discovery path is replaced, with no new dependency,
kernel effect, native schema or evidence-admission rule. The changed actual
selection implementation is captured by the compiler definition. Existing
semantic artifacts must not be stamped with that new definition.

## Controlled selection comparison

`profile-source-selection` calls the application's actual source/root-selection
service through the original offline file worker and frozen Lake evaluator. It
reports complete selection hashes, counts and native spans. It does not build
project modules, extract graphs, run search/inference or admit evidence.
Workspace/resource preparation and complete result hashing are outside the
selection interval. Project/OS caches remain present.

The workload is Riemann's original `DictGen.lean`: one selected source, ten
packages and twelve configuration observations containing 17,894 bytes.
The complete canonical selection has 31,655 bytes. A private hard-link mirror
keeps original input bytes while respecting the workspace lease; the diagnostic
does not write project files. Those shared inodes do not model concurrent host
mutation or establish an independent/cold installation. Complete observed
selection hashes must agree on every invocation.

After separate warmups, compare immutable executables in the fixed order
baseline/candidate/candidate/baseline/baseline/candidate/candidate/baseline.
No build, functional test or sampler runs concurrently. This isolates source
selection; it does not establish an end-to-end browser speedup or solve the
remaining Lake-query/full-input hashing wait.

| Four measured samples per implementation | Original selection | Batched configuration frontier |
| ---------------------------------------- | -----------------: | -----------------------------: |
| Median                                   |            1.484 s |                        1.139 s |
| Minimum                                  |            1.421 s |                        1.100 s |
| Maximum                                  |            1.572 s |                        1.152 s |
| Worker starts on every sample            |                 27 |                             16 |

The candidate's median is 23.2% lower for this selection workload. All ten
warmup/measured invocations return the same complete selection hash and reuse
the same twelve frozen configuration evaluations. The selected file worker is
held fixed across both diagnostic executables. An earlier prototype comparison
also reduced starts from 27 to 16; it is retained separately and is not mixed
with these final implementation samples.

## Regression scope

A real installed-Lake fixture independently reconstructs the full expected
source, module, package and metadata representation. Its original ten siblings
must reopen with the same complete hash and bounded worker starts. The retired
implementation preserves that complete result but starts 27 workers and fails
the 16-start budget. Invalid UTF-8, oversized individual configuration files
and an exhausted aggregate budget must return no complete or partial result
and leave project files unchanged.

The required public TUI Lake journey includes a configuration that rejects a
future sibling's presence in its actual frozen filesystem. A deliberately bad
executable that inserts prefetched values into published metadata fails there
with the actual Lean error. This distinguishes namespace isolation from merely
preserving the eventual metadata hash. The normal journey still covers stale
inputs, failed preparation, explicit recovery, repair and restart.

These are application/worker/cache tests, not a new Lean refinement of the
filesystem, batching, arbitrary project I/O or complete session loop. Compiler
output ownership, mixed-backend orchestration, remaining native waits and the
complete retirement audit in the architecture plan remain separate work.

## Validation and deployed observations

The [complete record](../../results/layout-frontier-20261125.json) binds the
sources, immutable executables, original measurements, negative controls and
raw validation captures. All 272 Rust test/doc checks, 40 architecture checks,
Clippy, schemas, formatting and Python lint pass. Twelve installed selection/
Lake checks and all 36 required public TUI journeys pass. Seven decoded TUI
journeys were inspected, including the initial graph behind the real namespace
guard, failure, recovery and restart. All 76 browser journeys pass without
skips or flaky results. All 88 browser assets remain exact. An initial Clippy
function-size failure was corrected by extracting the dependency-prefetch
helper; no failed construction or negative-control run counts as a passed gate.

Both actual projects run the same validated executable. The changed compiler
definition produces new bound artifacts once; search and rendering method
descriptors remain original. First complete public preparation takes 200.683 s
in Riemann and 49.345 s in EthLib. Source text, raw graphs, formal expressions,
physical resources, readers, inventory, navigation, history and evidence match
the originals. Original immutable artifacts remain intact.

| Actual cached public file opening | Fresh browser | After host restart |
| --------------------------------- | ------------: | -----------------: |
| Riemann                           |      10.985 s |           13.281 s |
| EthLib                            |       1.951 s |            2.240 s |

These are functional observations with existing caches, not a paired browser
speedup comparison. Complete new source DTOs and cache chains stay exact after
reuse/restart. Both stages execute no graph extraction, counterexample-search
or meaning-rendering worker. Node selection opens an original source line,
retains cameras/tabs and leaves chat unchanged. Captures have no native-ring
eviction; hosts end idle and unapproved.

EthLib's search observations remain identical. Riemann changes four:
`fmat_inv_mul`, `fmat_mul_inv` and `natCast_cancel` change from budget exhausted
to unsupported; `mGen_of` keeps budget exhaustion with a different diagnostic.
The original/current checks are retained verbatim as actual wall-budgeted
observations. They admit neither a counterexample verdict nor native evidence.
The remaining full-input/Lake-query wait is still present, including after host
restart. No cold installation, live-model quality, new proof or local/large-model
assurance is established by these captures.
