# Typed compiler, search and rendering cache artifacts

Baseline: `df202dc`. This advances the identity audit in the
[typed-core/plugin migration](../architecture/typed-core-and-plugins.md).
It is not a new latency or kernel-refinement claim.

The shared application now retains separate `Id<Compilation>`,
`Id<CounterexampleSearch>` and `Id<MeaningRendering>` values. A stored body's
definition fixes both its identity context and its directory. The reader no
longer accepts an independently selected role string, and each index retains
identities for that exact body's context. Publication, source, graph and
prepared-input identities remain separate. Lookup hashes are still private
selection keys, rather than content identities or authority tokens.

Parent references in search/render artifacts and the actual search request
retain the original compilation identity. The observation owner distinguishes
an original compilation from a searched result; it cannot represent a raw
observation with a different duplicate compiler ID. Searching a searched result
is refused before lookup or dispatch, including when its graph is empty and
there is no checker field from which a caller could infer its stage.

The original bare lowercase digests, field order, JSON bodies and hash framing
are preserved. Decoding validates canonical spelling only. A correctly hashed
foreign body copied under another role still fails decoding; original parent,
method, complete-content, source and resource checks remain necessary. Neither
an identity nor a cached search result accepts a requirement, authorizes an
effect or admits evidence.

The search port now declares its actual canonical parent identity in its schema,
and the registered Plausible implementation captures that codec. Its method
definition changes. Old search observations remain bound to their original
method; a new search must execute once. The compiler definition and immutable
compiler/rendering artifacts should remain reusable under their original exact
checks. No old result is relabeled to avoid that first search.

Compile examples reject search/compilation, rendering/search and publication/
compilation exchanges; a matching actual owner getter compiles. Generated
Unicode/digest cases compare the original parent serialization and hash framing.
Runtime checks cover malformed indices in all three roles, a correctly hashed
foreign-role body, searched-parent refusal and an empty graph, alongside the
existing corruption, stale-method, cancellation and independent-renderer cases.

The internal compiler observation still contains Lean-specific records. Generic
backend snapshots, retained module ownership, remaining orchestration identities,
native preparation waits and the complete retirement audit remain open. This
change supplies no cold-install, live-model, large-model or new proof evidence.

## Validation and actual project observations

The [complete record](../../results/cache-artifact-contexts-20261125.json) binds
the source files, executable, original/current cache chains, raw captures and
validation logs. All 264 Rust test/doc checks pass across the application, model
transport and CLI packages. The final 11 cache checks include 3,000 generated
Unicode/digest cases with fixed seed `20261125`. Clippy, architecture checks, schema consistency, formatting
and Python lint pass. The public TUI gate passes all 35 journeys; six complete
decoded review/failure/repair/stop/restart screens were inspected. All 76 browser
journeys pass without skips or flaky results. The browser assets remain exactly
the original 88 files. Initial construction/Clippy failures were corrected and
are not counted as completed gates.

Both actual services run the validated executable. Public file-tree selection,
node-to-original-line navigation, quiet chat, retained map/graph cameras and tab
returns succeed. Compiler and rendering definitions and their entire immutable
objects remain original. The original search objects also retain their exact
bytes. The new selected search definition executes exactly once per project;
neither compilation nor the external renderer runs. Source, statement, graph,
reader, inventory, navigation, history and evidence data remain original except
for the explicitly new graph/publication identities and newly observed checks.

EthLib's new search checks all match their predecessors. Riemann changes three
observations: `fmat_inv_mul` changes from unsupported to budget exhausted,
`natCast_cancel` changes from budget exhausted to unsupported, and `mGen_of`
retains budget exhaustion with a different diagnostic. These wall-budgeted
observations are retained as the actual new run, with no counterexample verdict
or native proof admission. Their complete original/current checks are recorded;
no old result is copied into the new method.

The initial concurrent functional preparations take 156.651 s in Riemann and
26.276 s in EthLib; the search worker spans take 123.400 s and 22.640 s. Reopening
in a fresh browser reuses the new result exactly, taking 10.985 s and 1.966 s.
After an actual host restart, the complete newly prepared views and cache chains
remain exact, taking 12.285 s and 2.229 s. Both cached stages run no compilation,
graph extraction, search or meaning-rendering worker. Each explicit file opening
inspects the file once; returning to its retained tab performs no inspection.
The bounded native metric captures report no eviction. All hosts end idle and
unapproved, with original conversations and candidates.

These are functional observations with project/OS caches, concurrent project
work and nested spans, rather than a paired latency comparison. The full input
validation wait remains. Installed tools and scripted inference in the gates do
not establish live-model quality, cold installation or new formal refinement.
