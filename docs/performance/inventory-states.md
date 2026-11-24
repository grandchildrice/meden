# Disjoint inventory observations

The [measurement record](../../results/inventory-states-20261124.json) compares the immediately preceding heading/detail split with the current served inventory. It preserves complete original file metadata, declaration headings, detail bindings, source/graph/reader/evidence data and world coordinates.

## Ownership and representation

An unindexed file previously repeated empty digests, imports, counts and heading arrays. The application now publishes either `unindexed` with the original path/byte count, or `indexed` with the complete syntax metadata and headings. The original complete input is validated before projection; contradictory unindexed navigation is refused. Indexed-but-empty remains distinct from not indexed. A view tag grants no compilation or proof authority. The old boolean/empty-field overview shape is deleted, with no compatibility decoder. Complete navigation chunks keep their separate original record shape.

The constructor borrows validated paths, imports and headings only while serializing frozen owned bytes. It no longer clones every path/name into another temporary overview. The browser consumes the disjoint state directly, without expanding every unindexed entry back into empty arrays. Both schemas/validators come from the same Rust DTO. Geometry derives exactly the old weights; cached world coordinates and lexical candidate reuse remain independent of this representation.

## Actual transfer and navigation

Private complete before/after bodies remain local. Their hashes and a full reconstruction comparison are recorded; every original header metadata field is retained or fixed by the unindexed invariant. Original navigation/indexer bindings are unchanged.

| Project | Files | Unindexed | Before | After |
| --- | ---: | ---: | ---: | ---: |
| riemann | 175,187 | 175,184 | 36.229 MB | 25.018 MB |
| ethlib | 2,517 | 0 | 7.301 MB | 7.308 MB |

Five navigations use one fresh browser context per version/project, Chrome 149.0.7827.53, 1440×960, normal motion, no throttling and CPU sampling. Host/project/OS caches remain warm. Visits 2–5 navigation-relative two-rAF medians:

| Project | Before | After |
| --- | ---: | ---: |
| riemann | 1283.7 ms | 1129.2 ms |
| ethlib | 738.0 ms | 725.8 ms |

This includes inventory transfer, JSON/runtime validation, index/geometry work and initial rendering. The first and retained geometry states are labelled from the actual DOM; a one-second storage-settling interval is outside readiness. No file compilation or inference is requested. These diagnostic observations are not field percentiles or a cold-install benchmark.

## Native publication and remaining preparation

First actual host materialization spans include input validation, detail fingerprinting and overview serialization:

| Project | Before | After |
| --- | ---: | ---: |
| riemann | 1210.2 ms | 772.8 ms |
| ethlib | 985.4 ms | 1001.0 ms |

The before spans come from the preceding deployed executable, the after spans from current startup. They are not an interleaved constructor benchmark. Current-thread CPU is retained separately and excludes workers. Compiler preparation and complete input validation remain unchanged:

| Project | Captured file setup | Cache hit |
| --- | ---: | --- |
| riemann | 12.31 s | true |
| ethlib | 2.14 s | true |

Riemann setup was 22.45 s in the preceding capture despite the same compiler implementation and inputs. This uncontrolled preparation variation is not attributed to the inventory change.

Riemann still validates 43,477 compiled inputs / 5,992,114,707 bytes. The actual preparation log records 4.887 s for build/setup/input resolution and 2.649 s for byte validation. Nested spans are not independent durations. A compiler cache hit and an already-open retained tab are separate states. Large path-vector construction and remaining deep-zoom DOM work also remain open.

## Events and retained tabs

Inclusive synchronous EventDispatch p95, then input/two-rAF p95 in parentheses, milliseconds. Complete distributions and raw trace/profile digests are retained:

| Action | Riemann | EthLib |
| --- | ---: | ---: |
| Map zoom | 4.343 (26.7) | 5.981 (18.8) |
| Graph zoom | 1.175 (21.8) | 0.598 (18.1) |
| Hover | 0.061 (23.1) | 0.087 (21.1) |
| Pan | 0.304 (22.4) | 0.402 (20.0) |
| Resize | 1.641 (24.4) | 1.070 (18.9) |
| Tab click | 14.275 (36.5) | 11.320 (27.0) |
| Distinct-node click | 17.473 (28.2) | 9.635 (12.1) |
| Fit View | 0.916 (24.6) | 0.747 (28.5) |

Deep-zoom maximum long tasks were 240.0 ms for Riemann and 82.0 ms for EthLib. This wire/publication change does not claim to eliminate later rendering work. The serialized real-Lean retained-tab gate preserves source/camera, requires no new inspections and keeps a 250 ms two-rAF p95 budget:

| Motion | Retained-tab p95 |
| --- | ---: |
| reduce | 34.6 ms |
| no-preference | 34.1 ms |

## Validation

All 126 application and 49 CLI unit cases, 63 frontend unit cases, 3,000-case map/reference/mixed-state corpora and 15 algorithm performance cases pass. Rust cases reject contradictory complete inputs before projection, distinguish empty indexing, preserve frozen old bytes and retain arbitrary u64 byte counts, including Unicode/newline paths. Generated runtime validators refuse the old shape and mixed/unknown/incomplete states. Mixed update traces still agree with fresh full reconstruction without mutating earlier scenes. The complete actual EthLib round trip preserves all 212,283 ordered reference edges, SHA-256 `e9cd425a9ebf8d032482e139a8a97cddceac63b52e1aa5a7eaaf381195f5a5f8`.

All 35 public TUI and 71 browser journeys pass with no skips/flaky/unexpected cases. Decoded Lake freshness-failure and historical/cancel/restart screens were inspected. The actual 10,001-file journey checks the compact complete unindexed state, retains keyboard focus/camera/chat through lazy details, recovers invalid input with public Refresh and reopens unapproved. Normal-motion runs repeat that and the held detail failure/retry/old-generation journey. Both serialized retained-tab budgets pass separately. Actual deployed projects retain original source/graph/reader/evidence/model bindings and end idle/unapproved. Compiler captured-source hashes remain unchanged. These installed-tool/scripted-inference checks do not establish live-model quality, local inference or cold installation.

```sh
npm run test:fuzz
npm run test:performance
node tools/gui/profile-map-load.mjs '<CURRENT_GUI_URL>' target/gui-profile/new-states
node tools/gui/profile-interactions.mjs '<CURRENT_GUI_URL>' target/gui-profile/new-events
```

Use the current session, idle hosts and exact matching asset source maps. See [the measurement tools](../../tools/perf/README.md). Broader architecture-plan work remains separate.
