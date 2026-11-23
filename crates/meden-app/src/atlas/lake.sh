set -euo pipefail
export LEAN_SYSROOT=/analysis/lean
export LEAN_PATH=/analysis/library
cp -R /analysis/sources/. /workspace/
exec /analysis/lean/bin/lean --load-dynlib=/analysis/lean/lib/lean/libLake_shared.so --run /analysis/library/MedenLake.lean "$@"
