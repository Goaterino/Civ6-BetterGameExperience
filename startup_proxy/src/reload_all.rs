//! When a client's game has objects the host's has not (or the other way round), the game would
//! reload that client alone, inside the turn-end sync check. A freshly loaded game differs from a
//! running one in state the sync check does not cover, so that player drifts away again. Instead
//! the host takes over: at the turn start (BGE's hold), it loads its own game, then every client
//! loads that game, once; games that all loaded the same save stay identical.
//!
//! Client side: the sync manager's byte + 0x3a makes a failed check acknowledged instead of asking
//! for a full copy (the client handler's tail, `CHECK`). Nothing in this build sets it otherwise;
//! a host migration clears it. It is set only while the host announces that it takes over.
//! Host side: `sync_notes.rs` counts the objects on one side only; `partie.lua` does the rest.
//! Setting `reload_all=off` keeps the game's own reload.

use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

const BASE: u64 = 0x1_4000_0000;

struct Exe {
    name: &'static str,
    /// Client handler: `mov r15,[rbp+380h] ; movzx eax,byte [r15+3Dh] ; test al,al ; jnz ;
    /// test dil,dil ; jz ; cmp [r15+3Ah],al ; jz <full snapshot>`.
    check: u64,
    /// Sync manager global, and `NetSnapshot::Process`, whose `mov rcx,[sync]` confirms it.
    sync: u64,
    snapshot_process: u64,
}

const EXES: [Exe; 2] = [
    Exe { name: "DX12", check: 0x1_407f_4c65, sync: 0x1_4239_2ee8, snapshot_process: 0x1_4081_0fa0 },
    Exe { name: "DX11", check: 0x1_407f_2d75, sync: 0x1_4235_b868, snapshot_process: 0x1_4080_f230 },
];

/// `None` marks the jnz displacement.
const CHECK: [Option<u8>; 30] = {
    const fn b(x: u8) -> Option<u8> {
        Some(x)
    }
    [
        b(0x4c), b(0x8b), b(0xbd), b(0x80), b(0x03), b(0x00), b(0x00), b(0x41), b(0x0f), b(0xb6), b(0x47), b(0x3d), b(0x84), b(0xc0),
        b(0x0f), b(0x85), None, None, None, None, b(0x40), b(0x84), b(0xff), b(0x74), b(0x0a), b(0x41), b(0x38), b(0x47), b(0x3a),
        b(0x0f),
    ]
};
/// In `NetSnapshot::Process`: `mov rcx,[sync]` at + 0x67.
const PROCESS_SYNC_AT: u64 = 0x67;
/// Sync manager: acknowledge a failed check instead of asking for a full copy.
const SYNC_NO_FULL_RESYNC: usize = 0x3a;

static MODULE: AtomicUsize = AtomicUsize::new(0);
static TABLE: AtomicUsize = AtomicUsize::new(usize::MAX);
/// When the host asked for its own load to take over a reload. Kept in this DLL, which stays
/// loaded across that load.
static PENDING: Mutex<Option<Instant>> = Mutex::new(None);

/// The `reload_all` setting (last occurrence wins; `on` by default).
fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("reload_all=")).last().map(str::trim) != Some("off")
}

fn config() -> String {
    crate::options::read_config().map(|(t, _)| t).unwrap_or_default()
}

/// Can this machine, as host, take a reload over? It must see the objects on one side only.
fn host_ready(cfg: &str) -> bool {
    !crate::salon::actif_off(cfg) && enabled(cfg) && crate::sync_notes::installed()
}

fn mark() {
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
}

/// Seconds since `mark`, and the request cleared; `None` if there is none.
fn take() -> Option<u64> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).take().map(|t| t.elapsed().as_secs())
}

/// Do the bytes at `check` and the global behind `sync` match this exe?
fn fingerprint<'a>(t: &Exe, read: impl Fn(u64, usize) -> &'a [u8]) -> Result<(), String> {
    let c = read(t.check, CHECK.len());
    if c.len() != CHECK.len() || CHECK.iter().zip(c).any(|(w, f)| w.is_some_and(|w| w != *f)) {
        return Err(format!("client check: unexpected bytes at {:#x}", t.check));
    }
    let at = t.snapshot_process + PROCESS_SYNC_AT;
    let p = read(at, 7);
    if p.len() != 7 || p[..3] != [0x48, 0x8b, 0x0d] {
        return Err(format!("NetSnapshot::Process: no sync manager read at {at:#x}"));
    }
    let rel = i32::from_le_bytes(p[3..7].try_into().unwrap());
    if (at + 7).wrapping_add_signed(i64::from(rel)) != t.sync {
        return Err(format!("NetSnapshot::Process: does not read the sync manager at {:#x}", t.sync));
    }
    Ok(())
}

/// Client: acknowledge a failed check (`on`) or ask for a full copy as the game does. False when the
/// switch is not installed or there is no sync manager yet.
unsafe fn set_no_self_reload(on: bool) -> bool {
    let Some(t) = EXES.get(TABLE.load(Ordering::Relaxed)) else { return false };
    let global = MODULE.load(Ordering::Relaxed) + (t.sync - BASE) as usize;
    let sync = std::ptr::read(global as *const usize);
    if sync == 0 {
        return false;
    }
    std::ptr::write_volatile((sync + SYNC_NO_FULL_RESYNC) as *mut u8, on as u8);
    true
}

/// Called by DllMain: finds the client switch.
pub unsafe fn install() {
    let cfg = config();
    if crate::salon::actif_off(&cfg) {
        log_line("not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log_line("turned off (reload_all=off)");
        return;
    }
    let module = crate::win::GetModuleHandleW(std::ptr::null()) as usize;
    let image = |address: u64| module + (address - BASE) as usize;
    let mut reasons = Vec::new();
    let Some(i) = EXES.iter().position(|t| {
        fingerprint(t, |address, n| std::slice::from_raw_parts(image(address) as *const u8, n))
            .map_err(|e| reasons.push(format!("{}: {e}", t.name)))
            .is_ok()
    }) else {
        log_line(&format!("unknown exe ({}), the game's own reload is kept", reasons.join("; ")));
        return;
    };
    MODULE.store(module, Ordering::Relaxed);
    TABLE.store(i, Ordering::Relaxed);
    log_line(&format!("installed ({}): the host takes reloads over when it announces it", EXES[i].name));
}

fn log_line(text: &str) {
    crate::log(&format!("reload_all: {text}"));
}

/// `Civ6Proxy_ReloadAllReady()` → 1 if this machine, as host, can take a reload over.
pub(crate) unsafe extern "C-unwind" fn lua_ready(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    (h.push_integer)(l, host_ready(&config()) as i32);
    1
}

/// `Civ6Proxy_ReloadAllMark()`: the host is about to load its own game to take a reload over.
pub(crate) unsafe extern "C-unwind" fn lua_mark(_l: *mut c_void) -> i32 {
    mark();
    0
}

/// `Civ6Proxy_ReloadAllTake()` → seconds since `Civ6Proxy_ReloadAllMark`, or -1; clears the request.
pub(crate) unsafe extern "C-unwind" fn lua_take(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    (h.push_integer)(l, take().map_or(-1, |s| s.min(i32::MAX as u64) as i32));
    1
}

/// `Civ6Proxy_ReloadAllNoSelfReload(on)` → 1 if set: this client acknowledges a failed check and
/// waits for the host instead of reloading by itself (`on` = 1), or behaves as the game does (0).
pub(crate) unsafe extern "C-unwind" fn lua_no_self_reload(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    // SAFETY: reads a Lua argument through the game's API.
    let on = crate::salon::arg_str(l, 1).is_some_and(|v| v.trim().trim_end_matches(".0") == "1");
    (h.push_integer)(l, set_no_self_reload(on) as i32);
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";
    const FILES: [&str; 2] = ["CivilizationVI_DX12.exe", "CivilizationVI.exe"];

    /// File offset of a virtual address, from the PE section table.
    fn offset(file: &[u8], address: u64) -> Option<usize> {
        let rd32 = |o: usize| u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
        let rd16 = |o: usize| u16::from_le_bytes(file[o..o + 2].try_into().unwrap()) as usize;
        let nt = rd32(0x3c);
        let sections = nt + 24 + rd16(nt + 20);
        let rva = (address - BASE) as usize;
        (0..rd16(nt + 6)).map(|i| sections + 40 * i).find_map(|s| {
            let (va, size, raw) = (rd32(s + 12), rd32(s + 16), rd32(s + 20));
            (rva >= va && rva < va + size).then(|| rva - va + raw)
        })
    }

    /// Each table matches its own installed exe, and only that one.
    #[test]
    fn bytes_in_installed_exes() {
        for (i, name) in FILES.iter().enumerate() {
            let Ok(file) = std::fs::read(format!(r"{EXE_DIR}\{name}")) else {
                eprintln!("{name} missing, test skipped");
                continue;
            };
            let read = |address: u64, n: usize| offset(&file, address).map_or(&[][..], |o| &file[o..][..n]);
            for (j, exe) in EXES.iter().enumerate() {
                let r = fingerprint(exe, read);
                if i == j {
                    assert_eq!(r, Ok(()), "{} on {name}", exe.name);
                } else {
                    assert!(r.is_err(), "the {} table must not match {name}", exe.name);
                }
            }
        }
    }

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("reload_all=off\r\n"));
        assert!(enabled("reload_all=off\nreload_all=on\n"), "the last occurrence wins");
    }

    #[test]
    fn mark_is_taken_once() {
        mark();
        assert_eq!(take(), Some(0));
        assert_eq!(take(), None, "taken once");
    }
}
