# Prepared-input reader count: keep four readers

The [measurement record](../../results/lake-reader-count-20261126.json) separates
Lake's query/build work from complete input validation. It follows the earlier
[four/eight-reader comparison](graph-publication-camera.md#preserve-complete-input-validation-and-the-current-reader-bound),
which rejected eight readers. This comparison rejects one and two readers too;
production code, resource identities, caches and deployed services are unchanged.

The actual confined `profile-prepared-inputs` application diagnostic prepares
Riemann's original `DictGen.lean`. Three pinned executables differ only in the
metadata helper's hash-reader constant. All twelve invocations read the same
**43,477 files / 5,992,114,707 bytes** and return the exact same fingerprint,
using the same installed Lean, public Python and compute resources. No graph,
model request, accepted contract or evidence is produced by the diagnostic.

The first invocation of each variant is a retained warmup. Nine measured
invocations use reader counts **4, 2, 1, 1, 2, 4, 2, 4, 1**. There are no
concurrent builds, tests or external samplers during measurement. Project/OS
caches remain installed; ordinary desktop background work is not controlled.

| Median, three measured invocations per variant | Four readers | Two readers | One reader |
| ---------------------------------------------- | -----------: | ----------: | ---------: |
| Complete preparation                           |      6.703 s |     7.376 s |    9.804 s |
| Lake query/build/setup worker                  |      3.329 s |     3.299 s |    3.324 s |
| Metadata/content worker                        |      3.188 s |     4.006 s |    6.363 s |
| Input resolution inside metadata worker        |      0.568 s |     0.558 s |    0.561 s |
| Complete hashing inside metadata worker        |      2.451 s |     3.186 s |    5.592 s |

Worker intervals include dispatch, confinement and output admission. The last
two rows are rounded monotonic observations inside that worker; they are nested,
not additional costs. Each column contains independently calculated medians,
which must not be added or subtracted to attribute exclusive work. Host calling
thread CPU excludes the Lake/Python child processes. No kernel-wait or physical
paint attribution follows from it.

The first four-reader warmup takes 15.020 s: query 8.963 s and metadata/content
5.997 s. Later measured four-reader invocations take 6.549–6.746 s. A single
before/after observation could therefore falsely suggest a large improvement.
The warmed diagnostic is also different from a GUI file request: its 6.703 s
median is **not** a speedup against the earlier 20.219 s native extraction or
11–12 s request-to-prepared measurements.

The current four-reader workload spends roughly comparable intervals in Lake
and metadata/content validation. Reducing reader concurrency makes the latter
slower while leaving Lake query medians similar. More readers were already
rejected separately. This does not prove four readers optimal on every host or
input set, nor identify every source of first-run variation.

Every compiled byte is still read and hashed in ordered, bounded batches, with
original regular-file, symlink, size, concurrent-change, cancellation and worker
admission checks. A cached graph is not current evidence. Skipping Lake based
only on an old input list would need to establish that configuration, discovery
and the complete dependency closure remain unchanged; this experiment does not
establish that condition or introduce such a cache.

The required public TUI gate and input-worker protocol/refusal corpus are run
separately after measurement. Decoded connection, Lake inventory-change,
failure/recovery, cancellation and restart screens are inspected. These use
installed tools and scripted providers; they are not cold-install, live-model,
large-model or confidentiality validation. Full gate results and inspected
screens are linked in the numeric record.

The sampled cancellation frame still includes a checking spinner, and some
composer cells retain decoder fragments. Passing the journey does not establish
that every captured frame is settled or that terminal rendering is authentic.

The remaining targets are Lake preparation overhead, complete input validation
and the independently measured first-detail rendering stall. Changing the
reader count does not resolve them or finish the typed-core migration.
