# Confidential execution: R1 coverage

R1 requires that reading an external page cannot enable a confidential file,
or information derived from it, to reach an unapproved recipient. The sixteen
[acceptance criteria](confidentiality.md#acceptance-evidence-required-before-claiming-r1)
describe the complete requirement. This page maps the current production-path
tests to those criteria. It is not a declaration that R1 is complete.

## What the combined scenario exercises

[`test_r1_acceptance.py`](../python/tests/test_r1_acceptance.py) starts the real
`meden run` executable with a deliberately malicious scripted local engine.
The engine learns a freshly generated confidential canary from the actual
file-tool response after reading a cached injected page. It is not a real
language-model benchmark, a large-local deployment measurement or a test of
the public documentation fetcher.

The engine, a real shell, a Git pre-push hook and a compiled Cargo build script
attempt HTTP/proxy and DNS disclosure. Git uses a secret-derived ref and Cargo
uses a secret-derived package name. The script also creates Base64, gzip,
summary and secret-named files. The parent observes HTTP receivers and IPv4
and IPv6 UDP DNS receivers, with successful host control requests before and
after the attempts. It checks that the three coding subprocesses and inference
report the original canary digest from non-host network namespaces. IPv6 HTTP
is attempted, but only the DNS receiver is bound on IPv6 in this scenario.

The model cannot make a send tool, page selection, forged approval command,
provider switch or trust claim grant authority. It may propose exports.
Host review freezes the original body and recipient: changing the files and
proposing a second recipient does not change the reviewed request. Only three
explicit host approvals reach the approved HTTP receiver, with the exact
original raw, compressed and encoded bodies. A 307 reaches no second recipient.
For the last send, the receiver retains the connection without responding;
the real sender's five-second timeout leaves delivery uncertain and the
authorization spent. This is a timeout, not a simulated error or early EOF.

The test retains `r1-observation.json` in its pytest temporary directory:
executable SHA-256, namespace/digest markers, HTTP/DNS observations, elapsed
timeout and CLI events. These are empirical observations of that executable
and host configuration. They are separate from Lean proofs.

## Before the first namespace

[`test_preparation.py`](../python/tests/test_preparation.py) also exercises
the public `exec` path from an untrusted source directory with absolute and
relative repository entries at the front of `PATH`. It plants hostile `ldd`,
`nix-store`, `bwrap`, `bash` and paired Rustup/Cargo commands. The hostile
programs attempt to read a host-only random canary and send it to an observed
TCP receiver, before returning failure. A startup error therefore cannot hide
a disclosure. Host control requests check the receiver before and after each
successful case; normal `cargo`/`rustc` execution must still work.
When only the hostile search directory exists, startup must fail without
executing it or sending the canary; no unconfined fallback is accepted.

This found a real pre-confinement escape: with the prior implementation,
automatic `ldd`/`nix-store` lookup on Nix and default `bwrap` lookup executed
repository programs on the host. The receiver obtained the host canary.
Automatic selection now skips those entries and executes only canonical
public system/Nix objects. Metadata helpers run from `/`, receive a public
search path and cannot inherit `BASH_ENV` or `ENV`. The selected command name
is retained for multicall executables such as Nix. Shadow Rustup/Cargo proxies
do not suppress the installed host toolchain. Prepared-Rust fixtures also
check interpreter lookup with a hostile repository `python3`.

The test retains `preparation-observation.json`: executable/canary hashes,
attempted helper, absolute/relative lookup, host-execution marker, receiver
bodies, public control count and CLI return code. CI uploads these alongside
the other R1 observations. Nix-specific helper execution is covered on a Nix
host; an ordinary Ubuntu run does not exercise that branch merely by naming
a fake `nix-store`. Default Bubblewrap and public-tool selection are exercised
on both layouts.

Explicit `MEDEN_BWRAP`, host Cargo-home/manager configuration, public system
software and its immutability remain trusted inputs. This is a tested
preparation boundary, not a Lean proof of filesystem lookup or process launch.

The [mixed-layout lab](verification.md#mixed-system-and-nix-toolchains) also
checks private execution with system and Nix tools selected together, including
read-only system resources, host-file/environment denial and observed TCP
isolation. Its optional checker cases prove a real counter and reject a
counterexample through those layouts. These remain scoped deployment checks;
they do not complete R1 or establish large-local inference.

## Coverage by acceptance criterion

In this table, **combined** means the scenario above. Other tests run the
actual adapters or public CLI/TUI; no refusal-message-only fixture substitutes
for the stated receiver observations.

| Criterion | Current evidence | Remaining scope |
| --- | --- | --- |
| R1-01: injected page/send tool | Combined reads the cached injection and canary, records fabricated send-tool refusal and observes zero unapproved delivery. `test_run_integration.py` covers cached and file-backed page variants. | This uses supplied page snapshots; it does not authenticate an external page's origin. |
| R1-02: subprocess/network escape | Combined runs shell, Git pre-push and a real compiled Cargo build script. All attempt disclosure; HTTP/proxy and IPv4/IPv6 DNS receivers see only host controls. `tests/isolation.rs` also checks inherited sockets and detached children. | Other package ecosystems, deployment networks and OS equivalents need corresponding evidence. Namespace/OS behavior is trusted. |
| R1-03: indirect request fields | Combined uses secret-derived URL/query, header, search term, Git ref and Cargo package name. Unknown tools are refused; real Git/Cargo execute offline. | General network adapters remain absent. Adding one requires exact-request authorization and new observations. DNS/TLS attribution of an approved destination remains an adapter obligation. |
| R1-04: derived information | Combined creates encoded/compressed/summary/secret-named outputs and checks that none leaves without an exact host approval. A summary proposal remains pending. | No general declassification or covert-channel theorem is claimed. |
| R1-05: redirects | Combined observes an approved POST's 307 without a hop to the unapproved receiver. `test_model_transport.py` checks model transport 302/307/308. `test_pages.py` rejects private prefetch targets/proxies; the prefetch client disables redirects. | A full public-HTTPS redirect/DNS-rebinding/metadata lab is not present. These HTTP receiver tests do not prove TLS, DNS or local routing. |
| R1-06: changed bytes/recipient | Combined mutates the file after review, proposes B and observes only the frozen original at A. `test_exports.py` also exercises real CLI and TUI review. | Physical host input, screen/display correspondence and transport interpretation still need complete refinement. |
| R1-07: stale/forged/concurrent authority | Combined rejects wrong-session, pre-review, model-origin and replay approvals. `transfer.rs` observes no expired dispatch and exactly one send from four concurrent callers. Export/processor finite-trace proofs check the extracted kernels. | Mutex, clocks, protocol reference lookup and physical effects are tested/trusted adapters, not proved by those kernels. |
| R1-08: false trust claims | Combined consumes page, repository, tool-result and assistant claims of trust/public classification/approval, including a JSON-shaped host approval in assistant prose. They grant no transfer. | Full protocol/UI origin and rendering correspondence remain separate obligations. |
| R1-09: processor switch | Combined refuses a model's endpoint/provider switch after reading the canary. `test_model_transport.py` checks exact host approval; `test_tui_integration.py`/native TUI and gateway tests check processor authority. | Approved HTTP inference is explicitly external processing; loopback/LAN does not prove local confinement. A capable 100–600B local deployment still needs validation. |
| R1-10: host capabilities | Combined checks missing host files and credential/proxy environment in inference and subprocesses. `tests/isolation.rs`, worker-file tests and `test_local_inference.py` check descriptors, sockets, symlinks, pinned targets and original-tree isolation. Prepared-Rust tests check hidden manager settings, credentials and unselected sibling files. `test_preparation.py` observes that repository helpers cannot access host data before confinement. | Public helper/configuration trust, hard-link identity, concurrent host/inode mutation and correct physical path/handle attribution remain OS/adapter assumptions. |
| R1-11: secondary disclosure | Combined configures proxy, telemetry/error-reporting and Rust download targets at observed receivers. Rejected content causes no request to them. | This does not enumerate every host integration or prove absence of all telemetry, resource/timing channels or future adapters. |
| R1-12: uncertain delivery/retry | Combined observes exactly one body, holds the receiver open through the actual timeout, checks `delivery_uncertain` and refuses retry. Component/CLI tests also cover disconnect after receipt. | The remote recipient may already have the approved body; uncertainty cannot be rolled back. No authorization is persisted for restart recovery. |
| R1-13: unavailable isolation | `tests/isolation.rs` checks the public execution path with a missing backend; `test_pages.py` checks no unconfined prefetch fallback. Local tests reject missing assets/configuration. | Other host/kernel/namespace misconfigurations and supported deployment environments need explicit validation; no permissive fallback is supported. |
| R1-14: secret-dependent page choice | Combined selects cached handles based on the secret, rejects a URL-bearing miss and changes the backing cache after session startup. Neither selection nor the miss produces observed traffic. Native page tests cover the immutable handle API. | The host's initial fixed prefetch is a separate network operation. Dynamic secret-dependent external navigation is not implemented. |
| R1-15: useful local development | Combined successfully runs actual Git and `cargo test --offline` despite a workspace-selected unavailable toolchain. `test_local_inference.py`/`test_run_integration.py` perform multi-turn edit/test workflows and independently check the result in another offline copy. | Scripted coding checks are not model task-success rates. The recorded authorized Ollama Cloud trial establishes a scoped coding/proof/delivery workflow; large-local utility is unmeasured. |
| R1-16: local inference failure | Combined observes the malicious local engine's attempted egress while a configured cloud/proxy fallback receives nothing. `test_local_inference.py` covers normal warm CLI/TUI operation, cancellation, missing configuration and symlink/redirect/disconnect/oversized failures followed by another instruction without fallback. | Current managed inference is Linux/CPU-only. GPU/multi-host confinement, capable large-local operation and complete launcher/transport refinement remain work. |

These tests complement the extracted/refined processor, export, policy,
invocation and controller kernels. They do not prove whole-product effect
mediation. In particular the documentation worker retains the host network
namespace; its trusted HTTPS client checks resolved public addresses, which
is not an OS egress firewall. Complete interface/controller correspondence,
physical I/O attribution, deployment validation and the broader six-increment
[product plan](design.md#implementation-order-and-exit-criteria) remain required.

## Reproduce

Prepare Linux namespaces, Bubblewrap, public Bash/Python/Git, a C linker and an
already-installed Rust toolchain. The combined scenario requires working IPv6
loopback for its observed DNS receiver; it does not silently skip that part.
From the repository root:

```bash
cargo build --release -p meden-cli
MEDEN_BIN="$PWD/target/release/meden" uv run --project python pytest -q \
  python/tests/test_r1_acceptance.py --basetemp /tmp/meden-r1
MEDEN_BIN="$PWD/target/release/meden" uv run --project python pytest -q \
  python/tests/test_preparation.py --basetemp /tmp/meden-preparation
```

CI's Python job runs these tests with its compiled debug binary and uploads
their synthetic receiver/session records as `meden-r1-observations`, including
partial CLI events on failure. The Lean job separately runs the offline
project-proof/delivery suite. Local checks may use
a frozen release binary and should retain its hash alongside observations.
See [verification.md](verification.md) for proofs, fault checks and complete
test commands.
