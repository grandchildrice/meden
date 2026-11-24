# GUI development

The GUI lives in `apps/web`: React, TypeScript, Vite, Material UI, assistant-ui
(including Markdown rendering) and React Flow. Node 22.12 or newer is required.

## Native project development

```sh
npm ci
npm run build
cargo build -p meden-cli
./target/debug/meden gui --workspace /path/to/project
```

After updating the checkout, run `npm ci` and `npm run build` again before opening
the GUI. `npm ci` installs the dependency graph in the current lockfile; existing
`node_modules` may belong to an older Vite version. `npm audit` reports dependency
advisories, while `npm audit fix` can change dependencies and the lockfile. Review
those changes separately. A missing `apps/web/dist` means the browser build has
not completed; `--assets` selects a completed build from another checkout.

Opening a workspace shows a VS Code style layout: a persistent file tree on the
left, a large project map in the center, and conversation/work history with its
composer on the right. The project-map tab stays mounted with its own camera.
Select a declaration to open a filename tab containing its source and reference
graph side by side. Source uses the existing browser Shiki Lean/Rust highlighting;
line numbers and selected source locations remain separate from token colors.
Selecting another declaration updates the source line; returning
to the project map preserves the scene and camera. At compact widths, **Files**
and **Chat** open side panels and the file split stacks vertically.
There are no separate graph/code/preview/settings mode tabs. Preview opens through
its play icon as a temporary editor tab. The explorer heading shows the shortened
workspace path and one refresh icon; the map has no refresh or project-whole button.

The map is built from the initial policy-bound source inventory and retained per
source revision. D3's treemap layout fixes every category/file rectangle once;
React Flow supplies the retained pan/zoom camera. Increasing zoom fades in nested
categories, files, then declaration nodes and reference lines in those same coordinates.
Zoom never dispatches extraction or switches views. A minimap keeps the current
position visible. The map contains lexical reference candidates, not compiler
proof evidence. The optional tools action **Extract compiled dependencies** recompiles
the selected source and its imports to extract elaborated dependencies (at most
32 files / 16 MiB). Its tooltip explains this scope. It has no permanent file-toolbar
button and is not required for source navigation;
a failed extraction does not remove the cached project map. The explorer refresh icon rebuilds
navigation after external edits; ordinary development turns also refresh the inventory.

The complete inventory is an application-owned generated read model. Its exact
content-ID query returns frozen bytes without repeating JSON serialization.
Refreshing or replacing it retires older query references. Source counts and
navigation observations cannot restore an inference approval or proof receipt.
The explorer separately fetches immutable directory listings: literal names,
including newlines/Unicode, are never decoded from formatted tool text. A folder
contains all observed entries up to 100,000, or an explicit local failure with
**Retry folder**; there is no silent 500-entry cutoff. Collapsing/reopening a
retained folder issues no command or body query. Directory/reader retention is
bounded independently at 64 entries / 64 MiB of serialized envelopes per owner.

Markers distinguish main tracked targets (star), theorems (diamond), lemmas (circle),
definitions (square) and axioms (triangle). Classes/structures have their own label.
The declaration keyword comes from the source; main markers are explicitly named
proof targets in saved work, not inferred mathematical importance or checked status.
Native file graphs have no center/scope dropdowns; select a node directly to navigate.

The model tag lives in the chat composer. Select it to choose this computer's
Hermes, Codex, Claude or HTTP server connection and model. Review the exact fixed
destination/model in the model picker before choosing **Use this model**.
Selecting provider/model updates the native binding directly; there are no list-refresh,
connection-selection or host-settings-refresh buttons. The picker retains the exact
endpoint and selected model within the fully visible approval heading. Credentials remain
on the host and never enter the browser or project files. Switching a connection
revokes its predecessor; stale review cannot approve the replacement. The header shows only Meden and compact navigation, without a duplicated project name
or connection warning banner. All product labels and accessibility names are English.
Conversation errors and stopped model work appear in chat. File reads never produce
chat activity, button-state changes or transcript scrolling. Conversation messages,
history and composer state are memoized from their actual presentation inputs;
inspection does not checkpoint conversation/portable history. Chat submissions are
queued until an existing host operation settles, with waiting shown only after a
chat send. Compiler progress, failures and cancellation stay with the file graph.
Opening a file displays its original UTF-8 source independently of semantic
preparation; only the graph pane waits for compilation, counterexample search
and English readings. The reader admits a complete input up to 1 MiB, publishes
it once by content ID, and derives 200-line pages locally with explicit EOF. Binary, unsupported-encoding, oversized and
failed reads have file-local outcomes, with Retry for failures. Other formats
can be read from the explorer without requiring an analysis backend. Paging
requires no host command or file/compiler/model I/O, even during compilation.
Status carries only reader references. Replaced, evicted or refreshed references
are unavailable; late responses cannot replace newer contents. Reader transfer
failures expose a file-local **Retry source** action. Explicit inspection still
observes complete files through the original policy-bound worker. Cached semantic observations remain bound
to their original inputs; they do not assert current proof authority.
Retained conversation streams have
session dividers. Work items, saved checks and tool records open through the composer’s
**Work history** icon; they no longer form task trees above the conversation.
Stopped responses use a compact inline disclosure at the end of the transcript.
Cancellation is not an error banner; actual failures retain an accessible inline alert.
Long user/assistant messages fold by rendered height, with Show more / Show less.
Drag either sidebar separator to resize it; arrow keys also resize it, and Home or
a double click resets the width. Width preferences are saved per workspace.
On compact screens the separators hide and the side panels use their normal drawers.

Optional `--provider YOUR_PROVIDER --model YOUR_MODEL` preselects a connection.
Use `meden providers` to list existing host connections, or supply
`--base-url URL --model MODEL --api-key-env NAME`. Open the exact URL printed by
the command (default port 4322, `--port 0` selects an available port).
The [local HTTP dependency patch](../vendor/tiny_http/MEDEN.md) counts queued
connections against idle worker capacity, so concurrent module fetches cannot
remain queued solely because earlier keep-alive readers stay open. Its scheduler
regression and the real-browser restart journeys exercise separate layers.
`--assets DIRECTORY` selects a built GUI from another working directory.
The browser shows the fixed workspace, exact inference URL and model before
approval. No inference happens before approval. Revocation/expiry is terminal;
select a new connection in the GUI to review a new gateway, or restart the host
command and open its new URL. Old gateways and grants remain terminal.
The URL's random session token moves into tab-scoped session storage and is
removed from the address bar; it never enters project files or exports.

Send natural-language instructions to edit/build/test the actual selected
repository with the same `Session`, tool registry, policy and offline workers
as the TUI. Native GUI views show file/directory reads, real tool arguments and
results, exposed reasoning/partial output, cancellation, stop explanations and
native project contract status. A contract review freezes its displayed body and
reference before accepting/rejecting it through the native controller. These
browser approval/rendering adapters do not acquire the TUI's refinement proofs.
Prepare the real verifier using `meden setup` before requesting formal proofs;
this gateway does not implement an automatic cold-install GUI yet.

Each Bash invocation has fresh temporary storage. Scripts/intermediate files
needed in later calls must live in the workspace, then can be removed there.
File tools accept workspace paths only. The tool descriptions explain this
boundary; ordinary `diff` is included among independently selected public tools.

Conversation and project journals remain host JSON files outside the writable
repository. Close the GUI command, then open `meden tui --workspace ...` to
continue; the same repository lock prevents concurrent native writers. Closing
the tab does not stop an active host turn: use **Stop** or stop the host.
SIGINT/SIGTERM cancels managed work and waits for the session checkpoint.
A new instruction starts an explicit new turn after a failure/cancellation;
provider failures are not silently retried. Unsent input remains tab/browser data.

**Check current evidence** rechecks the accepted source/evidence binding. File inspection
and completed turns also refresh status; polling alone is not a filesystem watcher.
Logs/partial reasoning in this initial adapter last for the host process, while
conversation text and admitted model history persist. **Previous page** /
**Next page** navigate the retained complete text without waiting for native
work; source line numbers identify the current range. Reader retention is bounded
by 64 files and 64 MiB of serialized envelopes. Retained text is an observation,
not a filesystem watcher or current evidence.
Native preview, selected-module Lean Atlas
extraction, delivery, Lake dependency discovery and separate-model meaning
judgment are described below with their current bounds.

## Portable work and check history

**Work history** stores a project's named work items, original requirements, theorem
names, next steps, progress notes and visible conversation in
`.meden/workspace.json`. Completed turns checkpoint the visible conversation;
**Save conversation and work** checkpoints it explicitly. Raw model reasoning, API keys, processor
grants and native accepted contracts are not exported to that file. Check the
visible conversation before publishing it: project text may contain private data.

Commit `.meden/workspace.json` together with the sources on your working branch.
On another machine, clone the branch and open it with Meden:

```sh
git clone --branch YOUR_WORK_BRANCH YOUR_FORK_URL project
meden gui --workspace "$PWD/project" --provider YOUR_PROVIDER --model YOUR_MODEL
```

Saved visible conversations already appear in chat. Use the task in chat or open **Work history**, select a task,
inspect its saved output or **Read proof source**,
and use **Resume this work item** after reviewing the new processor connection. The
selected task becomes a new user instruction with its original requirement,
next step, recent history and bounded excerpts of recent visible conversation.
The complete saved text remains available in the panel. Imported conversation is context;
it is not replayed as system messages or restored tool authority. Each device
keeps its credentials and native session data outside the repository.

**Recheck with Lean** needs no model approval. Install the exact distribution named
by `lean-toolchain` through Elan and prepare the project's pinned dependencies
before checking. An explicit host `MEDEN_PROJECT_LEAN` selects an installed
distribution instead. This adapter neither installs tools nor downloads dependencies.
Missing prerequisites remain visible failures. History is still readable without
Lean or a model connection.

The check runs `lake build --wfail MODULE` offline, then freshly elaborates the
selected source using Lake's original module setup and inspects each named
theorem's actual type and transitive axioms. It does not import the selected
module's old compiled artifact. A missing declaration, a definition
instead of a theorem, or an axiom beyond `propext`, `Classical.choice` and
`Quot.sound` fails inspection. The record includes the frozen requirement/names,
Lean executable digest, source/configuration fingerprints, termination and bounded
output. Changed inputs, truncation, interruption and failed inspection cannot
produce a successful record. Interrupted host processes may leave an unfinished
start event; retry explicitly. Malformed or oversized history is retained and
refused for import or overwrite.

These are observed Lake execution and declaration-inspection records. Repository
Lake configuration, executable Lean source, compiled dependencies, plugins, compiler behavior and physical
filesystem observations remain adapters. The records do not grant native contract
acceptance or project verification, and imported successes always require a local
recheck. Local checks invalidate previously displayed source/analysis/evidence
freshness because executable Lake configuration can edit the workspace.

The agent keeps resumed Lean work in its existing language and preserves the
task's requirement. `check_lean_work` takes a stored work ID and runs the same
offline compiler and fresh theorem inspection as the local check button. It
returns real diagnostics to the model for source repair and retains each check
in `.meden`. Lake and Lean can execute repository code, so the tool requires
process/shell execution with the existing writable, offline workspace mounts.
File scopes govern the separate metadata reads and history writes; they do not
restrict code admitted under shell execution. Those file operations use the
admitted policy. The host-selected compiler must already be
installed. Missing resources, cancellation and limits stop automatic repair;
ordinary completed compiler rejections remain repairable. Native contract
verification remains restricted to pure Rust transitions. The
[actual cloud repair trial](../results/portable-lean-ollama-cloud-minimax-m3.json)
exercises this path from a compiler rejection to a checked universal Lean theorem,
with saved conversation and an unapproved restart.

The public TUI supports `/work`, `/work ID`, `/work init NAME`, `/work save`,
`/work check ID`, `/work note ID -- TEXT` and `/work resume ID`. At connection
selection, `/` opens host commands without approving inference. For scripts,
`meden work --workspace PATH --help` lists the same operations and task editing.
`meden lake --workspace PATH --lean LEAN_ROOT -- build --wfail MODULE` exposes
explicit confined Lake execution; its command result alone is not proof authority.

## Native delivery review

Ask the agent to prepare a file for a specified HTTP destination. **Review delivery** appears beside the conversation only when a proposal exists and shows the
same immutable export queue as the TUI: `propose_export` prepares an ordinary file
only without a project contract; `propose_artifact_export` prepares the retained
checked package and requires current accepted meaning/evidence. Neither tool sends
or approves anything. No browser-supplied payload, destination or proof replaces
the retained request.

Open **内容と宛先を確認** to inspect the exact destination, method/content type,
payload length, SHA-256 and contract identity. Those fields remain above a separate
scrolling byte view; **Previous bytes** / **Next bytes** move through 2 KiB pages. Hex and
printable ASCII show the frozen bytes without interpreting them as HTML or terminal
controls. **Send once to this destination** and **Reject this delivery** operate on that displayed
reference and host review sequence. Sending is disabled if the complete heading
does not fit visibly. This browser geometry check is a tested UI adapter, not the
Crossterm predicate's Rust-to-Lean refinement or a display-authenticity proof.

Closing a review or starting another host operation discards the selected review;
reopen it before sending. A stale review sequence or a different session/export
cannot act on the current selection. Native dispatch checks current project source
and evidence again and reserves its one attempt before HTTP I/O. Responses show
their HTTP status, including errors and redirects; redirects are not followed.
Uncertain delivery remains terminal and instructs the user to inspect the receiver.
A response is not an assertion about the receiver's application-level processing.
There is no automatic resend. Queue entries last only for the host process and do
not return after restart.

Playwright exercises a real loopback receiver with binary snapshots, source edits
after preparation, fixed headings on desktop/mobile, clipped-heading refusal,
byte paging, stale selections, rejection, one-use delivery, redirects, uncertain
results and restart. Existing CLI/TUI delivery tests exercise the shared queue and
native controls independently. Host input provenance, browser rendering and physical
HTTP remain adapters; this integration does not complete whole R1.

## Native application preview

Ask the agent to create a browser application using local files. In **Preview**,
choose **Public folder (inside workspace)** (for example `public` for plain HTML/JS,
or `dist` for an already built Vite app), the entry HTML filename, then
**Reload files and preview**. The actual local application runs in the panel;
root-relative module imports, CSS, images and same-bundle JSON reads are supported.
Builds and tests still run through the ordinary offline tools. This static preview
does not start a development server, install packages or support backend endpoints,
external services, browser storage or service workers.

The panel retains its running document when switching tabs. Explicit rereading
starts a new document from fresh file observations. A new development/contract
continuation closes the previous preview. A failed or cancelled capture does not
retain the previous bundle. Reloading the browser reopens the host's frozen bundle;
restarting `meden gui` requires rereading. Opening the new session URL at the same
port reloads the document so the new response policies apply. Missing requested
assets appear beside the preview; that diagnostic does not detect every JavaScript
failure. A loaded iframe or successful interaction never grants proof status.

The preview description is fetched once per published update; status polling
contains only its frame and revision. **Retry preview** retries a failed
description transfer without rereading files or resetting the running document.
Missing-asset diagnostics update that same frame. Explicit recapture retires its
previous origin; long diagnostic details expand separately so the app stays visible.
A delayed former description cannot restore it. Static capture
and immutable descriptions belong to the shared application; the loopback
listener and browser response policies remain host adapters.

The existing policy-bound, offline file worker reads a bounded inventory and exact
binary bytes. Its read-only policy limits both original and resolved paths to the
selected folder; root aliases resolving elsewhere are refused. Hidden entries,
symlinks, special files, `node_modules` and `target` are excluded. Limits are
256 supported web files, 64 directories, depth 8, 4 MiB per file, 16 MiB total,
500 entries per directory and 90 seconds overall. Exceeding a limit fails rather
than serving a truncated success. Folder names containing policy wildcards `*`/`?`
are refused. Each bundle's SHA-256 binds the selected root, entry, file paths and
observed bytes. Concurrent host changes are not an atomic filesystem snapshot.

A separate loopback listener serves only these frozen bytes. Every capture gets
a fresh random `.localhost` hostname capability; old capabilities are invalidated.
Requests perform no project filesystem lookup. The document uses an opaque-origin
`sandbox="allow-scripts allow-forms"` iframe and response sandbox policy, without same-origin,
popups, top navigation or workers. Client-side form submit handlers work; CSP
`form-action 'none'` blocks actual form navigation and network submissions.
Resource CSP limits scripts/styles/images
and fetch to the frozen origin (inline scripts/styles and data images are supported).
The parent permits frames only on this loopback preview listener. Browser tests
check blocked cross-origin fetch/image/script/form loads and access to the GUI document,
storage and API, plus wrong-host/method/path rejection.

These are browser adapter constraints, not an OS egress firewall or an offline
browser worker. Native coding/inference/checker profiles remain offline; the
preview runs in the user's browser. Browser behavior, other communication APIs,
extensions and whole R1 confidentiality remain separate obligations. The preview
adds no Rust-to-Lean proof, application correctness or source-to-UI correspondence
claim. In particular, a JavaScript mirror of checked Rust is still a separate
implementation unless a further binding is established.

The permanent **Project map** is always the default editor tab. The file tree
reads actual directories, including non-Lean sources; clicking a file opens its code
in the center. File tabs show their basename with the complete path available in the
editor heading/title. Declaration selection prepares the same code tab at the
recorded line while leaving the camera untouched. Native contract review and exact
send review remain separate host-controlled operations in the conversation.

Saved conversation, work and tool records have a separate immutable read model.
`/api/state` returns their revision and native message count. The authenticated
same-origin `/api/history?revision=…` query returns the complete cached original
records for that exact revision; an obsolete reference receives HTTP 409.
Status polling and file/map navigation do not repeatedly transfer history.
The browser retains its projection, aborts superseded reads and exposes an
inline **Retry history** action for a failed read. These presentation records
cannot restore a model grant or proof evidence. The initial read still includes
the complete bounded history, rather than paging it.

Prepared source/graph data is separate too: `sources` in `/api/state` contains
only a current publication reference and references by file. Authenticated
`/api/source?revision=…` returns that complete frozen body when it is still
retained, otherwise HTTP 409. Its content identity includes explanations and
scope, independently of the original source/graph IDs used for explicit
analysis. Returning to an open file reuses its immutable browser data; a failed
body read has **Retry graph** beside the file. History and source share bounded
conflict recovery, including clear/republish under the same content ID. A query
does not read files or run compilation. Restart requires a new host observation;
view caches do not restore approval or native evidence.

Accepted/proposed contract reviews and evidence observations use their own
`/api/evidence?revision=…` publication. Status reports only its availability and
exact revision; polling does not transfer the canonical contract or full review.
**Retry evidence** recovers a failed transfer without checking sources or sending
an instruction. The complete review freezes for its selected original reference;
restarting retains accepted meaning but requires a fresh evidence observation.
The displayed evidence strength remains the original backend's stated scope.

The index reads through the pinned, policy-bound offline file worker. Read-only batches
retain each original request and check both original/resolved paths under the same policy;
partial/failed worker output is refused. These batches and the navigation parser are
tested I/O adapters, not new Rust-to-Lean refinements. Hidden entries, symlinks, special
files, `node_modules`, `target`, `dist` and `__pycache__` are excluded. Bounds are 10,000
source files for full declaration indexing, 1 MiB per file, 128 MiB total,
65,536 directories, depth 32 and 300 seconds;
each directory inventory is bounded at 10,000 entries and the traversal at one million.
Failures and cancellation retain chat and show a retry action; no partial complete graph
is installed. Completed model file/command checks trigger a fresh read of this overview.
Concurrent host changes are not an atomic filesystem snapshot.

The **Lean tools** icon opens optional single-file selection and a closed
multi-file/environment section. Ordinary file-tree and map-node navigation needs
no filename input or extraction action. **Extract compiled dependencies** starts an explicit compiler
job in the file split; **Refresh references** captures complete selected sources without
compiler import traversal. A job cannot silently replace a source candidate with
proof evidence. Tool inputs/results and checkpoint notices live in a closed inline
chat record, without a separate execution-log button.
Source observations use the same offline,
policy-bound file worker as project snapshots (1–32 files, 1 MiB per file,
16 MiB total). No truncated file is indexed. A SHA-256 binds the selected paths
and complete observed content. Failure discards the previous graph; starting a
new development turn also requires rereading sources. A snapshot lasts for this
host process, and reload can redisplay it; restarting the host requires rereading.
It is an observation, not a filesystem watch or an atomic concurrent-host snapshot.

`meden-app::view::SourcePage` projects lexical declarations and unambiguous
local references from the shared source index. Nested comments/literals are masked, Lean
namespaces and private declarations are tracked, and unknown edge kinds remain
conservative. Imported modules, elaboration, type-class resolution, macros and
full dependency completeness are outside this index. Every indexed declaration
starts **Proof unchecked**; source `sorry`, audit-looking comments or model output
cannot produce proof evidence. Select a node and open its exact highlighted
source line in the shared syntax-highlighted source reader. The
following recorded-demo documentation describes a separate standalone mode.

## Large source inventories

Main targets remain visible as fixed-size stars while zoomed out. Marks aggregate
by file and include saved targets, compiled main-theorem attributes and theorems in
the conventional `Main.lean` entry module; hover shows the provenance. Stars indicate
location rather than proof status.

Opening a Lean file prepares compilation, deterministic meaning and counterexample
observations before its graph/source page appears. Conventional Main entry modules
are warmed during workspace preparation; a previously unseen file may need its first
preparation. Warm visits reuse full snapshots after source/configuration/resource
validation. The optional extraction action remains available for an explicit rebuild.
There is no source-English preview or unrun-search message presented as an analysis.
A preparation failure remains local to the file, with a retry and the original source.
Explicit graph requests render their own saved source directly; selecting a node or
paging that source starts no duplicate inspection or compilation.

The installed Informath 0.4 / Grammatical Framework adapter generates verbal English;
the structural Lean renderer remains the base for unsupported forms. Information
icons and graph hovers show saved **Meaning**, a separate **Counterexample search**
outcome and a table of candidate values. Falsification conditions and provenance are
collapsible. A false input to a predicate definition is a boundary example, not a
refutation of its definition. Sampling without a candidate is not proof.
Prepare the pinned optional external renderer with
`uv run --project python python tools/prepare-informath.py` on Linux x86-64.
[Research implementation, adapter interface and limits](research/2026-11-22-observation-adapters.md).

Projects with more than 10,000 Lean/Rust sources retain a complete inventory (up to
250,000 files and 100,000 entries per directory) and prepare a disk source-map cache
before the map becomes ready. Reopening reuses the prepared inventory; explicit refresh checks file metadata
and rereads changed sources. The initial browser inventory has unloaded declaration details; file-count
weights keep coordinates stable. Zoom fetches small prepared chunks through a read-only
API instead of starting a worker or refetching the whole inventory. It preserves the
map camera, source tab and chat. Each chunk binds the exact requested inventory
and selected path; old-context queries are conflicts. Complete envelopes are
cached up to 64 chunks / 64 MiB per immutable inventory. First chunk reads use
prepared artifacts, while cache hits return existing bytes without artifact I/O.
Nearby import-only barrels are included, at most 32 files per chunk. Cached navigation errors are file-local and refresh repairs changed
sources; ambiguous module suffix matches are not chosen. This is lexical navigation,
not a whole-project Lean/proof build. The actual Riemann corpus has 175,187 files.

## Native Lean Atlas extraction

With an installed Lean distribution, opening a local Lean file prepares its compiled
dependencies automatically. The optional tools action **Extract compiled dependencies**
requests an explicit rebuild. The first preparation automatically compiles the pinned public Atlas library offline without sending
source to a model. The public TUI `/atlas` command performs the same preparation.
Meden selects the workspace's `lean-toolchain` from installed Elan toolchains,
including a custom `ELAN_HOME`. Without a project pin it uses Elan's default,
or a complete public distribution on the host PATH. It never runs a repository
executable or invokes Elan to download a missing version during discovery.
If the requested version is missing, the GUI shows its exact Elan install command;
after installing it, choose **Rediscover** without restarting Meden. A different installed
version cannot silently replace a missing project pin.
`MEDEN_ATLAS_LEAN=/absolute/lean-root` remains an optional explicit host override.
The public `meden setup-atlas --workspace PATH` command prepares the same library as the button.
Existing corrupted Atlas resources are refused; open **抽出ツールの修復** and choose
**Prepare dependency extraction** to rebuild them. This maintenance action is unnecessary for the
normal first extraction.
It does not download Lean or replace the host's ordinary toolchain.
`meden source-notices` displays the bundled Atlas and claimcheck source notices.

A Git clone normally excludes `.lake/packages`. Prepare the project's pinned
dependencies before compiled extraction. For EthLib and ordinary Mathlib projects,
the repository's initial setup is `lake exe cache get` from that project's directory.
This is a separate host preparation step; Meden's coding and analysis workers stay
offline. Missing dependencies show their name, expected location and a preparation
hint. They do not trigger a download or admit a partial compiled graph.

For a trusted local Lake workspace with installed dependencies, the host can select
the prepared-build adapter:

```bash
MEDEN_ATLAS_BACKEND=lake meden gui --workspace /path/to/project
```

Opening a file runs offline `lake build +Module` before preparing its page. Lake's
resolved source and reported compiled import artifacts are fingerprinted;
unchanged inputs reuse saved analysis, while dependency edits trigger a rebuild.
This profile executes project configuration/source with writable workspace access
during preparation. Extraction then mounts the workspace and Atlas resources read-only.
Both phases stay offline, without host credentials or an unconfined fallback. It is a
trusted host choice, not a model-selectable network executor. Initial dependency
downloads remain a separate host operation.

Only selected modules become graph nodes; imported library declarations remain
explicit unexpanded boundaries. This avoids the frozen collector's 32-source limit
without claiming a complete Mathlib graph. Preparation does not start a whole-project
build when the map opens. Selecting a large root can still require a long compilation;
compiler failures remain in its file pane. Exact source/configuration/resource and
compiled-artifact checks precede cache reuse, including after restart.

File preparation streams actual compiler output into the file pane, with the
current step, elapsed time and **Stop preparation**. Logs remain visible after a
timeout or cancellation, and **Retry preparation** reuses completed Lake outputs.
Successful pages retain a collapsed **Build log**. Preparation status is a generated,
resource-local application view. Polls carry the step, bounded durations, typed
outcome and a log reference; the retained diagnostic tail is fetched on a changed
reference or an explicit local retry. **Retry log** retries the read without restarting compilation.
Logs are bound to the original preparation and resource, so a superseded response
cannot populate a newer job. At most 64 preparation observations are retained;
replacement, workspace refresh and restart retire old log queries. This is display
state, independent of the kernel's consuming jobs and admitted evidence. Diagnostic history keeps the
latest 64 KiB; failure summaries are capped at 8000 UTF-8 bytes and explicitly
marked when shortened. These diagnostics do not appear in the conversation or grant proof status.
Open file tabs retain their observed source, graph and explanations in memory.
Graph hover, wheel gestures and resizing preserve the camera; the passive legend
passes pointer events through. Fit View remains an explicit reset. Selecting a
line already in the source page keeps its syntax tokens and scrolls to the line.
See [interaction measurements](gui-performance.md) for the performance audit.
Returning to a retained tab or its graph declaration does not inspect,
compile or validate dependencies again; local source paging also needs no worker.
The file tree also reuses open tabs. Refresh and chat edits invalidate
retained observations; returning to an invalidated tab prepares fresh inputs.
Navigation is presentation only:
an explicit meaning-analysis request rechecks the frozen source/configuration,
resources and prepared dependency fingerprint before making model requests.
Lake builds use a separate two-hour default budget; the trusted host can set
`MEDEN_ATLAS_BUILD_TIMEOUT_SECONDS` from 1 to 86400. Extraction retains its own
ten-minute ceiling. Counterexample observation workers have a ten-second wall
limit and share a 120-second file budget. Their Lean preparation additionally
caps each declaration at 10000 heartbeats and instance synthesis at 2000.
Exhausted or unavailable observations stay explicit; they do not discard the
compiled graph or prevent the next eligible declaration's observation within
the remaining file budget. These are search limits, not proof of a statement.

Select files from the left tree without typing a path. Start with one file in a
large project, such as `EthLib/Networking/Gossip/Parameters.lean` or Riemann's
`src/Statements.lean`. The project map remains available independently of compiler
preparation. An unavailable file reports the actual requested path.

The default `MEDEN_ATLAS_BACKEND=frozen` adapter copies a bounded source closure.
Selected modules are observed through the existing policy-bound file worker,
then mounted read-only in a fresh offline compilation job. Only fresh `.olean`
files from this job are imported. Existing build outputs,
unselected project files, host credentials and network services are not supplied.
Lean code can run macros, initializers and `#eval` inside this job; it is untrusted
executable analysis input, rather than Meden's restricted contract-data language.
The prepared library and fixed scripts are separate read-only resources. There is
no unconfined fallback. Import/compile/metadata/OS attribution remains an adapter.

The [MIT-licensed extraction core](../crates/meden-app/src/atlas/vendor/README.md)
is pinned to Lean Atlas `06be1eb76fb926b8cf8bd42a4a51208d128c327d`. Its actual
constant classifier and type/value dependency functions run on the imported
environment. Meden selects the observed modules, retains cross-module edges,
adds compiler-printed statements/definition values and transitive axiom names,
and records references outside those modules separately. Standard library and
other external definitions are not expanded. **Whole project** includes disconnected
declarations; the focused views retain their own dependency cones.

This path parses import headers with installed Lean and collects project-local
imports using the evaluated Lake source directories, or the workspace module root
when no Lake configuration is present. Leave the target field blank to discover
Lake library/executable roots, including configured module globs. It supports ordinary ASCII
module names and at most 32 files in the entire closure, under the 1 MiB/file and 16 MiB
source limits. Module paths use ordinary ASCII identifier components and cannot
replace the prepared Lean/Std/Atlas modules. Imports require either another
observed source module or the prepared Lean/Atlas libraries. Lake configuration
resolves local path dependencies and existing manifest package sources inside the
selected workspace. Missing packages are not fetched. Compilation/extraction has a shared 180-second budget;
truncated/failed/interrupted output produces no graph. Graph output is bounded
to 5,000 declarations and 100,000 internal edges. Resource hashes bind the selected
Lean executable, Lake shared library and prepared Atlas files; this is not a complete hash of the Lean
distribution, dynamic libraries, OS or launcher.

The graph retains the observed source hash plus a separate extraction revision
covering graph data and prepared-resource hashes. Preparation is cached outside
the model-writable project. Changed resources fail validation; the GUI can prepare
them again. Changed/missing source or compilation failure discards the old graph.
Restart retains the public prepared library but requires fresh source extraction.
`sorry` is displayed as an incomplete proof. Other declarations remain proof-unknown:
successful compilation and Atlas metadata do not admit a native proof receipt.

Meaning analysis from an Atlas graph uses the exact selected declaration and its
server-derived meaning scope. Theorem proof-only edges are pruned, definition value
edges remain, and compiler-printed types/values replace raw source and comments in
the translation request. Axiom and external-reference information remain explicit.
Both source and extraction revisions must match. Prepared resources and selected
source files are checked again around inference; the final context is bounded to
1 MiB without truncation. Lean parsing, pretty-printing, extensions, malicious source
effects and extraction authenticity are not established by Meden's control proofs.

## Native meaning analysis

After reading a source graph, select a declaration, enter **Requirement to compare** and choose
**Analyze declaration meaning**. The reviewed processor receives two separate requests:
first the selected declaration plus observed local files (or the elaborated Atlas
scope above), without the separate
requirement or conversation; then the requirement plus the back-translation,
without the formal source or conversation. The same host-selected model handles
both requests. They are separate contexts, not independent-model evidence.

This uses a Rust adapter of claimcheck's two-pass prompt/output approach, pinned
at `8eb37fedc0acb5bc5f89efc3933cb3955268a4bd`. Source attribution and the MIT
license are in `crates/meden-app/src/semantic/`. It does not run the upstream SDK,
CLI or Dafny verifier. No extra credential discovery, network client or
model-selected execution is added. Only the two structured result functions are
decoded as data; unexpected tool calls/fields/targets are refused, never executed.

The result shows the frozen requirement, literal back-translation, preconditions,
postcondition, scope and discrepancy/reasoning. **Candidate match** and **Candidate mismatch**
are model assessments. A semantic match never marks a declaration proved, accepts
native meaning or changes proof/export authority. Low-confidence or contradictory
matches remain **Undetermined**. This interface does not establish analyzer accuracy.
Lexical mode is not complete elaborated scope; its source comments can still expose
intent even though the separate requirement is withheld from translation. Atlas
mode omits those source comments and theorem proof scripts from translation.

The host freezes path/content SHA-256, target and exact requirement, reobserves
all selected files before and after inference, and checks cancellation and the
processor grant before admitting a result. A concurrent source change, missing
file, cancelled turn, provider failure or malformed result leaves no current
success. No automatic inference retry occurs. Editing the displayed requirement
or choosing another declaration labels the old result as a different target.

Every run is retained in host `meaning-analysis.json` next to the existing JSON
journals, outside the model-writable project. The atomic, locked local history
contains exact requirements, target/source/graph revisions, processor/model,
outputs and failure/cancellation states. Requirement IDs hash the exact original
text; changing that text creates another ID. Every invocation has a fresh random
analysis ID; mismatches and unclear assessments additionally have a Finding ID.
Reanalysis never overwrites a previous finding. The history selector restores
past assessments for reading without sending inference or renewing approval.
The TUI has `/analysis history` and `/analysis ID` (analysis or Finding ID); JSONL
has `analysis_history` and `select_analysis` commands.

Restart labels completed results historical and requires rereading sources,
fresh processor review and explicit rerun. Interrupted runs are not resumed.
The journal stops at 1000 runs or the existing 16 MiB storage limit, preserving
old entries instead of deleting them. An unreadable or older optional checkpoint
is reported and retained; explicitly rerunning analysis preserves that file as
`meaning-analysis-retained-*.json` before creating a new history. Ordinary project
development remains available. This is tested adapter behavior, not a new
Rust-to-Lean refinement or an atomic concurrent-filesystem proof.

New runs also retain their original source/Atlas snapshot under host
`analysis-sources/<content-sha256>.json`; each record links its exact artifact.
Repeated identical snapshots are shared. A changed retained artifact stops a new
run instead of overwriting it. Historical data never restores a live graph or
approval; fresh worker observations still gate execution.

For a retained mismatch/unclear Finding, **Prepare patch for this Finding** requests
one data-only repair to an observed source file. The original requirement and
accepted native contract stay fixed. Review the destination, content hash,
Finding/requirement identities, exact diff and explanation, then reject or apply
once. The authority heading remains above scrolling details; clipped headings
cannot approve. In the TUI, `/repair` opens the same frozen repair with
`y` to apply/reanalyze and `n`/Esc to reject.

Sources and Atlas resources are observed again; changed sources or accepted
contract identities refuse the old proposal. The actual admitted file policy
and typed request reach the ordinary offline worker, whose full-file comparison
refuses content changes. Applying invalidates native project proof coverage;
reanalysis uses the original requirement, freshly observed sources and, for
Atlas, a fresh compilation/target location. A candidate match never restores
proof evidence. Applied files survive failed/cancelled reanalysis, which must be
explicitly rerun. Source-only repairs preserve the selected declaration's line
anchor; Atlas repairs can relocate it by exact declaration name. Repair logs
under host `repairs/` retain the diff, original Finding/input, selected processor
and model, terminal outcome and resulting analysis ID. Restart discards all
pending repair authority. This is a bounded single-file flow, not a claim that
model repairs are correct or that filesystem concurrency is formally proved.

A separate live browser smoke exercises real application edits, an interactive
generated browser app, independent original Rust tests, a live model's proposal
and exact one-use delivery to a synthetic loopback receiver, the source graph, matching and mismatching requirements,
then restart/history with the explicitly selected cloud provider:

```sh
npm run build
cargo build -p meden-cli
node --experimental-strip-types tools/gui/live-native-smoke.ts \
  --binary target/debug/meden --provider YOUR_PROVIDER --model YOUR_MODEL \
  --out /tmp/meden-live-gui-review
```

This driver does not mock inference, alter private configuration or repair
project files outside the GUI. It writes `report.json` and milestone screenshots.
Passing these synthetic cases is not a general accuracy, local-inference or
cold-verifier-installation claim.

Add `--atlas` to ask the model for a standalone Lean analysis module, prepare the
real Atlas library through the GUI, compile that module and compare its elaborated
scope instead of the lexical Rust graph. This requires installed Lean as described
above. The generated Lean model does not establish correspondence with the Rust
application.

The [recorded native GUI cloud trial](../results/native-gui-development-ollama-cloud-minimax-m3-afa18b7.json)
binds the tested source commit and binary hash to generated source, three browser
boundary interactions, independent original Rust tests, one exact reviewed delivery,
matching/mismatching meaning candidates and historical restart. It also records the
preceding failed trials that exposed scratch-file and form-handler friction; those
failures were not counted as successful validation. That earlier trial used lexical
Rust graphs; selected-module Atlas extraction is a separate path described above.
A separate comparison model is now selectable with `--judge-model MODEL` on the
same host-selected HTTP connection. Both models/destinations appear in the fixed
processor heading and receive separate immutable session grants. Changing providers
requires a fresh review. Cancellation, revocation, expiry and invalid judge replies
stop analysis without a fallback. The journal records both bindings and retains a
completed blind translation when comparison fails. Different model IDs do not
establish statistical independence or attest provider weights. This option currently
requires HTTP; the ordinary confined local-model path remains separate.

The [selected-module Atlas cloud trial](../results/native-gui-atlas-ollama-cloud-minimax-m3-70faa07.json)
completed the application journey, compiled a model-written Lean module with
Lean 4.35.0 and obtained the expected threshold mismatch/match through the actual
graph-bound two-pass adapter. Its raw candidate text also introduced an unsupported
currency unit, guessed at an omitted proof and contained malformed prose. These
observations are retained: correct verdicts on two examples do not establish
general semantic accuracy or reliable explanations.

## Recorded demo: start

```sh
npm ci
npm run dev
```

Open `http://127.0.0.1:4320`. The server binds to loopback and refuses to select
another port silently. The home screen starts with a large message input and a
Send button. There is no workspace-name form. Fonts and assets are bundled;
the application does not load fonts, analytics or models from a CDN.

```sh
npm run build
npm run preview --workspace @meden/web
```

The production build is written to `apps/web/dist`.

## Layout and journeys

The left toolbar button opens or collapses the conversation sidebar, which
contains history, message entry, search and settings. The upper-right
“Open artifact” button opens the **Code / Preview / Proof** sidebar. Narrow
screens use drawers.

| Goal                            | Entry and result                                                                                                                                                                                                                                   |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Start a conversation            | Send an instruction with the arrow or Enter; its title is automatic. Shift+Enter adds a newline.                                                                                                                                                   |
| Read actual development history | Open the vending-machine conversation in the sidebar. Scroll to the supplied opening request or expand tool activity inline with its message.                                                                                                      |
| Inspect recorded execution      | Expand each tool call for its actual arguments and output, including the Lean recursion-limit failure, verification retries, source edits and seven passing CLI tests.                                                                             |
| Inspect a proof                 | Open Proof: the radial graph shows related declarations, including incoming references. Select a node for its statement, status and direct dependencies.                                                                                           |
| Open exact source               | Click the statement or file/line link; Code opens the retained file at the highlighted line. Return to Proof with selection intact.                                                                                                                |
| Follow dependencies             | Select a declaration to inspect it; double-click to center its immediate uses and users. Pan and zoom remain available.                                                                                                                            |
| Inspect proof coverage          | Open the proof-count control for the eight property records. Source-only helpers without individual audit entries remain unconfirmed.                                                                                                              |
| Run the meaning-analysis GUI    | Choose “Meaning analysis” inside Proof. A cancellable sequence opens the NG example at the ¥150 purchase threshold. The result puts the boundary case above the compared text. Switch between “Show matching example” and “Show mismatch example”. |
| Edit or request review          | From the meaning result, inspect the retained requirement/theorem record, edit a draft or prepare a revision-bound review export. No extra top-level tab is added.                                                                                 |
| Preserve work                   | Chat, drafts and edits survive reload. Failed storage retains the in-memory conversation and offers retry/export. Invalid imports keep current work.                                                                                               |

The vending conversation card shows the Rust implementation and Lean prover
logos and opens the preview from anywhere on the card. Its list shows plain
Japanese summaries of the eight audited contract properties, their Lean theorem
names, and the same statuses as the Proof tab.

## Home screen logos

The logo strip below the message suggestions uses the language and formal
verification assets from slide 5 of the Meden strategy pitch
(`meden-strategy/pitch/assets/evidence/v4/logos/`). The original PNGs are
bundled with the web app. The strip shows examples from the ecosystem.

## Recorded meden-demo example

The snapshot contains meden-demo's actual Rust kernel, CLI, tests, README,
retained Lean specification/candidate/proof, contract and matching report. The
original checkpoint has **57 messages**, from which **13 visible messages and
25 tool calls/results** are projected in order. The original opening request
provided by the user is prepended verbatim with `origin: provided`; the initial
displayed conversation therefore has 14 messages. Its missing assistant reply
and initial tool runs are not reconstructed. Activities attach after their
preceding visible message; source indices retain checkpoint order. Tool input
and output are recorded content, not reconstructed thoughts or new execution.

**Reasoning text is absent from this recording.** The native TUI can display
provider-exposed reasoning deltas, but its saved presentation transcript selects
only user/assistant message lines. The retained model-message objects in this
checkpoint contain no reasoning fields either. See
[`tui.rs`](../crates/meden-cli/src/tui.rs) (`delta` and `save_history`) and
[`history.rs`](../crates/meden-cli/src/session/history.rs). The GUI marks this
absence and does not invent reasoning. Persisting future exposed reasoning is a
separate native-adapter change, not implemented here.

The capture excludes host-injected continuation prompts and connection settings.
Only project tools are whitelisted; session/workspace/credential fields are
removed from structured outputs. Message timestamps are the recording's
last-modified time, not reconstructed individual event times.

The kernel SHA-256 is
`c14e1efc673ef51eae41ac0b435a7233abb005092d48119c965595f780c2ff83`.
The capture selects a retained report matching that hash:

```sh
uv run tools/gui/capture-meden-demo.py
```

`--project` and `--state` select explicit local inputs. Review the snapshot before
committing it. Existing browser copies of this exact bundled recording refresh
the graph and original activities while keeping chats and requirement edits.

The graph indexes **53 declarations / 150 references** from three saved Lean
files, including refinement and helper theorems omitted by the earlier view.
It resolves known project names in source text and classifies occurrences before
or after the declaration's definition boundary. This is a **lexical source index**,
not elaborated Lean dependencies, an Atlas/Compass execution, or complete standard
library coverage. Imported Atlas graphs continue to use their supplied edges.
The related-declarations view follows both incoming and outgoing references;
individual dependency views remain directed.

Eight property proof results are archived, source/input-bound verification
records for pure `step`. CLI parsing, display and I/O are outside that scope.
Opening a panel does not rerun Lean. The browser vending preview mirrors the
transition in TypeScript/BigInt, with ordinary unit coverage.

Meaning analysis is a **GUI simulation**. The progress sequence does not run an
analyzer or change stored proof/analysis evidence. The initial NG sample compares
a requirement allowing purchases at ¥150 or more with a hypothetical theorem
that requires more than ¥150. Both require stock. At exactly ¥150 with one item
in stock, the outcomes differ. The OK example uses the saved requirement text.
Runs can be cancelled. Revision changes invalidate the displayed run.
Imported proof progress/no-sorry metadata is never upgraded to fresh evidence.

The information button describes the planned two-pass approach, based on
claimcheck. A translation LLM sees the Lean theorem and renders it in natural
language without seeing the original request. A second LLM compares that text to
the request and flags missing or extra conditions.

## Storage and runtime boundary

Conversations, requirement edits and review drafts use IndexedDB in this browser;
unsent input and selection preferences use local storage. Clearing site data
removes this storage. Portable workspace/review exports are unencrypted.

Browser database version 3 normalizes the bundled vending project on every read
and save. Its existing continuations merge into one conversation, and messages
sent from home continue that same thread. This also repairs version-2 data and
prevents the earlier one-time-migration behavior from creating new forks. The
selected continuation and active unsent input follow the canonical conversation.
Other projects remain separate; original activities and repeated messages survive.

Settings provides workspace export, JSON import and conversation deletion. Send
appends a local demo response and opens the relevant panel. There is **no model call,
Lean execution, native contract acceptance or capability grant** in browser code.
Recorded and demo messages retain distinct origins in the shared schema. Editing
requirements only changes browser drafts, not the Rust controller's accepted
meaning. The existing TUI/runtime remains the execution authority.

The standalone demo uses the browser repository port; `meden gui` uses the
implemented native loopback gateway and existing bounded host JSON persistence. See the [architecture map](architecture/semantic-workspace.md).

## Interaction references

The layout follows documented chat/product interactions:

- [ChatGPT history search](https://help.openai.com/en/articles/10056348-finding-your-chats-projects-and-files-in-chatgpt): search and return to conversations from the sidebar.
- [Claude conversation management](https://support.claude.com/en/articles/8230524-delete-or-rename-a-conversation): conversations belong in the left sidebar.
- [Claude artifacts](https://support.claude.com/en/articles/17153992-what-are-artifacts-and-how-do-i-use-them): an artifact opens beside the conversation, keeping dialogue and output adjacent.
- [OpenAI's canvas introduction](https://openai.com/index/introducing-canvas/): a separate work surface alongside chat can open automatically when useful. This is an interaction precedent, not a claim about the exact current screen geometry.

These references inform the layout; the Meden proof sidebar and nested comparison are our
adaptation. Controls and copy follow Google's Material Design 3 guidance on
[UX writing](https://m3.material.io/foundations/content-design/style-guide/ux-writing-best-practices),
[adaptive layout](https://m3.material.io/foundations/layout/layout-overview) and
[accessibility](https://m3.material.io/foundations/accessible-design/overview).
Material UI is the React implementation, not a Google-authored UI framework.

The radial interaction is informed by
[Lean4SemanticBench](https://0xfab0131.github.io/Lean4SemanticBench/).
[Lean Atlas' graph format](https://github.com/NyxFoundation/lean-atlas/blob/main/web/lib/types.ts)
provides the import boundary and type/value distinction.
[claimcheck](https://github.com/metareflection/claimcheck) informs the requirement
versus blind back-translation view, without claiming integrated analysis quality.

## Refined dependency explorer

File dependency views use the actual MIT-licensed layout functions from Lean4SemanticBench:
uses above the center, used-by below it, parent barycenter ordering and adaptive radial
spacing. The default shows one hop, as L4SB does; double-click a node to center its
neighborhood. The project map remains the complete category/file navigation surface.
The previous radial scope modes and their dropdown/history toolbar have been removed.
Atlas meaning-review coloring walks from saved/main targets (or the focused theorem),
excluding only known theorem-body dependencies and retaining definition-body dependencies.
Unknown edge kinds remain unknown. No target means unknown scope. The colors label
review scope, not whether a person checked it or whether native proof evidence was admitted.
Hover and focus emphasize adjacent references while preserving the node's
position and hit area.

“Expand graph” or “Expand reader” opens a full-height graph with a docked reader.
Its statement and syntax-highlighted Lean source scroll independently beside the
graph. Code no longer depends on a clipped preview beneath the canvas. Source
links retain exact line selection; “Back to conversation” returns to the chat. At small widths
the reader uses the full expanded surface. [Source review and algorithm notes](research/2026-11-18-graph-ui.md).

## Checks and reviewed screens

```sh
npm run typecheck
npm test
npm run build
npm run schema:check
npm run format:check
cargo build -p meden-cli
npm run test:gui
```

The browser gate builds once and serves the production bundle on loopback port
4321, separate from the interactive development server on 4320. Screenshots,
traces and the HTML report are retained under `target/gui-checks/`. On Linux CI,
install Chromium with `npx playwright install --with-deps chromium`; on this
NixOS host the config uses installed Chrome. `MEDEN_GUI_CHROME` overrides its path.

Native browser journeys execute the actual binary and offline workers with a
scripted HTTP provider: file creation, provider failure, retained context/restart,
fresh processor review, revocation, silent waiting, cancellation before dispatch,
concurrent-turn refusal and origin/Host/token checks. A native graph journey covers
local source observation, exact-line navigation, development changes, failed-read
recovery, reload and compact accessibility. They are not live-model or
cold-verifier-install evidence. The Python UX gate also exercises GUI→TUI→GUI
checkpoint handoff through a real PTY.

Recorded-demo browser journeys cover send/title/history/reload, independent sidebar controls, preview actions, graph
inspection, freshness, scoped export, import errors, storage failure/retry,
legacy-history migration/restart, compact layout and keyboard entry. Axe checks home, conversation, all three
sidebar tabs, source navigation, algorithm popover and semantic differences against automated WCAG 2.1 AA rules. This is not a complete manual
or assistive-technology audit. The required [public TUI gate](ux-testing.md)
remains separate; scripted fixtures are not live inference evidence.

Earlier GUI captures

[Expanded graph](assets/gui/expanded-graph.png) · [Full-height source](assets/gui/expanded-source.png) · [Opening request](assets/gui/history-start.png) ·
[Home](assets/gui/home.png) · [Actual execution history](assets/gui/history.png) ·
[Proof graph](assets/gui/proof.png) · [Selected proof](assets/gui/proof-detail.png) ·
[Lean source](assets/gui/code.png) · [Meaning difference](assets/gui/analysis.png) · [Algorithm](assets/gui/semantic-method.png) ·
[Preview](assets/gui/preview.png) · [Compact home](assets/gui/compact-home.png) ·
[Compact preview](assets/gui/compact-preview.png).
The [check record](assets/gui/verification.json) records scope and limits.

Import collection evaluates frozen `lakefile.lean` or `lakefile.toml` with the
installed Lake package loader inside the offline analysis worker. It never calls
Lake dependency materialization, update, build hooks or host Lake execution.
Local path dependencies and root-manifest packages already present inside the
workspace are discovered. Installed Lean/Atlas library modules remain explicit
external boundaries. Missing/ambiguous sources or conflicting manifest entries stop.
The frozen graph records configuration/manifest contents, probed absences, module
identities and glob inventories; changes invalidate analysis and pending repairs.
Cached source contents are observed and hashed; their Git commit is not attested.
A missing import fails collection and clears the current graph. Selected roots,
collected files and external module names are retained with the source artifact.

The [two-model TUI repair trial](../results/tui-meaning-repair-gptoss120-exact-vocabulary.json)
used minimax-m3 for translation/repair and gpt-oss:120b for comparison. It detected
the 150/100 threshold mismatch, reviewed the frozen diff, applied it once,
reanalyzed the same requirement, passed independent offline Rust boundary tests
and restarted without restoring approval. The preceding
[retired-model failure](../results/tui-meaning-repair-qwen480-retired.json) and
[invalid-category failure](../results/tui-meaning-repair-gptoss120-retained-failure.json)
are retained too. Naming exact schema vocabulary in the comparison prompt enabled
this successful retry under a new host review; parsing remained strict with no
fallback. These synthetic cloud trials use an installed verifier prerequisite,
not cold setup, local inference or general quality validation.

The [Lake cloud trial](../results/tui-lake-ollama-cloud-minimax-m3-gptoss120.json)
used the public TUI to prepare real Lean/Lake, discover four modules across local
and prepopulated manifest packages, compare `Domain.eligible` through minimax-m3
and gpt-oss:120b, observe the expected 150/100 mismatch, refuse an older graph after
a missing input, recover through `/atlas` and restart with fresh processor review.
It retained the exact graph/source/configuration snapshot and unchanged fixture
files. This is a synthetic-data cloud workflow, not Git-origin attestation, native
project proof, cold installation, general semantic accuracy or local inference.
