# tagent-agentdojo

Runs [AgentDojo](https://github.com/ethz-spylab/agentdojo) against the
capability-typed runtime, so the claim the runtime makes can be checked
rather than asserted.

Every decision is made by the Rust binary. This package only describes the
suite's tools, relays calls, and records what the gate said. Nothing here can
allow a call the gate refused, and nothing here can refuse one it allowed.

## Running it

```bash
cargo build --release -p tagent-cli          # from the repository root
cd python && uv sync --group dev --extra agentdojo

uv run python -m tagent_agentdojo.benchmark \
  --suite banking --provider hermes:ollama-cloud --out results.json
```

`tagent providers` lists what it can plan with. `--benign-only` measures
utility and stops; `--limit` and `--injection-limit` cut the run down while
you are setting it up. A full banking run is 16 tasks benign and 16 × 9 under
attack.

## What it reports

Three numbers, kept apart because they answer different questions.

| | question |
|---|---|
| utility | what the defence costs when no one is attacking |
| utility under attack | whether the agent still works while under attack |
| attack success rate | whether the attack got what it wanted |

And then the part that matters more than the numbers: **why calls were
refused, by rule**, and **why benign tasks failed, by cause**. A task the
integrity rule refused is a cost of this design. A task the planner botched
is a cost of the model it was given. Reporting one headline number hides that
difference, and the two call for opposite fixes.

`results.json` carries a `runs` array with one record per task — its outcome,
the rules that refused it, and the sentence the session ended on — so any
number here can be traced back to the task that produced it.

## Declaring tools

`declarations.py` says, for each tool in a suite, which of its parameters
*steer* the call. A recipient steers; a subject line is carried. That single
judgement is the whole input to the integrity rule, and it is the work a
deployment does.

A tool with no declaration raises `UndeclaredTool` rather than defaulting.
Both defaults are wrong: treating every parameter as steering refuses
ordinary work, and treating none as steering is the hole the design exists
to close.

The banking declarations mirror the policy in CaMeL's own AgentDojo
integration, so that a number measured here can be set beside a number
measured there. Marking more parameters as steering would refuse more attacks
and make the comparison flattering rather than fair.

The policy used is `PERMISSIVE_POLICY`, which grants every capability. A
realistic deployment would also restrict capabilities and would refuse more,
but those refusals belong to the policy, not to the flow rules, and crediting
them here would overstate the result.

## Tests

```bash
TAGENT_BIN=../target/debug/tagent uv run pytest
```
