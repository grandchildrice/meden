'use strict';

// Which prebuilt binary this machine needs.
//
// The names match the archives the release workflow uploads, so adding a
// platform means adding it in both places and nowhere else.

const TARGETS = {
  'linux x64': 'x86_64-unknown-linux-gnu',
  'linux arm64': 'aarch64-unknown-linux-gnu',
  'darwin x64': 'x86_64-apple-darwin',
  'darwin arm64': 'aarch64-apple-darwin',
  'win32 x64': 'x86_64-pc-windows-msvc',
};

/** The Rust target triple for this machine, or null if there is no build for it. */
function target() {
  return TARGETS[`${process.platform} ${process.arch}`] || null;
}

/** What the binary is called once unpacked. */
function binaryName() {
  return process.platform === 'win32' ? 'tagent.exe' : 'tagent';
}

/** The archive uploaded for a target. */
function archiveName(triple) {
  return `tagent-${triple}.tar.gz`;
}

/** Where a release archive lives. */
function downloadUrl(version, triple) {
  const base = 'https://github.com/grandchildrice/t-agent/releases/download';
  return `${base}/v${version}/${archiveName(triple)}`;
}

/** A sentence naming this platform, for an error message. */
function describe() {
  return `${process.platform} ${process.arch}`;
}

module.exports = { TARGETS, target, binaryName, archiveName, downloadUrl, describe };
