# Contributing

## The merge rule

**A pull request merges when CI is green. No human approval is required.**

That is only safe because CI checks what a reviewer would otherwise have to
check by hand. Seventeen checks run in CI:

| Check | What it guarantees |
| --- | --- |
| `rust` | `cargo fmt`, `clippy -D warnings` with `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` and `arithmetic_side_effects` denied, `#![forbid(unsafe_code)]` in every crate, all tests, and `cargo doc` without warnings. |
| `cargo-deny` | No dependency with a disallowed license, a known advisory, or an unknown source. |
| `lean` | `lean/MedenSpec` builds, contains no `sorry`, and every audited theorem depends only on `propext`, `Classical.choice` and `Quot.sound`. Also runs `meden attest lean` end to end. |
| `conformance` | The Rust implementation and the Lean model agree on 1,500 random policy, glob, tier and information-flow cases from a fixed-seed generator. |
| `kani (…)` | Eleven jobs, one per harness: each property holds for *every* input within its bounds, panics and overflow included. |
| `python adapter` | The AgentDojo adapter lints and its tests pass against the built binary. |
| `npm` | Package launcher checks and package contents pass validation. |

Pushes to `main` are restricted to maintainers; everyone else contributes
through a pull request. Force pushes and branch deletion are blocked, and a
branch must be up to date with `main` before it merges.

**If you change a formally modelled kernel rule, update its Lean model in
the same pull request.**
A Rust change the model disagrees with fails `conformance`; a model change
that weakens a theorem fails `lean`. Satisfying both is the review.

The [coding agent design](docs/design.md) replaces the isolated planner as
the product direction. Its confidentiality requirement needs new runtime
boundaries and acceptance evidence. Existing Lean/Kani properties do not
prove it. Keep planned behavior, implemented behavior and verified
properties explicit in documentation; do not preserve the retired planner
as a required product mode merely because its code or proofs exist.

## Layout

```
crates/meden-core    capability vocabulary, glob, policy, attestation tiers (no I/O, no unsafe)
crates/meden-agent   the agent core: labelled values, plan language, interpreter
crates/meden-skill   SKILL.md frontend (Agent Skills / OpenClaw metadata)
crates/meden-cli     the `meden` binary, including the `attest lean` verifier plugin
lean/                 MedenSpec (model + theorems) and the `meden-spec` oracle
python/               AgentDojo adapter calling the binary
examples/             a benign and a malicious skill, and two policies
```

## Running everything locally

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
(cd lean && lake build && lake env lean Audit.lean)
cargo test -p meden-cli --test conformance     # uses lean/.lake/build/bin/meden-spec
cargo install --locked kani-verifier && cargo kani setup
cargo kani -p meden-core && cargo kani -p meden-agent
cargo run -p meden-cli -- invariants --explain
```

## Writing a Kani harness

The hosted runner is small, and CBMC will happily exhaust it. Four rules keep
the harnesses tractable; the existing ones in `kani_harness.rs` in both
`meden-core` and `meden-agent` follow all four.

1. **No heap.** State the rule on data the harness can hold on the stack and
   let the real API delegate to it, with a property test pinning the two
   together. `meden-core` does this with borrowed slices (`GrantRef`,
   `ReqRef`, `&[char]`, `&[&str]`); `meden-agent` does it by reducing a
   label to the two booleans the flow rule actually depends on
   (`integrity_ok_flags`, `disclosure_ok_flags`). A `Vec` in a harness is
   what made an earlier version run the runner out of memory.
2. **No panicking operation.** Index through `get`, never `a[i]`, so slice
   bounds checks stay out of the model. Use `assert!`, not `assert_eq!`,
   which drags in `Debug` formatting.
3. **No symbolic-length loops.** Build fixed-length arrays with symbolic
   contents and apply a symbolic length afterwards with `get(..n)`. A loop
   `for i in 0..n { a[i] = … }` with symbolic `n` leaves a reachable bounds
   check that CBMC reports as a failure.
4. **Mind `memcmp` when setting `#[kani::unwind]`.** Slice and string
   equality lower to a byte-wise builtin loop, so the bound must exceed the
   length *in bytes* (`&[char]` is four bytes per element). Either raise the
   bound or compare element by element, as `chars_eq` does. An unwinding
   assertion inside `memcmp` is reported as a failed check and looks, at
   first glance, like the code under test is wrong.

## Changing a catalogued theorem

`invariants/catalogue.json` records, for each property an attestation may
claim, the Lean theorem that states it and the hash of that theorem's
statement. `meden invariants` re-reads the sources and fails if they have
diverged, and that check is required to merge.

So if you change the statement of a catalogued theorem, the build will stop
and tell you which entry no longer matches. What to do depends on what you
changed:

- **Reformatted it.** Nothing to do. Whitespace is collapsed before hashing,
  so rewrapping a line is not a change.
- **Changed it, and the entry's sentence is still true of it.** Update
  `statement_sha256` in the same commit. `meden invariants` prints the new
  hash.
- **Changed what it says.** Then the entry's English sentence describes
  something that is no longer being proved. Rewrite the sentence too, or give
  the new statement a new id and retire the old one. Do not move a hash onto
  a statement the sentence no longer fits: the sentence is what a human
  approved, and the hash is the only thing holding the two together.

## What a change needs

- A theorem or a harness for any new rule the gate enforces. Rules without a
  stated property are not merged.
- No new `unwrap`, `expect`, `panic`, indexing or unchecked arithmetic in
  library code. Return errors.
- No new dependency unless `cargo deny` accepts it.
- Doc comments on public items that say what the item guarantees, not what it
  does.

Keep pull requests small enough that a failing check points at the cause.
