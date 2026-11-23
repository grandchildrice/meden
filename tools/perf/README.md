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

`npm run test:fuzz` generates 3000 bounded inventories and camera traces, including
empty maps, deep categories, tiny panes, extreme zooms and cyclic references.
It compares optimized visibility to a full-scan specification, checks world
coordinates are unchanged and layout replay is identical. Fast-check shrinks
failures and prints a seed/path; replay with `MEDEN_FUZZ_SEED`,
`MEDEN_FUZZ_PATH`, and `MEDEN_FUZZ_RUNS` using the direct Vitest file command.
Retain that counterexample in the corpus before fixing a discovered fault.
These are presentation properties, not verification or renderer proofs.

`npm run test:performance` uses the pinned 175187-file corpus and nine alternating
paired measurements (ten calls each) after warming both implementations. The
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
