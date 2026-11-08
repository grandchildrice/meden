# Isolated local inference

CLI and TUI use prepared local inference by default. Install Bubblewrap,
Bash and a llama.cpp `llama-server` with Unix socket support, and prepare a
GGUF model before confidential work. No component downloads models or
silently switches to an HTTP processor during a local session.

```bash
export MEDEN_LOCAL_ENGINE=/path/to/llama-server
export MEDEN_LOCAL_MODEL=/path/to/model.gguf
meden tui --workspace /path/to/project --pages pages.json

printf '%s\n' '{"instruction":"Inspect the project, fix the bug and run tests."}' | \
  meden run --workspace /path/to/project
```

`--local-engine PATH` and `--local-model PATH` override those environment
settings. The default path fails if either asset is missing. HTTP discovery
is explicit: `meden providers` lists connections, `--provider ID` selects
one, and `--provider auto` opts into discovery/the TUI picker. HTTP processing
still requires the existing trusted TUI approval or exact CLI
`--approve-processor` URL. Environment HTTP providers are ignored in a local
session. A pre-existing loopback server is an HTTP processor, not a confined
local engine.

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
the alias `meden-local`. It does not expose GPU devices or arbitrary engine
arguments. The executable must support `--host PATH.sock`, `--model`,
`--offline`, `--no-webui`, `--no-ui-mcp-proxy`, `--jinja`, `--n-gpu-layers`,
`--alias` and `--ctx-size`. This was exercised with llama.cpp 0.9.0, build
12110 (`cd959ba`), and a prepared Qwen3-0.6B Q4_K_M GGUF. A real model returned
a greeting through `meden run`; that smoke check is not a coding benchmark.
The [smoke model artifact](https://huggingface.co/bartowski/Qwen_Qwen3-0.6B-GGUF/blob/60b85c0e3d8fe0f6474f406922a26d12aca4550d/Qwen_Qwen3-0.6B-Q4_K_M.gguf)
was 484220320 bytes with SHA-256
`9acfc1e001311f34b4252001b626f2e466d592a42065f66571bff3790d4e1b14`.
The [llama.cpp server reference](https://github.com/ggml-org/llama.cpp/tree/master/tools/server)
documents the socket listener and native tool-calling API. The selected
model must supply a compatible chat/tool template.

[Adversarial CLI/TUI tests](../python/tests/test_local_inference.py) use a
deliberately hostile engine and observed TCP/UDP/Unix receivers, including
inheritable host capabilities, confidential context, forged approval,
redirects, disconnects, oversized responses and a symlink to a host service.
They also perform real local edits/tests, retain the warm process, check the
result independently in another offline copy, and cancel pending inference.
Rust transport tests replace the socket name after pinning and verify the
original receiver still gets the connection; an unresponsive peer times out.

This boundary relies on Linux, Bubblewrap, filesystem/Unix socket semantics,
the trusted launcher, prepared public software and host configuration. Keep
engine/weight files unchanged by other trusted host processes while in use;
a read-only bind mount is not a cryptographic artifact identity or a snapshot
against host-side changes. Resource exhaustion, GPU isolation, platform
equivalents and whole-controller/interface refinement remain separate work.
Only the shared authorization kernel has Rust-to-Lean refinement here; this
does not prove the launcher, transport, engine or OS isolation in Lean.
