# Actual GUI event and preparation observations

The deployed native executable is pinned by SHA-256 in
[the complete record](../../results/compute-metadata-isolation-20261125.json).
All 88 browser assets match the preceding deployment. These measurements identify
remaining costs; they do not claim a frontend speedup from the backend change.

Headless Chrome 149 runs at 1440 × 960 with normal motion and no throttling. Each project
uses its actual prepared graph, with no concurrent functional gate, Cargo build
or other browser sampler. CDP records synchronous event dispatch and sampled CPU;
two animation frames provide a presentation proxy, not physical presentation or
INP. This is one automated desktop run, not a field latency percentile.

| Action / recorded event           | Samples per project | Riemann dispatch p95 | EthLib dispatch p95 |
| --------------------------------- | ------------------: | -------------------: | ------------------: |
| File graph zoom / wheel           |                  24 |             0.819 ms |            1.115 ms |
| File graph hover / pointermove    |                  40 |             0.101 ms |            0.069 ms |
| File graph pan / pointermove      |                  31 |             0.231 ms |            0.316 ms |
| Retained tab switch / click       |                  24 |            14.984 ms |           10.080 ms |
| Project map ordinary zoom / wheel |                  24 |             3.809 ms |            4.331 ms |

None of those phases records a long task. The resize phase records only four /
two events: observed maximum dispatch is 1.014 / 0.993 ms. Sample counts and all
remaining events, rendering stages, network observations and original trace
paths are retained in the record. Event Timing queue observations cover only
reported discrete events; missing observations are unavailable, not zero.

## The remaining first-detail stall

The separate project-map drill gesture includes only two wheel events per
project. Its maximum dispatch is **115.985 ms for Riemann** and **16.594 ms for
EthLib**; these are observed maxima, not stable p95 estimates. Riemann records
four long tasks, maximum 174 ms, in that multi-action phase. Its wheel-to-two-frame
proxy reaches 162.1 ms. No such long task occurs in EthLib's drill phase.

Original sampled profiles were subsequently resolved using existing hidden
source maps, after verifying **all seven served JavaScript bundles byte-for-byte**
against the immutable deployment. The maps were not supplied by a newer build;
no served asset was replaced. Native/idle/program/GC and automation samples remain
in the report, rather than being assigned to application functions.

Within the actual 115.985 ms Riemann wheel dispatch, sampled self time includes
20.288 ms in `setAttribute`, 12.961 ms in `createElementNS`, 13.914 ms in
`visibleMapAreas` and 9.599 ms in map-area appearance generation. Another
126.852 ms callback in the same phase includes 32.700 ms in `setAttribute`,
25.380 ms in Canvas `stroke` and 9.431 ms in `fill`. Two further callbacks exceed
50 ms and spend about 25–26 ms each in `stroke`. These intervals can overlap and
must not be summed as independent latency.

The source agrees with that attribution:

- [Visibility traversal](../../apps/web/src/features/native/project-map-model.ts)
  starts at roots, enqueues a visible branch's children and tests them against the
  viewport. Dense newly revealed branches can still enumerate many children.
- [Canvas painting](../../apps/web/src/features/native/MapAreaCanvas.tsx) runs in
  a main-thread layout effect and repeats visibility traversal. Its OffscreenCanvas
  is not a dedicated rendering worker. Each visible area receives a fresh
  appearance object and rectangle fill/stroke.
- Revealing new areas also mounts and updates SVG interaction elements through
  React. Those DOM operations occur inside the measured slow wheel/callbacks.
- [Reference resolution](../../apps/web/src/features/native/project-map-references.ts)
  accounts for 81.371 ms of sampled self time over the complete drill phase. It
  iterates current symbols to emit ordered, globally deduplicated edges when
  descriptions change. This is a separate phase-wide contributor; it is not
  attributed to the 115.985 ms wheel interval without matching samples.

The next frontend work should therefore target bounded detail activation and
painting/visibility work, preserving original geometry, ordered edges, source
selection and camera. Ordinary wheel/hover dispatch is a different workload;
backend compilation is not the cause of these recorded synchronous drawing costs.

## The separate prepared-file wait

After actual host restart, both projects reuse their exact complete semantic
artifacts. Concurrent functional fidelity probes observe 12.276 s / 2.232 s from
file click to ready source DOM, including native work and browser scheduling.
The file-local jobs report 10.307 s / 0.865 s. No compiler, graph extraction,
search or rendering worker runs; original source/graph/reader/inventory/history
and evidence data are retained. These are single functional observations, not
a paired old/new speedup comparison.

A separate controlled Riemann preparation comparison measures 3.326 s for the
actual Lake query and 3.033 s for the input-validation worker. The latter still
validates 43,477 files / 5,992,114,707 bytes; its internal complete-content hashing
median is 2.246 s. See [the exact comparison](compute-metadata-isolation.md).
That native input-validation wait and the first-detail drawing stall need
separate fixes. Neither measurement grants new evidence or model authority.
