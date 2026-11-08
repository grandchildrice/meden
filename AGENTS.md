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
  control. The isolated planner is retired. The transfer, export review and
  evidence kernels have Rust-to-Lean refinement. The combined export state
  owns its frozen request; the runtime dispatches only its returned effect.
  The actual Crossterm approval predicate is extracted and refined too;
  decoding, screen routing and physical input provenance are separate gaps.
  The outer turn controller lives in the shared runtime. Its actual effects,
  state updates and arbitrary finite compositions are refined, including four
  conversation invocations per host turn, sticky terminal stops and a fresh
  check after budget exhaustion. One invocation may make multiple LLM requests.
  The session dispatches its returned operation; the I/O interpreter, signal
  attribution and inner conversation loop remain separate obligations.
  Typed verification-failure classification and repair-stop decisions are
  extracted/refined; I/O mapping, host-stop routing and history remain tested
  adapters. Operational failures stop automatic repair until a new host turn.
  Issued-job operations and arbitrary finite compositions are extracted/refined:
  exact session/contract/input-bound tickets are consumed once, and older
  generations remain invalid after cancellation or consumption. The ledger
  adapter prepares inputs before issue and derives receipt activity from this
  kernel. Input construction, locking, physical cancellation and the full
  session/executor correspondence remain separate obligations.
  Both ledger and release validation now use extracted/refined job-to-evidence
  completion. `Receipt` and direct recording are private; adapters provide
  validated output without an activity flag. Failed output admission never
  restores a consumed job. This is still not the complete session I/O proof.
  The evidence kernel gates
  Meden's own releases and the fixed Rust ledger project verifier. General
  contracts/backends, full interface/tool coverage and outbound
  confidentiality remain targets. See docs/project-verification.md.
