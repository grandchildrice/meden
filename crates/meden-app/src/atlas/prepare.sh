set -euo pipefail
export LEAN_SYSROOT=/analysis/lean
export LEAN_PATH=/workspace
for module in "$@"; do
  echo "Compiling analysis tool: $module" >&2
  mkdir -p "$(dirname "$module")"
  cp "/analysis/library/$module.lean" "$module.lean"
  /analysis/lean/bin/lean -o "$module.olean" "$module.lean"
done
