//! Centralized path + config resolution. All hardcoded absolute paths from
//! prior versions are gone — resolve dynamically so the exe is portable.
//!
//! Resolution order for every path:
//!   1. Env var override (BRAIN_VAULT_DIR / BRAIN_ASSETS_DIR)
//!   2. `<exe-dir>/../<name>` — assumes exe lives in `<repo>/bin/`
//!   3. Sibling of exe (`<exe-dir>/<name>`)
//!   4. Hardcoded `C:\.skills\...` fallback (last resort for legacy installs)

use std::env;
use std::path::PathBuf;

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

/// Where the Obsidian-style memory vault lives.
pub fn vault_dir() -> PathBuf {
    if let Ok(v) = env::var("BRAIN_VAULT_DIR") {
        let p = PathBuf::from(v);
        if p.exists() { return p; }
    }
    let mut candidates = Vec::new();
    if let Some(r) = repo_root() { candidates.push(r.join("memory")); }
    if let Some(d) = exe_dir()  { candidates.push(d.join("memory")); }
    candidates.push(PathBuf::from(r"C:\.skills\memory"));
    first_existing(&candidates).unwrap_or_else(|| PathBuf::from(r"C:\.skills\memory"))
}

/// Logo shown on the /about page.
pub fn logo_path() -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(r) = repo_root() {
        candidates.push(r.join("assets").join("logo.jpg"));
    }
    if let Some(d) = exe_dir() {
        candidates.push(d.join("logo.jpg"));
    }
    candidates.push(PathBuf::from(r"C:\.skills\assets\logo.jpg"));
    candidates.push(PathBuf::from(r"C:\.skills\bin\logo.jpg"));
    first_existing(&candidates).unwrap_or_else(|| PathBuf::from(r"C:\.skills\assets\logo.jpg"))
}

/// Tray icon (.ico) for the Windows system tray.
pub fn icon_path() -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(r) = repo_root() {
        candidates.push(r.join("assets").join("brain.ico"));
    }
    if let Some(d) = exe_dir() {
        candidates.push(d.join("brain.ico"));
    }
    candidates.push(PathBuf::from(r"C:\.skills\assets\brain.ico"));
    first_existing(&candidates).unwrap_or_else(|| PathBuf::from(r"C:\.skills\assets\brain.ico"))
}

/// The allowed Origin for browser-based mutating requests.
pub fn allowed_origin(port: u16) -> String {
    format!("http://127.0.0.1:{}", port)
}
