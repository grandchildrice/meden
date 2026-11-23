# Semantic workspace architecture

Status: native gateway, cached project inventory/navigation, selected-module Lean
Atlas extraction and two-pass meaning analysis with optional separate judging
implemented. General backend registration and full-project compiled dependency
coverage remain open. Refactoring policy: [typed core and plugins](typed-core-and-plugins.md).
Decision date: 2026-11-18, reviewed 2026-11-23. This applies the Semantic Assurance strategy's local-first
boundaries while using TypeScript/Vite, as requested, in place of its Next.js SPA.

## Repository structure

```text
apps/
  web/                        React/TypeScript/Vite application
    src/app/                  shell, Material theme, layout
    src/features/             chat, artifacts, demo, graph, analysis, review
    src/adapters/             browser persistence, validated files
    src/fixtures/             recorded meden-demo snapshot + optional sample
    e2e/                      public browser journeys + accessibility
packages/
  workspace-schema/           Zod workspace, evidence, revisions and statuses
  agent-protocol/             native wire schemas and repository port
  atlas-adapter/              Atlas import, dependency scope, radial coordinates
  expert-review/              target binding and minimal review package
schemas/                      generated JSON Schema for non-TypeScript adapters
tools/gui/                    schema generation, local profiling and demo capture
tools/architecture/           dependency/data-boundary regression checks
crates/                       existing Rust controller, workers, TUI and protocol
verification/                 existing Lean specifications/refinement tooling
python/                       existing adapter/UX checks
```

The Cargo workspace and proof-sensitive paths remain in place. The npm workspace
has no lifecycle publishing/install scripts and does not wrap or replace the
`npm/` distribution launcher. Data/graph/review packages do not import React,
Material UI, browser storage or a model SDK. Rendering and browser I/O stay in
the web app. Another frontend can consume the same JSON contracts without
porting these components first.

## Implemented contracts

`workspace-schema` owns requirement IDs, declaration identities, source spans,
proof status, proof bindings, analysis verdict/status, model versions, evidence
references, conversations and review drafts. It rejects duplicate/dangling
references and unknown authority-like top-level fields. All input files are
bounded to 16 MiB before parsing. A proof result requires evidence references and
its own source/input version. Imports with a mismatched binding are stale.

Proof status preserves `proved / bounded / empirical / refuted / unproved / unknown /
unsupported / stale`. Meaning preserves `full / partial / mismatch / vacuous /
missing / unformalizable / unclear`, plus execution/freshness status. A semantic
match does not turn an unproved theorem into a proof. Editing a requirement
invalidates old proof and analysis without changing their historical bindings.
Recorded workspace edits/imports never accept a native contract or issue an export grant. Native contract-review actions call the Rust controller with an exact frozen reference.

`agent-protocol` defines validated native state/command schemas and the
`WorkspaceRepository` port. The same-origin `NativeClient` implementation lives
in `apps/web/src/adapters/native-client.ts`, so HTTP I/O does not enter the data
package. Native progress currently uses bounded state polling. The unused
proposed `AgentClient`/job-event interface and its generated schema were removed;
future job progress must implement the actual shared application lifecycle.
Existing `crates/meden-cli/src/protocol.rs` is
JSON Lines over standard I/O; it is not an authenticated browser gateway and
cannot simply be exposed as one. No browser-managed keys or grants are added.

`atlas-adapter` consumes Lean Atlas' node/edge JSON. It retains unknown edges
conservatively and prunes a theorem's value-dependency edges only for the
meaning-related view. Definitions' value dependencies remain. It computes
cycle-safe graph scope and the vendored MIT Lean4SemanticBench cone geometry.
This is a viewer integration, not a new proof of semantic sufficiency. Broader graph rendering,
standard-library trust configuration and analyzer correspondence remain work.

`expert-review` freezes source and input versions, exports selected requirements,
related declarations and evidence, and rejects stale target drafts. It excludes
conversations. It explicitly labels the file unencrypted; key distribution,
sharing and a reviewer service are not implemented.

Generated JSON Schema describes structural fields; the Zod reference/freshness
checks also run in TypeScript. Future Rust/Python adapters must implement those
cross-reference checks too. Generated schemas do not establish authorization.

`npm run architecture:check` guards the current Rust dependency direction and
keeps shared data/port packages free of owned browser I/O, UI imports and direct
ambient clocks/randomness. Workspace creation/editing receives explicit identity
and timestamp values from its caller. The guard uses Cargo metadata and the
TypeScript syntax/symbol model; it does not prove third-party behavior, arbitrary
aliasing, determinism or host I/O enforcement. The future native DTO generation
and remaining application/context splits are specified in the refactoring policy.

## Replaceable compiler and meaning integrations

`meden-core::adapter` defines owned `Input`, `Artifact` and `Adapter<Context>`.
Formats/producers are explicit, while tool syntax, processes and configuration
remain host adapters. These artifacts are observations and cannot issue evidence
receipts or approve actions. The Lean compiler and the actual pinned Informath/GF
renderer implement this interface; future Z3/Yosys implementations need no
solver-specific core dependency. They are not implemented yet.

The Lean adapter separates elaboration, structural rendering, Dedukti presentation
interchange and Plausible sampling. It saves a full prepared snapshot before the
file page is published. Exact source/configuration/resource validation gates reuse.
A lexical navigation cache is prepared separately: zoom reads cached chunks without
starting compilation, workers or inference. Neither cache grants proof authority.
See [the implementation and research limits](../research/2026-11-22-observation-adapters.md).

## Chat and recorded-demo adapter

The shell owns conversation selection and persistence; the left sidebar lists
conversations across stored workspaces. The central assistant-ui composer sends
instructions without a title form. New titles derive from the first instruction.
A right artifact panel owns code/preview/proof tabs, reachable
from upper-right icons and opened automatically on send.

`fixtures/meden-demo` projects actual whitelisted source files, accepted contract,
matching retained verification report, the user-supplied original request, 13 recorded messages and 25 original tool events into
a workspace with `origin: replay`. Messages distinguish `recorded` from new
`demo` responses. The capture verifies the current Rust kernel's SHA-256 against
the report. The graph uses extracted MIT layout functions from Lean4SemanticBench and
DOM/SVG rendering, with a full-height docked source reader. It indexes 53 local declarations and 150 lexical references from three
retained files; it does not run Atlas or elaborate full library dependencies. Archived proof records remain version-bound and become stale after
requirement edits. A nested, cancellable CI-style meaning-analysis demo shows
text differences and an initial NG sample with missing inventory conditions,
plus an OK example and an algorithm-info popover; it never
writes completed analysis evidence. Existing analysis records stay pending/unclear.
Reasoning is absent from this checkpoint; actual tool inputs/results are displayed
in their original order, with host-injected prompts/settings excluded. The interactive
preview mirrors the pure transition with BigInt and has ordinary unit coverage;
it is not newly extracted or formally verified code.

The native mode (`meden gui`) serves the built app from a loopback HTTP adapter,
uses a fresh 256-bit session token, exact Host/Origin/fetch-site checks and
same-origin authenticated JSON commands. A single serial worker invokes the same
native `Session` and policy-bound tools as the TUI, with independently cancellable
turn controls. Session checkpoints and the exclusive workspace lease are shared.
Processor approval binds the launch-time endpoint; contract review/acceptance uses
native exact references. No browser-supplied credential, tool result or policy is
accepted. Revocation/expiry cannot renew a grant. GUI visibility, transport and
input provenance remain unproved adapters. Current live panels cover chat, file
reads, tool logs, native contract status, delivery review and local source-reference graphs.
Native preview freezes a bounded static build through offline file workers and
serves those bytes on a separate loopback origin with an opaque-origin iframe.
Browser resource policies are tested adapters, not OS egress isolation or R1
completion. Preview interactions confer no proof or source-to-UI correspondence.
Native meaning analysis runs through the reviewed processor. Source observations bind exact
selected paths/content with SHA-256 through the existing offline file worker.
The separate `atlas-adapter/source-index` parses lexical declarations/references;
all resulting proof statuses remain unknown. It does not execute Lean Atlas or
provide elaborated dependency completeness. Failed reads and new development
turns discard the old snapshot. The common source reader takes explicit file data,
so native content never falls back to archived demo source.

The native semantic adapter derives its two-pass prompt/output framing from
claimcheck commit `8eb37fedc0acb5bc5f89efc3933cb3955268a4bd` (MIT). Translation
receives frozen selected source context without the requirement/chat; comparison
receives the requirement/back-translation without source/chat, using separate
requests to the same configured model. Strict single-result parsing never executes
returned functions. Input/source identity, pre/post source observations,
revocation/cancellation and atomic host JSON persistence keep results scoped.
Restart restores historical candidates, not current proof or processor authority.
Selected-module Atlas mode compiles fresh source in an offline job, with source
files and the pinned public library mounted read-only. The actual upstream
classifier and type/value dependency functions run on the imported environment.
A separate extraction revision binds observed files, graph data, the Lean executable
and prepared Atlas hashes. No project Lake configuration or retained project build
output is executed. Resource/source changes and compile failures discard current
graph admission.

The server derives the selected declaration's meaning scope, prunes theorem
proof-only dependencies and retains definition values. Compiled statements/values
replace source/comments in translation; axioms and unexpanded external references
stay explicit. This imported analysis data never creates native proof coverage.
Full-project/library scope, broad accuracy and independent-model evaluation remain
open. Native I/O, Lean execution, metadata, model and rendering adapters have no new
refinement proof. Arbitrary source can execute Lean code inside the offline job;
graph authenticity is not a consequence of the control-kernel proofs.

No agent-client call is made by the demo response adapter. Native control,
accepted meaning, evidence admission and effect authorization stay with Rust.
The browser repository and demo responder can be replaced behind the established
ports as live graph/analysis adapters are implemented. [GUI provenance and layout references](../gui.md).

## Technology map and next adapters

| Strategy choice                      | Current boundary                                                                              | Next implementation                                                                                                     |
| ------------------------------------ | --------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Shared Rust runtime                  | Native GUI/TUI share sessions, offline tools, contracts and immutable delivery                | Versioned job events and remaining interface correspondence                                                             |
| Chat + Atlas GUI                     | Native chat, preview, source/Atlas graphs, delivery and two-pass meaning analysis implemented | Full-project scope and independent judge evaluation                                                                     |
| Local SQLite/event log               | `WorkspaceRepository`, browser IndexedDB adapter; Rust currently uses host JSON               | Native gateway uses existing host JSON journals; recorded demo uses browser IndexedDB; no database needed               |
| Lean Atlas/Compass                   | Pinned extraction core runs offline on selected modules; revision-bound server meaning scope  | Broader library/project closure and large-graph evaluation                                                              |
| Public semantic-gap assessment tools | Analysis input/output fields and comparison view                                              | Claimcheck-derived native adapter implemented; evaluate quality and elaborated scope against Lean/software requirements |
| Static Lean/SMT checks               | Existing Rust verification workers, proof/evidence bindings                                   | Analyzer adapters consume real reports; keep proof and meaning status independent                                       |
| Google Drive BYOS                    | Separate from repository and agent ports                                                      | `packages/sync` and host OAuth adapter after encryption/recovery are tested                                             |
| AES-256-GCM + recovery keys          | No encryption implementation yet                                                              | `packages/crypto`, then key storage/device revocation/recovery tests; never call plaintext IndexedDB E2EE               |
| Expert review                        | Local draft/package creation                                                                  | Encrypted recipient-scoped sharing and an independently authorized submission service                                   |
| Billing/entitlements                 | Absent from workspace content and runtime                                                     | Separate `services/business-control` holding minimal business metadata                                                  |
| Desktop/Rust UI                      | JSON contracts and graph/domain packages separated from React                                 | Add a Tauri/other host or rewrite the view while retaining the Rust controller and wire contracts                       |

Only packages with implemented responsibility exist; no inert gateway, crypto,
sync, billing or analysis service is shipped to suggest those functions work.

## Integration order

1. **Local gateway and persistence (initial adapter implemented):** same controller/session as TUI, loopback
   binding, origin and single-session attachment checks, typed events, cancellation,
   exact accepted/displayed meaning binding; host storage behind the repository port.
2. **Analysis:** reusable public assessment tools (claimcheck / FormalAlign candidates),
   pinned source/requirement/target inputs → Atlas scope → static
   results → blind back-translation → independent judge → candidate findings.
   Store versions/evidence and test false acceptance before claiming quality.
3. **Desktop and BYOS:** decide host packaging, implement keys/recovery, then
   encrypted Drive synchronization and conflict/replay handling. Sync and expert
   sharing remain separate operations.
4. **Expert service/business control:** explicit scoped package preview,
   recipient authorization, submission, reviewer outcomes and entitlements.

GUI/HTTP adaptation, browser rendering, approval visibility and event provenance
remain adapters outside the current Rust-to-Lean refinement. No existing proof
is extended to the GUI by this directory arrangement.
