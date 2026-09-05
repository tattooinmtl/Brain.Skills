#!/usr/bin/env node
/**
 * plan-from-prompt.js — turn a prompt into a project-local plan.
 *
 * Output layout (in the same folder as the invoking agent file):
 *
 *   <planDir>/plan.md            master plan
 *   <planDir>/phase-01.md        per-phase files (one per phase)
 *   <planDir>/phase-02.md
 *   <planDir>/phase-NN.md
 *   <planDir>/.skills-meta.json  rotation log + active skill record
 *
 * Each phase-NN.md begins with the skill-loading order:
 *
 *   <!-- skills: brainstorming, prompt-me, before-building -->
 *   <!-- phase: 1 (Understand) -->
 *
 * so the agent loads exactly those skills at conversation start.
 *
 * Usage:
 *   node plan-from-prompt.js --prompt "<text>"
 *                          [--agent <AGENTS.md path>]
 *                          [--plan-dir <dir>]
 *                          [--skill-root <dir>]
 */

const fs = require('fs');
const path = require('path');
const { resolve } = require('./path-resolver.js');
const { scan: scanSkills, loadIndex: loadSkillsIndex, suggestSkills } = require('./scan-skills.js');
const syncSkills = require('./sync-skills.js');

// Every skill listed here must resolve — either a top-level folder OR a
// <pack>/<skill> path under C:/.skills/skills. Verified against skills.json.
// Nested paths are used where a suitable skill only lives inside a pack.
// Tool names that aren't skills (rag_search, find_symbol, dev_env_report,
// git-worktrees) were removed in favor of the actual skill equivalents.
const PHASE_KINDS = {
  understand: { label: 'Understand', skills: ['brainstorming','thinking-and-docs/prompt-me','thinking-and-docs/before-building'],
                description: 'Restate the goal; ask one focused question if needed.' },
  explore:    { label: 'Explore',    skills: ['skill-authoring/using-addon-skills','github/codebase-inspection'],
                description: 'Map the stack; gather context before any change.' },
  plan:       { label: 'Plan',       skills: ['writing-plans','software-development/plan','thinking-and-docs/next-decision'],
                description: 'Decide what "done" means for each phase.' },
  implement:  { label: 'Implement',  skills: ['coder-ai-senior-developer','dispatching-parallel-agents','using-git-worktrees'],
                description: 'Apply changes in small increments, one file at a time.' },
  verify:     { label: 'Verify',     skills: ['test-driven-development','systematic-debugging','verification-before-completion','vibe-code-auditor'],
                description: 'Run tests/build/lint; paste command + output.' },
  report:     { label: 'Report',     skills: ['requesting-code-review','agent-orchestration/handoff','finishing-a-development-branch'],
                description: 'Summarise: what changed, how verified, what remains.' },
};

// FIX: `explor[a-z]*` so it matches explore/exploration/exploring/etc.
const KEYWORD_HINTS = [
  { pattern: /\b(audit[a-z]*|review[a-z]*|vulnerab[a-z]*|security|secrets?)\b/i,           kind: 'understand', weight: 1 },
  { pattern: /\b(explor[a-z]*|investigate[a-z]*|inspect[a-z]*|find[a-z]*|where[a-z]*|map[a-z]*)\b/i, kind: 'explore', weight: 1 },
  { pattern: /\b(plan[a-z]*|design[a-z]*|architect[a-z]*|break[a-z]*\s*down|phases?)\b/i,         kind: 'plan',       weight: 1 },
  { pattern: /\b(build[a-z]*|implement[a-z]*|create[a-z]*|write[a-z]*|code[a-z]*|scaffold[a-z]*|add[a-z]*)\b/i, kind: 'implement', weight: 1 },
  { pattern: /\b(test[a-z]*|verif[a-z]*|debug[a-z]*|fix[a-z]*|break[a-z]*)\b/i,                  kind: 'verify',     weight: 1 },
  { pattern: /\b(report[a-z]*|summar[a-z]*|document[a-z]*)\b/i,                                  kind: 'report',     weight: 1 },
];

function detectPhases(prompt) {
  const counts = {};
  for (const k of Object.keys(PHASE_KINDS)) counts[k] = 0;
  for (const h of KEYWORD_HINTS) {
    const m = prompt.match(h.pattern);
    if (m) counts[h.kind] += m.length * h.weight;
  }
  const order = ['understand'];
  if (counts.explore > 0 || /investigate|inspect|existing|codebase|current|explore/i.test(prompt)) order.push('explore');
  order.push('plan');
  if (counts.implement > 0 || /\b(build|create|add|implement|write)\b/i.test(prompt)) order.push('implement');
  order.push('verify');
  order.push('report');
  return [...new Set(order)];
}

function pad(n) { return String(n).padStart(2, '0'); }

function renderPlanMd(prompt, phases, ctx) {
  const ts = new Date().toISOString();
  let md = '# plan.md\n\n';
  md += '> Generated: ' + ts + '\n';
  md += '> Agent: ' + ctx.agentName + ' (' + (ctx.agentFile || 'unknown') + ')\n';
  md += '> Skills root: ' + ctx.skillsRoot + '\n\n';
  md += '## Prompt\n\n> ' + prompt.replace(/\n/g, '\n> ') + '\n\n';
  md += '## Phases\n\n';
  phases.forEach((k, i) => {
    const p = PHASE_KINDS[k];
    md += '- **Phase ' + pad(i + 1) + '** — ' + p.label + ' (skills: ' + p.skills.join(', ') + ')\n';
  });
  md += '\n## Done log\n\n<!-- append one-line summary per phase as it completes -->\n\n';
  md += '## Notes\n\n';
  md += '- Per-phase files: `phase-01.md`, `phase-02.md`, ... in this same folder.\n';
  md += '- Rotate to next phase: `node <skillsRoot>/bin/rotate-phase.js --plan-dir "' + ctx.planDir + '" --phase <NN>`\n';
  md += '- Load ONLY the skills listed at the top of each `phase-NN.md`.\n';
  return md;
}

function renderPhaseMd(prompt, kind, n, ctx) {
  const p = PHASE_KINDS[kind];
  const ts = new Date().toISOString();
  let md = '<!-- skills: ' + p.skills.join(', ') + ' -->\n';
  md += '<!-- phase: ' + n + ' (' + p.label + ') -->\n';
  md += '<!-- generated: ' + ts + ' -->\n\n';
  md += '# phase-' + pad(n) + '.md — ' + p.label + '\n\n';
  md += '## Load these skills (in order)\n\n';
  p.skills.forEach((s, i) => { md += (i + 1) + '. `' + s + '`\n'; });
  md += '\nSource: `' + path.join(ctx.skillsRoot, 'skills', '<skill-name>') + '/SKILL.md`\n\n';
  md += '## Goal\n\n' + p.description + '\n\n';
  md += '## Steps\n\n- [ ] (auto — first run starts here)\n\n';
  md += '## Notes\n\n- Parent: `plan.md` in this folder.\n';
  md += '- After completing: tick boxes, append one-line summary to `plan.md` `## Done log`, run rotate-phase for the next phase.\n';
  return md;
}

function parseArgs() {
  const args = process.argv.slice(2);
  const out = { prompt: null, agent: null, planDir: null, skillRoot: null };
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--prompt' && args[i + 1]) out.prompt = args[++i];
    else if (args[i] === '--agent' && args[i + 1]) out.agent = args[++i];
    else if (args[i] === '--plan-dir' && args[i + 1]) out.planDir = args[++i];
    else if (args[i] === '--skill-root' && args[i + 1]) out.skillRoot = args[++i];
    else if (args[i] === '--stdin') out.prompt = fs.readFileSync(0, 'utf8').trim();
  }
  return out;
}

function main() {
  const args = parseArgs();
  if (!args.prompt) {
    console.error('Usage: node plan-from-prompt.js --prompt "<text>" [--agent <AGENTS.md>] [--plan-dir <dir>] [--skill-root <dir>]');
    process.exit(1);
  }
  if (args.skillRoot) process.env.SKILLS_ROOT = args.skillRoot;
  if (args.planDir)   process.env.PLAN_DIR    = args.planDir;

  const ctx = resolve({ agentFile: args.agent });
  // Refresh the on-disk index-cache (scan-skills) AND regenerate the
  // catalog (sync-skills). scan-skills is now cache-only; sync-skills is
  // the sole writer of skills.json + INDEX.md. --no-install skips the
  // ~/.agents/skills junction step which install.js already handles.
  scanSkills({ skillsRoot: ctx.skillsRoot });
  syncSkills.main(['--no-install']);
  const skillsIndex = loadSkillsIndex(ctx.skillsRoot);
  const phases = detectPhases(args.prompt);

  // If the prompt strongly matches a domain skill the user has dropped
  // into skills/ (e.g. "sol2 lua bindings" -> cpp-lua-engine), append it
  // to the implement phase so the rotator pulls it in automatically.
  // Threshold (overlap >= 3) is high enough that generic prompts don't
  // pull in noise.
  if (phases.indexOf('implement') !== -1) {
    const suggestions = suggestSkills(args.prompt, skillsIndex, { minOverlap: 3, maxResults: 3 });
    const existing = new Set(PHASE_KINDS.implement.skills);
    for (const s of suggestions) {
      if (!existing.has(s.name)) PHASE_KINDS.implement.skills.push(s.name);
    }
  }

  fs.mkdirSync(ctx.planDir, { recursive: true });

  const planPath = path.join(ctx.planDir, 'plan.md');
  fs.writeFileSync(planPath, renderPlanMd(args.prompt, phases, ctx), 'utf8');

  phases.forEach((kind, i) => {
    const n = i + 1;
    const phasePath = path.join(ctx.planDir, 'phase-' + pad(n) + '.md');
    fs.writeFileSync(phasePath, renderPhaseMd(args.prompt, kind, n, ctx), 'utf8');
  });

  const metaPath = path.join(ctx.planDir, '.skills-meta.json');
  const meta = {
    prompt: args.prompt,
    agent: ctx.agentName,
    agentFile: ctx.agentFile,
    skillsRoot: ctx.skillsRoot,
    planDir: ctx.planDir,
    phases: phases.map((k, i) => ({
      n: i + 1,
      kind: k,
      label: PHASE_KINDS[k].label,
      skills: PHASE_KINDS[k].skills,
      file: path.join(ctx.planDir, 'phase-' + pad(i + 1) + '.md'),
    })),
    createdAt: new Date().toISOString(),
  };
  fs.writeFileSync(metaPath, JSON.stringify(meta, null, 2) + '\n', 'utf8');

  console.log('Agent:    ' + ctx.agentName + (ctx.agentFile ? ' (' + ctx.agentFile + ')' : ''));
  console.log('Skills:   ' + ctx.skillsRoot);
  console.log('Plan dir: ' + ctx.planDir);
  console.log('Phases:   ' + phases.join(' -> '));
  console.log('');
  console.log('Wrote:');
  console.log('  ' + planPath);
  phases.forEach((_, i) => console.log('  ' + path.join(ctx.planDir, 'phase-' + pad(i + 1) + '.md')));
  console.log('  ' + metaPath);
  console.log('');
  console.log('Review plan.md. Do not start work — wait for the user\'s go-ahead.');
}

main();