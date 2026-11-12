# Interactive development

Run `meden` in your project directory, or select a directory explicitly:

```bash
meden tui --workspace /path/to/project
```

The current directory is the default workspace. Meden writes development edits
directly there; it does not silently move the project into another working copy.
Reopening the directory restores conversation and accepted specification meaning.

## Choose a connection

The picker offers existing host connections and two setup actions:

- **Add HTTP connection:** enter an OpenAI-compatible API base URL, model and
  optional key. Ollama Cloud's URL is prefilled. Enter advances through the
  fields; Enter on the API key proceeds to destination review.
- **Use local model:** select a prepared `llama-server` executable and GGUF model
  (or the first split shard). This uses the offline inference profile and has no
  cloud fallback. Model downloads and GPU/multi-host setup are not provided here.
  Paths may be absolute, start with `~/`, or be relative to the selected project.
  Saved paths are made absolute while retaining their original symlink/shard names.

Tab/Shift+Tab and Up/Down move between fields. Ctrl+U clears the active field.
Paste is supported; newlines cannot submit a field or approve a connection.
Esc cancels settings. `/connect` reopens HTTP settings; `/model` opens the picker.

HTTP setup ends at the ordinary processor review, which shows the actual URL,
model, context scope and lifetime. Only its local, unmodified `y` key permits
inference. Editing fields, pressing Enter or pasting `y` does not approve it.
New connection parameters create a fresh gateway; an earlier approval is not
reused for another destination. Loopback alone does not establish confidentiality.

## Remember settings

After selection/approval, preferences are stored per project in
`$XDG_DATA_HOME/meden/projects/<project-id>/connection.json` (default data root:
`~/.local/share`). This file is outside the model-writable repository. The
directory has mode 0700 and atomic settings files have mode 0600 on Linux.

URLs, models and local asset paths are remembered automatically. Keys are masked
and session-only by default. To save one, Tab to **Remember API key on this device**,
toggle it with Space and continue with Enter. This stores the key in the private
host settings file; it is not an OS keychain. Existing account logins are remembered
by connection ID and model without copying their tokens into that file.

Without key persistence, the next launch opens the saved connection's key field.
Changing the URL clears the old key and its remember checkbox. Saved HTTP
preferences always require a new processor review in each session. They cannot
restore proof coverage, jobs, export approval or another session's processor grant.
An unavailable saved login or invalid settings file produces a visible picker
message; it does not silently authorize another connection.

## Work and inspect evidence

Describe the task in the message box. The specification/evidence card distinguishes
host-accepted intent, unproved properties, checker activity and current proof.
`/contract` reviews the proposal; `/proofs` or Ctrl+G shows statements, assumptions,
source/artifact identities and remaining scope. Edits invalidate current evidence.
The supported backend checks pure u64 Rust transitions; it does not prove CLI/I/O
or the whole project. See [project verification](project-verification.md).

Recoverable tool errors return to the bounded conversation loop. Stops retain a
host explanation and a next step; `/retry` starts a new host turn using existing
files/results. Ctrl+T expands tool details, `/copy` copies the response or stop
explanation, and `/help` lists implemented editing/navigation controls.

Explicit launch flags remain available for reproducible scripts. `--base-url`
selects only its explicitly supplied key; existing subscriptions use `--provider`.
Headless `meden run` requires local assets or an explicit HTTP selection and exact
`--approve-processor` URL. [CLI protocol](cli.md) · [Connection details](local-inference.md)
