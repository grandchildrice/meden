# Lean Atlas extraction core

MIT-licensed modules from `NyxFoundation/lean-atlas`, commit
`06be1eb76fb926b8cf8bd42a4a51208d128c327d`. See `LICENSE`.

Meden compiles this pinned subset with a host-selected Lean distribution. Its
separate `MedenAtlas.lean` adapter selects observed modules, retains cross-module
type/value edges, adds elaborated statements and definition bodies, and reports
external references separately. It does not run the upstream web server, project
discovery, Lake configuration, cache or metadata mutation commands.

This is executable analysis, not an extracted/refined Meden control kernel or a
new proof of dependency completeness, parser correctness or analysis authenticity.

Meden changes the axiom collector worklist from front-erasure to a stack. The
visited set and sorted results are unchanged; imported Mathlib closures no longer
shift the remaining array on every visited declaration.
