# Project-map reference lookup

The [measurement record](../../results/map-references-20261124.json) compares the former array/filter resolver with one pure indexed resolver. The obsolete runtime loop is removed. An independent copy remains only in tests as an exact ordered-edge oracle and timed counterfactual. The native executable, compiler inputs, world coordinates, semantic observations and model approval are unchanged.

The private, read-only actual EthLib inventory has **2,517 files, 42,857 declarations and 1,329,548 reference occurrences**. Its original publication is 24,209,335 bytes. Both implementations produce exactly **212,283 ordered edges**, SHA-256 `e9cd425a9ebf8d032482e139a8a97cddceac63b52e1aa5a7eaaf381195f5a5f8`. The original input is unchanged after resolution. Raw inventory and captures remain local; only numeric metadata, hashes and validation results are committed.

## What changed

The original algorithm reconstructed namespace candidates and filtered all same-name declarations for every reference. Public and file-private uniqueness indices now distinguish a sole target from ambiguous names. A cache for each exact file/namespace context reuses both resolved targets and misses during this one immutable graph construction. It caches resolution before suppressing a source’s self-edge, so another declaration in that scope can still target it. Namespace keys include the trailing separator, preserving leading-dot and empty-namespace distinctions. No resolution is reused across graph observations.

Nearest-namespace search, qualified names, same-file fallback, private visibility, ambiguity, cycles and ordered deduplication retain the original rules. These are lexical display links, with no proof, elaboration or approval authority. The module takes only structural symbol data; it has no browser, compiler, transport, clock or storage dependency. No worker, WASM backend, renderer or package dependency is added.

## Measurements

Nine alternating paired observations after checking complete ordered-edge equality:

| Workload                        | Original median | Indexed median | Ratio |
| ------------------------------- | --------------: | -------------: | ----: |
| Complete actual EthLib          |     2230.728 ms |     680.667 ms | 0.305 |
| Shared-name regression workload |      407.812 ms |      11.896 ms | 0.029 |

The first uniqueness-index prototype took 1040.963 ms on the actual corpus; adding file/namespace-local reuse reduced repeated candidate construction further. These are resolver-only timings, excluding parsing, scene construction, native work, rendering and storage.

The public performance gate requires the repeated synthetic ratio below 0.65. Forcing the former scan into the indexed arm is rejected (ratio 1.001). Both arms still consume original nonempty output; the full input is retained. Differential scope tests cover ordinary/qualified/Unicode/leading-dot names, private/global ambiguity, cached self references, cycles and misses. `npm run test:fuzz` exercises 3,000 generated reference scopes in addition to the existing map corpus.

## Complete browser navigation

Five complete navigations in a fresh context per version/project include transfer, decoding, validation, geometry and initial rendering. Actual cache-source attributes classify visits. Native host, project and OS caches remain warm; a one-second idle-storage interval between visits is outside readiness. Values below are two-rAF medians for visits 2–5, with CPU sampling and automation overhead. They are not physical presentation or field percentiles.

| Project |    Before |     After |
| ------- | --------: | --------: |
| riemann | 1436.5 ms | 1306.9 ms |
| ethlib  | 3296.7 ms | 1844.8 ms |

Riemann’s initial inventory has no loaded declarations, so reference lookup is not its dominant startup cost. Its complete inventory parsing/materialization and later React reconciliation remain separate measured work. Native Lake/query and exact input verification also retain their original checks. Source/graph/reader/inventory/history/evidence bindings and camera/source-line/chat behavior were rechecked after deployment, with fresh unapproved model state.

## Validation and reproduction

The public 35-journey TUI gate and all 70 real browser journeys pass on the fixed native executable. Decoded Lake freshness-failure and historical/restart screens were inspected. All 54 unit cases, the generated map/reference corpora, 15 performance cases, architecture/format/schema checks and the expected-failing scan mutation are retained. Fixtures use actual offline workers and installed tools with scripted inference; they do not validate model quality or a cold installation.

```sh
npm run test:fuzz
npm run test:performance
MEDEN_PERF_MUTANT=scan-references npm run test:performance # must fail
MEDEN_REFERENCE_WORKLOAD=/private/original-inventory.json \
  MEDEN_PERF_OUT=target/performance/private-references \
  npx vitest run apps/web/src/features/native/project-map-references.test.ts \
  --maxWorkers=1 --minWorkers=1
```

The private-workload option validates a complete original inventory publication through the generated schema. It runs the actual scene projection, compares all original edges, retains the output hash and verifies input immutability. It is separate from the CI workload and makes no inference requests. Browser capture commands and their observation limits are documented in [the event audit](map-events.md) and [profiling tools](../../tools/perf/README.md).
