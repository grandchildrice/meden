# Native backend ownership

Baseline: `9f8c91e`. This advances the application ownership and generic-view
exits in the [migration](../architecture/typed-core-and-plugins.md).

Native GUI, TUI and JSONL sessions no longer retain Lean snapshots or Lean repair
contexts. One application owner retains the original typed backend output and
its original preparation implementation behind dispatch. Associated Rust types
still connect source/semantic context, repair context, refreshed observation and
reanalysis. No downcast, prover enum or JSON execution interface is introduced.
Only the host preparation path can construct this owner; decoded historical
JSON cannot reconstruct an executable observation or choose another backend.

Original selected paths are bounded and normalized before preparation. A backend
cannot omit one of them. A private fingerprint binds the complete original
serialized body, while the original AST remains owned in its backend type. No
second raw JSON body is retained. Projection, serialization and explicit analysis
refuse a changed body. The view must also agree with the backend's original
semantic graph revision and compiled mode. The GUI keeps its existing frozen
publications; TUI declaration selection now uses the same validated generic view.

Native repair retention stores the original preparation/context pair. Installing,
reviewing, rejecting, abandoning and applying still delegate to the existing typed
queue and consuming controller. Applying consumes that queue on every attempt.
Reanalysis uses its original implementation, rather than the session's later
selection. Original requirement, target, context mode, source/dependency/resource
checks, model approval, file policy and evidence rules remain unchanged.

Plain source now retains a neutral source observation directly, including during
assessment and repair. Lean observations require a complete compilation: graph,
collection and exactly one frozen-module or prepared-Lake input. The retired
source-only constructor, nullable compilation, fallback projection and unreachable
missing-graph/collection paths are deleted. Missing/null compilation is refused
by the Lean codec; ordinary plain-source JSON stays in its own context.

Lean's original repair resources/collection are required too. Their original
metadata, dependency fingerprint and resource checks remain. Host composition
owns format selection, background eligibility, optional saved presentation and
backend-specific scope descriptions. These choices no longer require concrete
AST fields in native session/view retention. Existing saved-before-validation
presentation and failure retirement use this same preparation interface.

The compiler definition stops capturing the orchestration pipeline. Compiler
implementation/schema changes still prepare new artifacts once. A later pipeline
progress-label update can reuse exact original compilation/search/rendering
objects. The native update journey builds two real executables, verifies their
different embedded runtime labels, then checks exact artifact files, source view,
actual current-input query/fingerprint, quiet inference and restart state.

## Validation scope

Conformance passes a different decoded finite AST through the same native owner,
plain/compiled view retention, candidate assessment, reviewed repair and original
preparation callback. Real confined file reads and actual public Z3 observe an
overflow counterexample before repair and bounded success afterward. Selecting
another implementation and dropping the original handle cannot change the
pending repair's implementation. No accepted project evidence is granted by this
observation. Scripted model replies exercise orchestration, not model quality.

The native owner also refuses omitted original sources and a foreign semantic
view binding. Dependency edits, denied write policy, cancellation, foreign review,
rejection and restart consume/refuse the operation without edits or repreparation.
Changing serialized context after native binding stops before the first model
request, even though its source/graph identity is unchanged. Public construction
and JSON reconstruction fail compile examples. Maintenance guards reject
concrete backend observations in native retention and pipeline/facade capture in
compiler/search definitions; they are not an alias-proof Rust parser or proof of
truthful adapter I/O.

Production native compiler composition still registers Lean. The finite AST
conformance is not a shipped general SMT source viewer; public accepted-contract
verification already has its separate actual `z3.finite.v1` registration. Native
heterogeneous host selection, backend-local layout/collection ownership, residual
navigation waits and retirement still require completion. This change adds no
kernel effects or new refinement, live-model, local-inference or cold-install
claim.

All 298 Rust unit/doc checks, 52 architecture checks, 12 source/Lake checks,
36 required public TUI journeys and 80 browser journeys pass. The browser gate
has no skipped, flaky or retried outcomes. Eight decoded TUI journeys and the
saved-view/failure/pipeline-update browser captures were inspected. Clippy,
generated-schema consistency, Rust/web formatting and Python lint pass. The
browser's production source and all 88 built assets remain unchanged.

The [machine-readable result](../../results/native-backend-ownership-20261125.json)
records the sealed executable/worker, complete native update conformance,
actual-project observations, method bindings and raw-capture/driver hashes.

Cargo's initial multi-package Rust gate builds a distinct integration-worker
executable. Its observed hash and matching dependency executable are retained
separately from the fixed GUI/TUI candidate. The six repair conformance cases
were rerun with `MEDEN_TEST_WORKER` explicitly selecting that candidate; the
fixture freezes this trusted test input once. All six pass, and Clippy passes
for the changed fixture. Production code and the successful public gates remain
unchanged. The result does not label the original Cargo worker as the deployed
executable.

## Actual deployed observations

Both Riemann and EthLib preserve complete original source/formal graph, all
search observations, reader, inventory, navigation, evidence, history and
candidates, apart from the newly bound graph/publication identities. Their
original physical input bindings and renderer definitions remain exact. Original
immutable artifacts are retained without relabeling. The new compiler/search
schema prepares a new chain once; cached and actual restarted visits reuse its
complete bytes without compilation/search/rendering or model requests. Node
selection reaches the original source line and retains file/map cameras and
quiet chat. Both hosts finish idle without restored model approval.

| File click to observed full preparation | First new-method preparation | Cached visit | Actual restart |
| --- | ---: | ---: | ---: |
| Riemann | 196.673 s | 12.011 s | 13.095 s |
| EthLib | 50.376 s | 2.960 s | 3.249 s |

These are individual functional observations at 1440 × 960 with normal motion
and no other build/gate/sampler, not a controlled old/new performance comparison,
physical paint timestamp or population percentile. Saved presentation remains
separate from completed current-input validation. The held-query browser cases
show the original graph before release in 1.648 s (reduced motion) and 1.312 s
(normal motion); those fixture observations are also not real-project medians.

Native original-body binding takes 4.48–4.58 ms on Riemann and 6.73–7.38 ms on
EthLib across these visits. Native view materialization, including the new body
check, takes 6.23–6.33 ms and 10.60–11.46 ms respectively. These host-thread
intervals do not include subprocess CPU or establish browser rendering cost.
Full-input validation remains the substantial completion wait; this ownership
change does not claim to optimize that algorithm.
