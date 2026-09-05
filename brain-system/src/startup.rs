use std::process::Command;

pub const REG_KEY_PATH: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
pub const APP_NAME: &str = "BrainSystem";

pub fn is_startup_enabled() -> bool {
    let output = Command::new("reg")
        .args(["query", REG_KEY_PATH, "/v", APP_NAME])
        .output();

    if let Ok(out) = output {
        out.status.success()
    } else {
        false
    }
}

pub fn enable_startup(exe_path: &str) -> bool {
    let status = Command::new("reg")
        .args([
            "add",
            REG_KEY_PATH,
            "/v",
            APP_NAME,
            "/t",
            "REG_SZ",
            "/d",
            exe_path,
            "/f",
        ])
        .status();

    status.map_or(false, |s| s.success())
}

pub fn disable_startup() -> bool {
    let status = Command::new("reg")
        .args(["delete", REG_KEY_PATH, "/v", APP_NAME, "/f"])
        .status();

    status.map_or(false, |s| s.success())
}
