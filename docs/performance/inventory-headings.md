# Inventory headings and prepared navigation

The [measurement record](../../results/inventory-headings-20261124.json) separates the complete navigation overview from cached detail queries. Exact source/graph/reader/history/evidence data, declaration positions/roles and project geometry are compared against the original deployed projects. No compiler, renderer, search implementation or trusted-kernel source changes.

## Data ownership

The former overview transferred every reference string and prepared explanation, then resolved all lexical links on every browser navigation. The native application validates the complete original input and publishes all file metadata and declaration headings. The overview identity binds that metadata, the actual indexer and a separate fingerprint of every complete original nonempty declaration record, including omitted references and descriptions. Empty declaration vectors require no repeated path/metadata serialization. Overview, detail fingerprinting and chunk publications each have a 64 MiB framing budget; original worker/source/compiler/evidence bounds are unchanged. Omitted reference/description changes still retire the overview. The old full overview wire shape is deleted; both shapes are generated from their distinct Rust DTOs. Complete original records remain prepared on the host and are read through exact parent/path-bound chunks. Small and large maps use the same detail path; an unreadable completed response remains distinct from an unqueried heading.

Names, source lines, kinds, privacy and main-target metadata are preserved before detail loading. Geometry weights and cache revision are unchanged. A file tab also requests its prepared navigation independently of source/compiler publication. Concurrent reads share one transport request; failed transfers retry without recapture, compilation or model dispatch. Each generation owns its pending read, so an older held request cannot block new work or overwrite new data/errors. HTTP status remains available even when an error body is not JSON.

A first full-suite run caught a repeated detail read after opening a prepared file. The candidate cache now retains only lexical candidates for the same actual indexer, path and complete source digest, after matching all current syntax metadata. It owns copied vectors, excludes descriptions and uses current target flags. At most 64 entries/64 MiB of serialized data are retained. This cache does not restore evidence, an approval or compiler cache admission; source/indexer attribution remains an adapter obligation.

## Actual inventory transfer

Private complete before/after bodies remain local and their hashes are recorded. Overview bytes include the native envelope.

| Project | Before | After | Declaration headings |
| --- | ---: | ---: | ---: |
| riemann | 36.238 MB | 36.229 MB | 39 |
| ethlib | 24.209 MB | 7.301 MB | 42,857 |

EthLib originally included 1,329,548 reference occurrences across 42,857 declarations. Its complete heading/detail round trip still produces the original 212,283 ordered edges, SHA-256 `e9cd425a9ebf8d032482e139a8a97cddceac63b52e1aa5a7eaaf381195f5a5f8`, checked against the independent original resolver. Riemann is mostly unindexed inventory metadata, so this split is not expected to remove its large path-vector transfer.

## Browser navigation

Five navigations use one fresh browser context per version/project, Chrome 149.0.7827.53, 1440×960, normal motion, no throttling and CPU sampling. Native/project/OS caches are retained. Actual DOM attributes classify computed/stored geometry; a one-second storage interval is outside readiness. Visits 2–5 navigation-relative two-rAF medians:

| Project | Before | After |
| --- | ---: | ---: |
| riemann | 1275.8 ms | 1239.1 ms |
| ethlib | 2066.4 ms | 749.2 ms |

These are diagnostic navigation observations, not physical presentation or field percentiles. File inspection and compilation are not requested. Full per-event handler distributions, separate two-rAF proxies, deep zoom and exact preparation logs are retained separately.

## Event timing

Inclusive synchronous EventDispatch p95, followed by input/two-rAF p95 in parentheses, in milliseconds. Counts and complete distributions are retained in the measurement record. These are controlled diagnostic gestures; two-rAF does not measure physical presentation or field INP.

| Action | Riemann | EthLib |
| --- | ---: | ---: |
| Map zoom | 4.296 (29.0) | 6.366 (17.4) |
| Graph zoom | 1.095 (16.6) | 1.009 (16.7) |
| Hover | 0.125 (40.8) | 0.084 (23.8) |
| Pan | 0.272 (21.7) | 0.360 (19.4) |
| Resize | 1.412 (22.9) | 1.304 (18.8) |
| Tab click | 18.126 (53.6) | 10.664 (31.0) |
| Distinct-node click | 21.008 (27.8) | 16.965 (21.0) |
| Fit View | 0.929 (26.7) | 0.601 (29.0) |

The remaining deep-zoom maximum long tasks were 251.0 ms for Riemann and 87.0 ms for EthLib. Sampled active work includes DOM attributes, React property updates, lexical references and garbage collection. Moving detail work off initial navigation does not eliminate later detail reads or guarantee faster deep zoom; raw phases and preparation are retained separately.

## Native preparation cost

Binding the complete original navigation remains native work before readiness. The actual materialization spans below include validation, hashing and serialization; source capture and compiler preparation are separate. The historical baseline uses the unchanged preceding executable, with no evicted records in that capture. It is not an interleaved startup benchmark.

| Project | First materialization before | First materialization after |
| --- | ---: | ---: |
| riemann | 1165.9 ms | 1210.2 ms |
| ethlib | 776.6 ms | 985.4 ms |

The first full-input fingerprint implementation doubled Riemann materialization to about 2.30 s. The final composition hashes only nonempty original declaration records separately, then binds all metadata in the overview; this removes the repeated complete path-vector serialization. The repeated browser boundary and one-time native preparation are reported separately. It does not accelerate Lean or skip exact compiled-input validation. Large-inventory paths and remaining native/DOM work still require attention; the broader architecture-plan completion audit remains open.

## File preparation and retained tabs

A compiler-artifact cache hit is separate from a ready retained tab. In these actual captures, initial file setup took 22.45 s for Riemann and 2.15 s for EthLib; both reported cache hits. The Riemann preparation log records 11.939 s for build/setup and input resolution, then 5.858 s to validate 43,477 files / 5,992,114,707 bytes. Nested source/preparation spans are recorded with trace identities; they cannot be summed as independent durations. Parent-thread CPU excludes child-process CPU. This change does not speed up that preparation path.

The separate serialized retained-tab gate uses the real installed Lean fixture, a fresh analysis store and the exact deployed executable. It requires no new inspections while switching already-open tabs, preserves the source/camera and enforces a 250 ms two-rAF p95 budget:

| Motion | Retained-tab p95 |
| --- | ---: |
| reduce | 30.7 ms |
| no-preference | 28.4 ms |

These installed-tool checks are not cold installation or live-model validation. Initial preparation, navigation transfer and retained-tab responsiveness remain different observations.

## Validation

All 63 frontend unit cases, 124 application and 49 CLI unit cases pass. The 3,000-case map/reference/mixed heading-detail corpora and 15 algorithm performance cases pass. Mixed traces compare every projection with a fresh reconstruction and preserve previous scenes. Candidate-cache cases check original/returned mutation, changed source/method/heading refusal, fresh targets, description exclusion and whole-entry count/byte eviction.

The public 35-journey TUI gate passes with decoded Lake freshness-failure and historical/cancel/restart screens inspected. All 71 browser journeys pass with no skips, flaky or unexpected cases, including the original one-read large-map requirement. Normal-motion runs separately repeat the new held-response/retry/restart journey and the 10,001-file keyboard/focus path. The held bodies are actually released and consumed before checking refusal; no model reply or checker fixture substitutes for the public flow. Scripted inference in gates is not actual-model or cold-install validation. Actual deployment checks retain source/graph/reader/evidence and model bindings, quiet chat, camera and source-line selection, ending idle/unapproved.

```sh
npm run test:fuzz
npm run test:performance
node tools/gui/profile-map-load.mjs '<CURRENT_GUI_URL>' target/gui-profile/new-headings
node tools/gui/profile-interactions.mjs '<CURRENT_GUI_URL>' target/gui-profile/new-events
```

Use the current session and exact matching asset source maps. See [the profiling tools](../../tools/perf/README.md) for observation boundaries.
