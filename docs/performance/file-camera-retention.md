# Camera continuity through source preparation

A real source edit followed by **Refresh files and map** and file reinspection
reproduces a dependency-graph camera reset on the previously deployed assets.
The installed Lean compiler finishes successfully and the source revision changes;
only then does the camera assertion fail. The viewport changes from
`translate(205.656px, 259.947px) scale(0.50977)` to
`translate(205.433px, 222.511px) scale(0.336323)`.

The file workspace formerly keyed its compiled graph by the source/graph
revision. A new revision recreated the viewer and ran automatic fitting. Loading
and lexical/compiled transitions could also replace that viewer. The camera now
belongs to the retained file tab: only a user-adjusted viewport and logical
declaration/focus names survive viewer replacement. Initial automatic fitting
does not create a saved camera over the first compiled layout. The new viewer
restores that camera when the logical center still exists. An explicit new focus
or missing center selects a valid declaration and permits fitting. Changed declaration arrays alone no
longer discard a manually chosen center. The obsolete revision key is removed.

This does not retain an old backend observation, authorize inference, bypass
input freshness checks or reuse old verification evidence. No native/controller,
parser, compiler or wire format changes. The saved camera lasts only for the
file tab's lifetime.

The public regression creates and compiles a real Lean file, zooms with actual
wheel input, changes its definition on disk, refreshes through the explorer and
reopens it. It requires the changed source revision and visible source, unchanged
camera, no model requests and unapproved model state. The control serves the
original immutable assets with the same native executable and fails at camera
retention after successful compilation/reinspection.

This is a reproducible source-refresh reset, not proof that every pointer-only
reset has the same cause. The separate [mixed-wheel regression](camera-motion.md)
covers pointer crossings. It also does not reduce the independently measured
[first-detail rendering stalls or complete-input validation costs](event-audit-20261126.md).

## Validation

The [numeric record](../../results/file-camera-retention-20261126.json) retains
both motion modes, the old-assets control and exact executable/assets hashes.
Eight Lean/source/map journeys and the actual finite/Z3 source journey pass,
with no skips or flaky cases. Mixed ordinary/Control-wheel gestures retain one
viewport across 339 normal / 365 reduced-motion frames, 15 node entries each and
a maximum pointer-anchor error of 0.02547 px. Retained-tab two-animation-callback
p95 is 30.5 / 25.6 ms, below the existing 250 ms gate; this is a fixture scheduling
proxy, not physical presentation, actual-project latency or an achieved speedup.

All 37 required public TUI journeys pass on the unchanged native executable.
Decoded connection review, clipped repair authority, cancellation, source
failure/recovery and restart screens were inspected, as were the identical
normal/reduced camera screenshots. TypeScript, architecture, formatting and
Python lint checks pass. Installed-tool/scripted fixtures are not live-model or
cold-install validation. The initial browser batch driver exited 143 after
both complete positive reports. All nine positive journeys have complete passing
reports, and the remaining gates ran in a separate completed driver.

Only web assets change in the actual workspace services. All 88 served assets
match the new immutable deployment. Both actual projects retain their complete
source, reader, inventory, map-detail and evidence DTOs, conversations and
candidate history; file preparation hits its cache without compiler, search or
renderer execution. Both hosts finish idle and unapproved. Screenshots of their
source/graph views were inspected. This camera fix does
not complete the broader generic layout, metadata orchestration, observation
correspondence or native input-validation work.
