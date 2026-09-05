use std::fs;
use std::path::{Path, PathBuf};
use tiny_http::{Header, Request, Response, Server, StatusCode};
use walkdir::WalkDir;

use crate::config;

const HTML_INDEX: &str = include_str!("web/index.html");

// The About page is served from /about. Logo path is resolved via config
// module (was hardcoded to C:/.skills/bin/logo.jpg — now portable).
const RELEASE_DATE: &str = "September 4, 2026";
const COPYRIGHT_YEAR: &str = "2026";
const AUTHOR_NAME: &str = "Erik Boivin";
const AUTHOR_EMAIL: &str = "erik.boivin@proton.me";
const PORTAL_URL: &str = "https://portal.globalwarningnetworks.com";

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
    <div class="date">Released {release_date}</div>
    <div class="author"><a href="mailto:{email}">{name}</a></div>
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

#[derive(serde::Serialize)]
struct NodeItem {
    id: String,
    title: String,
    #[serde(rename = "type")]
    node_type: String,
    path: String,
    summary: String,
}

#[derive(serde::Serialize)]
struct LinkItem {
    source: String,
    target: String,
}

#[derive(serde::Serialize)]
struct GraphData {
    nodes: Vec<NodeItem>,
    links: Vec<LinkItem>,
}

#[derive(serde::Serialize)]
struct NoteDetail {
    title: String,
    path: String,
    #[serde(rename = "type")]
    node_type: String,
    content: String,
}

/// Returns true if the request either has no Origin header (native client
/// like curl) OR its Origin matches the local UI origin. Reject browser
/// cross-origin POSTs.
fn origin_ok(request: &Request, allowed: &str) -> bool {
    let origin = request.headers().iter().find(|h| h.field.equiv("Origin"));
    match origin {
        None => true, // native clients (curl, cli tools) have no Origin
        Some(h) => h.value.as_str() == allowed,
    }
}

pub fn run_server(port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    let server = match Server::http(&addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind server on {}: {}", addr, e);
            return;
        }
    };
    println!("Brain System HTTP Server running at http://{}", addr);

    let vault_path = config::vault_dir();
    // Canonicalize once — used to sandbox any user-supplied path.
    let vault_canon = vault_path.canonicalize().unwrap_or_else(|_| vault_path.clone());
    let allowed_origin = config::allowed_origin(port);

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().as_str().to_string();

        let nocache_h1 = Header::from_bytes(&b"Cache-Control"[..], &b"no-cache, no-store, must-revalidate"[..]).unwrap();
        let nocache_h2 = Header::from_bytes(&b"Pragma"[..], &b"no-cache"[..]).unwrap();
        // Was: Access-Control-Allow-Origin: *  (allowed any browser tab to fetch)
        // Now: locked to our own UI origin.
        let nocache_h3 = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], allowed_origin.as_bytes()).unwrap();

        if url == "/" || url == "/index.html" || url == "/wiki" || url == "/librarian" {
            let ct = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
            let resp = Response::from_string(HTML_INDEX)
                .with_header(ct)
                .with_header(nocache_h1)
                .with_header(nocache_h2)
                .with_header(nocache_h3);
            let _ = request.respond(resp);
        } else if url == "/about" {
            let ct = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
            let resp = Response::from_string(about_html())
                .with_header(ct)
                .with_header(nocache_h1)
                .with_header(nocache_h2)
                .with_header(nocache_h3);
            let _ = request.respond(resp);
        } else if url == "/logo.jpg" {
            match fs::read(config::logo_path()) {
                Ok(bytes) => {
                    let ct = Header::from_bytes(&b"Content-Type"[..], &b"image/jpeg"[..]).unwrap();
                    let resp = Response::from_data(bytes).with_header(ct);
                    let _ = request.respond(resp);
                }
                Err(_) => {
                    let _ = request.respond(Response::from_string("logo not found").with_status_code(StatusCode(404)));
                }
            }
        } else if url == "/api/version" {
            let resp_json = serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "release_date": RELEASE_DATE,
                "author": AUTHOR_NAME,
                "email": AUTHOR_EMAIL,
                "portal": PORTAL_URL,
            });
            let ct = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            let resp = Response::from_string(resp_json.to_string())
                .with_header(ct)
                .with_header(nocache_h1)
                .with_header(nocache_h2)
                .with_header(nocache_h3);
            let _ = request.respond(resp);
        } else if url == "/api/status" {
            let resp_json = serde_json::json!({
                "status": "active",
                "uptime": "live",
                "vault_path": vault_path.to_string_lossy().replace('\\', "/"),
                "port": port
            });
            let ct = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            let resp = Response::from_string(resp_json.to_string())
                .with_header(ct)
                .with_header(nocache_h1)
                .with_header(nocache_h2)
                .with_header(nocache_h3);
            let _ = request.respond(resp);
        } else if url == "/api/graph" {
            let graph = build_graph_data(&vault_path);
            let ct = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            let resp = Response::from_string(serde_json::to_string(&graph).unwrap_or_default())
                .with_header(ct)
                .with_header(nocache_h1)
                .with_header(nocache_h2)
                .with_header(nocache_h3);
            let _ = request.respond(resp);
        } else if url == "/api/notes" {
            let notes = list_notes(&vault_path);
            let ct = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            let resp = Response::from_string(serde_json::to_string(&notes).unwrap_or_default())
                .with_header(ct)
                .with_header(nocache_h1)
                .with_header(nocache_h2)
                .with_header(nocache_h3);
            let _ = request.respond(resp);
        } else if url.starts_with("/api/note?path=") {
            // SECURITY: sandbox to vault. Reject anything with `..` (defense in depth)
            // then canonicalize the requested path and verify it lives under the
            // vault root. Without this, any web page could fetch local files while
            // this server runs.
            let path_param = url.trim_start_matches("/api/note?path=");
            let decoded_path = percent_decode(path_param);
            let mut sandboxed: Option<PathBuf> = None;
            if !decoded_path.contains("..") {
                let p = PathBuf::from(&decoded_path);
                if let Ok(canon) = p.canonicalize() {
                    if canon.starts_with(&vault_canon) && canon.is_file() {
                        sandboxed = Some(canon);
                    }
                }
            }
            match sandboxed {
                Some(p) => match fs::read_to_string(&p) {
                    Ok(content) => {
                        let title = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Note".to_string());
                        let detail = NoteDetail {
                            title,
                            path: p.to_string_lossy().replace('\\', "/"),
                            node_type: if p.to_string_lossy().contains("Skills") { "Skill".to_string() } else { "Entity".to_string() },
                            content,
                        };
                        let ct = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                        let resp = Response::from_string(serde_json::to_string(&detail).unwrap_or_default())
                            .with_header(ct)
                            .with_header(nocache_h1)
                            .with_header(nocache_h2)
                            .with_header(nocache_h3);
                        let _ = request.respond(resp);
                    }
                    Err(_) => {
                        let _ = request.respond(Response::from_string("{\"error\":\"Read failed\"}").with_status_code(StatusCode(404)));
                    }
                },
                None => {
                    let _ = request.respond(Response::from_string("{\"error\":\"Forbidden: path not under vault\"}").with_status_code(StatusCode(403)));
                }
            }
        } else if url == "/api/proposals" {
            let proposals = crate::verifier::scan_and_verify();
            let ct = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            let resp = Response::from_string(serde_json::to_string(&proposals).unwrap_or_default())
                .with_header(ct)
                .with_header(nocache_h1)
                .with_header(nocache_h2)
                .with_header(nocache_h3);
            let _ = request.respond(resp);
        } else if url == "/api/proposals/resolve" && method == "POST" {
            if !origin_ok(&request, &allowed_origin) {
                let _ = request.respond(Response::from_string("{\"error\":\"Forbidden origin\"}").with_status_code(StatusCode(403)));
                continue;
            }
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);
            if let Ok(json_body) = serde_json::from_str::<serde_json::Value>(&body) {
                let id = json_body["proposal_id"].as_str().unwrap_or_default();
                let action = json_body["action"].as_str().unwrap_or("reject");
                let _ = crate::verifier::resolve_proposal(id, action);
            }
            let ct = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            let resp = Response::from_string("{\"status\":\"ok\"}")
                .with_header(ct)
                .with_header(nocache_h1)
                .with_header(nocache_h2)
                .with_header(nocache_h3);
            let _ = request.respond(resp);
        } else if url == "/api/verify" && method == "POST" {
            if !origin_ok(&request, &allowed_origin) {
                let _ = request.respond(Response::from_string("{\"error\":\"Forbidden origin\"}").with_status_code(StatusCode(403)));
                continue;
            }
            let proposals = crate::verifier::scan_and_verify();
            let ct = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            let resp = Response::from_string(serde_json::to_string(&proposals).unwrap_or_default())
                .with_header(ct)
                .with_header(nocache_h1)
                .with_header(nocache_h2)
                .with_header(nocache_h3);
            let _ = request.respond(resp);
        } else {
            let _ = request.respond(Response::from_string("Not Found").with_status_code(StatusCode(404)));
        }
    }
}

fn percent_decode(s: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            if let Ok(b) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                bytes.push(b);
            }
        } else if c == '+' {
            bytes.push(b' ');
        } else {
            bytes.push(c as u8);
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn build_graph_data(vault: &Path) -> GraphData {
    let mut nodes = Vec::new();
    let mut links = Vec::new();

    for entry in WalkDir::new(vault).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "md") {
            let path_str = entry.path().to_string_lossy().replace('\\', "/");
            let name = entry.path().file_stem().unwrap_or_default().to_string_lossy().to_string();
            let parent_name = entry.path().parent().and_then(|p| p.file_name()).unwrap_or_default().to_string_lossy().to_string();

            let node_type = if parent_name == "00-Inbox" {
                "Inbox"
            } else if parent_name == "Skills" {
                "Skill"
            } else if parent_name == "10-Entities" {
                if name.contains("Global-Brain") { "System" } else { "Entity" }
            } else if parent_name == "20-Concepts" {
                "Concept"
            } else {
                "Note"
            };

            let content = fs::read_to_string(entry.path()).unwrap_or_default();
            let summary = content.lines().take(5).collect::<Vec<_>>().join(" ");

            nodes.push(NodeItem {
                id: name.clone(),
                title: name.clone(),
                node_type: node_type.to_string(),
                path: path_str,
                summary,
            });

            // Extract [[Wikilinks]]
            let mut pos = 0;
            while let Some(start) = content[pos..].find("[[") {
                let actual_start = pos + start + 2;
                if let Some(end) = content[actual_start..].find("]]") {
                    let raw_link = &content[actual_start..actual_start + end];
                    let target_name = raw_link.split('|').next().unwrap_or(raw_link).trim();
                    let clean_target = target_name.split('/').last().unwrap_or(target_name).replace(".md", "");
                    
                    links.push(LinkItem {
                        source: name.clone(),
                        target: clean_target,
                    });
                    pos = actual_start + end + 2;
                } else {
                    break;
                }
            }
        }
    }

    GraphData { nodes, links }
}

fn list_notes(vault: &Path) -> Vec<NodeItem> {
    let mut notes = Vec::new();
    for entry in WalkDir::new(vault).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "md") {
            let path_str = entry.path().to_string_lossy().replace('\\', "/");
            let name = entry.path().file_stem().unwrap_or_default().to_string_lossy().to_string();
            let content = fs::read_to_string(entry.path()).unwrap_or_default();
            let summary = content.lines().take(4).collect::<Vec<_>>().join(" ");

            notes.push(NodeItem {
                id: name.clone(),
                title: name.clone(),
                node_type: if path_str.contains("Skills") { "Skill".to_string() } else { "Note".to_string() },
                path: path_str,
                summary,
            });
        }
    }
    notes
}
