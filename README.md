# T Agent

**Proof-driven development for agent actions.**

[Proof-Driven Development](https://www.microsoft.com/en-us/research/blog/formally-verifying-agent-written-full-stack-web-apps-with-pdd-kit/) turns a developer's intent into a formal specification and proves that agent-written *code* meets it. T Agent applies the same discipline to what an agent *does*: every tool call is typed with the capabilities it needs, every skill is admitted only with evidence that it stays inside its declared capabilities, and the runtime refuses anything that is not typed. The safety of the system does not depend on how clever the model is, which is what makes it usable with small, local, open-weight models.

The gate itself is held to the same standard. Its rules are stated in Lean 4 with proofs, the Rust implementation is model-checked with Kani within bounds and checked for conformance against the Lean model on every change, and a pull request merges when those checks pass. See [CONTRIBUTING.md](CONTRIBUTING.md).

> Status: pre-alpha research prototype.

## What is in this repository

| Path | Purpose |
| --- | --- |
| `crates/tagent-core` | Capability vocabulary, scope globs, policies and decisions, attestation tiers. No I/O, no `unsafe`, no panics. |
| `crates/tagent-agent` | The agent core: labelled values, the restricted plan language, and the interpreter that runs a plan through the gate. |
| `crates/tagent-skill` | Reads an [Agent Skills](https://agentskills.io) package (`SKILL.md` + scripts, including OpenClaw's `metadata.openclaw.requires`) and extracts the capabilities it would need. |
| `crates/tagent-cli` | The `tagent` binary: `extract`, `check`, `decide`, `tier`, `gate`, `digest`, and the first verifier plugin, `attest lean`. |
| `examples/` | A benign and a ClawHub-style malicious skill, and two policies. |
| `lean/` | `TAgentSpec`: the model and its theorems, plus the `tagent-spec` oracle used by the conformance test. |
| `python/` | AgentDojo adapter that mounts the gate as a pipeline element, calling the binary. |

## Design in one paragraph

The gate never sees a verifier, only an **attestation**: artifact hash, property id, verifier name and version, result, trusted computing base, assumptions, proof hash. Verifiers are plugins. Evidence whose TCB is Lean 4 alone is accepted mechanically; evidence from other provers (Verus/Z3, …) is accepted once a human has approved the property mapping and is labelled with its TCB; tests and scans never confer *verified*. Enforcement points are plugins too: the same policy kernel runs behind `tagent decide` and can be mounted on pi (`beforeToolCall`), Hermes (`pre_tool_call`) or OpenClaw (`before_tool_call`).

## Quick start

```bash
cargo build --release -p tagent-cli
alias tagent=target/release/tagent

# What does a skill ask for?
tagent extract path/to/skill/

# Does it fit a policy?
cat > policy.json <<'EOF'
{"name": "read-only", "grants": ["fs.read[/workspace/*]"], "denies": ["shell.exec"]}
EOF
tagent check path/to/skill/ --policy policy.json      # exit 0 allowed, 2 denied

# Admission: policy plus a verified attestation whose artifact hash matches the package digest
tagent digest path/to/skill/
tagent gate path/to/skill/ --policy policy.json --attestation proof.json --approve verus+z3

# Verifier plugin: turn a Lean 4 theorem into an attestation (proved only if the
# theorem depends on nothing beyond propext, Classical.choice, Quot.sound)
tagent attest lean --skill examples/skills/csv-summary --project lean \
  --module TAgentSpec.Policy --theorem TAgentSpec.Policy.deny_wins \
  --property inv.v0.deny-wins --statement lean/TAgentSpec/Policy.lean --out att.json
tagent gate examples/skills/csv-summary --policy examples/policies/read-only.json --attestation att.json
# -> policy: allowed / evidence: verified / admitted: true

# Runtime hook: decide a requirement set (what a tool call needs) against a policy
echo '{"policy":{"name":"ws","grants":["net.egress[*.example.com]"]},
       "requirements":[{"capability":"net.egress","scope":"evil.invalid","evidence":"send_email.to"}]}' \
  | tagent decide
```

## Properties the gate guarantees

Stated and proved in `lean/TAgentSpec`, mirrored by Kani harnesses and property tests in Rust:

- `allowed p rs ↔ ∀ r ∈ rs, ¬denied r ∧ granted r` (allowance is exactly this and nothing else).
- Deny wins: a requirement on a denied capability is never allowed, whatever the grants.
- Adding a grant or removing a deny never turns an allowance into a denial.
- A scoped grant never covers an unscoped requirement; a wildcard-free scope matches only itself; `*` matches everything.
- Only a `proved` attestation can be *verified*; a Lean-only TCB verifies without approval; a foreign TCB verifies only with an approved backend; tests and scans never verify.
- Trust cannot be laundered: combining untrusted data with trusted data never yields data that may steer an action.
- If any argument that steers a call carries an untrusted source, the call is refused, whatever the input says.
- If any argument may not be read by the principal a call discloses to, the call is refused.

## Evaluation

The reference benchmark is AgentDojo (v1.2.2 suites: workspace, slack, travel, banking). The adapter in `python/` mounts the gate as a `BasePipelineElement`; see `python/README.md`.

## License

Apache-2.0.
