# Security

## Current requirement and implementation status

The product target is a local-first verification orchestrator whose control
and interface/tool behavior are specified and connected to production Rust
through Lean refinement proofs. That broader assurance is not implemented
today. The current checks are decision-kernel proofs, bounded checks,
sampled conformance and tests; see [verification.md](docs/verification.md).
The proposed assurance chain is in [assurance.md](docs/assurance.md).

The product requirement is that reading an external page must not enable
the coding agent to send confidential files to an unapproved recipient.
The target is a single conversational architecture: models may read tool
results, while an isolated runner and host-owned outbound controls enforce
the boundary. See [the product design](docs/design.md) and
[confidential execution rules](docs/confidentiality.md).

**This requirement is not implemented or verified today.** The current
development TUI uses direct HTTP model clients and an optional unrestricted
shell. It has no trusted outbound transfer broker, no enforced processor
authorization and does not yet use the new OS-level coding runner. Its
benign AgentDojo scores and the formal kernel checks do not establish this
requirement.

## Existing implementation boundaries

The conversational agent (`tui` or `run`)
enforces the capability policy only. It reads tool output and does not
claim to prevent prompt injection or information disclosure within the
granted capabilities. Workspace file tools validate paths and symlink
targets, but are not an OS sandbox or a defence against filesystem races
with other local processes.

The TUI's `--allow-shell` grants unrestricted shell execution with the
current account's permissions. The workspace is its initial working
directory, not containment for shell commands. Timeout and cancellation
manage process lifetime, not isolation; a command that deliberately escapes
its Unix process group requires OS-level containment.

The CLI uses HTTP model connections only. The isolated planner, quarantined
reader, skill attestation subsystem and command-line model wrappers have
been removed. Their historical information-flow claims do not apply to
this conversational runtime.

The retained `meden-core` capability/policy rules are specified in Lean
and checked with six bounded Kani harnesses. The new exact-transfer kernel
has seven Lean theorems for request matching, expiry and single use. Rust
and the executable Lean oracle are compared on 5,000 policy, glob and
transfer cases. These checks cover decision rules; they do not establish
that every real outbound effect passes through those rules.

`meden-runtime` now provides a standalone Linux Bubblewrap runner. Its
integration test uses real subprocesses, a host-only file and a live host
TCP receiver. It checks offline execution and workspace editing, but the
production file tools and optional shell are not yet confined by it. An
immutable transfer broker, model-processor authorization and complete
effect mediation remain unimplemented. See
[verification scope](docs/verification.md) for the precise proof and test
boundaries.

## Reporting

Open a GitHub security advisory on this repository. Do not open a public
issue for a vulnerability in the gate itself.
