# meden

Conversational coding agent.

```bash
npx meden tui
```

Choose an OpenAI-compatible HTTP model, give it an instruction, and work on
real workspace files. The TUI retains conversation between instructions and
can use model connections discovered from Hermes configuration.

The first product requirement is that reading an external page must not
enable the agent to send confidential files to an unapproved recipient.
That boundary is planned, not implemented. The current optional
`--allow-shell` runs commands with the current account's filesystem and
network access. See the
[design and current limits](https://github.com/grandchildrice/meden#meden).

```bash
npx meden providers
npx meden tui --workspace /path/to/project
```

## What it does

The conversation engine reads tool results, proposes calls and checks the
current capability policy before execution. Workspace tools list/read,
write and edit files. Bash is opt-in. This is a development prototype and
does not yet prevent disclosure within granted capabilities.

The isolated planner, skill attestations and command-line model wrappers
have been removed. The CLI exposes `tui`, `providers` and `run` only. Existing
Lean/Kani checks cover the active capability/policy kernel, not the planned
end-to-end confidentiality boundary.

## Installing

`npx` fetches the binary for your platform from the GitHub release. To build
from source instead:

```bash
cargo install --git https://github.com/grandchildrice/meden meden-cli
```

Set `MEDEN_BIN` to use a binary you built yourself.

## License

Apache-2.0.
