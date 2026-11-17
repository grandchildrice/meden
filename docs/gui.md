# GUI development

The GUI lives in `apps/web` in this repository. It is a TypeScript/Vite SPA using
React, Material UI, assistant-ui and React Flow. It is an application frontend,
not the separate pitch mockup. Node 22.12 or newer is required.

## Start

From the repository root:

```sh
npm ci
npm run dev
```

Open `http://127.0.0.1:4320`. The server binds to loopback and refuses to select
another port silently. The first screen lets you create a workspace, import a
JSON file, or explicitly open the sample.

```sh
npm run build
npm run preview --workspace @meden/web
```

The production build is written to `apps/web/dist`. Fonts and assets are bundled;
the application does not load fonts, analytics or models from a CDN.

## Current journeys

| Goal                           | Entry and observable result                                                                                                                                                       |
| ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Start a workspace              | Create a named workspace; save messages as unsent drafts                                                                                                                          |
| Continue after restart         | Workspaces, saved conversations, requirement edits and review drafts survive a browser reload                                                                                     |
| Import results                 | Import a Meden workspace or Lean Atlas graph JSON; structural validation rejects broken references without replacing the current workspace                                        |
| Inspect dependencies           | Select a declaration, inspect its statement/source, center on a dependency, return to the previous center, pan/zoom or select meaning-related dependencies                        |
| Compare meaning                | Select a requirement; read its text beside the theorem's back-translation with textual differences highlighted; inspect reasons, evidence, model/version metadata and assumptions |
| Edit a requirement draft       | Save revised wording; retained proof/analysis becomes stale and remains stale on reload                                                                                           |
| Prepare expert review          | Select requirements, save notes, export a revision-bound JSON with related declarations and evidence; conversations are excluded                                                  |
| Use a small screen or keyboard | Navigation moves into a drawer; panes stack; standard focus, dialog focus return and a skip link remain available                                                                 |

The sample is hand-authored and labeled. Its strict inequality is an invented
case for exercising the comparison view; it is not a defect discovered in a
Meden commit. Its displayed proof/analysis results are not executed checks.

Imported results are displayed as imported reports. Atlas progress/no-sorry
metadata is not promoted to fresh proof evidence. Requirement edits in this UI
are draft edits, not acceptance operations on the Rust contract controller.

## Storage and connection boundary

This GUI currently uses IndexedDB in this browser, with draft input/preferences
stored locally. Clearing browser/site data removes this storage; export JSON to
retain a portable copy. Browser exports and review packages are unencrypted.
Review export is a local download, not an expert submission or a sync operation.

The local gateway is not implemented in this change. There is no model call,
Lean execution, contract approval or capability grant in browser code. The
connection status remains unavailable, and the composer explicitly saves
unsent drafts. Imported assistant messages are history, not new responses.

The existing Rust TUI/runtime stays the execution authority. Its current host
persistence uses bounded JSON checkpoints; it does not already have the SQLite
gateway assumed in the strategy. The shared schema, protocol port and repository
port establish the frontend seam for that integration. [Architecture and planned
adapters](architecture/semantic-workspace.md).

## Design and copy

The reference is Google's Material Design 3 guidance:

- [UX writing best practices](https://m3.material.io/foundations/content-design/style-guide/ux-writing-best-practices): specific headings, neutral action/consequence language, scannable content, restrained abbreviations.
- [Layout overview](https://m3.material.io/foundations/layout/layout-overview): panes and navigation adapt to available width.
- [Accessible design](https://m3.material.io/foundations/accessible-design/overview): perceivable states, keyboard access and readable content.

Material UI is the React component implementation; the theme/custom graph is
maintained here. It is not a Google-authored frontend framework. The UI uses
8px-based spacing, 44–48px controls, shared color/typography tokens, visible
keyboard focus, semantic labels and reduced-motion support. Content is named
for its function: conversation, dependency, comparison, review. There are no
promotional headlines, fabricated progress steps, certainty percentages or
subscription marketing in the workspace.

The radial inspection interaction is informed by
[Lean4SemanticBench](https://0xfab0131.github.io/Lean4SemanticBench/).
[Lean Atlas' graph format](https://github.com/NyxFoundation/lean-atlas/blob/main/web/lib/types.ts)
provides the import boundary and type/value distinction.
[claimcheck](https://github.com/metareflection/claimcheck) informs requirement vs.
blind back-translation comparison, not a claim of integrated analysis quality.

## Checks

```sh
npm run typecheck
npm test
npm run build
npm run schema:check
npm run format:check
npm run test:gui
```

For Linux CI or a fresh host, install Playwright's browser with
`npx playwright install --with-deps chromium`. On this NixOS workstation the
config uses the installed Chrome. Set `MEDEN_GUI_CHROME` to an explicit browser
binary if needed. The GUI tests launch Vite automatically and retain screenshots,
failure traces and an HTML report under `target/gui-checks/`.

The tests cover saved work/restart, import errors, graph selection/recentering,
semantic freshness, scoped review export, compact layout, keyboard entry and
axe-core automated WCAG AA checks. A green accessibility scan is not a complete
manual or assistive-technology audit. The Rust public-TUI UX gate remains required
by [ux-testing.md](ux-testing.md); fixture success is not live inference evidence.

## Reviewed screens

The images show the explicit sample workspace, not executed model/prover results.

[Conversation](assets/gui/chat.png) · [Dependencies](assets/gui/graph.png) ·
[Meaning comparison](assets/gui/analysis.png) · [Review request](assets/gui/review.png) ·
[Compact layout](assets/gui/compact-review.png).
The [check record](assets/gui/verification.json) records the exercised scope and
limits. Storage failure/retry and unsaved input after navigation/reload are also
covered by the browser journeys.
