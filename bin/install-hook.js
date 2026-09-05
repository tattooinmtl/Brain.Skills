#!/usr/bin/env node
/**
 * install-hook.js — install a hook JSON into one or more agent harnesses.
 * See C:/.skills/docs/PROTOCOL.md for the full plan-and-dispatch loop.
 *
 * Usage:
 *   node install-hook.js <hook.json> --harness <name> [--dry-run]
 *   node install-hook.js <hook.json> --harness all
 *
 * Supported: omni, claude, codex, cursor, pi, hermes, factory,
 * opencode, devin, nimagent, gemini, kimicode.
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const HARNESS_CONFIGS = {
  omni:      { name: 'Omni Agent',    hooksDir: '~/.omni/hooks',                configFile: '~/.omni/omni.config.json',     hookField: 'hooks'       },
  claude:    { name: 'Claude Code',   hooksDir: '~/.claude/hooks',              configFile: '~/.claude/settings.json',      hookField: 'hooks'       },
  codex:     { name: 'OpenAI Codex',  hooksDir: '~/.codex/hooks',               configFile: '~/.codex/hooks.json',          hookField: null          },
  cursor:    { name: 'Cursor',        hooksDir: '~/.cursor/hooks',              configFile: '~/.cursor/settings.json',      hookField: 'hooks'       },
  pi:        { name: 'Pi Dev',        hooksDir: '~/.pi/hooks',                  configFile: null,                           hookField: null          },
  hermes:    { name: 'Hermes Agent',  hooksDir: '~/.hermes/plugins/command-guard', configFile: '~/.hermes/config.json',     hookField: 'commandGuard' },
  factory:   { name: 'Factory AI',    hooksDir: '~/.factory/hooks',             configFile: '~/.factory/settings.json',     hookField: 'hooks'       },
  opencode:  { name: 'OpenCode',      hooksDir: '~/.config/opencode/plugins',   configFile: '~/.config/opencode/config.json', hookField: 'plugins'   },
  devin:     { name: 'Devin',         hooksDir: '~/.devin/hooks',               configFile: '~/.devin/config.json',         hookField: 'hooks'       },
  nimagent:  { name: 'NimAgent',      hooksDir: '~/.nimagent/hooks',            configFile: '~/.nimagent/nimagent.config.json', hookField: 'hooks'   },
  nimagent2: { name: 'NimAgent v2',   hooksDir: '~/.nimagent2/hooks',           configFile: '~/.nimagent2/nimagent.config.json', hookField: 'hooks'  },
  gemini:    { name: 'Gemini CLI',    hooksDir: '~/.gemini/hooks',              configFile: '~/.gemini/settings.json',      hookField: 'hooks'       },
  kimicode:  { name: 'Kimi Code CLI', hooksDir: '~/.kimi-code/hooks',           configFile: '~/.kimi-code/config.toml',     hookField: 'hooks'       },
  minimax:   { name: 'MiniMax CLI',   hooksDir: '~/.minimax/hooks',             configFile: null,                           hookField: null          },
};

function expandTilde(p) {
  const home = process.platform === 'win32'
    ? (process.env.USERPROFILE || process.env.HOME)
    : (process.env.HOME || process.env.USERPROFILE);
  return p.replace(/^~/, home);
}

function parseArgs() {
  const args = process.argv.slice(2);
  const out = { hook: null, harness: null, dryRun: false };
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--harness' && args[i + 1]) out.harness = args[++i];
    else if (args[i] === '--dry-run') out.dryRun = true;
    else if (!args[i].startsWith('--')) out.hook = args[i];
  }
  return out;
}

function validateHook(hookFile) {
  const validator = path.join(__dirname, 'validate-hook.js');
  try {
    execSync('node "' + validator + '" "' + hookFile + '"', { stdio: 'inherit' });
    return true;
  } catch (e) { return false; }
}

function installOne(hookFile, harness, dryRun) {
  const cfg = HARNESS_CONFIGS[harness];
  if (!cfg) throw new Error('Unknown harness: ' + harness);
  const hooksDir = expandTilde(cfg.hooksDir);
  const dest = path.join(hooksDir, path.basename(hookFile));
  if (dryRun) {
    console.log('[dry-run] ' + cfg.name + ': ' + hookFile + ' -> ' + dest);
    return;
  }
  fs.mkdirSync(hooksDir, { recursive: true });
  fs.copyFileSync(hookFile, dest);
  console.log('OK ' + cfg.name + ': ' + dest);
  if (cfg.configFile && cfg.hookField) {
    const configPath = expandTilde(cfg.configFile);
    // Only touch JSON configs. TOML/YAML/JSONC configs vary too much per
    // harness — leave them alone and rely on the hook file's presence in
    // hooksDir plus the harness's own auto-discovery.
    if (!/\.json$/i.test(configPath)) {
      console.log('SKIP config update (non-JSON): ' + configPath);
    } else {
      try {
        let config = {};
        if (fs.existsSync(configPath)) config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
        if (!config[cfg.hookField]) config[cfg.hookField] = [];
        const hookName = path.basename(hookFile);
        if (!Array.isArray(config[cfg.hookField])) {
          console.log('SKIP config update: "' + cfg.hookField + '" is not an array (harness uses a richer format — already wired?)');
        } else if (!config[cfg.hookField].includes(hookName)) {
          config[cfg.hookField].push(hookName);
          fs.writeFileSync(configPath, JSON.stringify(config, null, 2) + '\n');
          console.log('OK Updated config: ' + configPath);
        }
      } catch (e) {
        console.warn('WARNING: failed to update ' + configPath + ': ' + e.message);
      }
    }
  }
}

function main() {
  const { hook, harness, dryRun } = parseArgs();
  if (!hook || !harness) {
    console.error('Usage: node install-hook.js <hook.json> --harness <name|all> [--dry-run]');
    process.exit(1);
  }
  if (!validateHook(hook)) {
    console.error('FAIL Hook failed validation: ' + hook);
    process.exit(1);
  }
  const targets = harness === 'all' ? Object.keys(HARNESS_CONFIGS) : [harness];
  for (const h of targets) {
    try { installOne(hook, h, dryRun); }
    catch (e) { console.error('FAIL ' + h + ': ' + e.message); }
  }
}

main();