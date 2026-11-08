#!/usr/bin/env bash
# Regenerate from production Rust; never normalize or hand-edit the model.
set -euo pipefail
cd "$(dirname "$0")/.."

charon_bin=${CHARON:-charon}
aeneas_bin=${AENEAS:-aeneas}
case $("$charon_bin" version) in
  *8cae8fd1905946de61147a65ecdd7122cc6b491b*) ;;
  *) echo 'Unsupported Charon revision' >&2; exit 1 ;;
esac
case $("$aeneas_bin" -version) in
  *c5eb4a2*) ;;
  *) echo 'Unsupported Aeneas revision' >&2; exit 1 ;;
esac

units=(control evidence approval repair failure)
case "$#:${1:-}:${2:-}" in
  0::|1:--write:) ;;
  2:--write:control|2:--write:evidence|2:--write:approval|2:--write:repair|2:--write:failure) units=("$2") ;;
  *) echo 'Usage: tools/extract-transfer.sh [--write [control|evidence|approval|repair|failure]]' >&2; exit 2 ;;
esac
extract_tmp=$(mktemp -d)
trap 'rm -rf "$extract_tmp"' EXIT
for unit in "${units[@]}"; do
  namespace=MedenImpl
  cargo_args=(--locked --lib -p meden-core)
  extraction_args=()
  case "$unit" in
    control)
      rust_roots=(meden_core::transfer::reserve meden_core::export_control::step)
      for method in new request phase advance; do
        rust_roots+=("meden_core::export_control::State::$method")
      done
      model=Control ;;
    evidence)
      rust_roots=()
      for method in propose review accept is_accepted refresh record eligible; do
        rust_roots+=("meden_core::evidence::Gate::$method")
      done
      for method in new start invalidate abort finish is_active complete; do
        rust_roots+=("meden_core::evidence::CheckJobs::$method")
      done
      model=Evidence ;;
    approval)
      rust_roots=(meden::tui::approval::approves)
      namespace=MedenUIImpl
      cargo_args=(--locked --bin meden -p meden-cli)
      # Extract the actual event/bitflag/char comparisons, rather than importing
      # unknown library functions. Keep unrelated standard internals opaque.
      extraction_args=(--extract-opaque-bodies --include crossterm --include bitflags
        --opaque core --opaque alloc --opaque std --include core::cmp::impls)
      model=Approval ;;
    repair)
      rust_roots=(meden::session::needs_verification)
      namespace=MedenRepairImpl
      cargo_args=(--locked --bin meden -p meden-cli)
      extraction_args=(--include meden_agent)
      model=Repair ;;
    failure)
      namespace=MedenCheckImpl
      cargo_args=(--locked --lib -p meden-runtime)
      rust_roots=(meden_runtime::verification::failure::failed_worker
        meden_runtime::verification::failure::verification_stops
        meden_runtime::verification::failure::FailureKind::repairable)
      extraction_args=(--include std::process --include std::sys --include std::os::unix)
      model=Failure ;;
  esac
  extract_roots=()
  for rust_root in "${rust_roots[@]}"; do
    extract_roots+=(--start-from "$rust_root")
  done
  CARGO_TARGET_DIR="$PWD/target/verification/rust" \
    "$charon_bin" cargo --preset=aeneas --sysroot default \
    "${extraction_args[@]}" "${extract_roots[@]}" \
    --dest-file "$extract_tmp/$unit.llbc" -- "${cargo_args[@]}"
  "$aeneas_bin" -backend lean -namespace "$namespace" -dest "$extract_tmp" \
    -abort-on-error -warnings-as-errors -no-progress-bar "$extract_tmp/$unit.llbc"
  if grep -En '\b(axiom|sorry|admit)\b' "$extract_tmp/$model.lean"; then
    echo 'Extraction left an unknown definition or proof hole' >&2
    exit 1
  fi
  case ${1:-} in
    --write) cp "$extract_tmp/$model.lean" "lean/MedenSpec/Generated/$model.lean" ;;
    '') diff -u "lean/MedenSpec/Generated/$model.lean" "$extract_tmp/$model.lean" ;;
  esac
done
