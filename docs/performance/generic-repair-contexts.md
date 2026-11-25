# Backend-owned reviewed repairs

The shared repair queue no longer imports Lean snapshots, configuration records,
resource discovery or declaration relocation. A backend supplies a typed immutable
repair context and a refreshed target. The same existing review, file-policy,
single-use dispatch and reassessment service serve both Lean and finite-data
conformance. Native GUI/TUI composition still selects Lean for this workflow;
heterogeneous host selection remains open.

`Pending<G>`, `Queue<G>` and `Applied<G>` retain the backend's context type.
The frozen request owns the original policy, source body, target, requirement and
accepted-contract reference. `Applied` exposes no mutable intent: its consuming
reassessment checks the original file, declaration name, requirement and graph
mode before requesting an assessment. Plain source preserves the original line;
compiled Lean relocates only the unique original declaration. Preparation also
checks the complete original body/context around the model reply. No new
approval, file executor or evidence controller is introduced.

Lean's context now retains the complete original collection, including its
prepared-input fingerprint. Previously the repair path checked selected sources,
configuration and compiler resources, omitting dependencies outside the selected
source snapshot. An actual installed Lean/Lake probe changes only `Base.lean`
while the selected `Entry.lean`, metadata and resources remain identical. The old
observations still pass; the new context refuses the changed prepared dependency.
The fixture supplies a stable original Lake manifest. Two preliminary harness
failures (missing clean-launch dispatch and a newly generated manifest) are
retained separately and do not count as successful validation.

A second separation removes composition facades from compiler/search method
attribution. Snapshot data, compiler implementation and compiler attribution now
have separate modules. A repair-only change and a facade-only comment must not
force another compile, counterexample search or English rendering. The new
native browser journey builds two real executables with those changes, validates
inputs normally, and requires the original complete source view and immutable
artifact files after restart. The original metadata-helper update journey uses
the same fixed test builder. There is no runtime override for method identities.

This attribution change itself changes the compiler/search definitions once.
Existing artifacts remain intact; a first opening prepares a new bound chain.
Saved openings and restart must then reuse that exact new chain without semantic
workers. The new Riemann search differs in two saved diagnostics: one changes
from unsupported to wall-clock budget exhausted, and one retains budget
exhaustion with the file-budget diagnostic. Neither supplies a counterexample
verdict. Complete before/after values are retained; these observations are not
reported as identical search results. Formal source/graph data remain identical.
The attribution separation prevents unrelated future updates from invalidating
those objects; it does not remove mandatory input freshness validation or claim
faster Lean execution.

The finite-data conformance tests use typed transitions, the actual grammar/data
validator, confined file workers and real offline Z3. The broken model is refuted,
the reviewed replacement applies once, and the fixed model produces a bounded
solver result. No native project evidence is admitted by this diagnostic. Other
cases refuse changed source/dependency/contract, denied policy, cancellation,
foreign review, rejection, replay and restart; attempted replacement of the
reviewed requirement stops reassessment without another model request and retains
the applied edit. Model replies are scripted, so these are orchestration checks,
not model-quality, cloud, local-model or cold-install evidence.

All 286 Rust unit/doc tests, 45 architecture checks, 12 source/Lake checks,
36 required public TUI journeys and 77 browser journeys pass. The browser gate
has no skips or flaky outcomes. Eight decoded TUI journeys and the relevant
browser screens were inspected. Clippy, schema consistency, formatting and
Python lint pass; all 88 browser assets remain byte-identical.

Validation logs and actual deployed timings are recorded in the
[machine-readable record](../../results/generic-repair-contexts-20261125.json).
Public TUI review, failure/recovery and restart remain part of the required UX
gate, with decoded screens inspected. Complete selected-project source, reader,
inventory, navigation, evidence, history and camera comparisons complement the
native update regression. Timing spans overlap: parent and child durations must
not be added as independent costs.

No core/kernel transition, runtime worker protocol, public schema, frontend
source or production browser asset changes are part of this slice. Remaining
work includes native heterogeneous preparation, prepared-module ownership, full
input hashing/Lake waits, browser detail stalls and the retirement audit. The
five-step migration remains active.

## Deployed observations

Both services run the gated executable with the unchanged assets. The actual
selected Riemann `DictGen.lean` and EthLib gossip `Parameters.lean` retain their
complete formal source/graph, original reader, inventory, navigation, evidence,
conversation and candidate history. Node selection reaches the original source
line, preserves map/graph cameras and leaves chat unchanged. Saved/restarted
openings reuse the exact new source view and artifact chain, including the new
search observations; neither runs compilation, search or rendering. Original
immutable cache objects remain intact. Both hosts finish idle and unapproved.

| File click to observed ready | First new-method preparation | Saved analysis | After actual restart |
| --------------------------- | ---------------------------: | -------------: | -------------------: |
| Riemann                     |                    196.691 s |       10.991 s |             11.285 s |
| EthLib                      |                     50.365 s |        1.965 s |              2.227 s |

These are individual functional observations at 1440 × 960, normal motion and
no throttling, without another gate, build or sampler running. Readiness is the
observed native outcome; graph/source/camera assertions follow it. It is not a
physical-presentation or percentile measurement, and it is not an old/new
controlled performance comparison. No actual-project model request is sent.

Riemann's initial native extraction spans 194.557 s, including an actual
23.385 s compiler extraction, 121.563 s counterexample observation and 9.730 s
reading generation. Three query observations take 9.823, 8.138 and 3.996 s;
three fingerprint observations take 6.456, 3.825 and 3.499 s. These are nested
observations, not additive independent costs. A saved opening still revalidates
its original input. The remaining roughly 11-second Riemann cache-hit wait is
not caused by another semantic worker and is not fixed by this extraction.
Native full-input work remains a separate optimization target.

EthLib's initial compiler extraction takes 0.811 s, its search takes 16.812 s
and reading generation takes 17.298 s. Its saved/restarted openings execute none
of those workers. Riemann's saved opening separately records a 3.970 s query and
a 3.744 s fingerprint observation, identifying substantial retained input work
inside its roughly 11-second total. Worker CPU is not included in the host
thread CPU field.
