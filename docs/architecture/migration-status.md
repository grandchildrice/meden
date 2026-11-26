# Typed-core migration: current evidence and remaining work

This is a progress audit against the five exits in
[the implementation plan](typed-core-and-plugins.md), not a claim that the whole
migration or its proposed assurance obligations are complete. Source inspection
and checked-in validation records have different scopes; a wired CI command is
not evidence that a new remote run is green.

| Planned exit                                       | Current concrete evidence                                                                                                                                                                                                                                                         | Boundary of that evidence                                                                                                                                                                                                                                                                     |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Dependency boundaries and obsolete abstractions | `tools/architecture/check.mjs` checks Cargo direction, pure data/port imports, generated native types, role capture and owned navigation consumers. CI runs `architecture:check`. The unused event port and browser source index are retired.                                     | Structural owned-source checks do not establish alias-proof isolation or runtime I/O/refinement.                                                                                                                                                                                              |
| 2. Routine measurements and adversarial gates      | CI runs `test:fuzz`, paired `test:performance`, the rejected full-scan mutation and serialized public browser budgets in both motion settings. `tools/perf/README.md` links actual-project captures and rejected experiments.                                                     | Named corpora and timing proxies have coverage; universal smoothness, physical presentation, cold installation and live inference are separate.                                                                                                                                               |
| 3. Shared use cases and typed backend ports        | `meden-app` owns workspace/project, analysis/repair/delivery, compiler/search/renderer/model registrations, generic navigation collection/cache and typed file observations. The finite-transition backend runs real Z3 through the same service/viewer without Lean preparation. | The new observation adapter is tested against actual confined I/O. JSON/OS provenance and whole-session correspondence remain adapter obligations, not newly refined kernels. This does not claim a general SMT frontend or RTL verifier.                                                     |
| 4. Generic immutable read models                   | Application-owned `view` DTOs generate native schemas/validators. Inventory, availability, source/graphs, progress, candidates and evidence have separate publications. Native graph rendering consumes declaration/edge fields without inventing a recorded workspace.           | Bounded/binary/unsupported/cyclic inputs and freshness/restart have named journeys. Warm tabs and saved maps are fast paths; measured first-detail stalls and complete new-file validation remain performance work.                                                                           |
| 5. Retire replacements; choose one measured engine | The browser has one dependency renderer shared by native/recorded contexts and a project-map area painter with SVG interaction layers. The rejected Worker/paint-cache/placement trials are outside production.                                                                   | `npm run dev`'s recorded demo remains a documented live route (`main.tsx`, README and `docs/gui.md`), with its own repository port and captures. Its data context is distinct from native publications. It cannot be called unused or deleted without the planned replacement/coverage audit. |

The [consolidated SVG trial](../performance/svg-local-transforms.md) measures the
current executable on both actual projects and rejects fewer SVG groups/writes
as a fix: Riemann's worst pause is not consistently shorter, while EthLib's
style intervals grow. The production renderer stays unchanged. Canvas border/path
work, style invalidation and complete native preparation remain separate targets.

The [publication-camera fix](../performance/graph-publication-camera.md) retains
the native file viewer's supplied camera and center across reissued declaration
IDs, refuses ambiguous or differently named ID reuse, and selects current source
locations. Isolated shared-component cases and public native journeys have
different scopes. Rejected border/path-cache and eight-reader trials do not
establish shorter initial map stalls or prepared-input waits.

Current captures retain 94–95 ms first-detail wheel work, 138–140 ms overlapping
long tasks, EthLib style invalidation and separate cached native preparation
waits. The earlier [deployed event audit](../performance/event-latency-latest-20261126.md)
also separates these rendering and native stages. Those are not fixed by moving
metadata code, by a missing CPU sample, or by a fast retained-tab test.

The [attribute-invalidation audit](../performance/attribute-invalidation.md)
identifies concrete presentation-attribute/inserted-subtree observations and adds
a chronological per-dispatch export to the local profiler. Isolated reference
stroke and background grouping trials are rejected; reducing attribute writes
does not by itself establish faster detail exposure.

The [combined lexical index](../performance/lexical-index.md) removes a duplicate
production masking/line pass through the existing typed navigation port. Actual
EthLib source collection falls from 2.694 to 2.249 seconds in an interleaved
confined-worker comparison with exact original index observations. Its new
lexical identity stays separate from retained compiler/search/render artifacts.
This does not measure complete GUI loading, deep-wheel rendering or prepared
compiler-input validation.
The actual Riemann method update additionally spends 99.833 seconds populating
its large private syntax cache; reducing that upgrade/startup wait remains work.

Next work must preserve the original camera, ordered coordinates/layers,
keyboard targets, complete source bytes and exact semantic observations while
addressing those measured intervals. Native preparation must preserve current
content binding, failure and cancellation rather than treating metadata hints as
fresh proof inputs. Remaining retirement decisions need actual entry-point and
capture/negative-test evidence. Kernel refinement gaps documented in
`docs/verification.md` stay explicit; this migration does not silently enlarge
its proof claim.
