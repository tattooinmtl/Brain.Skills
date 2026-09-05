#!/usr/bin/env node
/**
 * path-resolver.js — shared path resolution for the .skills kit.
 *
 * Resolution order for the SKILLS ROOT (where skills/ and bin/ live):
 *   1. process.env.SKILLS_ROOT (explicit override)
 *   2. Walk up from this file to find a folder containing skills/ AND bin/
 *   3. C:/.skills  (Windows convention)
 *   4. ~/.skills   (Unix convention)
 *
 * The PLAN folder (project-local) is resolved separately:
 *   1. process.env.PLAN_DIR (explicit override)
 *   2. The directory containing the invoking agent file (AGENTS.md etc.)
 *
 * The SKILLS FOLDER for a given skill name is:
 *   <skillsRoot>/skills/<skill-name>/
 *
 * Usage:
 *   const { skillsRoot, planDir, skillsDir, agentFile } =
 *     require('./path-resolver.js')({ agentFile: __filename });
 */

const fs = require('fs');
const path = require('path');

function expandWin(p) { return p.replace(/[\\/]+/g, path.sep); }
function expandHome(p) {
  const home = process.platform === 'win32'
    ? (process.env.USERPROFILE || process.env.HOME)
    : (process.env.HOME || process.env.USERPROFILE);
  return p.replace(/^~/, home);
}

function findSkillsRoot(startDir) {
  if (process.env.SKILLS_ROOT) {
    const r = expandHome(expandWin(process.env.SKILLS_ROOT));
    if (fs.existsSync(path.join(r, 'bin'))) return r;
  }
  let dir = startDir;
  for (let i = 0; i < 10; i++) {
    const candidate = path.join(dir, 'bin');
    if (fs.existsSync(candidate) && fs.existsSync(path.join(dir, 'skills'))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  // Fallbacks
  const fallbacks = process.platform === 'win32'
    ? ['C:/.skills']
    : ['~/.skills'];
  for (const f of fallbacks) {
    const r = expandHome(expandWin(f));
    if (fs.existsSync(path.join(r, 'bin'))) return r;
  }
  throw new Error('Cannot locate skills root. Set SKILLS_ROOT env var.');
}

function findPlanDir(opts) {
  if (process.env.PLAN_DIR) return expandHome(expandWin(process.env.PLAN_DIR));
  if (opts && opts.agentFile) {
    return path.dirname(path.resolve(opts.agentFile));
  }
  if (opts && opts.cwd) return opts.cwd;
  return process.cwd();
}

function detectAgentName(agentFile) {
  if (!agentFile) return 'agent';
  const base = path.basename(agentFile).toLowerCase();
  // Strip .md, normalize to lowercase
  const stem = base.replace(/\.md$/i, '');
  if (stem === 'agents') return 'agents';
  return stem;
}

function resolve(opts) {
  const startDir = opts && opts.startDir
    ? opts.startDir
    : path.dirname(__filename);
  const skillsRoot = findSkillsRoot(startDir);
  const planDir = findPlanDir(opts);
  const agentName = detectAgentName(opts && opts.agentFile);
  return {
    skillsRoot,
    planDir,
    agentName,
    skillsDir: path.join(skillsRoot, 'skills'),
    binDir: path.join(skillsRoot, 'bin'),
    agentFile: opts && opts.agentFile ? path.resolve(opts.agentFile) : null,
  };
}

module.exports = { resolve, findSkillsRoot, findPlanDir, detectAgentName };