# Browser entry and generated-output retirement audit

Baseline: `f2efa6c`. This audits the fifth implementation exit in
[the migration plan](typed-core-and-plugins.md). A name that resembles a prototype
does not establish that its code or data is unused.

| Path                                                           | Current entry and consumer                                                          | Decision                                                                                                                                                |
| -------------------------------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `app/NativeApp.tsx`                                            | `main.tsx` with a native session; public `meden gui` and `meden open`               | Keep the native workspace composition and its generic file/source viewer.                                                                               |
| `app/App.tsx` and `fixtures/meden-demo`                        | `main.tsx` without a session; documented `npm run dev`                              | Keep the recorded context, IndexedDB repository and import/export/recovery behavior. No replacement entry was established.                              |
| `features/proof/ProofWorkspace.tsx`                            | Recorded artifact panel, reachable through its lazy import                          | Keep recorded proof/review presentation; dependency rendering uses the shared `GraphView`.                                                              |
| `features/native/NativeDependencyGraph.tsx`                    | The native analysis utility dialog in `NativeApp.tsx`                               | Keep the currently invoked explicit-tool controls; it is not a second graph renderer.                                                                   |
| `features/graph/GraphView.tsx` and Atlas cone layout           | Both source contexts; actual vendored L4SB layout                                   | Keep one dependency renderer/layout. TypeScript resolves the adjacent declaration file, but the JavaScript implementation is a live runtime dependency. |
| Project-map Canvas painter plus SVG interaction layers         | Native project map                                                                  | Keep the single measured production path. Worker, WASM, replay/cache and consolidated-SVG trials have no production selector.                           |
| `project-map-oracle.ts` and `fixtures/ledger.ts`               | Generated-corpus/differential and schema/adapter/review tests                       | Keep independent test inputs/oracles outside the production import closure.                                                                             |
| Ten standalone generated native status/index/reference modules | No barrel, browser or tool import; their models are already nested in session state | Delete modules, duplicate JSON schemas and generator/export selections.                                                                                 |

The source walk starts at the public `main.tsx`, resolves owned TypeScript
imports/exports/type imports and literal dynamic imports, and follows real
relative JavaScript beside declaration files. After cleanup it reaches 95 of 97
owned non-test implementation modules. The two remaining modules have the test
consumers named above. No owned module import is unresolved. This is module-level
structural evidence, not export-level liveness, runtime execution coverage, an
alias-proof isolation check or proof that no Rust code can be removed.

The deleted standalone outputs are `analyses-status`, `directory-index`,
`evidence-status`, `history-reference`, `inventory-status`, `preparation-index`,
`preview-status`, `reader-index`, `source-index` and `verification-job-status`.
The generator now emits 27 native entry models instead of 37. Their Rust types,
session-state fields, original constraints and actual endpoint validators remain.
Schema regeneration preserves every retained JSON/TypeScript output. Rebuilding
preserves every one of the 88 deployed static files byte for byte. No runtime
backend identity, source cache, semantic observation, camera or model grant changes.

## Recorded context is not a second native state owner

README and `docs/gui.md` still document the no-session recorded entry. Its capture
script reads retained project messages/code/proof and excludes host prompts and
connection configuration. That archive is historical input, not an executable
native preparation or accepted evidence. `WorkspaceRepository` has an actual
browser implementation/callers; its retirement would require replacing storage,
import/export and failure/restart journeys, not just removing the start screen.

Public recorded journeys exercise malformed import preservation, requirement
edits invalidating retained observations across restart, failed storage followed
by visible retry, and stable graph camera under pointer movement. The separate
portable-history journey uses a real clone and installed Lean, retains historical
observations and requires fresh approval. Native missing-assets and repeated
host-restart journeys exercise the other entry. The final validation record
links these current runs and inspected screens; scripted providers and installed
tools do not establish live-model quality or cold installation.

## Engine and entry decisions

The [startup comparison](../performance/bootstrap-splitting.md) rejects lazy
splitting of both entries: fewer bytes arrive, but controls appear later in the
direct native-server comparison. Rejected code, its transient-import recovery
cases and static assets stay in the diagnostic archive, not production. No new
dependency, compatibility path, runtime option or fallback engine is added.

Earlier actual-project Worker/cache/placement trials remain rejected; the
[target-batch change](../performance/map-target-batches.md) is the retained
measured renderer improvement. The audit closes the decision about those live
entries and identified duplicate outputs. It does not claim to remove the
remaining first-detail pauses or full current-input validation waits.
