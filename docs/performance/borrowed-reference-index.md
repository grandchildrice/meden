# Allocate reference candidates after deduplication

The concrete Lean/Rust lexical adapter now borrows ordinary reference tokens
while collecting a declaration. Ordinary tokens become owned strings only after
deduplication. Rust `::` names still allocate their normalized form before set
insertion, so a normalized owned name and an equal borrowed name deduplicate in
the same original ordering. The old unconditional replacement/allocation is
removed. Published symbols still own ordinary strings; no borrowed data crosses
the indexer port, cache or source view. No dependency or kernel change is needed.

## Complete actual-source comparison

The [numeric record](../../results/borrowed-reference-index-20261126.json) retains
fixed executable/source/corpus hashes and individual observations. The workload
is the complete private Riemann source copy from the
[cache-phase profile](navigation-cache-phases.md): 175,187 indexed files and
1,030,531,854 source bytes. Before timing, every copied source and every original
expected cache record is rechecked against its frozen manifest. The production
workspace lease remains intact.

Four serial sampling runs use the immediately preceding combined index and the
borrowed-reference index in ABBA order. Each invocation has a fresh private host
namespace and uses the same actual offline file policy, worker protocol, four
cache workers, nested source readers, batches, bounds and cache implementation.
Cold population, unchanged explicit refresh and sealed resume all compare every
field of every original cached `File` after their measured interval. All twelve
outcomes have the same framed value digest:
`4604096b4bd13696de57f4e10d0055a6444d33d8c07c1f635e4977a77422ee5d`.
Physical inode/time hints belong to the private copy and are not compared.

No test, build or other sampler runs during these invocations. OS/tool caches
remain warm. Each sampling capture checks lifetime ring accounting and contains
all 175,187 index intervals; missing records cannot become zero CPU. Setup,
comparison and result hashing are outside the measured phase intervals.

Both diagnostic and native executables use the development profile, matching
the deployed GUI service. These figures do not establish release-build savings.

| Median of two observations | Original combined index | Borrowed references |
| -------------------------- | ----------------------: | ------------------: |
| Summed index-thread CPU    |               232.383 s |           215.803 s |
| Cold cache population      |                54.870 s |            54.336 s |
| Explicit unchanged refresh |                14.033 s |            14.239 s |
| Sealed resume              |                 0.616 s |             0.622 s |

Index-thread CPU falls about 7.1% on this workload. Both candidate CPU observations
are below both original observations. Cold population wall intervals overlap:
original 54.838–54.902 s, candidate 53.831–54.840 s. This does **not** establish a
reliable startup-latency improvement. Refresh/resume do not perform indexing;
their small timing differences are not evidence of an indexer effect.

A subsequent quiet pair runs candidate then original, each on another empty
private namespace and with no metric polling. Cold population is 53.932 s for
the candidate and 53.967 s for the original; refresh is 14.108 s versus 14.013 s,
and resume is 0.621 s versus 0.618 s. All six additional phase outcomes match the
complete original values. The 35.7 ms cold difference does not establish a
startup improvement. The diagnostic and quiet observations must not be presented
as ordinary GUI latency or as a universal observer-overhead estimate.

These are only two observations per implementation, not population percentiles
or a universal speed estimate. Thread CPU excludes child processes; nested and
parallel intervals cannot be added as elapsed time. The lexical identity changes
honestly, requiring its own complete cache population once. Saved presentation
and compiler-input validation remain distinct paths. In particular, this change
does not address the separately observed prepared-input Lake query/fingerprint
waits, browser first-detail work, compilation, inference or physical presentation.

## Final-code provenance

The initial allocation prototype passed complete corpus comparisons but failed
Clippy's function-length limit. The unchanged namespace construction was moved
into a small helper, then the final source was rebuilt and checked with Clippy,
192 application cases and 2,000 seeded oracle cases before repeating all six
measurement invocations. The numeric record keeps prototype and final-code
observations separate. Final timings above refer only to the final executable;
prototype timings are not substituted for them.

## Verification and deployment

The 192 application cases, 2,000 seeded original-algorithm comparisons, strict
Clippy and formatting checks pass. Architecture/schema checks and all 37 public
TUI journeys pass; nine decoded journeys / 21 observations were manually read,
including failure, cancellation, stale/clipped approval and restart paths.

Five actual-worker browser journeys pass in each motion setting: finite-source
preparation/restart, retained tabs and wheel/hover camera continuity, saved-map
failure/recovery, an executable indexer update and the 10,001-file map. The update
requires full input validation while preserving original Lake/Plausible/GF
artifacts. The large-map journey checks correlated cache phases and repairs a
corrupt same-stamp entry through public refresh. These are installed-tool and
scripted-provider checks, not live inference or cold installation.

Both actual GUI hosts use the final native executable with the same 88 frontend
assets. Full source, reader and evidence response bytes remain equal. Inventory
comparison permits only the new indexer/generation attribution; neighborhood
files remain identical and bind the exact new parent inventory. Both raw envelope
digests are rechecked independently. All 58 Riemann and 46 EthLib retained
compiler/search/render files remain byte-identical. Selected-file preparation
reports cache hits; original history/candidates remain and model approval is not
restored. An evicted metric ring does not establish absence of earlier work.
