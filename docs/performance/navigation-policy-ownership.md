# Backend-owned navigation policy

Entry-module selection and import-name interpretation now live in the host
adapter. The CLI passes an immutable original inventory, a resolver and a saved
syntax reader to `meden_app::navigation::neighborhood`. The common application
walk checks inventory membership before loading, rejects foreign returned paths,
preserves import order and cycles, expands import-only intermediates and retains
the original 32-file navigation bound. This is cached navigation, not complete
compiler input closure or proof evidence.

The Lean location adapter preserves root-relative, workspace-relative and unique
suffix resolution. Its test-only retired GUI resolver is the differential oracle
for generated inventories. A separate literal SMT2-path neighborhood exercises
the common traversal without any Lean name conversion. Missing saved neighbors,
foreign results and exhausted bounds remain explicit; no partial result is
published after an error.

Indexed inventory headings and navigation chunks carry an explicit `entry`
boolean. Unindexed headings retain their previous small shape. The browser uses
the current backend flag rather than recognizing `Main.lean`; its lexical cache
retains references but takes entry/target metadata from current headings.
The map also uses the backend's generic theorem/definition/axiom kind. A finite
property whose declaration contains a sentence is therefore a theorem/property
marker instead of being guessed to be a definition. Lemma and structure spellings
remain presentation refinements. Main-target markers confer no proof status.
The dependency graph no longer privileges the demo name `KernelProof.actual_spec`.
Explicit focus and backend main-target metadata determine its preferred root.

Lean's conventional entry policy selects exact shallow `Main.lean` filenames.
The previous startup suffix check also selected `NotMain.lean`. The public browser
journey now requires that file to stay unprepared on startup and restart, then
explicitly opens it through the explorer, checks a real installed-Lean graph and
reopens with a compiler-cache hit and no inference approval. Existing distant
Main targets and their compiled English/counterexample observations remain covered.

The entry regression checks a literal filename key, including its dot; a passing
control using a dotted property-path string was rejected. The original executable
then fails the corrected quiet-start assertion after completing an unwanted
25,882 ms preparation in the installed-tool fixture. This is a specific observed
job, not a general startup benchmark. Restart coverage also distinguishes saved
presentation from freshness: a visible cached popup can precede current-input
validation, so the check separately waits for a ready/cache-hit outcome.

The wire change is intentional. The raw saved-inventory format, syntax namespace
and compiler/search/renderer method definitions are unchanged; entry flags are a
current presentation projection. Backend policy can change independently of the
lexical cache. No compatibility schema or alternate runtime path is retained.

This advances policy ownership in migration steps 3/4. It does not remove the
measured first-detail SVG/Canvas work, the complete dependency-content checks,
observation-correspondence obligations or the remaining retirement audit.

## Validation and deployment

The [numeric record](../../results/navigation-policy-ownership-20261126.json)
binds the new executable and all 88 browser assets. All 304 Rust test/doc checks,
87 browser unit checks, 37 required public TUI journeys and 11 focused native
GUI journeys pass. The corrected original-executable control fails at the
literal-filename quiet-start assertion; restoring filename inference in the
browser fails the backend-entry role assertion. Boundary checks, Clippy,
formatting, Python lint and schema checks pass. Decoded connection/repair review,
clipped approval, changed-source failure/recovery, cancellation and restart
screens were inspected, along with the new explicit-file graph and both actual
workspace source/graph screens. These use installed tools and scripted fixtures,
not live-model or cold-install validation.

Both actual workspaces use the new executable/assets and end idle and unapproved.
For their selected sources, complete source, reader and evidence publications
remain exact. The complete inventories preserve every former field while adding
current entry flags to three Riemann and 2,517 EthLib indexed headings. Their
navigation-detail identities remain exact; inventory and selected-chunk
publication identities deliberately rebind to the new fields/parent. Every
former selected-chunk field remains exact after that explicit translation.
Only `src/Main.lean` is an entry among the Riemann headings; EthLib has none.

Selected semantic preparations are cache hits with no new extraction, search or
rendering worker spans and no metric-ring evictions. Histories/candidates remain
exact. Riemann's original complete syntax seal and selected syntax-entry bytes
are unchanged; EthLib's absent large-map cache remains absent. This closes this
policy-ownership slice, not all five migration exits or the measured residual
rendering/native waits.
