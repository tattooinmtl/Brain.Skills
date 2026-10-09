//! The brain: one graph over everything the agents know and do.
//!
//!   brain ─ project ─┬─ conversation ─┬─ sub-agent
//!                    │                └─ vault notes / skills it edited
//!                    ├─ tools  (one node per project, every call of its sessions)
//!                    └─ skill  (one node per skill, shared by every project
//!                               that used it)
//!   brain ─ Memory Vault ─ folder ─ note / session  (+ [[wikilinks]], ghost targets)
//!   brain ─ Skills Library ─ category ─ skill ─ nested skill
//!
//! A background indexer keeps it current; the HTTP layer serves a cached JSON
//! snapshot plus a cheap activity feed for pulsing nodes and flowing paths.
//!
//! Every conversation gets a weight: how much happened in it (prompts, calls,
//! edits, sub-agents) scaled by how recent it is (half-life two weeks).
//! Projects add up their sessions. Agents read the weighted project and
//! session digests first and only open a transcript when those aren't enough.

use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::overlay::Overlay;
use crate::skills::{self, Skill};
use crate::transcripts::{self, Conv, Ingest};
use crate::vault::{self, Note, Resolver};
use crate::{config, util, verifier};

const LIVE_WINDOW_MS: i64 = 120_000;
/// A session's weight halves every this many days without activity.
const HALF_LIFE_DAYS: f32 = 14.0;

// Link kinds (index into the client's style table).
const L_CONTAINS: u8 = 0;
const L_WIKI: u8 = 1;
const L_USES: u8 = 2;
const L_TOUCH: u8 = 3;
const L_SPAWN: u8 = 4;
const L_TOOLS: u8 = 5;

#[derive(Clone)]
struct GNode {
    id: String,
    label: String,
    kind: &'static str,
    w: f32,
    ts: i64,
    extra: String,
    color: String,
    /// Recency 0..1 (1 = active now); negative = not time-based.
    r: f32,
}

#[derive(Default)]
struct Builder {
    nodes: Vec<GNode>,
    idx: HashMap<String, usize>,
    links: Vec<(u32, u32, u8)>,
    seen: HashSet<(u32, u32)>,
}

impl Builder {
    fn node(&mut self, id: &str, label: &str, kind: &'static str, w: f32, ts: i64) -> usize {
        if let Some(&i) = self.idx.get(id) {
            let n = &mut self.nodes[i];
            if w > n.w { n.w = w; }
            if ts > n.ts { n.ts = ts; }
            return i;
        }
        self.nodes.push(GNode { id: id.into(), label: label.into(), kind, w, ts, extra: String::new(), color: String::new(), r: -1.0 });
        self.idx.insert(id.into(), self.nodes.len() - 1);
        self.nodes.len() - 1
    }
    fn link(&mut self, a: usize, b: usize, kind: u8) {
        if a == b { return; }
        let key = if a < b { (a as u32, b as u32) } else { (b as u32, a as u32) };
        if self.seen.insert(key) { self.links.push((a as u32, b as u32, kind)); }
    }
}

/// Lookups the activity feed needs to turn events into node paths.
#[derive(Default)]
struct Lookup {
    conv_project: HashMap<String, String>, // conv key -> project node id
    skill_by_name: HashMap<String, String>, // bare name / folder name -> node id
    skill_by_rel: Vec<(String, String)>,    // (lowercase library rel, node id), longest first
    note_by_rel: HashMap<String, String>,   // lowercase vault rel -> node id
    node_ids: HashSet<String>,
    nodes: Vec<(String, String, &'static str)>, // (id, label, kind) in graph order
    index: HashMap<String, usize>,
    adj: Vec<Vec<u32>>,
}

pub struct Brain {
    pub ingest: Ingest,
    pub notes: Vec<Note>,
    note_cache: HashMap<String, Note>,
    pub skills: Vec<Skill>,
    pub overlay: Overlay,
    pub version: u64,
    pub graph: Arc<String>,
    pub ready: bool,
    pub last_sweep: Option<verifier::SweepReport>,
    lookup: Lookup,
    vault_norm: String,
    skills_norm: String,
    last_rebuild: Instant,
}

static BRAIN: OnceLock<RwLock<Brain>> = OnceLock::new();
static SWEEP_LOCK: Mutex<()> = Mutex::new(());

pub fn brain() -> &'static RwLock<Brain> {
    BRAIN.get_or_init(|| RwLock::new(Brain {
        ingest: Ingest::default(),
        notes: Vec::new(),
        note_cache: HashMap::new(),
        skills: Vec::new(),
        overlay: Overlay::load(),
        version: 0,
        graph: Arc::new(r#"{"version":0,"ready":false,"nodes":[],"links":[]}"#.to_string()),
        ready: false,
        last_sweep: None,
        lookup: Lookup::default(),
        vault_norm: canon_norm(&config::vault_dir()),
        skills_norm: canon_norm(&config::skills_dir()),
        last_rebuild: Instant::now(),
    }))
}

fn canon_norm(p: &Path) -> String {
    util::norm_path(&p.canonicalize().unwrap_or_else(|_| p.to_path_buf()).to_string_lossy())
}

pub fn read() -> std::sync::RwLockReadGuard<'static, Brain> {
    brain().read().unwrap_or_else(|e| e.into_inner())
}
pub fn write() -> std::sync::RwLockWriteGuard<'static, Brain> {
    brain().write().unwrap_or_else(|e| e.into_inner())
}

/// Start the background indexer. The first full transcript load happens
/// outside the lock so the UI and API stay responsive while it runs.
pub fn start_indexer() {
    thread::spawn(|| {
        // Saved index first: a restart only reads what was appended since.
        let mut ing = Ingest::load_cache();
        ing.poll(false);
        if let Some(bytes) = ing.snapshot() { transcripts::write_cache(&bytes); }
        {
            let mut b = write();
            b.ingest = ing;
            b.rescan_vault();
            b.rescan_skills();
            b.rebuild();
            b.ready = true;
            b.rebuild();
        }
        let mut tick: u64 = 0;
        loop {
            thread::sleep(Duration::from_millis(1500));
            tick += 1;
            let mut b = write();
            let mut structural = b.ingest.poll(true);
            if tick % 3 == 0 { structural |= b.rescan_vault(); }
            if tick % 40 == 0 { structural |= b.rescan_skills(); }
            let stale = b.ingest.dirty && b.last_rebuild.elapsed() > Duration::from_secs(12);
            if structural || stale { b.rebuild(); }
            let snap = if tick % 40 == 0 { b.ingest.snapshot() } else { None };
            drop(b);
            if let Some(bytes) = snap { transcripts::write_cache(&bytes); }
        }
    });

    // Verifier Librarian: first sweep shortly after start, then every 10 min.
    thread::spawn(|| {
        thread::sleep(Duration::from_secs(20));
        loop {
            run_sweep();
            thread::sleep(Duration::from_secs(600));
        }
    });
}

/// Held while resolving a proposal so it can't interleave with a sweep.
pub fn run_sweep_guard() -> std::sync::MutexGuard<'static, ()> {
    SWEEP_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Run one verification sweep (serialized) and remember its report.
pub fn run_sweep() -> verifier::SweepReport {
    // Lock order is always brain → sweep; never take the brain lock while
    // holding the sweep lock.
    let report = {
        let _g = run_sweep_guard();
        verifier::scan_and_verify()
    };
    write().last_sweep = Some(report.clone());
    report
}

impl Brain {
    /// Returns true if any note was added, removed or changed.
    pub fn rescan_vault(&mut self) -> bool {
        let notes = vault::scan(&config::vault_dir(), &self.note_cache);
        let changed = notes.len() != self.notes.len()
            || notes.iter().zip(self.notes.iter()).any(|(a, b)| a.rel != b.rel || a.mtime != b.mtime || a.size != b.size);
        if changed {
            self.note_cache = notes.iter().map(|n| (n.rel.clone(), n.clone())).collect();
            self.notes = notes;
        }
        changed
    }

    pub fn rescan_skills(&mut self) -> bool {
        let s = skills::scan(&config::skills_dir());
        let changed = s.len() != self.skills.len()
            || s.iter().zip(self.skills.iter()).any(|(a, b)| a.rel != b.rel || a.mtime != b.mtime || a.name != b.name);
        if changed { self.skills = s; }
        changed
    }

    pub fn rebuild(&mut self) {
        let (b, lookup) = self.build();
        self.lookup = lookup;
        self.version += 1;
        self.ingest.dirty = false;
        self.last_rebuild = Instant::now();
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for n in &b.nodes { *counts.entry(n.kind).or_insert(0) += 1; }
        let nodes: Vec<Value> = b.nodes.iter().map(|n| {
            let mut o = json!({ "id": n.id, "l": n.label, "k": n.kind, "w": (n.w * 100.0).round() / 100.0, "t": n.ts });
            if n.r >= 0.0 { o["r"] = json!((n.r * 100.0).round() / 100.0); }
            if !n.extra.is_empty() { o["x"] = json!(n.extra); }
            if !n.color.is_empty() { o["c"] = json!(n.color); }
            o
        }).collect();
        let links: Vec<[u32; 3]> = b.links.iter().map(|&(a, c, k)| [a, c, k as u32]).collect();
        let out = json!({
            "version": self.version,
            "ready": self.ready,
            "generated": util::now_ms(),
            "nodes": nodes,
            "links": links,
            "counts": counts,
        });
        self.graph = Arc::new(out.to_string());
    }

    fn build(&self) -> (Builder, Lookup) {
        let mut b = Builder::default();
        let mut lk = Lookup::default();
        let brain = b.node("brain", "Global Brain", "brain", 9.0, util::now_ms());
        let vault_hub = b.node("hub:vault", "Memory Vault", "hub", 4.0, 0);
        b.nodes[vault_hub].color = "#2fb8a8".into();
        let skills_hub = b.node("hub:skills", "Skills Library", "hub", 4.0, 0);
        b.nodes[skills_hub].color = "#e0b52e".into();
        b.link(brain, vault_hub, L_CONTAINS);
        b.link(brain, skills_hub, L_CONTAINS);

        // --- skills library ------------------------------------------------
        let mut skill_rel_to_idx: HashMap<String, usize> = HashMap::new();
        for s in &self.skills {
            let mut id = format!("skill:{}", s.name.to_lowercase());
            if b.idx.contains_key(&id) { id = format!("skill:{}", s.rel.to_lowercase()); }
            let i = b.node(&id, &s.name, "skill", 2.0, s.mtime);
            skill_rel_to_idx.insert(s.rel.clone(), i);
            lk.skill_by_name.entry(s.name.to_lowercase()).or_insert(id.clone());
            let dir = s.rel.rsplit('/').next().unwrap_or(&s.rel).to_lowercase();
            lk.skill_by_name.entry(dir).or_insert(id.clone());
            lk.skill_by_rel.push((s.rel.to_lowercase(), id));
        }
        lk.skill_by_rel.sort_by(|a, c| c.0.len().cmp(&a.0.len()));
        for s in &self.skills {
            let i = skill_rel_to_idx[&s.rel];
            if let Some(p) = s.parent.as_ref().and_then(|p| skill_rel_to_idx.get(p)) {
                b.link(*p, i, L_CONTAINS);
            } else if let Some(cat) = &s.category {
                let c = b.node(&format!("cat:{}", cat.to_lowercase()), cat, "category", 2.5, 0);
                b.nodes[c].color = "#c99a1e".into();
                b.link(skills_hub, c, L_CONTAINS);
                b.link(c, i, L_CONTAINS);
            } else {
                b.link(skills_hub, i, L_CONTAINS);
            }
        }

        // --- vault notes ---------------------------------------------------
        let resolver = Resolver::new(&self.notes);
        let mut note_node: Vec<usize> = Vec::with_capacity(self.notes.len());
        for n in &self.notes {
            let merged = if n.kind == "skillnote" {
                lk.skill_by_name.get(&n.stem.to_lowercase()).and_then(|id| b.idx.get(id).copied())
            } else { None };
            let i = match merged {
                Some(i) => i,
                None => {
                    let (kind, extra): (&'static str, &str) = match n.kind {
                        "session" => ("session", ""),
                        "skillnote" => ("skill", "vault"),
                        k => ("note", k),
                    };
                    let i = b.node(&format!("note:{}", n.rel), &n.title, kind, 1.2, n.mtime);
                    b.nodes[i].extra = extra.to_string();
                    let top = n.rel.split('/').next().unwrap_or("");
                    if n.rel.contains('/') && !top.is_empty() {
                        let f = b.node(&format!("folder:{}", top.to_lowercase()), top, "folder", 2.2, 0);
                        b.nodes[f].color = "#1f8f84".into();
                        b.link(vault_hub, f, L_CONTAINS);
                        b.link(f, i, L_CONTAINS);
                    } else {
                        b.link(vault_hub, i, L_CONTAINS);
                    }
                    i
                }
            };
            lk.note_by_rel.insert(n.rel.to_lowercase(), b.nodes[i].id.clone());
            note_node.push(i);
        }
        for (ni, n) in self.notes.iter().enumerate() {
            for t in &n.links {
                let src = note_node[ni];
                match resolver.resolve(t) {
                    Some(ti) => { let dst = note_node[ti]; b.link(src, dst, L_WIKI); }
                    None => {
                        let g = b.node(&format!("ghost:{}", t.to_lowercase()), t, "ghost", 0.8, 0);
                        b.link(src, g, L_WIKI);
                    }
                }
            }
        }

        // --- projects and their sessions -----------------------------------
        let now = util::now_ms();
        let mut convs: Vec<&Conv> = self.ingest.convs.values().collect();
        convs.sort_by(|a, c| a.key.cmp(&c.key));
        let ext_cat = "cat:plugins";
        // project node -> (summed session weight, best recency, total calls)
        let mut proj_agg: BTreeMap<usize, (f32, f32, u32)> = BTreeMap::new();
        for c in &convs {
            if c.parent.as_ref().map_or(false, |p| self.ingest.convs.contains_key(p)) { continue; }
            let (pid, plabel) = project_of(&c.cwd);
            let p = b.node(&pid, &plabel, "project", 3.0, c.last);
            b.link(brain, p, L_CONTAINS);
            lk.conv_project.insert(c.key.clone(), pid.clone());

            let (score, r) = conv_score(c, now);
            let total = c.total_calls();
            let agg = proj_agg.entry(p).or_insert((0.0, 0.0, 0));
            agg.0 += score; agg.1 = agg.1.max(r); agg.2 += total;

            let ci = b.node(&format!("conv:{}", c.key), &conv_label(c), "conversation", 1.5 + score * 0.6, c.last);
            b.nodes[ci].extra = c.harness.clone();
            b.nodes[ci].r = r;
            b.link(p, ci, L_CONTAINS);

            if total > 0 {
                let ti = b.node(&format!("tools:{}", pid), "Tools", "tools", 1.4, c.last);
                b.link(p, ti, L_TOOLS);
            }
            for s in &c.subs {
                let si = b.node(&format!("sub:{}:{}", c.key, s.key), &sub_label(&s.agent_type, &s.description), "subagent", 1.4 + (1.0 + s.calls as f32).log2() * 0.3, s.last);
                b.nodes[si].extra = s.agent_type.clone();
                b.nodes[si].r = r;
                b.link(ci, si, L_SPAWN);
                for name in s.skills.keys() {
                    let k = self.skill_node(&mut b, &mut lk, name, skills_hub, ext_cat);
                    b.link(si, k, L_USES);
                }
                for path in s.touched.keys() {
                    if let Some(t) = self.touched_node(&lk, path).and_then(|id| b.idx.get(&id).copied()) { b.link(si, t, L_TOUCH); }
                }
            }
            // Codex reviewer / child threads hang off their parent like sub-agents.
            for child in convs.iter().filter(|x| x.parent.as_deref() == Some(c.key.as_str())) {
                let si = b.node(&format!("sub:{}:{}", c.key, child.key), &sub_label("thread", &child.first_prompt), "subagent", 1.4 + (1.0 + child.total_calls() as f32).log2() * 0.3, child.last);
                b.nodes[si].extra = "thread".into();
                b.nodes[si].r = r;
                b.link(ci, si, L_SPAWN);
            }
            // Skills hang off the project: one edge per project, not per session.
            for (name, n) in &c.skills {
                let k = self.skill_node(&mut b, &mut lk, name, skills_hub, ext_cat);
                b.nodes[k].w += (*n as f32).min(20.0) * 0.08;
                if c.last > b.nodes[k].ts { b.nodes[k].ts = c.last; }
                b.link(p, k, L_USES);
            }
            for path in c.touched.keys() {
                if let Some(t) = self.touched_node(&lk, path).and_then(|id| b.idx.get(&id).copied()) {
                    b.link(ci, t, L_TOUCH);
                }
            }
        }
        for (&p, &(score, r, calls)) in &proj_agg {
            b.nodes[p].w = 3.0 + (1.0 + score).ln();
            b.nodes[p].r = r;
            let tid = format!("tools:{}", b.nodes[p].id);
            if let Some(&ti) = b.idx.get(&tid) {
                b.nodes[ti].label = format!("Tools · {}", calls);
                b.nodes[ti].w = 1.4 + (1.0 + calls as f32).log2() * 0.35;
                b.nodes[ti].r = r;
            }
        }

        // --- weights from degree + overlay edits -------------------------------
        let mut degree = vec![0u32; b.nodes.len()];
        for &(a, c, _) in &b.links { degree[a as usize] += 1; degree[c as usize] += 1; }
        for (i, n) in b.nodes.iter_mut().enumerate() {
            match n.kind {
                "note" | "session" | "ghost" => n.w += (1.0 + degree[i] as f32).log2() * 0.45,
                "skill" => n.w += (1.0 + degree[i] as f32).log2() * 0.25,
                _ => {}
            }
            if let Some(e) = self.overlay.nodes.get(&n.id) {
                if !e.label.is_empty() { n.label = e.label.clone(); }
                if !e.color.is_empty() { n.color = e.color.clone(); }
            }
        }
        lk.node_ids = b.idx.keys().cloned().collect();
        lk.nodes = b.nodes.iter().map(|n| (n.id.clone(), n.label.clone(), n.kind)).collect();
        lk.index = b.idx.clone();
        lk.adj = vec![Vec::new(); b.nodes.len()];
        for &(a, c, _) in &b.links { lk.adj[a as usize].push(c); lk.adj[c as usize].push(a); }
        (b, lk)
    }

    fn skill_node(&self, b: &mut Builder, lk: &mut Lookup, name: &str, skills_hub: usize, ext_cat: &str) -> usize {
        if let Some(id) = lk.skill_by_name.get(name) {
            if let Some(&i) = b.idx.get(id) { return i; }
        }
        let id = format!("skill:{}", name);
        let i = b.node(&id, name, "skill", 1.6, 0);
        b.nodes[i].extra = "external".into();
        let c = b.node(ext_cat, "Plugins & built-ins", "category", 2.5, 0);
        b.nodes[c].color = "#c99a1e".into();
        b.link(skills_hub, c, L_CONTAINS);
        b.link(c, i, L_CONTAINS);
        lk.skill_by_name.insert(name.to_string(), id);
        i
    }

    /// Map a normalized absolute path from a transcript to a vault note or skill.
    fn touched_node(&self, lk: &Lookup, path: &str) -> Option<String> {
        if let Some(rel) = path.strip_prefix(&format!("{}/", self.vault_norm)) {
            return lk.note_by_rel.get(rel).cloned();
        }
        if let Some(rel) = path.strip_prefix(&format!("{}/", self.skills_norm)) {
            return lk.skill_by_rel.iter()
                .find(|(r, _)| rel == r || rel.starts_with(&format!("{}/", r)))
                .map(|(_, id)| id.clone());
        }
        None
    }

    // ----- activity feed ---------------------------------------------------

    pub fn activity(&self, since: u64) -> Value {
        let now = util::now_ms();
        let live: Vec<String> = self.ingest.live_files.iter()
            .filter(|(_, &m)| now - m < LIVE_WINDOW_MS)
            .map(|(k, _)| format!("conv:{}", k))
            .filter(|id| self.lookup.node_ids.contains(id))
            .collect();
        // A fresh client (since=0) only needs the cursor, not old history.
        let events: Vec<Value> = if since == 0 { vec![] } else {
            self.ingest.events.iter().filter(|e| e.seq > since).filter_map(|e| {
                let conv = format!("conv:{}", e.conv);
                if !self.lookup.node_ids.contains(&conv) { return None; }
                let proj = self.lookup.conv_project.get(&e.conv).cloned().unwrap_or_default();
                let tools = format!("tools:{}", proj);
                let tools = if self.lookup.node_ids.contains(&tools) { Some(tools) } else { None };
                let sub = e.sub.as_ref().map(|s| format!("sub:{}:{}", e.conv, s)).filter(|s| self.lookup.node_ids.contains(s));
                let mut path: Vec<String> = match e.kind {
                    "prompt" => vec!["brain".into(), proj, conv],
                    "tool" => match &sub { Some(s) => vec![conv, s.clone()], None => [Some(conv), Some(proj), tools].into_iter().flatten().collect() },
                    "subagent" => [Some(conv), sub.clone()].into_iter().flatten().collect(),
                    "skill" => {
                        let target = self.lookup.skill_by_name.get(&e.name).cloned();
                        let mid = sub.clone().or(Some(proj));
                        [Some(conv), mid, target].into_iter().flatten().collect()
                    }
                    "touch" => match self.touched_node(&self.lookup, &e.name) {
                        Some(t) => [Some(conv), sub.clone(), Some(t)].into_iter().flatten().collect(),
                        None => return None,
                    },
                    _ => vec![conv],
                };
                path.retain(|p| !p.is_empty());
                Some(json!({ "seq": e.seq, "ts": e.ts, "kind": e.kind, "name": util::clip(&e.name, 80), "detail": e.detail, "path": path }))
            }).collect()
        };
        json!({
            "version": self.version,
            "ready": self.ready,
            "seq": self.ingest.seq,
            "live": live,
            "events": events,
        })
    }

    // ----- node details ----------------------------------------------------

    pub fn node_detail(&self, id: &str) -> Option<Value> {
        let edit = self.overlay.nodes.get(id).cloned().unwrap_or_default();
        let entries = self.overlay.entries.get(id).cloned().unwrap_or_default();
        let mut d = if id == "brain" {
            json!({
                "kind": "brain", "title": "Global Brain",
                "meta": {
                    "Vault": util::display_path(&config::vault_dir()),
                    "Skills library": util::display_path(&config::skills_dir()),
                    "Notes": self.notes.len(),
                    "Skills": self.skills.len(),
                    "Conversations": self.ingest.convs.len(),
                    "Graph version": self.version,
                },
            })
        } else if let Some(rel) = id.strip_prefix("note:") {
            let n = self.notes.iter().find(|n| n.rel == rel)?;
            self.note_json(n)
        } else if let Some(rest) = id.strip_prefix("skill:") {
            self.skill_json(id, rest)?
        } else if let Some(key) = id.strip_prefix("conv:") {
            self.conv_json(self.ingest.convs.get(key)?)
        } else if let Some(pid) = id.strip_prefix("tools:").filter(|k| k.starts_with("proj:")) {
            let convs = self.project_convs(pid);
            if convs.is_empty() { return None; }
            let mut counts: BTreeMap<String, u32> = BTreeMap::new();
            for c in &convs { for (k, v) in &c.tool_counts { *counts.entry(k.clone()).or_insert(0) += v; } }
            let mut calls: Vec<Value> = convs.iter().flat_map(|c| c.recent.iter().map(move |x| {
                let mut v = json!(x);
                v["conversation"] = json!(conv_label(c));
                v
            })).collect();
            calls.sort_by(|a, b| b["ts"].as_i64().cmp(&a["ts"].as_i64()));
            calls.truncate(60);
            json!({
                "kind": "tools", "title": format!("Tools · {}", self.project_label(pid)),
                "meta": { "Total calls": counts.values().sum::<u32>(), "Distinct tools": counts.len(), "Sessions": convs.len() },
                "tool_counts": sorted_counts(&counts),
                "calls": calls,
            })
        } else if let Some(key) = id.strip_prefix("tools:") {
            let c = self.ingest.convs.get(key)?;
            json!({
                "kind": "tools", "title": format!("Tools · {}", conv_label(c)),
                "meta": { "Total calls": c.total_calls(), "Distinct tools": c.tool_counts.len(), "Conversation": conv_label(c) },
                "tool_counts": sorted_counts(&c.tool_counts),
                "calls": c.recent.iter().rev().collect::<Vec<_>>(),
            })
        } else if let Some(rest) = id.strip_prefix("sub:") {
            self.sub_json(rest)?
        } else if id.starts_with("proj:") {
            self.project_json(id)?
        } else if let Some(t) = id.strip_prefix("ghost:") {
            let from: Vec<Value> = self.notes.iter()
                .filter(|n| n.links.iter().any(|l| l.to_lowercase() == t))
                .map(|n| json!({ "id": self.lookup.note_by_rel.get(&n.rel.to_lowercase()), "title": n.title })).collect();
            json!({ "kind": "ghost", "title": t, "meta": { "Status": "Unresolved link — no note with this name yet", "Linked from": from.len() }, "linked_from": from })
        } else if id.starts_with("hub:") || id.starts_with("cat:") || id.starts_with("folder:") {
            json!({ "kind": "group", "title": id.split(':').nth(1).unwrap_or(id) })
        } else {
            return None;
        };
        d["id"] = json!(id);
        d["edit"] = json!(edit);
        d["entries"] = json!(entries);
        if let Some(&i) = self.lookup.index.get(id) {
            let links: Vec<Value> = self.lookup.adj[i].iter().take(200).map(|&j| {
                let (nid, label, kind) = &self.lookup.nodes[j as usize];
                json!({ "id": nid, "label": label, "kind": kind })
            }).collect();
            d["links"] = json!(links);
            d["label"] = json!(self.lookup.nodes[i].1);
        }
        Some(d)
    }

    fn note_json(&self, n: &Note) -> Value {
        let content = std::fs::read_to_string(&n.path).unwrap_or_default();
        json!({
            "kind": n.kind, "title": n.title,
            "meta": { "Path": n.rel, "Folder": n.folder, "Updated": util::iso(n.mtime), "Links out": n.links.len() },
            "files": [{ "label": n.rel, "path": util::display_path(&n.path), "hash": util::sha256_hex(content.as_bytes()), "content": content }],
        })
    }

    fn skill_json(&self, id: &str, rest: &str) -> Option<Value> {
        let s = self.skills.iter().find(|s| s.name.to_lowercase() == rest || s.rel.to_lowercase() == rest);
        let bare = s.map(|s| s.name.to_lowercase()).unwrap_or_else(|| rest.to_string());
        let dir = s.map(|s| s.rel.rsplit('/').next().unwrap_or("").to_lowercase());
        let mut usage: Vec<Value> = self.ingest.convs.values().filter_map(|c| {
            let n: u32 = c.skills.iter()
                .filter(|(k, _)| **k == bare || Some(k.as_str()) == dir.as_deref())
                .map(|(_, v)| *v).sum();
            if n == 0 { return None; }
            Some(json!({ "id": format!("conv:{}", c.key), "title": conv_label(c), "harness": c.harness, "uses": n, "last": c.last }))
        }).collect();
        usage.sort_by(|a, b| b["last"].as_i64().cmp(&a["last"].as_i64()));
        let mut files = Vec::new();
        if let Some(s) = s {
            let content = std::fs::read_to_string(&s.skill_md).unwrap_or_default();
            files.push(json!({ "label": "SKILL.md", "path": s.skill_md, "hash": util::sha256_hex(content.as_bytes()), "content": content }));
        }
        if let Some(n) = self.notes.iter().find(|n| n.kind == "skillnote" && n.stem.to_lowercase() == bare) {
            let content = std::fs::read_to_string(&n.path).unwrap_or_default();
            files.push(json!({ "label": format!("Vault note · {}", n.rel), "path": util::display_path(&n.path), "hash": util::sha256_hex(content.as_bytes()), "content": content }));
        }
        if s.is_none() && files.is_empty() && usage.is_empty() && !self.lookup.node_ids.contains(id) { return None; }
        Some(json!({
            "kind": "skill",
            "title": s.map(|s| s.name.clone()).unwrap_or_else(|| rest.to_string()),
            "meta": {
                "Library path": s.map(|s| s.rel.clone()).unwrap_or_else(|| "(not in the library — plugin or built-in)".into()),
                "Category": s.and_then(|s| s.category.clone()).unwrap_or_default(),
                "Used in": format!("{} conversation(s)", usage.len()),
            },
            "description": s.map(|s| s.description.clone()).unwrap_or_default(),
            "usage": usage,
            "files": files,
        }))
    }

    fn project_convs(&self, pid: &str) -> Vec<&Conv> {
        self.lookup.conv_project.iter().filter(|(_, p)| p.as_str() == pid)
            .filter_map(|(k, _)| self.ingest.convs.get(k)).collect()
    }

    fn project_label(&self, pid: &str) -> String {
        self.lookup.index.get(pid).map(|&i| self.lookup.nodes[i].1.clone())
            .unwrap_or_else(|| pid.trim_start_matches("proj:").to_string())
    }

    /// The project digest: sessions ranked by weight, plus what the project
    /// as a whole used and changed. The first thing an agent should read.
    fn project_json(&self, pid: &str) -> Option<Value> {
        let now = util::now_ms();
        let mut convs: Vec<(&Conv, f32, f32)> = self.project_convs(pid).into_iter()
            .map(|c| { let (s, r) = conv_score(c, now); (c, s, r) }).collect();
        if convs.is_empty() && !self.lookup.node_ids.contains(pid) { return None; }
        convs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut tools: BTreeMap<String, u32> = BTreeMap::new();
        let mut skills: BTreeMap<String, u32> = BTreeMap::new();
        let mut changed: BTreeMap<String, u32> = BTreeMap::new();
        let (mut prompts, mut calls, mut edits, mut weight) = (0u32, 0u32, 0u32, 0f32);
        let (mut first, mut last) = (i64::MAX, 0i64);
        for (c, s, _) in &convs {
            for (k, v) in &c.tool_counts { *tools.entry(k.clone()).or_insert(0) += v; }
            for (k, v) in &c.skills { *skills.entry(k.clone()).or_insert(0) += v; }
            project_files(&c.changed, pid, &mut changed);
            prompts += c.prompts; calls += c.total_calls(); edits += c.edits; weight += s;
            if c.started > 0 { first = first.min(c.started); }
            last = last.max(c.last);
        }
        let mut files = sorted_counts(&changed);
        files.truncate(30);
        let sessions: Vec<Value> = convs.iter().map(|(c, s, r)| json!({
            "id": format!("conv:{}", c.key), "title": conv_label(c), "harness": c.harness,
            "last": util::iso(c.last), "weight": round1(*s), "recency": round2(*r),
            "prompts": c.prompts, "calls": c.total_calls(), "edits": c.edits, "files_changed": c.changed.len(),
            "live": self.ingest.live_files.get(&c.key).map_or(false, |m| now - m < LIVE_WINDOW_MS),
            "summary": util::clip(&c.first_prompt, 160),
            "outcome": util::clip(&c.last_reply, 200),
        })).collect();
        let path = convs.first().map(|(c, _, _)| project_root(&c.cwd)).unwrap_or_else(|| pid.trim_start_matches("proj:").to_string());
        Some(json!({
            "kind": "project", "title": self.project_label(pid),
            "meta": {
                "Path": path, "Sessions": convs.len(),
                "First seen": if first < i64::MAX { util::iso(first) } else { String::new() },
                "Last active": if last > 0 { util::iso(last) } else { String::new() },
                "Prompts": prompts, "Tool calls": calls, "Edits": edits, "Weight": round1(weight),
            },
            "conversations": sessions,
            "tool_counts": sorted_counts(&tools),
            "skills": sorted_counts(&skills),
            "files_changed": files,
        }))
    }

    /// Resolve a project from a node id, a path (cwd anywhere inside it,
    /// worktrees included) or a name.
    pub fn find_project(&self, q: &str) -> Option<String> {
        let q = q.trim();
        if q.is_empty() { return None; }
        let projects: Vec<(&String, &String)> = self.lookup.nodes.iter()
            .filter(|(_, _, k)| *k == "project").map(|(id, l, _)| (id, l)).collect();
        if q.starts_with("proj:") { return projects.iter().find(|(id, _)| id.as_str() == q).map(|(id, _)| id.to_string()); }
        let (as_path, _) = project_of(q);
        if let Some((id, _)) = projects.iter().find(|(id, _)| **id == as_path) { return Some(id.to_string()); }
        let norm = as_path.trim_start_matches("proj:").to_string();
        if norm.contains('/') {
            if let Some((id, _)) = projects.iter()
                .filter(|(id, _)| { let p = id.trim_start_matches("proj:"); norm.starts_with(&format!("{}/", p)) })
                .max_by_key(|(id, _)| id.len()) { return Some(id.to_string()); }
        }
        let lq = q.to_lowercase();
        projects.iter().find(|(_, l)| l.to_lowercase() == lq)
            .or_else(|| projects.iter().find(|(_, l)| l.to_lowercase().contains(&lq)))
            .map(|(id, _)| id.to_string())
    }

    fn conv_json(&self, c: &Conv) -> Value {
        let mut rel = BTreeMap::new();
        project_files(&c.changed, &project_of(&c.cwd).0, &mut rel);
        let mut files = sorted_counts(&rel);
        files.truncate(40);
        let subs: Vec<Value> = c.subs.iter().map(|s| json!({
            "id": format!("sub:{}:{}", c.key, s.key), "type": s.agent_type, "description": s.description, "calls": s.calls,
        })).collect();
        let touched: Vec<Value> = c.touched.iter().filter_map(|(p, n)| {
            self.touched_node(&self.lookup, p).map(|id| json!({ "id": id, "path": p, "count": n }))
        }).collect();
        json!({
            "kind": "conversation", "title": conv_label(c),
            "meta": {
                "Agent": c.harness, "Session id": c.session_id, "Project": c.cwd, "Model": c.model, "Branch": c.branch,
                "Started": if c.started > 0 { util::iso(c.started) } else { String::new() },
                "Last activity": if c.last > 0 { util::iso(c.last) } else { String::new() },
                "Prompts": c.prompts, "Assistant turns": c.turns, "Tool calls": c.total_calls(), "Edits": c.edits,
                "Weight": round1(conv_score(c, util::now_ms()).0),
                "Transcript": util::display_path(&c.file),
            },
            "first_prompt": c.first_prompt,
            "prompts_timeline": c.prompt_log.iter().map(|p| json!({ "ts": p.ts, "text": p.text })).collect::<Vec<_>>(),
            "prompts_skipped": c.prompts_skipped,
            "last_reply": c.last_reply,
            "files_changed": files,
            "tool_counts": sorted_counts(&c.tool_counts),
            "skills": sorted_counts(&c.skills),
            "subagents": subs,
            "touched": touched,
            "calls": c.recent.iter().rev().collect::<Vec<_>>(),
        })
    }

    fn sub_json(&self, rest: &str) -> Option<Value> {
        // rest = "<conv key>:<sub key>" where conv key itself contains one ':'
        let mut parts = rest.splitn(3, ':');
        let conv_key = format!("{}:{}", parts.next()?, parts.next()?);
        let sub_key = parts.next()?;
        let c = self.ingest.convs.get(&conv_key)?;
        if let Some(s) = c.subs.iter().find(|s| s.key == sub_key) {
            let calls: Vec<_> = c.recent.iter().rev().filter(|x| x.by.as_deref() == Some(sub_key)).collect();
            return Some(json!({
                "kind": "subagent", "title": sub_label(&s.agent_type, &s.description),
                "meta": { "Type": s.agent_type, "Parent": conv_label(c), "Tool calls": s.calls,
                          "Started": if s.started > 0 { util::iso(s.started) } else { String::new() } },
                "description": s.description,
                "tool_counts": sorted_counts(&s.tool_counts),
                "skills": sorted_counts(&s.skills),
                "calls": calls,
            }));
        }
        let child = self.ingest.convs.get(sub_key)?;
        let mut v = self.conv_json(child);
        v["kind"] = json!("subagent");
        Some(v)
    }

    // ----- edits -----------------------------------------------------------

    /// Overwrite a note/skill file. `base_hash` guards against clobbering a
    /// version the editor never saw.
    pub fn write_file(&mut self, path: &str, content: &str, base_hash: &str) -> Result<String, (u16, String)> {
        let roots = [config::vault_dir(), config::skills_dir()];
        let p = util::sandboxed(path, &roots).ok_or((403, "Path is outside the vault and skills library".to_string()))?;
        if p.extension().map_or(true, |e| e != "md") { return Err((403, "Only .md files can be edited".into())); }
        if p.components().any(|c| c.as_os_str() == "_system") { return Err((403, "Brain system files are not editable here".into())); }
        let current = std::fs::read(&p).map_err(|e| (404, e.to_string()))?;
        if !base_hash.is_empty() && util::sha256_hex(&current) != base_hash {
            return Err((409, "The file changed on disk since you opened it. Reload and re-apply your edit.".into()));
        }
        util::atomic_write(&p, content.as_bytes()).map_err(|e| (500, e.to_string()))?;
        {
            let _g = run_sweep_guard();
            verifier::mark_verified(&p, content);
        }
        self.rescan_vault();
        self.rescan_skills();
        self.rebuild();
        Ok(util::sha256_hex(content.as_bytes()))
    }

    pub fn has_node(&self, id: &str) -> bool { self.lookup.node_ids.contains(id) }

    /// Rank nodes by label match, then by text inside notes, skills and
    /// conversations. Used by the MCP tools and the API.
    pub fn search(&self, q: &str, kind: Option<&str>, limit: usize) -> Vec<Value> {
        let q = q.trim().to_lowercase();
        if q.is_empty() { return vec![]; }
        let terms: Vec<&str> = q.split_whitespace().collect();
        let mut hits: Vec<(i32, Value)> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let kind_ok = |k: &str| kind.map_or(true, |want| want.is_empty() || want == k);
        // 1. labels / ids
        for (id, label, k) in &self.lookup.nodes {
            if !kind_ok(k) { continue; }
            let l = label.to_lowercase();
            let score = if l == q { 100 } else if l.starts_with(&q) { 80 } else if l.contains(&q) { 60 }
                else if terms.iter().all(|t| l.contains(t) || id.to_lowercase().contains(t)) { 40 } else { continue };
            seen.insert(id.clone());
            hits.push((score, json!({ "id": id, "label": label, "kind": k, "match": "name" })));
        }
        // 2. note bodies
        for n in &self.notes {
            let Some(id) = self.lookup.note_by_rel.get(&n.rel.to_lowercase()) else { continue };
            if seen.contains(id) { continue; }
            let Some(&i) = self.lookup.index.get(id) else { continue };
            let k = self.lookup.nodes[i].2;
            if !kind_ok(k) { continue; }
            let Ok(text) = std::fs::read_to_string(&n.path) else { continue };
            let lower = text.to_lowercase();
            if !terms.iter().all(|t| lower.contains(t)) { continue; }
            let pos = lower.find(terms[0]).unwrap_or(0);
            seen.insert(id.clone());
            hits.push((20 + terms.len() as i32, json!({ "id": id, "label": self.lookup.nodes[i].1, "kind": k, "match": "content", "snippet": snippet(&text, pos) })));
        }
        // 3. skill descriptions
        for s in &self.skills {
            let id = format!("skill:{}", s.name.to_lowercase());
            if seen.contains(&id) || !kind_ok("skill") || !self.lookup.node_ids.contains(&id) { continue; }
            let d = s.description.to_lowercase();
            if terms.iter().all(|t| d.contains(t)) {
                seen.insert(id.clone());
                hits.push((18, json!({ "id": id, "label": s.name, "kind": "skill", "match": "description", "snippet": util::clip(&s.description, 200) })));
            }
        }
        // 4. conversations: first prompt and recent call arguments
        for c in self.ingest.convs.values() {
            let id = format!("conv:{}", c.key);
            if seen.contains(&id) || !kind_ok("conversation") || !self.lookup.node_ids.contains(&id) { continue; }
            let mut hay = c.first_prompt.to_lowercase();
            for p in &c.prompt_log { hay.push(' '); hay.push_str(&p.text.to_lowercase()); }
            for f in c.changed.keys() { hay.push(' '); hay.push_str(f); }
            for call in &c.recent { hay.push(' '); hay.push_str(&call.args.to_lowercase()); }
            if terms.iter().all(|t| hay.contains(t)) {
                seen.insert(id.clone());
                // Heavier (recent, busy) sessions first among activity matches.
                let w = conv_score(c, util::now_ms()).0;
                hits.push((10 + (w.round() as i32).min(9), json!({ "id": id, "label": conv_label(c), "kind": "conversation", "match": "activity", "snippet": util::clip(&c.first_prompt, 200) })));
            }
        }
        hits.sort_by(|a, b| b.0.cmp(&a.0));
        hits.into_iter().take(limit.max(1)).map(|(_, v)| v).collect()
    }

    /// Conversations, newest first — or heaviest first with `sort = "weight"`.
    pub fn recent(&self, limit: usize, q: Option<&str>, sort: Option<&str>) -> Vec<Value> {
        let q = q.unwrap_or("").to_lowercase();
        let now = util::now_ms();
        let mut convs: Vec<&Conv> = self.ingest.convs.values()
            .filter(|c| c.parent.is_none())
            .filter(|c| q.is_empty() || conv_label(c).to_lowercase().contains(&q) || c.cwd.to_lowercase().contains(&q) || c.first_prompt.to_lowercase().contains(&q))
            .collect();
        convs.sort_by(|a, b| b.last.cmp(&a.last));
        if matches!(sort, Some("weight") | Some("important")) {
            convs.sort_by(|a, b| conv_score(b, now).0.partial_cmp(&conv_score(a, now).0).unwrap_or(std::cmp::Ordering::Equal));
        }
        convs.into_iter().take(limit.max(1)).map(|c| {
            let mut skills: Vec<(&String, &u32)> = c.skills.iter().collect();
            skills.sort_by(|a, b| b.1.cmp(a.1));
            json!({
                "id": format!("conv:{}", c.key),
                "title": conv_label(c),
                "agent": c.harness,
                "project": c.cwd,
                "last": if c.last > 0 { util::iso(c.last) } else { String::new() },
                "live": self.ingest.live_files.get(&c.key).map_or(false, |m| now - m < LIVE_WINDOW_MS),
                "tool_calls": c.total_calls(),
                "prompts": c.prompts,
                "edits": c.edits,
                "weight": round1(conv_score(c, now).0),
                "summary": util::clip(&c.first_prompt, 140),
                "skills": skills.iter().take(5).map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
                "subagents": c.subs.len(),
            })
        }).collect()
    }

    /// Create a new markdown note (never overwrites). Agents may only write
    /// to the Inbox (Silver tier, reviewed later) or the session logs.
    pub fn create_note(&mut self, title: &str, content: &str, folder: &str) -> Result<(String, String), (u16, String)> {
        let folder = match folder {
            "" | "00-Inbox" | "inbox" => "00-Inbox",
            "30-Logs" | "logs" | "sessions" => "30-Logs",
            _ => return Err((400, "folder must be 00-Inbox or 30-Logs".into())),
        };
        let slug: String = title.trim().chars()
            .filter_map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { Some(c) } else if c.is_whitespace() { Some('-') } else { None })
            .collect::<String>().trim_matches('-').chars().take(80).collect();
        if slug.is_empty() { return Err((400, "title is empty".into())); }
        let vault = config::vault_dir();
        let path = vault.join(folder).join(format!("{}.md", slug));
        if path.exists() {
            return Err((409, format!("{}/{}.md already exists; add an entry to it instead", folder, slug)));
        }
        let trimmed = content.trim_start();
        let body = if trimmed.starts_with("---") || trimmed.starts_with('#') {
            content.to_string()
        } else {
            format!("# {}\n\n{}\n", title.trim(), content.trim_end())
        };
        util::atomic_write(&path, body.as_bytes()).map_err(|e| (500, e.to_string()))?;
        self.rescan_vault();
        self.rebuild();
        let rel = format!("{}/{}.md", folder, slug);
        Ok((format!("note:{}", rel), rel))
    }
}

fn snippet(text: &str, byte_pos: usize) -> String {
    let start = text[..byte_pos.min(text.len())].char_indices().rev().nth(80).map(|(i, _)| i).unwrap_or(0);
    let s: String = text[start..].chars().take(220).collect();
    util::one_line(&s, 220)
}

/// How recent: 1 now, 0.5 after HALF_LIFE_DAYS, toward 0 after that.
fn recency(last: i64, now: i64) -> f32 {
    if last <= 0 { return 0.0; }
    let days = (now - last).max(0) as f32 / 86_400_000.0;
    0.5f32.powf(days / HALF_LIFE_DAYS)
}

/// (weight, recency) of a session: log of how much happened in it, scaled
/// by recency. A big session from months ago keeps a quarter of its weight.
pub fn conv_score(c: &Conv, now: i64) -> (f32, f32) {
    let activity = 1.0 + c.prompts as f32 * 2.0 + c.turns as f32 * 0.5 + c.total_calls() as f32 * 0.3
        + c.edits as f32 * 1.5 + c.subs.len() as f32 * 3.0;
    let r = recency(c.last, now);
    (activity.ln() * (0.25 + 0.75 * r), r)
}

// Through f64 so JSON shows 9.3, not 9.300000190734863.
fn round1(x: f32) -> f64 { (x as f64 * 10.0).round() / 10.0 }
fn round2(x: f32) -> f64 { (x as f64 * 100.0).round() / 100.0 }

/// Changed files relative to their project, worktree folder stripped, so the
/// same file edited from several worktrees counts as one.
fn project_files(changed: &BTreeMap<String, u32>, pid: &str, into: &mut BTreeMap<String, u32>) {
    let root = format!("{}/", pid.trim_start_matches("proj:"));
    for (path, n) in changed {
        let mut rel = path.strip_prefix(&root).unwrap_or(path).to_string();
        for marker in [".claude/worktrees/", ".codex/worktrees/", "worktrees/"] {
            if let Some(rest) = rel.strip_prefix(marker) {
                rel = rest.split_once('/').map(|(_, r)| r.to_string()).unwrap_or_default();
                break;
            }
        }
        if !rel.is_empty() { *into.entry(rel).or_insert(0) += n; }
    }
}

/// The repo folder a cwd belongs to, original casing kept.
fn project_root(cwd: &str) -> String {
    let s = cwd.replace('\\', "/");
    let lower = s.to_lowercase();
    for marker in ["/.claude/worktrees/", "/.codex/worktrees/", "/worktrees/"] {
        if let Some(i) = lower.find(marker) { return s[..i].to_string(); }
    }
    s.trim_end_matches('/').to_string()
}

fn sorted_counts(m: &std::collections::BTreeMap<String, u32>) -> Vec<(String, u32)> {
    let mut v: Vec<(String, u32)> = m.iter().map(|(k, v)| (k.clone(), *v)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

pub fn conv_label(c: &Conv) -> String {
    if !c.title.is_empty() { return util::clip(&c.title, 70); }
    if !c.first_prompt.is_empty() { return util::clip(&c.first_prompt, 70); }
    format!("{} · {}", c.harness, c.session_id.chars().take(8).collect::<String>())
}

fn sub_label(kind: &str, desc: &str) -> String {
    if desc.is_empty() { kind.to_string() } else { util::clip(desc, 60) }
}

/// Group a working directory into a project. Worktrees fold into their repo.
fn project_of(cwd: &str) -> (String, String) {
    if cwd.is_empty() { return ("proj:(unknown)".into(), "(unknown project)".into()); }
    let mut n = util::norm_path(cwd);
    for marker in ["/.claude/worktrees/", "/.codex/worktrees/", "/worktrees/"] {
        if let Some(i) = n.find(marker) { n.truncate(i); break; }
    }
    let original = cwd.replace('\\', "/");
    let label = original.trim_end_matches('/').rsplit('/').find(|s| !s.is_empty() && !s.contains("worktree"))
        .map(|s| s.to_string()).unwrap_or_else(|| n.clone());
    let label = n.rsplit('/').next().filter(|s| !s.is_empty())
        .map(|s| original.split('/').find(|o| o.to_lowercase() == s).unwrap_or(s).to_string())
        .unwrap_or(label);
    (format!("proj:{}", n), label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_fold_worktrees() {
        let (a, la) = project_of(r"C:\.skills\.claude\worktrees\brain-x");
        let (b, _) = project_of(r"C:\.skills");
        assert_eq!(a, b);
        assert_eq!(la, ".skills");
        let (_, l) = project_of(r"C:\GameForgerAI-Editor");
        assert_eq!(l, "GameForgerAI-Editor");
        assert_eq!(project_root(r"C:\.skills\.claude\worktrees\brain-x"), "C:/.skills");
    }

    #[test]
    fn changed_files_merge_across_worktrees() {
        let mut changed = BTreeMap::new();
        changed.insert("c:/.skills/.claude/worktrees/a/src/x.rs".to_string(), 2);
        changed.insert("c:/.skills/src/x.rs".to_string(), 1);
        changed.insert("c:/other/y.rs".to_string(), 1);
        let mut out = BTreeMap::new();
        project_files(&changed, "proj:c:/.skills", &mut out);
        assert_eq!(out.get("src/x.rs"), Some(&3));
        assert_eq!(out.get("c:/other/y.rs"), Some(&1));
    }

    #[test]
    fn weight_decays_with_age() {
        let now = 1_000 * 86_400_000;
        let mut c = Conv::default();
        c.prompts = 10;
        c.last = now;
        let (fresh, r) = conv_score(&c, now);
        assert!((r - 1.0).abs() < 1e-6);
        c.last = now - 14 * 86_400_000;
        let (two_weeks, r) = conv_score(&c, now);
        assert!((r - 0.5).abs() < 1e-3);
        c.last = now - 365 * 86_400_000;
        let (old, _) = conv_score(&c, now);
        assert!(fresh > two_weeks && two_weeks > old && old > 0.0);
        assert!((old / fresh - 0.25).abs() < 0.01);
    }
}
