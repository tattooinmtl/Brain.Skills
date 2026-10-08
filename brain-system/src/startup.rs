//! Opt-in "start at login" via HKCU\...\Run, using the registry API directly
//! (no `reg.exe` child process, so no console window flashes).

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APP_NAME: &str = "BrainSystem";

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn open(access: u32) -> Option<HKEY> {
    let mut key: HKEY = ptr::null_mut();
    let rc = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, wide(RUN_KEY).as_ptr(), 0, access, &mut key) };
    if rc == ERROR_SUCCESS { Some(key) } else { None }
}

pub fn is_startup_enabled() -> bool {
    let Some(key) = open(KEY_QUERY_VALUE) else { return false };
    let rc = unsafe { RegQueryValueExW(key, wide(APP_NAME).as_ptr(), ptr::null(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) };
    unsafe { RegCloseKey(key) };
    rc == ERROR_SUCCESS
}

pub fn enable_startup(exe_path: &str) -> bool {
    let Some(key) = open(KEY_SET_VALUE) else { return false };
    // Quoted, so a path with spaces can't be mis-parsed by the shell.
    let value = wide(&format!("\"{}\"", exe_path.trim_matches('"')));
    let rc = unsafe {
        RegSetValueExW(key, wide(APP_NAME).as_ptr(), 0, REG_SZ, value.as_ptr() as *const u8, (value.len() * 2) as u32)
    };
    unsafe { RegCloseKey(key) };
    rc == ERROR_SUCCESS
}

pub fn disable_startup() -> bool {
    let Some(key) = open(KEY_SET_VALUE) else { return false };
    let rc = unsafe { RegDeleteValueW(key, wide(APP_NAME).as_ptr()) };
    unsafe { RegCloseKey(key) };
    rc == ERROR_SUCCESS
}
