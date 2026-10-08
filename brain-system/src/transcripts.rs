//! Session ingest. Turns agent transcripts into conversations, tool calls,
//! sub-agents, skill uses and touched files — incrementally: each file is
//! read from the byte offset where the previous poll stopped, so a live
//! session costs only its newly appended lines.
//!
//! Sources:
//!   * Claude Code  `~/.claude/projects/<proj>/<session>.jsonl` (+ `<session>/subagents/agent-*.jsonl`)
//!   * Codex CLI    `~/.codex/sessions/**/rollout-*.jsonl`
//!   * Any agent    `<vault>/_system/brain-events/*.jsonl` (format in README.md)

use serde_json::Value;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::{config, skills, util};

const RECENT_CALLS: usize = 40;
const MAX_TOUCHED: usize = 400;
const MAX_EVENTS: usize = 600;

/// Claude Code CLI commands that look like `/name` but are not skills.
const BUILTIN_COMMANDS: &[&str] = &[
    "add-dir", "agents", "artifacts", "bashes", "bug", "chrome", "clear", "compact", "config", "context",
    "cost", "desktop", "doctor", "effort", "exit", "export", "fast", "feedback", "help", "hooks", "ide",
    "install-github-app", "login", "logout", "mcp", "memory", "migrate-installer", "mobile", "model",
    "output-style", "permissions", "plugin", "plugins", "pr-comments", "privacy-settings", "release-notes",
    "remote-control", "resume", "rewind", "sandbox", "status", "statusline", "tasks", "terminal-setup",
    "theme", "todos", "upgrade", "usage", "vim", "auto-mode-setup",
];

#[derive(Clone, Debug, serde::Serialize)]
pub struct Call {
    pub ts: i64,
    pub tool: String,
    pub args: String,
    pub result: String,
    #[serde(skip)]
    pub id: String,
    pub by: Option<String>, // sub-agent key when the call came from a sub-agent
}

#[derive(Clone, Debug, Default, serde::Serialize)]
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
    #[serde(skip)]
    pub touched: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, Default)]
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
}

impl Conv {
    fn new(key: String, harness: &str, session_id: &str, file: &Path) -> Self {
        Conv { key, harness: harness.into(), session_id: session_id.into(), file: file.to_path_buf(), ..Default::default() }
    }
    pub fn total_calls(&self) -> u32 { self.tool_counts.values().sum() }
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
}

#[derive(Clone, Copy, PartialEq)]
enum Source { Claude, ClaudeSub, Codex, Sink }

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
        let mut start = st.offset;
        if size < st.offset {
            // Truncated / rewritten: start over.
            start = 0;
            if matches!(source, Source::Claude | Source::Codex) {
                if let Some(k) = &st.conv { self.convs.remove(k); }
            }
        }
        st.size = size;
        st.mtime = mtime;
        let buf = match read_from(path, start) { Some(b) => b, None => return };
        let end = match buf.iter().rposition(|&b| b == b'\n') { Some(i) => i + 1, None => return };
        let st_offset = start + end as u64;
        let text = String::from_utf8_lossy(&buf[..end]).into_owned();
        let st_conv = st.conv.clone();
        let st_sub = st.sub_key.clone();
        st.offset = st_offset;

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

    fn push_event(&mut self, ts: i64, conv: &str, kind: &'static str, name: &str, sub: Option<&str>) {
        if !self.emit { return; }
        self.seq += 1;
        self.events.push_back(Event {
            seq: self.seq, ts: if ts > 0 { ts } else { util::now_ms() },
            conv: conv.to_string(), kind, name: name.to_string(), sub: sub.map(|s| s.to_string()),
        });
        while self.events.len() > MAX_EVENTS { self.events.pop_front(); }
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
            if conv.first_prompt.is_empty() {
                conv.first_prompt = util::one_line(text, 200);
                if conv.title.is_empty() { self.structural = true; }
            }
            self.push_event(ts, key, "prompt", "prompt", None);
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
                    if conv.first_prompt.is_empty() { conv.first_prompt = util::one_line(t, 200); self.structural = true; }
                    self.push_event(ts, key, "prompt", "prompt", None);
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
                Some("text") if !counted_turn && sub.is_none() => {
                    self.convs.get_mut(key).unwrap().turns += 1;
                    counted_turn = true;
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
        {
            let conv = self.convs.get_mut(key).unwrap();
            *conv.tool_counts.entry(name.to_string()).or_insert(0) += 1;
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
                if c.first_prompt.is_empty() { c.first_prompt = util::one_line(text, 200); self.structural = true; }
                self.push_event(ts, &k, "prompt", "prompt", None);
            }
            (Some("event_msg"), Some("agent_message")) => { self.convs.get_mut(&k).unwrap().turns += 1; }
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
                                    *c.touched.entry(np).or_insert(0) += 1;
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

fn read_from(path: &Path, offset: u64) -> Option<Vec<u8>> {
    let mut f = File::open(path).ok()?;
    f.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    Some(buf)
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
