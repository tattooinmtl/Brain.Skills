# brain-system

Windows tray app + local web UI for the Global Brain: a live 3D neural view of
every agent session, tool call, sub-agent, skill and vault note, plus the wiki,
the Verifier Librarian and the skills library.

```
brain-system.exe                 tray + server on http://127.0.0.1:6789
brain-system.exe --verify        one verification sweep, print a summary
brain-system.exe --install-startup / --uninstall-startup
```

## What the Neural Brain shows

| Colour | Node | Source |
|---|---|---|
| white core | **Global Brain** | the root |
| orange | **Project** | a working directory (worktrees fold into their repo) |
| blue | **Conversation** | one agent transcript |
| green | **Tools** | one node per conversation, aggregating every tool call |
| purple | **Sub-agent** | each sub-agent a conversation spawned |
| yellow | **Skill** | one node per skill, shared by every conversation that used it |
| white | **Session** | `30-Logs/session-*` notes in the vault |
| teal / pink / slate | **Note** | entity / concept / other vault notes, linked by `[[wikilinks]]` |
| grey | **Unresolved link** | a `[[link]]` with no note yet (hidden by default) |

Nodes pulse and light travels along the edges as agents work: a new prompt
lights `brain → project → conversation`, a tool call lights `conversation → tools`,
a skill use continues on to the skill, and a file edit lights the vault note it
touched. Conversations whose transcript changed in the last two minutes keep
breathing.

Controls: drag to rotate, right-drag to pan, wheel zooms toward the cursor,
click a node to open its panel, double-click to fly to it, **F** for free
flight (WASD, Space/C up and down, Shift boost, wheel sets speed, Esc exits),
**/** to search. The legend toggles each category.

The side panel's **Edit** lets you rename a node, give it a colour and a note
(stored in `<vault>/_system/brain-graph/overlay.json`), edit the underlying
markdown file (vault notes and `SKILL.md`), and append entries to any node's
log. File saves are refused if the file changed on disk since you opened it.

## Where the data comes from

The indexer polls every 1.5 s and reads only what was appended since the last
poll.

* **Claude Code**: `~/.claude/projects/<project>/<session>.jsonl` and
  `<session>/subagents/agent-*.jsonl`
* **Codex CLI**: `~/.codex/sessions/**/rollout-*.jsonl`
* **Any other agent** (MiniMax, omni, scripts): append JSON lines to
  `<vault>/_system/brain-events/<anything>.jsonl`

```jsonl
{"ts":"2026-10-07T23:55:08Z","harness":"minimax","session_id":"abc","kind":"prompt","text":"Fix the parser","cwd":"C:\\proj","title":"Parser fix"}
{"ts":"2026-10-07T23:55:10Z","harness":"minimax","session_id":"abc","kind":"tool","name":"read_file","path":"C:\\proj\\src\\main.rs","result":"ok"}
{"ts":"2026-10-07T23:55:12Z","harness":"minimax","session_id":"abc","kind":"skill","name":"rust-guidelines"}
{"ts":"2026-10-07T23:55:15Z","harness":"minimax","session_id":"abc","kind":"subagent","name":"Review the diff","agent_type":"reviewer"}
```

`kind` is one of `prompt`, `tool`, `skill`, `subagent`. `harness` +
`session_id` identify the conversation. `title`, `cwd`, `model`, `path`,
`args`, `result` are optional.

## Claude (and other agents) reading the brain: MCP

`brain-system.exe --mcp` is a Model Context Protocol server on stdio. It
talks to the running brain and starts it if needed. Tools:

| Tool | Does |
|---|---|
| `brain_search` | search conversations, notes, skills, projects (names, note text, skill descriptions, what sessions did) |
| `brain_recent` | latest conversations across all agents, live ones flagged |
| `brain_node` | read a node: details, connections, tool/skill usage, recent calls, file content, entry log |
| `brain_status` | counts, paths, pending verification proposals |
| `brain_add_entry` | append a dated entry to any node (source `claude`) |
| `brain_save_note` | create a new note in `00-Inbox` or `30-Logs` (never overwrites) |

Install as a Claude Code plugin (MCP server + a skill telling Claude when to
use it):

```
brain-system.exe --install-claude-plugin            # writes %LOCALAPPDATA%\BrainSystem\claude-marketplace and registers it
brain-system.exe --install-claude-plugin --no-register [DIR]   # just write it
```

Or add only the MCP server: `claude mcp add --scope user global-brain -- "C:\.skills\bin\brain-system.exe" --mcp`.
Any other MCP client works the same way.

## Verifier Librarian

Gold notes (`10-Entities`, `20-Concepts`) are tracked by SHA-256 in
`<vault>/_system/verifier/manifest.json`, with snapshots of each verified
version. A sweep (20 s after start, then every 10 min, or on demand) proposes:

* **drift**: a Gold note changed outside the UI. *Accept* makes the new
  version verified. *Reject & restore* puts back the verified version; the
  rejected text is archived as `_system/archive/<id>.rejected.md`.
* **link**: a broken `[[link]]` that matches an existing note by
  case, spacing or a small typo. *Apply* rewrites the link.

Edits made through the UI count as verified. Every resolved proposal is
archived in `_system/archive/` and is never proposed again.

## Security

The server binds to 127.0.0.1 only and:

* rejects any request whose `Host` header isn't `127.0.0.1:<port>` or
  `localhost:<port>` (blocks DNS rebinding);
* rejects any POST whose `Origin` isn't the UI's own;
* serves the UI under a strict Content-Security-Policy (no inline scripts);
* confines reads and writes to the vault and skills library, `.md` only, never
  `_system/`.

## Configuration

| Env var | Default |
|---|---|
| `BRAIN_PORT` | `6789` |
| `BRAIN_VAULT_DIR` | `<repo>/memory` |
| `BRAIN_SKILLS_DIR` | `<repo>/skills` |
| `BRAIN_CLAUDE_PROJECTS` | `~/.claude/projects` |
| `BRAIN_CODEX_SESSIONS` | `~/.codex/sessions` |
| `BRAIN_WEB_DIR` | unset. Development only: serve `src/web/*` from disk instead of the embedded copies |

One instance runs per port. Launching again just opens the UI.

## Building

```
cargo build --release
cargo test
```

The UI (`src/web/`) is embedded in the exe. three.js r160 is vendored under
`src/web/vendor/` (MIT, see `THREE-LICENSE.txt`), so the viewer works offline.
