# Backend-owned compiler observations and prepared views

Baseline: `a8c16a0`. This advances the common compiler and source/view boundaries
in the [migration](../architecture/typed-core-and-plugins.md). It introduces no
new kernel effect, evidence authority or refinement proof.

`Compiler::Output`, its registry, consuming invocation and completed observation
retain one backend-owned decoded Rust type. The common port validates its
original source selection, full source bytes/revision and frozen method before
and after dispatch. Each implementation validates its own AST and completeness.
There is no universal JSON AST, implicit Lean output type or format switch in
that common boundary. The explicit native host composition still selects Lean.

Neutral source capture owns complete UTF-8 files and their original content
identity. Lean's adapter owns its elaborated graph, collection and prepared
module metadata. Its compiler validator checks the raw graph directly; it no
longer constructs a presentation graph to validate compilation. Original raw
graph bounds and locations remain checked, including the former view's identity
and axiom bounds. Search observations cannot masquerade as compiler-only data.

Prepared source retention is generic over a read-only projection. It freezes
the complete view once and retains the original observation in an `Arc` without
requiring a backend clone. Repeated revision queries perform no projection or
I/O. A valid, self-hashed page for different source bytes is refused against the
original owner. Lean graph projection and lexical source navigation now live in
separate adapters; unsupported text remains complete with no invented graph.
The existing lexical parser itself is byte-identical to the baseline, preserving
its actual project-navigation cache identity.

The public TUI source journey exposed an unrelated coupling: local source reads
were dispatched through model-worker preparation and could ask for inference
approval. Source capture and local Atlas preparation now use a local task and
the shared source service with their original policy-bound workers. They need
no model grant. Meaning analysis and repair retain their existing approvals.
Cancellation prevents a pending source update from becoming current, and a
failed read discards the current source instead of enabling stale analysis.

The finite-contract conformance implementation validates the existing bounded
contract grammar, preserves its typed AST and projects original statements
through the same compiler/view owners. It exercises no physical compiler or
solver and is not a new tool integration. Existing actual Z3 verification is a
separate application journey. The architecture check rejects direct Lean/Atlas
references in four common production modules; its text scan is not a complete
Rust dependency or alias analysis.

Compiler/search definitions and compiler output schemas capture changed code.
Their first requested preparation must execute under the new definitions; old
artifacts are not relabeled. JSON source framing and public view schemas remain
unchanged. Semantic orchestration, search/cache ownership, repair and native
mixed-backend selection still need migration. Native preparation latency and
remaining map stalls also remain separate measured work.

## Validation and actual project observations

The [complete record](../../results/generic-compiler-observations-20261125.json)
binds the changed sources, executable, captures, cache chains and validation
logs. All 272 Rust test/doc checks, 40 architecture checks, Clippy, schema
consistency, formatting and Python lint pass. The Unicode projection corpus
runs 3,000 cases with a fixed seed. The public TUI gate passes all 36 journeys;
seven decoded journeys were inspected, including original approval, failure,
recovery and restart screens. The complete browser gate passes all 76 cases
without skips or flaky results. All 88 browser assets remain byte-identical.

The first source journey exposed and fixed the actual model-worker coupling.
Compiler diagnostics and three drivers awaiting retired copy were corrected.
An initial browser run ended with exit 143 after 75 observed passes. A subsequent
launch refused the remaining test-owned listener; only that confirmed listener
was stopped before a complete independent browser run. Neither interrupted nor
failed gates count as passing validation.

Both actual services run that same validated executable. Riemann's new bound
preparation takes 205.824 s through public file selection. EthLib's native
analysis span takes 48.819 s. Its first public UI harness stops after collecting
the prepared view because it incorrectly expects Lake extraction rather than
EthLib's actual frozen compiler. Those original native measurements and artifacts
are retained; no successful cold UI verdict or client duration is invented.
Complete subsequent cached and restarted UI checks pass for both projects.

| Actual file opening | Fresh browser, saved analysis | After actual host restart |
| ------------------- | ----------------------------: | ------------------------: |
| Riemann             |                      12.026 s |                  13.281 s |
| EthLib              |                       2.000 s |                   2.108 s |

Both saved-analysis stages execute no graph extraction, counterexample-search
or meaning-rendering worker. Complete new source views and cache chains remain
exact across those visits. Public node selection opens the original source
line, leaves chat unchanged and preserves retained cameras and tabs. Source,
formal expressions, raw graphs, physical resources, reader, inventory,
navigation, history and evidence match the original data. Original immutable
cache objects remain intact. Native metric captures have no ring eviction.

EthLib's search observations remain exact. Riemann changes three observations:
`fmat_mul_inv` and `natCast_cancel` change from unsupported to budget exhausted;
`mGen_of` retains budget exhaustion with a different diagnostic. These are the
actual new wall-budgeted observations, with no counterexample verdict or native
proof admission. The record retains their complete original/current checks.

The warm Riemann trace spends 11.718 s in `analysis.extract`. Its largest
remaining preparation stages are `atlas.project.fingerprint` at 4.325 s,
`atlas.project.query` at 4.210 s and `analysis.discover_layout` at 2.022 s.
They perform complete input hashing, Lake input resolution/build checks and
layout discovery; their durations include their own workers/preparation.
Nested intervals overlap and must not be summed with their parents. Current
thread CPU excludes worker CPU. These captures identify the next native wait
to address; this refactor does not establish a warm-latency speedup, cold-install
success, live-model quality or new local/large-model assurance.
