# Backend-owned search observations and cache projections

Baseline: `e194dcd`. This advances the compiler/search boundary in the
[migration](../architecture/typed-core-and-plugins.md). It changes no kernel
effect, evidence admission, model approval or confinement policy.

The common search port retains a backend-owned input and report type through
registration, invocation and completion. It checks the original complete source,
backend input validity, frozen method, cancellation and complete original request
before accepting a decoded report. The backend validates its own result against
that original input. Neither registration nor a successful observation supplies
verification evidence. A different report type cannot replace the selected
method's output.

The request shares the immutable original observation through `Arc`. It requires
no AST `Clone`; JSON still contains the complete original `observation` and
`snapshot` fields. Before/after request serialization remains deliberate: sharing
a pointer alone would not detect a backend's interior mutation. The static native
spans `search.validate_input`, `search.freeze_input`, `search.validate_binding`
and `search.validate_report` distinguish these validation/serialization costs
from the physical search worker. Nested spans overlap their parents; current
thread CPU is not worker CPU.

Lean's exact declaration coverage, checker validation and graph projection now
belong to its adapter. The common search validator no longer clones the complete
Lean snapshot or constructs and discards a source page. Actual projection still
validates the resulting page. Consuming Lean projections take the uniquely owned
snapshot when available and clone only when it is still shared.

Common storage retains typed compiler/search bodies, role-owned identities,
complete serialization/checksum checks and immutable observation owners. Lean
owns input freshness, metadata and resource validation, scope selection, searched
graph revisions, reading overlays and its cache orchestration. Existing complete
parent/method/body checks, directory names and digest framing remain. The native
host still explicitly composes Lean; this is not yet a heterogeneous source
analysis registry or a public generic persistence SDK.

Conformance uses the existing finite-contract parser with a different typed AST
and report, without implementing `Clone` on that input. It checks shared input
ownership, exact original wire bytes, complete cache round trips, content
corruption, source/AST divergence, foreign calls, malformed result coverage and
bounds, method drift and cancellation. It invokes no physical solver. Existing
actual Z3 verification remains a separate integration. The architecture gate
now guards six common production source/compiler/search/cache/view modules
against direct Lean/Atlas ownership; it is not a full Rust alias analysis.

Changed compiler/search definitions capture their executing boundary code.
Original artifacts are retained, without stamping them with the new methods.
The first requested preparation runs under the new definitions. Later exact
cache hits must reuse that new chain without semantic workers. Public source
schemas and browser assets remain unchanged. No overall latency improvement is
inferred from removing these copies: initial compilation/search, complete input
hashing, Lake resolution and remaining browser stalls are separate costs.

## Actual deployed measurements

Both projects use the exact gated executable and unchanged browser assets. Each
actual file is selected through the public explorer, with no concurrent build,
functional gate or second sampler. Chrome runs at 1440 × 960, normal motion and
no throttling. These are individual functional observations, not percentiles or
an old/new speedup comparison. Initial preparation is deliberately a miss under
the changed methods; source, raw graph and original search observations remain
exact. New artifacts retain their actual parent/method bindings.

| Completed native interval        |   Riemann |   EthLib |
| -------------------------------- | --------: | -------: |
| Search input validation          |  0.572 ms | 0.296 ms |
| Freeze complete search input     |  3.306 ms | 4.788 ms |
| Validate original result binding |  3.351 ms | 4.838 ms |
| Backend search-report validation |  0.033 ms | 0.046 ms |
| Physical counterexample search   | 121.255 s | 17.845 s |
| English reading preparation      |  10.204 s | 16.680 s |

The recorded native search and rendering intervals include their own process
preparation/validation; they are not pure engine CPU measurements. The renderer
still starts a confined Informath process for each supported expression within
one bounded resource-validation batch. Riemann's fresh search also retains its
actual before/after Lake resolution and complete-input checks. These costs are
separate from the millisecond common request/report checks above.

From file click to observed ready status, first preparation takes 191.687 s /
51.354 s (Riemann / EthLib). A fresh browser using the saved analysis takes
10.992 s / 1.957 s. Graph visibility, original source-line selection, stable
cameras and unchanged chat are separately asserted after the ready measurement;
this is not a physical-presentation timing. Cached stages execute no compiler,
search or rendering worker.

Riemann's cached native extraction takes 10.173 s. Its source-layout discovery
takes 1.309 s, Lake query 3.887 s and complete-input fingerprinting 3.716 s.
Those remaining input-validation costs dominate this saved-analysis visit.
EthLib's cached native extraction takes 0.919 s. No time is assigned as zero
to absent search-dispatch spans: cached report projection still runs under the
separately measured cache/search operation.

After an actual host restart, the same file selection takes 12.181 s / 2.232 s.
Both projects again execute no semantic worker and retain the exact complete new
source view and compiler/search/rendering chain. The original immutable objects,
reader, inventory, navigation, history, candidate records and evidence remain
unchanged. Both services end idle without model approval or a new job/preview.

## Validation

The [complete record](../../results/generic-search-observations-20261125.json)
binds the source files, executable, metrics and validation logs. All 276 Rust
unit/doc checks, 40 architecture checks, 12 installed-worker source/Lake tests,
36 required public TUI journeys and 76 browser journeys pass. Seven decoded TUI
journeys and relevant browser/restarted project screens were inspected. Clippy,
schema consistency, formatting and Python lint pass; all 88 served browser assets
remain byte-identical.

The first browser run has 74 passes and two failures. The variant snapshot helper
encountered an unstaged removed test path; staging the actual move gives it the
correct file set. The judge test raced visible revocation layout changes before
Close; it now waits for the rendered action to disappear. An isolated original
diagnostic passes, followed by an independent complete 76-case run with no skips
or flaky outcomes. The failed run remains recorded and is not counted as a pass.
The initial architecture-script formatting failure is also retained separately.

These checks use actual offline file/Lean/Lake/Plausible/Informath tools and
scripted model replies where needed. They establish no new live-model,
cold-install, large/local-model or confidentiality validation. Native mixed
backend composition, semantic/repair orchestration, prepared module ownership,
remaining native waits and browser-detail/retirement work remain open.
