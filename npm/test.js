#!/usr/bin/env node
'use strict';

// What can be checked without a network or a release: the platform mapping,
// the version agreeing with the Rust workspace, and the launcher preferring
// a local build.

const assert = require('assert');
const fs = require('fs');
const path = require('path');

const platform = require('./platform.js');
const launcher = require('./launcher.js');

// Every target has a plausible triple and a URL built from it.
for (const [key, triple] of Object.entries(platform.TARGETS)) {
  assert.match(triple, /^(x86_64|aarch64)-/, `odd triple for ${key}: ${triple}`);
  const url = platform.downloadUrl('1.2.3', triple);
  assert.ok(url.includes('/v1.2.3/'), url);
  assert.ok(url.endsWith(`tagent-${triple}.tar.gz`), url);
}

// This platform is one of them, or the launcher explains itself.
const here = platform.target();
assert.ok(here === null || Object.values(platform.TARGETS).includes(here));

// The published version matches the Rust workspace, or a download would ask
// the release for a version that does not exist.
const cargo = fs.readFileSync(path.join(__dirname, '..', 'Cargo.toml'), 'utf8');
const workspaceVersion = /^version\s*=\s*"([^"]+)"/m.exec(cargo);
assert.ok(workspaceVersion, 'no version in the workspace Cargo.toml');
assert.strictEqual(
  launcher.version(),
  workspaceVersion[1],
  'npm/package.json and Cargo.toml disagree about the version'
);

// A binary built in this checkout is used in preference to a download.
const built = ['release', 'debug']
  .map((p) => path.join(__dirname, '..', 'target', p, platform.binaryName()))
  .find((p) => fs.existsSync(p));
if (built) {
  assert.strictEqual(launcher.existingBinary(), built);
}

console.log('npm package checks passed');
