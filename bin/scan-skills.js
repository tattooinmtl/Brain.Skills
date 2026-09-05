#!/usr/bin/env node
/**
 * scan-skills.js — auto-discover skills in <skillsRoot>/skills/.
 *
 * What it does, every time it runs:
 *   1. Walks <skillsRoot>/skills/<folder>/SKILL.md.
 *   2. Parses YAML frontmatter (name, description, metadata).
 *   3. Writes <skillsRoot>/skills/index-cache.json with the full index.
 *   4. Auto-adds any new skill folder to <skillsRoot>/skills.json (the
 *      discoverability whitelist) so dropping a folder in makes it
 *      visible everywhere — no manual registration required.
 *
 * Usage:
 *   node scan-skills.js
 *   node scan-skills.js --dry-run        # report only, no writes
 *   node scan-skills.js --skills-root <dir>
 *
 * Used as a library:
 *   const { scan, loadIndex, suggestSkills } = require('./scan-skills.js');
 *   const { added, skills } = scan({ skillsRoot, dryRun: false });
 */

const fs = require('fs');
const path = require('path');
const { findSkillsRoot } = require('./path-resolver.js');

// ---- frontmatter parsing -----------------------------------------------------

/**
 * Parse a YAML-ish frontmatter block. We don't need full YAML — only the
 * top-level scalars we use (name, description, metadata). This keeps the
 * scanner dependency-free and robust against multi-line description fields.
 */
function parseFrontmatter(md) {
  if (!md.startsWith('---')) return {};
  const end = md.indexOf('\n---', 3);
  if (end === -1) return {};
  const block = md.slice(3, end).replace(/^\n/, '');
  const out = {};
  const lines = block.split(/\r?\n/);
  let currentKey = null;
  let buffer = [];
  // How to join buffer when this key is finalized.
  //   'inline'  — single-line scalar; buffer has at most 1 entry
  //   'folded'  — YAML `>` folded scalar; join lines with a space
  //   'literal' — YAML `|` literal scalar; join lines with a newline
  //   'nested'  — key had no scalar value on its line; keep newlines
  let joinMode = 'inline';

  function flush() {
    if (!currentKey) return;
    const joiner =
      joinMode === 'folded' ? ' ' :
      joinMode === 'literal' ? '\n' :
      joinMode === 'nested' ? '\n' :
      /* inline */ '';
    out[currentKey] = buffer.join(joiner).trim();
  }

  for (const line of lines) {
    // Top-level key: "name:" (no leading whitespace).
    const m = line.match(/^([A-Za-z_][A-Za-z0-9_-]*)\s*:\s*(.*)$/);
    if (m && !line.startsWith(' ') && !line.startsWith('\t')) {
      flush();
      currentKey = m[1];
      const rest = m[2];
      if (rest === '>' || rest === '>-') {
        joinMode = 'folded';
        buffer = [];
      } else if (rest === '|' || rest === '|-') {
        joinMode = 'literal';
        buffer = [];
      } else if (rest === '' || rest === undefined) {
        joinMode = 'nested';
        buffer = [];
      } else {
        joinMode = 'inline';
        buffer = [rest.replace(/^["']|["']$/g, '')];
      }
      continue;
    }
    // Continuation line (indented or blank inside a block scalar).
    if (currentKey && (line.startsWith('  ') || line.startsWith('\t') || line === '')) {
      buffer.push(line.trim());
    }
  }
  flush();
  return out;
}

/**
 * Pull simple tags out of the metadata.tags list if present. We accept:
 *   metadata:
 *     tags: [cpp, lua, sol2]
 * or comma-separated:
 *   tags: cpp, lua, sol2
 */
function extractTags(fm) {
  if (!fm.metadata) return [];
  // metadata may have come in as a string (unparsed nested block); try to find tags
  const metaRaw = typeof fm.metadata === 'string' ? fm.metadata : '';
  const m = metaRaw.match(/tags\s*:\s*(.*)/i);
  if (!m) return [];
  const tail = m[1].trim();
  if (tail.startsWith('[') && tail.endsWith(']')) {
    return tail.slice(1, -1).split(',').map(s => s.trim().replace(/^["']|["']$/g, '')).filter(Boolean);
  }
  return tail.split(',').map(s => s.trim()).filter(Boolean);
}

function tokenize(text) {
  return (text || '')
    .toLowerCase()
    .replace(/[^a-z0-9+\-#./\s]/g, ' ')
    .split(/\s+/)
    .filter(Boolean);
}

// ---- core scan ---------------------------------------------------------------

// Folders we never descend into. node_modules etc. are common accidents when
// user points --extra-root at a repo checkout.
const SKIP_DIR_NAMES = new Set([
  'node_modules', '.git', '.hg', '.svn', 'dist', 'build', 'out',
  'target', '.next', '.turbo', '.cache', '__pycache__', 'index-cache',
  '$RECYCLE.BIN', 'System Volume Information',
]);

/**
 * Recursively walk `root` looking for folders that contain a SKILL.md.
 * Depth-capped so pathological trees don't hang the scanner.
 * Yields { skillMd, folderPath, relPath, depth }.
 */
function walkSkills(root, maxDepth) {
  const out = [];
  const cap = typeof maxDepth === 'number' ? maxDepth : 6;
  const rootReal = fs.realpathSync.native ? fs.realpathSync.native(root) : root;
  const seen = new Set([rootReal]);

  // depth semantics: 0 = folder directly under `root` (top-level skill),
  // 1 = one level nested (e.g. superpowers/writing-plans), etc.
  // The scan root itself is not counted as a skill.
  function walk(dir, relParts) {
    const depth = relParts.length - 1;
    if (depth > cap) return;
    let entries;
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch (_) { return; }

    if (relParts.length > 0 && entries.some(e => e.isFile() && e.name === 'SKILL.md')) {
      out.push({
        skillMd: path.join(dir, 'SKILL.md'),
        folderPath: dir,
        relPath: relParts.join('/'),
        depth,
      });
    }

    for (const e of entries) {
      if (!e.isDirectory()) continue;
      if (e.name.startsWith('.')) continue;
      if (SKIP_DIR_NAMES.has(e.name)) continue;
      const child = path.join(dir, e.name);
      try {
        const real = fs.realpathSync.native ? fs.realpathSync.native(child) : fs.realpathSync(child);
        if (seen.has(real)) continue;
        seen.add(real);
      } catch (_) { continue; }
      walk(child, relParts.concat(e.name));
    }
  }

  walk(root, []);
  return out;
}

/**
 * Discover every SKILL.md under a root and build an index.
 *
 * Returns { skills, added, indexPath, whitelistPath } where:
 *   skills = [{ id, name, folder, path, relPath, depth, parentPack,
 *               description, tags, tokens }]
 *   added  = string[] of top-level folder names auto-added to skills.json
 *
 * `id` is unique per SKILL.md (uses relPath). `name` may collide across
 * duplicates — use `id` when uniqueness matters.
 *
 * By default scans <skillsRoot>/skills. Pass opts.root to scan a chosen
 * folder instead (used by scan-now.js for on-demand discovery).
 */
function scan(opts) {
  opts = opts || {};
  const skillsRoot = opts.skillsRoot || findSkillsRoot(__dirname);
  const defaultSkillsDir = path.join(skillsRoot, 'skills');
  const scanRoot = opts.root ? path.resolve(opts.root) : defaultSkillsDir;
  const indexPath = path.join(defaultSkillsDir, 'index-cache.json');
  const whitelistPath = path.join(skillsRoot, 'skills.json');
  const dryRun = !!opts.dryRun;
  // Only mutate skills.json / index-cache.json when scanning the canonical
  // root. Ad-hoc --root scans return results without touching global state.
  const isCanonicalScan = path.resolve(scanRoot) === path.resolve(defaultSkillsDir);

  if (!fs.existsSync(scanRoot)) {
    return { skills: [], added: [], error: 'no scan root: ' + scanRoot };
  }

  const hits = walkSkills(scanRoot, opts.maxDepth);
  const skills = [];
  for (const hit of hits) {
    const raw = fs.readFileSync(hit.skillMd, 'utf8');
    const fm = parseFrontmatter(raw);
    const folder = path.basename(hit.folderPath);
    const name = (fm.name || folder).toString().trim();
    const description = (fm.description || '').toString().trim();
    const tags = extractTags(fm);
    const corpus = (name + ' ' + tags.join(' ') + ' ' + description).slice(0, 4000);
    const tokens = Array.from(new Set(tokenize(corpus)));
    // parentPack = the containing pack folder name when nested one or more
    // levels below the scan root (e.g. superpowers/writing-plans -> "superpowers").
    const relParts = hit.relPath.split('/');
    const parentPack = relParts.length > 1 ? relParts[0] : null;
    skills.push({
      id: hit.relPath,
      name,
      folder,
      path: hit.folderPath,
      relPath: hit.relPath,
      depth: hit.depth,
      parentPack,
      description,
      tags,
      tokens,
    });
  }

  // Stable order: by relPath (groups packs with their children).
  skills.sort((a, b) => a.relPath.localeCompare(b.relPath));

  if (!dryRun && isCanonicalScan) {
    fs.writeFileSync(indexPath, JSON.stringify({
      generatedAt: new Date().toISOString(),
      skillsRoot,
      scanRoot,
      count: skills.length,
      skills,
    }, null, 2) + '\n', 'utf8');
  }

  // Only top-level (depth === 0) folders are junction-eligible. Nested
  // skills live inside packs and are surfaced via the index only —
  // junction folder names must be unique on disk.
  const topLevel = skills.filter(s => s.depth === 0);

  // Detect any top-level folders not yet present in skills.json. We report
  // the diff but do NOT write skills.json here — `sync-skills.js` is the
  // sole writer (single source of truth, one shape). Callers who want the
  // catalog updated should invoke sync-skills.js.
  const added = [];
  if (isCanonicalScan && fs.existsSync(whitelistPath)) {
    try {
      const whitelist = JSON.parse(fs.readFileSync(whitelistPath, 'utf8'));
      const entries = Array.isArray(whitelist.entries) ? whitelist.entries : [];
      const knownFolders = new Set(entries.map(e => (e && e.name) || path.basename((e.path || '').replace(/[\\/]+/g, path.sep))));
      for (const s of topLevel) {
        if (!knownFolders.has(s.folder)) added.push(s.folder);
      }
    } catch (_) { /* ignore malformed skills.json — sync-skills will fix it */ }
  }

  return { skills, added, indexPath, whitelistPath, scanRoot, topLevelCount: topLevel.length };
}

function loadIndex(skillsRoot) {
  skillsRoot = skillsRoot || findSkillsRoot(__dirname);
  const indexPath = path.join(skillsRoot, 'skills', 'index-cache.json');
  if (!fs.existsSync(indexPath)) {
    // Lazy scan if the cache doesn't exist yet
    const r = scan({ skillsRoot });
    return r.skills;
  }
  try {
    const j = JSON.parse(fs.readFileSync(indexPath, 'utf8'));
    return Array.isArray(j.skills) ? j.skills : [];
  } catch (_) {
    return [];
  }
}

/**
 * Given a free-text prompt and the skill index, return skill names whose
 * token overlap with the prompt is strong enough to suggest them.
 * Used by plan-from-prompt.js to auto-include matching skills.
 */
function suggestSkills(prompt, skills, opts) {
  opts = opts || {};
  const minOverlap = opts.minOverlap || 1;
  const maxResults = opts.maxResults || 6;
  const promptTokens = new Set(tokenize(prompt));
  if (!promptTokens.size || !skills || !skills.length) return [];
  const scored = [];
  for (const s of skills) {
    let overlap = 0;
    const hits = [];
    for (const t of s.tokens) {
      if (promptTokens.has(t)) {
        overlap++;
        hits.push(t);
      }
    }
    if (overlap >= minOverlap) scored.push({ name: s.name, folder: s.folder, overlap, hits });
  }
  scored.sort((a, b) => b.overlap - a.overlap || a.folder.localeCompare(b.folder));
  return scored.slice(0, maxResults);
}

// ---- CLI ---------------------------------------------------------------------

function parseArgs() {
  const args = process.argv.slice(2);
  const out = { dryRun: false, skillsRoot: null, json: false, root: null };
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--dry-run') out.dryRun = true;
    else if (args[i] === '--json') out.json = true;
    else if (args[i] === '--skills-root' && args[i + 1]) out.skillsRoot = args[++i];
    else if (args[i] === '--root' && args[i + 1]) out.root = args[++i];
  }
  return out;
}

if (require.main === module) {
  const args = parseArgs();
  const result = scan({ skillsRoot: args.skillsRoot, dryRun: args.dryRun, root: args.root });
  const nested = result.skills.filter(s => s.depth > 0).length;
  if (args.json) {
    console.log(JSON.stringify({
      scanRoot: result.scanRoot,
      count: result.skills.length,
      topLevel: result.topLevelCount,
      nested,
      added: result.added,
      skills: result.skills.map(s => ({ id: s.id, folder: s.folder, name: s.name, depth: s.depth, parentPack: s.parentPack, tags: s.tags })),
    }, null, 2));
  } else {
    console.log('Scanned ' + result.skills.length + ' skills (' + result.topLevelCount + ' top-level, ' + nested + ' nested) under ' + result.scanRoot);
    if (result.added.length) {
      console.log('Auto-added to skills.json: ' + result.added.join(', '));
    } else {
      console.log('No new top-level skills to register.');
    }
    console.log('');
    console.log('Skills:');
    for (const s of result.skills) {
      const tag = s.tags.length ? '  [' + s.tags.join(',') + ']' : '';
      const marker = s.depth === 0 ? ' ' : '·';
      console.log('  ' + marker + ' ' + s.id.padEnd(48) + ' ' + s.name + tag);
    }
    if (!args.dryRun) {
      console.log('');
      console.log('Wrote: ' + result.indexPath);
      if (result.added.length) console.log('Updated: ' + result.whitelistPath);
    }
  }
}

module.exports = { scan, walkSkills, loadIndex, suggestSkills, parseFrontmatter, tokenize };
