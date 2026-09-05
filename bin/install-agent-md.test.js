'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

const SCRIPT = path.join(__dirname, 'install-agent-md.js');
const CANONICAL_PATH = path.join(__dirname, '..', 'agents', 'AGENTS.md');

test('canonical AGENTS.md exists and is valid markdown', () => {
  assert.ok(fs.existsSync(CANONICAL_PATH), 'agents/AGENTS.md must exist');
  const body = fs.readFileSync(CANONICAL_PATH, 'utf8');
  // First line must be an HTML comment marker, not a doctype
  const first = body.split('\n')[0];
  assert.match(first, /^<!-- name-agnostic -->/);
  assert.ok(!body.startsWith('<!DOCTYPE'), 'template must not start with an HTML doctype');
  assert.ok(body.length > 500, 'template must have real content');
});

test('install writes template into a target project dir', () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'agent-md-test-'));
  try {
    execFileSync('node', [SCRIPT, '--target', 'project', '--project', tmp, '--name', 'TEST.md'], { stdio: 'pipe' });
    const dest = path.join(tmp, 'TEST.md');
    assert.ok(fs.existsSync(dest));
    const body = fs.readFileSync(dest, 'utf8');
    assert.match(body, /Agent Entry Point/);
    assert.match(body, /Plan-and-dispatch/);
    // Same content as canonical (installer reads from disk, no drift)
    assert.equal(body, fs.readFileSync(CANONICAL_PATH, 'utf8'));
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});

test('--dry-run does not write', () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'agent-md-dry-'));
  try {
    execFileSync('node', [SCRIPT, '--target', 'project', '--project', tmp, '--name', 'AGENTS.md', '--dry-run'], { stdio: 'pipe' });
    assert.ok(!fs.existsSync(path.join(tmp, 'AGENTS.md')));
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});
