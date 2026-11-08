# Security

## Current requirement and implementation status

The product requirement is that reading an external page must not enable
the coding agent to send confidential files to an unapproved recipient.
The target is a single conversational architecture: models may read tool
results, while an isolated runner and host-owned outbound controls enforce
the boundary. See [the design](docs/design.md).

**This requirement is not implemented or verified today.** The current
development TUI uses direct HTTP model clients and an optional unrestricted
shell. It has no trusted outbound transfer broker, no enforced processor
authorization and no OS-level coding runner. Its benign AgentDojo scores
and the existing formal kernel checks do not establish this requirement.

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

The retained `meden-core` capability/policy rules are specified in Lean,
checked with bounded Kani harnesses, and compared with the Lean oracle on
1,000 policy/glob cases. These checks cover those kernel rules only. They
do not establish the planned runner isolation or outbound authorization.

## Reporting

Open a GitHub security advisory on this repository. Do not open a public
issue for a vulnerability in the gate itself.
