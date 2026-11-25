# Typed source, graph and prepared-input snapshots

Baseline: `1c9ebce`. This is another boundary in the
[typed-core/plugin migration](../architecture/typed-core-and-plugins.md).
It is not a new rendering or input-validation speedup.

The application owns `SourceRevision`, `GraphRevision` and `PreparedRevision`
with private representations, explicit original-digest constructors, canonical
decoding and explicit transport projections. Source/graph spellings retain the
original `sha256:` prefix; prepared-input fingerprints retain bare lowercase
hexadecimal. JSON spelling and the original serialization/hash framing do not
change. The lexical graph still intentionally projects its original source hash
into the distinct graph context. There is no implicit conversion, permissive
fallback, authority bit or kernel/controller change.

These values now cross actual source capture, compiler dispatch/cache restoration,
search and rendering revision composition, semantic requests, journals and source
view publication. Complete content/method/freshness checks still apply; decoding
does not establish the identity's origin or corresponding body. The former
display-side source/graph string validators are removed. Invalid record coverage
now uses an invalid target line because malformed source IDs are refused earlier,
at decoding; separate identity tests cover that refusal explicitly.

Prepared Lake fingerprint observation has one typed mode/result boundary.
Compilation, search and semantic freshness consume its original `PreparedRevision`
rather than repeating JSON field extraction. The original module order, query
arguments, preparation/fingerprint deadlines, read-only validation profile and
complete-input byte checks remain. Malformed worker digests fail before a typed
result is returned. The worker still emits its original complete diagnostic data.

The compiler and search schemas now declare the actual canonical identity
constraints, and compiler/search implementation capture includes the identity source.
Their definitions therefore change. Original compiler/search artifacts remain
bound to their original descriptors; no old object is relabeled as a new method.
The new implementation must prepare a fresh complete observation once, with
subsequent exact reuse. Physical Lean/GF/Plausible resources are unchanged.
Prepared-input/module identities, source bytes and raw graph data are compared
independently of the legitimately changed bound graph/view revisions.
New bounded-search results are observations of a new run, rather than immutable
compiler output; their exact differences are retained below.

Generated tests retain all original digest bytes, source JSON, lexical graph
framing and source-freshness refusal. Compile examples reject graph/source and
prepared/graph exchanges and unchecked construction; a matching snapshot getter
compiles. Native output schemas and browser validators remain unchanged because
their wire fields already declared these constraints. JSON remains structurally
typed: a syntactically valid foreign revision still needs the host comparison.

The strengthened native meaning journey submits 21 malformed identity bodies
through the actual HTTP endpoint, observes no source replacement or model request,
then follows the original UI analysis/restart journey. Its initial driver read
the globally latest source instead of the selected file and assumed compiled
node IDs in a lexical graph. It now uses the selected file's retained publication
and original declaration name. An initial parallel auxiliary run also collided
on the fixed preview port; that startup failure is not a negative control. The
full gate and original-executable control run separately.

## Validation and deployed observations

The [complete record](../../results/snapshot-contexts-20261125.json) binds the
final executable to its source hashes. All 382 Rust checks, two generated
properties with 3,000 cases each, Clippy, schema checks and both format checks
passed. The required public TUI gate passed all 35 journeys; decoded review,
judge, GUI-to-TUI resume, Lake failure, repair and cancellation screens were
inspected. The final executable passed all 76 native browser journeys without
skips or flaky cases. The original-executable negative control reached the
actual malformed-request assertion and returned HTTP 202 instead of 409.
An earlier interrupted 75-case run and the earlier executable's complete
recovery remain diagnostics, not final gate results.

Both actual project services run the validated executable. All 88 served
browser files remain byte-identical. Original source bytes, raw compiler graph,
prepared-input fingerprint, resource binding, reader, inventory, navigation,
history and accepted evidence remain equal to the previous deployment.
Compiler/search definitions and their observation/search/rendering IDs change;
old content-addressed objects remain intact with valid checksums.

The first real Riemann comparison correctly failed full semantic equality.
The new search run changes two declaration observations: `natCast_cancel`
changes from unsupported to budget exhausted, and `mGen_of` retains budget
exhaustion with a different diagnostic. Their complete original/current
observations remain in the record; all other declaration data is compared
exactly. No old verdict is copied into the new observation. The subsequent
public UI recovery and actual host restart preserve the entire newly prepared
publication exactly, including those results. An initial recovery mistakenly
expected another cache miss; that driver failure is also retained separately.

Initial native preparation takes 211.438 s in Riemann and 51.061 s in EthLib
in the concurrent two-project functional journeys. Riemann's search span alone
takes 121.573 s. After an actual restart, cache-hit preparation takes 11.830 s
and 0.928 s respectively, with no compilation, graph extraction, search or
meaning-rendering worker. Public click-to-ready observations are 14.297 s and
3.215 s. These are single functional observations with nested spans and shared
host caches, not paired performance comparisons. Complete compiled-input
validation remains a separate cost on the cached path.

This is application type/adapter validation, not additional Lean refinement or a
proof of native plugin behavior. The complete multi-backend compiler observation
shape, remaining raw orchestration/artifact identities, native preparation wait
and retirement audit remain open. Installed tools/scripted inference do not
establish paid/live-model quality, cold installation or large-model deployment.
