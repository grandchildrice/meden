<p align="center">
  <img src="docs/assets/tui.png" alt="MEDEN running in a real terminal" width="1080" />
</p>

<h1 align="center">MEDEN</h1>
<p align="center">Natural-language development, backed by checked evidence.</p>

<p align="center">
  <a href="https://github.com/grandchildrice/meden/actions/workflows/ci.yml"><img src="https://github.com/grandchildrice/meden/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="docs/verification.md"><img src="https://img.shields.io/badge/Lean_4-4.35.0-5f7dbd" alt="Lean 4.35.0 specifications" /></a>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/Rust-2021-dea584?logo=rust&amp;logoColor=white" alt="Rust 2021" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-68dadb" alt="Apache 2.0" /></a>
</p>

<p align="center">
  <a href="#getting-started">Getting started</a> ·
  <a href="docs/design.md">Design</a> ·
  <a href="docs/verification.md">Formal verification</a> ·
  <a href="results/README.md">Benchmarks</a>
</p>

Meden is building a local-first development orchestrator: describe what to
build and what must never go wrong, then connect that intent to code,
formal contracts, verification and the artifact you deliver. The target is
one natural-language workflow across Lean, Rust verification, language
checkers, model checking and fuzzing, with explicit evidence for each
requirement.

The current prototype is a native Rust conversational coding agent. It
works on real files, runs tools and keeps context across instructions. It
connects to OpenAI-compatible APIs and can reuse Hermes configuration.

- **Workspace tools:** list/read files, write files and make exact edits.
- **Terminal workflow:** multiline Unicode input, history, scrolling,
  cancellation and `/clear`.
- **Model choice:** local or hosted OpenAI-compatible HTTP endpoints.
- **Explicit boundaries:** Lean specifications for capability decisions and
  exact, single-use outbound authorization, checked in CI.

> Development prototype. Local-only orchestration, project proof workflows,
> controller-wide refinement and evidence-gated delivery are design targets.
> TUI file/Bash tools and `exec` now run in private offline workspaces.
> Model HTTP connections require host approval and may still be remote.
> Local inference confinement remains unfinished. See [verification scope](docs/verification.md)
> and [security limits](SECURITY.md).

## Getting started

The workspace backend currently requires Linux, Bubblewrap and Bash.
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

Before inference, the TUI shows the actual processor URL and asks whether
it may receive this workspace's conversation, file contents and tool
results. A local `y` key grants permission for this session, up to eight
hours. Choosing another processor requires fresh approval. Loopback alone
does not establish that a server keeps data local.

Automation must specify the exact inference URL with
`--approve-processor URL`; for example, a configured local endpoint uses
`--approve-processor http://localhost:11434/v1/chat/completions`.
This is required by `run` and optional in the TUI.

The project is imported into a private working copy under
`$XDG_DATA_HOME/meden/workspaces/` (default: `~/.local/share/meden/workspaces/`).
File tools and Bash share that copy at `/workspace`; the original project
is unchanged. The TUI displays the result location and retains it on exit.
There is no host-shell fallback. Dependency caches/toolchains must be
prepared locally; workers cannot download packages.

Run a command without a model through the same offline tools:

```bash
printf 'python3 -m unittest -v' | ./target/release/meden exec --workspace /path/to/project
```

`exec` returns JSON with the retained workspace path and execution result.
Applying/exporting reviewed changes back to another location is still a
separate, unfinished workflow.

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

## Product direction

Meden's controller will preserve accepted requirements, choose supported
verification backends, use counterexamples to guide repairs and allow
delivery only when the required evidence covers the exact artifact. Local
inference and local verification protect project data; external services
require explicit authorization. The TUI, CLI and tools share one controller.

The target controller will be specified in Lean, with refinement proofs
connecting its production Rust implementation. Extending those proofs
across Meden's own TUI, CLI and tool behavior is part of the product goal.
The transfer decision kernel now has Rust-to-Lean refinement proofs; the
remaining controller and effect paths are still implementation work.

[Architecture and implementation order →](docs/design.md) ·
[Assurance and refinement obligations →](docs/assurance.md)

## Confidential development

The confidentiality requirement remains:

> Reading an external page must not enable the agent to send confidential
> files to an unapproved recipient.

The agent can read tool results normally. The target boundary consists of
an isolated coding runner, a fixed model gateway, a local page cache and
host-approved outbound transfers. It does not use an isolated planner or
ask a model to decide whether its own action is authorized.

The exact-transfer decision kernel now has seven Lean theorems, including
full-request matching and no replay after reservation, plus two refinement
theorems connecting the generated Rust model to that specification. A
Linux runner now confines actual TUI file/Bash tools and `exec`. Tests use
real host files, inherited descriptors, sockets and receivers. Connecting
every remaining effect and confining local inference are still work in progress.

[Confidentiality design and acceptance criteria →](docs/confidentiality.md) ·
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
