# Backend-owned project layout and neutral workspace observation

Lake package configuration, module attribution and import closure collection now
live under `adapters::lean::{layout, collect}`. The common `analysis` module
retains source/assessment/preparation/repair services without a Lake layout or
collection type. Atlas execution and Lean compiler/search/repair adapters use
the backend-owned collection. The former public common module paths are deleted,
without compatibility aliases.

`workspace::observation` owns bounded, policy-bound directory/file observation
and complete metadata revalidation. It contains no Lean module, Lake package,
Atlas configuration or hardcoded `.lean` selection. Its reader retains the
trusted backend's original directory-entry predicate; revalidation receives that
same backend-selected predicate. Lean's original file-selection rule now lives
in its layout adapter. Existing canonical paths, batch/per-response/aggregate
budgets, absent-file observations, private prefetch buffers and original read
order remain. Successful checks are not retained across operations.

The predicate is host/plugin code, not model-provided executable data. Its
correspondence to a backend's original observation remains that adapter's
obligation. The shared service still uses the original consuming file policy
and confined worker; this is not a new filesystem or whole-application proof.
No kernel, processor approval, accepted intent or evidence-admission rule changes.

## Evidence

The existing complete-selection oracle independently reconstructs every source,
module, package, metadata file and directory observation in an installed Lake
project. It also checks bounded worker starts, unavailable/invalid configurations
and the frozen evaluator's original namespace. Public Lean and finite/Z3 source,
repair, failure and restart journeys use the unchanged common preparation owner.

A new actual confined metadata journey selects an SMT2 inventory without any
Lean resource preparation. It accepts unrelated Lean-file additions, refuses
new SMT2 inputs, detects a same-size metadata-content edit and refuses cancelled
work. Expected directory rows are explicit, independently of the production
reader. The original Lean metadata journeys still cover internal links, absent
paths, content edits, directory edits, invalid UTF-8, oversized responses,
external links and cancellation.

Two controls fail their intended checks: hardcoding Lean selection in the shared
reader rejects that actual SMT2 inventory; returning the original Lake layout to
the common analysis directory fails the actual architecture command. The common
ownership check now enumerates that directory, rather than checking only a
manually selected subset of its modules. The structural guard is not a Rust
parser or alias-proof security proof.

Compiler definitions capture the moved execution/validation leaves. Changed
method definitions require fresh bound artifacts once; existing artifacts are
never relabeled. Native schemas and public view fields remain unchanged. The
GUI assets are unchanged. No speedup is claimed for this ownership migration;
full prepared-input validation and browser rendering costs remain separate.

## Validation and actual workspaces

The [complete record](../../results/lean-collection-ownership-20261126.json)
binds the exact native executable, unchanged 88 GUI assets, controls and captures.
All 294 Rust test/doc checks, 12 installed-Lake selection/namespace cases, 37
required public TUI journeys and seven native GUI journeys pass without skips
or flaky cases. Clippy, schemas, architecture, formatting and Python lint pass.
Decoded review, clipped authority, changed inventory, source failure/recovery,
repair and restart screens were inspected. These are installed-tool/scripted
fixtures, not live-model or cold-install validation.

Both actual workspaces prepare new bound artifacts once. Their complete original
source, formal declarations/edges, locations, axioms, dependency-input fingerprint
and deterministic descriptions remain equal. Rendering-bound graph identities
change as required; they are not restamped with old identities. Riemann has two
fresh timing-dependent checker observations: `natCast_cancel` changes from
unsupported to budget exhausted; `mGen_of` retains budget exhaustion with a new
file-budget diagnostic. Neither establishes a counterexample or proof. EthLib's
checker observations are unchanged. Readers, inventories, map details, evidence,
conversations and candidate history remain exact.

After another actual host restart, all five complete source/reader/inventory/map/
evidence DTOs match those freshly prepared objects. Both selected files hit their
cache and execute no compiler, search or renderer worker, with complete native
capture rings. The retained compiled graph appears in about 1.68 s / 1.43 s in
Riemann/EthLib while current validation still runs; zoom remains unchanged when
validation finishes. These are unpaired functional visibility observations, not
an achieved speedup or physical-paint benchmark. Both hosts finish idle and
unapproved; their source/graph screenshots were inspected.

Initial test-import and Clippy failures were corrected before final validation.
The first actual-data comparator incorrectly required the old graph revision.
The completed preparation was retained without restarting it; comparison then
checked every formal/source/description field and separately recorded the new
bindings and checker observations. Full byte equality is required again across
the subsequent cache reuse/restart.
