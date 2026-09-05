use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::process::Command;
use std::ptr;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::UI::Shell::{
    NOTIFYICONDATAW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DispatchMessageW, GetCursorPos, GetMessageW, LoadImageW, PostQuitMessage, RegisterClassExW,
    SetForegroundWindow, TrackPopupMenuEx, HICON, IMAGE_ICON, LR_LOADFROMFILE, MF_CHECKED,
    MF_SEPARATOR, MF_STRING, MF_UNCHECKED, TPM_BOTTOMALIGN, TPM_RIGHTALIGN, WM_COMMAND,
    WM_DESTROY, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WM_USER, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};

const WM_TRAYICON: u32 = WM_USER + 1;

const IDM_STATUS: usize = 1001;
const IDM_OPEN_WIKI: usize = 1002;
const IDM_OPEN_LIBRARIAN: usize = 1003;
const IDM_OPEN_VAULT: usize = 1004;
const IDM_OPEN_SKILLS: usize = 1005;
const IDM_VERIFY_NOW: usize = 1006;
const IDM_AUTOSTART: usize = 1007;
const IDM_ABOUT: usize = 1008;
const IDM_EXIT: usize = 1009;

fn to_wstring(str: &str) -> Vec<u16> {
    OsStr::new(str).encode_wide().chain(Some(0)).collect()
}

pub fn run_tray(_port: u16, icon_path: &Path) {
    unsafe {
        let class_name = to_wstring("BrainSystemTrayClass");
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

        RegisterClassExW(&wnd_class);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            to_wstring("BrainSystemHiddenWindow").as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0, 0, 0, 0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
        );

        let icon_wpath = to_wstring(&icon_path.to_string_lossy());
        let hicon = LoadImageW(
            ptr::null_mut(),
            icon_wpath.as_ptr(),
            IMAGE_ICON,
            32, 32,
            LR_LOADFROMFILE,
        ) as HICON;

        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_INFO;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = hicon;

        let tip = to_wstring("Global Brain System (Active)");
        let max_tip = nid.szTip.len();
        let tip_len = tip.len().min(max_tip);
        nid.szTip[..tip_len].copy_from_slice(&tip[..tip_len]);

        let title = to_wstring("Global Brain System");
        let max_title = nid.szInfoTitle.len();
        let title_len = title.len().min(max_title);
        nid.szInfoTitle[..title_len].copy_from_slice(&title[..title_len]);

        let info = to_wstring("Brain active in taskbar. Click icon to open Wiki & Librarian.");
        let max_info = nid.szInfo.len();
        let info_len = info.len().min(max_info);
        nid.szInfo[..info_len].copy_from_slice(&info[..info_len]);

        Shell_NotifyIconW(NIM_ADD, &nid);

        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            DispatchMessageW(&msg);
        }

        Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            let event = lparam as u32;
            if event == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            } else if event == WM_LBUTTONDBLCLK {
                open_url("http://127.0.0.1:6789/");
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as usize;
            match id {
                IDM_OPEN_WIKI => {
                    open_url("http://127.0.0.1:6789/");
                }
                IDM_OPEN_LIBRARIAN => {
                    open_url("http://127.0.0.1:6789/librarian");
                }
                IDM_OPEN_VAULT => {
                    let _ = Command::new("explorer").arg(r"C:\.skills\memory").spawn();
                }
                IDM_OPEN_SKILLS => {
                    let _ = Command::new("explorer").arg(r"C:\.skills").spawn();
                }
                IDM_VERIFY_NOW => {
                    let _ = crate::verifier::scan_and_verify();
                    open_url("http://127.0.0.1:6789/librarian");
                }
                IDM_AUTOSTART => {
                    if crate::startup::is_startup_enabled() {
                        crate::startup::disable_startup();
                    } else {
                        if let Ok(exe) = std::env::current_exe() {
                            crate::startup::enable_startup(&exe.to_string_lossy());
                        }
                    }
                }
                IDM_ABOUT => {
                    open_url("http://127.0.0.1:6789/about");
                }
                IDM_EXIT => {
                    PostQuitMessage(0);
                }
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

    let startup_checked = if crate::startup::is_startup_enabled() { MF_CHECKED } else { MF_UNCHECKED };

    AppendMenuW(menu, MF_STRING, IDM_STATUS, to_wstring("Brain System: Active").as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(menu, MF_STRING, IDM_OPEN_WIKI, to_wstring("Open Wiki & Knowledge Graph").as_ptr());
    AppendMenuW(menu, MF_STRING, IDM_OPEN_LIBRARIAN, to_wstring("Open Verifier Librarian").as_ptr());
    AppendMenuW(menu, MF_STRING, IDM_OPEN_VAULT, to_wstring("Open Memory Vault Folder (C:/.skills/memory)").as_ptr());
    AppendMenuW(menu, MF_STRING, IDM_OPEN_SKILLS, to_wstring("Open Skills Directory (C:/.skills)").as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(menu, MF_STRING, IDM_VERIFY_NOW, to_wstring("Run Verification Sweep Now").as_ptr());
    AppendMenuW(menu, MF_STRING | startup_checked, IDM_AUTOSTART, to_wstring("Start on PC Startup").as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(menu, MF_STRING, IDM_ABOUT, to_wstring("About Brain System…").as_ptr());
    AppendMenuW(menu, MF_STRING, IDM_EXIT, to_wstring("Exit Brain System").as_ptr());

    let mut cursor = POINT { x: 0, y: 0 };
    GetCursorPos(&mut cursor);

    SetForegroundWindow(hwnd);
    TrackPopupMenuEx(
        menu,
        TPM_RIGHTALIGN | TPM_BOTTOMALIGN,
        cursor.x,
        cursor.y,
        hwnd,
        ptr::null(),
    );
    DestroyMenu(menu);
}

fn open_url(url: &str) {
    let _ = Command::new("cmd").args(["/c", "start", url]).spawn();
}
