# Security

## Current requirement and implementation status

The product target is a local-first verification orchestrator whose control
and interface/tool behavior are specified and connected to production Rust
through Lean refinement proofs. That broader assurance is not implemented
today. The current checks are decision-kernel proofs, bounded checks,
processor-kernel refinement, sampled conformance and tests; see
[verification.md](docs/verification.md).
The proposed assurance chain is in [assurance.md](docs/assurance.md).

The product requirement is that reading an external page must not enable
the coding agent to send confidential files to an unapproved recipient.
The target is a single conversational architecture: models may read tool
results, while an isolated runner and host-owned outbound controls enforce
the boundary. See [the product design](docs/design.md) and
[confidential execution rules](docs/confidentiality.md).

**R1 is not yet established end to end.** TUI, `run` file/Bash tools and `exec`
now execute in private offline Linux workspaces. Inference uses a separate
session-bound processor gateway. The default local path now launches an
offline inference process separate from coding workers, with dedicated
Unix socket IPC and no access to host credentials or the project tree.
The launcher/transport have empirical checks, not a Lean isolation proof.
Complete controller refinement remains unfinished. Benign scores and kernel
proofs do not establish the whole requirement.

The [R1 coverage matrix](docs/r1-coverage.md) identifies production-path
observations and remaining obligations for all sixteen acceptance criteria.
It includes real Git/Cargo subprocess attacks and a real send timeout;
these empirical checks do not establish complete effect mediation.

Documentation now uses a separate host-selected `prefetch` command and an
immutable `--pages` snapshot shared by CLI/TUI. The model can only list/read
cached text; a miss or model-selected URL cannot trigger a fetch. Prefetch
permits bounded HTTPS GETs, rejects nonpublic resolved connection addresses,
and disables proxies, credentials, redirects and subresource loading. The
prefetcher now runs in a dedicated Linux worker without the managed workspace,
host home, credential environment or inherited descriptors. It retains the
launching network namespace and read-only resolver configuration; its trusted
HTTP client restricts connections to the checked public addresses. This is not
an OS egress firewall. The numeric IPv4/IPv6 predicates are extracted/refined;
URL/IP conversion, DNS/TLS/connector behavior, JSON and OS isolation remain
separate adapters. Missing isolation has no host-fetch fallback. Cache files
are host configuration, not authenticated evidence of web origin. See [the cache contract](docs/cli.md#documentation-cache).

Model inference and provider discovery disable environment proxies and
automatic redirects. Each inference exchange is attempted once, including
when delivery is uncertain; non-success HTTP responses are rejected. This
fixes transport behavior; it does not confine the selected server itself.

Every inference exchange obtains a typed call from the extracted/refined
`processor::State`. The immutable binding contains the host session and exact
HTTP URL, provider/model/credential, or local engine/weights path bytes.
A first host approval fixes an exclusive eight-hour deadline. Revocation,
including before approval, and expiry observed by dispatch are terminal;
renewal and permit import/reset APIs do not exist. Each HTTP exchange consumes
a non-cloneable `HttpCall` and reads its original parameters. A `LocalCall`
cannot be used by HTTP, and its actual engine/weights configuration must match.
Conversation bodies are the grant's scope, separate from single-use exact
export. Already permitted requests can finish after revocation. The Rust-to-Lean
proofs cover this lifecycle, all parameter snapshots/getters and arbitrary finite
compositions; clock/input provenance, locking and physical I/O remain adapters.

The TUI displays the prepared destination and data scope in a host-owned
review screen; only a local `y` key grants permission. Model/tool text,
Enter and bracketed paste do not grant it. Provider changes require fresh
approval. HTTP `run` requires `--approve-processor` with the exact inference URL;
the optional TUI flag uses the same matching rule. No discovered endpoint
is implicitly approved. A loopback endpoint may forward to a cloud service:
approved processing is not proof that inference remains local.
Local CLI/TUI instead require prepared engine/weights and never discover or
fall back to those HTTP providers. The local process uses private namespaces,
read-only assets, a clean environment and closed inherited descriptors.
Host IPC pins the socket inode without following symlinks; replacing its
name cannot select another host service. Limits, cancellation, tested
versions and trust assumptions are in [local inference](docs/local-inference.md).

## Existing implementation boundaries

The TUI, `run` and `exec` import the original tree into a new persistent private
working copy. A trusted importer copies regular files, breaks hard links,
preserves literal symlinks and rejects sockets/devices/FIFOs. The original
source is never mounted for untrusted shell/file execution. Edits remain in
the displayed copy after exit; they are not automatically applied back.

All built-in file operations and Bash execute through the same Bubblewrap
layout: private network/PID/user namespaces, a clean environment, private
`/proc` and only the working copy plus read-only public toolchains. A fresh
single-threaded Rust launcher closes descriptors above 2 before executing
Bubblewrap; stdin and captured stdout/stderr are the only supplied handles.
Confidential tool inputs travel through a private anonymous file on stdin,
not argv or environment variables. A missing backend fails closed.

The Linux file worker pins canonical targets beneath an opened workspace root
using restricted `openat2` resolution. The core's extracted/refined consuming
check requires every content effect at both the requested and canonical path
under the same supplied policy before returning its owned resource. Prepared
model callbacks consume the controller's original request/policy pair and
propagate that admission policy separately from model
arguments; payload text cannot supply it. Reads then use held regular-file
descriptors; listing and replacement retain directory handles. Tests check
restricted alias grants and substitute parents/leaves before and after pinning.
This does not freeze concurrent contents or prove physical path/handle attribution; see
[the physical file boundary](docs/verification.md#physical-file-handles-in-the-linux-worker).

File operations and source/export snapshots share a typed result protocol.
The host admits successful bytes only after complete output and actual zero
exit status. Policy refusals remain refusals; missing infrastructure and host
limits stop automatic source repair. Worker output cannot claim host cancellation,
and diagnostic words never choose the failure class. Error constructors,
accessors, admission and agent/source classification are extracted/refined in
[the outcome kernel](docs/verification.md#typed-file-worker-outcomes).
Process facts and JSON/OS interpretation remain adapter obligations.

The host-selected worker executable, toolchain/libraries, workspace storage
and isolation configuration are trusted. Toolchain mounts must be public
resources, not confidential storage. OS enforcement and these worker/adapter
implementations have integration tests, not a Lean refinement proof.

The `run` JSON Lines interface accepts instructions only and executes the
same coding session as the TUI. Arbitrary host tool registration, supplied
policy/system messages, host result callbacks and compatibility entry points
are removed. Instruction input is bounded, unknown fields fail, and progress
events never request execution by the host. A broken output stream cancels
before the immediately following model/tool effect. This closes an unchecked
execution path; it does not establish all of R1.

The CLI uses confined local inference or explicitly selected HTTP connections.
The isolated planner, quarantined
reader, skill attestation subsystem and command-line model wrappers have
been removed. Their historical information-flow claims do not apply to
this conversational runtime.

The retained `meden-core` capability/policy rules are specified in Lean
and checked with six bounded Kani harnesses. The processor lifecycle and
frozen parameter paths have extraction-based Lean refinement and finite-trace
proofs. Rust and the executable Lean oracle are compared on 5,000 policy/glob
cases and processor traces. These checks do not establish that every physical
outbound effect passes through the corresponding rules.

The production export review-control kernel is separately extracted and
refined. Lean proves that only a host approval of a reviewed proposal grants
dispatch, and that arbitrary finite compositions of the extracted step
grant at most one dispatch. A combined production state now owns the frozen
request and performs review/reservation under one runtime mutex. Its
Rust-to-Lean refinement proves exact emitted-request correspondence and at
most one effect over arbitrary finite compositions. The runtime consumes a
borrowed dispatch for the original destination/body. The actual consuming
accessor and exact getters are refined too; this path has no JSON codec.
The actual TUI approval predicate and review-target selector are extracted
and refined, and each handler matches its own target against the host screen.
Correct attribution of typed origins by UI/protocol/transport code, screen
transitions, displayed-review correspondence, reference lookup, URI
normalization and the I/O loop remain unproved; this is not whole-controller refinement.

The production numeric display transformation is also extracted/refined.
It removes the specified control/directional values while preserving ordinary
values and newlines, and maps valid Unicode scalars to valid scalars. Finite
compositions are checked too. The String iterator/conversion/collector and
terminal renderer are separate unproved adapters; this is not a proof of
display authenticity. See the [exact policy and scope](docs/verification.md#display-transformation-actual-rust-code-points).

The third extracted kernel checks exact evidence correspondence. Only typed
checker results matching the accepted obligation, evidence kind, checker,
position and current subject advance completion. Changed input bindings
invalidate progress. Lean proves all seven production operations and coverage
over finite compositions. Typed origin attribution and complete/correct
subject construction remain host responsibilities. Its first consumer is
[Meden's own release gate](docs/release.md), which uses GitHub CI evidence
and trusted build provenance. The experimental pure-u64 transition backend
also uses it to admit actual Rust-to-Lean proofs under reviewed mathematical
contracts and retain the exact checked library. Contract terms are parsed
as restricted data; they cannot execute Lean commands or proof scripts.
Generic draft management, parsing and review rendering remain unproved
adapters. This does not prove arbitrary user projects or the
GitHub/OS/compiler/hash assumptions. See [the supported project scope](docs/project-verification.md).

The outer-turn and inner conversation controllers now have separate
Rust-to-Lean refinement and finite-trace properties. The inner controller
retains original call positions and typed results, bounds model requests
and preserves a latched stop. The actual interruption predicate is also
used immediately before tool dispatch after the decision observer. JSON
history, signal/observer attribution, physical tool effects and the complete
session interpreter remain outside these proofs; R1 remains an open
end-to-end obligation.

The production worker tests observe host-only files, inherited file/socket
capabilities, IPv4/IPv6 attempts, detached subprocesses, hard links, symlink
escapes and actual edits. PTY tests drive real file/Bash calls across user
turns and check terminal restoration and unchanged originals. Immutable
HTTP exports now use the refined kernel through TUI/CLI host review.
`propose_export` only reads a bounded offline snapshot. The host sees the
actual target/method/content type and exact bytes; a session-bound reference
cannot be changed into another proposal, and approval cannot precede review.
The model has no approve/review/send tool. Tests cover changed files,
cross-session references, rejection, replay, redirects and uncertain delivery.
Complete outbound routing, an isolation proof and general user-project
evidence-gated publishing remain implementation work. See [verification scope](docs/verification.md)
for exact proof and test boundaries.

## Reporting

Open a GitHub security advisory on this repository. Do not open a public
issue for a vulnerability in the gate itself.
