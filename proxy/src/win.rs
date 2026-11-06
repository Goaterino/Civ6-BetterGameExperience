//! Les quelques fonctions Win32 utilisées, déclarées à la main pour n'avoir aucune dépendance.
#![allow(non_snake_case, clippy::upper_case_acronyms)]

use std::ffi::c_void;

pub type HMODULE = *mut c_void;

pub const PAGE_EXECUTE_READWRITE: u32 = 0x40;
pub const MEM_COMMIT: u32 = 0x1000;
pub const MEM_RESERVE: u32 = 0x2000;
pub const DLL_PROCESS_DETACH: u32 = 0;
pub const DLL_PROCESS_ATTACH: u32 = 1;

#[link(name = "kernel32")]
extern "system" {
    pub fn LoadLibraryW(name: *const u16) -> HMODULE;
    pub fn GetModuleHandleW(name: *const u16) -> HMODULE;
    pub fn GetModuleFileNameW(module: HMODULE, buffer: *mut u16, size: u32) -> u32;
    pub fn GetProcAddress(module: HMODULE, name: *const u8) -> *mut c_void;
    pub fn VirtualAlloc(address: *mut c_void, size: usize, kind: u32, protect: u32) -> *mut c_void;
    pub fn VirtualProtect(address: *mut c_void, size: usize, protect: u32, old: *mut u32) -> i32;
    pub fn FlushInstructionCache(process: *mut c_void, address: *const c_void, size: usize) -> i32;
    pub fn GetCurrentProcess() -> *mut c_void;
    pub fn GetCurrentThreadId() -> u32;
    pub fn QueryPerformanceCounter(count: *mut i64) -> i32;
    pub fn QueryPerformanceFrequency(freq: *mut i64) -> i32;
    pub fn DisableThreadLibraryCalls(module: HMODULE) -> i32;
    pub fn GetLastError() -> u32;
    // Proxy de démarrage seulement : relais de winmm et chargement du proxy de GameCore.
    #[allow(dead_code)]
    pub fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
    #[allow(dead_code)]
    pub fn GetFileAttributesW(name: *const u16) -> u32;
    #[allow(dead_code)]
    pub fn MultiByteToWideChar(page: u32, flags: u32, src: *const u8, len: i32, dst: *mut u16, size: i32) -> i32;
}

#[allow(dead_code)]
pub const INVALID_FILE_ATTRIBUTES: u32 = u32::MAX;
#[allow(dead_code)]
pub const CP_ACP: u32 = 0;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn qpc() -> i64 {
    let mut v = 0;
    unsafe { QueryPerformanceCounter(&mut v) };
    v
}

pub fn qpc_ms(ticks: i64) -> f64 {
    let mut f = 0;
    unsafe { QueryPerformanceFrequency(&mut f) };
    ticks as f64 * 1000.0 / f.max(1) as f64
}
