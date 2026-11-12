# Isolated local inference

Development currently prioritizes an explicitly approved Ollama Cloud model
for the complete coding/proof workflow. The local deployment target is capable
100–600B models; comparable roughly 10B performance is deferred. The CPU backend
below remains available, but its small-model pilots are not a product gate.

## Cloud baseline and large local deployments

Use a configured Ollama Cloud connection, such as the existing Hermes provider:

```bash
meden tui --workspace /path/to/project --provider hermes:ollama-cloud --model minimax-m3
```

Alternatively pass the connection directly, without configuration or exports:

```bash
meden tui --workspace /path/to/project \
  --base-url https://ollama.com/v1 --model your-ollama-cloud-model \
  --api-key "your-ollama-api-key"
```

If the key is already in `OLLAMA_API_KEY`, use `--api-key-env OLLAMA_API_KEY`
instead. The explicit endpoint never inherits credentials from another
environment-based connection or subscription login. The TUI reviews the exact
processor before sending context; automation uses `--approve-processor
https://ollama.com/v1/chat/completions`. Code execution and proof checks stay
offline. This explicitly authorized cloud inference sends context externally.
See [Ollama's API](https://docs.ollama.com/api/openai-compatibility) for the
cloud/local endpoints and supported API subset.

A separately prepared large local server can use the same configured
OpenAI-compatible connection and exact-endpoint approval. Verify the actual
model, context capacity, native tool calls and deployment isolation; parameter
count alone is not evidence of these. HTTP connectivity is implemented, but
100–600B local operation has not been measured here. Loopback or a private LAN
does not itself establish confinement or prevent forwarding to a cloud.
GPU/multi-host confinement and the large-model deployment remain work.

## Subscription connections

`meden providers` discovers existing subscription credentials without making
an inference request. The TUI offers these connections with `--provider auto`,
or they can be selected directly:

```bash
meden tui --workspace /path/to/project --provider codex --model gpt-6.1-sol
meden tui --workspace /path/to/project --provider claude --model claude-sonnet-4-6
```

The host chooses the model with `--model`, `MEDEN_CODEX_MODEL` or
`MEDEN_CLAUDE_MODEL`. A matching Hermes `model.provider` / `model.model` also
supplies the default; otherwise the defaults are `gpt-6.1-sol` and
`claude-sonnet-4-6`. Model availability is determined by the provider, not by
the picker. `--provider auto --model NAME` overrides the discovered models.

| Connection | Existing host login sources, in priority order | Inference URL |
| --- | --- | --- |
| `codex` | `$HERMES_HOME/auth.json` → `providers.openai-codex.tokens`, then `$CODEX_HOME/auth.json` → `tokens` | `https://chatgpt.com/backend-api/codex/responses` |
| `claude` | `CLAUDE_CODE_OAUTH_TOKEN`, `ANTHROPIC_TOKEN` (process environment or Hermes `.env`), then `$CLAUDE_CONFIG_DIR/.credentials.json` → `claudeAiOauth.accessToken` | `https://api.anthropic.com/v1/messages` |

Unset home overrides default to `~/.hermes`, `~/.codex` and `~/.claude`.
The token-variable priority is applied before file discovery; for each
variable the process environment precedes Hermes `.env`. An API-only Codex
login or an Anthropic `sk-ant-api*` key is not offered as a subscription.
Credentials stored only in an OS keychain are not supported by these adapters.

Login remains outside Meden: use `codex login`, `hermes auth`,
`claude auth login` or `claude setup-token` as appropriate for the selected
source. Meden reads the access token without copying a refresh token into a
new store, executing another coding agent, or modifying the original login.
Tokens are frozen with the exact endpoint, model, protocol and optional
ChatGPT account before host review. An expired token, exhausted quota or
refused model stops the turn. Renew the appropriate host login and restart
Meden for a fresh review; there is no token refresh, account rotation,
automatic retry or API-key billing fallback within an approved gateway.

CLI automation must approve the exact native endpoint:

```bash
printf '%s\n' '{"instruction":"Inspect this project"}' | meden run \
  --workspace /path/to/project --provider codex --model gpt-6.1-sol \
  --approve-processor https://chatgpt.com/backend-api/codex/responses
```

For Claude, use `--provider claude` and approve
`https://api.anthropic.com/v1/messages`. Host-only `MEDEN_CODEX_BASE_URL`
and `MEDEN_CLAUDE_BASE_URL` can select another server; the complete resulting
inference URL still requires exact approval. Never point a subscription token
at an untrusted server. Remote processing does not establish local inference
or confidentiality; offline file tools and checking retain their existing
profiles and never receive the broker's credentials.

These adapters follow the existing Hermes token-file/Responses/Messages
integration, rather than registering a new OAuth application. Existing
credentials alone do not establish permission for third-party plan use.
OpenAI's [Sign in with ChatGPT plan-sharing flow](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)
uses its own consent/scopes and the public `/v1/responses` endpoint; that new
registration flow is not implemented here. Anthropic's
[credential-use rules](https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use)
restrict subscription credentials in third-party applications. Meden does not
imitate Claude Code's identity, rewrite its own identity or bypass a provider
refusal. Treat the token adapters as experimental and use them only where the
provider permits the selected account and integration.

## Managed CPU backend

CLI and TUI use prepared local inference by default. Install Bubblewrap,
Bash and a llama.cpp `llama-server` with Unix socket support, and prepare a
GGUF model before confidential work. No component downloads models or
silently switches to an HTTP processor during a local session.

```bash
meden tui --workspace /path/to/project --pages pages.json \
  --local-engine /path/to/llama-server --local-model /path/to/model.gguf

printf '%s\n' '{"instruction":"Inspect the project, fix the bug and run tests."}' | \
  meden run --workspace /path/to/project \
    --local-engine /path/to/llama-server --local-model /path/to/model.gguf
```

The same paths can optionally be set with `MEDEN_LOCAL_ENGINE` and
`MEDEN_LOCAL_MODEL`; command-line flags take precedence. The default path
fails if either asset is missing. `--base-url URL --model MODEL` selects an
HTTP connection directly without discovery. HTTP discovery
is explicit: `meden providers` lists connections, `--provider ID` selects
one, and `--provider auto` opts into discovery/the TUI picker. HTTP processing
still requires the existing trusted TUI approval or exact CLI
`--approve-processor` URL. Environment HTTP providers are ignored in a local
session. A pre-existing loopback server is an HTTP processor, not a confined
local engine.

For split GGUF weights, prepare every numbered sibling and point
`--local-model` at `NAME-00001-of-000NN.gguf`. Meden prepares the exact
numbered set before starting inference, preserving llama.cpp's split naming
inside the sandbox. Only those files are mounted read-only; the containing
host directory and unrelated files remain hidden. Cache symlinks to prepared
blob files are supported. A missing/non-file shard, a later starting shard,
or an invalid split count fails before the CLI emits `ready`. There is no
model download or automatic fallback. The engine still validates GGUF
metadata, tensor coverage and whether shards belong to the same model.
See the upstream [split-model loader](https://github.com/ggml-org/llama.cpp/blob/master/src/llama-model-loader.cpp).

## Execution and data

Meden starts one inference process lazily and keeps it warm across tool
rounds, user instructions and `/clear`. The process has its own private
network/PID/user namespaces and private `/proc`, `/dev` and `/tmp`. It sees
read-only prepared engine/weights, trusted public runtime libraries/tools,
and a private writable scratch directory. The coding workspace, original
project, host home, credentials, host services and inherited descriptors are
absent. Coding workers remain in separate namespaces and cannot access the
inference service. Engine logs are discarded.

The host sends context through one dedicated filesystem Unix socket. It
opens the endpoint without following symlinks, verifies it is a socket and
connects through the held inode descriptor, so replacing the name cannot
redirect a request to a host service. The HTTP framing client has no DNS or
TCP connector, proxy, redirects or fallback. Body limits are 8 MiB per request
and 1 MiB per response. Startup has a 120-second budget and each exchange a
300-second budget. Local session grants use the same refined session-bound
decision kernel, with an eight-hour lifetime. Explicit file exports still
go through host review and the separate one-use transfer path.

Cancellation/revocation terminates the managed inference process group;
namespace teardown terminates its children. Startup failure, dead engine,
bad/oversized reply or failed exchange ends that local backend. Restart the
session to prepare a new engine; there is no automatic replay, restart or
remote fallback. CLI exit zero means protocol handling completed: inspect
`done.outcome` for model failure.

## Supported scope and evidence

The current launcher is Linux/CPU-only, uses a 16384-token context and fixes
the alias `meden-local`. Extended model reasoning is disabled in this CPU profile.
Prompt processing and generation share the 300-second exchange limit;
either can still time out. It does not expose GPU devices or arbitrary
engine arguments. The executable must support `--host PATH.sock`, `--model`,
`--offline`, `--no-webui`, `--no-ui-mcp-proxy`, `--jinja`, `--n-gpu-layers`,
`--alias`, `--ctx-size` and `--reasoning-budget`. This was exercised with
llama.cpp 0.9.0, build
12110 (`cd959ba`), and a prepared Qwen3-0.6B Q4_K_M GGUF. A real model returned
a greeting through `meden run`; that smoke check is not a coding benchmark.
The [smoke model artifact](https://huggingface.co/bartowski/Qwen_Qwen3-0.6B-GGUF/blob/60b85c0e3d8fe0f6474f406922a26d12aca4550d/Qwen_Qwen3-0.6B-Q4_K_M.gguf)
was 484220320 bytes with SHA-256
`9acfc1e001311f34b4252001b626f2e466d592a42065f66571bff3790d4e1b14`.
The [llama.cpp server reference](https://github.com/ggml-org/llama.cpp/tree/master/tools/server)
documents the socket listener and native tool-calling API. The selected
model must supply a compatible chat/tool template.

A [Qwen3-8B ledger pilot](../results/local-ledger-qwen3-8b-80fbf44.json)
used the real offline engine and natural-language instruction. It ended
unverified after 17 requests; native counterexamples rejected its repairs.
It predates budget-recovery and identical-edit rejection. It demonstrates
neither current task success nor a successful project proof; the scripted
CI proof tests measure the independent checker/controller instead.

[Adversarial CLI/TUI tests](../python/tests/test_local_inference.py) use a
deliberately hostile engine and observed TCP/UDP/Unix receivers, including
inheritable host capabilities, confidential context, forged approval,
redirects, disconnects, oversized responses and a symlink to a host service.
They also perform real local edits/tests, retain the warm process, check the
result independently in another offline copy, and cancel pending inference.
Rust transport tests replace the socket name after pinning and verify the
original receiver still gets the connection; an unresponsive peer times out.
[Split-weight tests](../python/tests/test_split_weights.py) exercise real
namespace mounts, cache symlinks, read-only shards, hidden adjacent files,
early preparation failures and a persistent scripted engine. These test the
adapter, not model loading or measured 100–600B operation. Split discovery and
mount preparation are host adapters, outside the processor refinement.

This boundary relies on Linux, Bubblewrap, filesystem/Unix socket semantics,
the trusted launcher, prepared public software and host configuration. Keep
engine/weight files (including every shard) unchanged by other trusted host
processes while in use;
a read-only bind mount is not a cryptographic artifact identity or a snapshot
against host-side changes. Resource exhaustion, GPU isolation, platform
equivalents and whole-controller/interface refinement remain separate work.
Only the shared authorization kernel has Rust-to-Lean refinement here; this
does not prove the launcher, transport, engine or OS isolation in Lean.
