#!/usr/bin/env node

const { execFileSync } = require('child_process');

function showHelp() {
  console.log(`
Hunty CLI Tool
Usage: node hunty-cli.js <command> [options]

Commands:
  create-hunt     --title <title> --description <desc> --start <timestamp> --end <timestamp> --creator <address> [--max-submissions-per-minute <n> --start-multiplier-bps <bps> --default-points <n>]
  add-clue        --hunt <hunt_id> --answer <answer> --points <points>
  activate        --hunt <hunt_id> --caller <address>
  register        --hunt <hunt_id> --player <address>
  submit-answer   --hunt <hunt_id> --clue <clue_id> --answer <answer> --player <address> [--nonce <n> --submitted-at <timestamp>]
  leaderboard     --hunt <hunt_id> [--limit <n>]
  player-stats    --hunt <hunt_id> --player <address>
  pause           --admin <address>
  unpause         --admin <address>

Global Options:
  --network       Network to use (default: testnet)
  --contract      Contract ID (required)
  --source        Source account/identity for the transaction
`);
}

function parseArgs(args) {
  const result = {};
  for (let i = 0; i < args.length; i++) {
    if (args[i].startsWith('--')) {
      const key = args[i].substring(2);
      const val = args[i + 1] && !args[i + 1].startsWith('--') ? args[i + 1] : true;
      result[key] = val;
      if (val !== true) i++;
    }
  }
  return result;
}

const SENSITIVE_KEYS = new Set([
  'answer',
  'invite_code',
  'invite-code',
  'secret',
  'password',
  'private_key',
  'private-key',
  'mnemonic',
  'seed',
]);

function redactArgs(args) {
  const redacted = [];
  for (let i = 0; i < args.length; i++) {
    redacted.push(args[i]);
    if (args[i].startsWith('--') && SENSITIVE_KEYS.has(args[i].substring(2)) && i + 1 < args.length) {
      redacted.push('[REDACTED]');
      i++;
    }
  }
  return redacted;
}

function runInvoke(method, contractArgs, options) {
  if (!options.contract) {
    console.error("Error: --contract is required.");
    process.exit(1);
  }

  const args = [
    'contract', 'invoke',
    '--id', options.contract,
    '--network', options.network || 'testnet'
  ];
  if (options.source) {
    args.push('--source', options.source);
  }

  args.push('--', method);

  for (const [key, val] of Object.entries(contractArgs)) {
    args.push(`--${key}`, String(val));
  }

  console.log(`Executing: stellar ${redactArgs(args).join(' ')}`);
  try {
    const output = execFileSync('stellar', args, { encoding: 'utf-8' });
    console.log("Result:", output);
  } catch (error) {
    console.error("Error executing command:");
    console.error(error.stdout || error.message);
    process.exit(1);
  }
}

function main() {
  const args = process.argv.slice(2);
  if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
    showHelp();
    return;
  }

  const command = args[0];
  const options = parseArgs(args.slice(1));

  switch (command) {
    case 'create-hunt':
      runInvoke('create_hunt', {
        creator: options.creator,
        title: options.title || options.name,
        description: options.description || "",
        start_time: options.start,
        end_time: options.end,
        max_submissions_per_minute: options['max-submissions-per-minute'] || options.max_submissions_per_minute || 10,
        start_multiplier_bps: options['start-multiplier-bps'] || options.start_multiplier_bps || 10000,
        default_points: options['default-points'] || options.default_points || 100
      }, options);
      break;
    
    case 'add-clue':
      runInvoke('add_clue', {
        hunt_id: options.hunt,
        question: options.question || "Default Question",
        answer: options.answer,
        points: options.points,
        is_required: options.required || false,
        difficulty: options.difficulty || 1
      }, options);
      break;
      
    case 'activate':
      runInvoke('activate_hunt', {
        hunt_id: options.hunt,
        caller: options.caller
      }, options);
      break;
      
    case 'register':
      runInvoke('register_player', {
        hunt_id: options.hunt,
        player: options.player
      }, options);
      break;

    case 'submit-answer':
      runInvoke('submit_answer', {
        hunt_id: options.hunt,
        clue_id: options.clue,
        player: options.player,
        answer: options.answer,
        submission_nonce: options.nonce || Math.floor(Math.random() * 0xffffffff),
        submitted_at: options['submitted-at'] || options.submitted_at || Math.floor(Date.now() / 1000)
      }, options);
      break;
      
    case 'leaderboard':
      runInvoke('get_hunt_leaderboard', {
        hunt_id: options.hunt,
        limit: options.limit || 10
      }, options);
      break;

    case 'player-stats':
      runInvoke('get_player_progress', {
        hunt_id: options.hunt,
        player: options.player
      }, options);
      break;
      
    case 'pause':
      runInvoke('pause_contract', {
        admin: options.admin
      }, options);
      break;
      
    case 'unpause':
      runInvoke('unpause_contract', {
        admin: options.admin
      }, options);
      break;
      
    default:
      console.error(`Unknown command: ${command}`);
      showHelp();
      process.exit(1);
  }
}

main();
