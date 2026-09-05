#!/usr/bin/env node
/**
 * rotate-phase.js — phase rotator.
 *
 * Reads .skills-meta.json, finds the requested phase, and:
 *   1. Refreshes the skill-loading order block at the top of phase-NN.md.
 *   2. Updates .skills-meta.json with active-phase + active-skills
 *      (and tracks what was unloaded = previous active skills).
 *   3. Logs to <skillsRoot>/logs/rotations.log.
 *   4. Prints the START A NEW CONVERSATION message.
 *
 * Usage:
 *   node rotate-phase.js --plan-dir <dir> --phase <N> [--dry-run]
 */

const fs = require('fs');
const path = require('path');
const { resolve, findSkillsRoot } = require('./path-resolver.js');
const { scan: scanSkills } = require('./scan-skills.js');
const syncSkills = require('./sync-skills.js');

function parseArgs() {
  const args = process.argv.slice(2);
  const out = { planDir: null, phase: null, dryRun: false };
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--plan-dir' && args[i + 1]) out.planDir = args[++i];
    else if (args[i] === '--phase' && args[i + 1]) out.phase = parseInt(args[++i], 10);
    else if (args[i] === '--dry-run') out.dryRun = true;
  }
  return out;
}

function pad(n) { return String(n).padStart(2, '0'); }

function refreshPhaseFile(phaseFile, phase, skills, ts) {
  let body = '';
  if (fs.existsSync(phaseFile)) body = fs.readFileSync(phaseFile, 'utf8');
  body = body.replace(/^(?:<!--[^]*?-->\s*)+/, '');
  const head =
    '<!-- skills: ' + skills.join(', ') + ' -->\n' +
    '<!-- phase: ' + phase.n + ' (' + phase.label + ') -->\n' +
    '<!-- rotated: ' + ts + ' -->\n\n';
  fs.writeFileSync(phaseFile, head + body, 'utf8');
}

function main() {
  const args = parseArgs();
  if (args.planDir) process.env.PLAN_DIR = args.planDir;
  const ctx = resolve({});

  // Refresh cache + catalog before rotation. scan-skills writes only
  // index-cache.json now; sync-skills is the sole skills.json writer.
  scanSkills({ skillsRoot: ctx.skillsRoot });
  syncSkills.main(['--no-install']);

  const planDir = args.planDir || ctx.planDir;
  const metaPath = path.join(planDir, '.skills-meta.json');
  if (!fs.existsSync(metaPath)) {
    console.error('No .skills-meta.json in ' + planDir + '. Run plan-from-prompt.js first.');
    process.exit(1);
  }
  if (!args.phase || !Number.isFinite(args.phase)) {
    console.error('Usage: node rotate-phase.js --plan-dir <dir> --phase <N> [--dry-run]');
    process.exit(1);
  }
  const meta = JSON.parse(fs.readFileSync(metaPath, 'utf8'));
  const phase = meta.phases.find(p => p.n === args.phase);
  if (!phase) {
    console.error('Phase ' + args.phase + ' not found. Available: ' + meta.phases.map(p => p.n).join(', '));
    process.exit(1);
  }

  const phaseFile = path.join(planDir, 'phase-' + pad(args.phase) + '.md');
  const ts = new Date().toISOString();
  const skillsRoot = findSkillsRoot(__dirname);
  const logPath = path.join(skillsRoot, 'logs', 'rotations.log');
  const previousSkills = meta.active && Array.isArray(meta.active.skills) ? meta.active.skills.slice() : [];

  if (args.dryRun) {
    console.log('[dry-run] would refresh ' + phaseFile);
    console.log('[dry-run] would update ' + metaPath);
    console.log('[dry-run] would append to ' + logPath);
    return;
  }

  refreshPhaseFile(phaseFile, phase, phase.skills, ts);

  meta.active = { n: phase.n, label: phase.label, skills: phase.skills, since: ts };
  meta.unloaded = previousSkills;
  fs.writeFileSync(metaPath, JSON.stringify(meta, null, 2) + '\n', 'utf8');

  fs.mkdirSync(path.dirname(logPath), { recursive: true });
  const logLine = ts + '  planDir=' + planDir + '  phase=' + phase.n + '  label="' + phase.label + '"  skills=[' + phase.skills.join(', ') + ']\n';
  fs.appendFileSync(logPath, logLine, 'utf8');

  console.log('Phase ' + phase.n + ': ' + phase.label);
  console.log('Unloaded: ' + (previousSkills.length ? previousSkills.join(', ') : '(none — first phase)'));
  console.log('Loaded:   ' + phase.skills.join(', '));
  console.log('');
  console.log('  ' + phaseFile);
  console.log('');
  console.log('========================================================');
  console.log('  >>> START A NEW CONVERSATION <<<');
  console.log('  At conversation start, type:');
  console.log('');
  console.log('    load skills: ' + phase.skills.join(', '));
  console.log('    then read:   ' + phaseFile);
  console.log('    and follow its steps.');
  console.log('========================================================');
}

main();