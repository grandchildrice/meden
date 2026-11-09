#!/usr/bin/env bash
# Trusted checker recipe. No candidate Cargo/lake config or scripts are imported.
set -euo pipefail
ulimit -c 0
ulimit -v 16777216
export PATH=/verification/rust/bin:/tools:/bin:/usr/bin
export LEAN_SYSROOT=/verification/lean
# Charon embeds rustc and needs this pinned sysroot's driver library. The
# namespace clears inherited loader paths; only prepared public files apply.
export LD_LIBRARY_PATH=/verification/rust/lib
export CARGO_HOME=/tmp/cargo
export CARGO_TARGET_DIR=/workspace/build
export CARGO_BUILD_JOBS=2
export LEAN_PATH=@LEAN_PATH@
export CHARON_TOOLCHAIN_IS_IN_PATH=1

# Reserved exits are interpreted only from this fixed recipe: 65 is a
# completed rejection, 66 a Lean rejection with structured diagnostics,
# 69 an unavailable checker, and 75 an interrupted/resource-limited check.
native_check() {
  "$@" && return 0
  meden_status=$?
  case "$meden_status" in
    126|127) exit 69 ;;
    *) if (( meden_status >= 128 )); then exit 75; fi; exit 65 ;;
  esac
}
lean_check() {
  printf 'meden-check: lean diagnostics\n'
  /verification/lean/bin/lean --json -j2 -M4096 "$@" 2>&1 && return 0
  meden_status=$?
  case "$meden_status" in
    126|127) exit 69 ;;
    *) if (( meden_status >= 128 )); then exit 75; fi; exit 66 ;;
  esac
}

case "$(rustc --version)" in
  'rustc 1.100.0-nightly (923c95cdf '*) ;;
  *) echo 'unsupported project Rust toolchain' >&2; exit 69 ;;
esac
case "$(/verification/lean/bin/lean --version)" in
  *'version 4.35.0,'*) ;;
  *) echo 'unsupported project Lean toolchain' >&2; exit 69 ;;
esac
case "$(/verification/charon version)" in
  *8cae8fd1905946de61147a65ecdd7122cc6b491b*) ;;
  *) echo 'unsupported project Charon revision' >&2; exit 69 ;;
esac
case "$(/verification/aeneas -version)" in
  *c5eb4a2*) ;;
  *) echo 'unsupported project Aeneas revision' >&2; exit 69 ;;
esac

printf 'meden-check: contract syntax\n'
/verification/lean/bin/lean -j2 -M4096 --run Terms.lean && meden_status=0 || meden_status=$?
case "$meden_status" in
  0) ;;
  65|69|75) exit "$meden_status" ;;
  *) if (( meden_status >= 128 )); then exit 75; fi; exit 69 ;;
esac
printf 'meden-check: contract properties\n'
printf 'meden-check: required output bounds and example consistency\n'
lean_check -o Spec.olean Spec.lean
lean_check AuditSpec.lean
if [[ ${meden_contract_only:-0} == 1 ]]; then
  exit 0
fi

printf 'meden-check: tests\n'
(ulimit -v 8388608; native_check cargo test --offline --locked --release --lib)
printf 'meden-check: extraction\n'
(ulimit -v 8388608; native_check /verification/charon cargo --preset=aeneas --sysroot default \
  --start-from meden_kernel::kernel::@ENTRY@ --dest-file /workspace/candidate.llbc \
  -- --offline --locked --release --lib)
native_check /verification/aeneas -backend lean -namespace KernelImpl -dest /workspace \
  -abort-on-error -warnings-as-errors -no-progress-bar /workspace/candidate.llbc
if grep -En '\b(axiom|sorry|admit|unsafe)\b|#(eval|run)' Candidate.lean; then
  echo 'unsupported generated implementation model' >&2
  exit 65
fi
printf 'meden-check: proof\n'
lean_check -o Candidate.olean Candidate.lean
printf 'meden-check: refinement\n'
lean_check -o Check.olean Check.lean
printf 'meden-check: build\n'
(ulimit -v 8388608; native_check cargo build --offline --locked --release --lib)
