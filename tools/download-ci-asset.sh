#!/usr/bin/env bash
# Public CI tool preparation only. Every accepted byte is checksum-pinned.
set -euo pipefail
asset_path=$1
expected_sha=$2
destination=$3
for attempt in 1 2 3 4 5; do
  # A fresh query avoids replaying a cached gateway error or expired redirect.
  nonce="${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-1}-$attempt-$RANDOM"
  if timeout 120 gh api --method GET -H 'Accept: application/octet-stream' \
    "$asset_path?meden-download=$nonce" > "$destination"; then
    printf '%s  %s\n' "$expected_sha" "$destination" | sha256sum --check
    exit 0
  fi
  sleep 2
done
echo 'Pinned public CI tool download failed' >&2
exit 1
