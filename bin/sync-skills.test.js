'use strict';

const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  discoverTopLevelSkills,
  discoverAllSkills,
  installMissing,
  writeSkillsJson,
  writeIndexMd,
} = require('./sync-skills.js');

let tmpRoot;

before(() => {
  tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sync-skills-'));
});

after(() => {
  fs.rmSync(tmpRoot, { recursive: true, force: true });
});

function writeSkill(dir, name, body) {
  const skillDir = path.join(dir, name);
  fs.mkdirSync(skillDir, { recursive: true });
  fs.writeFileSync(path.join(skillDir, 'SKILL.md'), body || '---\nname: ' + name + '\n---\n');
  return skillDir;
}

test('discoverTopLevelSkills includes only directories with SKILL.md', () => {
  const source = path.join(tmpRoot, 'lib-a');
  fs.mkdirSync(source, { recursive: true });
  writeSkill(source, 'cpp-lua-engine');
  writeSkill(source, 'cpp-game-engine');
  fs.mkdirSync(path.join(source, 'creative'), { recursive: true });
  writeSkill(path.join(source, 'creative'), 'comfyui');
  fs.mkdirSync(path.join(source, '.hidden-skill'), { recursive: true });
  fs.writeFileSync(path.join(source, '.hidden-skill', 'SKILL.md'), '---\nname: hidden\n---\n');
  fs.mkdirSync(path.join(source, 'index-cache'), { recursive: true });
  fs.writeFileSync(path.join(source, 'README.md'), '# not a skill\n');

  const found = discoverTopLevelSkills(source).map((s) => s.name).sort();
  assert.deepEqual(found, ['cpp-game-engine', 'cpp-lua-engine']);
});

test('installMissing junctions new skills and leaves existing folders alone', () => {
  const source = path.join(tmpRoot, 'lib-b');
  const dest = path.join(tmpRoot, 'agents-b');
  fs.mkdirSync(source, { recursive: true });
  fs.mkdirSync(dest, { recursive: true });

  const lua = writeSkill(source, 'cpp-lua-engine');
  writeSkill(source, 'already-there');
  const existing = path.join(dest, 'already-there');
  fs.mkdirSync(existing, { recursive: true });
  fs.writeFileSync(path.join(existing, 'KEEP.txt'), 'do-not-touch');

  const skills = discoverTopLevelSkills(source);
  const result = installMissing(skills, dest);

  assert.equal(result.added.includes('cpp-lua-engine'), true);
  assert.equal(result.skipped.includes('already-there'), true);
  assert.equal(fs.readFileSync(path.join(existing, 'KEEP.txt'), 'utf8'), 'do-not-touch');
  assert.equal(fs.existsSync(path.join(dest, 'cpp-lua-engine', 'SKILL.md')), true);
  const st = fs.lstatSync(path.join(dest, 'cpp-lua-engine'));
  assert.equal(st.isSymbolicLink() || st.isDirectory(), true);
  assert.equal(fs.realpathSync(path.join(dest, 'cpp-lua-engine')), fs.realpathSync(lua));
});

test('writeSkillsJson and writeIndexMd list discovered skill names', () => {
  const source = path.join(tmpRoot, 'lib-c');
  fs.mkdirSync(source, { recursive: true });
  writeSkill(source, 'cpp-lua-engine');
  const skills = discoverTopLevelSkills(source);
  const jsonPath = path.join(tmpRoot, 'skills.json');
  const indexPath = path.join(tmpRoot, 'INDEX.md');
  writeSkillsJson(skills, jsonPath, source);
  writeIndexMd(skills, indexPath);

  const catalog = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
  assert.equal(catalog.generated, true);
  assert.equal(catalog.count, 1);
  assert.equal(catalog.entries[0].name, 'cpp-lua-engine');
  assert.match(catalog.entries[0].path, /cpp-lua-engine$/);

  const index = fs.readFileSync(indexPath, 'utf8');
  assert.match(index, /cpp-lua-engine/);
  assert.match(index, /Top-level skills \(1\)/);
});

test('discoverAllSkills walks recursively and marks nested skills', () => {
  const source = path.join(tmpRoot, 'lib-d');
  fs.mkdirSync(source, { recursive: true });
  writeSkill(source, 'cpp-lua-engine');            // top-level
  writeSkill(source, 'superpowers');                // pack root
  writeSkill(path.join(source, 'superpowers'), 'writing-plans'); // nested
  writeSkill(path.join(source, 'languages'), 'cpp-coding');       // nested only

  const all = discoverAllSkills(source);
  const ids = all.map(s => s.id).sort();
  assert.deepEqual(ids, [
    'cpp-lua-engine',
    'languages/cpp-coding',
    'superpowers',
    'superpowers/writing-plans',
  ]);
  const nested = all.filter(s => s.depth > 0);
  assert.equal(nested.length, 2);
  const wp = all.find(s => s.id === 'superpowers/writing-plans');
  assert.equal(wp.parentPack, 'superpowers');
  assert.equal(wp.depth, 1);
});

test('writeSkillsJson emits nested[] alongside top-level entries', () => {
  const source = path.join(tmpRoot, 'lib-e');
  fs.mkdirSync(source, { recursive: true });
  writeSkill(source, 'cpp-lua-engine');
  writeSkill(path.join(source, 'languages'), 'cpp-coding');
  const skills = discoverTopLevelSkills(source);
  const nested = discoverAllSkills(source).filter(s => s.depth > 0);
  const jsonPath = path.join(tmpRoot, 'skills-e.json');
  const indexPath = path.join(tmpRoot, 'INDEX-e.md');
  writeSkillsJson(skills, jsonPath, source, nested);
  writeIndexMd(skills, indexPath, nested);

  const catalog = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
  assert.equal(catalog.count, 1);
  assert.equal(catalog.nestedCount, 1);
  assert.equal(catalog.nested[0].id, 'languages/cpp-coding');
  assert.equal(catalog.nested[0].parentPack, 'languages');

  const index = fs.readFileSync(indexPath, 'utf8');
  assert.match(index, /Top-level skills \(1\)/);
  assert.match(index, /Nested library skills \(1\)/);
  assert.match(index, /languages\/cpp-coding/);
});

test('discoverAllSkills preserves colliding names via unique id', () => {
  const source = path.join(tmpRoot, 'lib-f');
  fs.mkdirSync(source, { recursive: true });
  writeSkill(source, 'writing-plans');
  writeSkill(path.join(source, 'software-development'), 'writing-plans');
  writeSkill(path.join(source, 'superpowers'), 'writing-plans');

  const all = discoverAllSkills(source);
  const ids = all.map(s => s.id).sort();
  assert.deepEqual(ids, [
    'software-development/writing-plans',
    'superpowers/writing-plans',
    'writing-plans',
  ]);
  // All three share the same base name — id is what makes them unique.
  const names = new Set(all.map(s => s.name));
  assert.equal(names.size, 1);
  assert.equal([...names][0], 'writing-plans');
});
