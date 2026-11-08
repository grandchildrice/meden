#!/usr/bin/env bash
# Trusted checker recipe. No candidate Cargo/lake config or scripts are imported.
set -euo pipefail
ulimit -c 0
ulimit -v 16777216
export PATH=/verification/rust/bin:/bin:/usr/bin
export CARGO_HOME=/tmp/cargo
export CARGO_TARGET_DIR=/workspace/build
export CARGO_BUILD_JOBS=2
export LEAN_PATH=@LEAN_PATH@
export CHARON_TOOLCHAIN_IS_IN_PATH=1

case "$(rustc --version)" in
  'rustc 1.100.0-nightly (923c95cdf '*) ;;
  *) echo 'unsupported project Rust toolchain' >&2; exit 1 ;;
esac
case "$(/verification/lean/bin/lean --version)" in
  *'version 4.35.0,'*) ;;
  *) echo 'unsupported project Lean toolchain' >&2; exit 1 ;;
esac
case "$(/verification/charon version)" in
  *8cae8fd1905946de61147a65ecdd7122cc6b491b*) ;;
  *) echo 'unsupported project Charon revision' >&2; exit 1 ;;
esac
case "$(/verification/aeneas -version)" in
  *c5eb4a2*) ;;
  *) echo 'unsupported project Aeneas revision' >&2; exit 1 ;;
esac

printf 'meden-check: tests\n'
(ulimit -v 8388608; cargo test --offline --locked --release --lib)
printf 'meden-check: extraction\n'
(ulimit -v 8388608; /verification/charon cargo --preset=aeneas --sysroot default \
  --start-from meden_ledger::ledger::transfer --dest-file /workspace/candidate.llbc \
  -- --offline --locked --release --lib)
/verification/aeneas -backend lean -namespace LedgerImpl -dest /workspace \
  -abort-on-error -warnings-as-errors -no-progress-bar /workspace/candidate.llbc
if grep -En '\b(axiom|sorry|admit|unsafe)\b|#(eval|run)' Candidate.lean; then
  echo 'unsupported generated implementation model' >&2
  exit 1
fi
printf 'meden-check: proof\n'
/verification/lean/bin/lean -j2 -M4096 -o Spec.olean Spec.lean
/verification/lean/bin/lean -j2 -M4096 -o Candidate.olean Candidate.lean
/verification/lean/bin/lean -j2 -M4096 -o Check.olean Check.lean
printf 'meden-check: build\n'
(ulimit -v 8388608; cargo build --offline --locked --release --lib)
