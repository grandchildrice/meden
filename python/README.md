# tagent-agentdojo

Python adapter that mounts the `tagent` capability gate inside an
[AgentDojo](https://github.com/ethz-spylab/agentdojo) pipeline. All policy
decisions are made by the Rust binary (`cargo build -p tagent-cli`); this
package only maps tool calls to requirements and relays the verdict.

```bash
cd python && uv sync --group dev --extra agentdojo
TAGENT_BIN=../target/debug/tagent uv run pytest
```
