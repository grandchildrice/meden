# Security

## Current requirement and implementation status

The product target is a local-first verification orchestrator whose control
and interface/tool behavior are specified and connected to production Rust
through Lean refinement proofs. That broader assurance is not implemented
today. The current checks are decision-kernel proofs, bounded checks,
transfer-kernel refinement, sampled conformance and tests; see
[verification.md](docs/verification.md).
The proposed assurance chain is in [assurance.md](docs/assurance.md).

The product requirement is that reading an external page must not enable
the coding agent to send confidential files to an unapproved recipient.
The target is a single conversational architecture: models may read tool
results, while an isolated runner and host-owned outbound controls enforce
the boundary. See [the product design](docs/design.md) and
[confidential execution rules](docs/confidentiality.md).

**R1 is not yet established end to end.** TUI file/Bash tools and `exec`
now execute in private offline Linux workspaces. Inference uses a separate
host-approved processor gateway; local inference server confinement remains
unfinished.
Page-cache/review integration and complete mediation of the headless host
tool interface remain unfinished. Benign scores and kernel proofs do not
establish the whole requirement.

Model inference and provider discovery disable environment proxies and
automatic redirects. Each inference exchange is attempted once, including
when delivery is uncertain; non-success HTTP responses are rejected. This
fixes transport behavior; it does not confine the selected server itself.

Every inference exchange checks a processor grant with the refined Rust
`transfer::allowed` kernel. The binding contains the host session, exact
inference URL, provider/model and exact credential; it expires after eight
hours and cannot be reset after revocation. Conversation bodies are the
processor grant's scope. This is separate from a single-use exact export.
Already permitted requests can finish after revocation.

The TUI displays the prepared destination and data scope in a host-owned
review screen; only a local `y` key grants permission. Model/tool text,
Enter and bracketed paste do not grant it. Provider changes require fresh
approval. `run` requires `--approve-processor` with the exact inference URL;
the optional TUI flag uses the same matching rule. No discovered endpoint
is implicitly approved. A loopback endpoint may forward to a cloud service:
approved processing is not proof that inference remains local.

## Existing implementation boundaries

The TUI and `exec` import the original tree into a new persistent private
working copy. A trusted importer copies regular files, breaks hard links,
preserves literal symlinks and rejects sockets/devices/FIFOs. The original
source is never mounted for untrusted shell/file execution. Edits remain in
the displayed copy after exit; they are not automatically applied back.

All TUI file operations and Bash execute through the same Bubblewrap
layout: private network/PID/user namespaces, a clean environment, private
`/proc` and only the working copy plus read-only public toolchains. A fresh
single-threaded Rust launcher closes descriptors above 2 before executing
Bubblewrap; stdin and captured stdout/stderr are the only supplied handles.
Confidential tool inputs travel through a private anonymous file on stdin,
not argv or environment variables. A missing backend fails closed.

The host-selected worker executable, toolchain/libraries, workspace storage
and isolation configuration are trusted. Toolchain mounts must be public
resources, not confidential storage. OS enforcement and these worker/adapter
implementations have integration tests, not a Lean refinement proof.

The `run` JSON Lines interface still delegates tool execution to its host.
It does not yet route those arbitrary host tools through this boundary.
It must not be treated as an R1 implementation.

The CLI uses HTTP model connections only. The isolated planner, quarantined
reader, skill attestation subsystem and command-line model wrappers have
been removed. Their historical information-flow claims do not apply to
this conversational runtime.

The retained `meden-core` capability/policy rules are specified in Lean
and checked with six bounded Kani harnesses. The new exact-transfer kernel
has seven Lean theorems for request matching, expiry and single use, plus
two refinement theorems over the extracted production Rust decision and
state update. Rust and the executable Lean oracle are compared on 5,000 policy, glob and
transfer cases. These checks cover decision rules; they do not establish
that every real outbound effect passes through those rules.

The production worker tests observe host-only files, inherited file/socket
capabilities, IPv4/IPv6 attempts, detached subprocesses, hard links, symlink
escapes and actual edits. PTY tests drive real file/Bash calls across user
turns and check terminal restoration and unchanged originals. A standalone
immutable HTTP export uses the refined kernel before actual sends; tests
cover exact bytes, one-use concurrency, redirects and uncertain delivery.
Complete outbound routing, inference confinement and reviewed publishing
remain implementation work. See [verification scope](docs/verification.md)
for exact proof and test boundaries.

## Reporting

Open a GitHub security advisory on this repository. Do not open a public
issue for a vulnerability in the gate itself.
