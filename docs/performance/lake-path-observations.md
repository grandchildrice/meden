# Reduce repeated prepared-input path work

Prepared Lake views still validate every compiled-input byte. The latest
Riemann inspection spent 15.854 s in preparation despite a semantic-artifact
cache hit; its log separated build/setup/input resolution from hashing. The
Python resolver also traversed every parent of every artifact separately.

The Linux worker now opens an `O_PATH` handle, compares its procfs name with the
original workspace-contained path and checks the opened object's regular-file
type. A final symlink and a symbolic parent alias are refused, including an
alias whose canonical target was already collected. The handle closes on every
observed success/failure. No content is opened by this metadata check.
[Linux open(2)](https://man7.org/linux/man-pages/man2/open.2.html) and
[proc_pid_fd(5)](https://man7.org/linux/man-pages/man5/proc_pid_fd.5.html)
document these descriptor operations.

This replaces the per-artifact Python ancestor walk. There is one production
path and no new dependency, fallback, controller or authority bit. The separate
four-reader, 256-job-batch hashing code still reads every byte, checks original
file metadata before/after reading and hashes ordered path/length/content
records. Its input bounds and `meden.prepared-inputs.v2` representation remain
unchanged. The short metadata handle is not retained into that later read, so
this is not a pinned-snapshot or filesystem-concurrency proof.

## Controlled comparison

The actual confined `profile-prepared-inputs` application entry builds/resolves
Riemann's original `DictGen.lean` and fingerprints all **43,477 files /
5,992,114,707 bytes**. Two tool/input warmups are retained separately. Eight
measured invocations use the fixed order base/candidate/candidate/base/
base/candidate/candidate/base, with no concurrent builds/tests or sampler.
Project/OS caches and installed dependencies remain present.

| Complete preparation invocation | Original ancestor walk | Kernel name observation |
| ------------------------------- | ---------------------: | ----------------------: |
| Median, four samples            |                7.314 s |                 6.313 s |
| Minimum                         |                7.135 s |                 6.123 s |
| Maximum                         |                7.523 s |                 6.647 s |

The candidate's median is 13.7% lower in this workload. Every invocation returns
the exact same fingerprint, file count and byte count. Resource preparation is
measured separately. This is not a paired browser/paint, cold-install, inference
or evidence-admission benchmark and does not predict every project's latency.

## Regression scope

The protocol suite includes 3,000 seeded real-filesystem cases against an
independent strict-resolution oracle: relative/absolute/normalized names,
Unicode, symbolic roots/parents/leaves, missing inputs, loops, directories and
FIFOs. An observed rename or unavailable proc/status read refuses the result
and leaves no descriptor behind. Existing tests retain complete query/EOF/exit
admission, same-size edits with restored timestamps, ordered multi-batch hashing,
concurrent mutation and the file/byte bounds.

A deep-input regression bounds Python filesystem-observation calls independently
of directory depth. Reintroducing the original ancestor walk still passes all
25 selected functional cases, but needs 2,944 calls for 128 inputs and fails the
512-call budget. Removing either the actual-name comparison or regular-file
check fails the independent refusal corpus. These are real filesystem and
worker-protocol tests; scripted Lake replies in this suite are separate from
the installed-tool comparison and public TUI/GUI journeys.

The script is part of the registered compiler's implementation/resources. Its
version change must invalidate former compiler observations; the exact-input
fingerprint's equality is not permission to relabel an old artifact. Public
file/source/graph validation checks original logical contents while allowing
their newly attributed graph/publication revisions. Current accepted-contract
evidence remains owned by its existing controller.

See [the validation record](../../results/lake-path-observations-20261125.json)
for the complete samples, negative controls, public failure/retry/restart
evidence and deployment checks. Broader session/snapshot typing, remaining
native waits and the full retirement audit are still open.

## Actual-project preparation and restart

Both deployed projects use all 88 unchanged browser assets. Their first
preparation under the newly attributed compiler/search resources takes
181.582 s for Riemann and 65.349 s for EthLib. Those complete semantic
preparations are performed before handing the updated GUI back to the user.
After an actual host restart, the selected sources reopen with complete cache
hits in 15.316 s / 2.235 s. These are single functional observations, not a
paired end-to-end speedup comparison. Native spans show no graph-extraction,
search or renderer worker invocation on those cache-hit inspections; Lake
dependency preparation and exact-input validation still run when required.

Every source/graph DTO field matches the original except its newly attributed
graph binding revision. Original readers, inventory, history, candidate
observations, evidence and public connection choices remain unchanged. The
checksummed compiler/search/rendering objects reconstruct each exact displayed
graph revision. Raw compiled graphs and complete input fingerprints are equal;
role implementation/schema metadata remains equal while compiler/search
configuration fingerprints change. The renderer definition stays identical.
After restart, those complete new DTOs and retained object chains are identical
to the freshly prepared versions. No inference approval is restored.

This also exposes remaining coupling: the frozen compiler and searcher currently
share broad resource fingerprints, so a prepared-input helper change causes
new observations even for EthLib's frozen path. Narrowing those resource sets
requires an audit of the actual transitive scripts/compiled modules and original
provenance. It must not relabel old observations or omit executed dependencies.
That partitioning and broader prepared-snapshot reuse remain open work.
