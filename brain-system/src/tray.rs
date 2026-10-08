use std::cell::RefCell;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicU32, AtomicU16, Ordering};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::UI::Shell::{
    ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetMessageW, LoadIconW, LoadImageW, PostMessageW, PostQuitMessage,
    RegisterClassExW, RegisterWindowMessageW, SetForegroundWindow, TrackPopupMenuEx,
    TranslateMessage, HICON, IDI_APPLICATION, IMAGE_ICON, LR_LOADFROMFILE, MF_CHECKED, MF_GRAYED,
    MF_SEPARATOR, MF_STRING, MF_UNCHECKED, SW_SHOWNORMAL, TPM_BOTTOMALIGN, TPM_RIGHTALIGN,
    WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK, WM_NULL, WM_RBUTTONUP, WM_USER, WNDCLASSEXW,
    WS_OVERLAPPEDWINDOW,
};

use crate::{config, server};

const WM_TRAYICON: u32 = WM_USER + 1;

const IDM_STATUS: usize = 1001;
const IDM_OPEN_BRAIN: usize = 1002;
const IDM_OPEN_LIBRARIAN: usize = 1003;
const IDM_OPEN_VAULT: usize = 1004;
const IDM_OPEN_SKILLS: usize = 1005;
const IDM_VERIFY_NOW: usize = 1006;
const IDM_AUTOSTART: usize = 1007;
const IDM_ABOUT: usize = 1008;
const IDM_EXIT: usize = 1009;
const IDM_OPEN_WIKI: usize = 1010;
const IDM_UPDATE_CORE: usize = 1011;
const IDM_UPDATE_SKILLS: usize = 1012;
const IDM_CHECK_UPDATES: usize = 1013;

static PORT: AtomicU16 = AtomicU16::new(config::DEFAULT_PORT);
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

thread_local! {
    // The icon data, kept so the icon can be re-added if Explorer restarts.
    static NID: RefCell<Option<NOTIFYICONDATAW>> = const { RefCell::new(None) };
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn copy_into(dst: &mut [u16], s: &str) {
    let w = wide(s);
    let n = w.len().min(dst.len());
    dst[..n].copy_from_slice(&w[..n]);
    if let Some(last) = dst.last_mut() { *last = 0; }
}

fn url(path: &str) -> String {
    format!("http://127.0.0.1:{}{}", PORT.load(Ordering::Relaxed), path)
}

pub fn run_tray(port: u16, icon_path: &Path) {
    PORT.store(port, Ordering::Relaxed);
    unsafe {
        let class_name = wide("BrainSystemTrayClass");
        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: ptr::null_mut(),
            hIcon: ptr::null_mut(),
            hCursor: ptr::null_mut(),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: ptr::null_mut(),
        };
        if RegisterClassExW(&wnd_class) == 0 { return; }

        let hwnd = CreateWindowExW(
            0, class_name.as_ptr(), wide("BrainSystemHiddenWindow").as_ptr(), WS_OVERLAPPEDWINDOW,
            0, 0, 0, 0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null(),
        );
        if hwnd.is_null() { return; }
        TASKBAR_CREATED.store(RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()), Ordering::Relaxed);
        crate::updater::NOTIFY_HWND.store(hwnd as isize, Ordering::Relaxed);

        let mut hicon = LoadImageW(ptr::null_mut(), wide(&icon_path.to_string_lossy()).as_ptr(), IMAGE_ICON, 32, 32, LR_LOADFROMFILE) as HICON;
        if hicon.is_null() {
            // Missing brain.ico must not leave an invisible tray app.
            hicon = LoadIconW(ptr::null_mut(), IDI_APPLICATION);
        }

        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = hicon;
        copy_into(&mut nid.szTip, "Global Brain System");
        NID.with(|n| *n.borrow_mut() = Some(nid));

        add_icon(true);

        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        NID.with(|n| if let Some(nid) = n.borrow().as_ref() { Shell_NotifyIconW(NIM_DELETE, nid); });
    }
}

unsafe fn add_icon(first: bool) {
    NID.with(|n| {
        let mut b = n.borrow_mut();
        let Some(nid) = b.as_mut() else { return };
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        if Shell_NotifyIconW(NIM_ADD, nid) == 0 {
            Shell_NotifyIconW(NIM_MODIFY, nid);
        }
        if first {
            let mut info = *nid;
            info.uFlags = NIF_INFO;
            copy_into(&mut info.szInfoTitle, "Global Brain System");
            copy_into(&mut info.szInfo, "Running in the tray. Double-click the icon to open the Neural Brain.");
            Shell_NotifyIconW(NIM_MODIFY, &info);
        }
    });
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let taskbar = TASKBAR_CREATED.load(Ordering::Relaxed);
    if msg == crate::updater::WM_UPDATE_FOUND {
        let st = crate::updater::state();
        let mut what = Vec::new();
        if st.core.available { what.push(format!("Brain.Skills {}", st.core.latest.clone().unwrap_or_default())); }
        if st.skills.available { what.push(format!("skills {}", st.skills.latest.clone().unwrap_or_default())); }
        balloon("Update available", &format!("{} — right-click the brain icon to update.", what.join(" and ")));
        return 0;
    }
    if taskbar != 0 && msg == taskbar {
        add_icon(false); // Explorer restarted: our icon was wiped, put it back.
        return 0;
    }
    match msg {
        WM_TRAYICON => {
            let event = (lparam as u32) & 0xFFFF;
            if event == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            } else if event == WM_LBUTTONDBLCLK {
                open(&url("/"));
            }
            0
        }
        WM_COMMAND => {
            match wparam & 0xFFFF {
                IDM_OPEN_BRAIN => open(&url("/")),
                IDM_OPEN_WIKI => open(&url("/wiki")),
                IDM_OPEN_LIBRARIAN => open(&url("/librarian")),
                IDM_OPEN_VAULT => open(&config::vault_dir().to_string_lossy()),
                IDM_OPEN_SKILLS => open(&config::repo_dir().to_string_lossy()),
                IDM_VERIFY_NOW => {
                    // Off the UI thread so the tray menu never freezes.
                    std::thread::spawn(|| {
                        crate::brain::run_sweep();
                        open(&url("/librarian"));
                    });
                }
                IDM_AUTOSTART => {
                    if crate::startup::is_startup_enabled() {
                        crate::startup::disable_startup();
                    } else if let Ok(exe) = std::env::current_exe() {
                        crate::startup::enable_startup(&exe.to_string_lossy());
                    }
                }
                IDM_ABOUT => open(&url("/about")),
                IDM_UPDATE_CORE => { let _ = crate::updater::launch("core"); }
                IDM_UPDATE_SKILLS => { let _ = crate::updater::launch("skills"); }
                IDM_CHECK_UPDATES => {
                    std::thread::spawn(|| {
                        let st = crate::updater::check();
                        if !st.core.available && !st.skills.available {
                            if let Some(e) = st.error { balloon("Update check failed", &e); }
                            else { balloon("Brain.Skills is up to date", &format!("Core {}", st.core.installed.unwrap_or_default())); }
                        }
                    });
                }
                IDM_EXIT => PostQuitMessage(0),
                _ => {}
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn show_tray_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    if menu.is_null() { return; }
    let status = match server::STATUS.load(Ordering::SeqCst) {
        server::STATUS_RUNNING => format!("● Brain running on 127.0.0.1:{}", PORT.load(Ordering::Relaxed)),
        server::STATUS_FAILED => format!("✖ Server failed: {}", server::STATUS_ERROR.lock().map(|s| s.clone()).unwrap_or_default()),
        _ => "… Brain starting".to_string(),
    };
    let startup_checked = if crate::startup::is_startup_enabled() { MF_CHECKED } else { MF_UNCHECKED };
    let vault_label = format!("Open Memory Vault Folder ({})", config::vault_dir().display());
    let skills_label = format!("Open Skills Directory ({})", config::repo_dir().display());

    AppendMenuW(menu, MF_STRING | MF_GRAYED, IDM_STATUS, wide(&status).as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(menu, MF_STRING, IDM_OPEN_BRAIN, wide("Open Neural Brain (3D)").as_ptr());
    AppendMenuW(menu, MF_STRING, IDM_OPEN_WIKI, wide("Open Wiki").as_ptr());
    AppendMenuW(menu, MF_STRING, IDM_OPEN_LIBRARIAN, wide("Open Verifier Librarian").as_ptr());
    AppendMenuW(menu, MF_STRING, IDM_OPEN_VAULT, wide(&vault_label).as_ptr());
    AppendMenuW(menu, MF_STRING, IDM_OPEN_SKILLS, wide(&skills_label).as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(menu, MF_STRING, IDM_VERIFY_NOW, wide("Run Verification Sweep Now").as_ptr());
    AppendMenuW(menu, MF_STRING | startup_checked, IDM_AUTOSTART, wide("Start on PC Startup").as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(menu, MF_STRING, IDM_ABOUT, wide("About Brain System…").as_ptr());
    let up = crate::updater::state();
    if up.enabled {
        AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
        if up.core.available {
            AppendMenuW(menu, MF_STRING, IDM_UPDATE_CORE, wide(&format!("⬆ Update Brain.Skills to {}", up.core.latest.clone().unwrap_or_default())).as_ptr());
        }
        if up.skills.available {
            AppendMenuW(menu, MF_STRING, IDM_UPDATE_SKILLS, wide(&format!("⬆ Update skills to {}", up.skills.latest.clone().unwrap_or_default())).as_ptr());
        }
        AppendMenuW(menu, MF_STRING, IDM_CHECK_UPDATES, wide("Check for updates").as_ptr());
    }
    AppendMenuW(menu, MF_STRING, IDM_EXIT, wide("Exit Brain System").as_ptr());

    let mut cursor = POINT { x: 0, y: 0 };
    GetCursorPos(&mut cursor);
    SetForegroundWindow(hwnd);
    TrackPopupMenuEx(menu, TPM_RIGHTALIGN | TPM_BOTTOMALIGN, cursor.x, cursor.y, hwnd, ptr::null());
    // Documented quirk: without this the menu may not dismiss on outside click.
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);
}

/// Show a tray balloon (works from any thread: posts through the tray thread
/// when called elsewhere is not needed because NIM_MODIFY is thread-safe).
fn balloon(title: &str, text: &str) {
    let hwnd = crate::updater::NOTIFY_HWND.load(Ordering::Relaxed);
    if hwnd == 0 { return; }
    unsafe {
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd as HWND;
        nid.uID = 1;
        nid.uFlags = NIF_INFO;
        copy_into(&mut nid.szInfoTitle, title);
        copy_into(&mut nid.szInfo, text);
        Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

/// Open a URL or folder with the shell — no console window.
pub fn open(target: &str) {
    unsafe {
        ShellExecuteW(ptr::null_mut(), wide("open").as_ptr(), wide(target).as_ptr(), ptr::null(), ptr::null(), SW_SHOWNORMAL);
    }
}
