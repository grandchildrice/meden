#!/usr/bin/env node
'use strict';

// Fetch the binary for this platform, once, at install time.
//
// Nothing here is required for correctness: the launcher downloads on demand
// if this did not run or did not succeed, so an install behind a firewall
// fails at the point where it can explain itself rather than silently.

const { fetchBinary } = require('./launcher.js');

fetchBinary()
  .then((path) => {
    console.log(`meden: ready at ${path}`);
  })
  .catch((err) => {
    // A failed download must not fail the install: the launcher will try
    // again, and by then the person is watching.
    console.warn(`meden: could not fetch the binary now (${err.message}).`);
    console.warn('meden: it will be fetched the first time you run it.');
  });
