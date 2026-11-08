# Contributing

## Workflow

Work on `main`, commit and push directly, as authorized by the repository
owner. Do not create a branch or PR for ordinary work. Review existing
external PRs and relevant checks before merging. Contributors without
direct push access use a PR.

Prefer a small, loosely coupled implementation. Backward compatibility is
not required at this stage. Delete obsolete code/dependencies rather than
keeping alternate runtimes or compatibility wrappers. The product direction
and unimplemented security requirement are in [docs/design.md](docs/design.md).

## Validation

CI has twelve checks: Rust (fmt, clippy, tests, docs), cargo-deny, six Kani
policy/glob harnesses, Lean build/axiom audit, Rust/Lean conformance, Python
adapter tests, and npm package checks. A green check proves its stated
scope, not end-to-end outbound confidentiality.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
(cd lean && lake build && lake env lean Audit.lean)
cargo test --workspace --all-features
cargo doc --workspace --no-deps
cargo build -p meden-cli
(cd python && uv sync --group dev --extra agentdojo)
(cd python && MEDEN_BIN=../target/debug/meden uv run pytest -q)
(cd python && uv run ruff check .)
node npm/test.js
(cd npm && npm pack --dry-run)
```

Changes to a formally modelled rule must update the Lean model in the same
commit. The conformance test compares 400 policy cases, 600 glob cases and
4,000 transfer cases against the built oracle. Workspace tests also require
Linux and Bubblewrap for real isolation checks. See
[verification.md](docs/verification.md) for theorem statements, bounds and
local setup.

## Kani harnesses

Run `cargo kani -p meden-core` with Kani installed. Keep harnesses bounded,
heap-free, and small enough for hosted CI. Use fixed arrays and concrete
scope strings, avoid symbolic loop bounds and panicking operations, and
account for builtin byte-comparison loops when setting unwind bounds.
Do not disable an active property to get a passing run.

## Evidence

Keep planned, implemented and verified behavior distinct. Historical
AgentDojo records are immutable evidence of the binary/settings they name;
they are not tests of a newer binary or the future outbound broker. The
HTTP replay fixture compares actual model requests with AgentDojo. Real
PTY tests exercise TUI editing, tool effects, conversation reset and terminal
restoration. The separate live-model smoke adds independent tests of the
agent's resulting code.
