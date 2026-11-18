set -euo pipefail
export LEAN_SYSROOT=/analysis/lean
export LEAN_PATH=/workspace
cp -R /analysis/library/LeanAtlas /workspace/
cp /analysis/library/LeanAtlas.lean /analysis/library/MedenAtlas.lean /analysis/library/MedenImports.lean /workspace/
for module in "$@"; do
  /analysis/lean/bin/lean -o "$module.olean" "$module.lean"
done
