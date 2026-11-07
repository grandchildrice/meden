import MedenSpec.Capability
import MedenSpec.Glob
import MedenSpec.Policy
import MedenSpec.Attestation
import MedenSpec.Flow

/-!
# MedenSpec

The formal model of the Meden gate. Every definition here has an
executable counterpart in `crates/meden-core`; the conformance test in
`crates/meden-cli/tests/conformance.rs` compares the two on random inputs
through the `meden-spec` executable (`Main.lean`).

The theorems are the properties a reviewer relies on when merging a change
to the Rust code: deny wins, allowance is exactly "every requirement granted
and none denied", adding grants never turns an allowance into a denial, and
only a `proved` attestation with a Lean-only TCB is *verified* without human
approval.
-/
