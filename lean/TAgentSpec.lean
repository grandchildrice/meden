import TAgentSpec.Capability
import TAgentSpec.Glob
import TAgentSpec.Policy
import TAgentSpec.Attestation

/-!
# TAgentSpec

The formal model of the T Agent gate. Every definition here has an
executable counterpart in `crates/tagent-core`; the conformance test in
`crates/tagent-cli/tests/conformance.rs` compares the two on random inputs
through the `tagent-spec` executable (`Main.lean`).

The theorems are the properties a reviewer relies on when merging a change
to the Rust code: deny wins, allowance is exactly "every requirement granted
and none denied", adding grants never turns an allowance into a denial, and
only a `proved` attestation with a Lean-only TCB is *verified* without human
approval.
-/
