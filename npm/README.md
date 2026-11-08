# meden

A conversational coding agent with isolated local inference, offline workspace
tools and scoped Lean refinement proofs. Development prototype; whole-product
correctness and the user-project verification workflow remain unfinished.

Linux x64/arm64, Bubblewrap and Bash are required. Prepare a compatible
llama.cpp server and GGUF weights before confidential work:

```bash
export MEDEN_LOCAL_ENGINE=/path/to/llama-server
export MEDEN_LOCAL_MODEL=/path/to/model.gguf
npx meden tui --workspace /path/to/project
```

The local model stays in a separate offline namespace. File tools and Bash
share a retained private project copy; the original is unchanged. HTTP model
connections are explicit and require host approval. Model proposals cannot
approve transfers. Documentation can be prefetched and attached as a fixed
local cache. See [setup and current limits](https://github.com/grandchildrice/meden#readme).

`npx` fetches the supported binary from GitHub releases; `MEDEN_BIN` selects a
local build. To install from source:

```bash
cargo install --git https://github.com/grandchildrice/meden meden-cli
```

Lean proofs cover the documented decision/state kernels. Release evidence
binds the source's required CI checks to staged archive hashes and trusted
build provenance; it is not proof of the complete delivered application.

Apache-2.0.
