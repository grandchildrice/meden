set -euo pipefail
export PATH=/analysis/lean/bin:/tools
export LEAN_SYSROOT=/analysis/lean
export MATHLIB_NO_CACHE_ON_UPDATE=1
export ELAN_NO_NETWORK=1
export LAKE_NO_NETWORK=1
export LEAN_NUM_THREADS=2
cd /workspace
exec /tools/python3 -I /analysis/library/project-atlas.py "$@"
