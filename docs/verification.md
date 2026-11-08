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
| Lean | `lake build`, unfinished-proof scan, axiom audit | A proof does not elaborate, a project `sorry`/`admit`/`axiom` appears, or an audited theorem uses an unapproved axiom |
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
Its existence alone does not constrain a network request: the eventual
broker must actually reserve under its lock and dispatch the same immutable
request. Integration of that broker is still in progress.

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

[`Audit.lean`](../lean/Audit.lean) prints the transitive axioms of the fifteen
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

The generator has a fixed seed. This catches implementation/specification
drift; it is not a proof that Rust agrees with Lean on every possible input.
With no oracle present, the local test prints a skip notice. CI explicitly
supplies the freshly built oracle, so that escape does not apply there.

## Tests of actual isolation

[`meden-runtime/src/sandbox.rs`](../crates/meden-runtime/src/sandbox.rs)
constructs an offline Linux environment using Bubblewrap. Its integration
test launches real Python and child processes, attempts IPv4/IPv6/external
connections, checks a live host TCP receiver, attempts access to a host-only
canary file, checks environment filtering and verifies a real workspace edit.
CI installs Bubblewrap before workspace tests. Unsupported isolation fails;
the test is not replaced with a mocked tool refusal.

This currently validates a standalone execution component with a private
test workspace. It is not yet the TUI's execution path. Managed workspace
copying, confinement of every file tool, the model gateway, transfer review,
page fetching and end-to-end malicious-page tests remain implementation
work. The old development shell still has host permissions until that
migration is complete.

## Reproduce locally

```bash
cd lean
lake build
lake env lean Audit.lean
cd ..
cargo test -p meden-core
cargo test -p meden-cli --test conformance -- --nocapture
cargo test -p meden-runtime
# With the Kani toolchain installed:
cargo kani -p meden-core
```

The runtime test needs Linux, Bubblewrap, Bash and Python. On NixOS, an
ephemeral environment works:

```bash
nix-shell -p bubblewrap --run 'cargo test -p meden-runtime'
```

`MEDEN_BWRAP` can select a trusted Bubblewrap binary. It is host
configuration, not a tool/model argument. No system package installation is
performed automatically by the library.
