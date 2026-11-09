# Project verification

Meden's experimental project backend checks a pure Rust state transition
against a host-reviewed mathematical contract. The contract declares its
own state fields, function, parameters, specification and properties. The
[counter](../verification/examples/counter.contract.json) and
[ledger](../verification/examples/ledger.contract.json) use the same backend;
neither is a built-in contract profile.

The current subset has one to eight `u64` state fields and zero to eight
`u64` parameters. It does not cover arbitrary Rust programs, whole
applications, persistent/concurrent transactions or certification. Other
languages and verification backends remain design targets.

## Conversation and host review

Start `meden tui` with a local model and a host-prepared checker bundle.
Describe the intended behavior, for example:

> Build a counter that never decreases or wraps. Reject an increment that
> exceeds the unsigned 64-bit limit without changing the counter.

The model translates the request into contract data and calls
`propose_contract`. Before exposing the draft, Meden parses its mathematical
terms, proves output bounds and listed properties, and checks every example
against the mathematical transition. A successful proposal stops inference
for host review; it is neither acceptance nor implementation evidence.

A failed proposal can trigger bounded repair after the conversation finishes
or exhausts its tool budget. Later file edits cannot erase that need or
stand in for a valid proposal. Repair never accepts the contract; a valid
draft still stops for host review. Unavailable or interrupted checking stops
automatic repair.

Calling `verify_project` before host acceptance returns a recoverable tool
usage error with the proposal/review steps. It does not start a checker,
create evidence or classify missing acceptance as a resource failure.
Actual unavailable or inconclusive verification still stops the turn.

The TUI displays the actual specification, properties, assumptions, source
path, Rust API, examples and contract SHA-256. Page Up/Down scroll the review.
`y` accepts that exact displayed proposal and resumes development; `n` or
Escape rejects it. Pasted text does not grant acceptance. `/contract` reopens
the current contract. Inspecting an accepted contract does not accept a new
proposal or grant new authority.

A replacement draft retains the previous accepted meaning and artifact.
The review shows both meanings. Until the host accepts the replacement,
the new proposal cannot become accepted or verified. Rejecting it restores
the previous contract view; source freshness is checked again before
reporting its evidence current. Replacing or rejecting a draft does not
transfer its review to different bytes. The TUI holds a frozen review and
the JSON-lines CLI uses an exact session/revision/hash reference. Reproposing
identical bytes after rejection creates a fresh revision; a previous review
cannot authorize it. Repeated submission of the currently displayed proposal
is idempotent.

After acceptance, the model implements the declared source and calls
`verify_project`. The shared outer controller independently checks after a
development turn, including false completion and tool-budget exhaustion.
A completed rejection can initiate bounded repair under the same accepted
meaning. The model cannot supply an acceptance flag, checker, receipt,
success result or alternative verification path.

`project_status` reports the displayed `contract`, retained
`accepted_contract`, current evidence and exact artifact/source identities.
The TUI labels success as a verified **project kernel**. Model prose and
`done.outcome = done` do not establish project verification.

## Contract data and mathematical meaning

For a complete JSON example, see
[counter.contract.json](../verification/examples/counter.contract.json).
Its essential entries are:

```json
{
  "name": "Bounded counter",
  "description": "Increment without wrapping.",
  "source_path": "counter.rs",
  "state": "Counter",
  "fields": ["value"],
  "parameters": ["amount"],
  "function": "increment",
  "specification": "if s.value + amount ≤ 18446744073709551615 then { value := s.value + amount } else s",
  "properties": [
    {"name": "monotone", "description": "The counter never decreases.", "statement": "s.value ≤ out.value"}
  ],
  "examples": [{"before": [0], "arguments": [1], "after": [1]}],
  "assumptions": ["The caller uses this transition for counter updates."]
}
```

`s` is the mathematical input state. Parameters and fields are represented
by natural numbers; Rust inputs range over unsigned 64-bit values. The
specification returns the next state. Properties are propositions in `s`,
the parameters and `out`; they may also apply `transition` again. Examples
use the declared field and parameter order.

`source_path` names a file relative to the private workspace; it is not Rust
source text. The model writes that file separately after host acceptance.

The fixed [term parser](../verification/kernel/Terms.lean) allows a bounded
subset of arithmetic, comparisons, propositional/Boolean operators,
conditionals, explicit record fields, `min`/`max` and the declared variables.
It parses a complete term before elaboration and rejects commands, tactics,
holes, arbitrary functions/projections, quotations and metaprograms. Record
labels are not global expression identifiers. Metadata is bounded and must
use supported names and a relative Rust path. Deserializing a draft grants
no authority.

Syntax failures name the rejected `specification` or property statement,
quote the submitted term and list that draft's available expression names
and record labels. The tool schema explains the fixed mathematical names
`s`, `out` and `transition`, Lean record syntax `:=` and the mathematical
`Nat` interpretation of u64 inputs. These diagnostics help repair the
proposal; they do not rewrite its meaning or relax the parser whitelist.

Before host review, the fixed checker proves that every mathematical output
field fits u64 for all bounded inputs and that every submitted example agrees
with the mathematical transition. These checks are mandatory even when no
properties or examples are listed. An unbounded Nat sum is not a u64 overflow
policy: it can exceed the representable state domain. Passing these consistency
checks still does not prove that a formula captures the user's intent.

For a mismatched example, the fixed proof normalizes the mathematical result
on the equality's left side before checking it against the declared expected
state on the right. A failure therefore shows concrete field values, including
large integer arithmetic. This uses Lean's definitional reduction and the
existing kernel-checked equality; the theorem statement, mandatory check and
axiom audit are retained. These values describe the mathematical contract;
implementation extraction has not yet run at this phase.

Descriptions and environmental assumptions are review material. There is
no theorem proving that a description is equivalent to a formula or that a
caller respects an assumption. Assumption strings are not imported as Lean
axioms. A weak formula remains weak even if it has a proof; reviewing the
actual formula is essential to accepting meaning.

The source contains only the declared public struct and function, with a
finite pure body. Supported expressions include branches/returns, `u64`
arithmetic, checked and saturating addition/subtraction, `Option<u64>`,
`Some`/`None`, `match`, `if let`, `let else`, and guarded
`is_some`/`is_none`/`unwrap`/`unwrap_or`. Every path must return successfully.
Imports, attributes, macros, loops, recursion, helper functions and arbitrary
free/method calls are rejected. Proof automation is fixed; a valid function
or proposition outside its capabilities can remain unverified.

## Actual checking and theorems

The host prepares a bounded immutable source snapshot, canonical contract,
worker/checker/resource hashes and an offline namespace. It then issues an
exact-input ticket through the extracted/refined `CheckJobs` kernel. In a
separate job directory, the fixed [recipe](../verification/kernel/check.sh):

1. Parses contract terms and compiles generated `Spec.lean`.
   `KernelContract.output_bounds` proves that every returned field fits u64
   for all inputs in that domain. For each listed property,
   `KernelContract.law_NAME` proves it about the mathematical transition.
   Each `KernelContract.example_INDEX` checks one example against that
   transition using kernel reduction, without native proof evaluation.
2. Runs generated native examples with a trusted Cargo manifest and no
   candidate build scripts or dependencies. Failures show the accepted
   example, inputs, actual result and expected result.
3. Extracts the **actual supplied Rust** with pinned Charon, then generates
   `Candidate.lean` with pinned Aeneas. An unrelated example/model does not
   substitute for that implementation.
4. Compiles the fixed [refinement template](../verification/kernel/Check.lean).
   `KernelProof.actual_refines` proves, for all native inputs:

   ```text
   map abstractState (actual extracted Rust call)
     = successfulResult (mathematical transition (abstractState input) parameters)
   ```

   This is total functional refinement, excluding panic, divergence and
   wrong results in the extracted model. For each property,
   `KernelProof.law_NAME` proves that the actual call succeeds and its output
   satisfies the accepted formula. These native corollaries depend on both
   implementation refinement and the mathematical laws.
5. Audits the exact ordered specification roots (output bounds, listed laws,
   examples), followed by the native roots (`actual_refines` and one
   `law_NAME` per listed property). Only `propext`, `Classical.choice` and
   `Quot.sound`, or no axioms, are accepted. Missing, duplicated, reordered,
   substituted or unfinished roots cannot establish success.
6. Builds the native library. The host rechecks resources, original inputs
   and current source, consumes the exact ticket, then retains the already
   checked bytes outside the coding copy. A later edit or cancellation
   invalidates current evidence; restoring bytes does not revive a consumed
   or invalidated job.

The artifact contains `kernel.rs`, canonical `contract.json`, generated
specification/harness/proof inputs, `Candidate.lean`, the extraction,
`libmeden_kernel.rlib`, the exact recipe and a file-hash report. The checker
identity covers the canonical contract, generated trusted files and recipe;
the report also binds the actual worker, bundle and artifact manifest.
Project-local Cargo, Lean, proof and report files do not replace these inputs.

Before publishing a retained result and whenever current status is inspected,
Meden checks the exact retained file inventory, each file hash and the report
bytes against the checker result held by the host. It does not accept a
replacement report from disk as authority. Files must be regular files;
symlinks and FIFOs are rejected. A mismatch is unavailable verification and
discards receipt coverage through the refined contract controller's edit
operation. Restoring the files alone cannot restore coverage; a fresh
independent check is required. These checks concern the files observed at
inspection time. Atomic storage, concurrent trusted-host mutation and physical
delivery remain separate I/O obligations.

Failures are host-attributed. Native rejection, invalid terms or an ordinary
completed Lean rejection are `unverified`; this does not prove the contract
false. Cancellation, time/output limits, signals, Lean resource tags or a
lost diagnostic protocol are `inconclusive`; missing prepared resources are
`unavailable`. Operational failures stop automatic repair until a new host
turn. Lean resource classification uses the pinned compiler's structured
diagnostic tags, not a model's diagnostic prose.

## Preparing resources and direct checks

Prepare the public pinned Rust/Lean/Charon/Aeneas tools and built Lean
libraries before confidential work. CI's
[Lean job](../.github/workflows/ci.yml) demonstrates the exact versions and
verified downloads. The host records their paths with
[`tools/prepare-verifier.py`](../tools/prepare-verifier.py), then sets
`MEDEN_VERIFIER_BUNDLE` to that manifest before starting TUI/CLI. Missing
assets do not trigger downloads, candidate configuration or a cloud fallback.

For a direct host-administered check:

```bash
./target/release/meden verify-kernel \
  --contract verification/examples/counter.contract.json \
  --source verification/examples/counter.rs \
  --bundle /trusted/project-verifier.json --output /trusted/artifacts
```

This command is not registered as a model tool. Its JSON describes a
successful check under the supplied mathematical contract, without claiming
conversational host acceptance or whole-project correctness.

## CI evidence and remaining boundaries

The Lean job runs `python/tests/test_project_verification.py` with the
prepared offline bundle. Cases drive real CLI and PTY host review, forged
acceptance/results, independent repair, changed source, cancellation/I/O
failures, alternative Rust implementations, and faults which pass examples
but fail refinement. The counter case drives a different contract/API,
repairs its implementation and rejects a proposed weaker replacement while
retaining the earlier accepted evidence. Parser cases exercise rejected
Lean code through the real checker, plus comments, explicit labels and
bound variables whose names also exist globally. Successful artifacts are
uploaded as `meden-project-evidence`.

The retained-artifact case starts with a real checked counter and introduces
six storage faults: changed library bytes, changed report, a self-consistent
forged report/library pair, an extra file, a symlink and a FIFO. Every fault
stops the turn before another tool effect. Restoration stays unverified until
the accepted source is independently checked again.

These are deterministic checker/controller integration tests. The
[historical local-model pilot](../results/local-ledger-qwen3-4b-d3d5fd8.json)
is separate; its generated Rust source is rechecked by the current suite.
No general task-success rate follows from these cases.

Meden's separate control audit has 176 roots; project roots are checked
afresh per candidate. The shared core evidence/job and contract-management
operations have Rust-to-Lean refinement, including their finite compositions.
Metadata/term parsing, canonical encoding, review
rendering, input provenance, source inspection, hashing, locking,
persistence and the whole session/effect interpreter remain adapters.
The proof also trusts Charon/Aeneas translation, Lean's kernel, prepared
libraries, the Rust compiler/native library, OS confinement and SHA-256.
The complete TUI/CLI/tools implementation and end-to-end confidentiality
require further correspondence proofs. This backend is progress toward
[the design](design.md), not completion of its mission-critical guarantees.
