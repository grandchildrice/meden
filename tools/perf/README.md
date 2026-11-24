# Local profiling and regression workloads

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
execution). Warm configuration reuse still requires `atlas.project.prepare` and
the exact compiled-input fingerprint. Compare these stages separately from
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
arbitrary rapid clicking. `preparation.json` retains the actual file-local build
log, elapsed preparation time and cache-hit flag alongside the profiles; it is
included in the native capture's digest manifest.

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
