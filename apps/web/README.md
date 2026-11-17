# Meden web

TypeScript / Vite GUI in the Meden repository.

From the repository root: `npm ci && npm run dev`, then open
`http://127.0.0.1:4320`. [Usage, scope and checks](../../docs/gui.md).

The agent/Lean backend is not connected. Messages are saved as unsent drafts;
imported results and the explicit sample exercise the inspection workflows.

Features own their views; adapters own browser I/O. Data, graph traversal and
review package logic live in shared packages outside this app.
