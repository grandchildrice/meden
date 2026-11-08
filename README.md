<p align="center">
  <img src="docs/assets/tui.png" alt="MEDEN running in a real terminal" width="1080" />
</p>

<h1 align="center">MEDEN</h1>
<p align="center">A native Rust coding agent. Read. Edit. Build. Test.</p>

<p align="center">
  <a href="https://github.com/grandchildrice/meden/actions/workflows/ci.yml"><img src="https://github.com/grandchildrice/meden/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="docs/verification.md"><img src="https://img.shields.io/badge/Lean_4-4.31.0-5f7dbd" alt="Lean 4.31.0 specifications" /></a>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/Rust-2021-dea584?logo=rust&amp;logoColor=white" alt="Rust 2021" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-68dadb" alt="Apache 2.0" /></a>
</p>

<p align="center">
  <a href="#getting-started">Getting started</a> ·
  <a href="docs/design.md">Design</a> ·
  <a href="docs/verification.md">Formal verification</a> ·
  <a href="results/README.md">Benchmarks</a>
</p>

Meden works on real files, runs tools and keeps the conversation across
instructions. Its core is a small native tool-calling loop with host-owned
tool implementations. It connects to OpenAI-compatible model APIs and can
reuse model configuration from Hermes.

- **Workspace tools:** list/read files, write files and make exact edits.
- **Terminal workflow:** multiline Unicode input, history, scrolling,
  cancellation and `/clear`.
- **Model choice:** local or hosted OpenAI-compatible HTTP endpoints.
- **Explicit boundaries:** Lean specifications for capability decisions and
  exact, single-use outbound authorization, checked in CI.

> Development prototype. The outbound confidentiality requirement is still
> being implemented. The current optional Bash tool has host filesystem and
> network permissions. Formal kernel checks do not establish end-to-end
> confidentiality; see [verification scope](docs/verification.md) and
> [security limits](SECURITY.md).

## Getting started

Build from source with Rust:

```bash
cargo build --release -p meden-cli
./target/release/meden tui --workspace /path/to/project
```

Select a configured connection explicitly:

```bash
./target/release/meden providers
./target/release/meden tui --workspace /path/to/project \
  --provider hermes:ollama-cloud --model minimax-m3
```

For another OpenAI-compatible endpoint, set `MEDEN_LLM_BASE_URL`,
`MEDEN_LLM_MODEL` and, when required, `MEDEN_LLM_API_KEY`. For example:

```bash
export MEDEN_LLM_BASE_URL=http://localhost:11434/v1
export MEDEN_LLM_MODEL=your-model
./target/release/meden tui --workspace . --provider configured
```

Add `--allow-shell` to enable the current development Bash tool for builds
and tests. Shell execution is off by default; this switch currently grants
the account's process permissions, not the new isolated runner.

### Controls

| Key | Action |
| --- | --- |
| Enter / Alt-Enter | Send / insert a newline |
| Arrows, Home, End | Edit input; Up/Down recall single-line history or move between lines |
| Ctrl-U | Clear input |
| PgUp / PgDn | Scroll the conversation |
| Tab | Model picker |
| Esc / Ctrl-C while running | Request cancellation; Ctrl-C again exits |
| `/clear` | Start a fresh conversation, retaining files |

## Security by explicit contracts

The first requirement is:

> Reading an external page must not enable the agent to send confidential
> files to an unapproved recipient.

The agent can read tool results normally. The target boundary consists of
an isolated coding runner, a fixed model gateway, a local page cache and
host-approved outbound transfers. It does not use an isolated planner or
ask a model to decide whether its own action is authorized.

The exact-transfer decision kernel now has seven Lean theorems, including
full-request matching and no replay after reservation. A standalone Linux
runner is tested against real connection and host-file attempts. Connecting
these components to every production effect remains work in progress.

[Design and acceptance criteria →](docs/design.md) ·
[CI checks, theorem statements and proof limits →](docs/verification.md)

## Benchmarks

Recorded AgentDojo benign utility with `minimax-m3`, four workers:

| Suite | Meden | AgentDojo baseline | Passes |
| --- | --- | --- | --- |
| banking | 44/48 | 42/48 | 3 |
| Slack | 18/21 | 18/21 | 1 |
| travel | 14/20 | 12/20 | 1 |
| workspace | 33/40 | 31/40 | 1 |

These measure the recorded conversational runtime, not the future complete
confidentiality boundary. The HTTP replay test checks request/effect parity
with AgentDojo. Live-model TUI checks independently test code changes across
two turns. [Raw records and settings →](results/README.md)

## Development

| Package | Responsibility |
| --- | --- |
| `meden-agent` | Conversation, native tool calls and tool interfaces |
| `meden-core` | Capability policy and exact-transfer decision kernels |
| `meden-llm` | HTTP model clients and connection discovery |
| `meden-runtime` | Host execution, process lifecycle and offline Linux isolation |
| `meden-cli` | TUI and JSON Lines host interface |
| `lean/` | Formal specifications, axiom audit and executable oracle |
| `python/` | AgentDojo adapters and PTY/live-model checks |

See [CONTRIBUTING.md](CONTRIBUTING.md) for local checks. Maintainer work
happens directly on `main`; small, loosely coupled implementations take
priority over backward compatibility.

The hero image is captured from the running TUI, not a mockup. The
[PTY capture script](tools/capture-tui.py) records actual terminal cells;
font and terminal dimensions are explicit for reproducibility.

## License

[Apache-2.0](LICENSE).
