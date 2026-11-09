# Working agreements

- Work on `main` and commit/push directly. The user explicitly authorized
  this workflow; do not create a branch or PR for ordinary work.
- Review existing PR changes and their relevant checks before merging.
- Prefer a small, loosely coupled implementation. Backward compatibility is
  not required at this stage. Delete obsolete code and its dependencies
  rather than adding compatibility layers or keeping unused alternatives.
- Follow `docs/design.md`: a local-first natural-language verification
  orchestrator with a shared Rust controller specified/refined in Lean.
  `docs/assurance.md` defines proposed proof obligations;
  `docs/verification.md` records checks that actually run today.
- Preserve accepted intent, evidence/source/artifact linkage and local data
  control. The isolated planner is retired. The processor, export review and
  evidence kernels have Rust-to-Lean refinement. The combined export state
  owns its frozen request; the runtime dispatches only its returned effect.
  The processor owns its immutable local/HTTP binding and session, issues typed
  consuming calls and never renews a grant. Pending revocation and observed
  expiry are terminal. Its constructors, lifecycle, full parameter snapshots,
  consuming getters and arbitrary finite compositions are extracted/refined.
  The runtime HTTP adapter accepts only HttpCall; local consumes LocalCall and
  checks the actual engine/weights configuration. I/O, preparation, host origin,
  clocks and locks remain adapters.
  The actual Crossterm approval predicate and review-target selector are
  extracted and refined too. All approval handlers match the selected target
  against the host's current screen. Decoding, screen transitions, rendering,
  effect interpretation and physical input provenance remain separate gaps.
  The numeric display transformation is extracted/refined too, including
  finite compositions, safe output, preserved ordinary values/newlines and valid scalars.
  Unicode String iteration/conversion/collection and terminal rendering remain
  separate adapters; the scalar proof does not establish display authenticity.
  The outer turn controller lives in the shared runtime. Its actual effects,
  state updates and arbitrary finite compositions are refined, including four
  conversation invocations per host turn, sticky terminal stops and a fresh
  check after budget exhaustion. One invocation may make multiple LLM requests.
  Inspection/check signals retain typed failures: unavailable or inconclusive
  source inspection stops instead of reusing an older rejected check.
  Failed proposals are retained independently of file edits. Before acceptance,
  normal or budget outcomes may request proposal repair under the same outer
  budget; failed model outcomes and unobserved failures cannot request it.
  The session dispatches its returned operation; I/O interpretation and signal
  attribution remain separate obligations.
  The inner conversation uses its own extracted/refined effect controller;
  it owns the original reply/call vector, cursor and typed result. Its
  request budget, latched stops and call/result observations are checked
  separately from JSON history, model transport and physical dispatch.
  Typed verification-failure classification and repair-stop decisions are
  extracted/refined; I/O mapping, host-stop attribution and history remain tested
  adapters. Operational failures stop automatic repair until a new host turn.
  Issued-job operations and arbitrary finite compositions are extracted/refined:
  exact session/contract/input-bound tickets are consumed once, and older
  generations remain invalid after cancellation or consumption. The project
  adapter prepares inputs before issue and derives receipt activity from this
  kernel. Input construction, locking, physical cancellation and the full
  session/executor correspondence remain separate obligations.
  Both project and release validation use extracted/refined job-to-evidence
  completion. `Receipt` and direct recording are private; adapters provide
  validated output without an activity flag. Failed output admission never
  restores a consumed job. This is still not the complete session I/O proof.
  The evidence kernel gates
  Meden's own releases and the generic pure-u64 Rust transition verifier.
  Contracts are bounded data: metadata and restricted mathematical terms,
  never candidate Lean commands, tactics, imports or proof scripts. Drafts
  cannot replace accepted meaning or evidence without host review of their
  exact canonical hash. A rejected replacement retains the accepted contract.
  TUI acceptance binds the frozen displayed review, not a new current reference.
  Contract selection, review, acceptance, rejection, edits and job admission
  now share one extracted/refined core controller, including arbitrary finite
  compositions. Proposals use monotonic revisions; an identical proposal after
  rejection needs a fresh review. Read-only views do not expose mutable gates.
  The accepted revision/session/contract/obligations stay frozen across any
  finite composition without a host acceptance operation; input bindings,
  proof coverage and job activity can change independently.
  Formula parsing, canonical bytes, rendering, origin attribution and session
  I/O correspondence remain adapters; do not claim whole-controller refinement
  from these controller-operation proofs.
  Retained project files/report are revalidated against the host-held checker
  result on persistence and status inspection. A mismatch discards coverage via
  Contracts.edit; restoring files alone cannot restore evidence. Storage reads,
  hashing and concurrent host mutation remain I/O obligations.
  Broader contracts/backends, full interface/tool coverage and outbound
  confidentiality remain targets. See docs/project-verification.md.
