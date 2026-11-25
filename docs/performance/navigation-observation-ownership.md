# Application-owned navigation observations

The concrete `adapters::navigation_io::Reader` now owns navigation's fixed file
worker operations, logical-path conversion, options, JSON decoding and limits.
Both syntax collection and cache-hint preparation call that application adapter.
The CLI keeps view/progress callbacks and cache composition; its former observer
and concrete directory/content decoder are deleted.

Public methods accept only the original paths. Directory and complete-content
results have the common typed shapes; metadata stamps remain opaque disposable
JSON hints. Callers cannot replace the selected operation, worker options or
host-held workspace policy. This does not make metadata hints content validity
or proof evidence, and does not change the generic cache's trust in its host
reader. The ordinary confined file worker still checks each original and
resolved resource against the same immutable policy.

The original limits and wire options remain: 30 seconds per worker, 64 MiB of
captured/total output, a 30-minute enclosing preparation deadline, complete
1 MiB file availability and 100,000 entries per directory. Runtime batch
admission still checks complete process output and exact response cardinality
before decoding. The adapter neither adds a compiler nor performs a backend
format selection. Syntax, saved-map and semantic cache identities are unchanged.

## Actual correspondence and failures

Three Linux native tests exercise the installed ordinary worker rather than a
scripted metadata provider:

- Multiple directories and out-of-order source selections retain their original
  literal names, including Unicode/newlines, and response positions. Content
  and every metadata field are checked against independent host filesystem
  observations. Internal links, empty text, binary/unsupported encodings and
  size-limit outcomes remain distinct; original text and files remain intact.
- A later missing source, failed directory or outside link refuses the whole
  batch without a returned prefix or outside canary. Invalid paths, empty/oversize
  request vectors and expired preparation are rejected before any `process.run`
  dispatch. Cancellation remains terminal. These are real confined I/O checks,
  not a proof of filesystem or concurrent-host behavior.
- Thirty-two independently readable 1 MiB newline files exceed the 64 MiB frame
  after JSON escaping. One file succeeds first; the combined frame then refuses
  all source data and preserves the host's typed `Inconclusive` failure. Raising
  the frame budget would fail this outcome assertion. No diagnostic text decides
  the failure class.

A structural architecture guard now rejects worker-request construction/options
or decoding in the two CLI navigation consumers. Restoring the original active
CLI observer fails that guard; the typed reader/progress and opaque-hint adapter
pass. This is an owned-source maintenance check, not a Rust parser, alias-proof
I/O theorem or sandbox for arbitrary native plugins.

The change advances the observation boundary in migration steps 3/4. It leaves
fresh content checks, the serial command queue and measured rendering/native
preparation waits intact. JSON/OS attribution and whole-session correspondence
remain adapter obligations, distinct from the extracted authority kernels.

## Validation and deployment

The [numeric record](../../results/navigation-observation-ownership-20261126.json)
binds the executable, unchanged 88 browser assets, actual read tests and both
negative controls. All 307 Rust test/doc checks, 37 required public TUI journeys
and eight focused public native GUI journeys pass without skips or flaky cases.
The 128 MiB mutation admits the complete frame and fails the refusal assertion;
the source and normal debug executable are then restored byte for byte.
Clippy, 62 architecture checks, schemas, formatting and Python lint pass. Decoded
connection/repair review, clipped authority, source failure/recovery, cancellation
and restart screens were inspected, along with both actual workspace graphs.
These are installed-tool/scripted checks, not live-model or cold-install evidence.

Both actual workspace services run the new binary with the original asset tree.
Their complete source, reader, inventory, selected navigation and evidence DTOs
remain exact. Histories/candidates are unchanged, selected semantic preparation
is a cache hit, no extraction/search/rendering worker spans execute, and the
metric rings have no evictions. There are still exactly two annotation loads
per host: startup and selected-file indexing. Riemann's complete syntax seal and
selected syntax-entry bytes remain exact; EthLib's absent large-map cache stays
absent. Both hosts finish idle and unapproved.

The [migration exit audit](../architecture/migration-status.md) retains the full
five-step scope. This closes the concrete navigation observation ownership and
its named correspondence checks, not general JSON/OS refinement or the remaining
rendering/native wait and retirement work.
