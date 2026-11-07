# Contributing

## The merge rule

A pull request is merged when CI is green. There is no other gate. That is
only safe because CI checks the things a reviewer would otherwise have to
check by hand:

| Check | What it guarantees |
| --- | --- |
| `rust` | `cargo fmt`, `clippy -D warnings` with `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `arithmetic_side_effects` denied, `#![forbid(unsafe_code)]` in every crate, tests, `cargo doc` without warnings. |
| `deny` | No dependency with a disallowed license, a known advisory, or an unknown source. |
| `kani` | The bounded model-checking harnesses in `crates/tagent-core/src/kani_harness.rs` hold for every input within their bounds, including absence of panics and overflow. |
| `lean` | `lean/TAgentSpec` builds, contains no `sorry`, and every audited theorem depends only on `propext`, `Classical.choice` and `Quot.sound`. |
| `conformance` | The Rust implementation and the Lean model agree on 1,200 random policy, glob and tier cases produced by a fixed-seed generator. |

If you change behaviour, change the Lean model in the same pull request. A
Rust change that the model does not agree with fails `conformance`; a model
change that weakens a theorem fails `lean`. The pull request has to make both
happy, which is the review.

## Layout

```
crates/tagent-core    capability vocabulary, glob, policy, attestation tiers (no I/O, no unsafe)
crates/tagent-skill   SKILL.md frontend (Agent Skills / OpenClaw metadata)
crates/tagent-cli     the `tagent` binary
lean/                 TAgentSpec (model + theorems) and the `tagent-spec` oracle
python/               AgentDojo adapter calling the binary
```

## Running everything locally

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
(cd lean && lake build && lake env lean Audit.lean)
cargo test -p tagent-cli --test conformance     # uses lean/.lake/build/bin/tagent-spec
cargo install --locked kani-verifier && cargo kani setup && cargo kani -p tagent-core
```

## What a change needs

- A theorem or a harness for any new rule the gate enforces. Rules without a
  stated property are not merged.
- No new `unwrap`, `expect`, `panic`, indexing or unchecked arithmetic in
  library code. Return errors.
- No new dependency unless `cargo deny` accepts it.
- Doc comments on public items, in English, that say what the item
  guarantees rather than what it does.

Commits are signed off by CI, not by people. Keep pull requests small enough
that a failing check points at the cause.
