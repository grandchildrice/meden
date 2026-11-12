<p align="center">
  <img src="docs/assets/tui.png" alt="MEDEN's current welcome screen, with the large logo and selected repository" width="1080" />
</p>

<h1 align="center">MEDEN</h1>
<p align="center">
  Proof-driven development, with ease.<br />
  Formal verification through natural language.
</p>

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

Meden is a Rust coding agent for proof-driven development. Describe what to
build, review the specification, and follow the implementation and its evidence
in one terminal. It edits your repository directly and keeps context when you
reopen it.

## Getting started

Build and open the project you want to work on:

```bash
cargo build --release -p meden-cli
./target/release/meden tui --workspace /path/to/project
```

No model flags, environment exports or configuration file are needed to open
the TUI. If `meden` is on your PATH, running `meden` in your project directory
opens that directory directly.

1. **Choose a connection.** Select an existing host login, **Add HTTP connection**
   or **Use local model**. The HTTP form has Ollama Cloud's URL prefilled; enter
   a model and API key, or edit the URL for another OpenAI-compatible server.
2. **Review the destination.** For HTTP inference, the TUI shows the exact
   destination and model before you allow it to receive project context.
3. **Describe your task.** For example: “Build a vending machine that rejects
   purchases when balance is insufficient.” Review its proposed specification
   before implementation and formal verification continue.

Connection settings are remembered per project. API keys are masked; enable
**Remember API key on this device** only if you want the key saved in a private
host settings file outside the repository. Otherwise the key stays in memory
for this session and the TUI asks for it again next time. Existing account
logins are reused without copying their tokens into these settings. Each new
HTTP session still requires a fresh destination review.

Use `/connect` to add or edit an HTTP connection, and `/model` to switch
connections or select local weights. Files, conversation and accepted meaning
survive reopening the same project; a fresh check is required to show verified again.
[Interactive setup and usage →](docs/tui.md)

**Requirements:** Rust, Linux 5.6+, Bash, Bubblewrap and permission to create
user namespaces. [Linux runner setup →](docs/release.md#exact-source-to-staged-archives)
The first TUI launch automatically prepares pinned verification tools and can
take several minutes. That preparation needs network access, Git, Curl, Tar,
XZ and Zstd; later launches reuse the installation. Project tools and proof
checks run offline, so project dependencies must be available locally.

## From unproved to verified

The specification/evidence card stays above the conversation. It distinguishes
accepted intent, missing proofs and current checked evidence.

**Before verification:** requirements are unproved and delivery is held.

<p align="center">
  <img src="docs/assets/evidence-unproven.png" alt="Accepted counter specification with unproved properties and delivery held" width="1080" />
</p>

**After verification:** the same requirements are proved and a checked package
is available. Sending it still needs your approval.

<p align="center">
  <img src="docs/assets/evidence-verified.png" alt="Counter transition verified with Lean, showing proved properties and a checked package" width="1080" />
</p>

These are actual TUI captures with a scripted, confined local model fixture and
real offline Rust extraction and Lean checks. They illustrate the workflow;
they do not measure live-model capability. [Reproduce the captures →](tools/capture-readme.py)

## Daily use

| Action | Key or command |
| --- | --- |
| Send / insert a newline | Enter / Shift+Enter (Ctrl+J also works) |
| Scroll | Mouse wheel or PgUp/PgDn |
| Inspect tool details | Ctrl+T |
| Inspect requirements and evidence | Ctrl+G or `/proofs` |
| Configure / switch a connection | `/connect` / `/model` |
| Continue a stopped task | `/retry` |
| Start a new chat, retaining files | `/new` |
| Clear chat and its view, retaining files | `/clear` |
| Find the working directory | `/workspace` |
| All shortcuts / quit | `/help` / `/quit` |

Ordinary tool and checker rejections feed bounded automatic repair. If work
must stop, the TUI explains why, what remains incomplete and what to do next.
Responses stream as they arrive; activity and elapsed time remain visible
while waiting. Source edits require fresh verification, and a model's success claim
cannot turn the host's proof status green.

## What is verified today?

This is a development prototype. The project backend checks accepted **pure
u64 Rust transitions** using Rust extraction and Lean refinement. Examples
include counters, ledgers and fee calculations. Its proof does not cover the
entire application, CLI or I/O. Tests and ordinary tool success are separate
from formal evidence. [Supported contracts and workflow →](docs/project-verification.md)

Local inference runs in an offline namespace with prepared llama.cpp/GGUF
assets. An approved HTTP connection may be remote; cloud success is not
evidence of local inference or confidentiality. The development baseline is
explicitly approved Ollama Cloud, followed by capable 100–600B local deployment.
[Model connections →](docs/local-inference.md) · [Security limits →](SECURITY.md)

Meden's processor, invocation, evidence and approval kernels have scoped
Rust-to-Lean refinement proofs. Connecting every adapter and proving the full
application remain work in progress. [Actual checks and proof boundaries →](docs/verification.md)

## Optional command-line configuration

<details>
<summary>Connection flags, automation and offline setup</summary>

You can still provide a connection directly:

```bash
meden tui --workspace /path/to/project \
  --base-url https://ollama.com/v1 --model your-model --api-key "your-key"
```

Use `--api-key-env OLLAMA_API_KEY` to read an existing key variable instead.
Host logins can be selected with `--provider codex`, `--provider claude` or
`--provider hermes:ollama-cloud`; `meden providers` lists available connections.

For a prepared local model:

```bash
meden tui --workspace /path/to/project \
  --local-engine /path/to/llama-server --local-model /path/to/model.gguf
```

`meden run` accepts JSON Lines for automation. `meden exec` runs an explicit
private snapshot, leaving its source unchanged. `meden setup` prepares the
verification bundle ahead of an offline session. `meden prefetch` prepares a
fixed documentation snapshot before confidential work.
[CLI reference →](docs/cli.md) · [Connection details →](docs/local-inference.md)

</details>

## For developers

The workspace contains the Rust CLI, shared controller/runtime, Lean
specifications and Python integration checks. Work on `main` and follow the
[contributing guide](CONTRIBUTING.md) for validation and proof regeneration.

[Design →](docs/design.md) · [Assurance targets →](docs/assurance.md) ·
[Recorded benchmarks and trials →](results/README.md) ·
[Release checks →](docs/release.md)

## License

[Apache-2.0](LICENSE).
