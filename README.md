# T Agent

**Proof-driven development for agent actions.**

[Proof-Driven Development](https://www.microsoft.com/en-us/research/blog/formally-verifying-agent-written-full-stack-web-apps-with-pdd-kit/) turns a developer's intent into a formal specification and proves that agent-written *code* meets it. T Agent applies the same discipline to what an agent *does*: every tool call is typed with the capabilities it needs, every skill is admitted only with evidence that it stays inside its declared capabilities, and the runtime refuses anything that is not typed. The safety of the system does not depend on how clever the model is, which is what makes it usable with small, local, open-weight models.

The gate itself is held to the same standard. Its rules are stated in Lean 4 with proofs, the Rust implementation is model-checked with Kani within bounds and checked for conformance against the Lean model on every change, and a pull request merges when those checks pass. See [CONTRIBUTING.md](CONTRIBUTING.md).

> Status: pre-alpha research prototype.

## What is in this repository

| Path | Purpose |
| --- | --- |
| `crates/tagent-core` | Capability vocabulary, scope globs, policies and decisions, attestation tiers. No I/O, no `unsafe`, no panics. |
| `crates/tagent-agent` | The agent core: labelled values, the restricted plan language, the interpreter, and the planner loop. |
| `crates/tagent-llm` | An OpenAI-compatible client, so the planner can be a local model served by vLLM, Ollama or llama.cpp. |
| `crates/tagent-skill` | Reads an [Agent Skills](https://agentskills.io) package (`SKILL.md` + scripts, including OpenClaw's `metadata.openclaw.requires`) and extracts the capabilities it would need. |
| `crates/tagent-cli` | The `tagent` binary: `extract`, `check`, `decide`, `tier`, `gate`, `digest`, and the first verifier plugin, `attest lean`. |
| `examples/` | A benign and a ClawHub-style malicious skill, and two policies. |
| `lean/` | `TAgentSpec`: the model and its theorems, plus the `tagent-spec` oracle used by the conformance test. |
| `invariants/` | The catalogue: what each property id means, and which theorem states it. |
| `python/` | AgentDojo adapter and benchmark: mounts the gate as a pipeline element and reports utility, attack success rate, and why each task failed. |

## Design in one paragraph

Untrusted text is read by a *quarantined* model, never by the planner: a
plan says `{"ask": "...", "text": EXPR}`, and the answer carries the text's
own label. So a summary of an injected page can be mailed as a body but
cannot choose the recipient, even when the quarantined model did exactly
what the page told it.

Keeping the planner blind is the whole basis for trusting its literals, so
the two things that do flow back to it are narrowed deliberately. A refusal
is explained without quoting any argument. A plan that needs to read before
it can act sets `"continues": true` and is told, for each value it bound,
which of six kinds it is and whether it may steer — never the content, and
not the length or the field names either. That is a channel, not nothing:
under three bits per binding per round, with rounds bounded. It cannot steer
a call at any width, because the gate checks the planner's output like
everything else.

What a property id *means* is written down. `invariants/catalogue.json`
gives each id an English sentence, the Lean theorem that states it, and the
hash of that theorem's statement; `tagent invariants --explain` prints the
lot and fails if a statement has changed. Without it the tier rules check
only the trusted computing base, so a verifier could prove `True`, call it
anything, and be believed. The catalogue does not check that the sentence
describes the theorem — a human does that once, when the entry is added. It
makes sure the thing they read is the thing still being proved.

The gate never sees a verifier, only an **attestation**: artifact hash, property id, verifier name and version, result, trusted computing base, assumptions, proof hash. Verifiers are plugins. Evidence whose TCB is Lean 4 alone is accepted mechanically; evidence from other provers (Verus/Z3, …) is accepted once a human has approved the property mapping and is labelled with its TCB; tests and scans never confer *verified*. Enforcement points are plugins too: the same policy kernel runs behind `tagent decide` and can be mounted on pi (`beforeToolCall`), Hermes (`pre_tool_call`) or OpenClaw (`before_tool_call`).

## The demonstration

```bash
cargo run -p tagent-cli -- demo
```

A page tells the agent to send the summary somewhere else. The address it
names is inside the company's own mail domain, so the policy allows it and an
allowlist would not catch it. The planner is taken in completely and writes
exactly the plan the page asked for:

```
attempt 1: calls fetch, send_email
  fetch allowed
  send_email REFUSED
    send_email: argument "to" steers this call but carries untrusted data from Tool("fetch")
    the planner is told: send_email: "to" decides what this call does, so it cannot
    come from data a tool returned; write the value yourself or take it from the user
attempt 2: calls fetch, send_email
  fetch allowed
  send_email allowed
```

The only thing wrong with that address is where it came from, and that is the
one thing the model cannot be argued out of noticing, because it is not the
model that checks. `--live` runs the same scenario with a real model writing
the plans.

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

## Running it against a local model

```bash
export TAGENT_LLM_BASE_URL=http://localhost:8000/v1   # vLLM, Ollama or llama.cpp
export TAGENT_LLM_MODEL=qwen3-30b-a3b
cargo run -p tagent-cli -- demo --live
```

Nothing about the guarantee changes with the model. A planner that falls for
the injection produces a plan the runtime refuses; a planner that does not
produces one it allows. That is what makes a small local model usable here.

## Evaluation

Measured on AgentDojo's banking suite against two baselines, same model
throughout. The baseline is AgentDojo's own undefended pipeline, built by
`AgentPipeline.from_config`: it scores 88% utility and is breached 3 times in
144. This runtime scores 50-56% and is breached none. But the same runtime
with its flow rules switched off is also breached none, so on this suite it
is the planner/reader split doing the work and not the labels, and the 88%
says the split is expensive. The numbers, the raw files and what they do and
do not show are in [`results/`](results/README.md).

The reference benchmark is AgentDojo (v1.2.2 suites: workspace, slack, travel, banking). The adapter in `python/` mounts the gate as a `BasePipelineElement`; see `python/README.md`.

## License

Apache-2.0.
