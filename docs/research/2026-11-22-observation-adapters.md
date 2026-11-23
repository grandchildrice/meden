# Prepared observations and replaceable backends

The November 23 application extraction replaces the original byte/format
`Adapter<Context>` primitive with role-specific ports in `meden-app`. The core
retains control/evidence vocabulary and no longer contains that unused primitive.
Compiler requests and complete snapshots are passed directly as typed values;
meaning renderers own their decoded formal-expression type. Observations remain
separate from an evidence receipt or authority bit.

Lean's frozen and prepared-Lake compilers are registered by trusted host code in
`meden-app/src/adapters`; the application owns enrichment and retention. Informath
implements the deterministic renderer role over its validated Dedukti input.
The model transport implements a session-bound application port while preserving
its actual consuming grant and refusal/expiry/revocation checks. Ordinary
integrations do not put Lean, SMT, Verilog, GF or process-launching logic in the
kernel. The complete multi-backend registry and second real verifier remain
migration work; these existing Lean modes are not counted as two verifier kinds.

## Adopted research implementation

We adopt Aarne Ranta's
[Informath / Symbolic Informalization](https://arxiv.org/abs/2606.16893).
It uses Dedukti as an intermediate representation and Grammatical Framework for
mathematical language generation. This fits an adapter boundary across formal
languages better than an English realizer alone, such as
[SimpleNLG](https://staff.um.edu.mt/albert.gatt/pubs/enlg2009-simplenlg.pdf).

This is an invocation of the actual released Informath executable and its English
PGF grammar, not a renaming of Meden's templates. We pin
[Informath 0.4](https://github.com/GrammaticalFramework/informath/releases/tag/informath-0.4),
the two release archive SHA-256 values, and symbol/base tables at commit
`cb11ebc4cf62d3563b547800b6e445c981f1178f`. The host-only
`uv run --project python python tools/prepare-informath.py` command installs its
Linux x86-64 resources and checks the executable before publishing a manifest.
On NixOS it selects dependencies from the public system closure, independently
of repository PATH entries. GUI/compiler jobs never download or install resources.

Meden's Lean bridge exports bounded elaborated expressions to Informath's
documented presentation dialect. Quantifiers, assumptions, logical/arithmetic
operators and membership preserve their explicit structure. Defined predicates
can be expanded; unsupported expressions retain the structural renderer. The
interchange omits universe and instance information and is not a checkable proof
export or a proved equivalence to the original Lean expression.

Renderer resource and adapter/profile fingerprints are checked before reuse.
A bounded renderer stage retains the structural reading if optional realization
cannot finish; it does not alter the separately stored search result.

Fixed `-synonyms=1 -symbolics=0 -nbest=1 -sampling=1` options select a verbal
reading deterministically. For example, a universal natural-number inequality
becomes “For all natural numbers …, the sum … is less than or equal to …”.
Variables/math islands use display-only KaTeX. The actual generated text, renderer
identity and any renderer failure are saved with the compiled observation.

The renderer runs in its own offline namespace with private scratch, explicit
read-only prepared resources and frozen formal input on stdin. It sees no project
or host home directory. Its result cannot classify proof success. Compiler,
structural rendering, interchange and Plausible search are separate modules;
failure in one declaration's realization preserves its structural explanation.
Lean compilation and Plausible sampling retain their own failure classifications.

## Prepare, then display

Opening a Lean file prepares compilation, meaning and counterexample observations
before publishing its graph/source page. Conventional Main entry modules are warmed
during workspace preparation. Full snapshots, rather than only explanation strings,
are retained for reopening. A cold, previously unprepared file has a preparation
screen; a warm visit reads the bound artifact. Hovering runs neither tool nor model. Explicit tool results render their saved
source directly, and node selection/paging within that prepared source requires no
new inspection or compilation.
There is no source-English preview or `not_run` sampling surrogate in the display.
Unsupported generators and failed computation remain explicit observations.

Compiled reuse checks all observed source bytes, Lake metadata, compiler resources
and the renderer fingerprint. Root/dependency/configuration/resource edits require
new observations. A failure remains local to the file; source browsing is retained,
and missing project dependencies are not silently downloaded or replaced by stubs.
Compiler collection remains bounded at 32 sources/16 MiB; this does not build all
175,187 Riemann sources or make an unavailable Mathlib installation complete.

For large inventories, workspace preparation creates a sharded, atomic, disposable
source-map cache before declaring the map ready. Reopening reuses the prepared inventory; explicit refresh compares pinned
file metadata and rereads changed files. Changed cache generations invalidate
previously hydrated browser chunks. Metadata is only a presentation hint;
compiled/evidence freshness still reads exact bytes. Corrupt/non-UTF-8/oversized
files have cached individual failures and do not suppress other map entries.

Zoom uses authenticated `GET /api/map?path=…`, which reads the prepared file/nearby
barrel entries only. It starts no worker, compiler, sampling or model call, changes
no conversation, and does not resend or rehash the whole inventory. Refresh prepares
changed entries. This cache is not a complete semantic dependency or proof graph.

The integration, grammar bridge, cache, JSON decoding, metadata, OS isolation and
rendering are tested adapters. No existing kernel refinement theorem is extended
to these components by introducing the common interface.
