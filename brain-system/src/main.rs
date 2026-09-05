#![windows_subsystem = "windows"]

mod config;
mod server;
mod startup;
mod tray;
mod verifier;

use std::thread;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = 6789;

    // Handle CLI arguments — startup registration is opt-in only.
    if args.len() > 1 {
        let cmd = &args[1];
        if cmd == "--install-startup" {
            let exe = std::env::current_exe().unwrap();
            crate::startup::enable_startup(&exe.to_string_lossy());
            println!("Brain System added to Windows Startup.");
            return;
        } else if cmd == "--uninstall-startup" {
            crate::startup::disable_startup();
            println!("Brain System removed from Windows Startup.");
            return;
        } else if cmd == "--verify" {
            let proposals = crate::verifier::scan_and_verify();
            println!("Verification sweep complete. {} proposals pending.", proposals.len());
            return;
        } else if cmd == "--help" || cmd == "-h" {
            println!("brain-system — memory vault tray + local HTTP API");
            println!();
            println!("Usage: brain-system [FLAG]");
            println!("  (no flag)             Run tray + HTTP server on 127.0.0.1:{}", port);
            println!("  --install-startup     Register in HKCU\\...\\Run so app starts at login");
            println!("  --uninstall-startup   Remove the startup registration");
            println!("  --verify              Run one verification sweep, print count, exit");
            println!();
            println!("Env vars:");
            println!("  BRAIN_VAULT_DIR       Override the memory vault path");
            return;
        }
    }

    // NOTE: no auto-startup registration. Prior versions silently wrote HKCU\Run
    // on first launch; that behavior was removed. Use --install-startup to opt in.

    // Spawn HTTP Server Thread (Port 6789)
    thread::spawn(move || {
        crate::server::run_server(port);
    });

    // Spawn Background Verifier Librarian Thread (every 10 minutes)
    thread::spawn(|| {
        loop {
            thread::sleep(std::time::Duration::from_secs(600));
            let _ = crate::verifier::scan_and_verify();
        }
    });

    // Run Native Windows System Tray on Main Thread
    let icon_path = crate::config::icon_path();
    crate::tray::run_tray(port, &icon_path);
}
