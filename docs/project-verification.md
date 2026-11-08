# First user-project verification path

Meden supports one experimental contract profile: a pure Rust ledger
transition. TUI and JSON-lines CLI share the same session, host contract
state, independent checker and evidence gate. Other project domains and
arbitrary user-written contracts remain unsupported.

## From a request to a verified artifact

Prepare the public tools and model before starting confidential work, then
start `meden tui --workspace /path/to/project` and ask:

> Build a formally verified local ledger. Transfers must conserve total
> balance, reject overdrafts and unauthorized actors, and never charge a
> repeated request twice. Keep all data local.

The model calls `propose_ledger`. After pairing the tool results, the host
ends the turn as `needs_review`, without asking the model to stop itself.
Meden displays the fixed supported contract
for host review. Read its assumptions and press `y` to accept and continue;
`n` or Escape returns without acceptance. `/contract` reopens the review.
Enter, bracketed paste, model prose and tool arguments cannot accept it.

After acceptance the model implements `ledger.rs`. On a development turn,
Meden independently checks the candidate even if the model claims it has
finished without calling `verify_ledger`. A failed check is returned for
repair, up to four conversation/check iterations. An exhausted, cancelled,
unsupported or failed check leaves the current source unverified. Read-only
conversation turns do not automatically rerun development.

The TUI's host status distinguishes a verified **ledger kernel** from an
unverified current source. The model's answer is not this status. CLI emits
`project.summary.verified` before the terminal `done` event. A successful
conversation alone does not establish a proof. The artifact directory,
source hash and artifact hash are in the project summary/tool result.

## Exact accepted contract

`LedgerState` contains three public `u64` fields: `debit`, `credit` and
`sequence`. `transfer(state, actor, id, amount)` returns a new state.

A transfer is permitted exactly when:

- `actor == 0`;
- `id > state.sequence`;
- `amount > 0` and `amount <= state.debit`;
- crediting the amount does not overflow `u64`.

A permitted transfer debits and credits exactly the amount and records
`id`. Every other request returns the unchanged state. Initially a sequence
of zero requires a positive first ID. IDs increase strictly, so an earlier
retry remains rejected after intervening successful transfers. A sequence
at `u64::MAX` permits no further request.

The independent [Lean specification](../verification/ledger/Spec.lean)
uses natural numbers. It proves total-balance conservation, rejection,
exact valid-transfer behavior, unauthorized-actor rejection, old-ID
rejection, repeated-request idempotence and bounded result fields. Exact
valid-transfer behavior prevents an implementation that always returns the
input from satisfying the contract vacuously.

| Audited theorem | Statement |
| --- | --- |
| `LedgerProof.actual_refines` | The actual extracted Rust transition returns successfully and its entire result equals the independent specification for every represented input |
| `LedgerContract.conservation` | Debit plus credit is unchanged |
| `LedgerContract.rejection` | Every unpermitted request returns the unchanged state |
| `LedgerContract.valid_transfer` | Every permitted request moves exactly the amount and stores its request ID |
| `LedgerContract.unauthorized` | An actor other than zero cannot change the state |
| `LedgerContract.old_request` | An ID no greater than the stored sequence cannot change the state |
| `LedgerContract.repeated_request` | Repeating the same request on its result has no further effect |
| `LedgerContract.bounded_result` | Represented input state/ID bounds imply bounded result fields |

The caller must authenticate the actor and use this transition for state
updates. The public struct does not prevent other code from changing its
fields. These claims cover this function, not authentication, parsers,
databases, concurrent transactions, persistence, settlement, an entire
application or financial certification.

## What the independent checker checks

1. Snapshot at most 64 KiB of `ledger.rs` using the confined file worker.
2. Parse the untrusted source inside an offline worker. Accept only the
   declared struct/function API and a finite pure subset: local variables,
   `if`, `return`, record/field operations, comparisons, Boolean operations
   and `u64` addition/subtraction. Attributes, macros, calls, loops, nested
   items, unsafe code and conditional compilation are unsupported. This
   syntax check is a boundary, not the correctness proof.
3. Create a fresh job with the exact source and Meden's fixed Cargo files,
   example tests, specification, proof harness and shell recipe. Ignore the
   project's Cargo configuration, `build.rs`, Lean files, cached `.olean`
   files and claimed reports. Mount only prepared public checker resources.
4. Run the fixed native examples with release overflow checks. Extract the
   **actual Rust function** with pinned Charon, translate with Aeneas and
   freshly compile the generated Lean model.
5. Check [the fixed refinement proof](../verification/ledger/Check.lean).
   `LedgerProof.actual_refines` states that every represented Rust input
   returns successfully and its entire result equals the independent
   specification. The total postcondition excludes failure, effects and
   divergence in the extracted computation; a passing example test is
   insufficient. The proof strategy can fail on a correct candidate too;
   that result remains unverified.
6. Require all eight exact theorem audit roots and only Lean's standard
   `propext`, `Classical.choice` and `Quot.sound` axioms. Reject failed Lean
   exits, proof holes, incomplete audits and output/resource exhaustion.
7. Compile the native Rust library from that same source and fixed build
   configuration. Recheck input/tool content bindings, resnapshot the coding
   copy, and admit an exact checker-origin receipt through the refined
   evidence kernel only if the source is still current. Final admission and
   local retention share a critical section with cancellation: cancellation
   first refuses admission; admission first finishes before cancellation
   returns. An inactive receipt cannot advance the extracted kernel.
8. Retain the already checked bytes outside the writable coding copy. Do
   not rebuild after granting eligibility. The artifact includes the Rust
   library, source, extraction, specification, proof harness, recipe and
   file/hash manifest in `report.json`.

The script gives checker execution 240 seconds, bounded captured/delivered
output and two build/Lean threads. Lean has a 4 GiB heap budget; the process
allows 16 GiB virtual address space for mapped Lean libraries. Tool-closure
hashing happens before and after execution and is cancellable. Preparing
and hashing the pinned Lean/Mathlib closure can take substantial time and
disk space; this is an experimental backend.

A later source change resets evidence progress. Restoring old bytes does
not recreate a discarded receipt. Existing retained artifacts still name
their own exact inputs; they do not make edited working files verified.
Changing host checker installations during work is unsupported. This is
local artifact production, not permission to publish it externally; raw
HTTP export uses its separate exact host authorization.

## Preparing the public checker bundle

Use the versions pinned by [CI](../.github/workflows/ci.yml) and
[the extraction checks](verification.md#rust-to-lean-refinement-of-the-transfer-kernel):
Rust `nightly-2026-09-17` with `rustc-dev`/`llvm-tools`, Charon
`8cae8fd1905946de61147a65ecdd7122cc6b491b`, Aeneas
`c5eb4a28cef65e0aa27743105308243e535a2a2b`, and Lean 4.35.0 with this
checkout's pinned Lake dependencies. Build the public dependencies before
opening confidential source:

```bash
(cd lean && lake build)
cargo build --release -p meden-cli
python3 tools/prepare-verifier.py \
  --rust /path/to/prepared/rust-sysroot \
  --charon /path/to/charon --driver /path/to/charon-driver \
  --aeneas /path/to/aeneas --output /path/to/host/verifier.json
export MEDEN_VERIFIER_BUNDLE=/path/to/host/verifier.json
./target/release/meden tui --workspace /path/to/project
```

Run the preparation script from the trusted checkout. It records canonical
paths to public tools and built proof libraries; it downloads nothing.
Keep that manifest outside the model-writable project. A session captures
the host-selected manifest path; model tools accept no alternative checker,
manifest path, contract, receipt or success flag. Missing assets fail
without a download or cloud fallback. Linux/Bubblewrap and the namespace
requirements of the workspace backend apply to checker jobs too.

For host administration/debugging, the fixed contract can be checked
without a model:

```bash
./target/release/meden verify-ledger --source ledger.rs \
  --bundle /path/to/host/verifier.json --output /path/to/artifacts
```

This command is not a conversation tool or a semantic user acceptance. Its
JSON result describes a successful check under this fixed contract only.

## CI evidence and remaining trust

The existing Lean CI job prepares the pinned bundle and executes
[`test_project_verification.py`](../python/tests/test_project_verification.py)
against the actual release binary. CLI and real-PTY TUI tests use a scripted
confined engine: forged acceptance fails, a false completion is independently
rejected, a different correct implementation is checked, and edited local
checker/config files cannot substitute for host inputs. The CLI test also
checks exact retained file hashes and source invalidation. Another test
injects a special-case no-op for amount four: the fixed native examples
pass, while the unchanged Lean refinement fails and no artifact is emitted.
These are controller integration tests, not a local LLM coding benchmark.
The ordinary Python job skips only these bundle-dependent tests; the Lean
job explicitly supplies the manifest and runs them.
Its `meden-ledger-evidence` artifact retains both successful cases' actual
source, native libraries, proof inputs and eight-root audit reports.

The separate Meden control audit contains 42 roots, including the extracted
acceptance getter now used by both interfaces. The eight ledger roots are
checked afresh per candidate. Neither count proves the entire TUI, CLI,
protocol parser, origin attribution, job adapter or filesystem behavior.

Trust dependencies include the accepted contract's meaning, trusted bundle
preparation/immutability, syntax adapter, Charon/Aeneas and standard-library
models, Lean kernel and libraries, Rust compiler/native libraries, host OS,
Bubblewrap confinement and SHA-256. Source/artifact hashes establish
correspondence under these assumptions; they are not machine-code
refinement or a proof of the compiler, and full outbound confidentiality
still requires the separate end-to-end assurance work.
