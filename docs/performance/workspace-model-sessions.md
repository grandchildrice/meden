# Owned workspace sessions at the model boundary

Baseline: `c7ba14b`. The [complete validation record](../../results/workspace-model-sessions-20261125.json)
binds this change to the tested executable, unchanged assets and exact original
source artifacts. This implements another part of the
[typed-core/plugin migration](../architecture/typed-core-and-plugins.md), separate
from the preceding rendering optimization. It makes no new latency claim.

`meden-runtime::workspace::SessionId` owns the exact original host byte vector.
Its private representation has explicit host construction and an explicit byte
projection. It has no serde decoder, grant operation, implicit conversion or
replacement-session argument. It remains distinct from the application's
reviewed-contract `identity::SessionId`, view artifacts and transport tokens.
Construction attributes a value; it does not prove its origin or authorize I/O.
No extra normalization, hashing, length restriction or UTF-8 interpretation is
introduced into the original processor's finite-byte-vector model.

The workspace, shared model port and actual HTTP/local gateways carry this value
through preparation, status and the retained caller. TUI/CLI/GUI composition,
independent judge selection and connection review use that same port. The TUI's
pre-workspace root fallback is retained once, then the actual workspace supplies
its process identity. Discovery uses an explicit fixed host identity and still
cannot grant inference. The gateway explicitly projects the original bytes to
the unchanged core processor constructor, authorization check and consuming
dispatch. There is no replacement grant, processor controller or model-specific
branch in core. Project/repair/job adapters currently project those same bytes
into their existing interfaces; their further typing remains open.

Native schemas, browser assets, portable/history encodings, compiler/search/
renderer definitions and complete source/graph observations must remain exact.
The HTTP model's implementation descriptor does change because its actual
gateway source changes. Historical method descriptors are retained as recorded;
they are not relabeled as the new implementation. Fresh gateways still need host
approval. The local engine/weights consuming check is unchanged.

Compile examples reject raw-byte model binding, reviewed-contract sessions in
that port, unchecked session construction and JSON session reconstruction. A
matching workspace call compiles. Generated finite traces compare the original
raw-byte processor to the typed adapter, including full local/HTTP parameters,
equal copied or different identities, host/model approval/revocation, expiry and
consuming dispatch. These are bounded generated tests, not another proof of
arbitrary compositions. The core and its extraction/refinement sources do not
change.

An actual loopback HTTP journey uses non-UTF-8/NUL bytes. A separately constructed
equal identity uses the original grant; changed bytes, truncation, appended NUL
and lossy UTF-8 cannot. Rejected attempts reach no listener. An approved complete
exchange retains the original message/model binding; revocation closes the
retained caller without renewal. Existing actual transport-registration,
in-flight revocation, expired-grant and no-local-resource fallback cases remain.

The required all-target Clippy run exposed earlier missing test-fixture lint
annotations and small production style issues. Scoped test annotations permit
intentional assertion/index failures only in fixture modules; assertions and
production lints remain. Hex formatting preserves original bytes without an
intermediate string per byte, and evidence binding observation is extracted from
the oversized projection without changing its validation or ordering. The OS
profile probe now tests the actual split: Lake query may write, metadata validation
and graph extraction are read-only, and neither sees the generic worker. It is
an actual mount probe, not a compiler or model-quality fixture.

The first local profiling build incorrectly reused dep-info pointing into a
previous test source copy. That executable was rejected before public journey or
deployment validation. Workspace crates were explicitly cleaned/rebuilt; the
accepted executable contains the workspace session projection symbol. The first
Rust run also exposed the old writable-metadata profile expectation above. The
earlier failed runs remain diagnostic evidence rather than successful gates.

The final workspace Rust run passes 375 checks, including documentation/compile
examples, with no ignored tests. The session-boundary property additionally runs
3,000 generated traces. All-target/all-feature Clippy, schemas, Rust/frontend
formatting, architecture and Python lint pass. The public installed-tool TUI gate
passes all 35 required journeys; six decoded review/failure/repair/restart screen
sequences were inspected. The complete GUI gate passes 76 cases in one run,
without skips or flaky results. Inference in these journeys is scripted; this
does not establish paid/live-model quality or cold installation.

Both deployed projects keep their exact complete original source DTO, graph
revision, readers, inventory/navigation, history and evidence observations, plus
the original checksummed compiler/search/rendering objects. All 88 browser assets
are unchanged. After a further real restart, source preparation reports complete
cache hits and starts no compiler, extraction, search or rendering worker. Hover,
retained-tab camera, node-to-source-line selection and quiet chat remain intact.
The observed file-click to ready-DOM times after restart are 12.294 s / 2.111 s
for Riemann / EthLib. These concurrent functional observations are not a paired
performance comparison; complete prepared-input validation remains a separate
optimization target. Backend snapshot types, remaining project/job session
interfaces and the retirement audit remain open.
