set -euo pipefail
export LEAN_SYSROOT=/analysis/lean
export LEAN_PATH=/analysis/library:/workspace/modules
output="/workspace/modules/$1.olean"
mkdir -p "${output%/*}"
exec /analysis/lean/bin/lean --root="/analysis/sources/$3" -o "$output" "/analysis/sources/$2"
