# Display a saved project map before discovery

The application now retains a disposable navigation inventory independently of
source preparation, accepted meaning, model sessions and admitted evidence.
On restart, the GUI validates and publishes that saved metadata before running
the existing bounded workspace refresh. The map says **Saved map** until
the current observation finishes. Explicit refresh also retains the old map
while the new inventory is prepared.

`meden-app::navigation::saved` owns bounded host storage for generic file/symbol
metadata. The native adapter selects its complete syntax-composition identity;
changing that method refuses the old saved record. A canonical inventory digest,
complete JSON decoding and the existing inventory projection validate the view
before publication. No compiler input, analysis backend or proof receipt is
restored through this path. Navigation annotations are already display metadata;
restoring them does not admit current verification evidence.

The 64 MiB bound applies to the saved metadata record, not source-file content.
Buffered streaming serialization avoids collecting another complete JSON copy
or issuing one write/hash update per small field. A failed replacement preserves
the previous complete file. Missing, malformed, changed-method, checksum-invalid
or oversized data falls back to normal discovery. This is disposable atomic
replacement, without an fsync durability claim or a filesystem refinement proof.

The original startup freshness policy is computed before loading the saved view.
Consequently restoration does not force a large project's existing metadata
cache to rebuild. Directory/content observation under the existing limits and
large-project cache policy, entry preparation and one normal annotation load
still run. Source and analysis contexts are cleared
and prepared through their original paths. The generated native wire schema,
compiler/parser implementations and dependencies remain unchanged.

After that new observation, an exact comparison covers every captured file,
symbol, exclusion, inventory policy and generation. When it matches the restored
inventory, the host reuses its validated publication and retains the original
saved file. It starts a new detail cache rather than retaining old query results.
Changed observations build and save a new publication. This avoids duplicate
materialization and rewriting on unchanged restart; it never skips discovery or
uses saved metadata as an execution input.

The saved record is the last successfully prepared workspace refresh, including
its inventory weighting and generation. It is not every later per-file detail
update or an atomic current-filesystem snapshot. Large maps still obtain their
later details through the existing metadata cache. The foreground command queue
remains serial, so file inspection and model work can still wait for startup
refresh. This change advances first map visibility, not every waiting path.

## Regression and measurement

The public native restart journey first prepares an actual Rust declaration,
edits it without changing file size/count, then holds the real directory-batch
dispatch. The original saved declaration remains visible with loading/unapproved
state and no prepared source or inference. Wheel input over the saved-status
badge reaches the map; zoom controls work. Releasing the same original worker
publishes the new declaration, removes the old one and preserves the camera.
Corrupting the actual retained file then recovers through ordinary discovery.
An additional unchanged restart must perform a new directory observation while
retaining the original saved inode/time, materializing once and rewriting zero
times. These work-count checks accompany actual visible declarations and fresh
model-approval state, rather than a simulated cache fixture.

The old executable is tested with the new viewer and must fail saved declaration
visibility after that real dispatch has started. This is a negative control for
early viewing, not a startup timing measurement. No worker result is fabricated.

The [numeric record](../../results/saved-project-map-20261126.json) retains final
validation, actual-project observations and artifact hashes. First scene/two-rAF
and host polling are presentation/readiness proxies, not physical paint, field
latency, cold installation or live-model validation. Native wall intervals
overlap; host thread CPU excludes child workers.

## Actual-project startup

One sequential capture per project/version uses the installed Chrome 149,
1440 × 960, normal motion, fresh isolated browser contexts and retained
project/OS/native caches. No builds, tests, inference or samplers run concurrently.
Both inventories retain the complete original values: 175,187 files in Riemann
and 2,517 in EthLib. The first scene has the same revision and camera; final
inventory JSON has the same digest in all three runs.

| Observation                       | EthLib baseline | EthLib final | Riemann baseline | Riemann final |
| --------------------------------- | --------------: | -----------: | ---------------: | ------------: |
| Navigation to scene/two-rAF       |        17.085 s |      2.590 s |          7.447 s |       3.212 s |
| Restart request to observed scene |        18.444 s |      3.919 s |          8.512 s |       4.258 s |
| Native complete refresh job       |        17.516 s |     18.980 s |          6.625 s |       7.773 s |

The final map is observed while the actual refresh is still running, with
**Saved map** and no restored model approval. This is early viewing, not faster
complete observation. Native job time remains about 1.1–1.5 s above this baseline;
loading and checking saved metadata costs 1.123 s in EthLib and 1.362 s in Riemann.
These single diagnostic pairs are not population percentiles or a universal
speedup guarantee. Directory discovery itself varies between runs.

The first prototype materialized twice and rewrote unchanged metadata. Its
native jobs took 20.263 s in EthLib and 9.609 s in Riemann. Exact comparison after
fresh observation removes that duplicate work: both final restarts materialize
once and do not retain/write the saved record. The prototype must fail the
unchanged-file regression. Full-update cost, the serial command queue and
first-exposure drawing remain open performance work.

All 179 application unit cases, 37 application documentation examples, 77 CLI
cases and 37 required TUI journeys pass. Seven focused native browser journeys
plus retained-tab navigation in both motion settings pass (nine executions),
with no skipped, flaky or unexpected positive cases. Retained-tab two-callback
p95 is 32.7 ms with reduced motion and 42.9 ms with normal motion, below the
existing 250 ms gate. Two intended negative controls fail their exact viewing
and unchanged-file conditions. Decoded review/clipping/failure/restart screens
and the saved-view screenshots were inspected. These installed-tool/scripted
checks are not live-model quality or cold-install validation.

Both actual projects retain their complete original source, reader, inventory,
navigation and evidence JSON after file inspection, with original history and
candidates. Compiler artifacts are reused without semantic compiler, extraction,
search or explanation-rendering workers; normal complete input checks still run.
Native rings have zero eviction and both hosts remain idle and unapproved.

To exercise the public startup regression with installed prerequisites:

```sh
cargo build -p meden-cli
MEDEN_BIN="$PWD/target/debug/meden" npm run test:gui -- \
  --workers=1 --grep 'saved project map appears before real discovery'
```

Generic layout, metadata-cache orchestration, observation-revision correspondence,
native freshness costs, first-exposure rendering and retirement remain under
the [original migration plan](../architecture/typed-core-and-plugins.md).
