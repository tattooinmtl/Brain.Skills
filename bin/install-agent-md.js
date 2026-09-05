#!/usr/bin/env node
/**
 * install-agent-md.js — copy the canonical agent template into a project,
 * under ANY of the supported names. Behavior is identical regardless of
 * whether the file ends up as AGENTS.md, CLAUDE.md, GEMINI.md, KIMI.md,
 * PI.md, or OMNI.md.
 *
 * Two modes:
 *
 *   1) Per-agent home dir (system-wide):
 *      node install-agent-md.js --target home --harness <name|all>
 *      Drops the template into the agent's home config dir
 *      (e.g. ~/.claude/CLAUDE.md).
 *
 *   2) Project folder (repo-local):
 *      node install-agent-md.js --target project --project <dir> --name <NAME.md>
 *      Drops <NAME>.md into <dir>/. Copies the canonical template.
 *
 * Run with --dry-run to preview.
 */

const fs = require('fs');
const path = require('path');
const { findSkillsRoot } = require('./path-resolver.js');

// The canonical agent template lives at agents/AGENTS.md — one source of
// truth, real markdown on disk. Previous versions embedded an inline JS
// string here that started with <!DOCTYPE html> (invalid md) and drifted
// from the aliases. Read from disk instead.
function loadCanonical() {
  const skillsRoot = findSkillsRoot(__dirname);
  const tpl = path.join(skillsRoot, 'agents', 'AGENTS.md');
  if (!fs.existsSync(tpl)) {
    throw new Error('Canonical template not found at ' + tpl);
  }
  return fs.readFileSync(tpl, 'utf8');
}

const HOME_TARGETS = {
  omni:     { dir: '~/.omni/AGENTS.md'        },
  claude:   { dir: '~/.claude/CLAUDE.md'      },
  gemini:   { dir: '~/.gemini/GEMINI.md'      },
  kimi:     { dir: '~/.kimi-code/KIMI.md'     },
  pi:       { dir: '~/.pi/PI.md'              },
  // Codex has no home-config md convention — skip.
};

const PROJECT_NAMES = ['AGENTS.md','CLAUDE.md','GEMINI.md','KIMI.md','PI.md','OMNI.md'];

function expandHome(p) {
  const home = process.platform === 'win32'
    ? (process.env.USERPROFILE || process.env.HOME)
    : (process.env.HOME || process.env.USERPROFILE);
  return p.replace(/^~/, home);
}

function parseArgs() {
  const args = process.argv.slice(2);
  const out = { target: null, harness: null, project: null, name: null, dryRun: false };
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--target' && args[i + 1]) out.target = args[++i];
    else if (args[i] === '--harness' && args[i + 1]) out.harness = args[++i];
    else if (args[i] === '--project' && args[i + 1]) out.project = args[++i];
    else if (args[i] === '--name' && args[i + 1]) out.name = args[++i];
    else if (args[i] === '--dry-run') out.dryRun = true;
  }
  return out;
}

function install(dest) {
  const body = loadCanonical();
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, body, 'utf8');
  console.log('OK ' + dest);
}

function main() {
  const args = parseArgs();
  if (!args.target) {
    console.error('Usage:');
    console.error('  node install-agent-md.js --target home    --harness <name|all> [--dry-run]');
    console.error('  node install-agent-md.js --target project --project <dir> --name <NAME.md> [--dry-run]');
    process.exit(1);
  }

  if (args.target === 'home') {
    const harness = args.harness || 'all';
    const list = harness === 'all' ? Object.keys(HOME_TARGETS) : [harness];
    for (const h of list) {
      if (!HOME_TARGETS[h]) { console.error('Unknown harness: ' + h); continue; }
      const dest = expandHome(HOME_TARGETS[h].dir);
      if (args.dryRun) { console.log('[dry-run] would write ' + dest); continue; }
      install(dest);
    }
    return;
  }

  if (args.target === 'project') {
    if (!args.project) { console.error('--project required'); process.exit(1); }
    const projectDir = path.resolve(args.project);
    const name = args.name || 'AGENTS.md';
    if (!PROJECT_NAMES.includes(name)) {
      console.warn('NOTE: ' + name + ' is not in the standard set; behavior still works.');
    }
    const dest = path.join(projectDir, name);
    if (args.dryRun) { console.log('[dry-run] would write ' + dest); return; }
    install(dest);
    return;
  }

  console.error('Unknown --target: ' + args.target);
  process.exit(1);
}

main();