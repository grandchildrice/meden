#!/usr/bin/env node
'use strict';

// Find the tagent binary and run it.
//
// In order: an explicit TAGENT_BIN, one already downloaded beside this file,
// a local cargo build (so a checkout runs its own code), then the release
// for this platform.

const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync, execFileSync } = require('child_process');
const crypto = require('crypto');

const platform = require('./platform.js');

const PACKAGE_DIR = __dirname;
const BIN_DIR = path.join(PACKAGE_DIR, 'bin');

/** The version this package was published at. */
function version() {
  return require('./package.json').version;
}

/** A binary already present, or null. */
function existingBinary() {
  const candidates = [];
  if (process.env.TAGENT_BIN) candidates.push(process.env.TAGENT_BIN);
  candidates.push(path.join(BIN_DIR, platform.binaryName()));
  // A checkout runs its own build rather than a download.
  for (const profile of ['release', 'debug']) {
    candidates.push(path.join(PACKAGE_DIR, '..', 'target', profile, platform.binaryName()));
  }
  return candidates.find((p) => {
    try {
      return fs.statSync(p).isFile();
    } catch {
      return false;
    }
  }) || null;
}

/** The expected SHA-256 for an archive, when the package records one. */
function expectedChecksum(archive) {
  try {
    return require('./checksums.json')[archive] || null;
  } catch {
    return null;
  }
}

/** Download, check and unpack the binary. Returns its path. */
async function fetchBinary() {
  const existing = existingBinary();
  if (existing) return existing;

  const triple = platform.target();
  if (!triple) {
    throw new Error(
      `no prebuilt binary for ${platform.describe()}; build from source: ` +
        'cargo install --git https://github.com/grandchildrice/t-agent tagent-cli'
    );
  }

  const url = platform.downloadUrl(version(), triple);
  const response = await fetch(url, { redirect: 'follow' });
  if (!response.ok) {
    throw new Error(`${url} returned ${response.status}`);
  }
  const bytes = Buffer.from(await response.arrayBuffer());

  const archive = platform.archiveName(triple);
  const expected = expectedChecksum(archive);
  if (expected) {
    const actual = crypto.createHash('sha256').update(bytes).digest('hex');
    if (actual !== expected) {
      throw new Error(
        `checksum mismatch for ${archive}: expected ${expected}, got ${actual}`
      );
    }
  }

  fs.mkdirSync(BIN_DIR, { recursive: true });
  const tmp = path.join(os.tmpdir(), `tagent-${process.pid}.tar.gz`);
  fs.writeFileSync(tmp, bytes);
  try {
    // tar is present on Linux, macOS and Windows 10 onwards.
    execFileSync('tar', ['-xzf', tmp, '-C', BIN_DIR], { stdio: 'inherit' });
  } finally {
    fs.rmSync(tmp, { force: true });
  }

  const binary = path.join(BIN_DIR, platform.binaryName());
  if (!fs.existsSync(binary)) {
    throw new Error(`${archive} did not contain ${platform.binaryName()}`);
  }
  if (process.platform !== 'win32') fs.chmodSync(binary, 0o755);
  return binary;
}

async function main() {
  let binary;
  try {
    binary = await fetchBinary();
  } catch (err) {
    console.error(`tagent: ${err.message}`);
    process.exit(1);
  }
  const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });
  if (result.error) {
    console.error(`tagent: could not run ${binary}: ${result.error.message}`);
    process.exit(1);
  }
  process.exit(result.status === null ? 1 : result.status);
}

module.exports = { fetchBinary, existingBinary, version };

if (require.main === module) main();
