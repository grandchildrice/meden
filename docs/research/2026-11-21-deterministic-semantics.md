# Deterministic declaration explanations and falsification observations

The GUI needs readable declaration labels and source hovers without a model request
on selection. It also needs a concrete way to inspect how a meaning-review target
could fail, while preserving the distinction between a statement, a definition,
a test observation and checked proof evidence.

## Method selection

[Lean's output extension documentation](https://lean-lang.org/doc/reference/latest/Notations-and-Macros/Extending-Lean___s-Output/)
describes delaboration from elaborated `Expr` values into display syntax. An
expression retains resolved constants, binder structure and inferred types; a
regular-expression rewrite of the original source cannot recover those facts.
Meden therefore reads the elaborated type and, for definitions, the defining
expression. Fixed templates render quantifiers, implications, conjunctions,
disjunctions, equality, order, membership and arithmetic in controlled English.
Unknown operations retain their names instead of acquiring guessed domain meaning.
Observed predicate definitions are expanded with their exact type arguments and
bound variables. Definition bodies and theorem proof terms remain distinct.

[Natural Lean](https://github.com/medovina/natural_lean) is a controlled-English
authoring language that translates into Lean, rather than a general inverse for
existing arbitrary Lean projects. Adding it would require changing the project's
source language. Meden instead adds a bounded output interpreter to its existing
Atlas compilation pipeline. It uses no LLM and introduces no new proof axiom.
This is an implementation of structural explanation, not a proved round-trip
between unrestricted English and dependent type theory.

The ordinary source index also produces a **source preview**, before compilation.
Its small grammar preserves supported binder/operator structure and labels unsupported
forms explicitly. Imports, notation resolution, section parameters and type checking
are not established by this preview. Its parser and lexical declaration boundaries
are shared between overview and frozen-source snapshots. The preview never invents
a sample result or substitutes for elaboration.

## Existing checker integration

[Plausible](https://github.com/leanprover-community/plausible/tree/b7eb3304aeae834b12dda98993a37f6a41f6f0bb)
is Lean's property-testing framework. Its
[Testable implementation](https://github.com/leanprover-community/plausible/blob/b7eb3304aeae834b12dda98993a37f6a41f6f0bb/Plausible/Testable.lean)
distinguishes failure, successful samples and inability to generate suitable
inputs. It supports explicit seeds and shrinking. Meden bundles only its six
generator/test runtime modules, with original notices and Apache-2.0 license.
The module/namespace is renamed to `MedenPlausible` so preparation cannot replace
the workspace's own Plausible version. Project-specific Plausible instances are
therefore not silently treated as Meden's generators.

Every compiled proposition is tested with seed 0, `numInst = 64`, `maxSize = 32`
and `numRetries = 10`. These are configuration bounds, not a claim about an exact
number of successful samples. Observed project predicates are delta-expanded
before instance synthesis; Lean checks that this preserves definitional equality.
The full original proposition, including assumptions, is tested. A generated
falsification condition is separate from a discovered witness. Its quantifiers
and logical scope are preserved explicitly.

The stored outcomes mean:

| Outcome | Meaning |
| --- | --- |
| `counterexample` | Plausible returned candidate bindings and its failure description. |
| `not_found` | No counterexample was returned under this configuration; this is not proof. |
| `gave_up` | Sampling preconditions was inconclusive, rather than successful validation. |
| `budget_exhausted` | Lean preparation reached its computation/depth bound; the graph and other observations remain available. |
| `unavailable` | A compiler/evaluation task failed outside the Meta exception boundary; no sample result is admitted. |
| `unsupported` | The proposition could not be prepared/evaluated with these generators. |
| `predicate_false_example` | An input where a defined predicate is false; it does not refute the definition itself. |
| `not_proposition` | An ordinary object/function definition does not assert a universally true proposition. |
| `not_run` | Only source structure is available; no compiled sample observation exists. |

Opaque types, noncomputable operations, missing instances and bounded expansion can
remain unsupported. Real/complex analysis does not acquire decidability from an
English explanation. A statement with `sorry` or additional axioms can still produce
a sample candidate; the GUI does not turn that observation into proof admission.
Each checker preparation has a fresh bounded heartbeat context. A preparation
timeout is recorded without rendering its exhausted lazy diagnostic or aborting
the surrounding graph. A separate PPContext and IO catch also isolate compiler-task
failures, which Lean can surface outside Meta's exception handler. Physical worker
deadlines still apply to the whole job.

## Build, cache and display

`/atlas` and GUI **Extract compiled dependencies** compile the frozen observed
modules offline, then run explanation and testing in the imported environment.
The analysis library is imported into that same environment for instance synthesis.
The prepared resource manifest includes `.olean`, available private/server sidecars,
interpreter IR and signatures. Changing an executable analysis resource invalidates
the observation just as changing a source does.

Compiled observations are stored under the host workspace state directory, indexed
by source path. Each content-addressed artifact retains its complete source-set
revision, frozen Lake metadata and analysis-resource fingerprint. Loading it checks
all observed source files and metadata again. A dependency edit cannot leave an old
candidate attached to unchanged root source. Invalid/stale artifacts leave a current
source preview; they do not confer proof or reuse an older successful check.

Graph cards display generated English; original identifiers remain accessible for
navigation. Code-line information icons and graph hovers show the saved explanation.
Meaning-review targets additionally show the falsification condition and checker
outcome. Hovering does not invoke compilation, inference or conversation updates.
All these observations remain outside native contract/proof authority and are
excluded from the existing blind model-based meaning-comparison input.

Overview stars mark saved targets, compiled main-theorem attributes and theorems in
the conventional entry module (`Main.lean` at the root or one directory below it).
Their provenance is shown on hover; none indicates verified status. Large inventories
preload those bounded entry modules so their locations do not depend on opening a
file. Distant marks aggregate by file and keep a fixed screen size.

The existing compiler collector remains bounded at 32 sources/16 MiB. This feature
does not establish a whole-Riemann build, complete domain semantics, cold installation
or a Rust-to-Lean refinement of rendering, cache I/O, naturalization or sampling.
