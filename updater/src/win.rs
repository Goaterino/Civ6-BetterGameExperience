//! Les quelques fonctions Win32 utilisées, déclarées à la main comme dans proxy/src/win.rs.
#![allow(non_snake_case, clippy::upper_case_acronyms)]

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

const HKEY_CURRENT_USER: isize = 0x8000_0001u32 as i32 as isize;
const RRF_RT_REG_SZ: u32 = 0x2;
const SEE_MASK_NOCLOSEPROCESS: u32 = 0x40;
const SW_HIDE: i32 = 0;
const INFINITE: u32 = 0xFFFF_FFFF;
const CF_UNICODETEXT: u32 = 13;
const GMEM_MOVEABLE: u32 = 0x2;
const COINIT_APARTMENTTHREADED: u32 = 0x2;

/// Fenêtre ouverte de l'installeur : parent de l'invite UAC, qui passe ainsi au premier plan.
pub static OWNER: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

#[repr(C)]
struct SHELLEXECUTEINFOW {
    cbSize: u32,
    fMask: u32,
    hwnd: *mut c_void,
    lpVerb: *const u16,
    lpFile: *const u16,
    lpParameters: *const u16,
    lpDirectory: *const u16,
    nShow: i32,
    hInstApp: *mut c_void,
    lpIDList: *mut c_void,
    lpClass: *const u16,
    hkeyClass: *mut c_void,
    dwHotKey: u32,
    hIconOrMonitor: *mut c_void,
    hProcess: *mut c_void,
}

#[repr(C)]
pub struct ACTCTXW {
    pub cbSize: u32,
    pub dwFlags: u32,
    pub lpSource: *const u16,
    pub wProcessorArchitecture: u16,
    pub wLangId: u16,
    pub lpAssemblyDirectory: *const u16,
    pub lpResourceName: *const u16,
    pub lpApplicationName: *const u16,
    pub hModule: *mut c_void,
}

#[link(name = "kernel32")]
extern "system" {
    fn WaitForSingleObject(handle: *mut c_void, ms: u32) -> u32;
    fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
    pub fn CreateActCtxW(ctx: *const ACTCTXW) -> *mut c_void;
    pub fn ActivateActCtx(ctx: *mut c_void, cookie: *mut usize) -> i32;
    pub fn LoadLibraryW(name: *const u16) -> *mut c_void;
    pub fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    fn GlobalAlloc(flags: u32, size: usize) -> *mut c_void;
    fn GlobalLock(mem: *mut c_void) -> *mut c_void;
    fn GlobalUnlock(mem: *mut c_void) -> i32;
    fn GlobalFree(mem: *mut c_void) -> *mut c_void;
}

#[link(name = "user32")]
extern "system" {
    pub fn SendMessageW(hwnd: *mut c_void, msg: u32, wparam: usize, lparam: isize) -> isize;
    pub fn SetProcessDPIAware() -> i32;
    fn OpenClipboard(hwnd: *mut c_void) -> i32;
    fn EmptyClipboard() -> i32;
    fn SetClipboardData(format: u32, mem: *mut c_void) -> *mut c_void;
    fn CloseClipboard() -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteExW(info: *mut SHELLEXECUTEINFOW) -> i32;
}

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, flags: u32) -> i32;
}

#[link(name = "advapi32")]
extern "system" {
    fn RegGetValueW(
        key: isize,
        subkey: *const u16,
        value: *const u16,
        flags: u32,
        kind: *mut u32,
        data: *mut c_void,
        size: *mut u32,
    ) -> i32;
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Dossier de Steam (`HKCU\Software\Valve\Steam\SteamPath`).
pub fn steam_path() -> Option<PathBuf> {
    let mut buf = [0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide("Software\\Valve\\Steam").as_ptr(),
            wide("SteamPath").as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            buf.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if r != 0 || size < 2 {
        return None;
    }
    let len = size as usize / 2 - 1;
    // Steam l'écrit avec des « / ».
    Some(PathBuf::from(String::from_utf16_lossy(&buf[..len]).replace('/', "\\")))
}

/// Lance `exe params` en administrateur (invite UAC), sans fenêtre, et attend son code de sortie.
pub fn run_elevated(exe: &Path, params: &str) -> Result<u32, String> {
    let verb = wide("runas");
    let file = wide(&exe.to_string_lossy());
    let params = wide(params);
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.hwnd = OWNER.load(Ordering::Relaxed);
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = params.as_ptr();
    info.nShow = SW_HIDE;
    // ShellExecuteEx demande COM sur le thread appelant (ici, celui de la fenêtre de progression).
    unsafe { CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED) };
    if unsafe { ShellExecuteExW(&mut info) } == 0 || info.hProcess.is_null() {
        return Err(crate::lang::t(crate::textes::ADMINISTRATOR_RIGHTS_DENIED).into());
    }
    let mut code = 1;
    unsafe {
        WaitForSingleObject(info.hProcess, INFINITE);
        GetExitCodeProcess(info.hProcess, &mut code);
        CloseHandle(info.hProcess);
    }
    Ok(code)
}

pub fn copy_to_clipboard(text: &str) -> bool {
    let text = wide(text);
    unsafe {
        if OpenClipboard(OWNER.load(Ordering::Relaxed)) == 0 {
            return false;
        }
        EmptyClipboard();
        let mem = GlobalAlloc(GMEM_MOVEABLE, text.len() * 2);
        let ok = !mem.is_null() && {
            let dst = GlobalLock(mem) as *mut u16;
            std::ptr::copy_nonoverlapping(text.as_ptr(), dst, text.len());
            GlobalUnlock(mem);
            !SetClipboardData(CF_UNICODETEXT, mem).is_null()
        };
        if !ok && !mem.is_null() {
            GlobalFree(mem);
        }
        CloseClipboard();
        ok
    }
}
