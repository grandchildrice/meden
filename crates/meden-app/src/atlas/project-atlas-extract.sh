set -euo pipefail
export PATH=/analysis/lean/bin:/tools
export LEAN_SYSROOT=/analysis/lean
export MATHLIB_NO_CACHE_ON_UPDATE=1
export ELAN_NO_NETWORK=1
export LAKE_NO_NETWORK=1
export LEAN_NUM_THREADS=2
cd /workspace
if [ "${1-}" = "--worker" ]; then
  shift
  export LEAN_PATH="/analysis/library:${LEAN_PATH-}"
  exec /tools/python3 /analysis/library/observe-atlas.py "$@"
fi
exec /analysis/lean/bin/lake env /tools/bash --noprofile --norc /analysis/library/project-atlas-extract.sh --worker "$@"
