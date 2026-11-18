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

The left sidebar contains **conversations**, new chat and search. The center is
the conversation, beginning with the large composer on an empty chat. Sending
an instruction derives the conversation title automatically and opens the right
artifact panel. The upper-right proof, dependency and panel icons open that same
panel; tabs switch between preview, proof, dependencies, meaning comparison and
code. A review draft opens inside the panel. At smaller widths, the sidebar and
artifact panel become drawers.

| Goal                     | Entry and observable result                                                                                                                                                        |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Start a conversation     | Type an instruction and send with the arrow or Enter; a title and demo response appear automatically. Shift+Enter adds a newline.                                                  |
| Continue existing work   | Open the vending-machine conversation to read the 13 original visible messages from meden-demo. New chats stay separate in the list.                                               |
| Search history           | Search titles and message bodies in the left sidebar.                                                                                                                              |
| Inspect results          | Send an instruction or use the upper-right icons; open proof, graph, comparison, preview or source tabs.                                                                           |
| Try the implementation   | Insert money, purchase a 150-yen item, refund or reset in the browser preview.                                                                                                     |
| Inspect dependencies     | Select a declaration, inspect its statement/source, center on a dependency, return, pan/zoom or select meaning-related dependencies.                                               |
| Compare meaning          | Read a requirement beside the comparison text, inspect evidence and assumptions, or create a scoped review draft. Demo comparison is pending, with no independent analyzer result. |
| Edit a requirement draft | Save revised wording; retained proof and analysis become stale and remain stale after reload.                                                                                      |
| Prepare expert review    | Export selected requirements, related declarations and evidence bound to their revisions. Conversations are excluded. This downloads a file; it does not submit a request.         |
| Import or export         | Use the upper-right menu for Meden workspace/Lean Atlas JSON. Invalid input retains the current conversation.                                                                      |
| Recover storage          | A failed write retains the in-memory conversation and offers retry or JSON export; no successful-save claim is shown.                                                              |

## Recorded meden-demo example

The default example comes from `../meden-demo`, a Rust vending-machine project,
and its matching retained Meden verification artifact. The bundled snapshot
contains the actual `kernel.rs`, CLI, tests, README, specification, extracted Lean
candidate, proof source, accepted contract, report and **13 visible conversation
messages**. Hidden model/tool traffic, connection settings and credentials are
not included. Message timestamps use the recording's last-modified time; they
are not reconstructed per-message event times.

The kernel SHA-256 is
`c14e1efc673ef51eae41ac0b435a7233abb005092d48119c965595f780c2ff83`.
The capture tool refuses a retained artifact whose source hash does not match
that file. To refresh from the same local project/state:

```sh
uv run tools/gui/capture-meden-demo.py
```

The capture accepts `--project` and `--state` for another explicit source. Inspect
the resulting snapshot before committing it. Only whitelisted files and the
visible presentation transcript are captured.

The **8 proof statuses are archived verification results** for the pure `step`
transition: insertion, overflow rejection, purchase, rejected purchase, refund,
invalid action, stock preservation and refunded-amount preservation. CLI input
parsing, display and I/O are outside that formal scope. Opening a panel does not
rerun Lean. The browser preview is a TypeScript/BigInt implementation of this
transition, covered by ordinary tests, not a newly proved Rust execution.

The graph is an authored projection of declaration references from the retained
Lean sources, with type/value edges. It is not a new Atlas/Compass extraction.
The meaning-comparison text is a demo and stays pending/unclear; there is no
independent judge or claimed analysis accuracy. Imported Atlas progress/no-sorry
metadata is likewise not promoted to fresh proof evidence.

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

These references inform the layout; the Meden proof/graph/comparison tabs are our
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
compact layout and keyboard entry. Axe checks home, conversation and all five
artifact tabs against automated WCAG 2.1 AA rules. This is not a complete manual
or assistive-technology audit. The required [public TUI gate](ux-testing.md)
remains separate; scripted fixtures are not live inference evidence.

[Home](assets/gui/home.png) · [Chat and proof](assets/gui/proof.png) ·
[Dependencies](assets/gui/graph.png) · [Meaning comparison](assets/gui/analysis.png) ·
[Preview](assets/gui/preview.png) · [Compact home](assets/gui/compact-home.png) ·
[Compact preview](assets/gui/compact-preview.png).
The [check record](assets/gui/verification.json) records scope and limits.
