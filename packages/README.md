# Shared frontend/domain packages

These packages contain implemented data or algorithms and have no React or
browser dependency. They are internal npm workspace packages, not published SDKs.

- `workspace-schema`: versioned data validation, status vocabulary, stale evidence.
- `agent-protocol`: jobs/events and ports for a future Rust gateway.
- `atlas-adapter`: Lean Atlas data import, semantic scope and radial layout.
- `expert-review`: version-bound, scoped local review export.

Generated structural contracts are in `../schemas`. See
[the architecture](../docs/architecture/semantic-workspace.md) for planned
semantic-engine, crypto, sync and business-control boundaries.
