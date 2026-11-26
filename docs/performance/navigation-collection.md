# Application-owned navigation collection

Directory discovery, complete-content validation, bounded parallel indexing and
canonical file ordering now have one owner in `meden-app::navigation::collection`.
The GUI no longer owns a second collector or its file-record definition. Its
adapter retains the existing policy-bound worker, original logical-path checks,
30-second worker/30-minute preparation limits, diagnostic mapping and progress
updates. Compiler preparation and annotation loading remain separate operations.

The collection interface has two small roles:

- `Reader` returns complete typed directory and file-availability observations,
  one per original request and in its original order.
- `Indexer` supplies pure format availability and a typed `Indexed<S>` result;
  `S` belongs to that implementation, without a common Lean/SMT/RTL AST.

The native host's existing syntax composition implements the indexer. Other
implementations do not require a GUI/kernel branch in collection. The unit
fixtures demonstrate opaque data for two textual formats; they do not install
SMT-LIB or Yosys support. Actual Lean, Rust and finite JSON navigation retain their
original parsers, selected formats and identities. The new collector does not
compile, run a search or infer verification from readable text.

Four readers retain batches of 32 and the original 128 MiB observed-content
budget. Discovery retains batches of 128, 32 directory levels, 65,536 directories,
one million entries and 250,000 supported files. Hidden/build entries and
symlinks/special files are not traversed. Complete text is validated before
indexing; empty indexed text differs from binary, unsupported encoding, size-limit
and unsupported-format observations. Oversized metadata is not charged as read
content or replaced by a truncated prefix.

Malformed components, duplicate paths, excess/missing observations, invalid text,
reader/indexer failures and exceeded limits return no complete-result prefix.
These checks do not establish the truth of a reader or parser, an atomic
filesystem snapshot, physical cancellation attribution or an extracted/refined
collector. File-worker I/O and original/resolved policy/resource correspondence
remain the original adapters.

## Retention and measurement

The complete host-cache file record retains its original field names/types and
ordering, including exact integer metadata before wire projection to decimal
strings. The lexical parser and compiler implementation identities are unchanged;
this extraction does not intentionally invalidate prepared observations or
reindex an existing source cache. The generated native wire schemas also stay
unchanged. There is no fallback collector or compatibility decoder.

The later [combined lexical pass](lexical-index.md) changes the syntax identity
separately: it rebuilds lexical records once while retaining semantic peer
artifacts. The measurements below describe the preceding collection extraction.

Bounded native spans now distinguish `navigation.collect.paths`,
`navigation.collect.sources` and `navigation.collect.worker`. Each worker retains
its source coordinator's trace and parent span. Worker current-thread CPU covers
its host parsing/indexing and adapter work; child process CPU is separate. Parent
and child wall intervals overlap and must not be added as independent elapsed
time. These diagnostics add no source/path/credential labels and are not a
measured speedup or a proof of whole-worker execution.

Generated directory forests are compared with an independent flat-file inventory.
Conformance cases exercise literal Unicode/newline/backslash names, excluded
entries, symlinks, recursive directory observations, complete opaque data,
97-file batching, byte/cardinality limits and a failed index after a successful
observed item. Literal persisted-record fixtures check the original cache shape.

The actual native availability journey now creates an external file symlink and
a cyclic directory symlink, checks the exact four-file inventory and unread byte
counts, follows parent-correlated collection spans, then exercises literal source
pages, missing-file recovery and restart without analysis/model approval. Existing
small/large map, retained annotations and actual finite/Lean compiler journeys
remain required. Scripted providers and installed-tool fixtures are not live-model
quality, cold-install, confidentiality or new proof evidence.

After deployment, actual EthLib and Riemann retain the complete original source,
reader, inventory, navigation and evidence payloads. Both selected files reuse
compilation caches, keep history/candidates and remain unapproved for inference.
The complete native rings retain correctly linked worker/coordinator spans.
One startup/selection probe observes path discovery at 11.712 s for EthLib and
4.705 s for Riemann. EthLib's full lexical collection takes 3.273 s of wall time
with four workers; their CPU intervals are separate from the coordinator's CPU.
These are single-workload diagnostic observations, not a controlled before/after
comparison. Discovery and initial materialization remain meaningful wait targets.
See the [numeric record](../../results/navigation-collection-20261126.json).

This advances shared materialization/collection ownership in migration steps 3/4.
Generic layout, metadata-cache orchestration, observation-revision correspondence,
native freshness waits, first-exposure rendering and retirement work remain under
the [full plan](../architecture/typed-core-and-plugins.md).
