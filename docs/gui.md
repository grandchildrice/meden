# GUI development

The GUI lives in `apps/web`: React, TypeScript, Vite, Material UI, assistant-ui
(including Markdown rendering) and React Flow. Node 22.12 or newer is required.

## Native project development

```sh
npm ci
npm run build
cargo build -p meden-cli
./target/debug/meden gui --workspace /path/to/project --provider YOUR_PROVIDER --model YOUR_MODEL
```

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
restart the host command and open its new URL to review a new session.
The URL's random session token moves into tab-scoped session storage and is
removed from the address bar; it never enters project files or exports.

Send natural-language instructions to edit/build/test the actual selected
repository with the same `Session`, tool registry, policy and offline workers
as the TUI. Native GUI panels show file/directory reads, real tool arguments and
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
the tab does not stop an active host turn: use **実行を中止** or stop the host.
SIGINT/SIGTERM cancels managed work and waits for the session checkpoint.
A new instruction starts an explicit new turn after a failure/cancellation;
provider failures are not silently retried. Unsent input remains tab/browser data.

**現在の証拠を確認** rechecks the accepted source/evidence binding. File inspection
and completed turns also refresh status; polling alone is not a filesystem watcher.
Logs/partial reasoning in this initial adapter last for the host process, while
conversation text and admitted model history persist. The file reader uses the
native tool's bounded first 200 lines. Native preview, selected-module Lean Atlas
extraction, delivery and meaning analysis are described below. Full-project Lake
dependency discovery and independent-model judging remain integration work.

## Native delivery review

Ask the agent to prepare a file for a specified HTTP destination. **配送** shows the
same immutable export queue as the TUI: `propose_export` prepares an ordinary file
only without a project contract; `propose_artifact_export` prepares the retained
checked package and requires current accepted meaning/evidence. Neither tool sends
or approves anything. No browser-supplied payload, destination or proof replaces
the retained request.

Open **内容と宛先を確認** to inspect the exact destination, method/content type,
payload length, SHA-256 and contract identity. Those fields remain above a separate
scrolling byte view; **前の内容** / **次の内容** move through 2 KiB pages. Hex and
printable ASCII show the frozen bytes without interpreting them as HTML or terminal
controls. **この宛先へ1回送信** and **この配送を拒否** operate on that displayed
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

Ask the agent to create a browser application using local files. In **プレビュー**,
choose **公開フォルダ（ワークスペース内）** (for example `public` for plain HTML/JS,
or `dist` for an already built Vite app), the entry HTML filename, then
**ファイルを読み直してプレビュー**. The actual local application runs in the panel;
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

The native **グラフ** tab takes one `.lean`/`.rs` path per line; **このファイルのグラフ**
in the code panel uses the current path. Source observations use the same offline,
policy-bound file worker as project snapshots (1–32 files, 64 KiB per file,
1 MiB total). No truncated file is indexed. A SHA-256 binds the selected paths
and complete observed content. Failure discards the previous graph; starting a
new development turn also requires rereading sources. A snapshot lasts for this
host process, and reload can redisplay it; restarting the host requires rereading.
It is an observation, not a filesystem watch or an atomic concurrent-host snapshot.

`@meden/atlas-adapter/source-index` builds lexical declarations and unambiguous
local references from those files. Nested comments/literals are masked, Lean
namespaces and private declarations are tracked, and unknown edge kinds remain
conservative. Imported modules, elaboration, type-class resolution, macros and
full dependency completeness are outside this index. Every indexed declaration
starts **証明は未確認**; source `sorry`, audit-looking comments or model output
cannot produce proof evidence. Select a node and open its exact highlighted
source line in the shared syntax-highlighted source reader. The
following recorded-demo documentation describes a separate standalone mode.

## Native Lean Atlas extraction

With an installed Lean distribution, select the **グラフ** tab, choose **依存抽出の準備**,
enter the local `.lean` module paths, then **Leanでコンパイルして依存を抽出**. Preparation
compiles the pinned public Atlas library offline without sending source to a model.
The default host selection is installed Elan Lean 4.35.0. To use another installed
distribution, set `MEDEN_ATLAS_LEAN=/absolute/lean-root` before launching Meden.
The public `meden setup-atlas` command prepares the same library as the button.
It does not download Lean or replace the host's ordinary toolchain.
`meden source-notices` displays the bundled Atlas and claimcheck source notices.

Selected modules are observed through the existing policy-bound file worker,
then mounted read-only in a fresh offline compilation job. Only fresh `.olean`
files from this job are imported. Project Lake files, existing build outputs,
unselected project files, host credentials and network services are not supplied.
Lean code can run macros, initializers and `#eval` inside this job; it is untrusted
executable analysis input, rather than Meden's restricted contract-data language.
The prepared library and fixed scripts are separate read-only resources. There is
no unconfined fallback. Import/compile/metadata/OS attribution remains an adapter.

The [MIT-licensed extraction core](../crates/meden-cli/src/atlas/vendor/README.md)
is pinned to Lean Atlas `06be1eb76fb926b8cf8bd42a4a51208d128c327d`. Its actual
constant classifier and type/value dependency functions run on the imported
environment. Meden selects the observed modules, retains cross-module edges,
adds compiler-printed statements/definition values and transitive axiom names,
and records references outside those modules separately. Standard library and
other external definitions are not expanded. **プロジェクト全体** includes disconnected
declarations; the focused views retain their own dependency cones.

This path parses import headers with installed Lean and collects project-local
imports from the selected workspace module root. It supports ordinary ASCII
module names and at most 32 files in the entire closure, under the existing 64 KiB/file and 1 MiB
source limits. Module paths use ordinary ASCII identifier components and cannot
replace the prepared Lean/Std/Atlas modules. Imports require either another
selected module or the prepared Lean/Atlas libraries; arbitrary Lake packages are not
discovered or fetched. Compilation/extraction has a shared 180-second budget;
truncated/failed/interrupted output produces no graph. Graph output is bounded
to 5,000 declarations and 100,000 internal edges. Resource hashes bind the selected
Lean executable and prepared Atlas files; this is not a complete hash of the Lean
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

After reading a source graph, select a declaration, enter **比較する要件** and choose
**この宣言の意味を解析**. The reviewed processor receives two separate requests:
first the selected declaration plus observed local files (or the elaborated Atlas
scope above), without the separate
requirement or conversation; then the requirement plus the back-translation,
without the formal source or conversation. The same host-selected model handles
both requests. They are separate contexts, not independent-model evidence.

This uses a Rust adapter of claimcheck's two-pass prompt/output approach, pinned
at `8eb37fedc0acb5bc5f89efc3933cb3955268a4bd`. Source attribution and the MIT
license are in `crates/meden-cli/src/semantic/`. It does not run the upstream SDK,
CLI or Dafny verifier. No extra credential discovery, network client or
model-selected execution is added. Only the two structured result functions are
decoded as data; unexpected tool calls/fields/targets are refused, never executed.

The result shows the frozen requirement, literal back-translation, preconditions,
postcondition, scope and discrepancy/reasoning. **一致の候補** and **意味の差異候補**
are model assessments. A semantic match never marks a declaration proved, accepts
native meaning or changes proof/export authority. Low-confidence or contradictory
matches remain **判定保留**. This interface does not establish analyzer accuracy.
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

For a retained mismatch/unclear Finding, **このFindingの修正差分を作る** requests
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
Independent-model judgment remains open integration work.

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
“成果物を開く” button opens the **Code / Preview / Proof** sidebar. Narrow
screens use drawers.

| Goal                            | Entry and result                                                                                                                                                                                                       |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Start a conversation            | Send an instruction with the arrow or Enter; its title is automatic. Shift+Enter adds a newline.                                                                                                                       |
| Read actual development history | Open the vending-machine conversation in the sidebar. Scroll to the supplied opening request or expand tool activity inline with its message.                                                                          |
| Inspect recorded execution      | Expand each tool call for its actual arguments and output, including the Lean recursion-limit failure, verification retries, source edits and seven passing CLI tests.                                                 |
| Inspect a proof                 | Open Proof: the radial graph shows related declarations, including incoming references. Select a node for its statement, status and direct dependencies.                                                               |
| Open exact source               | Click the statement or file/line link; Code opens the retained file at the highlighted line. Return to Proof with selection intact.                                                                                    |
| Change graph scope              | Search a declaration, center on it and return, or choose only outgoing dependencies / meaning-related dependencies. Pan and zoom remain available.                                                                     |
| Inspect proof coverage          | Open the proof-count control for the eight property records. Source-only helpers without individual audit entries remain unconfirmed.                                                                                  |
| Run the meaning-analysis GUI    | Choose “意味解析” inside Proof. A cancellable sequence opens the NG example at the ¥150 purchase threshold. The result puts the boundary case above the compared text. Switch between “一致例を表示” and “NG例を表示”. |
| Edit or request review          | From the meaning result, inspect the retained requirement/theorem record, edit a draft or prepare a revision-bound review export. No extra top-level tab is added.                                                     |
| Preserve work                   | Chat, drafts and edits survive reload. Failed storage retains the in-memory conversation and offers retry/export. Invalid imports keep current work.                                                                   |

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

The default view uses actual MIT-licensed layout functions from Lean4SemanticBench:
dependencies above the center, reverse references below it, parent-based ordering
and radial spacing. Project Overview still shows the wider connected graph.
Hover and focus emphasize adjacent references while preserving the node's
position and hit area.

“グラフを広げる” or “広げて読む” opens a full-height graph with a docked reader.
Its statement and syntax-highlighted Lean source scroll independently beside the
graph. Code no longer depends on a clipped preview beneath the canvas. Source
links retain exact line selection; “会話に戻る” returns to the chat. At small widths
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

Import collection never evaluates lakefile.lean, downloads packages, or silently
omits unresolved imports. Installed Lean/Atlas library modules are explicit external
boundaries; Lake srcDir and external package source roots are not yet discovered.
A missing import fails collection and clears the current graph. Selected roots,
collected files and external module names are retained with the source artifact.
