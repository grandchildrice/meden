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
The public toolchain is mounted read-only. The combined R1 scenario also
requires Git, a C linker, installed Rust/Cargo and IPv6 loopback for the DNS
receiver. It executes a real Git hook and compiled Cargo build script, checks
DNS/proxy observations and waits through the actual five-second export timeout.
`test_r1_acceptance.py` retains its executable hash and receiver/session
observations in `r1-observation.json` under the pytest temporary directory.
See the [R1 coverage matrix](../docs/r1-coverage.md) for scope and remaining work.
Private virtualenvs/caches are
not added to the mount layout just because they precede a tool in `PATH`.
`test_preparation.py` also places hostile helper commands on absolute and
relative repository PATH entries before startup. It checks observed TCP/host
file effects as well as successful Cargo/Rust execution, retaining
`preparation-observation.json`. The Nix metadata-helper paths require a Nix
host; naming those cases on Ubuntu does not exercise that platform branch.

`test_toolchains.py` constructs mixed system/Nix layouts inside an outer
read-only Linux namespace using real binaries. It checks private execution,
read-only resources, selected tool identities and observed network isolation.
Its proof cases additionally require the release binary and prepared checker:
the actual counter must refine its contract, and a fault that passes its native
examples must fail Lean refinement. The lab requires Nix resources, a dynamic
x86_64 glibc worker and nested namespaces. Ubuntu CI reports missing Nix
prerequisites as skips, rather than claiming this platform coverage.

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

The [experience gate](../docs/ux-testing.md) groups the required real-TUI
journeys, rejects skips and saves a navigable screen trace plus verdicts:

```bash
MEDEN_BIN=../target/debug/meden uv run python -m meden_checks.ux --out /tmp/meden-ux-review
```

Open the generated `index.html`. CI runs these marked cases within the full
suite, with `--ux-required --ux-artifacts PATH`, and uploads their evidence.
These are scripted-model checks with explicit checker fixtures.

A separate live-model check drives two actual TUI turns across a restart and
independently checks the generated program:

```bash
uv run python -m meden_checks.tui_smoke --binary ../target/debug/meden \
  --provider YOUR_PROVIDER --model YOUR_MODEL --out /tmp/meden-smoke.json
```

It approves the explicitly selected processor through the real review
screen for its synthetic project. It verifies edits in the selected repository,
original tests, retained conversation across restart, an additional feature and
terminal restoration. CI exercises this driver with a scripted provider and a
broken-code negative case; that is not live-model validation. Its independent
checker uses `meden exec`, so generated code is not run directly on the
host. Run it separately from performance measurements.

The old AgentDojo host-tool bridge, adapters and dependencies have been
removed. Historical measurements remain in [results](../results/README.md)
with their frozen binary/settings; they do not measure the current private
workspace implementation. See [the CLI protocol](../docs/cli.md) for current
automation, and [verification scope](../docs/verification.md) for proof limits.
