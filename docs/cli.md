# CLI automation

`meden run` accepts natural-language instructions as JSON Lines and executes
the same coding session as the TUI. Both use the same system instruction,
registered tools, capability policy, private workspace and approved model
gateway. The original project is unchanged; the result copy is retained.

```bash
printf '%s\n' '{"instruction":"Inspect the project, fix the bug and run its tests."}' \
  '{"instruction":"Add boundary tests for the corrected behavior."}' | \
  meden run --workspace /path/to/project --provider configured \
    --approve-processor http://localhost:11434/v1/chat/completions
```

Configure the provider as described in the [README](../README.md). The
approval must match its exact inference URL. It permits that processor to
receive this session's conversation, file contents and tool results for up
to eight hours; a loopback endpoint alone does not prove local inference.

## Input

Each nonblank line is one object with exactly one field:

```json
{"instruction":"Read the implementation, make the change, and verify it."}
```

The limit is 1 MiB per line, including JSON framing and its newline. Embedded
newlines in an instruction use JSON string escapes. Unknown fields fail;
input cannot provide tools, policy, system instructions, checker success,
processor approval or tool results. The old host-tool protocol is removed.
The first input is validated before provider discovery or workspace import.
Later inputs are processed in order after the previous turn finishes.

## Output

Each output line is a JSON object whose `type` identifies an event:

| Type | Contents |
| --- | --- |
| `ready` | Retained `workspace` path and actual `processor` URL |
| `asking` | Model request attempt within the current turn |
| `assistant` | Model text |
| `decision` | Checked tool name, whether allowed, and refusal rule/reason |
| `tool_result` | Tool name, returned text and error flag |
| `conversation` | Exact model history after the turn |
| `done` | `outcome`, optional `answer`, optional `error` |

`done.outcome` is `done`, `failed`, `incomplete` or `cancelled`. Check it:
exit zero means protocol processing completed, not that every task or test
succeeded. Tool errors can be returned to the model for repair. A broken
output stream cancels the conversation; it does not authorize another
effect. End stdin to finish the session.

The CLI runs tools itself. Events are observations, not requests for a host
to execute code. The fixed tools are `read_file`, `list_files`, `write_file`,
`edit_file` and `bash`. All file/process operations use the offline copy at
`/workspace`; unsupported send/network tools are refused. There is no host
callback, plugin registration or unrestricted fallback.

Output can contain confidential history and file contents. It is local
session data; an integrating application is responsible for any onward
transfer it chooses to perform. The CLI escapes JSON output and filters
terminal control characters in human-facing errors/provider listings.

The shared controller, parser and OS effects are not yet fully refined in
Lean. [Current verification](verification.md) records exactly which kernel
functions are proved and which production behaviors have integration tests.
