# Security

## What is and is not a boundary

- `tagent-core` is the boundary: policy decisions and attestation tiers are
  specified in Lean, model-checked with Kani within bounds, and checked for
  conformance against the Lean model on every change.
- `tagent-skill`'s static scan of scripts is **not** a boundary. It
  over-approximates the capabilities a skill needs so that the policy can
  refuse it; it does not prove that a skill lacks an effect. Proofs come from
  attestations.
- Attestations are trusted only as far as their declared TCB. A `verified`
  tier means "proved, and the TCB is Lean 4 and its extraction chain"; a
  `verified-with-tcb` tier names the other prover.

## Reporting

Open a GitHub security advisory on this repository. Do not open a public
issue for a vulnerability in the gate itself.
