#!/usr/bin/env bash
# Embedded, trusted first-launch preparation. No workspace/model input, secrets
# or project scripts. Runs before import; all subsequent checker jobs are offline.
set -euo pipefail
root=$1
host_home=$2
built_lean=$3
mkdir -p "$HOME"
cd /
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) ;;
  *) echo 'Automatic verifier installation currently requires Linux x86_64. A prepared MEDEN_VERIFIER_BUNDLE can select another supported host installation.' >&2; exit 1 ;;
esac

download() {
  local url=$1 expected=$2 target=$3
  if [[ -f "$target" ]] && printf '%s  %s\n' "$expected" "$target" | sha256sum --check --status; then
    return
  fi
  echo 'Downloading a pinned public verification tool…' >&2
  local attempt
  for attempt in 1 2 3; do
    if curl --fail --location --proto '=https' --proto-redir '=https' \
        --connect-timeout 20 --max-time 600 --silent --show-error \
        -H 'Accept: application/octet-stream' \
        "$url" -o "$target.partial" \
        && printf '%s  %s\n' "$expected" "$target.partial" | sha256sum --check --status; then
      mv "$target.partial" "$target"
      return
    fi
  done
  echo 'Could not download the pinned verification tool with its expected checksum.' >&2
  return 1
}

rustup_home=${RUSTUP_HOME:-$host_home/.rustup}
rust="$rustup_home/toolchains/nightly-2026-09-17-x86_64-unknown-linux-gnu"
rust_ready() {
  [[ -x "$rust/bin/rustc" && -x "$rust/bin/cargo" ]] \
    && compgen -G "$rust/lib/librustc_driver-*.so" >/dev/null
}
if ! rust_ready; then rust="$root/rust"; fi
if ! rust_ready; then
  echo 'Installing the dedicated verification Rust toolchain…' >&2
  # These component digests come from Rust's dated distribution manifest.
  # A private prefix avoids installing Rustup or changing the coding toolchain.
  for component in \
      rustc:6c4b66572411880b75f845892414e9b0ea04dfbbc50e0ce5394bd162c3e0dda6 \
      cargo:155812325ded8f28460f91584a641b38ffb6292c1f7ba993c464376dd2e849af \
      rust-std:ce0462a1f0c3d6bf22fb952918a9840f1e85e6c1c3edd1d7840a90ab0a08dd8c \
      rustc-dev:53e753dcf3977aa70779320f6dc3819b94f50a0529ac54725cb2e496a6b7fccb \
      llvm-tools:ff6d3cde983c620a4c57b7baa79de66421ba51f1a95cc2d84a1397c8529bc14e; do
    name=${component%%:*}
    archive="$name-nightly-x86_64-unknown-linux-gnu"
    download "https://static.rust-lang.org/dist/2026-09-17/$archive.tar.xz" \
      "${component#*:}" "$root/$archive.tar.xz"
    unpack=$(mktemp -d "$root/rust-component.XXXXXX")
    tar -xJf "$root/$archive.tar.xz" -C "$unpack"
    bash "$unpack/$archive/install.sh" --prefix="$rust" --disable-ldconfig >&2
    rm -rf "$unpack"
  done
  download https://static.rust-lang.org/dist/2026-09-17/rust-src-nightly.tar.xz \
    cd272bc7b901f15464a44cbae891c3c7fd1847d6309eb7546fbf785bcadc8d7c "$root/rust-src-nightly.tar.xz"
  unpack=$(mktemp -d "$root/rust-source.XXXXXX")
  tar -xJf "$root/rust-src-nightly.tar.xz" -C "$unpack"
  bash "$unpack/rust-src-nightly/install.sh" --prefix="$rust" --disable-ldconfig >&2
  rm -rf "$unpack"
fi
case "$("$rust/bin/rustc" --version)" in
  'rustc 1.100.0-nightly (923c95cdf '*) ;;
  *) echo 'The dedicated verification compiler has an unsupported version.' >&2; exit 1 ;;
esac

lean="${ELAN_HOME:-$host_home/.elan}/toolchains/leanprover--lean4---v4.35.0"
if [[ ! -x "$lean/bin/lean" || ! -x "$lean/bin/lake" ]]; then
  lean="$root/lean"
  if [[ ! -x "$lean/bin/lean" || ! -x "$lean/bin/lake" || ! -f "$lean/lib/lean/Init.olean" ]]; then
    download "https://api.github.com/repos/leanprover/lean4/releases/assets/634574462?meden-setup=$RANDOM" \
      7289597a44543246c9227b8cae501767e095ce17f6d448b95fc4a8dc97268f6d "$root/lean.tar.zst"
    mkdir -p "$lean"
    tar --zstd -xf "$root/lean.tar.zst" --strip-components=1 -C "$lean"
  fi
fi
case "$("$lean/bin/lean" --version)" in
  *'version 4.35.0,'*) ;;
  *) echo 'The verification Lean installation has an unsupported version.' >&2; exit 1 ;;
esac

extraction="$root/extraction"
if [[ ! -x "$extraction/aeneas" || ! -x "$extraction/charon" || ! -x "$extraction/charon-driver" ]]; then
  download "https://api.github.com/repos/AeneasVerif/aeneas/releases/assets/719588396?meden-setup=$RANDOM" \
    14fbaed4ce8624cf3d668f887d6ebb237a59cc37d7498c792d9f3f97332cbb25 "$root/aeneas.tar.gz"
  mkdir -p "$extraction"
  tar -xzf "$root/aeneas.tar.gz" -C "$extraction" aeneas charon charon-driver
fi
case "$("$extraction/charon" version)" in
  *8cae8fd1905946de61147a65ecdd7122cc6b491b*) ;;
  *) echo 'The verification extractor has an unsupported version.' >&2; exit 1 ;;
esac
case "$("$extraction/aeneas" -version)" in
  *c5eb4a2*) ;;
  *) echo 'The verification translator has an unsupported version.' >&2; exit 1 ;;
esac

# Reuse only the compiled binary's known source tree, with the exact embedded
# dependency lock; never search the current working directory or candidate.
libraries="$root/libraries"
if [[ -f "$built_lean/.lake/packages/aeneas/backends/lean/.lake/build/lib/lean/Aeneas.olean" ]] \
    && cmp -s "$built_lean/lake-manifest.json" "$libraries/lake-manifest.json" \
    && cmp -s "$built_lean/lean-toolchain" "$libraries/lean-toolchain"; then
  libraries="$built_lean"
else
  echo 'Preparing the pinned public proof libraries (first launch only)…' >&2
  cd "$libraries"
  export PATH="$lean/bin:$PATH"
  "$lean/bin/lake" exe cache get >&2
  "$lean/bin/lake" build Aeneas >&2
fi
cd "$libraries"
library_path=$("$lean/bin/lake" env printenv LEAN_PATH)
printf '%s\n%s\n%s\n' "$rust" "$lean" "$extraction"
IFS=: read -ra library_dirs <<< "$library_path"
for directory in "${library_dirs[@]}"; do
  [[ -d "$directory" ]] || continue
  directory=$(realpath "$directory")
  [[ "$directory" == "$lean/lib/lean" || "$directory" == "$libraries/.lake/build/lib/lean" ]] && continue
  printf '%s\n' "$directory"
done
