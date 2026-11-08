"""AgentDojo's own undefended agent, built by AgentDojo, on our model.

This baseline, and it is deliberately not hand-written. The
pipeline comes out of `AgentPipeline.from_config` with no defense, which is
the arrangement AgentDojo's own numbers are produced with:

    [SystemMessage(default), InitQuery(), llm, ToolsExecutionLoop([ToolsExecutor(), llm])]

Everything that would otherwise be a judgement call is therefore AgentDojo's:
the system message (its `data/system_messages.yaml` default, four
instructions that a hand-written one leaves out), the YAML tool-output
format, the fifteen-iteration loop, and the three-attempt retry. Its zero
temperature setting is omitted from HTTP requests by this AgentDojo version,
so the server supplies the sampling default. We supply the remote client
and append a final observer that records the unchanged conversation.
"""

from __future__ import annotations

import os
from pathlib import Path

from agentdojo.agent_pipeline import AgentPipeline, BasePipelineElement, PipelineConfig
from agentdojo.agent_pipeline.llms.openai_llm import OpenAILLM, _message_to_openai

from .pipeline import GateReport

#: What AgentDojo calls an OpenAI-compatible server it did not write.
#:
#: `from_config` has two local providers: `local` drives a model that cannot
#: call tools, by prompting and parsing; `vllm_parsed` drives one that can,
#: through `OpenAILLM`. Ours can, so this is the matching name. It is also
#: what the attacks read to decide who to address the injection to, and both
#: spellings resolve to "Local model" there, so the injected text is the same
#: either way.
AGENTDOJO_LOCAL_PROVIDER = "vllm_parsed"

DEFAULT_MODEL = "minimax-m3"
DEFAULT_BASE_URL = "https://ollama.com/v1"


def api_key(name: str = "OLLAMA_API_KEY") -> str:
    """The key, from the environment or from Hermes' own `.env`.

    Read, never printed: this hands it to a client and nothing else.
    """
    found = os.environ.get(name)
    if found:
        return found
    env = Path.home() / ".hermes" / ".env"
    if env.is_file():
        for line in env.read_text().splitlines():
            key, sep, value = line.partition("=")
            if sep and key.strip() == name:
                return value.strip().strip("\"'")
    raise SystemExit(f"no {name} in the environment or in ~/.hermes/.env")


def build(
    model: str = DEFAULT_MODEL,
    base_url: str = DEFAULT_BASE_URL,
) -> AgentPipeline:
    """The undefended pipeline, composed by AgentDojo."""
    import openai

    client = openai.OpenAI(api_key=api_key(), base_url=base_url, timeout=300.0)
    pipeline = AgentPipeline.from_config(
        PipelineConfig(
            # A BasePipelineElement is an accepted value for `llm`; passing
            # one is how a client that is not OpenAI's own gets in without
            # any other part of the composition changing.
            llm=OpenAILLM(client, model),
            model_id=None,
            defense=None,
            system_message_name=None,
            system_message=None,
        )
    )
    # `from_config` names a pipeline after its provider, and takes the name
    # from `llm.name` when the llm is passed as an object — which is None.
    pipeline.name = AGENTDOJO_LOCAL_PROVIDER
    pipeline.last_report = GateReport(summary="not run")

    class Observe(BasePipelineElement):
        def query(self, query, runtime, env, messages, extra_args):
            try:
                last = messages[-1] if messages else {}
                pending = last.get("role") != "assistant" or bool(last.get("tool_calls"))
                blocks = (last.get("content") or []) if last.get("role") == "assistant" else []
                answer = "".join(block.get("content") or "" for block in blocks) or None
                pipeline.last_report = GateReport(
                    outcome="incomplete" if pending else "done",
                    summary="AgentDojo baseline; no gate",
                    answer=answer,
                    messages=[_message_to_openai(message, model) for message in messages],
                )
            except Exception as error:  # noqa: BLE001 - logging must not alter task execution
                pipeline.last_report = GateReport(
                    outcome="unreported",
                    summary=f"baseline observer failed: {type(error).__name__}",
                )
            return query, runtime, env, messages, extra_args

    # Observe after the factory's complete pipeline. This changes no message,
    # model request, tool call or environment; the HTTP replay test checks it.
    pipeline.elements = [*pipeline.elements, Observe()]
    return pipeline
