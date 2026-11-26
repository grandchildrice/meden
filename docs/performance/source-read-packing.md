# Pack source reads within the original frame budget

The concrete navigation file reader uses original metadata sizes to group cached
source reads. The common cache passes its opaque hints alongside their original
paths; it does not interpret a language or declare content current. The concrete
reader owns JSON size estimation and the fixed confined observation protocol.

Ordinary readers retain 32-path source requests. A reader supplied with hints
requests up to 64 paths, then partitions each request into contiguous physical
frames. Each estimated content value costs at most six times its hinted byte
count plus 1,024 bytes; another 1,024 bytes covers the envelope. Unknown or
oversized hints use the maximum readable file size. A hint may be stale or false:
actual complete content, digests, process termination and frame limits still
control admission. Existing cache single-file recovery remains independently
cancellable; no incomplete frame becomes a successful prefix.

The original physical frame limit is 64 MiB, the per-file content limit is 1 MiB,
and complete collection retains its 128 MiB budget and four reader threads.
The worker still admits at most 256 requests and performs every original and
resolved path/policy check. No timeout, source limit or worker profile increases.
Before dispatching split frames, the reader validates all original logical paths,
including paths that would otherwise appear only in later frames.

The generic collection accepts only a reader width between 1 and 256. The CLI
cache adapter no longer retains the view or calls a second progress callback;
the application cache already owns its completed-batch progress. Parser,
compiler, search and renderer identities and cache entry/seal formats do not
change when source I/O is packed differently.

## Checks before measurement

The application suite passes 194 cases, including original hint/path association
through sorting and single-file recovery, and refusal before I/O for an invalid
reader width. Strict Clippy passes for all application/CLI targets. Actual Linux
worker tests compare every source field for 128 literal Unicode/quote/newline
paths: dispatches fall from four to two. Separate real frames preserve complete
escaped content, conservatively handle unknown sizes, refuse an understated
size's oversized response, reject a later invalid path before any dispatch and
stop cancelled work. No scripted checker substitutes for those file reads.

## Measurement procedure

Fixed original/candidate executables use the same complete private Riemann source
corpus as the [cache-phase profile](navigation-cache-phases.md): 175,187 indexed
files / 1,030,531,854 bytes. Every source and expected original cache record is
rechecked against the frozen manifest before measurement. Each invocation uses a
fresh private host-data namespace and the same source workspace lease, parser
identity, original offline file policy and complete original `File` values.

Four serial sampling runs use ABBA order, followed by a candidate/original quiet
pair without metric polling. Each measures cold population, explicit unchanged
refresh and sealed resume. Complete output comparisons occur after each timed
phase. Setup, comparison and hashing are outside intervals; OS/tool caches stay
warm. No build, test or other metric sampler runs during these invocations.
CPU excludes child processes and nested/parallel intervals cannot be summed as
elapsed time. The bounded sampler refuses lost records instead of treating
missing events as zero work. These are development-profile observations, not
cold installation, live inference or physical presentation.

## Complete-corpus observations

The [numeric record](../../results/source-read-packing-20261126.json) retains all
individual measurements, executable/source/corpus hashes and their scopes.
All 18 cold/refresh/resume outcomes preserve every original file field; their
framed value digest remains
`4604096b4bd13696de57f4e10d0055a6444d33d8c07c1f635e4977a77422ee5d`.

| Observation                          | Original |   Packed |
| ------------------------------------ | -------: | -------: |
| Sampled cold population, median of 2 | 56.841 s | 48.111 s |
| Sampled unchanged refresh, median    | 14.299 s | 14.553 s |
| Sampled sealed resume, median        |  0.618 s |  0.626 s |
| Quiet cold population, one sample    | 53.623 s | 46.595 s |
| Quiet unchanged refresh              | 14.210 s | 14.116 s |
| Quiet sealed resume                  |  0.625 s |  0.625 s |
| Cold physical worker invocations     |    6,160 |    3,423 |

Cold-cache wall falls about 15.4% in the sampled pair of observations and 13.1%
in the subsequent single quiet pair. Original sampled cold intervals are
54.274–59.408 s; packed intervals are 46.867–49.355 s. This is evidence for this
complete frozen corpus and development-profile execution, not a universal
startup or whole-GUI improvement. These are only two sampled observations per
implementation and one quiet observation each.

Metadata still uses 685 invocations; source reads fall from 5,475 to 2,738.
Both sampled runs of each implementation have those exact counts. Every capture
contains all 175,187 index intervals, with no lost records. Original captures
contain 189,582 completed spans and packed captures 186,845; the difference is
the 2,737 removed process invocations. The parser is unchanged; differences in
its measured CPU under this scheduling are not a new lexical-algorithm result.
Unchanged refresh and sealed resume perform no source reads, so their small
timing differences are not attributed to packing. Saved maps and retained
compiler/search/render results remain independent fast paths.

## Public journeys

All 37 required public TUI journeys pass. Nine decoded journeys / 21 observations
were manually read, including source failure/recovery, stale and clipped review,
cancellation and restart. The four actual-worker browser journeys pass in each
motion setting: finite-source preparation/restart, retained tabs with wheel/hover
camera continuity, saved-map failure/retry and the 10,001-file map. The latter
preserves cache-phase parent/trace linkage and recovers a corrupt same-stamp
entry through public refresh. Architecture/schema/format checks pass as well.
These installed-tool and scripted-provider checks do not establish live-model
quality or cold-install behavior. No new kernel refinement is claimed.

Both actual GUI hosts use the candidate native executable and the same 88
frontend assets. All ten original source/reader/inventory/neighborhood/evidence
response bodies remain byte-identical, including their original revisions.
All 58 Riemann and 46 EthLib retained compiler/search/render files are unchanged.
Selected-file preparation reports cache hits; history/candidates remain and
neither host restores model approval. This verifies reuse and complete data,
not a paired actual-project startup or physical-presentation measurement.
