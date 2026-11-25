# Application-owned navigation cache

The application's `navigation::cache` now owns syntax-cache preparation,
parallel batches, path-bound record admission, provider keys, generation seals
and bounded atomic entry persistence. The CLI retains the existing confined
metadata/source observations, cancellation adapter and screen progress updates.
The former CLI preparation/storage implementation is deleted.

The common reader exposes a backend-owned symbol type and opaque comparable
metadata hint. Neither is a compiler observation or proof receipt. No Lean
parser, executable, model client or viewer state enters this service. The
original syntax identities, default/additional-provider key framing, entry JSON,
inventory digest and seal format remain; there is no second cache implementation
or compatibility decoder. Successful complete records are reusable without
rewriting their bytes. A provider identity change invalidates its affected
format independently of the default parser.

Only complete inventory preparation publishes its completion marker. Selected
file preparation does not replace that marker. Metadata replies must retain the
original cardinality, and successful source replies must contain exactly the
requested paths. Incomplete/foreign replies and observed cancellation do not
seal a successful prefix. Ordinary unavailable files can retain explicit failed
records. The host's existing policy, canonical worker handles, per-response
bounds and complete compiler input checks remain separate and unchanged.

## Same-stamp corruption and recovery

The old implementation skipped a record whenever its path and metadata hint
matched, including a record with neither a file nor an error. Refresh could
therefore keep accepting that empty entry indefinitely. It also skipped saved
failures after the underlying problem was repaired without changing the hint.

The new store validates the original path and exclusive file/error state.
Explicit refresh rebuilds malformed/foreign/empty entries and retries failed
ones even when the metadata hint remains identical. Valid successes keep the
original fast path. Corrupt/missing completion data is still disposable; the
cache cannot grant model approval, current source validity or proof coverage.

The public large-map regression creates 10,001 real files, navigates/zooms,
opens a compiled source, restarts and repairs an unreadable file. It then empties
the actual selected cache record without changing its hint. **Refresh files and
map** must publish a new inventory, restore the declaration and preserve the
camera. The repaired record must have the exact original hint. No off-screen
cache deletion or configuration edit rescues the journey. The original immutable
executable reaches that final recovery check and fails to publish the new
inventory; its earlier setup, graph, source and restart checks pass.

Application tests cover original-format byte retention, independent SMT2 syntax
records, same-hint malformed/foreign/failed recovery, default/additional-provider
key framing and invalidation, bounded batches, selected-file seals, missing/extra
metadata replies, foreign/incomplete source replies and reported stops. These
are cache/adapter checks, not a filesystem refinement, transactional snapshot
proof or live-model/cold-install validation.

This migration preserves the existing saved-view/current-validation distinction.
It does not reduce the measured first-detail SVG/Canvas cost or remove the
complete dependency-content validation required by semantic operations.

## Final validation

The [numeric record](../../results/navigation-cache-ownership-20261126.json)
binds the executable, unchanged GUI assets, original-executable control and
actual workspace comparisons. All 299 Rust test/doc checks, 37 required public
TUI journeys and six native GUI journeys pass without skips or flaky cases.
The same-stamp recovery journey passes in both motion modes; the original
executable fails its final recovery assertion. Clippy, schemas, architecture,
formatting and Python lint pass. Decoded connection/repair review, clipped
authority, source failure/recovery, cancellation and restart screens were
inspected, as were the old failure and actual source/graph screenshots.

Both actual workspace services use the new executable and all 88 original GUI
assets. Their complete source, reader, inventory, map-detail and evidence DTOs,
conversations and candidate histories remain exact. Riemann's original complete
seal and selected syntax-entry bytes are unchanged; EthLib's absent large-map
cache stays absent. Selected source preparation reuses its semantic cache,
without compilation, search or rendering, and native capture rings have no
evictions. Hosts finish idle and unapproved.

The installed-tool/scripted fixtures are not live-model or cold-install
validation. This closes the represented cache-ownership coupling and recovery
bug; import-resolution/entry-point presentation policy, observation
correspondence, measured waits and the remaining retirement audit stay open.
