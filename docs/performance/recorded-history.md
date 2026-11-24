# Complete portable history loading

The [validation record](../../results/recorded-history-codec-20261124.json)
compares the original numeric JSON-array snapshot reader with the existing file
worker's encoded snapshot mode. The shared loader is the only production change;
no core, runtime, file-worker, compiler, model-transport or schema implementation changes.
The original 4 MiB document bound and strict document/linkage validation remain.
The complete worker reply now fits an 8 MiB bound.

The workload is a private, read-only copy of actual EthLib records: 4,137,646
original bytes, six tasks, 91 events and 68 conversations. Both implementations
use the same pinned, actual confined worker. Each receives a warm-up, followed by
eight interleaved ABBA/ABBA observations, four per condition. Every observation
returns the same full canonical document digest and counts. Input hashes are
checked before each invocation and after the sequence, so this deliberately
measures warm bytes. Workspace/tool preparation and result hashing are excluded.

| Median observation | Original reader | Encoded snapshot |
| --- | ---: | ---: |
| Complete record load/decode/validation | 1,595.187 ms | 239.879 ms |
| Loader's current-thread CPU | 961.137 ms | 77.492 ms |
| Nested worker process interval | 648.736 ms | 168.352 ms |

The measured load interval falls by about 85%. Removing the large intermediate
numeric JSON tree reduces both host parsing work and worker reply construction /
transfer. The nested process interval includes more than serialization and these
spans must not be added. Current-thread CPU excludes child processes. Four samples
per condition do not establish a portable latency guarantee, cold installation
or an overall GUI speedup.

A separate actual-CLI resource gate reads exactly 4 MiB of literal Unicode,
quotes, newlines, tabs, NUL data and a JSON-shaped cancellation string, appends a
note and reopens without losing the original content. A trusted host launcher
copies the real worker reply verbatim, preserving its arguments/result/exit code;
it neither fabricates output nor provides an unconfined worker fallback. Its
complete observed reply is 5,592,417 bytes, within the independent 8 MiB budget.
The original reader returns 12,623,124 bytes and must fail that same assertion.
Invalid UTF-8, malformed/oversized records and escaping links remain refused
without rewriting them. This tests a complete input and resource outcome,
allowing other efficient codecs rather than asserting an implementation tag.

The required public TUI and affected native GUI journeys exercise actual local
checking, cancellation/recovery, large-history retry/stale responses and fresh
approval on restart. Their inference is scripted and installed Lean is a
prerequisite. The compiler's captured sources, resources and existing cache
checks are unchanged. Riemann's separately measured long preparation/input
validation wait, broader jobs and versioned layout/order retirement remain open.
