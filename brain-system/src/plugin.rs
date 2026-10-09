//! `brain-system.exe --install-claude-plugin [--no-register] [DIR]`
//!
//! Writes a Claude Code plugin marketplace containing the `global-brain`
//! plugin: an MCP server entry pointing at *this* exe (`--mcp`) plus a skill
//! telling Claude when to consult the brain. Then registers it with the
//! `claude` CLI unless `--no-register` is given.

use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{config, util};

const MARKETPLACE: &str = "global-brain";
const PLUGIN: &str = "global-brain";

const SKILL_MD: &str = r#"---
name: global-brain
description: Consult and update the Global Brain — the shared memory of every past agent session (Claude Code, Codex, MiniMax), the tools and skills they used, the Obsidian memory vault and the skills library. Use at the start of non-trivial work to find prior sessions, decisions and notes on the same project or topic, when the user refers to earlier work ("like last time", "what did we do on X"), and at the end of meaningful work to record what was learned.
---

# Global Brain

The `global-brain` MCP server exposes the brain's index:

| Tool | Use it to |
|---|---|
| `brain_project` | **start here**: the current project's sessions ranked by weight (activity × recency) with summaries, plus its skills, tools and most-changed files |
| `brain_node` | read one node: a session's digest (prompt timeline, files changed, last reply, tools, skills, sub-agents), a note's text, a skill's usage history |
| `brain_search` | find prior conversations, notes, skills or projects about a topic |
| `brain_recent` | latest (or `sort: weight` heaviest) conversations across all agents and projects |
| `brain_status` | counts, vault location, pending verification proposals |
| `brain_add_entry` | append a dated lesson/decision to any node's log |
| `brain_save_note` | create a new vault note (00-Inbox, or 30-Logs for a session log) |

## When to use it

- **Before starting** substantial work on a project: `brain_project` (no
  arguments = your working folder), then `brain_node` on the one or two
  sessions that matter. Use `brain_search` for a feature or topic. Reuse what
  was learned — don't redo an audit or re-decide something already settled.
- **Digests first, transcripts last.** A session digest is a few hundred
  tokens; its transcript can be megabytes. Open the transcript file only when
  the digest doesn't answer the question.
- **When the user mentions earlier work** ("the audit from yesterday", "like we
  did in GameForger"): search before asking them to repeat it.
- **After finishing** something worth remembering: `brain_add_entry` on the
  skill, project or note it concerns (one or two sentences: what worked, what
  to avoid), or `brain_save_note` for a fuller write-up. Link related notes
  with `[[Note Name]]`.

## Rules

- Entries and notes are permanent shared memory read by every future agent:
  write facts and decisions, not chatter. Never store secrets, tokens or
  personal data.
- `brain_save_note` never overwrites; if a note exists, add an entry instead.
- Gold notes (10-Entities, 20-Concepts) are curated by the user through the
  Verifier Librarian — propose changes in a new Inbox note rather than editing
  them directly.
- Node ids look like `conv:claude:<uuid>`, `skill:<name>`, `note:<vault path>`,
  `proj:<path>`; always take them from tool results.
"#;

pub fn install(args: &[String]) {
    let register = !args.iter().any(|a| a == "--no-register");
    let dir = args.iter().find(|a| !a.starts_with("--")).map(PathBuf::from).unwrap_or_else(default_dir);
    let exe = std::env::current_exe().map(|p| p.canonicalize().unwrap_or(p)).unwrap_or_default();
    let exe_str = util::display_path(&exe).replace('/', "\\");

    if let Err(e) = write_marketplace(&dir, &exe_str) {
        eprintln!("Could not write the plugin to {}: {}", dir.display(), e);
        std::process::exit(1);
    }
    println!("Wrote Claude Code plugin marketplace to {}", dir.display());
    println!("  MCP server: {} --mcp", exe_str);
    if !register {
        println!("\nRegister it with:\n  claude plugin marketplace add \"{}\"\n  claude plugin install {}@{}", dir.display(), PLUGIN, MARKETPLACE);
        return;
    }
    // add + install on first run; update keeps an existing install current.
    let steps: [Vec<String>; 4] = [
        vec!["plugin".into(), "marketplace".into(), "add".into(), dir.to_string_lossy().into()],
        vec!["plugin".into(), "marketplace".into(), "update".into(), MARKETPLACE.into()],
        vec!["plugin".into(), "install".into(), format!("{}@{}", PLUGIN, MARKETPLACE)],
        vec!["plugin".into(), "update".into(), format!("{}@{}", PLUGIN, MARKETPLACE)],
    ];
    for step in steps {
        println!("\n> claude {}", step.join(" "));
        // `claude` is usually a .cmd/.exe shim on PATH; go through cmd so both work.
        match Command::new("cmd").arg("/C").arg("claude").args(&step).status() {
            Ok(s) if s.success() => {}
            Ok(s) => println!("  (exit code {:?} — if it says it already exists, that's fine)", s.code()),
            Err(e) => { eprintln!("  could not run the claude CLI: {}", e); return; }
        }
    }
    println!("\nDone. Restart Claude Code; the global-brain tools appear as mcp__plugin_global-brain_global-brain__*.");
}

fn default_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
        .or_else(config::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("BrainSystem").join("claude-marketplace")
}

fn write_marketplace(dir: &Path, exe: &str) -> std::io::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let description = "Connects Claude to the Global Brain: search and read every past agent session, tool, skill and vault note, and record what was learned.";
    let write = |rel: &str, body: String| util::atomic_write(&dir.join(rel), body.as_bytes());

    write(".claude-plugin/marketplace.json", serde_json::to_string_pretty(&json!({
        "name": MARKETPLACE,
        "owner": { "name": "Erik Boivin" },
        "metadata": { "description": "Local marketplace generated by brain-system.exe", "version": version },
        "plugins": [{ "name": PLUGIN, "source": format!("./{}", PLUGIN), "description": description, "version": version }],
    })).unwrap())?;

    write(&format!("{}/.claude-plugin/plugin.json", PLUGIN), serde_json::to_string_pretty(&json!({
        "name": PLUGIN,
        "version": version,
        "description": description,
        "author": { "name": "Erik Boivin" },
        "keywords": ["memory", "brain", "knowledge-graph", "mcp"],
    })).unwrap())?;

    let mut server = json!({ "command": exe, "args": ["--mcp"] });
    if config::port() != config::DEFAULT_PORT {
        server["env"] = json!({ "BRAIN_PORT": config::port().to_string() });
    }
    write(&format!("{}/.mcp.json", PLUGIN), serde_json::to_string_pretty(&json!({ "mcpServers": { "global-brain": server } })).unwrap())?;

    write(&format!("{}/skills/global-brain/SKILL.md", PLUGIN), SKILL_MD.to_string())?;
    Ok(())
}
