# Meden's own release evidence gate

This gates Meden's distribution, not projects edited by the coding agent.
It is the first production consumer of the
[evidence kernel](../crates/meden-core/src/evidence.rs). The general
natural-language contract/checker/delivery workflow remains a design target.

## Exact source to staged archives

[release.yml](../.github/workflows/release.yml) resolves its source ref once
to a commit. Both supported targets, `x86_64-unknown-linux-gnu` and
`aarch64-unknown-linux-gnu`, check out that exact commit and build with Rust
1.98.1, `cargo build --locked --release`. Each runs the actual binary's
offline `exec` backend before packaging. Linux is currently the only
supported workspace runtime; the package no longer advertises other systems.

Each target produces an archive, SHA-256 sidecar and source provenance JSON.
The provenance records repository, source commit, target, full Rust version
and the locked release build. It is an assertion of the trusted build
workflow, not an independently proved compiler attestation.

The collection job invokes the host-only command:

```bash
cargo run --locked -p meden-cli -- verify-release \
  --repository owner/meden --commit FULL_40_HEX_COMMIT --directory dist
```

`GH_TOKEN` or `GITHUB_TOKEN` can authorize the fixed GitHub API for private
repositories and rate limits. The command is not a model tool or a result
callback. It does not read receipt JSON from stdin. It sends no archive,
source contents or confidential workspace to GitHub: only repository,
commit and fixed workflow/run identifiers appear in its requests.

The production adapter:

1. Reads exactly the two archives and their four sidecars. It bounds sizes,
   rejects nonregular/symlink inputs, hashes actual archive bytes and checks
   checksums, repository, source, target and build configuration.
2. Retrieves the source commit's CI definition and requires byte equality
   with the definition embedded in the gate. An unrelated workflow with
   green jobs of the same names cannot satisfy this policy.
3. Selects the latest `ci.yml` main-push run for that commit. It requires
   completion and success; it does not filter for an older successful run.
4. Reads that exact run attempt's jobs. All twelve required checks must
   appear exactly once, with exact run/source identities and successful
   completed steps. Missing, duplicated, failed, cancelled, skipped or
   incomplete evidence fails. A second run lookup rejects an attempt change.
5. Uses the refined kernel to accept the compiled host checklist and record
   validated receipts, then rereads staged files. A changed subject resets
   progress; only exact fresh completion emits `release-evidence.json`.

The GitHub client has a fixed HTTPS origin, normal TLS checks, no environment
proxy, no redirect, response bounds and a timeout. API errors refuse release.
The token never appears in the report. The adapter does not accept an
arbitrary API base URL or model-selected checker configuration.

## What the manifest establishes

The report binds the repository/source, CI run and attempt, CI-definition
hash, exact archive hashes and build provenance to the accepted checklist.
The Lean job supplies scoped kernel/refinement evidence; the six Kani jobs
supply bounded evidence; Rust, Python, conformance, dependency and packaging
checks supply empirical evidence. A test is not promoted to a theorem.
The exact proof and test boundaries are in [verification.md](verification.md).

The kernel's six operations are extracted from production Rust. Lean proves
their correspondence to the independent specification, and that completion
after any finite composition requires passed matching receipts for every
accepted obligation on the current subject. This proves the decision rule
conditional on correct host attribution and bindings. It does not prove
GitHub, checker soundness, build provenance, compilers, OS or SHA-256.

The publication job does not modify the distribution after the gate. Its
npm checksum file is derived from the accepted archive manifest. This
workflow ordering assumes no untrusted concurrent writer on the trusted
runner. The command is a point-in-time check; arbitrary callers must not
mutate files after it and reuse its report. API parsing, hashing, filesystem
reads, workflow/build execution and publication adapters are not Lean-refined.
Checker binaries and the full build dependency closure are not independently
attested by this manifest.

## Dry runs and publication

A manual `workflow_dispatch` takes a `ref` input. Use an exact commit whose
main-push CI is complete. It builds both targets, runs the gate and uploads
archives, the manifest and the prepared npm directory as workflow artifacts.
It never creates a GitHub release or publishes to npm.

Only a pushed `v*` tag reaches publication. It must satisfy the same gate
and match the npm package version. GitHub release files include the archives,
sidecars and report. npm publication additionally requires `NPM_TOKEN`.
An absent token skips npm publication; a failed gate blocks both paths.

Component tests cover missing/wrong/stale CI, changed attempts, duplicate
jobs, failed steps, replaced archives with updated checksums, malformed
provenance and extra files. These fixtures test the adapter's interpretation;
the workflow dry run exercises real builds and GitHub result collection.
