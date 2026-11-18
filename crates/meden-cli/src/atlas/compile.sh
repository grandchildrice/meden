set -euo pipefail
export LEAN_SYSROOT=/analysis/lean
export LEAN_PATH=/analysis/library:/workspace
exec /analysis/lean/bin/lean --root=/analysis/sources -o "$1.olean" "/analysis/sources/$1.lean"
