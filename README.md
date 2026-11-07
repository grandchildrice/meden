# T Agent

**Proof-driven development for agent actions.**

[Proof-Driven Development](https://www.microsoft.com/en-us/research/blog/formally-verifying-agent-written-full-stack-web-apps-with-pdd-kit/) turns a developer's intent into a formal specification and proves that agent-written *code* meets it. T Agent applies the same discipline to what an agent *does*: every tool call is typed with the capabilities it needs, every skill is admitted only with evidence that it stays inside its declared capabilities, and the runtime refuses anything that is not typed. The safety of the system does not depend on how clever the model is, which is what makes it usable with small, local, open-weight models.

> Status: pre-alpha. The interfaces below are the first commit of a research prototype. Nothing here is a security boundary yet.

## What is in this repository

| Module | Purpose |
| --- | --- |
| `t_agent.capability` | The capability vocabulary (`fs.read`, `net.egress`, `shell.exec`, …) and requirement sets. |
| `t_agent.policy` | A policy is a set of granted capabilities plus deny rules; `check()` yields an allow/deny decision with reasons. |
| `t_agent.attestation` | The evidence format every verifier plugin must produce, and the three-tier acceptance rule (`verified`, `verified-with-tcb`, `not-verified`). |
| `t_agent.frontends.skill_md` | Reads an [Agent Skills](https://agentskills.io) package (`SKILL.md` + scripts, including OpenClaw's `metadata.openclaw.requires`) and extracts the capabilities it would need. |
| `t_agent.runtime.hook` | The `before_tool_call` contract (modelled on pi's `beforeToolCall`) and a `CapabilityGate` that implements it. |
| `t_agent.runtime.agentdojo` | A `BasePipelineElement` that drops untyped or over-privileged tool calls inside an [AgentDojo](https://github.com/ethz-spylab/agentdojo) pipeline. Requires the `agentdojo` extra. |

## Design in one paragraph

The gate never sees a verifier, only an **attestation**: artifact hash, property id, verifier name and version, result, trusted computing base, assumptions, proof hash. Verifiers are plugins. Evidence whose TCB is Lean 4 alone is accepted mechanically; evidence from other provers (Verus/Z3, …) is accepted once a human has approved the property mapping and is labelled with its TCB; tests and scans never confer *verified*. Enforcement points are plugins too: the same policy kernel runs in this repository's loop, and can be mounted on pi (`beforeToolCall`), Hermes (`pre_tool_call`) or OpenClaw (`before_tool_call`).

## Quick start

```bash
uv sync --group dev
uv run pytest
```

Extract the capabilities a skill asks for:

```bash
uv run python -m t_agent.frontends.skill_md path/to/skill/
```

## Evaluation

The reference benchmark is AgentDojo (v1.2.2 suites: workspace, slack, travel, banking). Install the extra and point it at a local OpenAI-compatible server:

```bash
uv sync --extra agentdojo
# see docs/evaluation.md (to be written)
```

## License

Apache-2.0. Parts of the runtime will be derived from [CaMeL](https://github.com/google-research/camel-prompt-injection) (Apache-2.0) and [Progent](https://github.com/sunblaze-ucb/progent) (MIT); their notices will be kept in `THIRD_PARTY_NOTICES.md` when that code lands.
