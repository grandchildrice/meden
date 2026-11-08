# Formal verification and CI

Meden keeps three kinds of evidence separate: mathematical proofs of a
specification, checks that the implementation agrees with it, and tests of
real operating-system effects. A green badge does not mean the whole agent
is proved secure.

This page describes checks that run today. The proposed orchestrator,
Rust extraction/refinement and artifact delivery requirements are in
[design.md](design.md) and [assurance.md](assurance.md). They are not
established by the current CI badge.

## What CI runs

[`.github/workflows/ci.yml`](../.github/workflows/ci.yml) runs on pushes to
`main`, pull requests and merge groups. There are twelve checks:

| Check | What runs | Failure condition |
| --- | --- | --- |
| Rust | `cargo fmt`, Clippy with warnings denied, workspace tests, documentation build | Formatting/lint errors, failed tests or documentation warnings |
| cargo-deny | Dependency license, advisory and source checks | A dependency violates `deny.toml` |
| Lean | Regenerate Rust model, `lake build`, proof scan/audit, four Rust mutation checks | Model drift, proof failure/hole, unapproved axiom, or a known Rust fault passes |
| Six Kani checks | One bounded Rust harness per job | A counterexample, panic, overflow, insufficient unwind bound or verification failure |
| Conformance | Execute the compiled Lean oracle on the same cases as Rust | A result or resulting transfer state differs |
| Python | Ruff and AgentDojo/PTY integration tests against the built binary | A request/effect/terminal behavior differs from expectations |
| npm | Launcher checks and package dry-run | Invalid launcher behavior or package layout |

Lean is pinned in [`lean/lean-toolchain`](../lean/lean-toolchain). CI builds
the `meden-spec` executable, uploads it as an artifact and passes that exact
artifact to the conformance job. That job sets `MEDEN_SPEC_BIN`; it does not
silently substitute an older local build.

## Lean: exact transfer authorization

[`MedenSpec/Transfer.lean`](../lean/MedenSpec/Transfer.lean) models one
single-use permit and an immutable outbound request. A request contains a
host session, recipient identity and canonical complete request contents.
The intended contents include the operation, target, audience, headers and
payload. The kernel compares the complete value, not a checksum or a
model-generated assertion that two requests are equivalent.

For a request `r`, permit `p`, current session `s` and monotonic time `t`:

```text
accepted = (r.session = s)
        ∧ (p.request = r)
        ∧ (t < p.expires)
        ∧ (p.spent = false)

new_spent = old_spent OR accepted
```

All of the following theorems are checked for arbitrary mathematical inputs, not
only for a finite test set:

| Theorem in `MedenSpec.Transfer` | Proven property |
| --- | --- |
| `accepted_iff` | Reservation succeeds exactly when all four authorization conditions hold |
| `exact_request` | Every successful reservation equals the approved request, including recipient and complete contents |
| `accepted_spends` | Successful reservation returns a state in which the permit is spent |
| `no_replay` | After success, any next request/session/time is rejected using that permit's resulting state |
| `rejection_preserves` | A rejected request leaves the permit unchanged |
| `no_changed_recipient` | Changing the recipient makes reservation fail |
| `no_changed_content` | Changing the canonical contents makes reservation fail |

`no_replay` does not require time to advance. A failed network delivery must
not roll back the reservation. Permit creation, the monotonic clock,
canonical request construction, locking and actual dispatch remain the
responsibility of the host implementation.

The Rust counterpart is
[`meden-core/src/transfer.rs`](../crates/meden-core/src/transfer.rs).
The standalone HTTP export component now reserves through it under a mutex
before dispatching its immutable request. The TUI's model client and other
effects are not yet routed through that component.

## Rust-to-Lean refinement of the transfer kernel

The production functions `meden_core::transfer::allowed` and `reserve`,
including the derived full-request equality and in-place spent-state
update, are extracted by Charon and translated by Aeneas. The checked-in
[generated model](../lean/MedenSpec/Generated/Transfer.lean) is not edited
by hand. [extract-transfer.sh](../tools/extract-transfer.sh) regenerates it
from the real crate and requires an exact diff. Unknown extracted functions
or proof holes make the script fail.

The pinned chain is Rust `nightly-2026-09-17`, Charon
`8cae8fd1905946de61147a65ecdd7122cc6b491b`, Aeneas
`c5eb4a28cef65e0aa27743105308243e535a2a2b` and Lean 4.35.0.
CI downloads the corresponding extractor release with a fixed SHA-256;
Lake pins the Aeneas library and all dependencies in the manifest.

[TransferRefinement.lean](../lean/MedenSpec/TransferRefinement.lean) proves:

| Theorem | Proven property |
| --- | --- |
| `allowed_refines` | The extracted Rust decision returns successfully and equals the abstract specification for every represented request, permit, session and `u64` time |
| `reserve_refines` | The acceptance result and the complete updated permit agree with the abstract reservation transition |

The proof handles variable-length full byte vectors, not hashes or bounded
text samples. Rust bytes map injectively to natural-number lists in the
abstract model; only values 0–255 arise from Rust. Arbitrary binary content,
embedded NULs and Unicode encodings need no favorable string-comparison
assumption. Native integer times map to their exact natural values.

Aeneas's Rust standard-library models, extraction, compilation and platform
semantics remain trust dependencies. There are no project axioms standing
in for unknown functions in this extraction. CI regenerates the normal
x86-64 Linux build; this does not prove all release targets/configurations
or native machine-code correctness. The shipping Rust compiler is still
part of the trusted base, distinct from the pinned extraction compiler.

The [mutation check](../tools/check-refinement-mutants.py) changes actual
Rust source in four ways: invert request equality, accept an expired permit,
ignore the spent flag, and fail to record reservation. Each must produce an
observed Rust test failure, extract/compile successfully, and fail the
unchanged correctness proof. Exact original files are restored and the
normal proof is rebuilt afterwards. This checks proof sensitivity and
source linkage; it does not replace the general refinement theorems.

These theorems establish a decision/state-update refinement. They do not
establish a complete control trace or that every real network operation is
mediated. Full broker refinement and interface obligations in
[assurance.md](assurance.md) remain open; the HTTP component tests below
exercise real reservation and delivery, including concurrency.

## Lean: capability and scope rules

These specifications cover the capability kernel already used by tool
dispatch. They are separate from the new outbound-transfer permit.

| Audited theorem | Proven property |
| --- | --- |
| `Capability.ofString_toString` | Capability serialization parses back to the same capability |
| `Capability.toString_injective` | Different capabilities cannot share a serialized name |
| `globMatch_star` | A lone `*` matches every text |
| `globMatch_literal` | A wildcard-free pattern matches exactly itself |
| `Policy.allowed_iff` | Every requirement is granted and none is denied exactly when the call is allowed |
| `Policy.deny_wins` | A denied requirement makes a call fail, regardless of grants |
| `Policy.grants_monotone` | Adding a grant cannot turn an allowed call into a refusal |
| `Policy.denies_antitone` | Removing a deny cannot turn an allowed call into a refusal |

Definitions and additional lemmas are in
[`Capability.lean`](../lean/MedenSpec/Capability.lean),
[`Glob.lean`](../lean/MedenSpec/Glob.lean) and
[`Policy.lean`](../lean/MedenSpec/Policy.lean).

## Axiom audit

[`Audit.lean`](../lean/Audit.lean) prints the transitive axioms of the seventeen
entry-point theorems listed above. CI permits only `propext`,
`Classical.choice` and `Quot.sound`, the project's standard Lean assumptions.
It rejects `sorryAx` and any other reported axiom. A separate source scan
rejects unfinished proofs and project axiom declarations in the spec and
oracle sources.

This does not prove the absence of bugs in Lean's kernel, its compiler or
the operating system. Those components remain part of the trusted base.

## Rust model checking and conformance

[`kani_harness.rs`](../crates/meden-core/src/kani_harness.rs) contains six
bounded model-checking harnesses:

- `glob_literal_is_identity`: wildcard-free patterns/text up to three characters.
- `star_matches_everything`: a lone wildcard and text up to three characters.
- `deny_wins`: a symbolic requirement, two grants and one deny.
- `allowed_iff_granted_and_not_denied`: two requirements, one grant and one deny.
- `grants_are_monotone`: one requirement and addition of a second grant.
- `scope_rules`: scoped/unscoped requirements and grants.

The capability is symbolic over the current enum. Scope strings come from
the small fixed menus in the source. Kani exhaustively checks these bounds;
the unbounded Lean theorems are a different form of evidence. The transfer
kernel is not currently covered by these six Kani harnesses.

[`conformance.rs`](../crates/meden-cli/tests/conformance.rs) sends one JSON
batch to the executable Lean oracle and compares with Rust:

- 400 generated policy cases.
- 600 generated glob cases.
- 4,000 generated transfer cases, comparing both acceptance and the updated
  permit. Cases include recipient/content/session changes, already-spent
  permits, exact expiry, near-expiry and `u64::MAX` times, and Unicode text.

The generator has a fixed seed. These comparisons catch implementation/specification drift. They are
regression evidence separate from the general transfer refinement proof;
policy/glob still have no extraction-based refinement theorem.
With no oracle present, the local test prints a skip notice. CI explicitly
supplies the freshly built oracle, so that escape does not apply there.

## Tests of actual isolation

The [workspace owner](../crates/meden-runtime/src/workspace.rs) imports a
project with a trusted Rust worker in a read-only source mount. Regular
files are copied, hard links become independent files, literal symlinks
are retained, and sockets/devices/FIFOs are rejected. The source is not
mounted during untrusted tool execution. Results stay in the private copy.

[Sandbox construction](../crates/meden-runtime/src/sandbox.rs) is now used
by the actual TUI file tools, Bash and `meden exec`. A dedicated
[launcher](../crates/meden-runtime/src/process.rs) closes inherited
capabilities before Bubblewrap starts. This matters: the regression test
originally demonstrated a real inherited-descriptor escape. Worker inputs
use an anonymous private stdin file; they are absent from argv/environment.

[CLI integration tests](../crates/meden-cli/tests/isolation.rs) launch the
real compiled worker, edit/read real files and run Bash/Python. They observe
host-only files, deliberately inheritable file/socket descriptors,
IPv4/IPv6/external connection attempts, a live TCP receiver, source sockets,
hard links, outside symlinks and a detached child which signals readiness
before its parent exits. They also run the public `exec` path and independently
check a changed program in another offline snapshot. Missing isolation
must prevent startup. No mocked tool refusal replaces these observations.

The Python PTY test drives read/edit/Bash, multiple turns and `/clear`,
checks retained private results and unchanged originals, and verifies
terminal restoration. Both Rust and Python CI jobs install Bubblewrap.
The live-model smoke harness's independent checker also uses `exec`;
it does not execute model-generated code on the host.

These tests cover production worker paths, not a Lean proof of OS effects.
The fixed model gateway, headless host tools, page cache, trusted export
review and the complete malicious-page acceptance suite remain work.

## Tests of actual HTTP delivery

The standalone [HTTP export component](../crates/meden-runtime/src/transfer.rs)
uses the refined `reserve` function under a mutex. Its target and body are
owned immutable snapshots; host approval is a separate method that is not
exposed as a model tool. Dispatch uses a fresh client with proxies and
redirects disabled. Failures leave authorization spent.

Four tests observe real TCP receivers: no send before approval or across
sessions, exact frozen binary payload after the source changes, concurrent
one-use consumption, no forwarding on a 307 redirect, and no reuse after
the receiver obtains the payload but closes without acknowledging it.
These tests use literal loopback HTTP; they do not prove the HTTP/TLS stack
or establish complete controller/transport refinement. The component
supports one raw POST, not Git/email adapters or service audience discovery.
It is not yet the TUI's outbound path. Authorization recovery after process
restart and complete effect mediation remain implementation work.

## Reproduce locally

```bash
cd lean
lake build
lake env lean Audit.lean
cd ..
cargo test -p meden-core
cargo test -p meden-cli --test conformance -- --nocapture
cargo test -p meden-runtime
cargo test -p meden-cli --test isolation
# With the Kani toolchain installed:
cargo kani -p meden-core
```

Regenerate the transfer model with the pinned tools in `PATH` (or set
`CHARON` and `AENEAS` to their trusted paths):

```bash
bash tools/extract-transfer.sh
# After a deliberate production change:
bash tools/extract-transfer.sh --write
(cd lean && lake build && lake env lean Audit.lean)
uv run tools/check-refinement-mutants.py
```

On NixOS, `nix shell github:aeneasverif/aeneas/c5eb4a28cef65e0aa27743105308243e535a2a2b`
provides the pinned extractors without a permanent system installation.
The standard Rust/Lean conformance and proof checks remain separate.

The runtime test needs Linux, Bubblewrap, Bash and Python. On NixOS, an
ephemeral environment works:

```bash
nix-shell -p bubblewrap --run 'cargo test --workspace --all-features'
```

`MEDEN_BWRAP` can select a trusted Bubblewrap binary. It is host
configuration, not a tool/model argument. No system package installation is
performed automatically by the library.
