# Meden web

React / TypeScript / Vite GUI in the Meden repository.

From the repository root: `npm ci && npm run dev`, then open
`http://127.0.0.1:4320`. [Usage, provenance and checks](../../docs/gui.md).

Start with an instruction, send, and get an automatic conversation title. The
left sidebar is chat history; proof, dependencies, comparison, code and a working
browser preview open in a right artifact panel.

The default example contains meden-demo's actual visible conversation, code and
retained verification report. New responses and panel transitions are a local
GUI demo; the agent/Lean backend is not connected. Archived proof results and
pending demo meaning comparison are distinct.

Features own their views; adapters own browser I/O. Data, graph traversal and
review package logic live in shared packages outside this app.
