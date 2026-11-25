# Load prepared navigation annotations once per refresh

The workspace inventory collector no longer loads compiler annotations before
entry-module preparation. Refresh captures the lexical inventory, prepares its
existing entry modules, then performs one original annotation load before
publishing the completed map. Per-file indexing retains its independent load.
There is no new cache, expiry policy, memoized success or compatibility path.

The removed call modified fields that the final overlay would populate again;
file selection and entry preparation use paths, not those earlier descriptions.
The surviving loader still checks complete original source/metadata/resource,
compiler, search and renderer bindings. If that final load fails, the map retains
lexical navigation without an earlier successful annotation prefix. No accepted
contract, proof receipt or model approval changes. The independent-capture and
filesystem attribution limits described in
[navigation annotations](navigation-annotations.md) still apply.

## Regression and diagnostic measurement

The public native browser journey checks the actual prepared explanation after
restart against the original source graph, then requires one annotation load for
that refreshed map. The previous installed binary reaches the same correct
source/explanation checks and fails specifically at two loads instead of one.
The updated binary passes and continues through changed source locations, an
actual Lean compilation failure and another restart without stale descriptions
or model approval. Its complete native span attachment is retained before the
count assertion, so the negative control exposes its real work too.

In this one two-file restart probe, old load intervals are 266.416 and 274.205 ms;
the new single load is 274.743 ms. Their combined loader wall time falls from
540.621 to 274.743 ms; current-thread CPU falls from 457.531 to 232.842 ms.
These are diagnostic stage observations from one run per executable, not an
end-to-end page-speed claim or a portable latency percentile. The deterministic
regression condition is the removed redundant operation, with original content
and failure/restart outcomes retained.

After deployment, both actual workspaces retain their complete original source,
reader, inventory, navigation and evidence payloads. Startup plus selection of the
original file now records two annotation loads rather than three: one refresh
load and one per-file indexing load. Both selections reuse their compilation cache,
retain original history and candidates and remain unapproved for inference. Both
native rings have zero eviction and no observed compile/search/render execution
spans. This verifies work removal and retained behavior; the full input validation
and file-indexing costs still remain.

The [numeric validation record](../../results/navigation-refresh-20261126.json)
records the original/candidate binaries, actual-workspace cache/view comparisons,
scoped timings and relevant public GUI/TUI gates. First-exposure rendering,
complete native input validation, generic layout/source collection and the
remaining retirement work are still open under the
[full migration plan](../architecture/typed-core-and-plugins.md).
