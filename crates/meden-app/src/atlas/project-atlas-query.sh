set -euo pipefail
export PATH=/analysis/lean/bin:/tools
export LEAN_SYSROOT=/analysis/lean
export MATHLIB_NO_CACHE_ON_UPDATE=1
export ELAN_NO_NETWORK=1
export LAKE_NO_NETWORK=1
export LEAN_NUM_THREADS=2
cd /workspace
echo "$ lake --json query $*" >&2
exec /analysis/lean/bin/lake --no-ansi --json query "$@"
