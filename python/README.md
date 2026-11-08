# meden-agentdojo

AgentDojo owns the environment and tool implementations. The Rust binary
owns conversation, native tool calling and capability checks. The Python
adapter connects them over JSON Lines. There is one Meden execution path;
the isolated planner, flow-rule declarations and standalone gate hook were
removed without compatibility aliases.

The comparison baseline is AgentDojo's own `AgentPipeline.from_config`, not
a hand-written agent. The replay test compares full HTTP conversations,
including schemas, message ordering, argument serialization and tool output.
Both agents use fifteen tool groups and the server's sampling defaults
(AgentDojo omits temperature zero at the HTTP boundary).

## Run

```bash
cargo build --release -p meden-cli
cd python
uv sync --group dev --extra agentdojo
uv run python -m meden_agentdojo.benchmark --agent meden --suite banking \
  --provider hermes:ollama-cloud --model minimax-m3 --benign-only --workers 4 \
  --repeat 3 --out native.json
uv run python -m meden_agentdojo.benchmark --agent tool-calling --suite banking \
  --model minimax-m3 --benign-only --workers 4 --repeat 3 --out baseline.json
```

Use the same model, endpoint and worker count for comparisons. The baseline
uses Ollama Cloud by default. Suites: banking, slack, travel and workspace.
Omit `--benign-only` to measure injections as well; the runner first checks
which injection goals are reachable and records known scoring collisions.
Records include settings, binary digest, calls, decisions, answers and exact
API history. Refusals due to invalid model arguments are kept separate from
capability denials. These results do not establish the new confidentiality
requirement in [the design](../docs/design.md).

## Verify

Build the Rust binary before running tests; do not overwrite it during a run.

```bash
MEDEN_BIN=../target/debug/meden uv run pytest -q
uv run ruff check .
uv run python -m meden_agentdojo.tui_smoke --binary ../target/release/meden \
  --provider hermes:ollama-cloud --model minimax-m3 --out ../results/tui-live-smoke.json
```

The live-model check drives two real TUI turns: a bug fix, then a feature
addition. It independently checks the original tests and added function,
real file effects, successful agent-run tests and restored terminal settings.
Run it separately from benchmark measurements to preserve worker counts.
