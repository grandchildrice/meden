# Versioned L4SB cone layout

The dependency cone now has one explicit `l4sb.cone.v1` definition, retaining
Lean4SemanticBench's MIT-licensed layout at commit
`97e7c29df8d3c358c395bd1a4f2fb6973527b33d`. This is a correctness and
reproducibility change, not a measured rendering or compilation speedup. The
[event audit](event-audit-20261125.md) still identifies deep-zoom React/SVG work,
global reference rebuilding and native input validation as separate costs.
The [validation record](../../results/cone-layout-20261125.json) retains the
before/after comparison, original-production negative control and public gates.

The public recorded-workspace importer previously accepted declarations named
`__proto__` and `constructor`, then displayed no cone because an inherited
object property was treated as an adjacency array. The browser reported
`(i[o] ?? i[o]).push is not a function`. Null-prototype dictionaries now own
declaration-keyed adjacency, levels, parents, positions, overflow and angles.
Original identifiers remain unchanged; no special-name rewriting or fallback
layout is introduced. The upstream license and local adaptation notice remain.

The exported frozen definition supplies the actual numeric settings, upstream
identity, one-hop scope, cone angles and coordinate/card scaling. Edge input
order, ECMAScript own-key enumeration (including numeric keys), and UTF-16
lexical barycentric ties preserve the original order. The algorithm reads no
locale, clock, random source or font metrics. The existing cone element exposes
the version through an invisible data attribute; there are no new visible
controls or annotations. There is no persistent cone cache to migrate. A future
cache must bind original graph/root inputs and the complete definition.

Before editing the implementation, an independent driver captured 512 finite
ordinary graphs at 3,146 roots and the original prepared Riemann/EthLib graphs
at 57 roots. Same-process comparison preserves every coordinate and ordered
entry exactly. A small checked-in six-graph golden covers ordinary behavior;
its cross-engine floating-point comparison uses a twelve-decimal tolerance.
These are different comparisons: the large before/after hashes require exact
equality. Tests also cover all twelve own property names of `Object.prototype`
and a 3,000-case shrinkable identifier/cycle corpus. The original implementation
passes the ordinary golden but fails thirteen regression tests with the actual
dictionary exception; this negative control is separate from test setup errors.

Scope and pruning remain upstream behavior. In particular, the existing
longest-path treatment of self loops can prune a neighbor; this change does not
claim complete reachability or change ordinary coordinates to fix it. The Lean
extractor also continues to exclude `Name.isInternal`, so the real compiled
journey uses `constructor` and `hasOwnProperty`; imported graphs separately
exercise `__proto__`. No compiler filter or captured compiler resource changes.

The public journeys import original graphs, inspect unknown proof status,
retain camera and independent chat drafts, and reopen saved conversations. The
native journey uses real installed Lean, original source-line navigation,
compiler rejection/recovery and an unapproved cached restart. These checks are
installed-tool adapter evidence, not live-model quality, cold installation,
new proof authority or whole-renderer refinement.

Validation passed 78 unit tests, 22 fuzz tests (including the 3,000-case cone
corpus), the existing 15 algorithm checks, all 35 required public TUI journeys
and all 73 GUI journeys. Three focused normal-motion cases also passed.
Serialized retained-tab two-rAF p95 was 31.9 ms with reduced motion and 36.5 ms
with normal motion, below the unchanged 250 ms scenario budget. This proxy is
not physical paint or INP. Decoded TUI failure/stop/restart screens and the
native source/graph screenshot were inspected.

Both deployed projects retain their original complete source/graph/reader,
inventory, history, candidate/evidence and connection observations. The native
binary and all thirteen captured compiler sources are unchanged; all 88 served
asset files match the validated build and matching hidden-source-map build.
The hosts end idle and unapproved. Single post-deployment cache-hit file opens
took 18.197 s in Riemann and 2.227 s in EthLib. Riemann's nested spans contain
16.577 s extraction and 13.597 s Lake preparation, including 13.102 s in its
worker process. These overlapping intervals must not be added. They retain
evidence of significant backend preparation waits; this unpaired recheck is
not a regression or speedup measurement. The separate repeated event audit
remains the comparison workload for prioritizing further optimization.
