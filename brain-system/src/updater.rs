//! Version check + updater for installed copies (`<root>/app/bin/brain-system.exe`).
//!
//! Compares the installed builds (`<root>/installed.json`,
//! `<root>/hub/skills-installed.json`) with the manifests published on the
//! repo's main branch, and launches the installer scripts to update. A dev
//! checkout (no install root) never checks or updates.

use serde_json::{json, Value};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use crate::{config, util};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

/// Tray window to notify when an update is found (set by the tray).
pub static NOTIFY_HWND: AtomicIsize = AtomicIsize::new(0);
pub const WM_UPDATE_FOUND: u32 = 0x8000 + 7; // WM_APP + 7

#[derive(Clone, Default, serde::Serialize)]
pub struct Part {
    pub installed: Option<String>, // version label
    pub latest: Option<String>,
    pub available: bool,
}

#[derive(Clone, Default, serde::Serialize)]
pub struct UpdateState {
    pub enabled: bool,
    pub checked_at: Option<String>,
    pub error: Option<String>,
    pub core: Part,
    pub skills: Part,
}

static STATE: Mutex<Option<UpdateState>> = Mutex::new(None);

fn repo() -> String {
    std::env::var("BRAINSKILLS_REPO").unwrap_or_else(|_| "tattooinmtl/Brain.Skills".into())
}
fn branch() -> String {
    std::env::var("BRAINSKILLS_BRANCH").unwrap_or_else(|_| "main".into())
}

pub fn state() -> UpdateState {
    STATE.lock().ok().and_then(|s| s.clone()).unwrap_or_else(|| UpdateState { enabled: config::install_root().is_some(), ..Default::default() })
}

/// Background checks: shortly after start, then every 6 hours.
pub fn start() {
    if config::install_root().is_none() { return; }
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(45));
        loop {
            check();
            std::thread::sleep(Duration::from_secs(6 * 3600));
        }
    });
}

fn read_json(p: PathBuf) -> Option<Value> {
    std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
}

fn curl(url: &str) -> Result<Vec<u8>, String> {
    let out = Command::new("curl.exe")
        .args(["-fsSL", "--max-time", "25", "-H", "User-Agent: BrainSkills-updater", url])
        .stdin(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("curl unavailable: {}", e))?;
    if !out.status.success() {
        return Err(format!("could not reach GitHub ({})", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(out.stdout)
}

/// Files each installer manages, by repo path. Must match install.ps1 /
/// install-skills.ps1 exactly, or every check would report an update.
pub fn in_core(path: &str) -> bool {
    matches!(path, "bin/brain-system.exe" | "bin/skills.exe" | "assets/brain.ico" | "assets/logo.jpg")
}
pub fn in_skills(path: &str) -> bool {
    path.starts_with("skills/") || path.starts_with("commands/") || path == "bin/skills.exe"
}

/// build = first 12 hex of sha256 over the sorted "path:blobsha" lines of
/// the managed files. Same formula as the installer scripts.
fn build_of(tree: &Value, keep: fn(&str) -> bool) -> (String, usize) {
    let mut lines: Vec<String> = tree["tree"].as_array().map(|a| a.iter()
        .filter(|e| e["type"] == "blob")
        .filter_map(|e| Some((e["path"].as_str()?, e["sha"].as_str()?)))
        .filter(|(p, _)| keep(p))
        .map(|(p, s)| format!("{}:{}", p, s)).collect()).unwrap_or_default();
    lines.sort();
    let n = lines.len();
    (util::sha256_hex(lines.join("\n").as_bytes())[..12].to_string(), n)
}

/// Remote "manifests" for core and skills, computed from the commit's tree.
fn fetch_remote() -> Result<(Value, Value), String> {
    let commit: Value = serde_json::from_slice(&curl(&format!("https://api.github.com/repos/{}/commits/{}", repo(), branch()))?)
        .map_err(|e| e.to_string())?;
    let sha = commit["sha"].as_str().ok_or("GitHub API: no commit (rate-limited?)")?.to_string();
    let date = commit["commit"]["committer"]["date"].as_str().unwrap_or("").chars().take(10).collect::<String>();
    let tree: Value = serde_json::from_slice(&curl(&format!("https://api.github.com/repos/{}/git/trees/{}?recursive=1", repo(), sha))?)
        .map_err(|e| e.to_string())?;
    let version = curl(&format!("https://raw.githubusercontent.com/{}/{}/VERSION", repo(), sha))
        .map(|b| String::from_utf8_lossy(&b).trim().to_string()).unwrap_or_default();
    let (core_build, _) = build_of(&tree, in_core);
    let (skills_build, n) = build_of(&tree, in_skills);
    Ok((
        json!({ "version": version, "build": core_build, "commit": sha }),
        json!({ "version": date, "build": skills_build, "commit": sha, "files": n }),
    ))
}

fn label(v: &Value) -> Option<String> {
    let ver = v["version"].as_str()?;
    Some(match v["build"].as_str() { Some(b) => format!("{} ({})", ver, &b[..b.len().min(8)]), None => ver.to_string() })
}

fn part(installed: Option<Value>, remote: &Result<Value, String>) -> Part {
    let mut p = Part { installed: installed.as_ref().and_then(label), ..Default::default() };
    if let Ok(r) = remote {
        p.latest = label(r);
        p.available = match (&installed, r["build"].as_str()) {
            (Some(i), Some(b)) => i["build"].as_str() != Some(b),
            _ => false,
        };
    }
    p
}

/// Check now. Returns the new state; notifies the tray when something new appears.
pub fn check() -> UpdateState {
    let Some(root) = config::install_root() else {
        return UpdateState { enabled: false, ..Default::default() };
    };
    let remote = fetch_remote();
    let core_remote = remote.clone().map(|r| r.0);
    let skills_remote = remote.map(|r| r.1);
    let skills_installed = read_json(root.join("hub").join("skills-installed.json"));
    let mut st = UpdateState {
        enabled: true,
        checked_at: Some(util::iso(util::now_ms())),
        error: core_remote.as_ref().err().cloned(),
        core: part(read_json(root.join("installed.json")), &core_remote),
        skills: part(skills_installed, &skills_remote),
    };
    if st.skills.installed.is_none() { st.skills.available = false; }
    let was = STATE.lock().ok().and_then(|s| s.clone());
    let newly = (st.core.available && !was.as_ref().map_or(false, |w| w.core.available && w.core.latest == st.core.latest))
        || (st.skills.available && !was.as_ref().map_or(false, |w| w.skills.available && w.skills.latest == st.skills.latest));
    if let Ok(mut g) = STATE.lock() { *g = Some(st.clone()); }
    if newly {
        let hwnd = NOTIFY_HWND.load(Ordering::Relaxed);
        if hwnd != 0 {
            unsafe { windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(hwnd as _, WM_UPDATE_FOUND, 0, 0); }
        }
    }
    st
}

/// Launch the updater for "core" or "skills" in its own console window, so
/// the user sees progress. The core updater stops and restarts this app.
pub fn launch(target: &str) -> Result<String, String> {
    let root = config::install_root().ok_or("This copy is a development checkout; it is not managed by the installer.")?;
    let script = match target {
        "core" => root.join("update.ps1"),
        "skills" => root.join("hub").join("update-skills.ps1"),
        _ => return Err("target must be core or skills".into()),
    };
    if !script.exists() { return Err(format!("{} is missing — run the setup again.", script.display())); }
    Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .creation_flags(CREATE_NEW_CONSOLE)
        .spawn()
        .map_err(|e| format!("Could not start the updater: {}", e))?;
    Ok(format!("Updater started ({}).", script.display()))
}

pub fn as_json() -> Value {
    json!(state())
}
