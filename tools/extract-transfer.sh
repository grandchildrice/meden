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

extract_tmp=$(mktemp -d)
trap 'rm -rf "$extract_tmp"' EXIT
CARGO_TARGET_DIR="$PWD/target/verification/rust" \
  "$charon_bin" cargo --preset=aeneas --sysroot default \
  --start-from meden_core::transfer::reserve \
  --dest-file "$extract_tmp/transfer.llbc" -- --locked --lib -p meden-core
"$aeneas_bin" -backend lean -namespace MedenImpl -dest "$extract_tmp" \
  -abort-on-error -warnings-as-errors -no-progress-bar "$extract_tmp/transfer.llbc"
if grep -En '\b(axiom|sorry|admit)\b' "$extract_tmp/Transfer.lean"; then
  echo 'Extraction left an unknown definition or proof hole' >&2
  exit 1
fi
case ${1:-} in
  --write) cp "$extract_tmp/Transfer.lean" lean/MedenSpec/Generated/Transfer.lean ;;
  '') diff -u lean/MedenSpec/Generated/Transfer.lean "$extract_tmp/Transfer.lean" ;;
  *) echo 'Usage: tools/extract-transfer.sh [--write]' >&2; exit 2 ;;
esac
