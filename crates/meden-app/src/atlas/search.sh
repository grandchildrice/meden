set -euo pipefail
export LEAN_SYSROOT=/analysis/lean
export LEAN_PATH=/analysis/library:/workspace/modules
exec /tools/python3 /analysis/library/search-atlas.py
