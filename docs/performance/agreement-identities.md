# Typed accepted-contract identities

The [validation record](../../results/agreement-identities-20261125.json) binds this application change to the tested native executable, generated schemas, public failure/restart journeys and actual deployed projects. It follows the [typed-core migration](../architecture/typed-core-and-plugins.md).

## Representation and ownership

`SessionId`, `RequirementRevision` and `ContractId` are owned application values with private representations. A session cannot be supplied as a contract digest or an ordinary integer as a requirement revision. Explicit decoding validates representation once; it does not establish the identity's origin or authority.

The actual project/controller reference, replayed review/accept/reject decisions, proposal/check observations, repair linkage and delivery queue now use these types. Project/export session spellings remain the original bounded opaque strings. Host raw bytes retain their existing lowercase hexadecimal encoding. Requirement revisions are positive original u64 values, without an application-side increment. Contract identities hash the exact original canonical bytes. There is no new kernel state, receipt, grant, plugin dispatcher or dependency.

`Summary.reference` is absent when no agreement is displayed. The former revision-zero/empty-contract sentinel is removed. An evidence projection refuses a standalone reference without its agreement and a displayed agreement without its reference. Existing disjoint proposed/accepted/current observations still come from the original host/controller state.

The shared typed values make evidence and delivery reference projections infallible. Duplicate string checks and obsolete fallible repair-view conversion are removed. Ordinary delivery index/payload/source bounds remain. Actual project comparison still requires the current original session, revision and canonical contract before every review/accept/reject operation. Export selection still matches the original queue/session. A malformed TUI review cannot accept and returns to conversation; it does not trap the user on an unusable review.

## Storage and transport

Journal decisions retain their exact positive JSON integers and lowercase digest strings. Native references retain positive canonical decimal strings for revisions, avoiding JavaScript number loss. Explicit serde conversion is local to that transport; application identities do not depend on a view module. Common sequence counters and raw processor/workspace sessions remain separate domains.

Ten generated JSON Schema files now name the shared `SessionId`/`ContractId` definitions instead of repeating inline constraints. The generated browser validators remain byte-identical. This does not make the JSON wire nominally typed: host comparisons remain necessary for syntactically valid foreign or stale identities. Initial model-facing project status now has a null reference, replacing the invalid sentinel; accepted/proposed reference values remain unchanged.

## Checks

Application coverage includes lossless arbitrary positive-u64 journal/native-reference round trips, Unicode/control-text sessions, byte bounds, host byte encoding, canonical digest vectors, zero/malformed/foreign reference refusal, missing-reference view retirement and actual original-host comparison. Compile examples reject cross-role assignment, unchecked digest construction and raw-integer revision calls; a matching full reference round trip compiles.

The real finite-verifier browser journey submits 24 malformed review/accept/reject references and requires HTTP 409 before execution. Nine structurally valid references with another session, revision or contract are queued normally but refused by the original host comparison: the proposal and model-request count remain unchanged. Correct visible review and explicit acceptance then run the actual Z3/refutation/repair path. Restart preserves original accepted meaning without current evidence or model approval, and refuses the former session's reference. The queue's HTTP 202 acknowledgement is not reported as semantic acceptance.

All 132 application and 50 CLI unit cases, 12 compile examples, 63 frontend cases, the 3,000-case generated corpora and 15 algorithm performance cases pass. Workspace-wide target checking, schemas, architecture, Rust/frontend formatting and Python lint also pass. The public TUI gate passes all 35 cases; decoded contract-review, Lake-failure and failed/cancelled/historical restart screens were inspected. All 71 GUI cases and three normal-motion follow-ups pass with no skips or flaky cases. Serialized retained-tab p95 is 37.6 / 39.6 ms with reduced / normal motion, below the existing 250 ms budget. Installed Lean/Lake/GF/Z3 and scripted-model fixtures are distinguished from live-model quality or cold installation. Valid deployed source/graph/reader/inventory/evidence/history bindings and the original public connection choices are checked independently; all 88 browser assets are byte-identical to the preceding deployment.

The initial TUI run failed one CLI restart assertion that indexed the retired fake reference's session before any contract existed. That test now requires absent references and directly refuses restart without a fresh explicit processor approval, followed by a successful explicitly approved restart. The full gate was rerun without dropping cases; the initial failure and final verdict remain in the record.

The 13 compiler-captured Rust sources remain unchanged. This identity change is not a compiler/search or graph-rendering optimization; the [event audit](event-audit-20261125.md) remains the measured performance baseline. Backend snapshot/job identities, raw processor/workspace sessions and the remaining cone-layout/versioning audit are still required before the full architecture goal is complete.
