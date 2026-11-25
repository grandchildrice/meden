# Prepared navigation annotations behind an application port

Navigation enrichment now has a small application-owned annotation contract and
host registry. The GUI asks for annotations through host navigation composition;
it no longer opens a Lean cache or matches compiler records itself. The trusted
control kernel, generated wire shapes, coordinates and compiler identity remain
unchanged.

A registration binds non-overlapping suffixes to a read-only prepared-description
loader. Selection retains the original context and ordered selected paths.
Unsupported formats invoke no loader. Empty or overlapping registrations fail
before changing the registry. A foreign path, invalid source location, incomplete
explanation or failed reader refuses the whole load; it cannot publish a successful
prefix. These are application adapter checks, not new evidence authority or a
proof that arbitrary native plugins are read-only.

Host composition currently registers the existing Lean cache reader. Its complete
source, metadata, resources, compiler, renderer and search checks still execute.
The compiler-captured reader implementation and its original serialized records
are unchanged: moving composition does not require recompiling a prepared project.
Finite source graphs retain their separate registered preparation; this change
does not invent finite navigation explanations or a new search result. Adding
another annotation reader changes host registration rather than the GUI or kernel.

The overlay indexes notes by exact original file, formal name and one-based line.
A later note replaces an earlier note for that location; only the first matching
lexical declaration receives it, preserving the former ordered scan even for
duplicate declarations. It preserves all other names, positions, references,
visibility and lexical metadata. The overlay matches locations; it does not establish
an atomic snapshot across independently observed file bytes and cached notes.
The existing loader freshness checks remain I/O adapter obligations. This removes a repeated full-file and declaration
search for every note. It is not evidence of faster project-map paint or lower
end-to-end latency.

The bounded native metrics stream now separates `navigation.annotations.load`
from `navigation.annotations.overlay` under `navigation.annotations`. Labels
contain no source names, paths or text. Wall and current-thread CPU intervals have
the existing metrics scope and eviction limits; worker execution needs its own
observations. A prepared-description loader remains distinct from compilation,
counterexample search and admitted verification.

One actual two-file restart probe records two loader intervals of 266.204 and
266.191 ms (224.473 and 224.396 ms current-thread CPU), versus 0.018 and 0.020 ms
for overlay. This is a diagnostic fixture observation, not a project-wide
percentile or a controlled before/after speedup. It is consistent with the
current capture and post-entry-preparation refresh calls. Reducing the duplicated
load is the next candidate; original input/resource validation and failure behavior
must remain explicit. The indexed overlay is not the dominant wait in this probe.

After deployment, actual Riemann and EthLib each retain byte-equivalent complete
source, reader, inventory, navigation and evidence payloads. Selecting the original
file reuses its compiler cache, without model approval or semantic-worker execution
spans in either complete native ring. Startup plus file selection records three
loads per project: Riemann 63.401–67.888 ms with 0.003–0.005 ms overlays; EthLib
618.625–708.112 ms with 0.032–0.498 ms overlays. These are single-workload wall
intervals, not a before/after comparison or pure I/O timings. The
[numeric record](../../results/navigation-annotations-20261126.json) retains CPU
intervals, scopes and artifact hashes. Repeated native reads dominate this overlay
on the actual workspaces too.

The generated differential corpus compares complete original overlay records,
including duplicates and missing locations. Its shrunk duplicate-declaration case
is retained as a regression seed. Registry tests cover a second host registration,
exact dispatch, unsupported paths and complete refusal. The native browser journey
uses actual offline Lean/Plausible/Informath observations, checks annotations after
restart against the original source graph, rejects cross-file attachment, changes
the declaration's line, fails real compilation and reopens without stale English
or model approval. Existing large-map annotation and real finite-source journeys
remain relevant. Scripted-provider/installed-tool journeys do not establish
live-model quality, cold installation, local inference or new proof obligations.

This advances navigation-annotation ownership in migration step 4. Generic layout
and source collection, native freshness waits, first-exposure rendering and the
remaining retirement audit are still open in the
[full plan](../architecture/typed-core-and-plugins.md).
