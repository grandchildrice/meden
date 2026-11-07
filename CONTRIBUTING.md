# Contributing

## The merge rule

**A pull request merges when CI is green. No human approval is required.**

That is only safe because CI checks what a reviewer would otherwise have to
check by hand. Sixteen checks are required on `main`:

| Check | What it guarantees |
| --- | --- |
| `rust` | `cargo fmt`, `clippy -D warnings` with `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` and `arithmetic_side_effects` denied, `#![forbid(unsafe_code)]` in every crate, all tests, and `cargo doc` without warnings. |
| `cargo-deny` | No dependency with a disallowed license, a known advisory, or an unknown source. |
| `lean` | `lean/TAgentSpec` builds, contains no `sorry`, and every audited theorem depends only on `propext`, `Classical.choice` and `Quot.sound`. Also runs `tagent attest lean` end to end. |
| `conformance` | The Rust implementation and the Lean model agree on 1,500 random policy, glob, tier and information-flow cases from a fixed-seed generator. |
| `kani (…)` | Eleven jobs, one per harness: each property holds for *every* input within its bounds, panics and overflow included. |
| `python adapter` | The AgentDojo adapter lints and its tests pass against the built binary. |

Pushes to `main` are restricted to maintainers; everyone else contributes
through a pull request. Force pushes and branch deletion are blocked, and a
branch must be up to date with `main` before it merges.

**If you change behaviour, change the Lean model in the same pull request.**
A Rust change the model disagrees with fails `conformance`; a model change
that weakens a theorem fails `lean`. Satisfying both is the review.

## Layout

```
crates/tagent-core    capability vocabulary, glob, policy, attestation tiers (no I/O, no unsafe)
crates/tagent-agent   the agent core: labelled values, plan language, interpreter
crates/tagent-skill   SKILL.md frontend (Agent Skills / OpenClaw metadata)
crates/tagent-cli     the `tagent` binary, including the `attest lean` verifier plugin
lean/                 TAgentSpec (model + theorems) and the `tagent-spec` oracle
python/               AgentDojo adapter calling the binary
examples/             a benign and a malicious skill, and two policies
```

## Running everything locally

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
(cd lean && lake build && lake env lean Audit.lean)
cargo test -p tagent-cli --test conformance     # uses lean/.lake/build/bin/tagent-spec
cargo install --locked kani-verifier && cargo kani setup
cargo kani -p tagent-core && cargo kani -p tagent-agent
```

## Writing a Kani harness

The hosted runner is small, and CBMC will happily exhaust it. Four rules keep
the harnesses tractable; the existing ones in `kani_harness.rs` in both
`tagent-core` and `tagent-agent` follow all four.

1. **No heap.** State the rule on data the harness can hold on the stack and
   let the real API delegate to it, with a property test pinning the two
   together. `tagent-core` does this with borrowed slices (`GrantRef`,
   `ReqRef`, `&[char]`, `&[&str]`); `tagent-agent` does it by reducing a
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

## What a change needs

- A theorem or a harness for any new rule the gate enforces. Rules without a
  stated property are not merged.
- No new `unwrap`, `expect`, `panic`, indexing or unchecked arithmetic in
  library code. Return errors.
- No new dependency unless `cargo deny` accepts it.
- Doc comments on public items that say what the item guarantees, not what it
  does.

Keep pull requests small enough that a failing check points at the cause.
