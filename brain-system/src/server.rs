//! Local HTTP API + UI on 127.0.0.1. Requests are served by a small worker
//! pool so a slow call (a verification sweep, a big graph) never stalls the
//! rest. Every request must carry our own Host header (DNS-rebinding guard),
//! and every POST must come from our own Origin.

use serde_json::{json, Value};
use std::io::Read;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Instant;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use crate::{brain, config, overlay, util, verifier};

const WORKERS: usize = 6;
const MAX_BODY: usize = 8 * 1024 * 1024;

const RELEASE_DATE: &str = env!("BRAIN_BUILD_DATE");
const COPYRIGHT_YEAR: &str = env!("BRAIN_BUILD_YEAR");
const AUTHOR_NAME: &str = "Erik Boivin";
const AUTHOR_EMAIL: &str = "erik.boivin@proton.me";
const PORTAL_URL: &str = "https://portal.globalwarningnetworks.com";

// ---- server status, read by the tray ----------------------------------------
pub const STATUS_STARTING: u8 = 0;
pub const STATUS_RUNNING: u8 = 1;
pub const STATUS_FAILED: u8 = 2;
pub static STATUS: AtomicU8 = AtomicU8::new(STATUS_STARTING);
pub static STATUS_ERROR: Mutex<String> = Mutex::new(String::new());
static STARTED: OnceLock<Instant> = OnceLock::new();

// ---- embedded UI ------------------------------------------------------------
const INDEX_HTML: &str = include_str!("web/index.html");
const APP_CSS: &str = include_str!("web/app.css");
const APP_JS: &str = include_str!("web/app.js");
const BRAIN_JS: &str = include_str!("web/brain.js");
const LAYOUT_JS: &str = include_str!("web/layout-worker.js");

const VENDOR: &[(&str, &str)] = &[
    ("/vendor/three.module.min.js", include_str!("web/vendor/three.module.min.js")),
    ("/vendor/jsm/controls/OrbitControls.js", include_str!("web/vendor/jsm/controls/OrbitControls.js")),
    ("/vendor/jsm/postprocessing/EffectComposer.js", include_str!("web/vendor/jsm/postprocessing/EffectComposer.js")),
    ("/vendor/jsm/postprocessing/RenderPass.js", include_str!("web/vendor/jsm/postprocessing/RenderPass.js")),
    ("/vendor/jsm/postprocessing/UnrealBloomPass.js", include_str!("web/vendor/jsm/postprocessing/UnrealBloomPass.js")),
    ("/vendor/jsm/postprocessing/OutputPass.js", include_str!("web/vendor/jsm/postprocessing/OutputPass.js")),
    ("/vendor/jsm/postprocessing/ShaderPass.js", include_str!("web/vendor/jsm/postprocessing/ShaderPass.js")),
    ("/vendor/jsm/postprocessing/MaskPass.js", include_str!("web/vendor/jsm/postprocessing/MaskPass.js")),
    ("/vendor/jsm/postprocessing/Pass.js", include_str!("web/vendor/jsm/postprocessing/Pass.js")),
    ("/vendor/jsm/shaders/CopyShader.js", include_str!("web/vendor/jsm/shaders/CopyShader.js")),
    ("/vendor/jsm/shaders/LuminosityHighPassShader.js", include_str!("web/vendor/jsm/shaders/LuminosityHighPassShader.js")),
    ("/vendor/jsm/shaders/OutputShader.js", include_str!("web/vendor/jsm/shaders/OutputShader.js")),
];

/// Addons import the bare specifier 'three'. Rewriting it here avoids an
/// inline import map, which the Content-Security-Policy would block.
fn vendor_file(path: &str) -> Option<&'static str> {
    static CACHE: OnceLock<Vec<(&'static str, String)>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| VENDOR.iter().map(|(p, src)| {
        let fixed = if p.contains("/jsm/") {
            src.replace("from 'three';", "from '/vendor/three.module.min.js';")
        } else { src.to_string() };
        (*p, fixed)
    }).collect());
    cache.iter().find(|(p, _)| *p == path).map(|(_, s)| s.as_str())
}

/// UI files are embedded; `BRAIN_WEB_DIR` serves them from disk instead so
/// the UI can be edited without rebuilding (development only).
fn web_asset(name: &str, embedded: &str) -> Vec<u8> {
    if let Ok(dir) = std::env::var("BRAIN_WEB_DIR") {
        if let Ok(bytes) = std::fs::read(std::path::Path::new(&dir).join(name)) { return bytes; }
    }
    embedded.as_bytes().to_vec()
}

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; worker-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'";

fn about_html() -> String {
    format!(r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<title>About Brain System</title>
<style>
  html, body {{ margin: 0; padding: 0; background: #0f1114; color: #e6e6e6; font-family: -apple-system, "Segoe UI", Roboto, Arial, sans-serif; }}
  .card {{ width: 360px; margin: 32px auto; padding: 24px; background: #171a1f; border: 1px solid #262a30; border-radius: 12px; text-align: center; box-shadow: 0 6px 24px rgba(0,0,0,.4); }}
  h1 {{ font-size: 18px; margin: 0 0 4px; letter-spacing: .5px; }}
  .ver {{ color: #7aa2f7; font-size: 13px; margin-bottom: 16px; }}
  img.logo {{ display: block; margin: 0 auto 14px; max-width: 220px; max-height: 220px; border-radius: 8px; }}
  .date {{ color: #9aa0a6; font-size: 12px; margin-bottom: 10px; }}
  .author, .portal {{ margin: 6px 0; font-size: 13px; }}
  a {{ color: #7aa2f7; text-decoration: none; }}
  a:hover {{ text-decoration: underline; }}
  .copy {{ margin-top: 18px; color: #6b7280; font-size: 11px; }}
</style>
</head>
<body>
  <div class="card">
    <h1>Brain System</h1>
    <div class="ver">Version {version}</div>
    <img class="logo" src="/logo.jpg" alt="Brain System logo" />
    <div class="date">Built {release_date}</div>
    <div class="author">{name} · {email}</div>
    <div class="portal"><a href="{portal}" target="_blank" rel="noopener">GWN-Portal</a></div>
    <div class="copy">&copy; {year} {name}. All rights reserved.</div>
  </div>
</body>
</html>"#,
        version = env!("CARGO_PKG_VERSION"),
        release_date = RELEASE_DATE,
        name = AUTHOR_NAME,
        email = AUTHOR_EMAIL,
        portal = PORTAL_URL,
        year = COPYRIGHT_YEAR,
    )
}

fn header(k: &str, v: &str) -> Header {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).expect("static header")
}

fn send(rq: Request, status: u16, ctype: &str, body: Vec<u8>, cache: bool) {
    let mut resp = Response::from_data(body)
        .with_status_code(StatusCode(status))
        .with_header(header("Content-Type", ctype))
        .with_header(header("X-Content-Type-Options", "nosniff"))
        .with_header(header("Referrer-Policy", "no-referrer"));
    if cache {
        resp = resp.with_header(header("Cache-Control", "public, max-age=86400"));
    } else {
        resp = resp.with_header(header("Cache-Control", "no-cache, no-store, must-revalidate"));
    }
    if ctype.starts_with("text/html") {
        resp = resp.with_header(header("Content-Security-Policy", CSP))
            .with_header(header("X-Frame-Options", "DENY"));
    }
    let _ = rq.respond(resp);
}

fn send_json(rq: Request, status: u16, v: &Value) {
    send(rq, status, "application/json; charset=utf-8", v.to_string().into_bytes(), false);
}

fn send_err(rq: Request, status: u16, msg: &str) {
    send_json(rq, status, &json!({ "ok": false, "error": msg }));
}

fn host_ok(rq: &Request, port: u16) -> bool {
    let allowed = config::allowed_hosts(port);
    match rq.headers().iter().find(|h| h.field.equiv("Host")) {
        None => true, // HTTP/1.0 native clients; browsers always send Host
        Some(h) => allowed.iter().any(|a| h.value.as_str().eq_ignore_ascii_case(a)),
    }
}

/// POSTs must come from our own page (or a native client with no Origin).
fn origin_ok(rq: &Request, port: u16) -> bool {
    match rq.headers().iter().find(|h| h.field.equiv("Origin")) {
        None => true,
        Some(h) => {
            let o = h.value.as_str();
            o == config::allowed_origin(port) || o == format!("http://localhost:{}", port)
        }
    }
}

fn read_json(rq: &mut Request) -> Result<Value, String> {
    let mut body = Vec::new();
    rq.as_reader().take(MAX_BODY as u64 + 1).read_to_end(&mut body).map_err(|e| e.to_string())?;
    if body.len() > MAX_BODY { return Err("Request body too large".into()); }
    serde_json::from_slice(&body).map_err(|e| format!("Invalid JSON: {}", e))
}

pub fn run_server(port: u16) {
    STARTED.get_or_init(Instant::now);
    let addr = format!("127.0.0.1:{}", port);
    let server = match Server::http(&addr) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            *STATUS_ERROR.lock().unwrap_or_else(|e| e.into_inner()) = format!("Port {} unavailable: {}", port, e);
            STATUS.store(STATUS_FAILED, Ordering::SeqCst);
            return;
        }
    };
    STATUS.store(STATUS_RUNNING, Ordering::SeqCst);

    let mut handles = Vec::new();
    for _ in 0..WORKERS {
        let s = Arc::clone(&server);
        handles.push(thread::spawn(move || loop {
            match s.recv() {
                Ok(rq) => handle(rq, port),
                Err(_) => break,
            }
        }));
    }
    for h in handles { let _ = h.join(); }
}

fn handle(mut rq: Request, port: u16) {
    if !host_ok(&rq, port) {
        return send_err(rq, 421, "Unexpected Host header");
    }
    let (path, params) = util::split_url(rq.url());
    let is_post = *rq.method() == Method::Post;
    if is_post && !origin_ok(&rq, port) {
        return send_err(rq, 403, "Forbidden origin");
    }

    match (rq.method().clone(), path.as_str()) {
        // ---- UI ----------------------------------------------------------
        (Method::Get, "/" | "/index.html" | "/brain" | "/wiki" | "/librarian" | "/skills") => {
            send(rq, 200, "text/html; charset=utf-8", web_asset("index.html", INDEX_HTML), false)
        }
        (Method::Get, "/app.css") => send(rq, 200, "text/css; charset=utf-8", web_asset("app.css", APP_CSS), false),
        (Method::Get, "/app.js") => send(rq, 200, "text/javascript; charset=utf-8", web_asset("app.js", APP_JS), false),
        (Method::Get, "/brain.js") => send(rq, 200, "text/javascript; charset=utf-8", web_asset("brain.js", BRAIN_JS), false),
        (Method::Get, "/layout-worker.js") => send(rq, 200, "text/javascript; charset=utf-8", web_asset("layout-worker.js", LAYOUT_JS), false),
        (Method::Get, p) if p.starts_with("/vendor/") => match vendor_file(p) {
            Some(src) => send(rq, 200, "text/javascript; charset=utf-8", src.as_bytes().to_vec(), true),
            None => send_err(rq, 404, "Not found"),
        },
        (Method::Get, "/about") => send(rq, 200, "text/html; charset=utf-8", about_html().into_bytes(), false),
        (Method::Get, "/logo.jpg") => match std::fs::read(config::logo_path()) {
            Ok(bytes) => send(rq, 200, "image/jpeg", bytes, true),
            Err(_) => send_err(rq, 404, "logo not found"),
        },
        (Method::Get, "/favicon.ico") => match std::fs::read(config::icon_path()) {
            Ok(bytes) => send(rq, 200, "image/x-icon", bytes, true),
            Err(_) => send_err(rq, 404, "icon not found"),
        },

        // ---- info --------------------------------------------------------
        (Method::Get, "/api/version") => send_json(rq, 200, &json!({
            "version": env!("CARGO_PKG_VERSION"),
            "release_date": RELEASE_DATE,
            "author": AUTHOR_NAME,
            "email": AUTHOR_EMAIL,
            "portal": PORTAL_URL,
        })),
        (Method::Get, "/api/status") => {
            let b = brain::read();
            let v = json!({
                "status": "active",
                "ready": b.ready,
                "uptime_s": STARTED.get().map(|s| s.elapsed().as_secs()).unwrap_or(0),
                "vault_path": util::display_path(&config::vault_dir()),
                "skills_path": util::display_path(&config::skills_dir()),
                "port": port,
                "notes": b.notes.len(),
                "skills": b.skills.len(),
                "conversations": b.ingest.convs.len(),
                "graph_version": b.version,
                "last_sweep": b.last_sweep.as_ref().map(|s| json!({ "ran_at": s.ran_at, "checked": s.checked, "pending": s.pending.len() })),
                "update": crate::updater::as_json(),
            });
            drop(b);
            send_json(rq, 200, &v)
        }

        // ---- wiki --------------------------------------------------------
        (Method::Get, "/api/notes") => {
            let b = brain::read();
            let notes: Vec<Value> = b.notes.iter().map(|n| json!({
                "id": format!("note:{}", n.rel),
                "title": n.title,
                "type": n.kind,
                "rel": n.rel,
                "path": util::display_path(&n.path),
                "updated": util::iso(n.mtime),
                "summary": n.summary,
            })).collect();
            drop(b);
            send_json(rq, 200, &Value::Array(notes))
        }
        (Method::Get, "/api/note") => {
            let raw = util::param(&params, "path").unwrap_or("");
            let roots = [config::vault_dir(), config::skills_dir()];
            match util::sandboxed(raw, &roots).filter(|p| p.is_file()) {
                None => send_err(rq, 403, "Forbidden: path not under vault or skills library"),
                Some(p) => match std::fs::read_to_string(&p) {
                    Err(_) => send_err(rq, 404, "Read failed"),
                    Ok(content) => {
                        let vault = config::vault_dir().canonicalize().unwrap_or_else(|_| config::vault_dir());
                        let rel = p.strip_prefix(&vault).ok().map(|r| r.to_string_lossy().replace('\\', "/"));
                        let kind = match &rel { Some(r) => crate::vault::classify(r), None => "skill" };
                        let title = crate::vault::title_and_summary(&content, &p.file_stem().unwrap_or_default().to_string_lossy()).0;
                        let id = match &rel { Some(r) => format!("note:{}", r), None => String::new() };
                        send_json(rq, 200, &json!({
                            "id": id,
                            "title": title,
                            "path": util::display_path(&p),
                            "type": kind,
                            "updated": util::iso(util::mtime_ms(&p)),
                            "hash": util::sha256_hex(content.as_bytes()),
                            "content": content,
                        }))
                    }
                },
            }
        }
        (Method::Get, "/api/graph") => {
            // Legacy flat vault graph (kept for external tools).
            let b = brain::read();
            let resolver = crate::vault::Resolver::new(&b.notes);
            let nodes: Vec<Value> = b.notes.iter().map(|n| json!({ "id": n.stem, "title": n.title, "type": n.kind, "path": util::display_path(&n.path), "summary": n.summary })).collect();
            let mut links = Vec::new();
            for n in &b.notes {
                for t in &n.links {
                    if let Some(i) = resolver.resolve(t) { links.push(json!({ "source": n.stem, "target": b.notes[i].stem })); }
                }
            }
            drop(b);
            send_json(rq, 200, &json!({ "nodes": nodes, "links": links }))
        }
        (Method::Get, "/api/skills") => {
            let b = brain::read();
            let mut uses: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
            for c in b.ingest.convs.values() {
                for (k, v) in &c.skills { *uses.entry(k.as_str()).or_insert(0) += v; }
            }
            let list: Vec<Value> = b.skills.iter().map(|s| {
                let dir = s.rel.rsplit('/').next().unwrap_or("").to_lowercase();
                let name = s.name.to_lowercase();
                let n = uses.get(name.as_str()).copied().unwrap_or(0) + if dir != name { uses.get(dir.as_str()).copied().unwrap_or(0) } else { 0 };
                json!({
                    "name": s.name, "rel": s.rel, "category": s.category, "parent": s.parent,
                    "description": s.description, "path": s.skill_md,
                    "node_id": format!("skill:{}", name), "uses": n,
                })
            }).collect();
            drop(b);
            send_json(rq, 200, &Value::Array(list))
        }

        // ---- librarian ---------------------------------------------------
        (Method::Get, "/api/proposals") => {
            let last = brain::read().last_sweep.clone();
            send_json(rq, 200, &json!({
                "pending": verifier::pending(),
                "last_sweep": last.map(|s| json!({ "ran_at": s.ran_at, "checked": s.checked, "baselined": s.baselined, "new_drift": s.new_drift, "new_links": s.new_links })),
            }))
        }
        (Method::Post, "/api/verify") => {
            let r = brain::run_sweep();
            send_json(rq, 200, &json!({ "ok": true, "report": r }))
        }
        (Method::Post, "/api/proposals/resolve") => {
            let body = match read_json(&mut rq) { Ok(v) => v, Err(e) => return send_err(rq, 400, &e) };
            let id = body["proposal_id"].as_str().unwrap_or("");
            let Some(action) = body["action"].as_str() else { return send_err(rq, 400, "Missing \"action\" (approve or reject)") };
            let result = {
                let _g = brain::run_sweep_guard();
                verifier::resolve_proposal(id, action)
            };
            match result {
                Ok(msg) => send_json(rq, 200, &json!({ "ok": true, "message": msg })),
                Err((code, msg)) => send_err(rq, code, &msg),
            }
        }

        // ---- brain -------------------------------------------------------
        (Method::Get, "/api/brain/graph") => {
            let g = brain::read().graph.clone();
            send(rq, 200, "application/json; charset=utf-8", g.as_bytes().to_vec(), false)
        }
        (Method::Get, "/api/brain/activity") => {
            let since = util::param(&params, "since").and_then(|s| s.parse().ok()).unwrap_or(0);
            let v = brain::read().activity(since);
            send_json(rq, 200, &v)
        }
        (Method::Get, "/api/brain/node") => {
            let id = util::param(&params, "id").unwrap_or("");
            let d = brain::read().node_detail(id);
            match d {
                Some(v) => send_json(rq, 200, &v),
                None => send_err(rq, 404, "No such node"),
            }
        }
        (Method::Post, "/api/brain/file") => {
            let body = match read_json(&mut rq) { Ok(v) => v, Err(e) => return send_err(rq, 400, &e) };
            let (Some(path), Some(content)) = (body["path"].as_str(), body["content"].as_str()) else {
                return send_err(rq, 400, "Expected { path, content, hash }");
            };
            let hash = body["hash"].as_str().unwrap_or("");
            let r = brain::write().write_file(path, content, hash);
            match r {
                Ok(h) => send_json(rq, 200, &json!({ "ok": true, "hash": h })),
                Err((code, msg)) => send_err(rq, code, &msg),
            }
        }
        (Method::Post, "/api/brain/overlay") => {
            let body = match read_json(&mut rq) { Ok(v) => v, Err(e) => return send_err(rq, 400, &e) };
            let id = body["id"].as_str().unwrap_or("");
            let mut b = brain::write();
            if !b.has_node(id) { drop(b); return send_err(rq, 404, "No such node"); }
            if !b.overlay.set_edit(id, body["label"].as_str(), body["color"].as_str(), body["note"].as_str()) {
                drop(b); return send_err(rq, 400, "Colour must look like #rrggbb");
            }
            let saved = b.overlay.save();
            b.rebuild();
            let edit = b.overlay.nodes.get(id).cloned().unwrap_or_default();
            drop(b);
            match saved {
                Ok(()) => send_json(rq, 200, &json!({ "ok": true, "edit": edit })),
                Err(e) => send_err(rq, 500, &format!("Could not save overlay: {}", e)),
            }
        }
        (Method::Post, "/api/brain/entry") => {
            let body = match read_json(&mut rq) { Ok(v) => v, Err(e) => return send_err(rq, 400, &e) };
            let id = body["id"].as_str().unwrap_or("");
            let text = body["text"].as_str().unwrap_or("").trim();
            if text.is_empty() { return send_err(rq, 400, "Entry text is empty"); }
            let mut b = brain::write();
            if !b.has_node(id) { drop(b); return send_err(rq, 404, "No such node"); }
            let source = body["source"].as_str().filter(|s| !s.is_empty()).unwrap_or("manual");
            b.overlay.add_entry(id, text, &util::clip(source, 40));
            let saved = b.overlay.save();
            let entries: Vec<overlay::Entry> = b.overlay.entries.get(id).cloned().unwrap_or_default();
            drop(b);
            match saved {
                Ok(()) => send_json(rq, 200, &json!({ "ok": true, "entries": entries })),
                Err(e) => send_err(rq, 500, &format!("Could not save entry: {}", e)),
            }
        }
        (Method::Get, "/api/brain/search") => {
            let q = util::param(&params, "q").unwrap_or("");
            let kind = util::param(&params, "kind");
            let limit = util::param(&params, "limit").and_then(|s| s.parse().ok()).unwrap_or(20usize).min(200);
            let v = brain::read().search(q, kind, limit);
            send_json(rq, 200, &Value::Array(v))
        }
        (Method::Get, "/api/brain/recent") => {
            let limit = util::param(&params, "limit").and_then(|s| s.parse().ok()).unwrap_or(15usize).min(200);
            let v = brain::read().recent(limit, util::param(&params, "q"), util::param(&params, "sort"));
            send_json(rq, 200, &Value::Array(v))
        }
        (Method::Get, "/api/brain/project") => {
            // ?q= a project id, a path inside it (e.g. the agent's cwd) or a name.
            let q = util::param(&params, "q").unwrap_or("");
            let b = brain::read();
            let d = b.find_project(q).and_then(|id| b.node_detail(&id));
            drop(b);
            match d {
                Some(v) => send_json(rq, 200, &v),
                None => send_err(rq, 404, "No project matches; try brain_recent or a folder name"),
            }
        }
        (Method::Post, "/api/brain/note") => {
            let body = match read_json(&mut rq) { Ok(v) => v, Err(e) => return send_err(rq, 400, &e) };
            let title = body["title"].as_str().unwrap_or("");
            let content = body["content"].as_str().unwrap_or("");
            let folder = body["folder"].as_str().unwrap_or("00-Inbox");
            let r = brain::write().create_note(title, content, folder);
            match r {
                Ok((id, rel)) => send_json(rq, 200, &json!({ "ok": true, "id": id, "path": rel })),
                Err((code, msg)) => send_err(rq, code, &msg),
            }
        }
        (Method::Post, "/api/update/check") => {
            let st = crate::updater::check();
            send_json(rq, 200, &json!(st))
        }
        (Method::Post, "/api/update") => {
            let body = match read_json(&mut rq) { Ok(v) => v, Err(e) => return send_err(rq, 400, &e) };
            match crate::updater::launch(body["target"].as_str().unwrap_or("")) {
                Ok(msg) => send_json(rq, 200, &json!({ "ok": true, "message": msg })),
                Err(e) => send_err(rq, 400, &e),
            }
        }
        (Method::Options, _) => send_err(rq, 405, "Cross-origin requests are not accepted"),
        _ => send_err(rq, 404, "Not found"),
    }
}
