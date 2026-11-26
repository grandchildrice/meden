# Complete navigation-cache phase measurements

`navigation.cache.prepare` previously covered the whole initial map-cache wait.
The cache now exposes four fixed, parent-correlated intervals per batch:

| Interval                    | Included work                                                                                                       |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `navigation.cache.metadata` | Original confined metadata request and returned hints                                                               |
| `navigation.cache.lookup`   | Read/decode original entries, compare hints and collect pending paths                                               |
| `navigation.cache.sources`  | Original complete source collection/indexing; existing single-file retry and failed-entry retention when applicable |
| `navigation.cache.persist`  | Construct/serialize successful entries, write temporary files and atomically replace records                        |

This adds observation, not another cache implementation. Four cache workers,
their source readers, original batches, byte limits, result checks and complete
inventory sealing remain unchanged. No timing label contains a filename, prompt
or credential. The lexical implementation identity and compiler/search/render
identities do not change when these cache timers change.

Worker/stage wall intervals overlap and cannot be summed as elapsed time.
`navigation.cache.sources` includes its nested collection/index intervals;
current-thread CPU excludes child file-worker processes. The diagnostic captures
the bounded public metric projection without widening runtime record access.
It refuses lost records instead of interpreting missing events as zero work.

## Corpus and procedure

The experiment uses a complete source-only private copy of the actual Riemann
inventory: **175,187 indexed files / 1,030,531,854 source bytes**. Every indexed
source SHA and byte count matches its original cache record. Complete source
copies and original expected-record copies are checked before profiling; there
are no shortened files or generated stand-ins. The production workspace and its
exclusive lease remain intact. Lake/compiler artifacts are not copied or run.

`profile-navigation-cache` uses the normal policy-bound file worker and the same
cache service on a fresh private host-data directory. It measures cold population,
an explicit unchanged refresh, then sealed resume. After every phase, all fields
of every cached `File<Symbol>` are compared with the complete original record.
The comparison and canonical result hashing are outside measured intervals.
Physical inode/time hints belong to the private copy and are not confused with
original source/graph/evidence identities.

Sampling merges actual completed spans every 100 ms, checks its final unique
record count against lifetime ring accounting and refuses incomplete capture.
The local diagnostic has a one-million-record bound; production keeps the same
4,096-record ring. A separate quiet run, on a new empty cache namespace, calibrates
observer cost. OS/tool caches remain warm. These are diagnostic observations,
not cold installation, physical presentation, compilation or model quality.

The [numeric record](../../results/navigation-cache-phases-20261126.json) retains
full scope, source/executable/corpus hashes, phase outcomes and raw-capture hashes.
Raw records and source corpora remain local. Parser changes still require honest
cache invalidation; metadata hints do not replace compiler input validation.

## Observations

One sampling run followed by one quiet run uses separate empty host-data
directories, the same source copy/worker and retained OS/tool caches. No build,
test or other sampler runs during either invocation. All original `File` fields
match after all three phases of both runs; their framed canonical value digest
is `4604096b4bd13696de57f4e10d0055a6444d33d8c07c1f635e4977a77422ee5d`.

| Cache operation            | Sampling |    Quiet |
| -------------------------- | -------: | -------: |
| Cold population            | 56.616 s | 57.003 s |
| Explicit unchanged refresh | 14.292 s | 14.580 s |
| Sealed resume              |  0.616 s |  0.619 s |

The sampler is slightly faster in this single pair. That does not establish zero
observer overhead or a population speed estimate. This private-copy experiment
does not demonstrate an improvement over the earlier 99.833-second actual-host
deployment observation: source placement, surrounding work and cache conditions
differ. The cache algorithm has not changed in this measurement slice.

The complete sampling capture contains **189,582 records**, including exactly
175,187 lexical index intervals. Initial population has 685 batches:

| Phase                        | Summed cache-worker wall | Summed current-thread CPU |
| ---------------------------- | -----------------------: | ------------------------: |
| Metadata                     |                 38.727 s |                   5.050 s |
| Entry lookup                 |                  2.233 s |                   2.197 s |
| Source collection            |                144.425 s |                   0.442 s |
| Successful entry persistence |                 34.926 s |                  34.464 s |

The four cache-worker stages are sequential within a worker and overlap across
workers. Their summed wall is not the 56.616-second coordinator wall. Source
collection uses nested reader threads: its caller's CPU excludes those threads
and child processes. Actual lexical indexing uses **237.247 seconds** of summed
thread CPU, within **261.707 seconds** of collection-worker CPU. These nested
CPU intervals must not be added. This identifies lexical processing as the main
measured host CPU cost; the low outer source-stage CPU is not evidence of cheap
indexing. File-worker child CPU remains separate.

Unchanged refresh performs 685 metadata calls and 685 lookups, with no source
collection, indexing or persistence. Its summed lookup CPU is 19.531 seconds;
metadata/current-worker wait also matters. Sealed resume performs none of those
batch operations. This existing fast path is presentation-cache reuse, not a
substitute for complete compiler-input validation.

## Actual-host retained-file observation

Deployment validation restarts both existing GUI hosts, then opens one retained
file through the public explorer. Complete metric rings have no evictions in
these snapshots. This is one diagnostic observation per project with warm
OS/tool caches and surrounding API, asset/hash and screenshot checks; it is not
an isolated benchmark or a timing comparison with the former executable.

| Observed interval                 |  Riemann |  EthLib |
| --------------------------------- | -------: | ------: |
| Retained-file `analysis.extract`  | 20.219 s | 0.910 s |
| Nested prepared-input Lake query  | 10.728 s |  absent |
| Nested prepared-input fingerprint |  7.030 s |  absent |

The two Riemann input intervals have the same trace and the extraction span as
their parent. `atlas.project.query` invokes offline `lake --json query` for the
original module facets before the metadata-only input fingerprinter runs.
These intervals mostly wait outside the calling Rust thread: its CPU is 0.082 s
and 0.063 s respectively. The capture does not identify the child processes'
internal CPU/I/O split. This is a second source of waiting besides lexical map
population, even when the final observation is a cache hit. EthLib's selected
file follows a different retained-input path; absent phases are not zero-duration
calls. No compile/extract/search/render worker spans occur in either snapshot.

All ten original source/reader/inventory/neighborhood/evidence response bodies
remain byte-identical, as do all 104 retained semantic files. The 88 frontend
assets and lexical indexer identity remain unchanged. Existing input freshness
checks are preserved; a faster navigation cache cannot stand in for them.

## Verification

The application suite passes 192 cases; the diagnostic loss-refusal test and
Clippy pass. Architecture/schema/format checks pass. All 37 required public TUI
journeys pass, with nine decoded journeys / 21 observations manually inspected,
including failure, cancellation, stale review and restart paths.

Four actual-worker browser journeys pass under each motion setting: finite-source
preparation/restart, retained file tabs, saved-map discovery/recovery and the
10,001-file map. The latter observes all four cache phases with the original
worker parent/trace and repairs a corrupt same-stamp entry through public refresh.
The retained-tab journey also checks wheel/node-hover interleaving without a
camera reset. Its two-rAF tab timings are a fixture presentation proxy, not
physical paint or actual-project latency. Full observations are in the numeric
record; no live inference or cold installation is claimed.
