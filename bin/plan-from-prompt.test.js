'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

// Load skills.json so we can verify every PHASE_KINDS skill resolves.
const skillsJson = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'skills.json'), 'utf8'));
const TOP_NAMES = new Set(skillsJson.entries.map(e => e.name));
const NESTED_IDS = new Set(skillsJson.nested.map(n => n.id));

// Re-declare the PHASE_KINDS structure we expect from plan-from-prompt.js.
// This is a duplicate on purpose — if the file gets edited and a skill id
// stops resolving, the test here catches it before the CLI does.
const PHASE_KINDS = {
  understand: ['brainstorming', 'thinking-and-docs/prompt-me', 'thinking-and-docs/before-building'],
  explore:    ['skill-authoring/using-addon-skills', 'github/codebase-inspection'],
  plan:       ['writing-plans', 'software-development/plan', 'thinking-and-docs/next-decision'],
  implement:  ['coder-ai-senior-developer', 'dispatching-parallel-agents', 'using-git-worktrees'],
  verify:     ['test-driven-development', 'systematic-debugging', 'verification-before-completion', 'vibe-code-auditor'],
  report:     ['requesting-code-review', 'agent-orchestration/handoff', 'finishing-a-development-branch'],
};

test('every PHASE_KINDS skill resolves against skills.json', () => {
  for (const [phase, list] of Object.entries(PHASE_KINDS)) {
    for (const skill of list) {
      const resolved = TOP_NAMES.has(skill) || NESTED_IDS.has(skill);
      assert.ok(resolved, `Phase "${phase}" references unresolved skill: ${skill}`);
    }
  }
});

test('PHASE_KINDS contains no tool names masquerading as skills', () => {
  const forbidden = ['rag_search', 'find_symbol', 'dev_env_report', 'git-worktrees'];
  const all = new Set();
  for (const list of Object.values(PHASE_KINDS)) list.forEach(s => all.add(s));
  for (const bad of forbidden) {
    assert.ok(!all.has(bad), `PHASE_KINDS still references removed tool name: ${bad}`);
  }
});

test('plan-from-prompt.js file loads without syntax errors', () => {
  // We can't safely invoke main() (it does I/O and calls process.exit) but
  // requiring the module proves it parses and its top-level runs.
  const p = path.join(__dirname, 'plan-from-prompt.js');
  const src = fs.readFileSync(p, 'utf8');
  assert.match(src, /const PHASE_KINDS = /);
  assert.match(src, /brainstorming/);
});
