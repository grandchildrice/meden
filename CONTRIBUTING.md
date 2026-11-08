# Contributing

## Workflow

Work on `main`, commit and push directly, as authorized by the repository
owner. Do not create a branch or PR for ordinary work. Review existing
external PRs and relevant checks before merging. Contributors without
direct push access use a PR.

Prefer a small, loosely coupled implementation. Backward compatibility is
not required at this stage. Delete obsolete code/dependencies rather than
keeping alternate runtimes or compatibility wrappers. The product direction
is in [docs/design.md](docs/design.md). Keep proposed assurance obligations
in [docs/assurance.md](docs/assurance.md) distinct from the current checks in
[docs/verification.md](docs/verification.md).

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
(cd python && uv sync --group dev)
(cd python && MEDEN_BIN=../target/debug/meden uv run pytest -q)
(cd python && uv run ruff check . ../tools)
node npm/test.js
(cd npm && npm pack --dry-run)
```

Changes to a formally modelled rule must update its specification and
proofs in the same commit. The transfer kernel additionally requires exact
regeneration with `bash tools/extract-transfer.sh` and passing refinement
proofs; use `--write` only for a deliberate production update. The conformance test compares 400 policy cases, 600 glob cases and
4,000 transfer cases against the built oracle. Workspace tests also require
Linux and Bubblewrap for real isolation checks. See
[verification.md](docs/verification.md) for theorem statements, bounds and
local setup.

The Lean CI job additionally prepares the pinned public checker bundle and
runs the real Rust-ledger proof plus TUI/CLI contract tests. For local runs,
use a release binary and set `MEDEN_VERIFIER_BUNDLE`; see
[project-verification.md](docs/project-verification.md). The ordinary Python
job skips those three bundle-dependent tests; the Lean job requires them.

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
current HTTP replay fixture compares actual TUI and CLI requests/effects. Real
PTY tests exercise TUI editing, tool effects, conversation reset and terminal
restoration. The separate live-model smoke adds independent tests of the
agent's resulting code.
