# Prepared Lake input measurements

The [calibration record](../../results/lake-preparation-calibration-20261124.json)
separates actual prepared-input work from GUI protocol time and native sampling.
The production compiler, input validation, resource identities and caches are
unchanged. Its executable is byte-identical to the previous validated deployment.

The host diagnostic `profile-prepared-inputs` calls the shared application's real
offline preparation path. It reports resource preparation separately, then
records each invocation's wall time, complete fingerprint/file/byte counts,
bounded original worker output and host spans. It also identifies the selected
public Python object and original resource revision. See the
[command and limits](../../tools/perf/README.md#isolate-prepared-input-work).

For Riemann's selected `DictGen.lean`, every compared implementation read
43,477 files / 5,992,114,707 bytes and returned exactly the same fingerprint.
An eight-run, interleaved baseline/candidate comparison found a median of
7.449 seconds for the original preparation and 8.313 seconds for a prototype
that moved canonical path checks onto the reader pool: about 11.6% slower.
That prototype is rejected and removed. A standalone earlier capture would
have suggested an improvement; it was not a controlled comparison.

A second eight-run comparison used the same native GUI inspection command and
alternated the actual 99 Hz user-space/DWARF sampler off/on:

| Observation                  | Sampler off | Sampler on |
| ---------------------------- | ----------: | ---------: |
| Median protocol-to-idle time |    10.886 s |   11.057 s |
| Median Lake worker span      |     7.586 s |    7.815 s |

The end-to-end median difference was about 1.6% for this workload and environment.
Four samples per condition are diagnostic evidence, not a portable overhead
constant or continuous performance budget. Earlier sequential captures differed
more; their difference cannot be attributed entirely to sampling or a code fix.

The remaining protocol-to-idle time includes other preparation, responses and
100 ms status polling; subtraction does not isolate UI CPU or physical paint.
Native sampling covers user-space host/inherited workers, not kernel stacks.
The largest resolved individual symbol in the earlier capture was Python's
SHA-256 routine (25.53% of samples); many Lean addresses remain unattributed.
Thread-CPU spans exclude child work, and nested spans are not additive.

All operations reused existing project/OS caches and installed resources. Every
GUI observation retained its source publication and conversation revision, with
no inference approval or model request. The new alias regression refuses a
symbolic parent path even when its canonical file is already in the input set.
Actual filesystem concurrency, clocks and observation attribution remain adapters.
Broader job read models, initial preparation waits and layout/order retirement
still remain in the architecture plan; these measurements do not finish them.
