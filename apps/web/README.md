# Meden web

React / TypeScript / Vite GUI in the Meden repository.

From the repository root: `npm ci && npm run dev`, then open
`http://127.0.0.1:4320`. [Usage, provenance and checks](../../docs/gui.md).

Start with an instruction, send, and get an automatic conversation title. The
left sidebar is chat history; the compact right sidebar has Code, Preview and
Proof tabs. Proof starts with a radial graph, then statement and exact-source
navigation. A nested meaning-analysis demo shows progress and text differences.

The default example contains meden-demo's user-supplied opening request, 13 recorded messages, 25 original tool
calls/results, code and retained verification report. Reasoning was not saved
in that recording; the GUI does not invent it. New responses and panel transitions are a local
GUI demo; the agent/Lean backend is not connected. Archived proof results and
pending demo meaning comparison are distinct.

Features own their views; adapters own browser I/O. Data, graph traversal and
review package logic live in shared packages outside this app.

The vending example remains a single continuous conversation, including messages
sent from the home composer. The graph uses L4SB's MIT layout code; expand it to
read the highlighted Lean file in a separate full-height pane. See the source
review in `docs/research/2026-11-18-graph-ui.md` for provenance and tool candidates.
