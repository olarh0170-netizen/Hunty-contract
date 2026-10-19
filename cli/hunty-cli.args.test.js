#!/usr/bin/env node
// Checks that hunty-cli.js sends the argument names the deployed
// hunty-core contract expects (create_hunt, submit_answer,
// get_hunt_leaderboard). Run with: node cli/hunty-cli.args.test.js
const fs = require('fs');
const path = require('path');
const assert = require('assert');

const cliSrc = fs.readFileSync(path.join(__dirname, 'hunty-cli.js'), 'utf8');

// Contract spec: method -> ordered argument names (excluding Env).
// Mirrors contracts/hunty-core/src/lib.rs signatures.
const SPEC = {
  create_hunt: [
    'creator',
    'title',
    'description',
    'start_time',
    'end_time',
    'max_submissions_per_minute',
    'start_multiplier_bps',
    'default_points',
  ],
  submit_answer: [
    'hunt_id',
    'clue_id',
    'player',
    'answer',
    'submission_nonce',
    'submitted_at',
  ],
  get_hunt_leaderboard: ['hunt_id', 'limit'],
};

function cliArgsFor(method) {
  const start = cliSrc.indexOf(`'${method}'`);
  assert(start !== -1, `CLI never invokes ${method}`);
  const open = cliSrc.indexOf('{', start);
  const close = cliSrc.indexOf('}', open);
  const block = cliSrc.slice(open, close);
  return [...block.matchAll(/^\s*([a-z_]+):/gm)].map((m) => m[1]);
}

let failed = 0;
for (const [method, expected] of Object.entries(SPEC)) {
  const got = cliArgsFor(method);
  try {
    assert.deepStrictEqual(got, expected);
    console.log(`ok - ${method} args match contract spec`);
  } catch (e) {
    failed++;
    console.error(`FAIL - ${method}: CLI sends [${got}] but contract expects [${expected}]`);
  }
}
process.exit(failed ? 1 : 0);
