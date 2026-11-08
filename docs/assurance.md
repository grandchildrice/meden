# Assurance and refinement plan

Status: proposed obligations, not existing proofs. This defines completion
criteria for [the product architecture](design.md). The current fifteen
Lean theorem entry points, six Kani harnesses and conformance checks are
listed separately in [verification.md](verification.md).

## What a result means

A claim always names a requirement, accepted contract, exact subject,
property, evidence kind, checker, assumptions and any bounds. The subject
includes source, proof inputs, dependencies, toolchain/configuration and
relevant executable or RTL artifact. A project-wide green indicator cannot
hide an uncovered required property.

| Evidence kind | Permitted interpretation |
| --- | --- |
| Checked theorem | The named formal statement follows in the named logic under its listed assumptions |
| Implementation refinement | The named implementation model simulates/refines the accepted specification under stated extraction and environment assumptions |
| Bounded/model check | No counterexample in the reported harness, model and checked bounds/configuration |
| Empirical test/fuzzing | The recorded executions did not violate the tested property; include counterexamples when found |
| Semantic review | A person accepted that a contract and assumptions describe their intent; this is not a mathematical theorem |

Evidence kind and result status are independent. A job can pass, refute a
property, fail operationally, time out, be unsupported or become stale.
"Not disproved" is not "proved". Missing proof cannot be replaced by a
passing fuzz run unless the trusted accepted requirements already permit
that evidence kind; any such change is explicit in the report.

There are two proof chains: the user's project and Meden's own controller.
Neither can be inferred from the other.

## From intent to the actual project

```text
reviewed natural-language intent
    -> accepted formal contract
    -> actual implementation model
    -> refinement / functional correctness proof
    -> specified build -> delivered artifact
```

Each arrow has a different obligation:

1. **Intent to contract.** Explain domain terms, examples, failure cases,
   preconditions and environmental assumptions in ordinary language. Obtain
   a trusted decision for material ambiguity. A model may help draft and
   review this mapping; natural-language intent is not automatically proved
   equivalent to its own generated specification.
2. **Contract to implementation model.** Extract the actual supported
   source or formally connect its semantics. Handwriting an equivalent
   looking Lean function is not implementation refinement. Unextracted
   wrappers and effects remain named obligations or trust dependencies.
3. **Proof.** Check the relevant functional/refinement statements with a
   pinned checker. Record theorem identities, transitive assumptions and
   the exact inputs. Do not accept new axioms, holes or ignored production
   branches as a way to finish.
4. **Build to delivery.** Link that source and configuration to the actual
   artifact; retain its build and dependency closure. Reproducible builds,
   hashes and attestations bind evidence, but are not a compiler-correctness
   proof. Compiler correctness remains a named assumption unless separately
   established.

For Rust/Lean, the intended path is production Rust -> Charon LLBC -> Aeneas
Lean model -> proof against the accepted Lean specification. The current
[Aeneas documentation](https://github.com/AeneasVerif/aeneas#targeted-subset-and-current-limitations)
limits extraction to a supported Rust subset and describes external models.
Unsupported concurrency, unsafe or library operations must not silently be
replaced with favorable behavior. Pin extraction versions and record all
external models and the extraction/compilation trust assumptions.

For Bend, a checked law about language semantics is separate from parser,
translation and backend correctness. The upstream
[limitations](https://github.com/bendlang/bend#limitations) explicitly list
such gaps. Record them rather than assuming a proved checker also proves
the delivered executable. The same rule applies to RTL models, Verus
wrappers and TLA+ implementations.

### A concrete first project

Use a small Rust ledger kernel with checked integer amounts, explicit error
results, account identities and idempotency keys. Accepted obligations cover
balance conservation, no overdraft, authorization, overflow behavior and
no second charge for the same accepted operation.

Extract and prove the production transition. Then test the application
adapter with real persistence/retry effects. If persistence is only tested,
report exactly that gap; do not describe settlement as proved. Seed an
independent fault such as debiting without crediting or accepting an
idempotency key twice and require the relevant checker to reject it.
Mutation sensitivity helps detect weak contracts but is not itself a proof
that the contract captures every business requirement.

## Meden's control specification

Keep the authority-bearing state machine small enough to specify and
extract. State contains the accepted contract/policy revisions, current
snapshot, prepared jobs, checked evidence identities, cancellation state
and outbound authorizations. Commands carry typed origins; the event
"user accepted" can be created only by a trusted interaction channel.
Parsing the same words from a page or tool result cannot create that event.

The abstract Lean transition returns a new state and a set of permitted
effect commands. The Rust executor receives those commands, not a free-form
model explanation of what should be allowed. Authority-bearing logic has
one implementation used by all entry points.

The intended Lean properties include:

| ID | Obligation |
| --- | --- |
| CTL-01 | Every effect has a permitting transition for its exact prepared inputs; refusal produces no external effect |
| CTL-02 | Only trusted user/configuration events can accept contracts, assumptions, policy changes or processor/transfer authorization |
| CTL-03 | Delivery eligibility requires current evidence for every accepted required obligation on the exact subject |
| CTL-04 | A changed source, contract, dependency, harness, proof or configuration invalidates dependent evidence before delivery |
| CTL-05 | A worker cannot delete an obligation, broaden a precondition, replace a checker or substitute an evidence kind to make delivery eligible |
| CTL-06 | Expired, cancelled, stale or mismatched jobs/results cannot start a new effect or discharge an obligation; returned results match the issued job |
| CTL-07 | A single-use export is reserved once, before dispatch, for the exact recipient/content/session; uncertain delivery does not restore permission |
| CTL-08 | A local session issues no command for external inference, proving, telemetry or caches; explicit transfers remain subject to their authorizations |

These are target theorem statements. Existing Transfer theorems cover part
of the abstract rule in CTL-07, not the dispatcher, concurrency or complete
controller. CTL-08 also requires a real confinement argument: a transition
theorem cannot disable a process's network socket by itself.

### What refinement must establish

Let `Spec.step` be the Lean state machine and `Impl.step` the Lean extraction
of the actual production Rust kernel. Let `R` relate their states, and `obs`
describe externally relevant decisions and effect parameters.

The proof obligations are:

```text
Initial: R(implementation_initial, specification_initial)

Step: R(i, s) and an implementation step i --command/effects--> i'
      imply a permitted specification transition sequence s -->* s'
      with R(i', s') and the same relevant observations.

Trace: after hiding internal bookkeeping, every finite implementation
       trace is allowed by the specification under the stated assumptions.
```

Prove command encoding/decoding, numeric bounds, error paths and observation
correspondence, not only successful examples. Excluding internal steps
requires a stated stuttering relation. Safety trace inclusion alone does
not prove progress, termination or eventual cancellation. Any liveness
claim needs explicit scheduler, fairness and operating-system assumptions.

The executor/adapter boundary needs a separate correspondence: the actual
side effect uses exactly the inputs permitted by the transition. Model
checks of concurrent races complement a proof of a serialized kernel but
do not establish that real callers serialize it correctly. Reservation,
locking, crashes, restart recovery and delivery uncertainty are named
implementation obligations. A restart must not resurrect spent permission
or accept a partial result as checked evidence.

Random Rust/Lean comparisons remain useful regression checks. They do not
replace initialization, simulation and trace theorems for arbitrary
supported implementation inputs.

## TUI, CLI and Tool coverage

The scope is all first-party behavior that can change meaning, authority,
evidence or effects. The final product goal also includes specified user
interaction behavior. Extend proofs in small modules; keep a coverage
matrix tied to production entry points.

| Surface | Required contract and refinement target |
| --- | --- |
| TUI | Input/editor transitions, session selection, cancellation and host-derived review/status presentation; displayed approval corresponds to the prepared operation |
| CLI | Argument/protocol decoding, explicit failures, correct command origin and dispatch into the same kernel |
| Tool interface | Typed requests/results, identities, size/error handling and inability to manufacture trusted control events |
| Files/processes | Workspace confinement, exact snapshot use, bounded capture, cancellation and no inherited unauthorized authority |
| Checker adapters | Exact job/result correspondence and backend-specific proof validation, status, assumptions and bounds |
| Release/export adapters | Exact artifact/request delivery, current evidence/authorization and no alternate unchecked path |

A proof over Unicode editor state needs an explicit model of graphemes,
terminal cells and rendering; testing a few strings is not that proof.
Trusted review must remain distinguishable from untrusted terminal text.
First-party parsers, serializers and presentation models need specifications
or named gaps. Terminal libraries, the emulator, OS isolation, filesystem,
TLS stack and hardware remain in the trust/dependency inventory unless
their relevant semantics are separately established.

Do not claim "TUI/CLI/Tool fully proved" while proving only a shared policy
function. Publish the properties and uncovered surfaces until the stated
whole-product target is met.

## Checking tools and protecting evidence

A backend worker is not an authority. Its exit code, JSON or transcript is
untrusted until a host-selected adapter validates the corresponding job.
Separate candidate generation from checking in a clean offline environment
with immutable inputs and pinned checker binaries. Do not trust worker
produced executables, cached Lean compiled objects, build hooks or shell
scripts as independent proof validation. Regenerate/recheck the relevant
closure from its bound source, using a supported invocation.

For Lean, check the exact theorem statements and transitive assumptions;
reject holes and unaccepted axioms. For another backend, use its actual
checker and record its logic, options and trust base. If its proof artifact
cannot be independently checked, report reliance on that checker/service;
never import a service's "proved" string as a new Lean axiom. Proof checking
may require resources: budget exhaustion remains unknown.

The kernel may prove statements conditional on soundness of a named checker
or effect adapter. The report must expose those hypotheses. It cannot claim
soundness of every external verifier because a well-typed receipt exists.
Cryptographic hashes bind local artifacts under a collision-resistance
assumption; hashes do not prove semantics, execution locality or compiler
correctness.

## CI and release gates to add

[Current CI](verification.md) is not this proposed gate. Add these checks
as their implementation becomes real, and then make them required:

1. Re-extract production Rust under pinned toolchains. Fail on unsupported
   code, extraction drift or unrecorded external models.
2. Build Lean specifications, implementation models and refinement proofs;
   audit holes and assumptions across the actual checked closure.
3. Exercise the same transitions through TUI, CLI and tools, including
   forged approvals/results, stale evidence and contract weakening.
4. Run real isolation and delivery tests with synthetic confidential data
   and observed receivers, including the R1 acceptance suite.
5. Build the deliverable from the bound source/configuration and emit a
   coverage/evidence/dependency report. Gate publication on the required
   proof and integration results for that exact artifact.

The existing release workflow currently packages binaries without this
proof/evidence gate. Changing design documents does not make its releases
verified. The migration must connect the real publication path, not just
add a new unused check.

Track the remaining trusted base explicitly: accepted human meaning and
environment assumptions, extraction tools and external models, proof/checker
kernels, compilers/linkers, platform libraries, OS/isolation, hardware and
approved external processors. Reduce it incrementally and report changes.
A mission-critical example is accepted only when both its project-specific
chain and Meden's relevant control/effect chain are accounted for.
