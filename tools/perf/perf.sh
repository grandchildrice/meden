#!/usr/bin/env bash
# Host-only developer tool. Never mounted into coding/inference/checker workers.
set -euo pipefail
if [[ -n ${MEDEN_PERF:-} ]]; then
  exec "$MEDEN_PERF" "$@"
fi
if command -v perf >/dev/null; then
  exec perf "$@"
fi
# Resolve only perf; do not replace the installed host Rust/Lean toolchain.
package=$(nix build --no-link --print-out-paths \
  github:NixOS/nixpkgs/93a9ed0a866099a230ed5146a5bb7d16fd5c9b14#perf)
exec "$package/bin/perf" "$@"
