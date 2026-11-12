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
const SW_SHOWNORMAL: i32 = 1;
const TH32CS_SNAPPROCESS: u32 = 0x2;
const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
const INFINITE: u32 = 0xFFFF_FFFF;
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

/// `PROCESSENTRY32W` de tlhelp32.h.
#[repr(C)]
struct PROCESSENTRY32W {
    dwSize: u32,
    cntUsage: u32,
    th32ProcessID: u32,
    th32DefaultHeapID: usize,
    th32ModuleID: u32,
    cntThreads: u32,
    th32ParentProcessID: u32,
    pcPriClassBase: i32,
    dwFlags: u32,
    szExeFile: [u16; 260],
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> *mut c_void;
    fn Process32FirstW(snapshot: *mut c_void, entry: *mut PROCESSENTRY32W) -> i32;
    fn Process32NextW(snapshot: *mut c_void, entry: *mut PROCESSENTRY32W) -> i32;
    fn WaitForSingleObject(handle: *mut c_void, ms: u32) -> u32;
    fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
    pub fn CreateActCtxW(ctx: *const ACTCTXW) -> *mut c_void;
    pub fn ActivateActCtx(ctx: *mut c_void, cookie: *mut usize) -> i32;
    pub fn LoadLibraryW(name: *const u16) -> *mut c_void;
    pub fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

#[link(name = "user32")]
extern "system" {
    pub fn SendMessageW(hwnd: *mut c_void, msg: u32, wparam: usize, lparam: isize) -> isize;
    pub fn SetProcessDPIAware() -> i32;
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

/// Noms des exe des processus en cours (`CivilizationVI.exe`…), par un instantané Toolhelp.
pub fn process_names() -> Vec<String> {
    let mut names = Vec::new();
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return names;
    }
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut ok = unsafe { Process32FirstW(snapshot, &mut entry) };
    while ok != 0 {
        let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
        names.push(String::from_utf16_lossy(&entry.szExeFile[..len]));
        ok = unsafe { Process32NextW(snapshot, &mut entry) };
    }
    unsafe { CloseHandle(snapshot) };
    names
}

/// Ouvre `target` (une adresse `steam://`) comme un double-clic, par le shell, sans attendre.
pub fn open(target: &str) {
    let verb = wide("open");
    let file = wide(target);
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.hwnd = OWNER.load(Ordering::Relaxed);
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.nShow = SW_SHOWNORMAL;
    unsafe {
        CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED);
        ShellExecuteExW(&mut info);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Vérifie la disposition de `PROCESSENTRY32W` : le processus du test doit y être, sous son nom.
    #[test]
    fn processus_en_cours() {
        let me = std::env::current_exe().unwrap();
        let me = me.file_name().unwrap().to_string_lossy().to_lowercase();
        let names = process_names();
        assert!(names.len() > 1);
        assert!(names.iter().any(|n| n.to_lowercase() == me), "{me} absent de {names:?}");
    }
}
