# Local profiling and regression workloads

The [publication-camera record](../../docs/performance/graph-publication-camera.md)
fixes a native viewer reset when valid graph updates reissue IDs. It separately
records rejected border/path replays, an actual world-path-cache comparison and
four/eight-reader full-input preparation measurements. Faster prebuilt-path
replay does not establish faster initial map exposure; the production painter
and four-reader bound remain unchanged.

The [consolidated SVG comparison](../../docs/performance/svg-local-transforms.md)
measures all thirteen operation windows on the deployed `5771462` executable.
Each actual project runs current/prototype/prototype/current with matching hidden
source maps. Fewer declaration/reference attributes do not consistently shorten
Riemann's pause and EthLib style intervals grow to about 111 ms. The prototype
is rejected; only its archived patch and measurements are retained.

The [earlier deployed event timings](../../docs/performance/event-latency-latest-20261126.md)
measure all thirteen operation windows twice per actual project on `d09eb9e`.
Riemann's first-detail wheel takes 91–92 ms, with 136–137 ms long tasks;
cached file preparation takes 11–12 s. Ordinary graph wheel handlers take about
1 ms. Per-event CSVs, exact matching source maps, native stages and sampling
separate main-thread rendering from full-input validation. This audit changes
no application behavior.

The [camera-motion record](../../docs/performance/camera-motion.md) combines
actual mixed pointer/wheel input, a rejected hover-remount fault and an isolated
SVG construction comparison. Fewer JavaScript writes did not improve construction
time: numeric batch parsing is rejected and the original runtime remains.

The [saved project-map record](../../docs/performance/saved-project-map.md)
separates first displayed navigation from complete native refresh. It retains
baseline/prototype/final startup observations, exact original inventory digests,
source/reader/navigation/evidence fidelity and both intended negative controls.
Saved viewing does not remove current-input checks or the serial command queue.

The [November 26 deployed audit](../../docs/performance/event-audit-20261126.md)
records all 13 interaction windows twice per actual project on `e41a847`, with
event dispatch, later rendering, input/frame proxies and native preparation
separated. Its numeric record retains exact executable, asset and capture hashes.

The [ce0c88f event audit](../../docs/performance/event-latency-current.md) measures two captures per actual project on `ce0c88f`. It separates repeatable 117–119 ms Riemann deep-wheel dispatch, 50–164 ms detail-rendering stalls, occasional node-selection work and cached native input validation. This is measurement of the unchanged implementation, with exact source maps and native sampling.

The [ordered sibling index](../../docs/performance/map-sibling-bounds.md) keeps
closed intersections and original order while pruning dense off-screen branches.
Its 100,000-file/six-camera gate compares nine interleaved pairs with the retired
sibling walk under a 0.02 ratio budget. `MEDEN_PERF_MUTANT=linear-siblings` and
`MEDEN_PERF_MUTANT=disconnected-siblings` must fail that budget. Actual inventory
queries, browser events and startup observations are recorded separately.

The [Worker-paint experiment](../../docs/performance/worker-paint.md) compares
the same Riemann gestures in current/Worker/Worker/current order. Total long-task
time falls, but worst stalls do not improve, so the prototype is not promoted.
The profiler now records each window's document process/thread and excludes
simultaneous Worker callbacks; ambiguous/missing marks fail capture.

The [snapshot-boundary event audit](../../docs/performance/snapshot-events.md)
records all 13 operation windows on both deployed projects, with matching hidden
source maps and exact native executable attribution. It separates the first-detail
DOM/Canvas stall from cache-hit full-input validation and initial bounded search.
It adds measurements and documents without changing product code.

The [November 25 event audit](../../docs/performance/event-audit-20261125.md)
records two actual-project captures per project, all 13 interaction windows,
five complete navigations and native sampling. It distinguishes fast overview
handlers from later deep-zoom tasks and cache-hit file preparation; no runtime
or deployment change is part of that measurement.

The [SVG placement comparison](../../docs/performance/map-placement.md) measures
stable groups of at most 128 original area siblings and separate painter layers
against the preceding served map. CPU attribution uses the exact matching hidden
source maps and retains original profiles, not a simulated React lookup. The
generated map corpus additionally checks lossless group order, unique keys and
the file-parent bound; the real large-map browser journey retains keyboard focus
through lazy declaration loading. These are presentation checks. No sampled
function frame means unavailable at that resolution, never zero execution.

The [area-paint comparison](../../docs/performance/map-area-paint.md) keeps
OffscreenCanvas paint separate from retained SVG interaction geometry. The
public native map journey checks actual draw calls/pixels, constant screen sizes,
reference endpoints, keyboard focus, resize, restart and zero retained-area
geometry/style writes during eight wheel gestures. It runs at device-pixel ratio 2. The native fixture launcher's `MEDEN_GUI_TEST_ASSETS` selects an immutable
old asset directory for the negative control; that renderer must fail the write
budget after passing geometry checks. The DOM-Canvas control must instead fail
zero DOM font/text/image-copy work; the retained-tab control must fail zero
hidden-input layout reads after applying a real held navigation response. This
input affects test hosts only. The
interaction profiler locates rectangular bounds for both target structures,
without forced clicks. Repeated served captures remain separate from this
work-count regression and from native preparation.

The prepared-map update budget compares complete index reconstruction with a
sparse file-description update, using the same already prepared geometry and
checking the complete resulting scene before timing. Nine alternating runs on
175,187 files require the update/rebuild median ratio below 0.25. It excludes
layout preparation, browser paint and native compilation. Output is
`target/performance/map-overlay.json`. `MEDEN_PERF_MUTANT=full-index` forces
complete reconstruction into the update arm and must fail that ratio assertion.
`npm run test:fuzz` also exercises 3,000 finite file/target update traces against
fresh complete projections, preserving previous scenes and original inventory
data. See the [actual browser comparison](../../docs/performance/map-updates.md).

The project-map reference budget is part of `npm run test:performance` and
`npm run test:fuzz`. Its public workload retains ambiguous public/file-private
names, scoped fallback, cycles, repeated references and nonempty original
ordered edges. Nine alternating original-scan/indexed runs must stay below a
0.65 median ratio. `MEDEN_PERF_MUTANT=scan-references` forces the old scan into
the indexed arm and must fail that ratio assertion. The original resolver is
test-only; there is one runtime implementation. Output is
`target/performance/references.json` (`MEDEN_PERF_OUT` selects another folder).
An optional `MEDEN_REFERENCE_WORKLOAD=/private/inventory.json` validates the
original complete-file vector through the generated navigation-record schema
and compares every ordered edge before timing. Supply a complete original
navigation capture, rather than the separate heading-only overview. The harness
also replays its headings/details split and checks the same symbols and edges.
Private inventory data is not a CI prerequisite. See the
[actual-project comparison](../../docs/performance/map-references.md).

The same fuzz command runs 3,000 identifier/cycle cases through the actual
`l4sb.cone.v1` dependency-cone implementation. Its separate six-graph golden
retains ordinary upstream coordinates and order, while all twelve
`Object.prototype` names remain valid original declaration identifiers.
This is a correctness corpus, not a cone latency budget. See the
[version/configuration audit](../../docs/performance/cone-layout.md).

The reference-update gate in the same performance command resolves eight detail
arrivals over 38,400 fixed headings. Nine interleaved pairs require retained
candidate indices to stay below 0.65 of forced global reconstruction, with exact
ordered edges and complete symbol values. Each series starts with a fresh
prepared scene; initial materialization is outside the update interval.
`MEDEN_PERF_MUTANT=rebuild-reference-index npm run test:performance` must fail
that budget. The generated update corpus now also compares every step with the
independent original reference oracle, including stable identities with changed
reference arrays and changing ambiguity/private visibility. See the
[actual-project comparison](../../docs/performance/map-reference-updates.md).

The [heading/detail transfer comparison](../../docs/performance/inventory-headings.md)
records the preceding served inventory payloads, event distributions, native
preparation spans and both serialized retained-tab budgets. It distinguishes
compiler-artifact cache hits from already-open tabs.
The [disjoint inventory follow-up](../../docs/performance/inventory-states.md)
retains those observations while replacing repeated empty navigation with an
explicit unindexed state and borrowing metadata during publication freezing.

`node --experimental-strip-types tools/perf/native-command-codec.mjs OUTPUT.json`
compares the generated command tag lookup with a sequential union of the same
owned branches. It validates and JSON-encodes preconstructed synthetic commands,
consumes/checks their encoded lengths and records nine interleaved batches per
strategy/type after warmup. The output path must be new. This is a diagnostic
codec comparison, not HTTP/queue/model or browser input-to-paint latency. The
former GUI did not validate outbound commands, so it is not an old/new GUI
speedup claim or a new performance gate.

`bash tools/perf/perf.sh stat -e task-clock:u -- true` probes real same-user
counter access. The wrapper uses an explicitly selected `MEDEN_PERF`, installed
`perf`, or one pinned Nix package; it never changes sysctls, privileges or the
host Rust/Lean selection. A failed permission/tool probe is an unavailable
prerequisite, not completed native coverage. Keep debug symbols (`cargo build
-p meden-cli --profile profiling`). The separate profiling binary avoids
replacing a running GUI's executable.

```sh
node tools/perf/profile-native.mjs /absolute/project selected/File.lean target/native-profile/run
```

This starts an independent native GUI on port 4324, performs public explorer and
graph interactions, samples the host and inherited workers with user-space
`cpu-clock` at 99 Hz/DWARF, exports bounded host spans, and decodes the recording.
It uses installed Lean and existing project/OS caches, and makes no model
requests. An already-open workspace is locked; use an idle existing GUI instead:
set `MEDEN_PROFILE_PID` and `MEDEN_GUI_URL` to its actual PID and session URL.
Attach mode never stops the existing host. `MEDEN_PROFILE_PORT` selects another
port for independent mode. Do not record while another user job is running.
Old hosts without `/api/metrics` export `null`, not fabricated span coverage.

Raw captures stay local in `target/`: they contain local paths, symbols and
project metadata. The manifest records the executable digest, counter settings
and raw-artifact digests. Session fragments are redacted in exported browser
traces. Do not publish raw captures as ordinary telemetry.

The native `/api/metrics` query requires the same loopback host/origin/session
checks as state. It exposes only static stage names, numeric trace/parent IDs,
monotonic offsets, wall time and current-thread CPU time, retaining at most 4096
completed intervals with an eviction count. Queue spans intentionally have no
CPU value. Nested spans overlap; never sum them as exclusive costs. Thread CPU
excludes child processes, and wall minus thread CPU includes scheduling, locks,
I/O and child execution; it is not a measured I/O-wait counter. Instrumentation
is diagnostic and cannot change result admission. No source, prompt, token,
endpoint or filename is a metric label. No remote export is enabled.

Lake configuration now distinguishes `atlas.lake_configuration.resolve` (resource
validation and exact frozen-input lookup), `atlas.lake_configuration.reuse` (a
validated retained result) and `atlas.lake_configuration` (actual evaluator
execution). Warm configuration reuse still requires `atlas.project.query` and
`atlas.project.fingerprint`: the actual offline Lake build/query and a separate
read-only full-content validator. The latter's Python/script are absent from
the compute library visible to project code. Older traces combine these stages
under `atlas.project.prepare`. Compare these stages separately from
source-view materialization and retained-tab navigation.

`analysis.metadata.validate` measures complete configuration/absence/inventory
revalidation. Its `analysis.metadata.directory_batch` and
`analysis.metadata.file_batch` children expose the bounded policy-bound worker
batches. Count descendant `process.run` spans within each cache validation;
direct children alone omit nested source capture. Compare complete public
inspection duration separately from `source_preparation.elapsed_ms`: navigation
notes may perform a second validation after the file-local preparation completes.
Batching reduces worker startup; it does not remove either check, reuse a prior
successful validation, or avoid Lake's complete compiled-input fingerprint.

`view.analysis.materialize` freezes a selected/running candidate, whereas
`view.analysis_history.materialize` freezes the complete retained history only
when the original list changes. Exact reads have separate `gui.analysis_respond`
and `gui.analysis_history_respond` intervals. Selection, progress and source
invalidation must leave the latter body's identity and allocation unchanged.
The large-history browser workload records original body sizes, the small status
projection and history reads/commands during local retry, navigation and selection.
These candidate observations are independent of current accepted-project evidence.

`view.evidence.materialize` freezes the complete original accepted/proposed
contract review and typed evidence observation at the host mutation boundary.
`gui.evidence_respond` measures exact retained-byte responses separately from
`gui.state`; polling and evidence queries cannot materialize or verify a contract.
The finite-verifier browser journey records 20 sequential complete-body fetch
intervals, status/body byte sizes and the native span snapshot after actual
Z3 rejection/repair. It checks that those reads add no materializations. These
are diagnostic loopback/browser observations under functional validation, not
an old/new latency benchmark or exclusive-stage CPU profile.

Portable work checks emit `recorded.check.total`, `.lake`, `.inputs` and
`.inspect` spans, plus `recorded.history.read` / `.write`. These surround the
actual shared service, fixed offline Lake dispatch, complete input fingerprint,
named-theorem inspection and history I/O. Input capture occurs before and after
the build. They preserve wall and current-thread CPU separately; nested spans
must not be added together and child-process CPU is excluded. The historical
progress publication is separate from accepted-contract verification/evidence.
The held-build browser journey validates instrumentation and recovery rather
than natural compilation throughput.

Accepted-project checks also emit `verification.capture_source`,
`verification.prepare`, `verification.check`, `verification.refresh_source`
and `verification.retain` around their actual application boundaries.
`view.verification_job.materialize` freezes a progress observation;
`gui.verification_job_respond` serves its exact retained bytes. Context keeps
the original contract/method and stages after issue keep the source/prepared-input
digests. These observations do not issue jobs or admit evidence. A cancelled
issued job remains distinct from a later successful job. Wall spans can nest
other stages; host-thread CPU excludes child-process work and cannot attribute
solver/Lean CPU. The browser fixture deliberately holds dispatch and response
timing, so its spans validate instrumentation rather than natural throughput.

`view.preview.materialize` freezes a complete preview description at capture or
missing-asset mutation. `gui.preview_respond` serves its original frozen bytes.
The real browser journey records small status/body sizes and twenty direct
loopback reads, requiring no new materializations or commands. Frame identity
is independent of diagnostic revision, so updates and transfer retry do not
reload the live app. Its transfer faults are instrumentation/UX conformance,
not a natural throughput benchmark or proof of browser isolation.

Prepared roots now use one batched Lake query for their normal build and
source/output/compiler-setup results. The file's streamed diagnostic log separates
build/setup/input resolution from the exact file/byte validation duration.
A cached open must still validate all bytes; compare a new compiler/resource
artifact's initial extraction/search fill separately from subsequent cache hits.

Fingerprint version 2 hashes each complete file with four readers and at most
256 queued jobs, then combines the content digests in sorted path order with the
original setup records and exact names/lengths/counts. It retains the same file
and byte limits. Metadata checks detect ordinary concurrent changes; they do not
replace byte reads or provide a concurrent-snapshot proof. Worker conformance
tests cover reordered completion, multiple batches, empty/multi-chunk files,
same-size edits with restored timestamps, replacement, symlinks and byte limits.
Changing the fingerprint implementation invalidates old compiler/search cache
identities; measure the initial fill separately from subsequent prepared opens.

`npm run test:fuzz` generates 3000 bounded inventories and camera traces, including
empty maps, deep categories, tiny panes, extreme zooms and cyclic references.
It compares visibility and center breadcrumbs to independent full-scan
specifications, checks the declaration legend against the rendered visible
symbols, and verifies unchanged world coordinates and identical layout replay.
Every generated inventory also targets area centers and corners at deep zoom.
Fast-check shrinks
failures and prints a seed/path; replay with `MEDEN_FUZZ_SEED`,
`MEDEN_FUZZ_PATH`, and `MEDEN_FUZZ_RUNS` using the direct Vitest file command.
Retain that counterexample in the corpus before fixing a discovered fault.
The correctness corpus has a 60-second runner allowance, matching the large-map
test, rather than Vitest's five-second default. Its 3000 cases and assertions
remain unchanged; this is separate from the numerical performance ratio below.
These are presentation properties, not verification or renderer proofs.

`npm run test:performance` uses the pinned 175187-file corpus and nine alternating
paired measurements (ten calls each) after warming both implementations. Workload
version 2 checks visible areas, the visible declaration legend and the center
breadcrumb at overview, category and fitted-file cameras. The
optimized/full-scan median ratio must remain below 0.35. This relative budget
runs on the same process/runner and detects reverting to a whole-map scan without
pretending shared CI provides stable absolute hardware timings. Absolute layout
and sample timings, workload, Node/kernel/CPU identity and the ratio are saved
in `target/performance/map.json` and uploaded by CI. It does not measure browser
paint, native preparation, cold installation or actual model quality.

For browser motion/cache coverage run the real retained-tab journey under both
`MEDEN_GUI_MOTION=reduce` and `MEDEN_GUI_MOTION=no-preference`; it starts with an
empty analysis store and revisits prepared tabs repeatedly without another
inspection. The full diagnostic browser profiler uses a fresh page, 600 ms
post-gesture tail and eight further frames, excluding CPU-profiler boundaries.
The diagnostic browser tool permits up to ten minutes for initial file
preparation before measuring gestures; worker budgets remain separate. Its
environment records preparation time and the host's actual cache-hit flag, so
an uncached compiler/search run is not labelled cache validation alone.
Repeat captures per motion mode and cache condition before comparing browser
latency; do not treat a single diagnostic percentile as a continuous browser
performance budget. See `docs/gui-performance.md` for the current measurements.

The browser report separates dispatch durations from the input/two-rAF proxy
for each event type, including pointer movement and window resize. It records
the retained graph transform before/after each phase. A hidden file has no
active graph transform; a deliberate pan or Fit View changes it normally.
Native stage distributions include only newly completed span IDs since the
initial snapshot and report eviction during the capture. Stages overlap, and
zero CPU samples means unavailable measurement rather than zero CPU work.

`events.csv` and `events.md` expose each action/event type's sample count,
synchronous handler p50/p95/max and the corresponding two-rAF p95. The JSON
keeps chronological dispatches, inclusive rendering spans and sampled CPU
leaders. A separate `project-map-drill` phase clicks reachable category headers
and a file box, retaining lazy map traffic and its rendering tail; overview
wheel events alone do not exercise declaration-chunk updates. Discrete
[Event Timing](https://www.w3.org/TR/event-timing/) observations additionally
retain input queue delay, processing and reported duration at a 16 ms threshold.
Absent observations are unavailable, never zero; continuous wheel/move/resize
events do not receive that API's queue measurement. Delayed observer delivery
is attributed by start time to the original measured window. No diagnostics
are added to the normal product UI or sent to an analytics service.

```sh
node tools/gui/profile-map-load.mjs '<GUI_URL>' target/gui-profile/map-load
```

This separate driver measures five full navigations in a fresh browser context,
retaining the same native host/project/OS caches. It captures inventory transfer,
long tasks and a CPU profile per visit, including JSON decoding, canonical input
construction, geometry preparation and initial rendering. DOM attachment and
two-rAF timings are navigation-relative proxies, not physical presentation.
Actual DOM cache-source attributes classify hits; the first visit number alone
does not. It opens no files or chat, refuses non-directory commands and checks
that the final host remains idle/unapproved. Optional storage gets a one-second
settling interval between visits, outside the measured readiness proxy. Keep
these diagnostic captures serialized with other performance work.

The report also retains the complete API request timeline, including setup and
between-phase reads. Use it to distinguish one revisioned history fetch from
repeated status traffic; do not infer that setup performed no transfer merely
because it is outside a gesture phase. Response sizes are loopback body bytes,
and response-end timing includes transport/browser scheduling. Native status
computation and non-browser fetch/body-read probes measure different boundaries.

Fit View has its own phase. Selection first clicks one declaration eight times,
then alternates between two reachable declarations for eight more clicks. Each
click must reveal its original source line; target IDs, per-selection camera
transforms and individual dispatch durations are retained so repeated selection
cannot hide the cost of changing declarations. The profiler waits three frames
after each selection before the next click. This does not cover every node or
arbitrary rapid clicking. `preparation.json` retains file-local progress, elapsed
preparation time and cache-hit status. `preparation-log.json` reads the exact
original revisioned log, including its truncation flag; both are retained beside
the profiles and included in the native capture's digest manifest.

Prepared sources are queried through the exact revision advertised by the small
status index. The profiler performs one additional source-body read after
preparation for its declaration/source-line oracle; distinguish that read from
the application's one cached read when counting setup traffic. A declaration
without a reported source line has no line-jump assertion. These diagnostic
queries never become inference or compiler commands.

Set `MEDEN_GUI_ASSETS` to the asset root containing the source maps for the
served bundles (default `apps/web/dist`). The report retains the map digests.
For a pinned deployment without maps, build separately with `--sourcemap hidden`
and compare every generated JavaScript bundle byte-for-byte with its served
counterpart before using those maps. Do not attribute a running bundle through
maps from a different build. The November 23 event audit checks all seven
JavaScript bundles this way without replacing the deployed assets.

Complete Cargo builds/tests before starting a GUI/TUI validation run, or copy
the built executable to an immutable digest-named validation path. `cargo test`
may rebuild the ordinary CLI binary for integration tests. Replacing a running
host's executable can invalidate its prepared worker path (`/proc/self/exe`
reports a deleted file); a resulting ENOENT is not a valid latency sample.
Do not rebuild that path while measuring. Restart deployed hosts only after
confirming no job is active, and issue fresh session URLs after restart.
Pin the built browser assets alongside the executable as well: a running host
must not serve a mutable `apps/web/dist` while browser tests replace its chunks
or a new wire contract is being built. Name the deployment from both the binary
digest and a deterministic asset-tree digest, and pass that copied asset directory
to `gui --assets`. A pinned executable alone does not freeze its browser client.

The serialized native browser performance gate sets `MEDEN_GUI_PERFORMANCE=1`:

```sh
MEDEN_GUI_PERFORMANCE=1 MEDEN_GUI_MOTION=reduce npm run test:gui -- --workers=1 --grep 'open file tabs restore'
MEDEN_GUI_PERFORMANCE=1 MEDEN_GUI_MOTION=no-preference npm run test:gui -- --workers=1 --grep 'open file tabs restore'
```

Run those sequentially, with separate `MEDEN_GUI_OUTPUT` / `MEDEN_GUI_REPORT`
directories to retain both reports. It records the cold analysis-store preparation
and ten retained-tab input/two-rAF samples; warm p95 must be below 250 ms. It also
requires zero new inspection commands and retained graph/camera/source identity.
This is a broad desktop scenario budget, not a promise for every device or a
physical presentation measurement. Ordinary parallel browser journeys attach
the timing data; numeric budgeting is reserved for this serialized gate.

`MEDEN_PERF_MUTANT=full-scan MEDEN_PERF_OUT=target/performance/mutant npm run
test:performance` is a negative control and must fail at the ratio assertion.
CI checks its actual report so an unrelated tool failure cannot satisfy the
negative control.

## Isolate prepared-input work

```sh
cargo build -p meden-app --example profile-prepared-inputs
target/debug/examples/profile-prepared-inputs /absolute/project \
  '[{"name":"Entry","file":"Entry.lean"}]' 3 > /tmp/prepared-inputs.json
```

This host-only diagnostic uses the application's actual selected Lean/resources,
original offline sandbox and runtime descriptor-closing launcher. It builds the
selected Lake roots and validates every compiled-input byte. It does not extract
a graph, call a model, accept a contract or admit evidence. The JSON report
separates initial resource preparation from each measured invocation, retains the
actual fingerprint/file/byte counts and bounded original worker diagnostic, and
names the selected public Python object and resource revision. Project/OS caches
are not reset. The output contains local paths/diagnostics; keep it local.

Compare implementations with interleaved runs on the same input/resource setup,
require exact result equality and retain executable/script digests. Measure
sampler on/off separately; sequential before/after captures can differ for other
reasons. The [Riemann calibration](../../results/lake-preparation-calibration-20261124.json)
rejects a slower path-validation prototype rather than adopting a misleading
standalone improvement. Production compilation and cache checks remain unchanged.

## Isolate source and layout selection

```sh
cargo build -p meden-app --example profile-source-selection
target/debug/examples/profile-source-selection /absolute/private-project \
  /absolute/installed/meden '["src/Entry.lean"]' 3 > /tmp/source-selection.json
```

This diagnostic calls the shared source/root-selection service through the real
policy-bound file worker and frozen offline Lake configuration evaluator. It
retains complete canonical selection hashes, source/package/configuration
counts, configuration byte counts and original native spans. The JSON excludes
source/configuration contents. Preparation and result hashing are outside the
selection intervals. There is no project-module build, graph extraction,
counterexample search, inference or evidence admission. Existing project/OS and
frozen-evaluation caches remain present; keep paths and failure diagnostics local.

Use a private byte-identical mirror when the selected project is already open;
the exclusive workspace lease is not bypassed. Compare fixed executables with
the same original worker, resources and input paths, require complete hash
equality, and interleave samples without concurrent builds/tests/sampling.
`python/tests/test_source_selection.py` adds an independent complete-input
oracle, unavailable-input checks and a deterministic worker-start budget.
It builds the diagnostic example if needed; `MEDEN_SOURCE_SELECTION_PROFILE`
can select a pinned diagnostic executable for an explicit negative comparison.
This count budget is separate from timing and cold-install/model validation.
See [the layout comparison](../../docs/performance/layout-frontier.md).

## Isolate portable history loading

```sh
cargo build -p meden-app --example profile-recorded-history
target/debug/examples/profile-recorded-history /absolute/workspace \
  /absolute/installed/meden 3 > /tmp/recorded-history.json
```

This host-only diagnostic calls the shared application's actual complete record
loader through the original confined file worker and policy. It reports each
read/decode/validation interval, a canonical document digest and counts, and the
existing native wall/current-thread-CPU spans. It prints no document contents.
Workspace/tool preparation and result hashing are outside the read interval.
There is no inference, Lean execution or evidence admission. The source file is
still bounded at 4 MiB and the complete encoded worker reply at 8 MiB.

Use a private, byte-identical copy when the actual workspace is already open;
the normal exclusive workspace lease is not bypassed. Keep both input and host
data directories private. Compare pinned implementations and the same selected
worker with interleaved runs, check complete document equality, and distinguish
warm-byte reads from cold installation/startup. The
[recorded-history comparison](../../docs/performance/recorded-history.md) records
the original numeric-array reader against the existing encoded snapshot mode;
the production loader retains only the latter.

For separate duplicate-background diagnostics, set
`MEDEN_GUI_PAINT_OBSERVATIONS=1` when running either profiler. The optional
`area-paints.json` records bounded ordered-command agreement and is included in
the native wrapper manifest. Hooks perturb timing; leave the flag unset for
comparisons. This is scoped to the current area painter, not arbitrary Canvas
pixel equivalence. See [the rejected cache trial](../../docs/performance/map-repaint-cache.md).
