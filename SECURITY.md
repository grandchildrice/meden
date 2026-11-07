# Security

## What is and is not a boundary

The isolated planner enforces capability, integrity and disclosure rules.
The native conversational agent (`tui`, or `run --mode tool-calling`)
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

Command-line model providers execute external programs. Their independent
actions are outside Meden's gate; only the returned plan is interpreted by
Meden. Use HTTP model providers when all tool execution must pass through
this runtime. The native TUI supports HTTP providers only.

- `meden-core` is the boundary: policy decisions and attestation tiers are
  specified in Lean, model-checked with Kani within bounds, and checked for
  conformance against the Lean model on every change.
- `meden-skill`'s static scan of scripts is **not** a boundary. It
  over-approximates the capabilities a skill needs so that the policy can
  refuse it; it does not prove that a skill lacks an effect. Proofs come from
  attestations.
- Attestations are trusted only as far as their declared TCB. A `verified`
  tier means "proved, and the TCB is Lean 4 and its extraction chain"; a
  `verified-with-tcb` tier names the other prover.

## Reporting

Open a GitHub security advisory on this repository. Do not open a public
issue for a vulnerability in the gate itself.
