//! Centralized path + config resolution. Every path resolves dynamically so
//! the exe is portable.
//!
//! Resolution order for every path:
//!   1. Env var override (BRAIN_VAULT_DIR / BRAIN_SKILLS_DIR / BRAIN_PORT ...)
//!   2. `<exe-dir>/../<name>` — assumes exe lives in `<repo>/bin/`
//!   3. Sibling of exe (`<exe-dir>/<name>`)
//!   4. The per-user install folder `%USERPROFILE%\.brain-skills\...`

use std::env;
use std::path::PathBuf;

pub const DEFAULT_PORT: u16 = 6789;

fn exe_dir() -> Option<PathBuf> {
    env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

/// Repo root — assumes exe is at `<repo>/bin/brain-system.exe`.
fn repo_root() -> Option<PathBuf> {
    exe_dir().and_then(|d| d.parent().map(|p| p.to_path_buf()))
}

fn first_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|p| p.exists()).cloned()
}

fn env_dir(var: &str) -> Option<PathBuf> {
    env::var(var).ok().map(PathBuf::from).filter(|p| p.exists())
}

/// `%USERPROFILE%\.brain-skills`: where the installers put everything.
pub fn user_root() -> PathBuf {
    home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".brain-skills")
}

/// `<repo>/<name>` next to a dev build, else the user install folder.
fn repo_relative(name: &str) -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(r) = repo_root() { candidates.push(r.join(name)); }
    if let Some(d) = exe_dir() { candidates.push(d.join(name)); }
    candidates.push(user_root().join(name));
    first_existing(&candidates).unwrap_or_else(|| user_root().join(name))
}

/// HTTP port. `BRAIN_PORT` overrides (useful to run a second instance for testing).
pub fn port() -> u16 {
    env::var("BRAIN_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_PORT)
}

/// Install root when running from an installed copy:
/// `<root>/app/bin/brain-system.exe` -> `<root>`.
pub fn install_root() -> Option<PathBuf> {
    let app = repo_root()?;
    let is_app = app.file_name().map_or(false, |n| n.eq_ignore_ascii_case("app"));
    let root = app.parent()?.to_path_buf();
    if is_app && root.join("installed.json").exists() { Some(root) } else { None }
}

/// `brain.json` written by the installer: `{ "vault_dir": ..., "skills_dir": ... }`.
fn settings() -> serde_json::Value {
    static CACHE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    CACHE.get_or_init(load_settings).clone()
}

fn load_settings() -> serde_json::Value {
    let mut candidates = Vec::new();
    if let Some(r) = install_root() { candidates.push(r.join("brain.json")); }
    if let Some(r) = repo_root() { candidates.push(r.join("brain.json")); }
    for c in candidates {
        if let Ok(t) = std::fs::read_to_string(&c) {
            if let Ok(v) = serde_json::from_str(t.trim_start_matches('\u{feff}')) { return v; }
        }
    }
    serde_json::Value::Null
}

fn setting_dir(key: &str) -> Option<PathBuf> {
    settings()[key].as_str().map(PathBuf::from).filter(|p| p.exists())
}

/// Where the Obsidian-style memory vault lives.
pub fn vault_dir() -> PathBuf {
    env_dir("BRAIN_VAULT_DIR")
        .or_else(|| setting_dir("vault_dir"))
        .or_else(|| install_root().map(|r| r.join("memory")).filter(|p| p.exists()))
        .unwrap_or_else(|| repo_relative("memory"))
}

/// The real skills library (folders containing SKILL.md).
pub fn skills_dir() -> PathBuf {
    env_dir("BRAIN_SKILLS_DIR")
        .or_else(|| setting_dir("skills_dir"))
        .or_else(|| install_root().map(|r| r.join("skills")).filter(|p| p.exists()))
        .unwrap_or_else(|| repo_relative("skills"))
}

/// Repo root shown by the tray's "Open Skills Directory".
pub fn repo_dir() -> PathBuf {
    skills_dir().parent().map(|p| p.to_path_buf()).unwrap_or_else(user_root)
}

/// Brain-owned state inside the vault (overlay edits, verifier manifest, event sink).
pub fn system_dir() -> PathBuf {
    vault_dir().join("_system")
}

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("USERPROFILE").or_else(|| env::var_os("HOME")).map(PathBuf::from)
}

/// Claude Code transcripts (`~/.claude/projects/<project>/<session>.jsonl`).
pub fn claude_projects_dir() -> Option<PathBuf> {
    env_dir("BRAIN_CLAUDE_PROJECTS").or_else(|| home_dir().map(|h| h.join(".claude").join("projects")))
}

/// Codex CLI rollouts (`~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`).
pub fn codex_sessions_dir() -> Option<PathBuf> {
    env_dir("BRAIN_CODEX_SESSIONS").or_else(|| home_dir().map(|h| h.join(".codex").join("sessions")))
}

/// Rebuildable state (the session index). Outside the vault so Obsidian
/// never sees it; deleting it only costs one full transcript re-read.
pub fn cache_dir() -> PathBuf {
    env::var("BRAIN_CACHE_DIR").ok().map(PathBuf::from).unwrap_or_else(|| user_root().join("cache"))
}

/// Harness-agnostic JSONL sink any agent can append to (see README).
pub fn events_dir() -> PathBuf {
    system_dir().join("brain-events")
}

/// Logo shown on the /about page.
pub fn logo_path() -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(r) = repo_root() { candidates.push(r.join("assets").join("logo.jpg")); }
    if let Some(d) = exe_dir() { candidates.push(d.join("logo.jpg")); }
    let installed = user_root().join("app").join("assets").join("logo.jpg");
    candidates.push(installed.clone());
    first_existing(&candidates).unwrap_or(installed)
}

/// Tray icon (.ico) for the Windows system tray.
pub fn icon_path() -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(r) = repo_root() { candidates.push(r.join("assets").join("brain.ico")); }
    if let Some(d) = exe_dir() { candidates.push(d.join("brain.ico")); }
    let installed = user_root().join("app").join("assets").join("brain.ico");
    candidates.push(installed.clone());
    first_existing(&candidates).unwrap_or(installed)
}

/// The allowed Origin for browser-based mutating requests.
pub fn allowed_origin(port: u16) -> String {
    format!("http://127.0.0.1:{}", port)
}

/// Host headers we answer to. Anything else is refused, which blocks DNS
/// rebinding (a hostile page re-pointing its own domain at 127.0.0.1).
pub fn allowed_hosts(port: u16) -> [String; 2] {
    [format!("127.0.0.1:{}", port), format!("localhost:{}", port)]
}
