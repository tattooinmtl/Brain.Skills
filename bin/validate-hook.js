#!/usr/bin/env node
/**
 * validate-hook.js — minimal hook validator.
 *
 * Checks:
 *   - JSON parses
 *   - name, trigger, command are present non-empty
 *   - trigger is a known event
 *   - no obvious unsafe shell patterns
 *   - no obvious hardcoded secrets
 */

const fs = require('fs');

const KNOWN = ['PreToolUse','PostToolUse','PreInvocation','PostInvocation','SessionStart','Stop',
               'pre_tool_use','post_tool_use','pre_invocation','post_invocation','session_start','stop'];

const SECRETS = [
  /(?:api[_-]?key|apikey|secret|token|password|passwd)\s*[:=]\s*["]?[A-Za-z0-9+/=]{20,}/gi,
  /sk-[A-Za-z0-9]{20,}/g,
  /ghp_[A-Za-z0-9_]{20,}/g,
];

const UNSAFE = [
  /;\s*rm\s+-rf\s+\/?/i,
  /\|\s*sh\b/i,
  /\|\s*bash\b/i,
  /git\s+push\s+--force/i,
  /gh\s+repo\s+delete/i,
];

function main() {
  const file = process.argv[2];
  if (!file) { console.error('Usage: node validate-hook.js <hook.json>'); process.exit(1); }
  if (!fs.existsSync(file)) { console.error('Not found: ' + file); process.exit(1); }
  const raw = fs.readFileSync(file, 'utf8');
  let cfg;
  try { cfg = JSON.parse(raw); } catch (e) { console.error('JSON parse error: ' + e.message); process.exit(1); }

  const errs = [];
  const warns = [];

  if (!cfg.name || typeof cfg.name !== 'string') errs.push('missing "name"');
  if (!cfg.trigger) errs.push('missing "trigger"');
  else if (!KNOWN.includes(cfg.trigger)) warns.push('unknown trigger "' + cfg.trigger + '"');
  if (!cfg.command || typeof cfg.command !== 'string' || !cfg.command.trim()) errs.push('missing "command"');

  for (const p of SECRETS) {
    if (p.test(raw)) { errs.push('potential secret detected'); p.lastIndex = 0; }
  }
  for (const p of UNSAFE) {
    if (cfg.command && p.test(cfg.command)) errs.push('unsafe shell pattern in command');
  }

  if (warns.length) { console.warn('Warnings:'); warns.forEach(w => console.warn('  - ' + w)); }
  if (errs.length)  { console.error('Errors:');   errs.forEach(e => console.error('  - ' + e));  process.exit(1); }
  console.log('OK Hook is valid.');
}

main();