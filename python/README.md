# Meden checks

Integration checks for the compiled Rust coding agent. The Python package
locates a binary and drives its public CLI or a real PTY; it does not host
or execute model-selected tools.

```bash
cargo build -p meden-cli
cd python
uv sync --group dev
MEDEN_BIN=../target/debug/meden uv run pytest -q
uv run ruff check . ../tools
```

Linux, Bubblewrap, Bash and Python are required for real workspace tests.
The public toolchain is mounted read-only. Private virtualenvs/caches are
not added to the mount layout just because they precede a tool in `PATH`.

The project-verification tests additionally require a release binary
and a prepared `MEDEN_VERIFIER_BUNDLE`. The Lean CI job supplies that bundle
and runs them explicitly; other Python runs skip them. See
[the project checker setup and scope](../docs/project-verification.md).

The tests compare TUI and CLI HTTP requests, conversation history and real
file effects over multiple instructions. Adversarial cases read an imported
page fixture and confidential canary, then attempt fabricated send tools,
forged approvals, encoded output and TCP/UDP/DNS through actual shell
execution. Receivers observe traffic and a separate offline copy checks
the resulting program. These checks are narrower than the full R1 suite.

Export tests also use real CLI streams and decoded terminal cells. They
review frozen binary bodies, reject forged/cross-session/replayed approvals,
and observe exact receiver bytes after the source changes. Redirects and
uncertain delivery never cause automatic resend. The terminal decoder is a
development dependency, not part of the Rust runtime.

A separate live-model check drives two actual TUI turns and independently
checks the generated program:

```bash
uv run python -m meden_checks.tui_smoke --binary ../target/debug/meden \
  --provider YOUR_PROVIDER --model YOUR_MODEL --out /tmp/meden-smoke.json
```

It approves the explicitly selected processor through the real review
screen for its synthetic project. It verifies private file effects, original
tests, an additional feature and terminal restoration. Its independent
checker uses `meden exec`, so generated code is not run directly on the
host. Run it separately from performance measurements.

The old AgentDojo host-tool bridge, adapters and dependencies have been
removed. Historical measurements remain in [results](../results/README.md)
with their frozen binary/settings; they do not measure the current private
workspace implementation. See [the CLI protocol](../docs/cli.md) for current
automation, and [verification scope](../docs/verification.md) for proof limits.
