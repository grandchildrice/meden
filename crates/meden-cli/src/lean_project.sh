set -eu
export PATH="/analysis/lean/bin:/tools"
export LEAN_SYSROOT=/analysis/lean
export MATHLIB_NO_CACHE_ON_UPDATE=1
export ELAN_NO_NETWORK=1
export LAKE_NO_NETWORK=1
export LEAN_NUM_THREADS=4
cd /workspace
if [ "${1-}" = "--meden-fingerprint" ]; then
  exec /tools/python3 /analysis/library/project_hash.py
fi
if [ "${1-}" = "--meden-inspect" ]; then
  shift
  exec /tools/python3 /analysis/library/inspect.py "$@"
fi
exec /analysis/lean/bin/lake "$@"
