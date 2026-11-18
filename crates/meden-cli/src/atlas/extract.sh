set -euo pipefail
export LEAN_SYSROOT=/analysis/lean
export LEAN_PATH=/analysis/library:/workspace
exec /analysis/lean/bin/lean --run /analysis/library/MedenAtlas.lean "$@"
