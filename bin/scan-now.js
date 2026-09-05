#!/usr/bin/env node
'use strict';

/**
 * scan-now.js — on-demand skill discovery.
 *
 * Default:
 *   Scans C:/.skills/skills (the canonical skill folder). This is what runs
 *   on brain-system / session startup.
 *
 *   node scan-now.js
 *
 * Discovery mode:
 *   Point the scanner at a chosen folder (opt-in — no full-PC walk).
 *   Discovered top-level SKILL.md folders are reported. Add --import to
 *   junction them into C:/.skills/skills so they become first-class.
 *
 *   node scan-now.js --root D:/projects/my-skill-pack
 *   node scan-now.js --root D:/projects/my-skill-pack --import
 *
 * Flags:
 *   --root <dir>       Scan this folder instead of C:/.skills/skills.
 *   --import           Copy discovered top-level skills into C:/.skills/skills
 *                      by creating a junction (or dir on non-Windows). Only
 *                      valid with --root. Refuses to overwrite existing names.
 *   --dry-run          Report only; no writes.
 *   --json             Machine-readable output.
 *   --max-depth <n>    Walk depth cap (default 6).
 *
 * Safety:
 *   - Refuses to scan roots inside node_modules, .git, %TEMP%, %APPDATA%, or
 *     the recycle bin.
 *   - Never walks into node_modules/.git/dist/build/etc. inside the root.
 *   - Only the canonical scan updates skills.json / index-cache.json; a
 *     custom --root scan is read-only unless --import is passed.
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const { scan, walkSkills } = require('./scan-skills.js');
const { findSkillsRoot } = require('./path-resolver.js');

const REFUSED_ROOT_PATTERNS = [
  /[\\/]node_modules([\\/]|$)/i,
  /[\\/]\.git([\\/]|$)/i,
  /[\\/]\$RECYCLE\.BIN([\\/]|$)/i,
  /[\\/]System Volume Information([\\/]|$)/i,
];

function envDir(name) {
  const v = process.env[name];
  return v ? path.resolve(v) : null;
}

function isRefusedRoot(root) {
  const resolved = path.resolve(root);
  for (const pat of REFUSED_ROOT_PATTERNS) {
    if (pat.test(resolved)) return 'matches disallowed pattern';
  }
  const forbidden = [
    envDir('TEMP'), envDir('TMP'), envDir('APPDATA'),
    envDir('LOCALAPPDATA'),
  ].filter(Boolean);
  for (const f of forbidden) {
    if (resolved === f || resolved.toLowerCase().startsWith(f.toLowerCase() + path.sep)) {
      return 'inside ' + f;
    }
  }
  // Refuse drive roots — too broad.
  if (/^[A-Za-z]:[\\/]?$/.test(resolved) || resolved === '/') {
    return 'drive root — pick a specific folder';
  }
  return null;
}

function parseArgs(argv) {
  const out = { root: null, dryRun: false, json: false, import: false, maxDepth: 6, help: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--root' && argv[i + 1]) out.root = argv[++i];
    else if (a === '--import') out.import = true;
    else if (a === '--dry-run') out.dryRun = true;
    else if (a === '--json') out.json = true;
    else if (a === '--max-depth' && argv[i + 1]) out.maxDepth = parseInt(argv[++i], 10);
    else if (a === '--help' || a === '-h') out.help = true;
  }
  return out;
}

function importDiscoveredSkills(hits, skillsDir, dryRun) {
  const linkType = process.platform === 'win32' ? 'junction' : 'dir';
  const imported = [];
  const skipped = [];
  const errors = [];
  fs.mkdirSync(skillsDir, { recursive: true });
  for (const h of hits) {
    if (h.depth !== 0) continue; // only top-level of the imported root
    const destName = path.basename(h.folderPath);
    const dest = path.join(skillsDir, destName);
    if (fs.existsSync(dest)) { skipped.push({ name: destName, reason: 'name exists' }); continue; }
    if (dryRun) { imported.push(destName); continue; }
    try {
      fs.symlinkSync(h.folderPath, dest, linkType);
      imported.push(destName);
    } catch (err) {
      errors.push({ name: destName, error: err.message });
    }
  }
  return { imported, skipped, errors };
}

function main(argv) {
  const args = parseArgs(argv || process.argv.slice(2));
  if (args.help) {
    process.stdout.write(
      'Usage: node scan-now.js [--root DIR] [--import] [--dry-run] [--json] [--max-depth N]\n' +
      '  Default scans C:/.skills/skills. --root scans a chosen folder.\n' +
      '  --import (with --root) junctions discovered top-level skills into the canonical folder.\n'
    );
    return 0;
  }

  const skillsRoot = findSkillsRoot(__dirname);
  const canonicalSkillsDir = path.join(skillsRoot, 'skills');

  if (args.root) {
    const reason = isRefusedRoot(args.root);
    if (reason) {
      process.stderr.write('scan-now: refusing to scan ' + args.root + ' (' + reason + ')\n');
      return 2;
    }
    if (!fs.existsSync(args.root)) {
      process.stderr.write('scan-now: root does not exist: ' + args.root + '\n');
      return 2;
    }
  }

  const scanRoot = args.root ? path.resolve(args.root) : canonicalSkillsDir;
  const result = scan({ root: scanRoot, dryRun: args.dryRun, maxDepth: args.maxDepth });
  const nested = result.skills.filter(s => s.depth > 0).length;

  let importResult = null;
  if (args.root && args.import) {
    const hits = walkSkills(scanRoot, args.maxDepth);
    importResult = importDiscoveredSkills(hits, canonicalSkillsDir, args.dryRun);
    // Re-scan the canonical folder so skills.json / index-cache.json pick up the imports.
    if (!args.dryRun) scan({ dryRun: false });
  }

  if (args.json) {
    process.stdout.write(JSON.stringify({
      scanRoot: result.scanRoot,
      count: result.skills.length,
      topLevel: result.topLevelCount,
      nested,
      added: result.added,
      import: importResult,
      skills: result.skills.map(s => ({ id: s.id, folder: s.folder, name: s.name, depth: s.depth, parentPack: s.parentPack })),
    }, null, 2) + '\n');
  } else {
    const prefix = args.dryRun ? 'scan-now (dry-run): ' : 'scan-now: ';
    process.stdout.write(prefix + result.skills.length + ' skills (' + result.topLevelCount + ' top-level, ' + nested + ' nested) under ' + result.scanRoot + '\n');
    if (result.added.length) process.stdout.write('  registered ' + result.added.length + ' new top-level skill(s): ' + result.added.join(', ') + '\n');
    if (importResult) {
      process.stdout.write('  import: ' + importResult.imported.length + ' junctioned, ' + importResult.skipped.length + ' skipped, ' + importResult.errors.length + ' errors\n');
      for (const s of importResult.skipped) process.stdout.write('    - skipped ' + s.name + ' (' + s.reason + ')\n');
      for (const e of importResult.errors) process.stdout.write('    ! ' + e.name + ': ' + e.error + '\n');
    }
  }
  return 0;
}

module.exports = { main, parseArgs, isRefusedRoot };

if (require.main === module) {
  process.exit(main());
}
