# Meden

A conversational coding agent that reads files, external documentation and
tool results to decide what to do next.

The first security requirement is:

> Reading an external page must not enable the agent to send confidential
> files to an unapproved recipient.

The [coding agent design](docs/design.md) defines the requirement, trusted
components, outbound authorization, implementation work and acceptance
criteria. The target uses one conversation architecture. The isolated
planner, quarantined reader and their product entry points have been removed.

**Status: development prototype. The confidentiality requirement is not yet
implemented.** The current TUI can edit files and run commands, but its
optional Bash tool has the current account's filesystem and network access.
Capability checks and workspace path checks do not establish the new
requirement. See [SECURITY.md](SECURITY.md).

## Run the current TUI

```bash
cargo build --release -p meden-cli
target/release/meden tui --workspace /path/to/project
# Explicit provider/model, using the configuration discovered from Hermes:
target/release/meden tui --workspace /path/to/project \
  --provider hermes:ollama-cloud --model minimax-m3
# Enable the current unrestricted development shell when needed:
target/release/meden tui --workspace /path/to/project --allow-shell
```

The TUI has real directory listing, UTF-8 file reading, file writing and
exact text replacement tools. It retains conversation across instructions.
Shell execution is off by default and is not an OS sandbox when enabled.

Enter sends, Alt-Enter inserts a newline, arrow keys and Home/End edit the
instruction, and Ctrl-U clears input. Up/Down recall previous instructions
in single-line input and move between lines in multiline input. Japanese
and emoji are handled at grapheme boundaries, and the input follows the
cursor. `/clear` starts a fresh conversation while retaining workspace
files. PgUp/PgDn scroll the transcript. Esc or Ctrl-C requests cancellation;
running commands are stopped, while a current model request may finish.
Ctrl-C again exits.

The TUI currently supports OpenAI-compatible HTTP model providers. Hermes
configuration discovery is a convenience; the future design additionally
requires authorization before sending confidential context to a provider.
There is no web-reading tool or outbound transfer broker in the current TUI.

## Target architecture

The conversation engine proposes tool calls. The host executes them through
an isolated coding runner, a local external-page cache, an authorized model
gateway and an outbound transfer broker. Shell commands and tests have no
independent network path. Transfer authorization belongs to the user/host;
a page or model cannot grant it. External requests, model context, remote
tools, uploads and diagnostic services are all relevant outbound channels.

These boundaries are planned. The current `--allow-shell` implementation
and direct HTTP clients must be migrated before the requirement can be
claimed. Existing Lean/Kani checks cover their specified kernel rules, not
this future end-to-end architecture.

## Code layout

| Path | Current responsibility |
| --- | --- |
| `crates/meden-agent` | Conversation loop, JSON tool API and registry |
| `crates/meden-cli` | TUI, input editor, workspace tools, development shell and JSON Lines host protocol |
| `crates/meden-llm` | Model clients, provider discovery including Hermes configuration, and managed process lifecycle |
| `crates/meden-core` | Capability/policy kernel used by tool dispatch |
| `python/` | AgentDojo adapters, measurement runner and real PTY/live-model TUI checks |
| `lean/` | Formal policy/glob specifications and the conformance oracle |
| `docs/design.md` | Current coding agent design and acceptance requirements |
| `results/` | Recorded measurements with their configuration and limitations |

The CLI has three commands: `tui`, `providers` and `run`. `run` is a single
conversational JSON Lines interface for hosts such as the AgentDojo adapter.
There are no plan modes, CLI model wrappers, skill admission, attestations
or compatibility aliases. Tool arguments/results use JSON directly.

## Measured utility

Same model (`minimax-m3`) and four workers:

| AgentDojo suite | Meden conversation engine | AgentDojo baseline | Passes per agent |
| --- | --- | --- | --- |
| banking | 44/48 | 42/48 | 3 |
| Slack | 18/21 | 18/21 | 1 |
| travel | 14/20 | 12/20 | 1 |
| workspace | 33/40 | 31/40 | 1 |

These are benign-task results for the existing conversational runtime.
They do not establish the new confidentiality requirement, injection
resistance or the utility of the planned isolated coding runner. A live
model also completed a code repair and feature addition across two TUI
turns, with independent passing tests and restored terminal settings.
See [raw evidence and comparison settings](results/README.md).

## Development

Build, verification and contribution rules are in
[CONTRIBUTING.md](CONTRIBUTING.md). Work takes place directly on `main`.
The retired implementation remains available in Git history; measured
historical reports are preserved with their original configuration.

## License

Apache-2.0.
