# CLI automation

`meden run` accepts natural-language instructions as JSON Lines and executes
the same coding session as the TUI. Both use the same system instruction,
registered tools, capability policy, offline repository workers and approved model
gateway. Changes are written directly to the selected `--workspace`. Reopening
that same directory restores its saved model conversation and accepted meaning;
processor grants and proof coverage require fresh checks/approval. `meden exec`
is the explicit private-snapshot command.

Progress may include `notice` records describing a bounded recovery pass.
Terminal `done` records retain the typed `outcome` and raw `error`, and add an
`explanation` for failed, incomplete or cancelled work. This host explanation
does not require another model request and does not establish verification.
An explicit new `instruction` continues a stopped task; the TUI exposes this
as `/retry`, retaining current files and conversation context.

```bash
printf '%s\n' '{"instruction":"Inspect the project, fix the bug and run its tests."}' \
  '{"instruction":"Add boundary tests for the corrected behavior."}' | \
  meden run --workspace /path/to/project \
    --base-url http://localhost:11434/v1 --model your-model \
    --approve-processor http://localhost:11434/v1/chat/completions
```

The default headless `run` path uses [confined local inference](local-inference.md), configured
with `--local-engine`/`--local-model` or `MEDEN_LOCAL_ENGINE`/`MEDEN_LOCAL_MODEL`.
It performs no HTTP discovery and needs no external processor approval.
For the explicit HTTP path above, add `--api-key KEY` or `--api-key-env NAME`
if authentication is needed. `--base-url` and `--model` select the connection
without environment exports or provider discovery. Keys from other connections
are not inherited. The approval must match its exact inference URL. It permits
that processor to receive this session's conversation, file contents and tool results for up
to eight hours; a loopback endpoint alone does not prove local inference.

Subscription connections use `--provider codex` with an approved `/responses`
URL, or `--provider claude` with an approved `/messages` URL. They share the
same session, tools and processor review; login sources and provider limits
are described in [subscription connections](local-inference.md#subscription-connections).

## Input

Each nonblank line is one host command with exactly one field. Instructions
use:

```json
{"instruction":"Read the implementation, make the change, and verify it."}
```

The limit is 1 MiB per line, including JSON framing and its newline. Embedded
newlines in an instruction use JSON string escapes. Unknown fields fail;
input cannot provide tools, policy, system instructions, checker success,
processor approval or tool results. The old host-tool protocol is removed.
The first input is validated before provider discovery or workspace import.
Later inputs are processed in order after the previous turn finishes.
Stdin is a trusted user/controller channel. Do not forward model or tool
text into it as control commands.

Workspace `write_file` creates regular files. Parent directories must exist;
the model can create them with offline Bash. Writing empty content creates
an empty file, not a directory. Missing parents and parents of the wrong
type are reported explicitly, and the workspace directory itself cannot
be replaced with a file.

## Documentation cache

Fetch the exact host-selected documentation batch before starting confidential
work. No model chooses the requests or their order:

```bash
meden prefetch --url https://lean-lang.org/ \
  --url https://docs.python.org/3/library/unittest.html > pages.json
meden tui --workspace /path/to/project --pages pages.json
# Or add --pages pages.json to meden run.
```

`prefetch` emits one JSON object only after the entire batch succeeds. At most
eight distinct URLs are accepted, each at most 8192 bytes. Requests are HTTPS
GETs with empty bodies, without URL credentials/fragments, environment proxies,
cookies or automatic redirects. The resolver rejects nonpublic/special-use IPs
and mixed public/private answers before TCP connection; the checked socket
addresses are passed directly to the connector. TLS identity validation stays
enabled. It does not load scripts, links, images or other subresources.
Responses must be successful UTF-8 documents, at most 1 MiB each; content-type
metadata is limited to 1024 bytes. A failure does not output a partial cache.

The cache is loaded once into host memory before workspace import/inference.
It cannot be changed by editing the workspace or the cache file later. Loading
it performs no DNS/network requests. `list_cached_pages` returns the local
handles and metadata; `read_cached_page(id, offset?, limit?)` returns inert text.
Offsets count Unicode scalar values from zero; the default chunk is 16000
characters, maximum 32000, with `next_offset` for continuation. A missing handle
returns an error, and URL/approval arguments are rejected. There is no fetch
tool, reload command or cache-miss fallback. `/clear` and processor changes
retain the same documentation snapshot.

Treat cache files as trusted host configuration and all document contents and
response metadata as untrusted. This JSON format does not authenticate a page's
origin or prove its contents. Do not run `prefetch` with model-generated URLs
after reading confidential data: the URL itself can disclose that data. The
prefetcher runs in a dedicated Linux namespace with an empty scratch directory,
private process/filesystem views, a cleared environment and closed inherited
file descriptors. It receives only the bounded URL batch on stdin. The managed
workspace, host home and credentials are absent. Public tools/libraries and the
host resolver configuration are read-only mounts. Missing isolation fails;
there is no host-fetch fallback.

This worker retains the launching network namespace for DNS and HTTPS. Its
trusted HTTP client enforces the public-address policy; this is not a kernel
egress firewall. Coding, inference and verification workers remain offline.
The IPv4/IPv6 numeric predicates have Rust-to-Lean refinement; URL/IP decoding,
DNS resolution/connector attribution, TLS, JSON and OS isolation remain tested
adapters and trust assumptions. See [verification](verification.md#fixed-documentation-prefetch-and-cache).

## Exact file exports

The model may call `propose_export(path, target)`. This snapshots at most
1 MiB inside the offline worker; it does not approve, resolve DNS or send.
This ordinary-file path requires a session without a project contract.
After a contract is proposed or accepted, use `propose_artifact_export(target)`
for the current verified kernel. A raw file queued before that contract cannot
be sent while the project requirement exists.
Eight proposals are allowed per session, including resolved proposals;
references are never reused. The returned reference contains `session`
and `id`, binding it to that workspace session and immutable request.

The host can then issue:

```json
{"list_exports":null}
{"review_export":{"session":"SESSION_FROM_PROPOSAL","id":0}}
{"approve_export":{"session":"SESSION_FROM_PROPOSAL","id":0}}
```

`export_review` returns the actual target, method, content type and complete
frozen body as `body_hex`. Review must precede approval. To decline, send
`reject_export` with the same reference; rejection needs no prior review.
Model tool calls named `approve_export` or `review_export` are unsupported.

Project proposals contain a `contract` SHA-256 in their host summary; ordinary
files have `contract: null`. A project payload is JSON with
`format: "meden-verified-kernel"` and a `files` map from names to byte arrays.
It preserves the exact retained `report.json`, native library, source,
extraction and proof inputs. Raw retained content is limited to 4 MiB and the
encoded package to 16 MiB. The model supplies only the destination; it cannot
choose another file, report, checker or proof context for this operation.

Approval permits one POST with those exact frozen payload bytes. Project
source and retained files are inspected again before dispatch to require
current matching evidence. Edits, pending replacements, active jobs, changed
contracts or damaged retained files block dispatch. Restoring discarded
coverage requires a fresh independent check. The HTTP adapter follows no
redirect, uses no credentials or environment proxy,
and does not retry automatically. Cross-session references, rejected or
already attempted proposals cannot send. `delivery_uncertain` means the
recipient may already have received the bytes; approval stays spent.
Targets require HTTPS, except literal loopback HTTP, without userinfo or
fragments; the URL limit is 8192 bytes.
An HTTP response is not proof that an upload was accepted by the service.
This is a scoped kernel-package path, not whole-application assurance or
Git/email audience adapters. Raw file exports carry no correctness claim.

## Project contract and verification

After `meden setup` (also automatic on the TUI's first launch), the model can use
`propose_contract` to propose a bounded pure-u64 Rust transition contract.
The host reviews and accepts the
exact reference returned in `project.summary.reference`:

```json
{"review_contract":{"session":"SESSION_FROM_PROPOSAL","revision":1,"contract":"SHA256_FROM_PROPOSAL"}}
{"accept_contract":{"session":"SESSION_FROM_PROPOSAL","revision":1,"contract":"SHA256_FROM_PROPOSAL"}}
{"project_status":null}
```

Copy all three reference fields from the proposal, including its `revision`;
the example number is not a default. Review must precede acceptance.
Acceptance continues development automatically.
Only this host channel and the TUI review can accept; model tools cannot.
`verify_project` and `project_status` model tools take empty arguments. The
former invokes the independent fixed backend and requires host acceptance.
Before acceptance it returns a recoverable tool usage error directing the
model to `propose_contract` and host review; it starts no verification job.
`project.summary.verified` describes the current project kernel, independently
of the model answer or `done.outcome`. Source changes invalidate evidence.
The summary names the retained artifact directory and its hashes. There is
no arbitrary receipt callback or model-selected checker/verification path.
The summary includes the actual proposal and retained accepted contract.
To reject a replacement, send `reject_contract` with its exact reference;
this retains the previous accepted meaning and artifact, subject to a fresh
source/status check. A stale reference cannot accept another draft, including
identical contract bytes proposed again after rejection: each fresh proposal
has a new revision and needs its own review.
See [contract, preparation and proof scope](project-verification.md).

## Meden release evidence

`meden verify-release --repository owner/repo --commit FULL_40_HEX_COMMIT
--directory dist` is a host administration command for Meden's own releases.
It reads staged archives and source/checksum sidecars, contacts only the
fixed GitHub API for exact-source CI evidence and emits a manifest only
after the evidence gate permits it. `GH_TOKEN` or `GITHUB_TOKEN` is optional
authorization for that API. Archive/source contents are not uploaded.

This command is outside the conversation's model tools and `run` protocol;
there is no model-controlled receipt callback or API destination. A model
invoking it inside a coding worker still has the worker's offline boundary.
It does not establish verification of the user's project. See
[release inputs, policy and trust assumptions](release.md).

## Output

Each output line is a JSON object whose `type` identifies an event:

| Type | Contents |
| --- | --- |
| `ready` | Retained `workspace` path and actual `processor` URL |
| `asking` | Model request attempt within the current turn |
| `assistant` | Model text and original `tool_calls`, emitted before tool execution |
| `decision` | Original call `id`, checked tool name, whether allowed, and refusal rule/reason |
| `tool_result` | Original call `id`, tool name, returned text and error flag |
| `conversation` | Exact model history after the turn |
| `done` | `outcome`, optional `answer`, optional `error` |
| `project` | Host contract state, current kernel verification and artifact/source hashes |
| `exports` | Current proposal summaries |
| `export_review` | Summary, method, content type and exact `body_hex` |
| `export_result` | Summary, optional `http_status`, and optional error |

`done.outcome` is `done`, `needs_review`, `failed`, `incomplete` or `cancelled`.
`needs_review` means the host stopped after a contract proposal; review and
accept the exact reference to continue. `done` means the model returned a
final answer; it may have answered without executing a tool. It does not
establish task completion or project verification. Use the host's current
project status and bound artifact report to check verification.

Process exit zero means protocol processing completed, not that every task or test
succeeded. Tool errors can be returned to the model for repair. A broken
output stream cancels the conversation; it does not authorize another
effect. End stdin to finish the session.

`assistant.tool_calls` preserves each call's `id`, `function.name` and raw
`function.arguments` string, including malformed JSON. An unfinished turn can
therefore retain the actual proposals without waiting for `conversation`.
The model history also retains the original argument strings; it does not
parse and reformat them when echoing a previous assistant call. Execution
validates arguments separately.
Call IDs come from the model and need not be unique; retain event order too.
These are local observations of untrusted content, not host commands or
verification evidence. Treat the stream as confidential session data.

The CLI runs tools itself. Events are observations, not requests for a host
to execute code. The fixed tools are `read_file`, `list_files`, `write_file`,
`edit_file`, `bash`, `propose_export`, `propose_artifact_export`, `list_cached_pages`, `read_cached_page`,
`propose_contract`, `verify_project` and `project_status`.
File/process operations use the offline copy at
`/workspace`; unsupported send/network tools are refused. There is no host
callback, plugin registration or unrestricted fallback.

Output can contain confidential history and file contents. It is local
session data; an integrating application is responsible for any onward
transfer it chooses to perform. The CLI escapes JSON output and filters
terminal control characters in human-facing errors/provider listings.

The combined export lifecycle/reservation kernel has transition and
finite-composition refinement proofs for exact emitted requests and at most
one effect. Its consuming dispatch accessor and exact destination/body getters
are refined too; the approval-to-send path has no JSON codec. Origin attribution,
URI normalization, the shared I/O controller, parser and OS effects
are not yet fully refined in Lean. [Current verification](verification.md) records which kernel
functions are proved and which production behaviors have integration tests.
