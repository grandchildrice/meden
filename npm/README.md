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

The old isolated planner and its label-rule demonstrations remain in the
implementation during migration. They are retired from the product design;
their Lean/Kani proofs do not apply to the conversational runtime's
end-to-end confidentiality. See the
[repository](https://github.com/grandchildrice/meden).

## Installing

`npx` fetches the binary for your platform from the GitHub release. To build
from source instead:

```bash
cargo install --git https://github.com/grandchildrice/meden meden-cli
```

Set `MEDEN_BIN` to use a binary you built yourself.

## License

Apache-2.0.
