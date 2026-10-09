//! Session ingest. Turns agent transcripts into conversations, tool calls,
//! sub-agents, skill uses and touched files — incrementally: each file is
//! read from the byte offset where the previous poll stopped, so a live
//! session costs only its newly appended lines.
//!
//! Sources:
//!   * Claude Code  `~/.claude/projects/<proj>/<session>.jsonl` (+ `<session>/subagents/agent-*.jsonl`)
//!   * Codex CLI    `~/.codex/sessions/**/rollout-*.jsonl`
//!   * Any agent    `<vault>/_system/brain-events/*.jsonl` (format in README.md)
//!
//! Each conversation also keeps a small digest (prompt timeline, files it
//! changed, its last reply) so agents can learn what a session did without
//! opening the transcript. The whole index (offsets plus digests) is saved
//! to `<cache>/sessions-index.json`, so a restart reads only new bytes.
//! Files are read in bounded chunks, so memory stays flat however large a
//! transcript grows.

use serde_json::Value;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::{config, skills, util};

const RECENT_CALLS: usize = 40;
const MAX_TOUCHED: usize = 400;
const MAX_EVENTS: usize = 600;
/// Prompt timeline per conversation: the first few plus the most recent.
const PROMPT_HEAD: usize = 4;
const PROMPT_TAIL: usize = 36;
/// Bytes read per step; a single longer line grows the buffer just for it.
const CHUNK: usize = 4 << 20;
const CACHE_VERSION: u32 = 1;

/// Claude Code CLI commands that look like `/name` but are not skills.
const BUILTIN_COMMANDS: &[&str] = &[
    "add-dir", "agents", "artifacts", "bashes", "bug", "chrome", "clear", "compact", "config", "context",
    "cost", "desktop", "doctor", "effort", "exit", "export", "fast", "feedback", "help", "hooks", "ide",
    "install-github-app", "login", "logout", "mcp", "memory", "migrate-installer", "mobile", "model",
    "output-style", "permissions", "plugin", "plugins", "pr-comments", "privacy-settings", "release-notes",
    "remote-control", "resume", "rewind", "sandbox", "status", "statusline", "tasks", "terminal-setup",
    "theme", "todos", "upgrade", "usage", "vim", "auto-mode-setup",
];

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Call {
    pub ts: i64,
    pub tool: String,
    pub args: String,
    pub result: String,
    #[serde(default)]
    pub id: String,
    pub by: Option<String>, // sub-agent key when the call came from a sub-agent
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Sub {
    pub key: String,
    pub agent_id: Option<String>,
    pub agent_type: String,
    pub description: String,
    pub started: i64,
    pub last: i64,
    pub calls: u32,
    pub tool_counts: BTreeMap<String, u32>,
    pub skills: BTreeMap<String, u32>,
    #[serde(default)]
    pub touched: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Prompt {
    pub ts: i64,
    pub text: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Conv {
    pub key: String,
    pub harness: String,
    pub session_id: String,
    pub file: PathBuf,
    pub cwd: String,
    pub title: String,
    pub custom_title: bool,
    pub first_prompt: String,
    pub started: i64,
    pub last: i64,
    pub prompts: u32,
    pub turns: u32,
    pub model: String,
    pub branch: String,
    pub parent: Option<String>, // conv key of the parent thread (codex reviewer threads)
    pub tool_counts: BTreeMap<String, u32>,
    pub recent: VecDeque<Call>,
    pub subs: Vec<Sub>,
    pub skills: BTreeMap<String, u32>,
    pub touched: BTreeMap<String, u32>,
    // --- digest: what happened, without the transcript ---
    pub prompt_log: Vec<Prompt>,
    /// Prompts left out of the timeline between head and tail.
    pub prompts_skipped: u32,
    pub last_reply: String,
    /// Write/Edit/patch calls.
    pub edits: u32,
    /// Files this session wrote to (any path), with edit counts.
    pub changed: BTreeMap<String, u32>,
}

impl Conv {
    fn new(key: String, harness: &str, session_id: &str, file: &Path) -> Self {
        Conv { key, harness: harness.into(), session_id: session_id.into(), file: file.to_path_buf(), ..Default::default() }
    }
    pub fn total_calls(&self) -> u32 { self.tool_counts.values().sum() }
    fn log_prompt(&mut self, ts: i64, text: &str) {
        let text = util::one_line(text, 220);
        if text.is_empty() { return; }
        self.prompt_log.push(Prompt { ts, text });
        if self.prompt_log.len() > PROMPT_HEAD + PROMPT_TAIL {
            self.prompt_log.remove(PROMPT_HEAD);
            self.prompts_skipped += 1;
        }
    }
    fn stamp(&mut self, ts: i64) {
        if ts <= 0 { return; }
        if self.started == 0 || ts < self.started { self.started = ts; }
        if ts > self.last { self.last = ts; }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Event {
    pub seq: u64,
    pub ts: i64,
    pub conv: String,
    pub kind: &'static str, // prompt | tool | skill | subagent | touch | title
    pub name: String,
    pub sub: Option<String>,
    /// One short line for the live view: the file, command or query.
    pub detail: String,
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum Source { Claude, ClaudeSub, Codex, Sink }

#[derive(serde::Serialize, serde::Deserialize)]
struct FileState {
    source: Source,
    offset: u64,
    size: u64,
    mtime: i64,
    conv: Option<String>, // owning conversation (Claude/Codex/ClaudeSub)
    sub_key: Option<String>,
}

#[derive(Default)]
pub struct Ingest {
    files: HashMap<PathBuf, FileState>,
    pub convs: HashMap<String, Conv>,
    pub events: VecDeque<Event>,
    pub seq: u64,
    pub live_files: HashMap<String, i64>, // conv key -> transcript mtime
    emit: bool,
    structural: bool,
    /// Any transcript grew since the caller last cleared this.
    pub dirty: bool,
    /// Changed since the index was last saved to disk.
    pub unsaved: bool,
}

#[derive(serde::Deserialize)]
struct CacheFile {
    version: u32,
    files: Vec<(PathBuf, FileState)>,
    convs: Vec<Conv>,
}

impl Ingest {
    /// Poll all sources. Returns true when nodes/edges may have changed.
    /// Events are only recorded after the first full load, so the activity
    /// feed shows what is happening now — not a replay of history.
    pub fn poll(&mut self, emit_events: bool) -> bool {
        self.emit = emit_events;
        self.structural = false;
        let mut seen: Vec<PathBuf> = Vec::new();
        for (path, source) in discover() {
            seen.push(path.clone());
            self.poll_file(&path, source);
        }
        // Forget transcripts that were deleted.
        let gone: Vec<PathBuf> = self.files.keys().filter(|p| !seen.contains(p)).cloned().collect();
        for p in gone {
            self.unsaved = true;
            if let Some(st) = self.files.remove(&p) {
                if matches!(st.source, Source::Claude | Source::Codex) {
                    if let Some(k) = st.conv { self.convs.remove(&k); self.structural = true; }
                }
            }
        }
        self.structural
    }

    fn poll_file(&mut self, path: &Path, source: Source) {
        let meta = match fs::metadata(path) { Ok(m) => m, Err(_) => return };
        let size = meta.len();
        let mtime = meta.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64).unwrap_or(0);
        let st = self.files.entry(path.to_path_buf()).or_insert(FileState {
            source, offset: 0, size: 0, mtime: 0, conv: None, sub_key: None,
        });
        if st.size == size && st.mtime == mtime { return; }
        self.dirty = true;
        self.unsaved = true;
        if size < st.offset {
            // Truncated / rewritten: start over.
            st.offset = 0;
            if matches!(source, Source::Claude | Source::Codex) {
                if let Some(k) = st.conv.take() { self.convs.remove(&k); }
            }
        }
        st.size = size;
        st.mtime = mtime;
        let Ok(mut f) = File::open(path) else { return };
        // Bounded chunks: memory stays ~CHUNK however big the transcript is.
        loop {
            let offset = self.files[path].offset;
            if offset >= size { break; }
            let Some(buf) = read_chunk(&mut f, offset) else { break };
            let Some(end) = buf.iter().rposition(|&b| b == b'\n') else { break }; // partial last line
            self.files.get_mut(path).unwrap().offset = offset + end as u64 + 1;
            let text = String::from_utf8_lossy(&buf[..end]);
            self.ingest_text(path, source, &text, mtime);
        }
    }

    fn ingest_text(&mut self, path: &Path, source: Source, text: &str, mtime: i64) {
        let st_conv = self.files[path].conv.clone();
        let st_sub = self.files[path].sub_key.clone();
        match source {
            Source::Claude => {
                let key = st_conv.unwrap_or_else(|| {
                    let sid = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    format!("claude:{}", sid)
                });
                if !self.convs.contains_key(&key) {
                    let sid = key.trim_start_matches("claude:").to_string();
                    self.convs.insert(key.clone(), Conv::new(key.clone(), "claude", &sid, path));
                    self.structural = true;
                }
                self.files.get_mut(path).unwrap().conv = Some(key.clone());
                for line in text.lines() { self.claude_line(&key, None, line); }
                self.live_files.insert(key, mtime);
            }
            Source::ClaudeSub => {
                // <proj>/<session>/subagents/agent-<id>.jsonl
                let sid = path.parent().and_then(|p| p.parent()).and_then(|p| p.file_name())
                    .map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let key = format!("claude:{}", sid);
                if !self.convs.contains_key(&key) { return; } // parent not loaded (yet)
                let agent_id = path.file_stem().unwrap_or_default().to_string_lossy()
                    .trim_start_matches("agent-").to_string();
                let sub_key = match st_sub {
                    Some(k) => k,
                    None => {
                        let meta = read_sub_meta(path);
                        let k = self.attach_sub(&key, &agent_id, meta);
                        self.files.get_mut(path).unwrap().sub_key = Some(k.clone());
                        k
                    }
                };
                self.files.get_mut(path).unwrap().conv = Some(key.clone());
                for line in text.lines() { self.claude_line(&key, Some(&sub_key), line); }
                self.live_files.insert(key, mtime);
            }
            Source::Codex => {
                let mut key = st_conv;
                for line in text.lines() { self.codex_line(path, &mut key, line); }
                if let Some(k) = key {
                    self.files.get_mut(path).unwrap().conv = Some(k.clone());
                    self.live_files.insert(k, mtime);
                }
            }
            Source::Sink => {
                for line in text.lines() { self.sink_line(path, line, mtime); }
            }
        }
    }

    // ----- persistence ---------------------------------------------------

    /// Load the saved index. A missing, unreadable or older-format file just
    /// means a full re-read.
    pub fn load_cache() -> Ingest {
        let parsed = File::open(cache_path()).ok()
            .and_then(|f| serde_json::from_reader::<_, CacheFile>(std::io::BufReader::new(f)).ok())
            .filter(|c| c.version == CACHE_VERSION);
        let Some(c) = parsed else { return Ingest::default() };
        let mut ing = Ingest::default();
        for conv in c.convs { ing.convs.insert(conv.key.clone(), conv); }
        ing.files = c.files.into_iter().collect();
        ing
    }

    /// Serialize the index if it changed since the last save. Quick enough to
    /// call under the brain lock; write the bytes with `write_cache` after.
    pub fn snapshot(&mut self) -> Option<Vec<u8>> {
        if !self.unsaved { return None; }
        self.unsaved = false;
        #[derive(serde::Serialize)]
        struct Out<'a> { version: u32, files: Vec<(&'a PathBuf, &'a FileState)>, convs: Vec<&'a Conv> }
        let out = Out { version: CACHE_VERSION, files: self.files.iter().collect(), convs: self.convs.values().collect() };
        serde_json::to_vec(&out).ok()
    }

    fn push_event(&mut self, ts: i64, conv: &str, kind: &'static str, name: &str, sub: Option<&str>) {
        if !self.emit { return; }
        self.seq += 1;
        self.events.push_back(Event {
            seq: self.seq, ts: if ts > 0 { ts } else { util::now_ms() },
            conv: conv.to_string(), kind, name: name.to_string(), sub: sub.map(|s| s.to_string()), detail: String::new(),
        });
        while self.events.len() > MAX_EVENTS { self.events.pop_front(); }
    }

    /// Attach a detail line to the event just pushed.
    fn detail_last(&mut self, d: &str) {
        if !self.emit { return; }
        if let Some(e) = self.events.back_mut() { e.detail = util::one_line(d, 120); }
    }

    fn attach_sub(&mut self, conv_key: &str, agent_id: &str, meta: (String, String)) -> String {
        let conv = self.convs.get_mut(conv_key).unwrap();
        if let Some(s) = conv.subs.iter().find(|s| s.agent_id.as_deref() == Some(agent_id)) {
            return s.key.clone();
        }
        let (agent_type, description) = meta;
        if let Some(s) = conv.subs.iter_mut().find(|s| s.agent_id.is_none() && !description.is_empty() && s.description == description) {
            s.agent_id = Some(agent_id.to_string());
            return s.key.clone();
        }
        let key = format!("a-{}", agent_id);
        conv.subs.push(Sub { key: key.clone(), agent_id: Some(agent_id.into()), agent_type, description, ..Default::default() });
        self.structural = true;
        key
    }

    // ----- Claude Code ---------------------------------------------------

    fn claude_line(&mut self, key: &str, sub: Option<&str>, line: &str) {
        let v: Value = match serde_json::from_str(line) { Ok(v) => v, Err(_) => return };
        let ts = v["timestamp"].as_str().and_then(util::parse_ts).unwrap_or(0);
        let kind = v["type"].as_str().unwrap_or("");
        {
            let conv = self.convs.get_mut(key).unwrap();
            conv.stamp(ts);
            if sub.is_none() {
                if conv.cwd.is_empty() { if let Some(c) = v["cwd"].as_str() { conv.cwd = c.to_string(); self.structural = true; } }
                if let Some(b) = v["gitBranch"].as_str() { if !b.is_empty() { conv.branch = b.to_string(); } }
            }
            if let Some(s) = sub {
                if let Some(sb) = conv.subs.iter_mut().find(|x| x.key == s) {
                    if ts > 0 { if sb.started == 0 { sb.started = ts; } sb.last = sb.last.max(ts); }
                }
            }
        }
        match kind {
            "custom-title" if sub.is_none() => {
                if let Some(t) = v["customTitle"].as_str() {
                    let conv = self.convs.get_mut(key).unwrap();
                    if conv.title != t { conv.title = t.to_string(); conv.custom_title = true; self.structural = true; }
                }
            }
            "summary" if sub.is_none() => {
                if let Some(t) = v["summary"].as_str() {
                    let conv = self.convs.get_mut(key).unwrap();
                    if !conv.custom_title && conv.title != t { conv.title = t.to_string(); self.structural = true; }
                }
            }
            "user" => self.claude_user(key, sub, &v, ts),
            "assistant" => self.claude_assistant(key, sub, &v, ts),
            _ => {}
        }
    }

    fn claude_user(&mut self, key: &str, sub: Option<&str>, v: &Value, ts: i64) {
        let content = &v["message"]["content"];
        if let Some(text) = content.as_str() {
            if sub.is_some() || v["isMeta"].as_bool() == Some(true) { return; }
            // Slash commands are skill invocations too.
            if let Some(cmd) = between(text, "<command-name>", "</command-name>") {
                let name = skills::bare_name(cmd);
                if !name.is_empty() && !BUILTIN_COMMANDS.contains(&name.as_str()) {
                    self.record_skill(key, None, &name, ts);
                }
                return;
            }
            if text.starts_with('<') { return; }
            let conv = self.convs.get_mut(key).unwrap();
            conv.prompts += 1;
            conv.log_prompt(ts, text);
            if conv.first_prompt.is_empty() {
                conv.first_prompt = util::one_line(text, 200);
                if conv.title.is_empty() { self.structural = true; }
            }
            self.push_event(ts, key, "prompt", "prompt", None);
            self.detail_last(text);
            return;
        }
        let Some(blocks) = content.as_array() else { return };
        for b in blocks {
            match b["type"].as_str() {
                Some("tool_result") => {
                    let id = b["tool_use_id"].as_str().unwrap_or("");
                    let text = block_text(&b["content"]);
                    let conv = self.convs.get_mut(key).unwrap();
                    if let Some(c) = conv.recent.iter_mut().rev().find(|c| c.id == id) {
                        c.result = util::one_line(&text, 240);
                    }
                    // Agent launches report the sub-agent id: bind it.
                    if let Some(aid) = v["toolUseResult"]["agentId"].as_str() {
                        if let Some(s) = conv.subs.iter_mut().find(|s| s.key == id) {
                            s.agent_id = Some(aid.to_string());
                        }
                    }
                }
                Some("text") if sub.is_none() && v["isMeta"].as_bool() != Some(true) => {
                    let t = b["text"].as_str().unwrap_or("");
                    if t.starts_with('<') { continue; }
                    let conv = self.convs.get_mut(key).unwrap();
                    conv.prompts += 1;
                    conv.log_prompt(ts, t);
                    if conv.first_prompt.is_empty() { conv.first_prompt = util::one_line(t, 200); self.structural = true; }
                    self.push_event(ts, key, "prompt", "prompt", None);
                    self.detail_last(t);
                }
                _ => {}
            }
        }
    }

    fn claude_assistant(&mut self, key: &str, sub: Option<&str>, v: &Value, ts: i64) {
        let msg = &v["message"];
        if sub.is_none() {
            if let Some(m) = msg["model"].as_str() {
                if !m.starts_with('<') { self.convs.get_mut(key).unwrap().model = m.to_string(); }
            }
        }
        let Some(blocks) = msg["content"].as_array() else { return };
        let mut counted_turn = false;
        for b in blocks {
            match b["type"].as_str() {
                Some("text") if sub.is_none() => {
                    let conv = self.convs.get_mut(key).unwrap();
                    if !counted_turn { conv.turns += 1; counted_turn = true; }
                    let t = b["text"].as_str().unwrap_or("").trim();
                    if !t.is_empty() { conv.last_reply = util::one_line(t, 600); }
                }
                Some("tool_use") => {
                    let name = b["name"].as_str().unwrap_or("tool").to_string();
                    let id = b["id"].as_str().unwrap_or("").to_string();
                    let input = &b["input"];
                    self.record_call(key, sub, &name, &id, input, ts);
                }
                _ => {}
            }
        }
    }

    fn record_call(&mut self, key: &str, sub: Option<&str>, name: &str, id: &str, input: &Value, ts: i64) {
        let args = summarize_args(name, input);
        let mut paths: Vec<String> = Vec::new();
        for k in ["file_path", "path", "notebook_path"] {
            if let Some(p) = input[k].as_str() { paths.push(util::norm_path(p)); }
        }
        let mut skill_hits: Vec<String> = Vec::new();
        if name == "Skill" {
            if let Some(s) = input["skill"].as_str() { skill_hits.push(skills::bare_name(s)); }
        }
        for p in &paths {
            if p.ends_with("/skill.md") {
                if let Some(dir) = p.trim_end_matches("/skill.md").rsplit('/').next() { skill_hits.push(dir.to_string()); }
            }
        }
        let is_edit = matches!(name, "Write" | "Edit" | "MultiEdit" | "NotebookEdit" | "apply_patch" | "write_file" | "edit_file" | "edit_block");
        {
            let conv = self.convs.get_mut(key).unwrap();
            *conv.tool_counts.entry(name.to_string()).or_insert(0) += 1;
            if is_edit {
                conv.edits += 1;
                for p in &paths {
                    if conv.changed.len() < MAX_TOUCHED || conv.changed.contains_key(p) { *conv.changed.entry(p.clone()).or_insert(0) += 1; }
                }
            }
            conv.recent.push_back(Call { ts, tool: name.into(), args: args.clone(), result: String::new(), id: id.into(), by: sub.map(|s| s.to_string()) });
            while conv.recent.len() > RECENT_CALLS { conv.recent.pop_front(); }
            if let Some(s) = sub {
                if let Some(sb) = conv.subs.iter_mut().find(|x| x.key == s) {
                    sb.calls += 1;
                    *sb.tool_counts.entry(name.to_string()).or_insert(0) += 1;
                    for p in &paths { if sb.touched.len() < MAX_TOUCHED { *sb.touched.entry(p.clone()).or_insert(0) += 1; } }
                }
            }
            for p in &paths {
                if conv.touched.len() < MAX_TOUCHED || conv.touched.contains_key(p) {
                    if !conv.touched.contains_key(p) { self.structural = true; }
                    *conv.touched.entry(p.clone()).or_insert(0) += 1;
                }
            }
            if (name == "Agent" || name == "Task") && sub.is_none() {
                conv.subs.push(Sub {
                    key: id.to_string(),
                    agent_type: input["subagent_type"].as_str().unwrap_or("general-purpose").to_string(),
                    description: util::one_line(input["description"].as_str().unwrap_or(""), 120),
                    started: ts, last: ts,
                    ..Default::default()
                });
                self.structural = true;
            }
        }
        self.push_event(ts, key, "tool", name, sub);
        self.detail_last(&args);
        if name == "Agent" || name == "Task" { self.push_event(ts, key, "subagent", &args, Some(id)); }
        for p in &paths { self.push_event(ts, key, "touch", p, sub); }
        for s in skill_hits { self.record_skill(key, sub, &s, ts); }
    }

    fn record_skill(&mut self, key: &str, sub: Option<&str>, name: &str, ts: i64) {
        let conv = self.convs.get_mut(key).unwrap();
        if !conv.skills.contains_key(name) { self.structural = true; }
        *conv.skills.entry(name.to_string()).or_insert(0) += 1;
        if let Some(s) = sub {
            if let Some(sb) = conv.subs.iter_mut().find(|x| x.key == s) { *sb.skills.entry(name.to_string()).or_insert(0) += 1; }
        }
        self.push_event(ts, key, "skill", name, sub);
    }

    // ----- Codex ---------------------------------------------------------

    fn codex_line(&mut self, path: &Path, key: &mut Option<String>, line: &str) {
        let v: Value = match serde_json::from_str(line) { Ok(v) => v, Err(_) => return };
        let ts = v["timestamp"].as_str().and_then(util::parse_ts).unwrap_or(0);
        let p = &v["payload"];
        if v["type"] == "session_meta" {
            let id = p["id"].as_str().or(p["session_id"].as_str()).unwrap_or("").to_string();
            if id.is_empty() { return; }
            let k = format!("codex:{}", id);
            if !self.convs.contains_key(&k) {
                let mut c = Conv::new(k.clone(), "codex", &id, path);
                c.cwd = p["cwd"].as_str().unwrap_or("").to_string();
                if let Some(parent) = p["parent_thread_id"].as_str() {
                    if parent != id { c.parent = Some(format!("codex:{}", parent)); }
                }
                self.convs.insert(k.clone(), c);
                self.structural = true;
            }
            *key = Some(k);
            return;
        }
        let Some(k) = key.clone() else { return };
        self.convs.get_mut(&k).unwrap().stamp(ts);
        match (v["type"].as_str(), p["type"].as_str()) {
            (Some("turn_context"), _) => {
                let c = self.convs.get_mut(&k).unwrap();
                if let Some(m) = p["model"].as_str() { c.model = m.to_string(); }
                if c.cwd.is_empty() { if let Some(cwd) = p["cwd"].as_str() { c.cwd = cwd.to_string(); self.structural = true; } }
            }
            (Some("event_msg"), Some("user_message")) => {
                let text = p["message"].as_str().unwrap_or("");
                let c = self.convs.get_mut(&k).unwrap();
                c.prompts += 1;
                c.log_prompt(ts, text);
                if c.first_prompt.is_empty() { c.first_prompt = util::one_line(text, 200); self.structural = true; }
                self.push_event(ts, &k, "prompt", "prompt", None);
                self.detail_last(text);
            }
            (Some("event_msg"), Some("agent_message")) => {
                let c = self.convs.get_mut(&k).unwrap();
                c.turns += 1;
                if let Some(t) = p["message"].as_str().filter(|t| !t.trim().is_empty()) { c.last_reply = util::one_line(t, 600); }
            }
            (Some("event_msg"), Some("thread_name_updated")) | (Some("event_msg"), Some("thread_renamed")) => {
                if let Some(t) = p["thread_name"].as_str().or(p["name"].as_str()) {
                    let c = self.convs.get_mut(&k).unwrap();
                    c.title = t.to_string(); c.custom_title = true; self.structural = true;
                }
            }
            (Some("response_item"), Some(t @ ("function_call" | "custom_tool_call" | "local_shell_call"))) => {
                let name = p["name"].as_str().unwrap_or(if t == "local_shell_call" { "shell" } else { "tool" }).to_string();
                let id = p["call_id"].as_str().unwrap_or("").to_string();
                let input: Value = match (&p["arguments"], &p["input"]) {
                    (Value::String(s), _) => serde_json::from_str(s).unwrap_or(Value::String(s.clone())),
                    (_, Value::String(s)) => Value::String(s.clone()),
                    (a, i) => if a.is_null() { i.clone() } else { a.clone() },
                };
                // apply_patch carries file paths in its body.
                if name == "apply_patch" {
                    if let Some(body) = input.as_str() {
                        for l in body.lines() {
                            for pre in ["*** Update File: ", "*** Add File: ", "*** Delete File: "] {
                                if let Some(f) = l.strip_prefix(pre) {
                                    let c = self.convs.get_mut(&k).unwrap();
                                    let np = util::norm_path(f.trim());
                                    if !c.touched.contains_key(&np) { self.structural = true; }
                                    *c.touched.entry(np.clone()).or_insert(0) += 1;
                                    if c.changed.len() < MAX_TOUCHED { *c.changed.entry(np).or_insert(0) += 1; }
                                }
                            }
                        }
                    }
                }
                self.record_call(&k, None, &name, &id, &input, ts);
            }
            (Some("response_item"), Some("function_call_output" | "custom_tool_call_output")) => {
                let id = p["call_id"].as_str().unwrap_or("");
                let text = block_text(&p["output"]);
                if let Some(c) = self.convs.get_mut(&k).unwrap().recent.iter_mut().rev().find(|c| c.id == id) {
                    c.result = util::one_line(&text, 240);
                }
            }
            _ => {}
        }
    }

    // ----- Generic sink --------------------------------------------------

    fn sink_line(&mut self, path: &Path, line: &str, _mtime: i64) {
        let v: Value = match serde_json::from_str(line) { Ok(v) => v, Err(_) => return };
        let sid = v["session_id"].as_str().unwrap_or("").to_string();
        if sid.is_empty() { return; }
        let harness = v["harness"].as_str().unwrap_or("agent").to_lowercase();
        let key = format!("{}:{}", harness, sid);
        let ts = v["ts"].as_str().and_then(util::parse_ts).or(v["ts"].as_i64()).unwrap_or_else(util::now_ms);
        if !self.convs.contains_key(&key) {
            self.convs.insert(key.clone(), Conv::new(key.clone(), &harness, &sid, path));
            self.structural = true;
        }
        {
            let c = self.convs.get_mut(&key).unwrap();
            c.stamp(ts);
            if let Some(cwd) = v["cwd"].as_str() { if c.cwd.is_empty() { c.cwd = cwd.into(); self.structural = true; } }
            if let Some(t) = v["title"].as_str() { if c.title != t { c.title = t.into(); c.custom_title = true; self.structural = true; } }
            if let Some(m) = v["model"].as_str() { c.model = m.into(); }
        }
        self.live_files.insert(key.clone(), ts);
        let name = v["name"].as_str().unwrap_or("").to_string();
        match v["kind"].as_str().unwrap_or("") {
            "prompt" => {
                let c = self.convs.get_mut(&key).unwrap();
                c.prompts += 1;
                c.log_prompt(ts, v["text"].as_str().unwrap_or(&name));
                if c.first_prompt.is_empty() { c.first_prompt = util::one_line(v["text"].as_str().unwrap_or(&name), 200); }
                self.push_event(ts, &key, "prompt", "prompt", None);
            }
            "tool" => {
                let mut input = v["args"].clone();
                if let Some(p) = v["path"].as_str() { input = serde_json::json!({ "file_path": p, "summary": v["args"] }); }
                self.record_call(&key, None, if name.is_empty() { "tool" } else { &name }, "", &input, ts);
                if let Some(r) = v["result"].as_str() {
                    if let Some(c) = self.convs.get_mut(&key).unwrap().recent.back_mut() { c.result = util::one_line(r, 240); }
                }
            }
            "skill" => self.record_skill(&key, None, &skills::bare_name(&name), ts),
            "subagent" => {
                let c = self.convs.get_mut(&key).unwrap();
                let sk = format!("s-{}", c.subs.len());
                c.subs.push(Sub { key: sk.clone(), agent_type: v["agent_type"].as_str().unwrap_or("agent").into(), description: util::one_line(&name, 120), started: ts, last: ts, ..Default::default() });
                self.structural = true;
                self.push_event(ts, &key, "subagent", &name, Some(&sk));
            }
            _ => {}
        }
    }
}

fn discover() -> Vec<(PathBuf, Source)> {
    let mut out = Vec::new();
    if let Some(root) = config::claude_projects_dir() {
        if let Ok(projects) = fs::read_dir(&root) {
            for proj in projects.flatten() {
                let pdir = proj.path();
                if !pdir.is_dir() { continue; }
                let Ok(items) = fs::read_dir(&pdir) else { continue };
                let mut subs = Vec::new();
                for it in items.flatten() {
                    let p = it.path();
                    if p.extension().map_or(false, |e| e == "jsonl") { out.push((p, Source::Claude)); }
                    else if p.is_dir() {
                        let sd = p.join("subagents");
                        if let Ok(files) = fs::read_dir(&sd) {
                            for f in files.flatten() {
                                let fp = f.path();
                                if fp.extension().map_or(false, |e| e == "jsonl") { subs.push((fp, Source::ClaudeSub)); }
                            }
                        }
                    }
                }
                out.extend(subs); // after parents, so the parent conversation exists
            }
        }
    }
    if let Some(root) = config::codex_sessions_dir() {
        for e in walkdir::WalkDir::new(root).max_depth(5).into_iter().filter_map(|e| e.ok()) {
            if e.file_type().is_file() && e.path().extension().map_or(false, |x| x == "jsonl") {
                out.push((e.path().to_path_buf(), Source::Codex));
            }
        }
    }
    if let Ok(files) = fs::read_dir(config::events_dir()) {
        for f in files.flatten() {
            let p = f.path();
            if p.extension().map_or(false, |e| e == "jsonl") { out.push((p, Source::Sink)); }
        }
    }
    out
}

/// Read up to CHUNK bytes from `offset`, extended until the buffer holds at
/// least one complete line (or the file ends).
fn read_chunk(f: &mut File, offset: u64) -> Option<Vec<u8>> {
    f.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = Vec::new();
    loop {
        let before = buf.len();
        f.by_ref().take(CHUNK as u64).read_to_end(&mut buf).ok()?;
        if buf.len() == before || buf[before..].contains(&b'\n') { break; }
    }
    if buf.is_empty() { None } else { Some(buf) }
}

pub fn cache_path() -> PathBuf {
    config::cache_dir().join("sessions-index.json")
}

/// Returns false if the index could not be written (the caller retries later).
pub fn write_cache(bytes: &[u8]) -> bool {
    let p = cache_path();
    if let Some(d) = p.parent() { let _ = fs::create_dir_all(d); }
    util::atomic_write(&p, bytes).is_ok()
}

fn read_sub_meta(jsonl: &Path) -> (String, String) {
    let meta = jsonl.with_extension("meta.json");
    let v: Value = fs::read_to_string(meta).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    (
        v["agentType"].as_str().unwrap_or("general-purpose").to_string(),
        util::one_line(v["description"].as_str().unwrap_or(""), 120),
    )
}

fn between<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> {
    let i = s.find(a)? + a.len();
    let j = s[i..].find(b)? + i;
    Some(&s[i..j])
}

fn block_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter()
            .filter_map(|i| i["text"].as_str().or(i.as_str()))
            .take(3).collect::<Vec<_>>().join(" "),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// One readable line describing what a tool call did.
fn summarize_args(name: &str, input: &Value) -> String {
    let pick = |keys: &[&str]| keys.iter().find_map(|k| input[*k].as_str().map(|s| s.to_string()));
    let s = match name {
        "Bash" | "shell" | "shell_command" | "exec_command" => pick(&["command", "cmd"]),
        "Read" | "Write" | "Edit" | "NotebookEdit" => pick(&["file_path", "notebook_path"]),
        "Grep" | "Glob" => pick(&["pattern"]).map(|p| match input["path"].as_str() { Some(d) => format!("{}  in {}", p, d), None => p }),
        "WebFetch" => pick(&["url"]),
        "WebSearch" => pick(&["query"]),
        "Skill" => pick(&["skill"]).map(|s| match input["args"].as_str() { Some(a) => format!("{} — {}", s, a), None => s }),
        "Agent" | "Task" => pick(&["description"]),
        _ => None,
    };
    let s = s.unwrap_or_else(|| match input {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        v => {
            if let Some(cmd) = v["command"].as_array() {
                cmd.iter().filter_map(|c| c.as_str()).collect::<Vec<_>>().join(" ")
            } else { v.to_string() }
        }
    });
    util::one_line(&s, 180)
}
