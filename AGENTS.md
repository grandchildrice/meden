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
  The evidence kernel gates
  Meden's own releases and the fixed Rust ledger project verifier. General
  contracts/backends, full interface/tool coverage and outbound
  confidentiality remain targets. See docs/project-verification.md.
