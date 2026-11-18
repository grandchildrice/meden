# GUI development

The GUI lives in `apps/web`: React, TypeScript, Vite, Material UI, assistant-ui
(including Markdown rendering) and React Flow. Node 22.12 or newer is required.

## Start

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

The left sidebar contains conversations, new chat and search. The center remains
the main work area. The narrower right sidebar has exactly **Code / Preview /
Proof** tabs. The upper-right proof icon opens the graph; the panel icon opens
or closes the sidebar. Narrow screens use drawers.

| Goal                            | Entry and result                                                                                                                                                                                            |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Start a conversation            | Send an instruction with the arrow or Enter; its title is automatic. Shift+Enter adds a newline.                                                                                                            |
| Read actual development history | Open the vending-machine conversation or choose “実際の開発履歴を見る” from a demo chat. “実行ログ” jumps to retained tool activity.                                                                        |
| Inspect recorded execution      | Expand each tool call for its actual arguments and output, including the Lean recursion-limit failure, verification retries, source edits and seven passing CLI tests.                                      |
| Inspect a proof                 | Open Proof: the radial graph shows related declarations, including incoming references. Select a node for its statement, status and direct dependencies.                                                    |
| Open exact source               | Click the statement or file/line link; Code opens the retained file at the highlighted line. Return to Proof with selection intact.                                                                         |
| Change graph scope              | Search a declaration, center on it and return, or choose only outgoing dependencies / meaning-related dependencies. Pan and zoom remain available.                                                          |
| Inspect proof coverage          | Open the proof-count control for the eight property records. Source-only helpers without individual audit entries remain unconfirmed.                                                                       |
| Run the meaning-analysis GUI    | Choose “意味解析” inside Proof. A cancellable CI-style sequence opens comparison results; “差分サンプルを試す” demonstrates missing inventory conditions with colored text differences and a boundary case. |
| Edit or request review          | From the meaning result, inspect the retained requirement/theorem record, edit a draft or prepare a revision-bound review export. No extra top-level tab is added.                                          |
| Preserve work                   | Chat, drafts and edits survive reload. Failed storage retains the in-memory conversation and offers retry/export. Invalid imports keep current work.                                                        |

## Recorded meden-demo example

The snapshot contains meden-demo's actual Rust kernel, CLI, tests, README,
retained Lean specification/candidate/proof, contract and matching report. The
original checkpoint has **57 messages**, from which **13 visible messages and
25 tool calls/results** are projected in order. Activities attach after their
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
analyzer or change stored proof/analysis evidence. Default comparison uses the
saved requirement text; the explicitly labeled difference sample changes an
imagined requirement to omit inventory conditions. It is not a discovered
meden-demo defect or evidence of semantic-analysis accuracy. Its red/green diff
and “balance 150, stock 0” boundary illustrate the intended result UI. Runs can
be cancelled; revision changes invalidate the displayed run. Imported proof
progress/no-sorry metadata is never upgraded to fresh evidence.

## Storage and runtime boundary

Conversations, requirement edits and review drafts use IndexedDB in this browser;
unsent input and selection preferences use local storage. Clearing site data
removes this storage. Portable workspace/review exports are unencrypted.

The small Demo badge and settings describe the current mode. Send appends a
local demo response and opens the relevant panel. There is **no model call,
Lean execution, native contract acceptance or capability grant** in browser code.
Recorded and demo messages retain distinct origins in the shared schema. Editing
requirements only changes browser drafts, not the Rust controller's accepted
meaning. The existing TUI/runtime remains the execution authority.

A local gateway is still pending. The shared schema, agent protocol and repository
port establish its frontend seam; existing host persistence uses bounded JSON,
not an already implemented SQLite gateway. See the [architecture map](architecture/semantic-workspace.md).

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

## Checks and reviewed screens

```sh
npm run typecheck
npm test
npm run build
npm run schema:check
npm run format:check
npm run test:gui
```

The browser gate builds once and serves the production bundle on loopback port
4321, separate from the interactive development server on 4320. Screenshots,
traces and the HTML report are retained under `target/gui-checks/`. On Linux CI,
install Chromium with `npx playwright install --with-deps chromium`; on this
NixOS host the config uses installed Chrome. `MEDEN_GUI_CHROME` overrides its path.

Browser journeys cover send/title/history/reload, preview actions, graph
inspection, freshness, scoped export, import errors, storage failure/retry,
compact layout and keyboard entry. Axe checks home, conversation, all three
sidebar tabs, source navigation and semantic differences against automated WCAG 2.1 AA rules. This is not a complete manual
or assistive-technology audit. The required [public TUI gate](ux-testing.md)
remains separate; scripted fixtures are not live inference evidence.

[Home](assets/gui/home.png) · [Actual execution history](assets/gui/history.png) ·
[Proof graph](assets/gui/proof.png) · [Selected proof](assets/gui/proof-detail.png) ·
[Lean source](assets/gui/code.png) · [Meaning difference](assets/gui/analysis.png) ·
[Preview](assets/gui/preview.png) · [Compact home](assets/gui/compact-home.png) ·
[Compact preview](assets/gui/compact-preview.png).
The [check record](assets/gui/verification.json) records scope and limits.
