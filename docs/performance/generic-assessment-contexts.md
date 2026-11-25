# Backend-owned assessment contexts and shared orchestration

Baseline: `710dd20`. This advances the application ownership and generic source
boundaries in the [migration](../architecture/typed-core-and-plugins.md).
There is no new kernel effect, model grant, verification evidence or refinement
proof.

The common meaning-assessment use case no longer takes a Lean snapshot. A typed
observation supplies its original source, graph revision, backend-owned formal
context and dependency/resource freshness check. The shared service retains
source-content checks, original complete-body comparison, two separated model
requests, cancellation/authorization observations, progress and history linkage.
Backend freshness distinguishes a changed input from an unavailable inspection;
neither can produce an admitted current assessment.

`semantic::Prepared` validates the original source/graph/target binding and
serializes the typed context once at the final message boundary. It owns the
complete frozen model messages, which callers borrow. It does not interpret or
retain a universal JSON AST. Later backend/source mutation cannot rewrite that
request. The separately held requirement enters the comparison request, not the
blind translation request. Existing prompt text, framing, source limits and
history digest framing remain. Matching types and spellings still confer no
authority.

Lean's adapter owns formal declaration scope, graph validation, theorem
proof-only dependency pruning and exclusion of generated prose/search readings.
Its context is a typed record of formal nodes. Metadata, prepared-input and
resource freshness keep their existing checks and failure distinctions.
The explicit native Lean source/compiler/search/reading composition also moves
out of the common analysis module. Its former application entry points are
deleted rather than re-exported as compatibility aliases. Plain observed text
implements the same assessment interface without an elaborator.

Common analysis-source retention accepts the complete original serialized owner,
preserving original canonical JSON and immutable artifact comparison. The
`analysis.observe` native span covers body/source/backend freshness work. It is
an observation-only timer: an unchanged body, cache hit or model match cannot
admit proof evidence. The ordinary host session dispatches through the generic
assessment operation; native preparation still explicitly selects Lean.

## Conformance and original data

A separate integration test uses the real bounded finite-contract parser,
its own typed AST/context and actual confined source reads through the public
worker executable. Scripted model replies exercise shared analysis/history
completion, foreign context refusal, absent approval, source/dependency changes,
unavailable inspection, complete-body mutation, cancellation and processor
termination. Failed observations retain their original input and no assessment
or Finding. Historical reads preserve existing state projections. This is not
a new solver integration or live-model quality check.

A 3,000-case generated graph corpus compares the new typed context with the
retired JSON projection, retained only as a test oracle. It includes cycles,
Unicode, optional metadata/search kind and known/unknown dependency kinds.
Complete context values and original declaration order must agree, with no
generated reading in the formal payload. A separate mutation test confirms
that an already prepared model request cannot change with its former backend.

The same complete comparison runs on checksummed original compiler artifacts
from the actual selected files: all 15 line-addressable Riemann declarations
and 24 EthLib declarations agree, and their shared preparations succeed.
These are the selected observed modules, not every file in either project.
The diagnostic reads those original bodies; it performs no model or solver
request and does not stamp old artifacts with a new method.

The architecture gate now checks nine common source/compiler/search/cache/view/
assessment/journal modules for direct Lean/Atlas ownership. Its source scan is
not a complete Rust alias/dependency analysis. Reviewable repair preparation,
prepared module ownership, native heterogeneous backend selection and remaining
latency/retirement work remain open. This extraction makes no whole-workflow
speedup, cold-install, new local/large-model or confidentiality claim.

## Validation

The [complete record](../../results/generic-assessment-contexts-20261125.json)
binds the source, executable, original-context diagnostic and validation logs.
All 281 Rust unit/doc checks, 40 architecture checks, 12 installed-worker
source/Lake tests, 36 required public TUI journeys and 76 browser journeys pass.
The browser gate has no skips or flaky outcomes. Seven decoded TUI journeys
and relevant browser screens were inspected. Clippy, schema consistency,
formatting and Python lint pass. All 88 browser assets remain byte-identical.

Development diagnostics corrected the moved scope test's private-constructor
fixture and the generic caller's boxed observation projection. Integration-test
expectations also retain the existing historical projection of stale records;
the final live notification separately retains `Stale`. The scripted caller
honors its own terminal permission state. These fixtures are not a proof of
processor enforcement; the existing public gateway revocation journeys remain
part of the complete gates. No product approval rule is weakened.

## Deployed source preparation and restart

Both actual services run the same gated executable with all original assets.
Each selected file completes its first preparation under the changed
compiler/search definitions, then reuses its exact new cache chain in a fresh
browser and after an actual host restart. Original source, formal expressions,
raw graph, physical resources, prepared-input fingerprint and search results
remain exact; only the owned method/parent/publication identities change.
Original immutable artifacts are retained without relabeling. Reader,
inventory, navigation, candidate history, conversation and evidence remain
unchanged. Node selection follows the original source line, retains cameras
and leaves chat quiet. Both hosts end idle without model approval.

| File click to observed ready | First new-method preparation | Saved analysis | After actual restart |
| ---------------------------- | ---------------------------: | -------------: | -------------------: |
| Riemann                      |                    193.701 s |       10.028 s |             12.228 s |
| EthLib                       |                     49.333 s |        1.945 s |              2.216 s |

These are individual functional observations at 1440 × 960, normal motion and
no throttling, with no concurrent build/gate or other sampler. Graph visibility
and source/camera/chat assertions follow the ready observation; this is not a
physical-presentation or percentile measurement. Saved/restarted visits execute
no compiler extraction, counterexample search or reading-generation worker.
Initial setup still includes actual bounded search and rendering; this
structural extraction does not establish a whole-workflow latency improvement.
No actual-project model request is sent by these fidelity probes. The new
`analysis.observe` span covers candidate assessment rather than file opening;
absence in these traces is not a zero-cost measurement.
