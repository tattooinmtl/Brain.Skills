#![windows_subsystem = "windows"]

mod brain;
mod config;
mod mcp;
mod overlay;
mod plugin;
mod server;
mod skills;
mod startup;
mod transcripts;
mod tray;
mod updater;
mod util;
mod vault;
mod verifier;

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::thread;
use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Threading::CreateMutexW;

/// GUI-subsystem apps have no console; attach to the parent's (if any) so
/// CLI flags like --help can print.
fn attach_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS); }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port = config::port();

    // MCP over stdio: no console attach (stdout is the protocol channel).
    if args.get(1).map(|s| s.as_str()) == Some("--mcp") {
        mcp::run();
        return;
    }

    if args.len() > 1 {
        attach_console();
        match args[1].as_str() {
            "--install-startup" => {
                let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                if startup::enable_startup(&exe) {
                    println!("Brain System added to Windows Startup.");
                } else {
                    eprintln!("Could not write the startup registry value.");
                }
            }
            "--uninstall-startup" => {
                startup::disable_startup();
                println!("Brain System removed from Windows Startup.");
            }
            "--install-claude-plugin" => plugin::install(&args[2..]),
            "--verify" => {
                let r = verifier::scan_and_verify();
                println!(
                    "Verification sweep complete: {} Gold notes checked, {} newly baselined, {} drift + {} link proposals created, {} pending.",
                    r.checked, r.baselined, r.new_drift, r.new_links, r.pending.len()
                );
            }
            _ => {
                println!("brain-system {} — memory vault tray + 3D neural brain on 127.0.0.1:{}", env!("CARGO_PKG_VERSION"), port);
                println!();
                println!("Usage: brain-system [FLAG]");
                println!("  (no flag)             Run tray + HTTP server");
                println!("  --install-startup     Start at login (HKCU\\...\\Run)");
                println!("  --uninstall-startup   Remove the startup registration");
                println!("  --verify              Run one verification sweep, print a summary, exit");
                println!("  --mcp                 MCP server on stdio (for Claude Code and other agents)");
                println!("  --install-claude-plugin [--no-register] [DIR]");
                println!("                        Write the global-brain Claude Code plugin and register it");
                println!();
                println!("Env vars:");
                println!("  BRAIN_PORT            HTTP port (default {})", config::DEFAULT_PORT);
                println!("  BRAIN_VAULT_DIR       Memory vault path");
                println!("  BRAIN_SKILLS_DIR      Skills library path");
                println!("  BRAIN_CLAUDE_PROJECTS Claude Code transcripts (default ~/.claude/projects)");
                println!("  BRAIN_CODEX_SESSIONS  Codex rollouts (default ~/.codex/sessions)");
            }
        }
        return;
    }

    // Single instance per port: a second launch just opens the running UI
    // instead of adding a second, dead tray icon.
    let name: Vec<u16> = OsStr::new(&format!("Local\\BrainSystem-{}", port)).encode_wide().chain(Some(0)).collect();
    let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if !mutex.is_null() && unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        tray::open(&format!("http://127.0.0.1:{}/", port));
        return;
    }

    thread::spawn(move || server::run_server(port));
    brain::start_indexer();
    updater::start();
    tray::run_tray(port, &config::icon_path());
}
