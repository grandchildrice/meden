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
- Establish the development workflow with explicitly approved Ollama Cloud
  inference first. Target capable 100–600B local deployments; comparable
  performance on roughly 10B models is deferred. Do not make small-model
  parity a gate or prioritize repeated small-model pilots. Cloud success is
  not evidence of local inference or confidentiality; keep execution/proofs
  local and retain the local deployment and R1 obligations.
- Documentation prefetch uses its own Linux worker with only the fixed URL
  batch, empty scratch, public tools/libraries and read-only resolver config.
  It retains the launching network namespace; the trusted HTTPS client gates
  public resolved addresses. This is not an OS egress firewall. All coding,
  inference and checker profiles stay offline; there is no unconfined fallback.
  Numeric IPv4/IPv6 decisions are extracted/refined against packed-prefix
  interval specifications. URL/IP conversion, DNS/connector/TLS attribution,
  JSON/cache interpretation and OS behavior remain adapters. Never expose the
  prefetch worker as a model-selected request or a general network executor.
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
  Tool execution and controller-owned checker calls share the extracted
  core invocation controller. Its only constructor evaluates the actual owned
  policy against the declared requirements, then freezes a prepared one-use
  executor with that policy's immutable reference. Attempt::with_policy is the
  public constructor; the unbound constructor is private. Dispatch returns an
  opaque Call, whose consuming projection returns the original request/policy
  pair without a replacement-policy argument. Construction, projection and
  arbitrary finite traces are extracted/refined, including complete original
  policy values, refusal, stops and at most one call after notification.
  File preparation composes with this bound controller too. Rust forbids policy
  mutation while retained and unchecked Call construction. The agent uses only
  the returned pair; closure interpretation and physical I/O remain adapters.
  Tool.prepare owns the request; Tool.call and independent requirements
  methods have been removed. It requires a permitting notification, consumes preparation
  on every dispatch attempt and checks cancellation/host stop after notification.
  Generic payload/notification observations and arbitrary finite compositions
  are refined, including exact single-use admitted payloads and notification precedence.
  Denied or failed preparation emits no payload across any finite composition.
  The actual agent failure adapter preserves refusal, cancellation and host stops.
  Project verification, model status and artifact preparation freeze one core
  Inspection before capability admission. Session, proposal sequence, accepted
  and displayed views, and acceptance readiness must still match on consuming it.
  These native constructors/getters/comparisons/consumption are extracted/refined;
  a later proposal generation remains invalid across every finite controller trace.
  CLI Access derives read scopes from the frozen parsed agreement and checks its
  view against the consumed original before entering the I/O callback under the
  same state mutex. Verification and freshness reads use that original contract.
  Host status can inspect current state directly. JSON interpretation, mutex and
  callback semantics, workspace attribution and physical effects remain adapters.
  Registry/JSON preparation, requirement/resource attribution, observer interpretation, physical stop
  attribution, trait implementations and I/O remain adapter obligations.
  File tools and snapshots share one typed core operation/path/payload request.
  Its preparation derives all requirements from the original operation and exact
  decoded path, preserving the whole request; edit requires read and write.
  Preparation, Scope/char cloning and consuming worker fields are extracted/refined,
  including composition with the invocation controller for finite request traces.
  The actual worker consumes those fields instead of reparsing a path or operation
  from payload/model-facing tool names. JSON/UTF-8, boxed-closure capture, filesystem
  resolution and physical I/O still require adapter correspondence. The Linux
  file worker resolves existing internal links, then pins the canonical target
  beneath an open workspace root with openat2 BENEATH/NO_SYMLINKS. Content reads
  use the pinned regular-file descriptor; lists and atomic replacements retain
  the directory descriptor. New symlinks during pinning are refused; later path
  substitutions cannot redirect those handles. Only ENOENT admits a new write
  target. There is no unrestricted fallback. This is tested OS adapter behavior,
  not an extracted filesystem proof. The actual admitted immutable Policy is
  consumed from the controller's Call, passed to prepared executors and
  serialized separately from model payloads.
  Workers check the original request before lookup, then bind its canonical
  UTF-8 path and O_PATH handles into the core Resolved value. Its consuming
  check derives every operation effect at both paths and evaluates both against
  that same policy before returning the original request/resource. Binding and
  consumption are extracted/refined; there is no allowance flag or unchecked
  resource getter. All model file/snapshot/project/export callbacks propagate
  that policy. Host status and host-approved delivery freshness use the fixed
  host workspace file policy. Content descriptors/entry listing/replacement
  are opened only after the resolved check. JSON/UTF-8 transport, callback and
  handle/path attribution, hard-link identities, concurrent inode contents/
  directory entries and host mutation remain adapter obligations.
  Scopes are immutable owned scalar vectors, serialized as ordinary strings.
  Scope.matches uses the extracted/refined Unicode glob kernel. The actual
  owned policy APIs, capability/scope checks, deny precedence, full requirement
  aggregation and per-input refusal vectors are extracted/refined too, with
  total completion in Aeneas's model. Diagnostics are produced after evaluation;
  sorting/deduplication never determines the authority bit. Requirement provenance
  is optional diagnostic text, absent when unknown. String decoding,
  JSON, diagnostic rendering, input mapping and resource attribution remain gaps.
  Unknown grant labels and policy/grant object keys are configuration errors;
  do not restore permissive fallbacks for malformed authority configuration.
  File tools and project/export snapshots share the runtime's typed worker
  result protocol. Host cancellation, limits/truncation and actual exit status
  precede parsed output. Only complete raw-zero exit plus an original Ok
  response admits success. Worker cancellation/limit tags become unavailable;
  diagnostics never classify outcomes. The private standard-sum ToolError's
  constructors/accessors, host admission and agent/source failure mappings are
  extracted/refined. Refusal/infrastructure/interruption stops source repair;
  ordinary completed operation failures remain repairable. JSON/OS observation
  provenance and physical effects remain tested adapters.
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
  Project delivery uses the same contract controller. Its frozen context and
  export reservation share one extracted core model; current evidence is
  required again before dispatch. A raw export queued before a project
  contract cannot bypass it. Checked packages come only from retained
  checker files; payload construction, fresh source/storage observation,
  host origin, locks and HTTP remain adapters. Do not describe this scoped
  package path as whole-application delivery assurance or complete R1.
  Broader contracts/backends, full interface/tool coverage and outbound
  confidentiality remain targets. See docs/project-verification.md.
