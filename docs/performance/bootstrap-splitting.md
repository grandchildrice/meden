# Reject entry splitting on the local startup workload

The retirement audit found two live browser entries: native workspace sessions
and the separate recorded vending-machine demo. Both are statically imported by
`main.tsx`. Before deleting or separating them, a prototype used React lazy
imports for both entries, with visible loading and reload-based recovery from a
failed module request. It introduced no application, protocol or kernel change.

The prototype is **rejected**, not deployed. The original entry remains. Smaller
downloaded JavaScript did not produce earlier usable controls on this workload.
The [numeric record](../../results/retirement-audit-20261126.json) binds the original
and prototype sources/assets, raw observations and checks.

## Direct native-server comparison

Two instances of the same frozen native executable serve original and prototype
production assets directly. Each has its own empty workspace and data directory;
initial read-only discovery finishes before measurement. No request interception,
model approval, inference, compiler, source graph or sampled CPU profiler is used.
Each mode runs original → prototype → prototype → original twice, with a fresh
Chrome 149 process/context on every load, normal motion, 1440 × 960, without
throttling. No build or test runs concurrently with those sixteen observations.

| Mode     | Original JS bytes | Prototype JS bytes | Original ready median | Prototype ready median |
| -------- | ----------------: | -----------------: | --------------------: | ---------------------: |
| Native   |         2,084,139 |          1,838,444 |            400.350 ms |             507.600 ms |
| Recorded |         2,084,139 |          1,370,494 |            367.750 ms |             443.400 ms |

Ready means the first DOM observation of the native model-picker button or the
recorded welcome heading. It is not physical paint, complete API readiness, LCP,
a large-project startup measurement or a field percentile. Counts are complete
uncompressed JavaScript response bytes through that observation, not compressed
network transfer. Browser execution metrics and every individual observation
remain in the numeric record; they are not added to overlapping wall intervals.

Native readiness increases about 26.8% and recorded readiness about 20.6%, despite
11.8% and 34.2% fewer JavaScript bytes. Recorded script duration falls, while
ready controls arrive later. Additional module-loading stages are a plausible
explanation, not a measured causal attribution of the entire difference. This
small desktop comparison does not establish behavior under other network or
cache conditions.

An earlier sixteen-observation comparison substitutes static responses while
preserving native security headers. Its similarly slower result is retained
separately; the direct-server comparison checks that substitution is not required
to observe the regression. An initial diagnostic wrongly required zero POSTs:
the existing first native visit performs the read-only `inspect` of `.`. That
preflight failed, and a later diagnostic failed during route teardown. Neither
incomplete run is counted as a completed comparison. Complete runs allow only
that exact directory-inspection request and retain its body. No model work runs.

Two prototype-only public browser cases exercise failed native/recorded module
loads, visible Retry, conversation restart and fresh model approval. They pass,
but their entry/failure UI is archived with the rejected source rather than
retained as an unused production alternative. The unchanged production entry's
own missing-assets, native restart and recorded failure/recovery journeys are
validated separately.

## Accepted cleanup

The source audit instead removes ten unreferenced standalone native status/index/reference
validators and their JSON schemas, plus their generator/export selections. The
same models remain embedded in the actual session-state schema and runtime
validator. The status-negative test now exercises that real nested boundary.
All retained schemas and all 88 rebuilt production assets are byte-identical to
the original build. This deletes duplicate generated material; it is not a
browser-latency improvement and needs no GUI restart or artifact invalidation.
