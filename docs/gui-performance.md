# Graph interaction measurements

The [source-refresh camera fix](performance/file-camera-retention.md) retains
the viewport in its file tab across a changed compiled observation. The original
assets reproduce a zoom reset after successful reinspection; this remains
separate from first-detail rendering latency and native input validation.

The [mixed camera-motion regression](performance/camera-motion.md) now exercises
ordinary wheel and pinch-equivalent input while crossing compiled graph nodes.
Both motion modes retain the viewport and pointer anchor; an armed hover-remount
fault is rejected. Numeric SVG batch parsing is also rejected after a slower
measured construction result. This slice changes no runtime or deployment.

The [saved project-map startup](performance/saved-project-map.md) shows retained
navigation before normal discovery completes. Actual first-scene/frame proxies
fall from 17.1 to 2.6 seconds in EthLib and 7.4 to 3.2 seconds in Riemann. Complete
refresh remains slightly slower; unchanged restart avoids duplicate projection
and rewriting. This is separate from the interaction costs below.

The [November 26 deployed audit](performance/event-audit-20261126.md) repeats
all 13 operation windows twice per actual project on `e41a847`. It records
event dispatch, rendering, input/frame proxies and native preparation separately,
with exact executable/assets and correlated browser/native CPU profiles.

The [current deployed event audit](performance/event-latency-current.md) measures two captures per actual project on `ce0c88f`. It separates repeatable 117–119 ms Riemann deep-wheel dispatch, 50–164 ms detail-rendering stalls, occasional node-selection work and cached native input validation. This is measurement of the unchanged implementation, with exact source maps and native sampling.

The [ordered sibling index](performance/map-sibling-bounds.md) reduces hidden-file
query work with unchanged world coordinates and pixels. Riemann's actual-data
query matrix improves by about 90%; two browser captures per version reduce total
deep-zoom long-task time by 28.7%, while worst stalls remain about 150 ms. EthLib
does not show a stable overall improvement. Native preparation remains separate.

The [current deployed event measurements](performance/snapshot-events.md) cover
all 13 interaction windows after snapshot typing. Ordinary graph wheel/hover/drag
handlers remain small, while first-detail Riemann zoom still has a 109.610 ms
wheel dispatch and 156 ms maximum long task. A new cached file inspection still
takes 12.286 s due to separate native preparation. The record separates those
costs and retains source-mapped CPU attribution; it claims no new optimization.

The [reviewed-repair/cache separation](performance/generic-repair-contexts.md)
prevents repair and composition-facade changes from invalidating original
compiler/search/render objects. Two real native builds exercise this reuse
while preserving full input validation. New-method preparation and subsequent
cache-hit waits are recorded separately; this does not establish faster Lean
execution or remove the remaining native freshness cost.

The [compiled-input ownership validation](performance/compiled-input-ownership.md)
separates frozen-module and prepared-Lake execution inputs from display graphs.
Actual native update/corruption/restart journeys retain the original content
checks; deployed ready times are functional observations, not a new speedup claim.

The [retained-reference comparison](performance/map-reference-updates.md) reduces
actual EthLib detail-update projection median from 316.544 ms to 42.480 ms over
the same seven responses, preserving complete scenes and ordered edges. Riemann
does not improve in that workload; its SVG work and native input preparation
remain distinct costs. These same-process measurements exclude browser paint.

The [cone-layout audit](performance/cone-layout.md) fixes valid declaration names
colliding with inherited object properties and makes the existing L4SB settings
explicit. Ordinary coordinates and ordering are preserved; this is separate
from the event costs measured below and makes no new speedup claim.

The [accepted-reference validation](performance/agreement-identities.md) replaces
project revision/session/contract primitives with owned application identities
and removes the empty-reference sentinel. It retains original host comparisons,
wire values and prepared compiler inputs; it is a type/ownership change with
separate retained-tab checks, not a new rendering or compilation speedup.

The [November 25 event audit](performance/event-audit-20261125.md) repeats all
13 operation windows twice per actual project. It separates ordinary handlers
from 83–226 ms deep-zoom tasks, repeated global reference work, cache-hit native
preparation and complete initial navigation. This is measurement of the unchanged
deployed implementation, with matched source maps and native sampling.

The [typed publication validation](performance/artifact-contexts.md) replaces
interchangeable view-reference strings with owner-specific Rust identities and
DTO-bound publication contexts. Original wire digests, 37 browser validators,
88 assets and complete actual-project inventory bytes are unchanged. The public
failure/retry/restart journey also checks 54 own/foreign/malformed queries;
retained-tab timing remains a separate unchanged budget, with no new speedup claim.

The [disjoint inventory comparison](performance/inventory-states.md) retains
every original file/header field while removing repeated empty navigation from
175,184 unindexed Riemann entries. Transfer fell from 36.229 MB to 25.018 MB;
prepared-map navigation median changed from 1.284 s to 1.129 s. It records
native publication costs, unchanged compiler-input validation, actual events
and both retained-tab budgets separately.

The [inventory-heading comparison](performance/inventory-headings.md) records
the preceding served event timings, initial map transfer and retained-tab budget.
EthLib's prepared-map navigation median fell from 2.066 s to 0.749 s; Riemann's
large file inventory remains a separate cost. The report also preserves the
22.45 s cache-hit Riemann file preparation and remaining deep-zoom long tasks.
The preceding [SVG placement comparison](performance/map-placement.md) isolates
React sibling placement from those data and preparation costs.

The [reference-lookup follow-up](performance/map-references.md) checks all
212,283 original EthLib display edges and replaces repeated candidate filtering
with immutable-construction-local indices. Its complete browser-navigation
comparison is separate from the resolver-only performance gate.

The [earlier event and startup audit](performance/map-events.md) separates
synchronous handlers, asynchronous declaration updates, initial map loading and
native Lake/input preparation. Versioned geometry reuse reduces deep-zoom work;
the report also retains the remaining startup overhead and reference-resolution
cost rather than treating a cache hit as an end-to-end speedup.

Prepared-input comparisons now have a [separate calibration report](performance/lake-preparation.md).
It rejects a slower prototype and separates normal invocation time from sampler
overhead; the production compiler and existing graph caches are unchanged.

## Portable history loading follow-up (November 24)

The [complete comparison](performance/recorded-history.md) replaces the portable
loader's intermediate numeric JSON array with the existing file worker's encoded
snapshot mode. The same private copy of actual 4,137,646-byte EthLib records and
the same pinned, real worker returned exactly the same complete document across
eight interleaved measured observations after warm-up.

Median load/decode/validation fell from **1,595.187 ms to 239.879 ms**, about 85%.
The loader's current-thread CPU fell from **961.137 ms to 77.492 ms**, excluding
worker CPU. This isolates the record-loading path, not startup as a whole or
Lean preparation. Input bytes are warmed and validated before each invocation;
workspace/tool preparation and result hashing are outside the read interval.

The original 4 MiB input limit remains. Complete encoded replies fit an 8 MiB
budget. An actual-CLI boundary-size Unicode/control-text read, append and reopen
passes; an independent unchanged-reply observer measures 5,592,417 bytes. The
original reader fails the same resource assertion at 12,623,124 bytes. Invalid
UTF-8, malformed/oversized records and escaping links still fail without edits.
No core, runtime, file-worker, schema or captured compiler source changes are
needed. Broader preparation waits remain separate work.

## Recorded work check observations (November 24)

The [validation record](../results/recorded-work-observations-20261124.json)
covers the shared portable-work service and its historical progress/timers.
The former CLI storage, checkpoint, checker and model-tool implementations are
removed. The existing job publisher/query serves disjoint project and recorded
observations; explicit host work stays in its history dialog and cannot replace
conversation progress. Historical completion follows the original successful
history write and cannot admit native evidence. Retry transfers the same body.

Timers separate `recorded.check.total`, `.lake`, `.inputs`, `.inspect`, and
`recorded.history.read` / `.write`. The real installed-Lean browser journey
holds only dispatch, retries a failed transfer without a command, cancels and
retains the observation, runs a distinct actual check, clones/reopens unapproved,
and repairs through the original model tool. Its final host recheck waits for
its new job and saved completion while preserving the original model publication.
These fixture timings validate instrumentation, not natural compilation speed.

Checks passed 340 Rust tests, 43 TypeScript tests, 33 architecture checks,
37 generated native schemas, all 35 required public TUI journeys and 69 browser
journeys. A final targeted portable rerun strengthens the completion assertion;
its three completed checks also retain all six timer labels. Decoded TUI
failure/recovery/restart screens and the running work dialog were inspected.
Both serialized motion gates passed. Retained-tab two-animation-callback p95 was
31.8 ms with ordinary motion and 38.7 ms with reduced motion; this is a scheduling
proxy, not physical paint or INP. Existing budgets were unchanged.

Actual post-restart cache-hit file opens took 21.314 s for Riemann and 2.238 s
for EthLib. Riemann's matching native trace contains 19.244 s in extraction,
including 16.153 s in offline Lake preparation and 15.656 s in its worker span.
Preparation used 112.7 ms of host-thread CPU, excluding the worker's CPU.
These nested spans must not be added. Riemann's dominant file-open wait remains backend
preparation/input validation; this capture does not isolate hashing from the
other worker steps. The separate interleaved Lake calibration remains the
controlled comparison, and rejects the slower path-check parallelism prototype.
EthLib also exposes 1.606–1.651 s history reads during startup, with roughly
0.96–0.97 s host-thread CPU; those are different traces from its file inspection.
Neither these single startup observations nor the service extraction establish
an overall speedup.

Both actual projects retain their complete original source/graph/reader,
inventory, history, candidate/evidence observations, camera and quiet chat.
The compiler's 13 captured sources and the four fixed portable checker resources
retain their exact original bytes. Broader jobs, remaining preparation waits and
versioned layout/order/retirement remain open. No core/runtime/agent changes,
new kernel refinement, cold installation or live-model validation are claimed.

## Proposal check observations (November 24)

The [validation record](../results/proposal-observations-20261124.json) extends
the existing job publisher to the actual shared application's proposal check
and journal/controller recording boundaries. Timers named
`verification.proposal.check`, `.record` and `.total` retain wall time and
current-thread CPU time under the original tool trace. These are separate from
accepted implementation checks, compiler preparation and HTTP response spans.
The total starts after the observation is created; it includes the terminal
publication but excludes prior decoding/binding checks and later model replies.

The generated subject distinguishes the original draft session/contract from an
accepted reference. Draft completion only makes a proposal available for host
review; it cannot display acceptance or implementation evidence. The existing
revision-bound query and chat activity row carry progress, with folded terminal
details. Transfer retry does not run another check or make a model request.
Duplicate displayed proposals retain their existing observation. Replacement,
cancellation and restart keep their original lifecycle rules.

The browser example uses the real finite-data validator, which does not need a
solver or compiler for proposal validation. Its timings do not measure Lean
compilation or model quality, and are not an isolated before/after speedup.
The final two samples recorded 13.419–19.039 ms for checking,
2.905–3.512 ms for recording and 16.492–22.817 ms for the measured total.
Checking was predominantly current-thread CPU time. Rebinding, method/schema
consistency checks and data validation are still one aggregate; these samples
do not attribute the cost to a particular substep.

Serialized retained-tab two-rAF p95 was 35.3 ms with ordinary motion and
34.8 ms with reduced motion. Actual Riemann and EthLib cache-hit file opens
took 24.322 s and 2.217 s respectively, preserving the original complete
source/graph/reader/inventory/history/candidate/evidence observations. These are
single preparation observations, not paired speedup measurements. They show
that significant initial preparation waits remain even when compilation is
cached. Idle navigation requested no job or preview body and changed no chat.
Compiler-captured sources and the original input/resource checks are unchanged.
Remaining compilation, navigation and response-body waits are separate work.

## Shared native wire definitions (November 24)

The [validation record](../results/native-wire-20261124.json) covers the complete
application-owned session projection and host command vocabulary. Rust emits
the root and command JSON Schemas plus browser validators/types, replacing the
handwritten native protocol definitions. The session borrows existing publication
owners and serializes their lightweight references. Model/provider labels expose
no concrete gateway or credential fields. Actual approval remains with the host's
original gateway/session/endpoint/model/judge comparison and consuming processor.

Connection selection counters are canonical positive-u64 decimal strings, or
null before selection. Numeric, null, noncanonical or overflowing counters in an
approval command are refused. The browser also validates commands before HTTP.
Typed exhaustive classification preserves the original operation/model-use rules;
it cannot authorize execution. The architecture guard keeps the native barrel
to generated re-exports and derived history aliases.

The owned generator uses a unique required string tag to dispatch exclusive
object unions. A pure Node codec probe compares that lookup with a sequential
union of exactly the same branches. Nine interleaved batches per type/strategy
validate and JSON-encode 1000 preconstructed commands after warmup, consuming and
checking encoded lengths. The table contains median batch means, not individual
event p95 or a new CI budget.

| Validation and encoding, median batch mean | Tag lookup | Same branches, sequential |
| --- | ---: | ---: |
| File/folder inspection | 0.617 µs | 8.541 µs |
| Instruction | 0.484 µs | 2.760 µs |
| Source snapshot | 0.440 µs | 6.152 µs |
| Processor review | 1.158 µs | 2.555 µs |
| Delivery dispatch request | 1.455 µs | 6.371 µs |

The former GUI did not validate outbound commands, so this compares two codec
implementations for the new check, not old/new GUI performance. It excludes
HTTP, queuing, compiler/model work and browser presentation. This migration does
not address the separately recorded compiler wait or approximately 40 ms large
response tail.

The preceding commit's CI trace also exposed an asynchronous counter-fixture
race: a click completed before its settings fetch, so no JS submit handler was
installed and the preview's existing CSP refused the default form submission.
The authored fixture now disables submission through initialization; recovery
deliberately holds the real settings response, observes disabled/zero state and
then confirms the changed increment. No preview CSP or isolation rule changed,
and the original count, refresh, retirement, missing-entry and restart assertions
remain. This tests fixture readiness, not arbitrary embedded-app readiness.

The initial live deployment also caught a cache-binding regression that the
single-build fixtures did not cover. Adding the availability schema to the Atlas
entry module changed a captured compiler implementation source. Riemann therefore
performed extraction/search again (**190.614 seconds**) and the original-cache
fidelity assertion failed. The final change restores that compiler source and
projects availability through a separate borrowed `view::session` DTO. Captured
compiler implementation files remain identical to the base revision; the actual
definition/resource/input checks remain intact. A new structural guard prevents
wire-schema derives in that captured entry. No cache file or fingerprint rule
was edited to force reuse.

Final checks passed **332 Rust tests**, **43 TypeScript tests**, **33 architecture
checks / 47 owned sources**, all **37 native schemas / 74 generated files**,
**35 required public TUI journeys** and **68 browser journeys**. Both serialized
motion gates passed; retained-tab two-animation-callback p95 is **29.0 / 29.3 ms**
for normal/reduced motion. These callbacks are presentation proxies, not physical
paint or INP. The 3000-case corpus and full-scan negative control also passed.

The final actual Riemann/EthLib integration reuses the original pre-change
compiler/search/rendering artifacts. Complete original source/graph/reader,
inventory, history, candidate and evidence observations match exactly. Camera,
selected source line and quiet chat are retained. The generated root validator
accepts both real hosts; their public connection labels remain identical, with
the intentional unselected-counter change from numeric zero to null. Cache-hit
opens take **19.289 / 2.112 seconds** in this smoke, not a paired speedup benchmark.
The large-project preparation wait remains separate from the resolved metadata
invalidation regression.

## Shared delivery observations and response-body probe (November 24)

The [validation record](../results/delivery-observations-20261124.json) covers
moving the immutable export queue and host review selection into the shared
application, deleting their CLI implementations and generating the delivery
DTOs. Exact canonical positive-u64 counters replace numeric browser review
serials. Original kernel authorization, payloads and consuming dispatch remain
unchanged. This boundary cleanup makes no graph-interaction speedup claim.

A separate read-only probe investigated the preceding fixture's approximately
40 ms tail. Both idle deployed hosts served exactly retained bytes through the
authenticated public API. Each client mode made 24 sequential complete reads;
the first four were excluded from summaries. Functional checks ran concurrently,
so these are diagnostic loopback samples, not an isolated throughput benchmark
or a browser paint/INP measurement.

| Complete response p95 | Riemann | EthLib |
| --- | ---: | ---: |
| 111-byte empty-evidence body, Node fetch | 1.021 ms | 1.062 ms |
| Prepared source body, new connection per read | 1.592 ms | 2.259 ms |
| Prepared source body, one reused HTTP connection | 0.955 ms | 41.389 ms |
| Prepared source body, Node fetch | 42.580 ms | 42.233 ms |

The source bodies contain **69,201 / 146,985 bytes**. Node fetch's header p95
is **2.422 / 2.317 ms**, while the interval from headers to complete body is
**41.448 / 41.526 ms**. All 72 corresponding server response spans per project
take at most **0.465 / 0.570 ms**. The reads start no application jobs or source
materialization. The tail is therefore observed after headers during completion
of a retained response, independently of compiler or graph regeneration. Its
size/connection sensitivity warrants a packet-level transport investigation;
these observations alone do not establish Nagle, delayed ACK or a browser cause.
No socket settings, client connection policy or response implementation changed
in this migration. The larger-body result also limits the earlier small-body
observation: low empty-evidence latency does not establish low source latency.

A synthetic paired probe used the same installed `tiny_http 0.12` frozen
`Arc`/`Cursor` response path with a 146,985-byte body. Interleaved default-chunked
and explicit-length responses both retained the tail: complete p95 **41.631 /
41.325 ms** across 24 reads per mode, excluding four warmups. Changing only
response framing therefore did not remove it in this probe. This experiment is
neither an actual GUI fix nor evidence for a particular TCP mechanism.

The final local checks passed **328 Rust tests**, **42 TypeScript tests**,
**26 architecture checks / 45 owned sources**, the **70 generated files**,
all **35 required public TUI journeys** and **68 browser journeys**. The TUI
gate now includes the actual frozen-delivery, clipped-heading and replay path.
Both serialized motion gates passed; retained-tab two-animation-callback p95
is **36.0 / 33.9 ms** for normal/reduced motion. These callbacks are presentation
proxies, not physical paint or INP. The 3000-case differential corpus and actual
full-scan performance negative control passed without weakening their assertions.

After deployment, both actual projects retain the identical source/graph,
reader, inventory, conversation, candidate and evidence observations. Hover,
tab return and node/source-line selection preserve the camera and chat. Ordinary
graph navigation fetches no job or preview body. Cache-hit file opens take
**23.222 / 2.121 seconds** for Riemann/EthLib in this integration smoke; this is
not a paired speedup benchmark and the large-project preparation wait remains.

## Immutable preview descriptions (November 24)

The [validation record](../results/preview-publications-20261124.json) covers
moving static capture out of CLI composition and generating its read model.
Frequent status now contains only idle/ready/unavailable and the original frame
and publication identities. `/api/preview?revision=…` shares complete frozen
metadata; capture and missing-asset observations publish at their mutation
boundaries. The original bounded offline file worker and isolated host listener
remain responsible for actual content and browser response policies.

The actual confined-capture/browser journey retains a running counter through
failed description transfer, command-free retry and delayed diagnostic updates.
An explicit recapture retires the former origin and a delayed old description
cannot replace the new frame. Twenty supplementary-Unicode diagnostics produce
a **41,355-byte** body with a **184-byte** status. Twenty direct reads add zero
commands and leave the materialization count at **24**. Diagnostics remain
available under Details so the live application stays in the viewport.

The direct Node/browser-fixture read intervals include HTTP, JSON validation and
assertion overhead under parallel functional validation. They alternate between
roughly 3 and 44 ms despite retained bodies. This is an investigation target,
not an exclusive-stage CPU or natural-throughput benchmark; transport attribution
requires a separate controlled probe.

Rust **322**, TypeScript **42**, architecture **24** / **40 owned sources** and
the **60 generated files** passed their checks. The required public TUI passed
all **34 journeys**. All **68 browser journeys** and both serialized motion
gates passed; retained-tab two-animation-callback p95 is **40.2 / 35.9 ms** for
normal/reduced motion. These callbacks are presentation proxies, not physical
paint or INP. The 3000-case corpus, repeated algorithm budget and its actual
full-scan negative control also passed. After rendered-screen review, folding
long diagnostics was checked again through all three affected preview journeys,
including an explicit assertion that the live frame stays in the viewport.

Both deployed projects retain their exact original source/graph/reader,
inventory, conversation, candidate and evidence observations. Prepared caches
still hit; source-line selection, chat and camera remain stable. Idle graph
navigation requests neither preview nor verification-job bodies. No new graph
event speedup, live-model validation or proof/refinement claim is made here.
Remaining native-wire definitions, broader job views and versioned layout/order
and retirement requirements stay open.

## File-local selection and current event audit (November 24)

The [measurement record](../results/selection-retention-events-20261124.json)
contains one fresh normal-motion CDP capture per deployed project, with no
concurrent build/test/profile. The original source/graph/reader, inventory,
evidence, candidate observations and conversation revisions remain identical.
Both projects reuse their prepared caches. These are diagnostic captures with
profiling overhead, not paired speedup measurements or field percentiles.

| Synchronous event dispatch p95 | Riemann | EthLib |
| --- | ---: | ---: |
| Project-map wheel, 24 events | 3.447 ms | 5.931 ms |
| Dependency-graph wheel, 24 events | 0.662 ms | 0.937 ms |
| Hover, 40 pointer events | 0.112 ms | 0.062 ms |
| Pan, 31 pointer events | 0.212 ms | 0.312 ms |
| Resize | 2.541 ms | 0.631 ms |
| File/map tabs, 24 clicks | 17.713 ms | 9.487 ms |
| Fit View | 1.034 ms | 0.996 ms |
| Different-node selection, 8 clicks | 14.107 ms | 6.378 ms |

Input/two-animation-callback p95 is **23.0 / 12.8 ms** for different-node
selection, **42.1 / 29.6 ms** for tabs and **30.2 / 35.8 ms** for hover.
These include browser scheduling; they are not physical paint or INP. Hover,
resize, tab return and selection preserve the camera. Measured gestures issue
only status reads, with no commands or analysis/history/source/evidence/job-body
reads. Setup and the profiler's source-line oracle remain separate reads.
Actual pointer selections also preserve chat DOM and select the original source
line. The Riemann/EthLib files contain 15/42 observed declarations, within their
175,187/2,517-file inventories; this does not cover every graph in those projects.

Cache-hit file preparation still takes **21.496 / 2.069 seconds** in these
captures. Riemann's analysis span is **20.901 seconds**. Its real build log
records **10.972 seconds** for build/setup and input resolution, then **5.960
seconds** to validate all **43,477 inputs / 5,992,114,707 bytes**. Cache reuse
still performs this work. These stages identify the remaining wait; nested
spans must not be added as exclusive CPU time. Host-thread CPU excludes child
processes. The renderer change does not claim to reduce this compiler wait.

The UI correction separates the file's selected declaration and unsent meaning
input from graph lifetime. Actual graph replacement no longer discards them;
selecting a node in another Lean file and returning preserves both. Selecting
another saved analysis restores that record's original requirement. The graph
now only renders prepared data and reports the original selected name/file/line.
No Rust executable, execution authority, checker or proof changed in this slice.

All **67 browser journeys** passed without skips or retries. Both serialized
motion gates passed; retained-tab two-animation-callback p95 is **38.6 ms** with
normal motion and **29.5 ms** with reduced motion, below the existing 250 ms
budget. The unchanged pinned executable passed the required **34 public TUI
journeys**, whose decoded finite/refutation/restart/stop screens were inspected.
The record retains the original failure, intermediate failed attempts and the
complete successful rerun. This is installed-tool/scripted-inference validation;
remaining native-wire generation and versioned layout/order/retirement stay open.

## Verification-stage observations (November 24)

The [validation record](../results/verification-job-observations-20261124.json)
adds separate source-read, checker-preparation, checker-execution, freshness-read
and result-retention spans. A revision-bound application observation carries
the original accepted contract and exact issued-input hashes. The existing chat
activity row displays the actual stage; polling and transfer retry cannot start
a check. Shared evidence publication now follows the actual project mutation,
so completion and source-edit invalidation appear before the next model reply.

The actual offline Z3 journey exercises a failed progress transfer, command-free
retry, cancellation of issued dispatch, a delayed old response, a distinct
successful job and source editing before a deliberately held model reply.
Its trusted host launcher holds dispatch before Z3 starts and later runs the
original confined solver; its timings validate instrumentation, not solver
throughput or model-transport cancellation latency. The ready status is 182
bytes and the observed retry sends zero commands. Projection and response spans
are below 0.2 ms in this fixture; nested spans are not exclusive CPU work.

All 67 browser journeys and 34 public TUI journeys passed locally. Serialized
retained-tab two-animation-callback p95 was **48.1 ms** with normal motion and
**34.9 ms** with reduced motion, below the existing 250 ms budget. These are
presentation proxies with installed Lean and empty analysis stores, not physical
paint, cold installation or live-model validation. After deployment, both actual
projects preserved their exact source/graph/reader, inventory metadata, evidence,
conversation and candidate observations. File reopening reused prepared caches;
hover/tab return preserved the camera and idle navigation fetched no job body.
No new event-profile comparison is claimed for this observation-only slice.

## Accepted-contract publications and deployed event audit (November 24)

The [measurement and validation record](../results/evidence-publications-20261124.json)
contains one fresh normal-motion CDP capture per actual project after deployment.
Both reuse prepared compiler/search/renderer artifacts. No other test or profile
ran during these sequential captures. Profiling adds overhead; these are local
diagnostic event distributions, not field percentiles or a before/after speedup.
The selected files are Riemann's `AdelicClassFiniteness/DictGen.lean` and EthLib's
`Networking/Gossip/Parameters.lean`; their inventories retain 175,187 and 2,517 files.

| Synchronous event dispatch p95 | Riemann | EthLib |
| --- | ---: | ---: |
| Project-map wheel, 24 events | 3.804 ms | 6.790 ms |
| Dependency-graph wheel, 24 events | 0.882 ms | 1.152 ms |
| Hover, 40 pointer events | 0.145 ms | 0.106 ms |
| Pan, 31 pointer events | 0.176 ms | 0.578 ms |
| Resize | 1.732 ms | 1.232 ms |
| File/map tabs, 24 clicks | 18.592 ms | 13.588 ms |
| Fit View | 1.067 ms | 0.715 ms |
| Different-node selection, 8 clicks | 24.188 ms | 12.584 ms |

Node selection remains the largest measured synchronous interaction. Its
input/two-animation-callback p95 is **38.0 / 18.9 ms**; tabs are **43.7 / 37.4 ms**.
These callbacks are scheduling proxies, not physical presentation or INP.
Hover, resize, tab return and both repeated/different-node selection preserve
the graph camera. All measured gestures send only status reads: **zero commands,
source-body or evidence-body requests**. Source-line assertions use the complete
original prepared source. Source maps are unavailable for these pinned bundles,
so sampled compiled frames do not establish exclusive source-function costs.

The large remaining wait occurs on explicit file preparation. Riemann takes
**22.752 s** from the explorer click to prepared readiness, including **19.781 s**
inside analysis and its nested **17.033 s** Lake preparation. The streamed log
separately reports **11.870 s** for build/setup/input resolution and **4.421 s**
for complete validation of **43,477 inputs / 5,992,114,707 bytes**. EthLib's
corresponding browser wait is **2.350 s**, with **0.985 s** inside analysis.
These are single cached observations, not comparable to prior repeated API
medians. Native stages overlap; do not add parent and child intervals. Current
status handling p95 is **1.223 / 1.396 ms**. Lake resolution and complete-input
validation therefore remain the first preparation targets; node selection is
the clearest remaining interaction target.

Accepted/proposed contract meaning and evidence now have an independent
application-owned immutable publication. Status reports its availability and
exact revision; `/api/evidence?revision=…` shares complete frozen bytes. Disjoint
states retain accepted meaning during replacement and distinguish current
coverage from retained outputs and typed failures. Original canonical contracts,
reviews, backend strength/scope and source/output hashes are preserved. Invalid
or oversized observations retire the old body while retaining the original;
there is no partial successful view. Browser reads cannot construct receipts,
grant authority or initiate verification.

The actual Z3 rejection/repair/restart browser journey transfers a **3,940-byte**
complete evidence body with a **102-byte** status projection. Twenty sequential
direct body reads add **zero materializations**; native response p95 is
**0.148 ms**, while complete browser fetch median is **42.2 ms**, including the
transport and scheduling boundary. HTTP 503 recovers through **Retry evidence**
without a command. Required nullable fields and exact positive-u64 decimal
references are generated from Rust; original repair queue/journal data remain
unchanged. Restart retains accepted meaning without restoring current coverage.
Both actual projects currently have no accepted/proposed native contract, so
their evidence bodies are **111 bytes**, and this fixture's large-review saving
is not a measured speedup of those empty observations.

The pinned binary passed Rust **314**, TypeScript **40**, **23 architecture
checks**, reproducible generation of **52 files**, public TUI **34** and the full
browser gate **66**, with no skips. Both serialized motion gates passed: warm
tab p95 **29.5 / 32.9 ms**, compiler-held paging maximum **29.4 / 34.0 ms**.
The first full browser attempt exposed one remaining old inline-status assertion
in the portable-clone test; its replacement reads the actual evidence publication,
and the complete rerun passes. Decoded failure/restart screens and deployed map
captures were inspected. Complete original source/graph, reader, inventory,
candidate and conversation identities match the private before-deployment capture.
Both hosts finish idle and unapproved. No kernel or checker execution behavior changed;
general verification-job views and the remaining architecture migration stay open.

## Separate candidate and history publications (November 24)

The [implementation and validation record](../results/candidate-observations-20261124.json)
separates candidate meaning observations into two application-owned immutable views:
the selected/running record and the complete retained history. Status contains
their bounded availability references. Progress, selection and source
invalidation freeze only the selected record; they do not serialize or hash
unchanged history. Exact-revision queries share existing bytes and cannot start
inference, compilation or a command. The old inline `analysis` and
`analysis_history` status fields and handwritten meaning-record/input validators
are removed. The persisted journal format remains unchanged; presentation uses
canonical decimal strings for original u64 sequences.

The targeted native browser journey retained **400 original observations**:
the JSON history envelope was **3,340,291 bytes**, the selected record was
**8,440 bytes**, and their status projection was **238 bytes**. Injected HTTP
503 recovered through **Retry analyses** without a command, while source
preparation remained independent. Map/file navigation and changing the selected
observation performed **zero additional history reads**. A held former selection
could not replace the later displayed observation. Restart retained the exact
seeded journal and all 400 selectable runs, with no restored approval or new
model requests. These figures are one protocol workload, not an old/new latency
comparison, field percentile, live-model or cold-install claim. The fixture is
seeded from one actual preceding scripted-model job; source/graph preparation
uses the actual installed host workers and compiler.

The two actual deployed projects currently have **zero saved candidate runs**.
Their original source/graph/reader and inventory observations were captured
before deployment. The large-history result must not be presented as a measured
speedup of their currently empty histories.

Serialization-contract schema generation preserves required nullable fields,
opaque method diagnostics and lossless sequence values. Generation still rejects
unknown constraints; unconstrained diagnostic JSON is explicit data, not a
plugin decoder or authority. Typed availability separates absent, ready and
unavailable observations. A bad projection retires that role's old body while
retaining its original records; it cannot admit a prefix or invent an empty
successful history. Current/history envelopes share a 64 MiB retention budget
and history contains at most 1000 runs. This does not bound raw host records,
in-flight response references or total process memory. Candidate assessments
remain separate from current accepted-project evidence and kernel job tickets.

The exact binary passed all **66 browser cases**, public TUI **34 required
cases**, Rust **308**, TypeScript **39**, **23 architecture checks** and
reproducible export of **42 generated files**, with no skips. The strengthened
candidate-view tests were rerun after adding the shared-quota case; individually
valid current/history envelopes still cannot exceed the combined budget. Both
serialized motion gates exited zero. Prepared-tab p95 was **31.8 / 31.6 ms**,
compiler-held paging maximum **34.6 / 35.2 ms**, and first readable source
**1,194 / 1,197 ms**, within the existing 250 ms / 5 s budgets. These are
DOM/two-animation-callback proxies, not physical presentation or field INP.
Decoded TUI recovery/restart and native failure/selection screens were inspected.

Both actual GUIs now run the pinned binary and asset tree. Complete original
source/graph, reader and inventory metadata hashes and conversation revisions
match the before-deployment capture without normalization. Source preparation
reuses the existing bound compiler/search/renderer cache; no implementation
attribution is rewritten. The single GUI captures (**15.616 / 2.604 s** to
cached graph readiness for Riemann/EthLib) include first browser navigation and
are not comparable to the prior repeated warmed API medians. Both hosts finish
idle and unapproved. Accepted-project evidence, verification-job read models and
remaining native wire/layout migration work remain open.

## Batched exact metadata validation (November 24)

The [implementation and measurement record](../results/metadata-batches-20261124.json)
reduces confined worker startup during complete cache revalidation. Directory
requests follow actually observed parent frontiers, in groups of four; present
configuration files use the existing read-only worker's batches of at most 256.
Every original/resolved path check, complete-output admission, exact file byte
comparison, missing-path observation and filtered Lean inventory check remains.
The original 8 MiB response limit is checked before filtering, in addition to a
64 MiB aggregate frame. No successful validation is reused between operations,
and neither caller's cache check is removed.

Three serial explicit public inspection requests per project, before and after,
used an idle, unapproved host, the same selected file and fully warmed artifacts.
No tests or profiles ran concurrently with these samples. These are diagnostic
medians with 50 ms completion polling, not field percentiles or file-render times.

| Cached inspection | Before median | After median | Reduction | Worker calls per cache validation |
| --- | ---: | ---: | ---: | ---: |
| EthLib `Networking/Gossip/Parameters.lean` | 3,918.529 ms | 1,706.444 ms | 56.45% | 32 → 9, on both checks |
| Riemann `AdelicClassFiniteness/DictGen.lean` | 12,596.045 ms | 11,111.434 ms | 11.79% | 24 → 8; the separate empty check stays empty |

Process counts include nested source capture. EthLib's first/second cache
validation medians fell from **1,592.830 / 1,592.356 ms** to
**478.217 / 480.718 ms**; Riemann's first fell from **1,241.999** to
**429.177 ms**. File-local preparation duration excludes the navigation-note
validation that follows it, so it must not substitute for whole-request timing.
New static `analysis.metadata.validate`, `analysis.metadata.directory_batch` and
`analysis.metadata.file_batch` spans separate these observations from source
capture and Lake work. Nested spans overlap, and host-thread CPU excludes child
execution.

Riemann still validates **43,477 compiled inputs / 5,992,114,707 bytes** on a
prepared open. Its last measured after-log records **4.890 s** for build/setup
and input resolution, then **2.339 s** for complete byte fingerprinting. These
remaining costs explain why batching metadata alone gives a smaller total gain.
They are single logged phases, not additive estimates from overlapping spans.

The changed observation implementation deliberately changes the compiler plugin
identity. Initial graph/explanation preparation was **197.483 s / 49.585 s** for
Riemann/EthLib and is retained separately, before all three cached after-samples.
Installed dependencies remained present; this is not cold-install validation.
Complete original compiler snapshots, counterexample reports and rendered
readings compare equal across the old/new implementations, without field
normalization. Content-addressed compilation and view IDs change to preserve
their actual implementation attribution. Inventory metadata, original reader
references and conversation revisions remain unchanged; both deployed hosts
finish idle and unapproved.

The exact binary passed Rust **305**, **22 architecture checks**, public TUI
**34 required cases** and all **65 browser cases**, without skips. The full
browser report records 65 expected, zero unexpected/flaky/skipped and no errors;
the outer sequential runner ended with code 143 after those detached cases
completed, so its missing exit receipt is retained rather than reported as zero.
Both remaining serialized motion gates were run separately and exited zero.
Prepared-tab p95 was **32.0 / 31.2 ms**, and compiler-held paging maximum was
**31.4 / 35.5 ms**, under 250 ms; readable source appeared in
**1,193 / 1,189 ms**, under 5 seconds. These are DOM/two-animation-callback
proxies, not physical presentation or field INP. Decoded TUI recovery/restart
screens, real prepared-Lake and compiler-failure screens, and both deployed
source/map views were inspected. No kernel, authority or Lean proof changed;
filesystem concurrency and physical I/O remain adapter obligations.

## Resource-local preparation and diagnostic publications (November 24)

The [implementation and validation record](../results/preparation-views-20261124.json)
moves preparation outcomes, durations and diagnostic framing into the application.
The old CLI DTO and handwritten browser schema are deleted. The host supplies
clocks and normalized output; immutable observer handles bind the original host
session, sequence and resource. Replacement, completion, backwards time, refresh
and eviction cannot revive an older observer. These display observations carry
no kernel job ticket, approval or evidence authority.

Status contains only a small generated progress record and exact log reference.
Changed output freezes a bounded UTF-8 tail; queries clone its existing bytes.
The actual installed Lean/Lake fixture retained **65,536 diagnostic bytes** while
its JSON-encoded progress projection was **352 bytes**. Local **Retry log**
recovered an injected HTTP 503 with **zero host commands**, while compilation
remained held. This measures avoided diagnostic retransmission, not faster
compilation. Retention is 64 resources, with 64 KiB of diagnostic text per resource.
Long non-BMP compiler failures remain readable after restart: step labels and
failure summaries are byte-bounded, and shortened diagnostics are explicit.

The exact final binary passed all **65 browser journeys**, Rust **302**,
TypeScript **39**, **22 architecture checks**, reproducible export of **32**
generated files and public TUI **34 required cases**, without skips. Decoded
failure/restart screens and streaming, transfer-failure and Unicode diagnostic
screens were inspected. The record retains the earlier route-drain fixture race,
a missed multiline fixture field, and the corrected full-suite rerun.

Serialized browser gates also passed in both motion modes, without concurrent
checks or profiles. Prepared-tab p95 was **36.1 / 36.6 ms**; paging while the real
compiler was held had maximum **26.1 / 36.6 ms**, under the 250 ms budgets.
Initial readable source appeared in **1,192 / 1,203 ms**, under 5 seconds. These
are DOM/two-animation-callback proxies, not physical presentation or field INP.
The prior CI run passed all 64 then-current journeys and one complete motion
gate, but its 20-minute whole-job deadline interrupted the second gate. The
workflow now allows 30 minutes; individual test and numeric limits are unchanged.

Further analysis of the existing actual-project capture identifies the remaining
preparation targets. Riemann's cached setup took 22.820 seconds: the worker log
records **10.148 seconds** for Lake build/setup and input resolution, followed by
**4.575 seconds** validating **43,477 files / 5,992,114,707 bytes**. EthLib's
4.231-second setup contains two separate cache-validation spans of **1.549 /
1.604 seconds**, each with **31 direct process calls**, mostly 41–51 ms each.
At that revision `collect::check_metadata` performed these observations individually;
`overview::apply_compiled` invokes cache validation again for navigation notes.
The existing confined `file::read_batch` provides a concrete batching boundary
without removing original/resolved policy checks or complete-output admission.
These are observations from the preceding inventory-publication capture, not a
new rendering comparison. Host-thread CPU excludes child workers, and nested
spans must not be added as exclusive CPU time. Lake/input validation, broader
job/evidence read models and versioned layout/retirement remain work.

## Frozen inventory and complete typed directories (November 24)

The [implementation and measurement record](../results/inventory-publications-20261124.json)
moves inventory, directory, navigation chunk and status DTOs into the shared
application and generates their runtime validators. A complete inventory freezes
once outside the status lock. Its envelope is the sole content identity, binding
original metadata and a lossless prepared-cache generation. The former separate
whole-metadata serialization/hash is deleted. Exact queries return existing bytes;
the unversioned `/api/overview` route and handwritten wire schemas are retired.

Ten sequential raw-body requests per project compared immutable old/new services,
with the same original metadata and no concurrent validation/profiling:

| Inventory request | Riemann before / after | EthLib before / after |
| --- | ---: | ---: |
| Response body bytes | 35,887,894 / 36,238,294 | 24,204,275 / 24,209,335 |
| Node complete response p50 | 1,862.115 / 61.839 ms | 1,159.687 / 45.403 ms |
| Node complete response p95 | 1,891.940 / 77.211 ms | 1,191.839 / 53.710 ms |
| New cached response host-thread CPU p95 | 21.869 ms | 15.955 ms |

File sizes are now canonical decimal strings, so the body is slightly larger.
Comparison normalizes only that representation and excludes the retired
standalone revision/new cache-generation hint: every original file, name, count,
import, symbol and description matches. The improvement removes per-query JSON
work rather than omitting data. Native old `serialize_respond` spans are untagged
and include visible status/metrics polls (30 / 11 records); their diagnostic p95
is 1,081.839 / 768.037 ms. The primary comparison is the ten identified HTTP
responses; new `inventory_respond` has exactly ten records per project. These are
small within-run samples, not universal or field percentiles.

One actual Chrome capture per deployed project used 1440 × 960, normal motion,
CDP traces and sampled CPU profiles, with all seven served JS bundles matching
the hidden-map build. Riemann's two distinct inventory revisions transferred in
136.548 / 102.824 ms; EthLib's one in 393.519 ms. Initial one-time materialization
remains work: Riemann's observed update span took 1,146.126 ms outside the status
lock. It is not a zero-cost inventory or a universal startup-speedup claim.

| Synchronous handler p95 | Riemann | EthLib |
| --- | ---: | ---: |
| Project-map wheel | 4.086 ms | 5.764 ms |
| Graph wheel | 1.066 ms | 0.893 ms |
| Hover | 0.121 ms | 0.124 ms |
| Pan | 0.387 ms | 0.385 ms |
| Resize | 1.452 ms | 1.022 ms |
| Tabs | 15.843 ms | 10.392 ms |
| Alternating-node selection | 13.298 ms | 9.052 ms |

Gesture phases fetched no inventory/directory/source/history/reader body and
issued no command. Tab input/two-rAF was 39.5 / 26.8 ms; distinct-node click proxy
was 24.8 / 14.9 ms. Single captures do not establish a general rendering win.
Complete cached graph readiness still took **22.820 / 4.231 seconds**, with cache
hits and original input checks. Initial map layout and independent job/log/evidence
read models remain work.

The exact deployed binary also passed serialized numeric gates in both motion
modes. Prepared-tab p95 was **27.6 / 32.4 ms**, and held-compiler paging maximum
was **29.4 / 32.2 ms**, each under 250 ms. Initial source was visible in
1,166.5 / 1,169.6 ms under its 5-second budget. These are DOM/two-animation-callback
proxies, not physical presentation or field INP.

The migration fixes a real CI-exposed directory bug: formatted `list_files`
took an OS-order subset of 500 entries and sorted only that subset, making some
files unreachable. The GUI now observes complete typed lists through the same
original/resolved policy-bound worker, at most 100,000 entries, and publishes
immutable envelopes. Limits/failures admit no prefix or omitted-entry sentinel;
literal newline/Unicode names survive. Reader/directory retention shares one
64-entry / 64 MiB table per owner. Retained folder folding/reopening performs no
read; missing folders retry locally. A 1,201-entry actual browser journey reads
the last file and a newline/Unicode filename. Inventory failure/retry does no
recapture, stale inventory/chunk requests conflict, delayed original bodies cannot
replace refreshed nodes, and restart reobserves actual files.

Large-project chunks bind the exact parent publication and requested path, check
scope before/after reading prepared artifacts, and memoize complete envelopes
(up to 64 / 64 MiB per inventory). Cache hits do no artifact I/O; initial reads
use prepared navigation artifacts without source workers, compilation or models.
Canonical order/count/size, original parent/path and count/byte eviction are
checked at the application boundary and through typed client tests.

All **64 browser journeys**, Rust **298**, TypeScript **38**, **22 architecture
checks**, the 3,000-case differential corpus and public TUI **34 required cases**
passed, alongside Clippy, exact-CI Python lint, formatting and reproducible schema
export. Both browser motion gates passed separately. Decoded TUI failure/restart/
continuation screens and actual source/tree/map screens were reviewed. The record
also retains the real CI trace identifying a metadata-job wait in the history
fixture, proper draining before host restart, and selection of the actual frozen
library instead of its separate compiled-module cache. Inference remains scripted
in regression gates; no new live-model, cold-install, confidentiality or refinement
claim is made.

## Reader publications and local paging (November 24)

The [implementation and measurement record](../results/reader-publications-20261124.json)
removes complete reader data from status polling and deletes the queued paging
command, server page DTO and unused generic mapping. The application freezes
complete policy-bound observations before compiler preparation, retains at most
64 files / 64 MiB of serialized envelopes, and serves exact cached bytes outside
the status lock. The browser fetches each wanted observation once and derives
200-line pages locally. Paging requires no file/compiler/model I/O, including
while the actual Lean compiler is held. Reader failures retry within the file;
replacement/refresh/eviction retires old query references, and late bodies cannot
replace newer input. Original source/graph/history comparison hashes are unchanged.

Twenty sequential raw-body probes per project used the same selected files and
immutable deployments, without concurrent validation or profiling:

| Measurement | Riemann before / after | EthLib before / after |
| --- | ---: | ---: |
| Status bytes | 20,644 / 11,758 | 6,043 / 3,190 |
| Native status processing p95 | 2.587 / 1.971 ms | 2.069 / 1.727 ms |
| Node fetch/body-read p95 | 6.552 / 42.053 ms | 42.559 / 41.613 ms |

Status bytes fell **43.04% / 47.21%**. End-to-end Node timing did not improve
consistently; approximately 42 ms response-body tails remain. Native timing and
payload reduction do not establish a general HTTP speedup. Transport buffering /
ACK behavior needs a separate controlled experiment before attribution. Settled
actual Chrome status response p95 was **5.841 / 4.131 ms**.

Serialized browser gates used the final pinned binary and actual installed
Lean/Lake, with numeric checks enabled in both motion modes:

| User-visible scheduling proxy | Normal motion | Reduced motion |
| --- | ---: | ---: |
| Page change during held compilation, maximum of four | 33.6 ms | 40.8 ms |
| Prepared tab return, p95 of ten | 37.8 ms | 36.2 ms |
| Initial readable source, before compilation completes | 1,136.6 ms | 441.7 ms |

The first two measure DOM outcome plus two animation callbacks, not physical
presentation or field INP. Their gates are 250 ms; initial source has a 5-second
budget. Paging and retained source-only tab return issue no host command or
body query. Regressions cover literal Unicode/JSON, empty text, CRLF/final empty
line, EOF, missing/binary/oversized inputs, failure/retry, delayed old bodies and
restart. The real compiler journey retains streamed logs through timeout,
cancellation and retry. Four other journeys retain actual 35-module Lake
freshness/failure/restart, source-line selection, IDE camera and missing Mathlib.

One fresh actual Chrome capture per deployed project used normal motion,
1440 × 960, CDP traces, sampled CPU profiles and matching source maps. All seven
served JS bundles match the hidden-map build:

| Synchronous handler p95 | Riemann | EthLib |
| --- | ---: | ---: |
| Project-map wheel, 24 events | 4.185 ms | 5.578 ms |
| Graph wheel, 24 events | 0.912 ms | 1.393 ms |
| Hover, 40 pointer events | 0.076 ms | 0.127 ms |
| Pan, 31 pointer events | 0.190 ms | 0.320 ms |
| Resize, coalesced events | 1.147 ms | 0.782 ms |
| File/map tabs, 24 clicks | 12.184 ms | 10.959 ms |
| Alternating-node selection, 8 clicks | 14.917 ms | 9.964 ms |

The tab input/two-rAF proxy was 33.5 / 29.1 ms; alternating-node click proxy was
23.4 / 17.6 ms. Gesture phases issued no commands and fetched no reader/source/
history body. Camera observations remained stable for hover, resize, tabs and
node selection. These single runs include profiling overhead, and small-sample
p95 can be the maximum; historical runs are not a controlled rendering comparison.

The remaining waits are distinct. Complete cached graph preparation still takes
**21.447 / 5.981 seconds**, with real cache hits and original input checks. The
initial Riemann polling phase contains **232.103 ms sampled self time in
`projectMap`** and a **396 ms long task**. This is evidence of initial map work,
not an exclusive attribution of that entire long task. Versioned/incremental
layouts and separate job/log/evidence read models remain migration work.

Rust 293 tests, TypeScript 37, 22 architecture checks, Clippy, generated-schema
reproduction and Ruff passed. The exact deployed binary passed the public TUI
34-case required gate with no skips; decoded restart/failure/continuation screens
and browser source/error/build screens were inspected. Intermediate fixture and
missing-PATH failures, with their corrected reruns, are retained in the record.
Inference remains scripted in these regression gates; no new live-model,
cold-install, confidentiality or whole-application refinement claim is made.

## Prepared source publications (November 24)

The [implementation record](../results/source-publications-20261124.json) separates
prepared source/graph bodies from status. The shared application owns original
snapshots, complete frozen envelopes, a generated reference index and bounded
retention. Queries return exact retained bytes without file/compiler/model I/O.
Publication IDs cover all presentation data, independently of input-source and
graph IDs. Changed/unavailable input invalidates every related retained graph.
Source and history now share bounded serialization/framing and browser recovery.

Twenty sequential raw-body probes per project, against the same selected files,
compared the immutable `d7aaf6b` service with the new pinned deployment:

| Measurement | Riemann before / after | EthLib before / after |
| --- | ---: | ---: |
| Status response bytes | 89,654 / 20,644 | 152,838 / 6,043 |
| Native status processing p95 | 7.865 / 1.529 ms | 17.261 / 0.753 ms |
| Node fetch/body-read p95 | 11.350 / 42.166 ms | 18.940 / 41.801 ms |

Status bytes fell **76.97% / 96.05%**. Complete decoded source, graph and history
values have identical comparison hashes across deployment. Node's end-to-end
probe became slower despite lower native computation; transport/client timing
remains unattributed. The actual browser's settled state-response p95 was
**6.446 / 4.853 ms**. These are different measurement boundaries and small
within-run percentiles, rather than a universal HTTP latency claim.

Two sequential actual Chrome captures used normal motion, 1440 × 960, CDP
tracing/CPU sampling, matching source maps and no concurrent validation job:

| Synchronous handler p95 | Riemann | EthLib |
| --- | ---: | ---: |
| Project-map wheel, 24 events | 4.878 ms | 6.585 ms |
| Dependency-graph wheel, 24 events | 1.083 ms | 1.339 ms |
| Hover pointer movement, 40 events | 0.109 ms | 0.089 ms |
| Drag pointer movement, 31 events | 0.189 ms | 0.359 ms |
| Resize, coalesced events | 1.107 ms | 0.873 ms |
| File/map tab click, 24 events | 15.181 ms | 10.486 ms |
| Alternating-node selection, 8 clicks | 17.514 ms | 11.635 ms |

The input/two-rAF scheduling proxy was 41.3 / 30.4 ms for tabs and 53.9 / 35.1 ms
for alternating-node selection. It is not physical presentation or field INP;
small-sample p95 can be the maximum. Rendering did not improve consistently
across historical diagnostic captures. The graph camera stayed fixed across
hover, resize, tabs and selections. Source-line assertions applied to all 16
Riemann selections and 12 of 16 EthLib selections; generated declarations without
reported lines do not receive an invented position. Fit View and initial/same-node
clicks are recorded separately.

Each application fetched history once and source once during setup. The profiler
adds a separate source-body read for its declaration/line oracle. Neither body
was fetched during gestures, and gestures issued no host command. All seven
served JavaScript bundles match the hidden-map build byte-for-byte. Original
compiler/input checks remain: complete cached graph readiness still took
**22.656 / 5.397 seconds**, with cache hits. Actual EthLib startup/host-refresh
history materialization was 89–95 ms per observed span after removing duplicate
serialization; these were not controlled model-turn measurements.

The source regression retains a 408,776-byte complete body behind a 2,762-byte
status fixture. It covers local failure, same-ID conflict recovery, the
three-attempt limit, explicit retry, quiet cached navigation, delayed older
responses and restart. A real 35-module Lake journey covers fresh dependency
inputs, actual compiler/configuration failures and late index views. A held
model-list acknowledgement also reproduces and fixes a swallowed approval click;
only approval itself disables its button, while the current binding/visible
heading checks remain. Source failure no longer leaves a lone source spinner.

Rust 292 tests, TypeScript 36 tests, 22 architecture checks and the required public
TUI 34-case gate passed. Serialized numeric gates in both motion modes passed at
39.3 / 39.7 ms against 250 ms. Separate reports retain the intermediate approval
failure and a routed-poll teardown race, with their later passing regressions.
Screens and decoded TUI failure/retry/restart history were reviewed. Browser/TUI
model replies remain scripted; no new live-model or cold-install claim is made.
At this checkpoint reader pages still traveled in status and queued behind active
semantic work; the reader publication section above replaces that path.
Generic inventory/job/log/evidence read models and layout/retirement work remain.

## Revisioned history (November 24)

The [implementation and measurement record](../results/revisioned-history-20261124.json)
removes original history from frequent status responses. The application publishes
complete immutable bytes by exact revision; the browser fetches them on revision
changes or explicit retry. It memoizes chat projection by that data object.
Superseded reads abort, mismatched envelopes fail, and original records remain
available after a publication failure. No authority or proof admission changed.

Twenty sequential raw-body probes per project, against the same selected files
before and after deployment, recorded:

| Measurement | Riemann before / after | EthLib before / after |
| --- | ---: | ---: |
| Status response bytes | 89,654 / 89,654 | 4,316,903 / 152,838 |
| Native status computation p95 | 8.199 / 8.241 ms | 113.779 / 13.466 ms |
| Node fetch/body-read p95 | 11.577 / 44.097 ms | 120.009 / 53.739 ms |

EthLib's status bytes fell **96.46%**. Original portable work, transcript and tool
records have identical decoded-content hashes across deployment. Riemann has
nearly empty history; its bytes and native computation did not improve. Its
Node transport result became slower, so native computation must not be reported
as end-to-end latency. The separate actual browser's settled state-response p95
was **10.802 ms for Riemann and 17.634 ms for EthLib**. These small within-run
percentiles are diagnostic, and the clients/load conditions differ. Node counts
decoded response bytes; Playwright's reported response-body size is retained
separately rather than treated as an identical byte boundary.

Two sequential actual Chrome captures, one per project, measured normal-motion
1440 × 960 interactions with CDP tracing/CPU sampling and matching source maps:

| Synchronous handler p95 | Riemann | EthLib |
| --- | ---: | ---: |
| Project-map wheel, 24 events | 3.644 ms | 5.767 ms |
| Dependency-graph wheel, 24 events | 0.853 ms | 0.730 ms |
| Hover pointer movement, 40 events | 0.105 ms | 0.083 ms |
| Drag pointer movement, 31 events | 0.335 ms | 0.333 ms |
| Resize, 7 / 2 delivered events | 1.119 ms | 1.057 ms |
| File/map tab click, 24 events | 15.583 ms | 11.919 ms |
| Alternating-node selection, 8 clicks | 15.667 ms | 9.284 ms |

The input/two-rAF scheduling proxy was 43.8 / 31.0 ms for tabs and 45.6 / 34.6 ms
for alternating node selection. It is not physical presentation or field INP;
eight-event p95 is the maximum. This is not a controlled before/after comparison
of rendering performance. Fit View and first/same-node selection remain separate
in the record. Hover, resize, tabs and all selected nodes retained the graph
camera and revealed their original source line. Each browser fetched history
once during setup, with zero history requests/materialization or commands during
gestures. Source maps match all seven deployed JavaScript bundles byte-for-byte.

History publication/hash work remains synchronous at record mutation boundaries.
Startup/host-refresh hashing took 90–188 ms per observed EthLib span; unchanged
revisions reuse their bytes. Model-turn latency was not profiled. Complete cached graph preparation
still took **19.065 / 5.926 seconds**, with cache hits and original exact-input
checks. Prepared page queries still queue behind semantic work, and sizeable
source/graph status fields still repeat. These remain separate migration work.

The three-megabyte regression retains complete notes and conversations, recovers
from a failed history read, rejects delayed older responses, preserves quiet
file/map navigation and survives restart with fresh approval. Its 2.2–2.6 KB
status fixture is smaller than actual EthLib's 153 KB response. Rust's 287 tests,
35 TypeScript tests, 22 architecture checks and required public TUI 34-case gate
passed. Ten distinct browser journeys passed across the retained reports; both
motion-mode numeric gates (`MEDEN_GUI_PERFORMANCE=1`) passed at 31.1 / 29.7 ms
against 250 ms. Earlier blank-screen
and outdated-test failures remain recorded separately. Screens and decoded TUI
failure/retry/restart history were inspected. No live inference was used.

## Independent source reader (November 24)

The [implementation record](../results/file-reader-20261124.json) separates
source display from semantic preparation. The shared application checks complete
bounded file bytes and publishes a typed reader page before compiler/search/
English preparation. Only the graph pane waits. Unreadable files report their
availability locally instead of failing the whole source inventory. Returning
to retained tabs preserves their graphs; paging does not request compilation.

One actual deployed observation per project, after preparing the new map cache,
showed the original first source line in **165 ms for Riemann** and **798 ms for
EthLib**. Both were still resolving Lean modules at that point. Complete graph
readiness remained **21.514 / 5.134 seconds**, respectively, with cache hits and
the original exact-input checks. The captures used Chrome 149, normal motion,
1440 × 960, no CPU profiler or inference. They are not controlled comparisons
against the earlier instrumented captures or guarantees for every file/device.
Chat DOM and message counts stayed unchanged; inference remained unapproved.

The real held-compiler journey displayed source in 679 ms, before compilation
could finish, against its 5-second scenario bound. Serialized retained-tab p95
was 29.8 ms with normal motion and 31.0 ms with reduced motion, below the 250 ms
scenario budget and without new inspections. The two-rAF proxy measures browser
scheduling, not physical presentation. Actual screenshots and decoded public
TUI failure/restart screens were reviewed.

The multi-megabyte history transfer below was still present at this source-reader
revision; the follow-up above removes it from status polling. The current host
worker still serializes new page-read commands behind active semantic work;
prepared reader queries and independent job read models remain migration work.
Source-reader separation improves initial readability without eliminating
compilation or all remaining waits.

## Current deployed event audit (November 24)

The [measurement record](../results/current-event-audit-20261124.json) covers
four sequential fresh-browser captures of the immutable `1f50834` deployments:
two Riemann runs, then two EthLib runs. Each used Chrome 149, normal motion,
1440 × 960 and no throttling. The executable digest and all seven served
JavaScript bundles were verified; source maps match those exact bundles.
No application update, server restart or model request was performed. Unfinished
generic-reader worktree changes are excluded. Raw traces, CPU samples and
screenshots remain in `target/native-profile/current-events-20261124-*`.

| Operation                          | Riemann dispatch p95, runs 1 / 2 | EthLib dispatch p95, runs 1 / 2 |
| ---------------------------------- | -------------------------------: | ------------------------------: |
| Project-map wheel, 24 events       |                 4.317 / 3.980 ms |                6.706 / 5.977 ms |
| Dependency-graph wheel, 24 events  |                 1.097 / 0.810 ms |                0.687 / 0.712 ms |
| Hover pointer movement, 40 events  |                 0.089 / 0.143 ms |                0.083 / 0.106 ms |
| Drag pointer movement, 31 events   |                 0.236 / 0.263 ms |                0.332 / 0.266 ms |
| Resize, 2–4 delivered events       |                 0.966 / 0.921 ms |                0.852 / 1.662 ms |
| File/map tab click, 24 events      |               15.816 / 12.487 ms |              12.978 / 15.070 ms |
| Different-node selection, 8 clicks |               16.841 / 15.666 ms |              11.833 / 13.668 ms |

The input/two-animation-frame proxy is 17.4–18.3 ms for project-map zoom,
16.5–22.7 ms for graph zoom, 34.0–38.4 ms for tab clicks and 14.8–25.9 ms for
different-node selection across these captures. It includes browser scheduling;
it is not physical presentation or field INP. These are small within-capture
percentiles: eight-event p95 is the maximum. Fit View and initial/same-node
selection are separate phases in the record. For example, Riemann's initial
same-node clicks cost 40.416 / 28.098 ms; their seven repeats cost 0.089–0.255 ms.
Those repeats must not substitute for measuring a change of declaration.

The long waits occur before these interactions:

| File opening                         | Browser wait | Observed preparation                                                                           |
| ------------------------------------ | -----------: | ---------------------------------------------------------------------------------------------- |
| Riemann, prepared cache, run 1       |     21.493 s | Lake build/setup/input resolution 10.666 s; complete-byte validation 4.581 s                   |
| Riemann, prepared cache, run 2       |     23.832 s | Lake build/setup/input resolution 11.853 s; complete-byte validation 5.035 s                   |
| EthLib, initial artifact preparation |     87.386 s | Layout discovery 38.594 s; compiler 0.881 s; search worker 17.668 s; English renderer 15.220 s |
| EthLib, prepared cache               |      4.970 s | Analysis controller 1.786 s; compilation, search worker and renderer are reused                |

Riemann still validates all **43,477 files / 5,992,114,707 bytes** on a prepared
open; no compiler, search worker or English renderer executes. EthLib's initial
layout discovery contains 19 Lake configuration evaluator executions totalling
35.585 seconds. Cached configuration avoids those executions on the second run.
These native intervals nest and overlap; they must not be added as exclusive
costs or subtracted from the browser wait to invent an unmeasured stage.
Installed dependencies remain present, so initial artifact preparation is not
cold installation. Different host/cache conditions also prevent interpreting the
higher Riemann waits as a controlled regression against earlier captures.

EthLib exposes a separate, persistent cost: idle polling repeatedly transfers
**4,320,881–4,334,772 bytes** per response, roughly every **493–501 ms**. Idle
request response p95 is **123.7–129.9 ms**; the native `gui.state` interval p95
is **113.0–114.5 ms**. Its nested serialization/response interval alone has
p95 **106.3 ms**, so queueing is not the principal cause. A separate read-only
payload observation attributes about **4.09 MB / 94.8%** of the decoded state
to the saved portable document. Field sizes use UTF-8 JSON re-serialization;
the captured response sizes remain authoritative for network comparisons.
Riemann, which has no saved conversation, transfers 89,696 bytes about every
361 ms, with response p95 around 12 ms.

The current host serializes the whole view for every `/api/state` request.
The browser polls 350 ms after completing the previous request and assigns a
new top-level React state. `NativeApp` also computes
`JSON.stringify(visibleHistory(state))` on every render; the conversation memo
does not avoid that traversal. Profiles sample history processing and the chat
`TextareaAutosize` during graph selection. Popper offset/scroll-parent traversal
appears during hover/selection, and XYFlow node remeasurement appears during tab
switching. Automation queries/hit tests contribute their own CPU samples and
must not all be attributed to the application.

The first Riemann post-preparation window also contains 187 / 398 ms long tasks,
with 233 ms of sampled self time in `projectMap` and additional Zod parsing.
The next settled idle window has no long tasks. That is a distinct late
preparation/layout burst, not a measured cost on every wheel event. Another
60 ms long task appears in each selection phase of that first capture; none
appears in the other captures' measured interaction windows. Hover, resize,
tab round trips and every selection preserve the graph camera, and no measured
interaction sends a command, compilation request or model request. The reported
camera snap-back was not reproduced by this workload.

The measurements set the next implementation priorities:

1. Split revisioned chat/history, file, map and job read models so unchanged
   multi-megabyte documents are not repeatedly serialized, transferred and
   traversed. Preserve cancellation/command generation fencing and fresh review.
2. Let bounded source reading display independently while richer compiler,
   search and English observations are prepared; keep their provenance and
   freshness explicit. Continue reducing exact-input preparation work without
   replacing complete-byte admission with a timestamp-only shortcut.
3. Narrow React updates to the affected graph/source pane and retain deterministic
   map layouts instead of rebuilding them on the main thread after a revision.

Those are measured target costs, not claimed savings from unimplemented changes.
The existing profiler records synchronous events, the two-rAF proxy, native
wall/current-thread CPU spans, idle transfer sizes and raw-artifact digests.
Zero-count CPU/event percentiles are `null` in the review record. No chat was
sent, so model response/streaming latency is outside this audit.

## Bounded parallel input validation (November 24)

The deployed worker now validates complete compiler inputs with four readers
and at most 256 queued jobs. It hashes every file's bytes, then combines those
digests in canonical path order with exact names, lengths, file counts and the
original compiler setup records. Version 2 has an explicit domain and is part of
the compiler resource identity; old fingerprints are not reused under a new
method. The previous serial aggregate loop is removed. No file/timestamp-only
shortcut replaces byte validation, and the actual Lake build/query still runs.

The [validation record](../results/parallel-input-validation-20261124.json)
compares two prepared-cache captures before and after deployment on the same
175,187-file Riemann workspace and 15-declaration file. Chrome 149 used normal
motion at 1440 × 960 without throttling. All seven served JavaScript bundles
remain byte-identical; only the native executable changed. Validation finished
before the repeated warm captures.

| Stage                                 | Before, runs 1 / 2 | After, runs 1 / 2 |
| ------------------------------------- | -----------------: | ----------------: |
| Explorer open to prepared view        |  15.764 / 15.792 s | 13.428 / 14.596 s |
| Prepared-input build/query/validation |  10.040 / 10.005 s |   7.810 / 8.420 s |
| Complete-byte validation              |    4.514 / 4.509 s |   2.428 / 2.536 s |

Both new captures read exactly **43,477 files / 5,992,114,707 bytes** and reuse
compilation, search and English readings. No compiler, search worker or renderer
executes on these warm opens. Lake resolution still takes 4.725 / 5.243 seconds.
Nested native intervals overlap and cannot be added. The observed complete-byte
stage is 44–46% shorter, while file opening improves by 1.2–2.3 seconds. OS cache
and scheduling conditions vary; these repeated diagnostics do not establish a
universal improvement or change the meaning of the two-rAF proxy.

A separate host-only comparison uses the same actual offline Lake setup and
all input bytes, excluding the Lake query/build. Three alternating repetitions
measure a median 8.720 seconds for the old implementation and 6.621 seconds for
the new one. The new implementation with one reader takes 9.997 seconds; one
and four readers produce the same v2 identity. This is separate from the actual
confined-worker and browser measurements. The new resource/fingerprint identity
also requires an initial cache fill, observed separately at 178.744 seconds;
installed dependencies remain present, so this is not a cold-install comparison.
The bounded initial search still takes about 121 seconds.

Twenty-three worker tests cover independent framing/content expectations,
forced out-of-order completion, multiple batches, empty/multi-chunk inputs,
timestamp-restored edits, observed mid-read changes, replacements, links and
limits. The actual 35-module Lake journey covers dependency edits, build failure,
retry and restart. All 34 required public TUI journeys and 276 Rust tests passed,
as did warning-free Clippy and architecture checks. Normal/reduced-motion
retained-tab p95 is 33.0 / 29.7 ms against the 250 ms scenario budget, with no new
inspection. Actual browser screenshots and decoded TUI failure/restart screens
were inspected. These checks concern adapters and named workloads; they add no
new refinement, inference-quality or confidentiality claim.

## Event baseline before parallel validation (November 24)

Two sequential captures of the preceding pinned Riemann GUI separate Fit View,
initial/repeated selection, and switching between two different declarations.
The [measurement record](../results/event-latency-20261124.json) contains
per-event distributions, native stages and raw-artifact digests. Chrome 149 used
normal motion at 1440 × 960 without throttling, with 175,187 indexed files and a
15-declaration compiled graph. All seven served JavaScript bundles were compared
byte-for-byte with the build supplying the source maps. The deployed executable
and assets were unchanged; no server restart or model request was needed.

| Operation                          | Dispatch p95, runs 1 / 2 | Input/two-rAF p95, runs 1 / 2 |
| ---------------------------------- | -----------------------: | ----------------------------: |
| Project-map wheel, 24 events       |         4.862 / 4.053 ms |                17.7 / 18.1 ms |
| Dependency-graph wheel, 24 events  |         0.996 / 0.853 ms |                16.7 / 22.2 ms |
| Hover pointer movement, 40 events  |         0.104 / 0.100 ms |                23.6 / 26.8 ms |
| Drag pointer movement, 31 events   |         0.213 / 0.240 ms |                23.3 / 21.8 ms |
| Resize, 3 delivered events         |         0.936 / 1.264 ms |                18.0 / 21.7 ms |
| File/map tab click, 24 events      |       15.468 / 15.528 ms |                42.1 / 43.1 ms |
| Different-node selection, 8 clicks |       14.812 / 16.749 ms |                22.7 / 27.7 ms |

Dispatch measures synchronous event handling; the two-animation-frame proxy
includes scheduling and is neither physical presentation nor field INP. These
are within-capture percentiles, with small sample counts: eight-event p95 is the
maximum. Selection alternates two reachable nodes, rather than covering every
declaration. The driver verifies the actual selected source line after each click
and records its target ID and camera. An explicit Fit View costs 1.041 / 1.059 ms.
The first same-node selection costs 32.022 / 28.170 ms; its seven repeats cost
0.082–0.218 ms. Treating those repeats as representative selections would hide
the cost of changing declarations. One initial selection window contains a
52 ms long task; none occurs in the other measured windows.

File opening remains the dominant wait: **15.764 / 15.792 seconds despite a
complete analysis-cache hit**. The nested prepared-input stage takes
**10.040 / 10.005 seconds**. Streamed logs divide that work into 4.780 / 4.874
seconds for Lake build/setup/input resolution and 4.514 / 4.509 seconds reading
all **43,477 files / 5,992,114,707 bytes**. Native CPU sampling attributes
24.66 / 24.03 percent to Python's SHA-256 block routine; these sampled CPU shares
are not wall-time shares. Compilation, search and rendering outputs are reused.

A separate initial cache fill took 229.335 seconds in the native controller.
Its actual counterexample worker took 121.410 seconds and exhausted its file
budget; its report preserves unavailable observations, rather than inventing a
verdict. Graph extraction took 19.996 seconds, initial input preparation
26.147 seconds, and two later fingerprint checks 14.748 and 10.695 seconds.
Layout/root discovery took 19.963 seconds and rendering 11.002 seconds. These
native intervals nest and overlap. This is a single setup observation with
installed dependencies, not a cold-install or steady-state measurement.

Browser profiles show Popper scroll/offset-parent traversal, React updates and
source explanation hints during hover/selection. Chat `TextareaAutosize` also
appears during graph selection, consistent with the top-level selection update
reaching more than the graph/source panes. Tab switches sample XYFlow node
remeasurement. Automation's selectors and hit tests are present too; do not
attribute their entire sampled cost to the application. Graph camera transforms
remain identical through hover, resize, retained-tab round trips and every
selection. No interaction phase requests compilation, inspection or inference.
This workload did not reproduce the reported camera snap-back.

Settled idle polling still transfers 89,695 bytes per response approximately
every 360 ms; native state projection p95 is 8.397 / 7.822 ms. The next measured
optimization targets are exact-input preparation on a cache hit, initial
background preparation/search, narrower React updates and revision-aware state
transfer. Retain exact input freshness and cancellation-generation fencing while
changing those paths. A profile identifies candidates; it does not establish
their eventual savings. Raw captures remain local under
`target/native-profile/event-latency-20261124-riemann-{2,3}`.

## Deployed event measurements (November 23)

Two fresh sequential captures of the current pinned Riemann GUI reproduce the
same prepared-file wait and separate first selection from repeat selection.
The [complete measurement record](../results/event-latency-final-20261123.json)
retains both distributions, native stages, sampled source locations and verified
digests for 30 raw artifacts. The executable and all seven served JavaScript
bundles match the immutable deployment; unfinished model-transport worktree
changes are excluded. No application update, restart or inference was needed.
Chrome 149 used normal motion, 1440 × 960, no throttling, 175,187 indexed files
and a prepared 15-declaration file. Validation ran after profiling.

| Operation                             | Dispatch p95, runs 1 / 2 | Input/two-rAF p95, runs 1 / 2 |
| ------------------------------------- | -----------------------: | ----------------------------: |
| Project-map wheel, 24 events          |         4.210 / 4.275 ms |                18.6 / 18.2 ms |
| Dependency-graph wheel, 24 events     |         0.777 / 0.952 ms |                16.6 / 23.3 ms |
| Hover mouse movement, 40 events       |         0.104 / 0.067 ms |                24.1 / 20.5 ms |
| Drag mouse movement, 31 events        |         0.193 / 0.281 ms |                20.3 / 21.5 ms |
| Resize, 3 / 4 delivered events        |         1.794 / 0.974 ms |                24.9 / 20.9 ms |
| File/map tab click, 24 events         |       15.463 / 14.175 ms |                40.0 / 36.5 ms |
| Fit View and node selection, 9 clicks |       34.114 / 29.549 ms |                71.9 / 59.7 ms |

Dispatch is synchronous browser event processing. The two-animation-frame
measurement also includes scheduling; it is neither physical presentation nor
field INP. Resize events are coalesced. There are no observed 50 ms long tasks
in these measured gesture or idle windows. This does not establish smoothness
for other workloads or exclude work outside those windows.

Chronological click traces resolve the selection tail more precisely: the
explicit Fit View costs 0.991 / 1.038 ms, the first node selection costs
34.114 / 29.549 ms, and the seven following selections of that same node cost
0.066–0.126 ms. Repeating a selected node is a different workload from changing
between declarations. Popper scroll-parent traversal, React rendering and
XYFlow node remeasurement appear in profiles. Automated selector/hit-testing
queries appear too; their sampled time must not all be attributed to the app.
Distinct-node selection remains a separate follow-up measurement.

Opening the file takes **15.730 / 15.714 seconds**, with a full analysis-cache
hit in both runs. The nested `atlas.project.prepare` stage takes
**10.214 / 9.941 seconds**; source-view materialization takes only
0.430 / 0.433 milliseconds. The last completed build log separates 4.844 seconds
of build/setup/input resolution from 4.499 seconds validating every byte in
43,477 files / 5,992,114,707 bytes. Python's SHA-256 routine accounts for
26.39 / 26.72 percent of sampled native host/worker self CPU, which is not a
wall-time fraction. These observations make exact-input preparation the first
wait-time optimization target. Layout discovery and cache admission also cost
time; nested spans overlap and cannot be added as exclusive costs.

Hover, resizing and retained-tab round trips preserve the exact graph transform.
Every gesture phase issues only state requests, with no new inspection, build
or inference. Idle polling still transfers 89,688 bytes roughly three times a
second, and native state projection p95 is 8.605 / 8.323 ms. Revision-aware
updates merit measurement, though these idle windows contain no long task.
The native ring evicts 539 / 541 older records during capture; the newly retained
record counts equal those deltas. Raw traces and profiles remain local under
`target/native-profile/event-latency-final-{1,2}`. These warm-cache diagnostic
captures are not cold-install or live-model validation.

The same pinned executable passed all 34 required public TUI journeys after
profiling. Decoded Lake failure/restart, meaning-history, provider-limit/retry
and GUI/TUI continuation screens, plus the captured browser screenshot, were
inspected. The gate's binary and summary digest are retained in the record.

## Batched Lake preparation (November 23)

Prepared analysis now requests the ordinary root build and the source, compiled
output and compiler-setup facets in one `lake --json query`. All selected roots
share Lake's build state. Separate build/setup-file/per-root query processes
were removed; exact setup and complete-byte input validation remain mandatory.
The worker rejects incomplete, extra, oversized and nonzero query output before
returning a fingerprint, streams actual compiler stderr, and reports resolution
and byte-validation durations separately.

The [measurement record](../results/lake-query-batching-20261123.json) compares
the same Riemann file, 175,187-file inventory and 15-declaration graph. Chrome 149
used normal motion at 1440 × 960 without throttling. Two warm post-update captures
both hit the complete analysis cache:

| Stage                          | Before, runs 1 / 2 | After, runs 1 / 2 |
| ------------------------------ | -----------------: | ----------------: |
| Explorer open to prepared view |  35.529 / 40.231 s | 15.605 / 15.790 s |
| Build/query/input validation   |  30.500 / 34.855 s | 10.066 / 10.300 s |
| Source-view materialization    |   0.640 / 0.385 ms |  0.413 / 0.424 ms |

The final cached log confirms one query and a full read of **43,477 files /
5,992,114,707 bytes**. Build/setup/input resolution took 5.135 seconds and byte
validation 4.524 seconds in that log. Twelve configuration observations were
reused in each warm capture; no configuration evaluator or fresh graph/search
extraction ran. Nested native stages overlap and cannot be added as exclusive
costs. Raw captures and artifact digests are retained locally.

These observations show a shorter prepare interval for this workload; they do
not establish a universal speedup or isolate every end-to-end difference to one
script edit. The native update also deploys compiler/renderer bindings already
on main but absent from the preceding executable. The asset update includes
the explicit-refresh wake-up fix described below. Exact source/build hashes are
recorded, and profiling begins after unrelated validation jobs finish.

The changed compiler/resource identities require a new observation artifact.
That **initial cache fill took 197.448 seconds**, including 134.974 seconds of
actual graph extraction and bounded counterexample observations, a second exact
fingerprint, rendering and persistence. It is reported separately from warm
cache validation and is not a cold installation. Parallel hashing was not
adopted: an independent read-only diagnostic on the same cached input inventory
measured serial per-file SHA-256 at 4.12–4.66 seconds and four threads at
1.78–1.80 seconds. Its aggregate format differs from production and is only a
diagnosis of hashing cost.

Faster background work exposed a real navigation failure: a directory request
could start and finish within one React batch, leaving the visible busy boolean
unchanged and never waking the pending file read. Command activity now retains
generation and pending count. The real 35-module browser journey requires one
requested-file inspection on every explicit refresh, including configuration
failure/recovery, dependency edits and restart. Cached logs require one query and
completed byte validation. Held-compiler tests cover streamed output, timeout,
cancel/retry and UTF-8 diagnostic truncation. Retained-tab budgets pass in both
motion modes. Fifteen worker-protocol fixtures, 32 TypeScript tests, all 262 Rust
tests, warning-free Clippy and all 34 required public TUI journeys passed; decoded
failure/retry/restart screens and actual browser screenshots were inspected.
The worker-mutation fixture now makes only its own temporary copy writable so
the original pinned executable can stay read-only. These checks add no new
refinement, inference-quality or confidentiality claim.

Warm opens still take about 16 seconds. Exact input reads, initial extraction
and independent search caching remain optimization/migration work.

## Indexed map navigation (November 23)

The deployed map now reuses its branch-pruned visible areas for declaration
legend and chunk-selection decisions. The legend appears only for rendered,
nonempty symbol sets. The center breadcrumb traverses containing branches and
derives directly from the current camera; its separate location state and
whole-inventory move-end scan are deleted. A canvas overlay keeps the breadcrumb
from resizing the viewport during normal-motion focus.

Two post-deployment Riemann captures use the same 175,187 files, 15-declaration
file, native executable digest, Chrome 149, 1440 × 960 viewport and normal motion
as the preceding two captures. The
[comparison record](../results/map-navigation-pruning-20261123.json) retains
raw-artifact digests, per-event distributions and unchanged-backend attribution.

| Operation                                  | Dispatch p95 before, runs 1 / 2 | Dispatch p95 after, runs 1 / 2 |
| ------------------------------------------ | ------------------------------: | -----------------------------: |
| Project-map wheel, 24 events               |               10.900 / 9.050 ms |               2.697 / 2.519 ms |
| File/map tab click, 24 events              |              16.800 / 23.225 ms |             10.803 / 19.297 ms |
| Node sequence including Fit View, 9 clicks |              34.064 / 29.695 ms |             35.499 / 30.836 ms |

The wheel handler reduction is about 72–75% in these captures. The input/two-rAF
proxy did **not** improve consistently: map-wheel p95 was 20.3 / 17.3 ms before
and 24.6 / 24.6 ms after. These are diagnostic event dispatch and scheduling
observations, not physical paint or field INP; do not claim a 75% end-to-end
rendering improvement. Tab click/two-rAF p95 was 32.9 / 42.9 ms after, versus
37.7 / 47.0 ms before. CPU samples no longer show the legend's whole-area scan
among the map-wheel hotspots.

Hover, resize and retained-tab round trips preserve the graph transform in both
captures; no interaction phase issues a non-state request. Neither settled
graph idle nor map idle contains a long task. The first post-restart graph-idle
window contains 174 / 434 ms tasks, with `projectMap` and Zod parsing sampled;
the second capture does not reproduce them. This supports a remaining initial
materialization cost, not a claim that every state poll rebuilds the map. One
tab phase contains an 86 ms task and one selection phase a 53 ms task. Selection
and initial materialization remain follow-up work.

Prepared-file opens took 35.529 / 40.231 seconds after deployment, with nested
build/input resolution and fingerprinting taking 30.500 / 34.855 seconds.
Both analysis caches hit, twelve configuration observations were reused and
no Lake configuration evaluator ran. This dominant native wait remains variable;
the frontend change does not optimize or explain its variation. Hosts restarted
for the new immutable assets, so the before/after cache/OS conditions are not a
controlled cold-start comparison. The actual native executable bytes did not
change. Riemann and EthLib now serve the same pinned frontend build.

The new browser regression caught a stale `File15.lean` breadcrumb while
`File0.lean` occupied the center. It now requires the correct settled breadcrumb,
visible legend and unchanged canvas height, along with the existing real
10,001-file declaration/selection, invalid-source, public-refresh and restart
journey. The actual screenshot and decoded TUI failure/retry/restart screens
were inspected. Three normal-motion browser journeys and the reduced-motion
retained-tab budget passed, as did 32 TypeScript tests, 3,000 generated
inventory/camera cases, schema/type/architecture checks and all 34 required TUI
journeys. The expanded algorithm budget covers visibility, legend and breadcrumb
at overview, category and fitted-file cameras; the full-scan mutation fails its
ratio assertion. These are presentation/runtime checks, not refinement,
live-model or cold-install evidence.

## Per-event and native-stage audit (November 23, evening)

Two sequential captures of the pinned Riemann GUI cover 175,187 indexed files
and the same prepared 15-declaration graph. Both opens hit the complete analysis
cache. The [measurement record](../results/event-stage-audit-20261123.json)
retains distributions, verified raw-artifact digests, native stages and sampled
source attribution. Chrome 149 used normal motion at 1440 × 960 without
throttling. No application runtime or deployment changed for this audit.

| Operation                                  | Dispatch p95, run 1 / 2 | Input/two-rAF p95, run 1 / 2 |
| ------------------------------------------ | ----------------------: | ---------------------------: |
| Project-map wheel, 24 events               |       10.900 / 9.050 ms |               20.3 / 17.3 ms |
| Dependency-graph wheel, 24 events          |        0.904 / 1.051 ms |               25.8 / 17.8 ms |
| Hover mouse movement, 40 events            |        0.080 / 0.100 ms |               25.9 / 24.1 ms |
| Drag mouse movement, 31 events             |        0.312 / 0.279 ms |               21.2 / 21.6 ms |
| Resize, 5 / 3 delivered events             |        1.867 / 1.918 ms |               23.1 / 20.3 ms |
| File/map tab click, 24 events              |      16.800 / 23.225 ms |               37.7 / 47.0 ms |
| Node sequence including Fit View, 9 clicks |      34.064 / 29.695 ms |               74.2 / 37.4 ms |

The proxy now separates event types: hover/drag use pointermove observations
for the corresponding mouse movements, and resize uses window resize. It is
not physical presentation or field INP. Node selection includes one explicit
Fit View and eight selections of a reachable node; it is not a distribution
over arbitrary declarations. Its first run contained one 52 ms long task.
Neither initial/settled graph idle nor map idle contained a long task. The
hover, resize and tab-return phases preserve the retained graph transform in
both runs. These checks do not rule out a reset during other gestures or layouts.
Every interaction phase issued only state polls, with no new inspection,
preparation or inference request. Settled state responses contained 89,419 bytes.

The profiler now selects source maps explicitly with `MEDEN_GUI_ASSETS`.
All seven generated JavaScript bundles were compared byte-for-byte with the
immutable deployment before using a separate hidden-source-map build. This
avoids attributing a running bundle through a newer mutable build. Map digests
are retained; the deployed assets were not replaced.

The main remaining costs are distinct:

1. **Opening a prepared file still validates its compiled inputs.** The two
   opens took 29.040 / 24.897 seconds. The nested actual build/input-resolution
   and fingerprint stage took 23.847 / 19.979 seconds, including validation of
   43,477 compiled inputs. Layout discovery took 1.846 / 1.807 seconds; twelve
   Lake configuration observations were reused with zero evaluator executions.
   Exact cache-validation spans totaled 1.211 / 1.161 seconds. Source-view
   materialization took 0.412 / 0.409 milliseconds. These overlapping stages
   cannot be added as exclusive costs. Reusing precisely bound preparation
   observations with explicit invalidation is the first optimization target.
2. **Map legend and breadcrumb decisions bypass visibility pruning.** Wheel
   CPU samples resolve to `NativeWorkspaceGraph.tsx:468`,
   `project-map-model.ts:252/256` and `WorkspaceMap`. The legend's
   `scene.areas.some` runs on camera updates, and `onMoveEnd` separately walks
   all areas for the breadcrumb. Tab reactivation also samples this work.
   Both decisions should use the existing indexed visible-area query.
3. **Selection and tab activation have a rendering tail.** DOM queries,
   Popper scroll-parent traversal and React work appear in the selection
   samples. This identifies a narrower follow-up around selection/tooltips;
   it does not assign all click latency to one function. Measure that path
   independently after removing map-wide scans.

Native stage summaries exclude span IDs present before the capture. There was
no eviction in either run. Host-thread CPU excludes workers, and nested spans
overlap. Raw captures remain local under
`target/native-profile/event-audit-riemann-{2,3}`. These are controlled desktop
observations using warm project/OS caches, not cold-install measurements or
confidence intervals.

The same measured native executable passed all 34 required public TUI journeys.
Decoded Lake inventory-refusal/history, provider-limit/retry and GUI/TUI
continuation/fresh-review screens were inspected, along with the actual map
screenshot. All 30 TypeScript tests, architecture boundaries, script syntax and
format checks passed. This audit adds no live-model or cold-install claim.

## Frozen Lake configuration reuse (November 23)

The host now retains deterministic configuration evaluation separately from
compiler/search observations. Keys bind the complete frozen input map and
absences, command/options, validated Lean/library manifest and implementation.
Reads check the full identity and output hash and use the same result parser.
Failed evaluations are not saved; the 4096 replacement slots retain outputs up
to 1 MiB. A collision or larger output requires fresh evaluation. The actual
Lake build and compiled-input fingerprint remain mandatory on a reopened file.

Three sequential Riemann captures use the same native executable, 175,187-file
inventory and 15-declaration file. All three hit the existing complete analysis
cache. The [measurement record](../results/lake-configuration-cache-20261123.json)
retains native/browser artifacts, checked digests and validation evidence.

| Stage                                     | Initial configuration cache fill | Repeated open 1 | Repeated open 2 |
| ----------------------------------------- | -------------------------------: | --------------: | --------------: |
| File request to prepared view             |                         59.748 s |        35.658 s |        24.817 s |
| Actual configuration evaluations          |                               12 |               0 |               0 |
| Reused configuration observations         |                                0 |              12 |              12 |
| Layout discovery                          |                         20.783 s |         2.042 s |         1.933 s |
| Build/input resolution and fingerprinting |                         35.046 s |        29.896 s |        19.277 s |

These are nested diagnostic spans, not additive exclusive costs. Project/OS
caches also warm, so the entire elapsed-time change cannot be attributed to
configuration reuse. The initial capture used the preceding navigation asset
build; repeated captures used the final typed navigation build. Asset digests
are retained. No graph-engine improvement or cold-install result is claimed.
The remaining build/input validation is still the largest measured wait.

The public GUI regression checks real configuration failure on two attempts,
dependency edits, compiler rejection/retry and restart. A delayed-status fixture
also reproduces an older retained-view map after refresh: the next requested
file must still dispatch. Navigation distinguishes an explicit read, a retained
tab and an already prepared/requested graph, avoiding both a lost refresh and
a second inspection after explicit preparation. Existing tab returns retain
source/graph/camera without inspections. The three real-Lean browser journeys,
34 required TUI journeys, 45 application unit tests, two doc tests and 30 TypeScript
tests passed; decoded failure/restart screens and the browser screenshot
were inspected. These checks add no new proof authority or live-model claim.

## Production recheck (November 23)

A third capture of the same pinned Riemann host confirms that preparation,
map navigation and selection have separate costs. It uses a fresh browser
page, the existing project/compiler caches and normal motion. The
[recheck record](../results/event-measurements-recheck-20261123.json) retains
the distributions, latest native trace, environment and verified artifact
digests. Application code, project sources and the running services were not
changed for this measurement.

| Operation                         | Events | Handler p95 | Phase input/two-rAF p95 |
| --------------------------------- | -----: | ----------: | ----------------------: |
| Project-map wheel                 |     24 |   10.721 ms |                 26.7 ms |
| Dependency-graph wheel            |     24 |    0.962 ms |                 17.1 ms |
| Hover mouse movement              |     40 |    0.116 ms |                 29.4 ms |
| Drag mouse movement               |     31 |    0.400 ms |                 37.1 ms |
| Resize                            |      6 |    1.894 ms |        Not instrumented |
| File/map tab click                |     24 |   24.676 ms |                 55.5 ms |
| Node sequence, including Fit View |      9 |   29.863 ms |                 76.6 ms |

The phase proxy combines instrumented input types, so it is not the latency
of each handler or a physical paint measurement. The drag proxy contains only
two instrumented samples. Node clicks repeatedly select a reachable node and
include explicit Fit View; the sequence contained one 50 ms long task.
None of the gesture or tab phases issued non-state API requests, and none of
the initial/settled graph or map idle windows contained a long task.
State polling still transfers 89,687 bytes per response in this capture;
response-end p95 was 25.454 ms in settled graph idle and 14.688 ms in map idle.
That is recurring network/serialization work, but these idle windows do not
show a corresponding repeated long rendering task.

The prepared-cache open took **68.762 seconds**, compared with 40.04 / 41.20
seconds in the preceding captures. Native extraction took 67.529 seconds:

- Layout discovery: 26.013 seconds, including twelve Lake configuration calls
  totaling 24.013 seconds.
- Build/input resolution and fingerprinting: 38.746 seconds. The visible build
  log reported validation of 43,477 compiled inputs.
- Exact cache-validation spans: 1.269 seconds.
- Source-view materialization: 0.427 milliseconds.

These stages overlap and must not be added as exclusive costs. Native sampling
collected 4,106 samples, with Python/OpenSSL SHA-256 and actual Lake execution
visible. This confirms expensive revalidation before cached display; it does
not assign all preparation time to hashing. Timing variation on the same
binary does not establish a code regression or identify its cause.

The next optimization order remains: reuse evaluated Lake configuration and
prepared input observations with explicit invalidation, remove full-inventory
legend/breadcrumb scans during navigation, then reduce tab/selection layout
work. Cache reuse must preserve the original source/configuration/resource
bindings. Raw captures remain local under
`target/native-profile/riemann-events-recheck`; repeat with the commands below.
The required TUI gate passed all 34 journeys on the current main validation
executable, with decoded Lake-failure/restart, provider-stop/retry and fresh
processor-review screens inspected. This is a separate executable from the
pinned production host measured above; neither gate establishes cold-install
or live-model performance.

## Current event audit (November 23)

Two sequential captures of the pinned running Riemann GUI cover 175,187 files
and the prepared 15-declaration `DictGen.lean` graph. Chrome 149 used normal
motion at 1440 × 960, without throttling. Both opens reported a persisted-cache
hit. The [complete event record](../results/event-measurements-20261123.json)
retains event counts, distributions, rendering costs, native spans and raw
artifact digests. No application runtime or user source was changed for this
audit; the profiler now separates initial and settled idle windows.

| Operation                                             | Handler p95, capture 1 | Handler p95, capture 2 |
| ----------------------------------------------------- | ---------------------: | ---------------------: |
| Project-map wheel, 24 events                          |              10.324 ms |              10.857 ms |
| Dependency-graph wheel, 24 events                     |               1.000 ms |               1.048 ms |
| Hover mouse movement, 40 events                       |               0.108 ms |               0.103 ms |
| Drag mouse movement, 31 events                        |               0.253 ms |               0.276 ms |
| Resize, 4 / 3 delivered events                        |               0.942 ms |               0.960 ms |
| File/map tab click, 24 events                         |              21.808 ms |              20.696 ms |
| Node selection, including explicit Fit View, 9 clicks |              33.120 ms |              32.079 ms |

Handler time excludes subsequent rendering. The tab phase's input/two-rAF p95
was 44.5 / 46.3 ms; the node-selection phase's was 77.4 / 63.8 ms. Those phase
proxies combine instrumented pointer/click events and are not physical paint or
field INP. The node sequence repeatedly selects a reachable node and includes
one explicit Fit View; it is not a percentile across arbitrary declarations.
One selection phase contained a 53 ms long task. Resize events were coalesced.
No measured gesture or tab phase issued a non-state API request.

The largest delay remains opening a prepared file: **40.04 / 41.20 seconds**.
Within those intervals, twelve Lake configuration executions totaled
17.23 / 17.12 seconds, and prepared build/input resolution and fingerprinting
took 17.87 / 19.03 seconds. Exact cache-validation spans totaled
1.14 / 1.20 seconds; source-view materialization took 0.418 / 0.397 ms.
These are nested native stages and must not be added as exclusive costs.
The prepare interval includes real Lake commands and sandbox work as well as
hashing; it cannot all be attributed to file hashing. Native CPU sampling
confirmed 2,922 / 2,956 samples, with hashing in Python/OpenSSL and actual Lake
execution visible. Some Lean-library addresses remain unresolved.

The map-wheel CPU call tree identifies another concrete cost:
`WorkspaceMap` calls `declarationDetail` through its full `scene.areas.some`
scan to decide whether to show the legend. That decision runs during camera
updates even though viewport visibility already has a branch-pruning index.
The declaration-detail function alone accounted for 150.8 ms of sampled self
time across the first wheel sequence. The breadcrumb's `onMoveEnd` also scans
all areas; it should reuse an indexed viewport query, with its own regression.

Optimize in this order:

1. Reuse evaluated Lake configuration and prepared immutable input observations,
   with explicit source/configuration/resource invalidation. Preserve exact
   freshness checks; a cache hit must remain bound to its original inputs.
2. Derive legend and breadcrumb decisions from indexed visible areas, avoiding
   whole-inventory work during navigation.
3. Reduce retained-tab reactivation and source/popover layout work, measuring
   the rendering tail separately from the click handler.

Neither capture reproduced the historical roughly 384 ms map materialization
in its initial idle window. The additional settled graph and map idle windows
contained no long tasks. A new inventory/revision can still require expensive
layout; these observations do **not** establish that every 350 ms state poll
rebuilds it. Initial overview arrival and revision changes need their own
capture before making that attribution.

Raw recordings are local under
`target/native-profile/riemann-events-{current,settled}`. Use the existing
reproduction commands below; the second capture adds `graph-idle-settled` and
`project-map-idle` phases. These are diagnostic observations on one desktop,
not confidence intervals, cold-install results or live-model measurements.

The November 22, 2026 audit uses the actual Riemann workspace: 175,187 indexed
files and the compiled 15-declaration `DictGen.lean` graph. Chrome 149 ran at
1440 × 960 without CPU or network throttling, with reduced motion enabled.
The [measurement record](../results/gui-interactions-20261122.json) retains the
event distributions, sampled CPU attribution and intermediate changes.

## Observed event costs

These are renderer `EventDispatch` p95 durations, in milliseconds, from one
controlled interaction sequence per revision. They are not field INP or a
hardware-independent guarantee.

| Operation                                             | Before |  Final |
| ----------------------------------------------------- | -----: | -----: |
| Project-map wheel, 24 events                          | 42.809 | 10.554 |
| File/map tab click, 24 events                         | 47.509 | 25.809 |
| Dependency-graph wheel, 24 events                     |  0.762 |  1.031 |
| Hover mouse movement, 40 events                       |  0.073 |  0.104 |
| Graph drag mouse movement, 31 events                  |  0.286 |  0.415 |
| Resize, 6–8 delivered events                          |  1.133 |  1.461 |
| Node-selection sequence, including one Fit View click | 66.353 | 40.223 |

Small differences in the already cheap pointer/graph-wheel handlers are not
evidence of an improvement. The expensive work was the map traversal and work
triggered after selection. In the node-selection phase, the longest style
recalculation fell from 240.611 ms to 9.313 ms. The initial run recorded a
248 ms long task there; the final action window recorded none above 50 ms.

The input-to-two-animation-frames proxy fell from 66.5 to 28.9 ms p95 for map
zoom and from 93.4 to 60.1 ms for tab switching. This proxy includes browser
scheduling, not actual physical presentation. Wheel-end/debounce callbacks can
finish outside the action window. Raw traces retain the surrounding work;
absence of a long task in one window does not establish universal smoothness.

## Causes and changes

- **Automatic fitting reclaimed the camera.** A second fit controller observed
  every canvas resize. A regression reproduced a zoom of 0.168469 falling to
  0.0948569 after an incidental resize. Removing that controller leaves initial
  fitting and the explicit Fit View control. Hover and resizing preserve the
  user's transform. The passive review legend also intercepted pointer events
  over nodes and wheel gestures; it now passes them through.
- **Visibility calculation scanned all 175,187 files on every move.** The map
  now builds a parent/children index with its stable coordinates and skips
  branches outside the viewport or below the opacity cutoff. The visibility
  result is checked against an independent full-scan oracle across 24 views.
  The minimap uses the root index too. Hidden maps retain their camera without
  processing parent-driven status updates; activation receives current data.
  Previously, `visibleMapAreas` consumed about 114 ms of sampled self time even
  during a three-second idle period on the file tab. It no longer appears among
  that period's leading CPU samples after the change.
- **Selecting a loaded line rebuilt the source page.** The reader moved its
  200-line page to `selectedLine - 5` on each selection, even for a line already
  loaded. That replaced tokens/DOM and forced style work during Popper's position
  reads. The page now remains intact for selections within it, while the reader
  scrolls/highlights the target. Out-of-page selection still loads its page.
- **Reduced motion created unintended transitions.** Giving every element a
  nonzero transition duration activated default `transition-property: all` on
  changing SVG coordinates and font sizes. The initial map-wheel sequence
  generated 1,813 transition starts; the final sequence generated zero. A zero
  transition duration removes this work. Normal-motion hover remains tested.

There is still work to investigate. Map legend and breadcrumb decisions retain
large-array checks, and tab reactivation includes real layout/paint work.
State polling transfers roughly 178 KB per response at the existing interval;
the dominant sampled cost was the map work it triggered. Initial prepared-cache
validation remained about 53–56 seconds and is outside the warm interaction
comparison. This audit does not establish its Lean/Lake or sandbox-preparation
CPU breakdown, and does not claim first-open compilation is instant.

## Reproduce and inspect

Use installed development dependencies and an already prepared file in a running
native GUI. The explorer open validates that file's cache before timing begins.
The profiler does not send chat, request inference or edit project sources.

```sh
npm run typecheck
npm run build --workspace @meden/web -- --sourcemap
node tools/gui/profile-interactions.mjs '<GUI_URL>' target/gui-profile/review
```

`MEDEN_GUI_CHROME` selects another installed Chrome executable. The output
contains `summary.json`, a Chrome `trace.json`, and one `.cpuprofile` per action.
CPU attribution resolves the built bundles' local source maps when present.
Session fragments are redacted from exported traces. Profiling runs locally and
adds no always-on instrumentation or dependency to the production renderer.

The browser MCP was unavailable; the tool uses installed Playwright and Chrome's
[Tracing](https://chromedevtools.github.io/devtools-protocol/tot/Tracing/) and
[Profiler](https://chromedevtools.github.io/devtools-protocol/tot/Profiler/)
protocols. The first capture includes profiler boundaries in frame intervals;
later captures exclude them. Frame maxima are therefore not used for a before/
after claim. Raw profiles and traces for this audit live in
`target/gui-profile/riemann-{baseline,after,final,motion-fix}`.

Required public TUI journeys and decoded failure/restart screens are recorded
with the same native binary. Browser regressions exercise pointer movement,
wheel routing through the legend, resizing, explicit fit, a 10,001-file map,
retained views and selection without source-page replacement. Compiler/model
fixtures do not establish cold installation, live-model quality or renderer
refinement.

## Native preparation and routine gates (November 23)

`tools/perf/perf.sh` provides a pinned host-only perf tool. Same-user user-space
CPU sampling succeeded with paranoid=2 and kptr_restrict=1, without changing
kernel permissions. The production host now retains 4096 completed local spans
and an eviction count, with static stage names, numeric correlation and parent
IDs, monotonic offsets, wall time and current-thread CPU time. The authenticated
`/api/metrics` query has the same session/loopback checks as state. There is no
source/prompt/credential label or default remote export. Native samples include
inherited worker CPU; host thread CPU does not. Wall minus thread CPU includes
child execution and scheduling, not just I/O wait. Nested spans overlap.

The same actual Riemann file/cache-validation journey took 64.98 s in the initial
unoptimized development host, 46.40 s with SHA-256 dependency optimization, and
39.33 s in a subsequent normal-motion capture. These are diagnostic observations,
not repeated before/after confidence intervals: OS caches and concurrent test
load can differ. The first capture collected 4765 native samples. Initial capture
finalization exposed two tool bugs (SIGINT exit 130 and a not-yet-open report
stream); both were fixed, the retained recordings decoded independently, and the
final attach run completed with 2899 confirmed samples and an artifact manifest.
Some installed Lean shared-library addresses have no usable symbol names; the
profiles do not establish function attribution for those addresses.

The first instrumented file trace exposes the remaining work:

| Stage                                 | Wall time | Host thread CPU | Interpretation                                      |
| ------------------------------------- | --------: | --------------: | --------------------------------------------------- |
| Analysis extraction                   |  45.775 s |         1.521 s | Overall preparation interval                        |
| Select roots/discover Lake layout     |  20.441 s |         1.181 s | Includes 12 Lake configuration executions           |
| Lake configuration executions         |  18.575 s |         0.484 s | Includes their process/sandbox intervals            |
| Prepared Lake build/input fingerprint |  23.391 s |         0.159 s | Worker checks 43477 compiled input files            |
| Cached observation validation         |   0.988 s |         0.066 s | Exact resource, metadata and selected-source checks |

These rows are nested, so their durations cannot be added. Resource selection
and validation take 0.870 s across this host snapshot; 14 sandbox preparations
total 7.136 s, including startup and nested compiler work. This identifies repeated
Lake configuration and the compiled-input fingerprint as the next application
optimization targets. No hash/freshness check was removed: only `sha2` is optimized
in development builds. The separate profiling binary retains debug symbols and
avoids replacing a user's running executable.

CI now runs a 3000-case shrinkable differential inventory/camera corpus and nine
paired visibility measurements on 175187 generated files. Optimized median cost
must stay below 35% of an independent full scan in the same runner/process; a
full-scan negative mutation must fail that budget. Absolute layout timings remain
diagnostic. The native retained-tab journey records first preparation from an
empty analysis store, then ten actual tab-click/two-rAF samples. Serialized runs
in reduced/normal motion require warm p95 below 250 ms, no new inspection or
preparation and stable retained graph/source/camera. This is a scenario target,
not physical paint/INP, cold installation or model-quality evidence. Separate
output directories preserve both motion-mode reports. The diagnostic CDP profiler
also includes a 600 ms gesture tail plus eight frames, with profiler boundaries
excluded consistently.

See [tools/perf/README.md](../tools/perf/README.md) for reproduction and seed/path
replay, and [the native result record](../results/native-profiling-20261123.json)
for raw-artifact references and completed checks. Shared CI hardware is not a
fixed physical CPU: the algorithm gate uses paired relative timings and records
its environment; the browser budget uses an explicit serialized workload and a
broad scenario deadline. Neither is a universal performance claim.

## Source projection follow-up (November 23)

The native client now receives an application-owned prepared source/graph page,
instead of raw Atlas/package records and a second browser lexical index. Original
backend inputs remain retained for explicit analysis; the projection carries no
accepted-contract proof receipt. Its `analysis.materialize_view` span took
1.767 ms on the actual 15-declaration Riemann page. A separate 4,000-declaration
lexical regression completed in 89.6 ms in an unoptimized development test;
that single diagnostic is not a performance budget.

The updated production binary and browser assets were pinned together. Two real
native/browser captures distinguish the first open after an adapter update from
a new host session reusing the resulting persisted cache:

| Operation                                               | Observation |
| ------------------------------------------------------- | ----------: |
| First preparation after renderer cache identity changed |    229.97 s |
| Preparation with a confirmed persisted-cache hit        |     40.09 s |
| Project-map wheel handler p95                           |   10.533 ms |
| Dependency-graph wheel handler p95                      |    1.008 ms |
| Hover pointer movement p95                              |    0.065 ms |
| Drag pointer movement p95                               |    0.169 ms |
| Resize handler p95                                      |    1.316 ms |
| Tab click handler p95                                   |   19.267 ms |
| Node-selection click handler p95                        |   28.274 ms |

These are single-run diagnostic observations on the same 175,187-file project,
Chrome 149 at 1440 × 960, normal motion, without throttling. Handler durations
exclude later rendering. The warm trace's input/two-rAF proxies were 16.9 ms for
graph wheel, 39.2 ms for tabs and 58.0 ms for node selection. The separate final
serialized native regression recorded warm-tab p95 of 29.2 ms against its 250 ms
scenario budget. These are different controlled sequences, not interchangeable
field percentiles.

The retained artifacts explain the unusually slow first open: source revision,
compiler resources and prepared input fingerprint were unchanged, while the
renderer binding differed. The renderer hashes its adapter source, which changed
in the preceding application extraction. The combined cache consequently reran
graph extraction and counterexample observations before rendering. Search has a
120-second file wall budget, with at most ten seconds per declaration; a timeout
is explicitly inconclusive. Compiler/search artifacts should be cached separately
from renderer output so a renderer update does not repeat unrelated search.

Even a valid persisted cache still spends 19.059 s selecting roots/layout,
including twelve Lake configuration calls totaling 16.939 s, followed by
17.105 s preparing/fingerprinting compiled inputs. These intervals are nested.
The exact input and sandbox checks remain in place. Deduplicating configuration
preparation is still necessary; source-view materialization is not the bottleneck.

The trace also retains a remaining UI pause: the initial idle/poll phase recorded
two long tasks, the largest 394 ms. Its sampled project-map layout function used
230.9 ms of self time. The production minified function body matches `projectMap`
in `project-map-model.ts`; this follow-up did not include browser source maps.
This is whole-inventory layout work outside a wheel handler and must not be hidden
by reporting cheap pointer events. Layout materialization/reuse is still needed.
State polling transferred about 89.7 KB per response in this capture.

See [the source-projection record](../results/source-projection-20261123.json)
for binary/artifact digests, both traces, residual costs and completed checks.
The first raw report retains the old `setup_cache_validation_ms` name despite
its cache miss; the profiler now emits `setup_preparation_ms` and an explicit
`setup_cache_hit` value to prevent that ambiguity.

## Independent compiler and renderer caches

The combined cache identified above has been replaced. Compiler ports return raw
compiler/search observations; the application retains them before selecting the
renderer. The raw artifact includes original source text, module/configuration
observations, any prepared Lake input fingerprint, compiler resources, structural
English and search outcomes. Existing exact source, metadata and resource checks
still run before reuse. Lake still prepares and fingerprints its compiled inputs;
this change does not remove its roughly forty-second validation cost.

Renderer readings are a separate artifact keyed by the immutable raw artifact
and selected renderer fingerprint. They contain only declaration IDs, sentences,
renderer attribution and rendering errors. Publication refuses missing, duplicate
or foreign declarations and changes to any compiler/search field. The completed
graph revision also binds the reading artifact, so returning a new reading for
the same source cannot leave an old browser memoized view. A failed renderer
starts from retained structural English, rather than attributing a previous
renderer's sentence to a different implementation.

Compiler/search data are persisted before rendering, and the original is held in
an immutable consuming application value. A rendering interruption therefore
does not require repeating successful compiler/search work. Cancellation gates
raw retention and final rendering-cache admission. These cache observations neither
accept a contract nor carry an admitted verification receipt. Filesystem and
physical stop correspondence remain adapter obligations.

The new cache is under host-owned `compiled-views`, with separate bounded indices
and checksummed objects for observations and renderings. The obsolete combined
cache loader was removed. An old combined artifact cannot reconstruct its lost
structural reading, so first use prepares the new raw artifact once; no legacy
compatibility path is retained.

The public renderer-update regression copies actual prepared Informath resources,
changes a host resource with a matching manifest, and opens the same source after
restart. It checks preserved counterexamples and formal inputs, a different view
revision, renderer execution, and no compiler/search scripts in authenticated
native spans. Another restart reuses the complete view without inference approval.
An actual malformed source then fails, and the public retry succeeds after a
source correction. The attached `renderer-cache-preparation` timings are
diagnostic installed-tool measurements, not cold installation, live inference or
physical presentation. Unit checks also reject changes to checker results,
source, refutation and expressions, corrupted/misbound overlays and partial or
unbounded readings. A compile-fail example checks that the original observation
cannot be mutated through its public accessor.

On the actual one-theorem installed-Lean/GF fixture, the final run recorded
30.43 s for initial preparation, 2.30 s after the renderer resource update, and
1.77 s for a complete-cache reopen. These are single diagnostic file-request to
prepared-DOM intervals, with concurrent functional validation and no hardware
budget claim. The no-recompilation claim comes from the independent native spans
and preserved observations, not a timing threshold. They do not predict Riemann's
larger Lake input-validation cost. See
[the cache split record](../results/renderer-cache-20261123.json).

## Remaining prepared-file wait (November 25)

The observation-identity change preserves every compiler-captured source and all
88 browser assets. Its final actual-project probe still exposes a large native
wait, despite complete semantic-artifact cache hits:

| Current fidelity probe | File click to prepared-source DOM | Native job wall time |
| ---------------------- | --------------------------------: | -------------------: |
| Riemann                |                          20.234 s |             19.931 s |
| EthLib                 |                           2.225 s |              1.669 s |

Riemann's correlated `atlas.project.prepare` span lasts 15.854 s, including a
15.350 s worker invocation. That worker's original build log reports 10.961 s
for build/setup/input resolution and 4.278 s to validate **43,477 files /
5,992,114,707 bytes**. The remaining host spans include layout discovery and
view materialization. These nested intervals must not be added as exclusive
costs. Host-thread CPU measurements exclude child-process CPU, so they do not
establish that the worker is I/O-bound.

This is one unpaired cache-hit observation per project, collected after the
public TUI/GUI gates and deployment. It is neither a controlled comparison with
the preceding binary nor a cold-install, physical-paint or inference benchmark.
It identifies input preparation as a remaining bottleneck; the nominal job
types claim no speedup and do not weaken complete-input validation. Exact
correlated spans, original log timings, cache outcomes, source/graph equality
and their limits are retained in
[the validation record](../results/job-observation-identities-20261125.json).

## Bound repeated input-path observations (November 25)

The Linux preparation worker now checks an opened object's actual procfs name
and regular-file type instead of repeatedly walking every ancestor in Python.
The original full-content hashing, metadata change checks, root/file/setup
binding, bounds and ordered fingerprint remain intact. Eight interleaved actual
Lake invocations over the same 43,477 files / 5,992,114,707 bytes reduce the
complete preparation median from 7.314 s to 6.313 s (13.7%). Every fingerprint,
file count and byte count matches. Two separate resource/input warmups are
excluded; no concurrent builds/tests or native sampler ran during comparison.
This measures preparation, not browser latency or physical paint.

The source script's resource attribution changes correctly. Actual-project
semantic preparations consequently run once under the new compiler/search
identities (181.582 s Riemann / 65.349 s EthLib). After host restart, cached
file opening takes 15.316 s / 2.235 s in single unpaired functional probes.
Complete source/graph data matches the original except the graph binding
revision; checksummed compiler/search/rendering chains and raw graphs also
match their expected new attribution and remain identical across restart.
No graph-extraction/search/render worker runs during the cache-hit inspections.

The same experiment exposes remaining resource coupling: the frozen compiler
and searcher also invalidate when the prepared-input helper changes, although
their logical output is unchanged. Partitioning executed resources and sharing
appropriately owned prepared snapshots still need work. This optimization
keeps the exact-input validation and makes no whole-workflow speedup claim. See
[the implementation and regression scope](performance/lake-path-observations.md)
and [the complete record](../results/lake-path-observations-20261125.json).

## Separate Lake execution from input validation (November 25)

The metadata helper is now absent from the analysis environment visible to
project code. A successful actual Lake query precedes a separate read-only
validation worker. The static native spans `atlas.project.query` and
`atlas.project.fingerprint` distinguish these costs. They replace the former
combined preparation span without weakening full-input validation.

A native helper-only update/restart regression preserves the complete source
view, counterexample and every original retained compiler/search/render object.
It revalidates inputs but starts none of those semantic workers. The resource
inventory refuses unbound additions, rather than silently omitting readable
files from the cache identity. This removes unnecessary semantic recomputation
when updating only the helper; it is not a claim that the extra isolated worker
makes an unchanged-input query faster. See
[the implementation and validation scope](performance/compute-metadata-isolation.md).

The [November 25 actual-event report](performance/event-observations-20261125.md)
separates ordinary dispatch, first-detail drawing stalls and prepared-file wait.
Ordinary file-graph wheel/hover/pan remains below 1.2 ms dispatch p95 in these
single-project runs. Riemann's first-detail gesture still includes a 115.985 ms
wheel dispatch and four long tasks. Byte-matched source maps and original trace
intervals attribute that stall to visibility/appearance work, SVG mutations and
main-thread Canvas painting; phase-wide reference resolution is recorded
separately. These observations are a remaining optimization target, not a claim
that all zoom paths or physical presentation now meet a latency budget.

## Share map visibility and omit inert targets (November 25)

The [interaction follow-up](performance/map-interaction.md) shares exact viewport
results between SVG, Canvas and prefetch and omits non-interactive faded SVG
targets, retaining the original Canvas paint and current keyboard focus. One
sequential actual-Riemann comparison observes first-detail long tasks fall from
four / 452 ms to two / 219 ms; worst wheel dispatch remains 111.893 ms versus
113.852 ms. This is not a stable speedup estimate or field percentile. All 13
phase cameras, drill actions and declaration counts agree. The same native
binary and exact original source/graph/compiler/search/render objects remain
after deployment and restart in both projects. Native input validation and
main-thread DOM/painting remain separate work. See
[the complete measurements and validation record](../results/map-interaction-20261125.json).

## Search input/report timings (November 25)

The [backend-owned search audit](performance/generic-search-observations.md)
adds separate native input-validation, full-input serialization, result-binding
and backend-report-validation spans. Actual deployed report validation takes
0.033 / 0.046 ms (Riemann / EthLib), while their physical search intervals take
121.255 / 17.845 s. These are single functional observations, not percentiles
or a before/after speedup. Common validation no longer constructs a discarded
Lean source page; Lean retains actual graph projection and cache freshness.

Fresh-browser saved-analysis visits take 10.992 / 1.957 s to observed ready
status, with no compiler, search or rendering worker. Riemann's remaining Lake
query and complete-input fingerprint take 3.887 / 3.716 s. These native waits
remain distinct from the previously recorded first-detail browser drawing
stalls. The common port/storage migration does not change the browser assets.
