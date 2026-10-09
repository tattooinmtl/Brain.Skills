//! `brain-system.exe --mcp`: a Model Context Protocol server on stdio, so
//! Claude Code (or any MCP client) can read from and write to the brain.
//! It is a thin client of the running brain's HTTP API and starts the brain
//! (tray app) if it isn't running yet.

use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use crate::{config, util};

const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "The Global Brain indexes every past agent conversation (Claude Code, Codex, MiniMax…) \
by project: each session has a digest (its prompts, files it changed, its last reply), tools and skills used, \
sub-agents, plus the Obsidian memory vault and the skills library. Sessions are weighted by how much happened and how recent. \
Start with brain_project (defaults to your working folder) to see the project's weighted sessions; brain_node on a \
session id for its digest; brain_search for a topic across everything. Open a raw transcript only if a digest is not enough. \
When you finish meaningful work, record what you learned with brain_add_entry or brain_save_note.";

pub fn run() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() { continue; }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => { reply(&mut out, &json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": e.to_string() } })); continue; }
        };
        let Some(id) = msg.get("id").cloned().filter(|v| !v.is_null()) else { continue }; // notification
        let method = msg["method"].as_str().unwrap_or("");
        let result = match method {
            "initialize" => Ok(initialize(&msg["params"])),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => Ok(call(&msg["params"])),
            "resources/list" => Ok(json!({ "resources": [] })),
            "prompts/list" => Ok(json!({ "prompts": [] })),
            _ => Err(json!({ "code": -32601, "message": format!("Method not found: {}", method) })),
        };
        let resp = match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": e }),
        };
        reply(&mut out, &resp);
    }
}

fn reply(out: &mut std::io::Stdout, v: &Value) {
    let mut lock = out.lock();
    let _ = writeln!(lock, "{}", v);
    let _ = lock.flush();
}

fn initialize(params: &Value) -> Value {
    let asked = params["protocolVersion"].as_str().unwrap_or("");
    let version = if PROTOCOL_VERSIONS.contains(&asked) { asked } else { PROTOCOL_VERSIONS[0] };
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "global-brain", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

fn tools() -> Value {
    json!([
        {
            "name": "brain_search",
            "description": "Search the Global Brain: past conversations (any agent), vault notes and session logs, skills, projects, tools and sub-agents. Matches names first, then note text, skill descriptions and what conversations did. Returns node ids for brain_node.",
            "inputSchema": { "type": "object", "properties": {
                "query": { "type": "string", "description": "Words to look for, e.g. \"tray icon\" or \"gauntlet-loop\"" },
                "kind": { "type": "string", "enum": ["conversation", "skill", "note", "session", "project", "subagent", "tools", "ghost"], "description": "Only return this kind of node" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 12 }
            }, "required": ["query"] },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "brain_project",
            "description": "Start here. A project's digest: its sessions (all agents) ranked by weight (activity × recency) with one-line summaries and outcomes, plus the skills, tools and files the project used. Defaults to the current working folder. Much cheaper than reading transcripts.",
            "inputSchema": { "type": "object", "properties": {
                "project": { "type": "string", "description": "Folder path, project name or proj: id. Omit for the current working folder." },
                "limit": { "type": "integer", "minimum": 1, "maximum": 60, "default": 15, "description": "Sessions to list" }
            } },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "brain_node",
            "description": "Read one brain node by id (from brain_project, brain_search or brain_recent). For a conversation: its digest (prompt timeline, files changed, last reply), tools, skills, sub-agents and recent calls. For a skill: usage history. For notes/skills: file content. Plus connections and the entry log.",
            "inputSchema": { "type": "object", "properties": {
                "id": { "type": "string", "description": "Node id, e.g. conv:claude:<uuid>, skill:pdf, note:10-Entities/X.md" },
                "max_chars": { "type": "integer", "default": 12000, "description": "Truncate file content to this many characters" }
            }, "required": ["id"] },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "brain_recent",
            "description": "List agent conversations (all agents and projects), newest first or heaviest first, with project, weight, summary, tool calls, skills and whether they are live right now.",
            "inputSchema": { "type": "object", "properties": {
                "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 },
                "sort": { "type": "string", "enum": ["recent", "weight"], "default": "recent", "description": "weight = most important (activity × recency) first" },
                "filter": { "type": "string", "description": "Only conversations whose title, project path or first prompt contains this" }
            } },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "brain_status",
            "description": "Global Brain overview: counts of notes, skills and conversations, vault location, pending verification proposals.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true }
        },
        {
            "name": "brain_add_entry",
            "description": "Append a dated entry to any brain node's log — e.g. a lesson learned on a skill, a decision on a project, a follow-up on a conversation. Never edits files; entries are append-only.",
            "inputSchema": { "type": "object", "properties": {
                "id": { "type": "string", "description": "Node id to annotate" },
                "text": { "type": "string", "description": "What to record (markdown ok)" }
            }, "required": ["id", "text"] },
            "annotations": { "readOnlyHint": false, "destructiveHint": false }
        },
        {
            "name": "brain_save_note",
            "description": "Create a NEW markdown note in the memory vault (never overwrites). Goes to 00-Inbox (unverified, reviewed later) by default, or 30-Logs for a session log. Use [[Note Name]] wikilinks to connect it to existing notes.",
            "inputSchema": { "type": "object", "properties": {
                "title": { "type": "string" },
                "content": { "type": "string", "description": "Markdown body" },
                "folder": { "type": "string", "enum": ["00-Inbox", "30-Logs"], "default": "00-Inbox" }
            }, "required": ["title", "content"] },
            "annotations": { "readOnlyHint": false, "destructiveHint": false }
        }
    ])
}

fn call(params: &Value) -> Value {
    let name = params["name"].as_str().unwrap_or("");
    let args = &params["arguments"];
    let result = match name {
        "brain_search" => search(args),
        "brain_project" => project(args),
        "brain_node" => node(args),
        "brain_recent" => recent(args),
        "brain_status" => status(),
        "brain_add_entry" => add_entry(args),
        "brain_save_note" => save_note(args),
        _ => Err(format!("Unknown tool {}", name)),
    };
    match result {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
        Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
    }
}

// ---- tools ---------------------------------------------------------------

fn enc(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
        _ => format!("%{:02X}", b),
    }).collect()
}

fn search(a: &Value) -> Result<String, String> {
    let q = a["query"].as_str().unwrap_or("").trim();
    if q.is_empty() { return Err("query is required".into()); }
    let limit = a["limit"].as_u64().unwrap_or(12).clamp(1, 50);
    let mut path = format!("/api/brain/search?q={}&limit={}", enc(q), limit);
    if let Some(k) = a["kind"].as_str() { path.push_str(&format!("&kind={}", enc(k))); }
    let hits = api_get(&path)?;
    let list = hits.as_array().cloned().unwrap_or_default();
    if list.is_empty() { return Ok(format!("No brain nodes match \"{}\".", q)); }
    let mut out = format!("{} match(es) for \"{}\":\n", list.len(), q);
    for h in list {
        out.push_str(&format!("- [{}] {} — id: {}", h["kind"].as_str().unwrap_or(""), h["label"].as_str().unwrap_or(""), h["id"].as_str().unwrap_or("")));
        if let Some(s) = h["snippet"].as_str().filter(|s| !s.is_empty()) { out.push_str(&format!("\n    “{}”", s)); }
        out.push('\n');
    }
    out.push_str("\nUse brain_node with an id to read it.");
    Ok(out)
}

fn node(a: &Value) -> Result<String, String> {
    let id = a["id"].as_str().unwrap_or("").trim();
    if id.is_empty() { return Err("id is required".into()); }
    let max = a["max_chars"].as_u64().unwrap_or(12_000) as usize;
    let d = api_get(&format!("/api/brain/node?id={}", enc(id)))?;
    let mut o = String::new();
    let title = d["label"].as_str().or(d["title"].as_str()).unwrap_or(id);
    o.push_str(&format!("# {}\nkind: {}   id: {}\n", title, d["kind"].as_str().unwrap_or(""), id));
    if let Some(m) = d["meta"].as_object() {
        for (k, v) in m {
            let v = match v { Value::String(s) => s.clone(), other => other.to_string() };
            if !v.is_empty() { o.push_str(&format!("{}: {}\n", k, v)); }
        }
    }
    for key in ["description", "first_prompt"] {
        if let Some(s) = d[key].as_str().filter(|s| !s.is_empty()) { o.push_str(&format!("\n{}:\n{}\n", key.replace('_', " "), s)); }
    }
    if let Some(e) = d["edit"]["note"].as_str().filter(|s| !s.is_empty()) { o.push_str(&format!("\nuser note:\n{}\n", e)); }
    if let Some(ps) = d["prompts_timeline"].as_array().filter(|a| !a.is_empty()) {
        o.push_str("\nwhat the user asked (in order):\n");
        let skipped = d["prompts_skipped"].as_u64().unwrap_or(0);
        for (i, p) in ps.iter().enumerate() {
            if i == 4 && skipped > 0 { o.push_str(&format!("  … {} more prompts …\n", skipped)); }
            o.push_str(&format!("- {} {}\n", util::iso(p["ts"].as_i64().unwrap_or(0)).get(..16).unwrap_or(""), p["text"].as_str().unwrap_or("")));
        }
    }
    if let Some(r) = d["last_reply"].as_str().filter(|s| !s.is_empty()) { o.push_str(&format!("\nlast reply:\n{}\n", r)); }
    if let Some(f) = d["files_changed"].as_array().filter(|a| !a.is_empty()) {
        o.push_str(&format!("\nfiles changed ({}):\n", f.len()));
        for p in f.iter().take(30) { o.push_str(&format!("- {} ×{}\n", p[0].as_str().unwrap_or(""), p[1])); }
    }
    let pairs = |v: &Value| -> String {
        v.as_array().map(|a| a.iter().filter_map(|p| Some(format!("{} ×{}", p[0].as_str()?, p[1])))
            .collect::<Vec<_>>().join(", ")).unwrap_or_default()
    };
    let tc = pairs(&d["tool_counts"]);
    if !tc.is_empty() { o.push_str(&format!("\ntool calls: {}\n", tc)); }
    let sk = pairs(&d["skills"]);
    if !sk.is_empty() { o.push_str(&format!("skills used: {}\n", sk)); }
    if let Some(subs) = d["subagents"].as_array().filter(|a| !a.is_empty()) {
        o.push_str("\nsub-agents:\n");
        for s in subs { o.push_str(&format!("- {} ({}) — {} calls — id: {}\n", s["description"].as_str().unwrap_or(""), s["type"].as_str().unwrap_or(""), s["calls"], s["id"].as_str().unwrap_or(""))); }
    }
    if let Some(cs) = d["conversations"].as_array().filter(|a| !a.is_empty()) {
        o.push_str("\nsessions (heaviest first):\n");
        o.push_str(&session_lines(cs, 25));
    }
    if let Some(u) = d["usage"].as_array().filter(|a| !a.is_empty()) {
        o.push_str("\nused in:\n");
        for c in u.iter().take(30) { o.push_str(&format!("- {} ({}) ×{} — id: {}\n", c["title"].as_str().unwrap_or(""), c["harness"].as_str().unwrap_or(""), c["uses"], c["id"].as_str().unwrap_or(""))); }
    }
    if let Some(calls) = d["calls"].as_array().filter(|a| !a.is_empty()) {
        o.push_str("\nrecent calls (newest first):\n");
        for c in calls.iter().take(15) {
            o.push_str(&format!("- {}: {}", c["tool"].as_str().unwrap_or(""), c["args"].as_str().unwrap_or("")));
            if let Some(r) = c["result"].as_str().filter(|s| !s.is_empty()) { o.push_str(&format!("  → {}", util::clip(r, 160))); }
            o.push('\n');
        }
    }
    if let Some(links) = d["links"].as_array().filter(|a| !a.is_empty()) {
        o.push_str(&format!("\nconnections ({}):\n", links.len()));
        for l in links.iter().take(60) { o.push_str(&format!("- [{}] {} — id: {}\n", l["kind"].as_str().unwrap_or(""), l["label"].as_str().unwrap_or(""), l["id"].as_str().unwrap_or(""))); }
        if links.len() > 60 { o.push_str(&format!("- … {} more\n", links.len() - 60)); }
    }
    if let Some(entries) = d["entries"].as_array().filter(|a| !a.is_empty()) {
        o.push_str("\nentry log:\n");
        for e in entries.iter().rev().take(20) {
            o.push_str(&format!("- {} [{}] {}\n", util::iso(e["ts"].as_i64().unwrap_or(0)), e["source"].as_str().unwrap_or(""), e["text"].as_str().unwrap_or("")));
        }
    }
    if let Some(files) = d["files"].as_array() {
        for f in files {
            let content = f["content"].as_str().unwrap_or("");
            let shown: String = content.chars().take(max).collect();
            o.push_str(&format!("\n--- {} ({}) ---\n{}\n", f["label"].as_str().unwrap_or(""), f["path"].as_str().unwrap_or(""), shown));
            if content.chars().count() > max { o.push_str(&format!("… truncated ({} chars total)\n", content.chars().count())); }
        }
    }
    if d["kind"] == "conversation" {
        o.push_str("\n(The raw transcript is listed above under Transcript; open it only if this digest is not enough.)\n");
    }
    Ok(o)
}

fn session_lines(cs: &[Value], limit: usize) -> String {
    let mut o = String::new();
    for c in cs.iter().take(limit) {
        o.push_str(&format!(
            "- {}[w {}] {} — {} ({}) · {} prompts · {} calls · {} edits\n    id: {}\n",
            if c["live"].as_bool() == Some(true) { "● LIVE " } else { "" },
            c["weight"], c["last"].as_str().unwrap_or("").get(..10).unwrap_or(""),
            c["title"].as_str().unwrap_or(""), c["harness"].as_str().unwrap_or(""),
            c["prompts"], c["calls"], c["edits"], c["id"].as_str().unwrap_or(""),
        ));
        let summary = c["summary"].as_str().unwrap_or("");
        if !summary.is_empty() && summary != c["title"].as_str().unwrap_or("") { o.push_str(&format!("    asked: {}\n", summary)); }
        if let Some(out) = c["outcome"].as_str().filter(|s| !s.is_empty()) { o.push_str(&format!("    ended: {}\n", out)); }
    }
    if cs.len() > limit { o.push_str(&format!("- … {} older/lighter sessions (raise limit)\n", cs.len() - limit)); }
    o
}

fn project(a: &Value) -> Result<String, String> {
    let q = match a["project"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
        Some(p) => p.to_string(),
        None => std::env::current_dir().map(|p| p.to_string_lossy().to_string()).map_err(|e| e.to_string())?,
    };
    let limit = a["limit"].as_u64().unwrap_or(15).clamp(1, 60) as usize;
    let d = api_get(&format!("/api/brain/project?q={}", enc(&q)))
        .map_err(|e| format!("{} (looked for \"{}\")", e, q))?;
    let mut o = format!("# Project {}\nid: {}\n", d["title"].as_str().unwrap_or(""), d["id"].as_str().unwrap_or(""));
    if let Some(m) = d["meta"].as_object() {
        let line: Vec<String> = m.iter().filter_map(|(k, v)| {
            let v = match v { Value::String(s) => s.clone(), other => other.to_string() };
            if v.is_empty() { None } else { Some(format!("{}: {}", k, v)) }
        }).collect();
        o.push_str(&line.join(" · "));
        o.push('\n');
    }
    let pairs = |v: &Value, n: usize| -> String {
        v.as_array().map(|a| a.iter().take(n).filter_map(|p| Some(format!("{} ×{}", p[0].as_str()?, p[1])))
            .collect::<Vec<_>>().join(", ")).unwrap_or_default()
    };
    let sk = pairs(&d["skills"], 15);
    if !sk.is_empty() { o.push_str(&format!("\nskills: {}\n", sk)); }
    let tc = pairs(&d["tool_counts"], 12);
    if !tc.is_empty() { o.push_str(&format!("tools: {}\n", tc)); }
    if let Some(f) = d["files_changed"].as_array().filter(|a| !a.is_empty()) {
        o.push_str("\nmost-changed files:\n");
        for p in f.iter().take(15) { o.push_str(&format!("- {} ×{}\n", p[0].as_str().unwrap_or(""), p[1])); }
    }
    if let Some(cs) = d["conversations"].as_array().filter(|a| !a.is_empty()) {
        o.push_str(&format!("\nsessions ({}), heaviest first:\n", cs.len()));
        o.push_str(&session_lines(cs, limit));
    }
    if let Some(entries) = d["entries"].as_array().filter(|a| !a.is_empty()) {
        o.push_str("\nproject log:\n");
        for e in entries.iter().rev().take(10) {
            o.push_str(&format!("- {} {}\n", util::iso(e["ts"].as_i64().unwrap_or(0)).get(..10).unwrap_or(""), e["text"].as_str().unwrap_or("")));
        }
    }
    o.push_str("\nNext: brain_node <session id> for a session's digest. Read a transcript only if the digest is not enough.");
    Ok(o)
}

fn recent(a: &Value) -> Result<String, String> {
    let limit = a["limit"].as_u64().unwrap_or(10).clamp(1, 50);
    let mut path = format!("/api/brain/recent?limit={}", limit);
    if let Some(f) = a["filter"].as_str().filter(|s| !s.is_empty()) { path.push_str(&format!("&q={}", enc(f))); }
    if let Some(s) = a["sort"].as_str().filter(|s| !s.is_empty()) { path.push_str(&format!("&sort={}", enc(s))); }
    let list = api_get(&path)?.as_array().cloned().unwrap_or_default();
    if list.is_empty() { return Ok("No conversations found.".into()); }
    let mut o = String::from(if a["sort"] == "weight" { "Conversations (heaviest first):\n" } else { "Recent conversations (newest first):\n" });
    for c in list {
        let skills = c["skills"].as_array().map(|a| a.iter().filter_map(|s| s.as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
        o.push_str(&format!(
            "- {}[w {}] {} [{}] {} · {} calls{}{}\n    project: {} · id: {}\n",
            if c["live"].as_bool() == Some(true) { "● LIVE " } else { "" },
            c["weight"],
            c["title"].as_str().unwrap_or(""),
            c["agent"].as_str().unwrap_or(""),
            c["last"].as_str().unwrap_or(""),
            c["tool_calls"],
            if skills.is_empty() { String::new() } else { format!(" · skills: {}", skills) },
            if c["subagents"].as_u64().unwrap_or(0) > 0 { format!(" · {} sub-agents", c["subagents"]) } else { String::new() },
            c["project"].as_str().unwrap_or(""),
            c["id"].as_str().unwrap_or(""),
        ));
    }
    Ok(o)
}

fn status() -> Result<String, String> {
    let s = api_get("/api/status")?;
    let p = api_get("/api/proposals").ok();
    Ok(format!(
        "Global Brain v{} — {}\nnotes: {} · skills: {} · conversations: {} · graph version {}\nvault: {}\nskills library: {}\npending verification proposals: {}",
        api_get("/api/version").ok().and_then(|v| v["version"].as_str().map(|s| s.to_string())).unwrap_or_default(),
        if s["ready"].as_bool() == Some(true) { "indexed" } else { "indexing…" },
        s["notes"], s["skills"], s["conversations"], s["graph_version"],
        s["vault_path"].as_str().unwrap_or(""), s["skills_path"].as_str().unwrap_or(""),
        p.and_then(|v| v["pending"].as_array().map(|a| a.len())).unwrap_or(0),
    ))
}

fn add_entry(a: &Value) -> Result<String, String> {
    let id = a["id"].as_str().unwrap_or("");
    let text = a["text"].as_str().unwrap_or("");
    if id.is_empty() || text.trim().is_empty() { return Err("id and text are required".into()); }
    let r = api_post("/api/brain/entry", &json!({ "id": id, "text": text, "source": "claude" }))?;
    let n = r["entries"].as_array().map(|a| a.len()).unwrap_or(0);
    Ok(format!("Entry added to {} ({} entr{} in its log).", id, n, if n == 1 { "y" } else { "ies" }))
}

fn save_note(a: &Value) -> Result<String, String> {
    let r = api_post("/api/brain/note", &json!({
        "title": a["title"].as_str().unwrap_or(""),
        "content": a["content"].as_str().unwrap_or(""),
        "folder": a["folder"].as_str().unwrap_or("00-Inbox"),
    }))?;
    Ok(format!("Saved {} (node id {}). It appears in the brain within a couple of seconds.", r["path"].as_str().unwrap_or(""), r["id"].as_str().unwrap_or("")))
}

// ---- HTTP client to the running brain ------------------------------------------

fn api_get(path: &str) -> Result<Value, String> { request("GET", path, None) }
fn api_post(path: &str, body: &Value) -> Result<Value, String> { request("POST", path, Some(body)) }

fn request(method: &str, path: &str, body: Option<&Value>) -> Result<Value, String> {
    ensure_running()?;
    let (code, text) = http(method, path, body).map_err(|e| format!("Brain unreachable: {}", e))?;
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if code >= 400 {
        return Err(v["error"].as_str().map(|s| s.to_string()).unwrap_or_else(|| format!("HTTP {}", code)));
    }
    Ok(v)
}

fn http(method: &str, path: &str, body: Option<&Value>) -> std::io::Result<(u16, String)> {
    let port = config::port();
    let mut s = TcpStream::connect_timeout(&format!("127.0.0.1:{}", port).parse().unwrap(), Duration::from_secs(2))?;
    s.set_read_timeout(Some(Duration::from_secs(60)))?;
    let payload = body.map(|b| b.to_string()).unwrap_or_default();
    let req = format!(
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        method, path, port, payload.len(), payload
    );
    s.write_all(req.as_bytes())?;
    let mut raw = Vec::new();
    s.read_to_end(&mut raw)?;
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or_else(|| std::io::Error::other("bad response"))?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let mut body = raw[split + 4..].to_vec();
    let code = head.split_whitespace().nth(1).and_then(|c| c.parse().ok()).unwrap_or(500);
    if head.to_lowercase().contains("transfer-encoding: chunked") { body = dechunk(&body); }
    Ok((code, String::from_utf8_lossy(&body).to_string()))
}

fn dechunk(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let Some(nl) = b[i..].windows(2).position(|w| w == b"\r\n") else { break };
        let size = usize::from_str_radix(String::from_utf8_lossy(&b[i..i + nl]).trim(), 16).unwrap_or(0);
        if size == 0 { break; }
        let start = i + nl + 2;
        out.extend_from_slice(&b[start..(start + size).min(b.len())]);
        i = start + size + 2;
    }
    out
}

/// Start the brain tray app if nothing answers on the port, then wait until
/// its index is ready (first full load takes a few seconds).
fn ensure_running() -> Result<(), String> {
    static READY: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    if READY.get().is_some() { return Ok(()); }
    let ready = || http("GET", "/api/status", None).ok()
        .and_then(|(c, t)| if c == 200 { serde_json::from_str::<Value>(&t).ok() } else { None })
        .map(|v| v["ready"].as_bool() == Some(true));
    match ready() {
        Some(true) => { let _ = READY.set(()); return Ok(()); }
        Some(false) => {}
        None => {
            use std::os::windows::process::CommandExt;
            const DETACHED_PROCESS: u32 = 0x0000_0008;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            std::process::Command::new(exe)
                .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
                .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
                .spawn().map_err(|e| format!("Could not start the brain: {}", e))?;
        }
    }
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(45) {
        std::thread::sleep(Duration::from_millis(400));
        if ready() == Some(true) { let _ = READY.set(()); return Ok(()); }
    }
    Err("The brain did not become ready within 45 s (is port in use by something else?)".into())
}
