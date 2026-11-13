//! Rejoining during the turn change: no reload every two turns.
//!
//! The snapshot a player loads when joining at a turn end (or on any resync) is an end-of-turn save.
//! After loading it, the game controller enters its turn barrier and sends a sync check for that
//! turn. The host has already left that turn: it keeps the check and compares it at the end of the
//! next turn, against a game that has played one more turn. Its answer rewrites hundreds of values
//! and lists objects to create and delete, which a client cannot do; one turn later the client
//! really differs, asks for a snapshot, loads it, and sends a late check again.
//!
//! The fix: a client does not enter the barrier after loading. It sends no late check, and checks
//! normally at the next turn end. The host, and every barrier entered at a turn end, are untouched.
//!
//! Settings in `rollover_proxy.cfg`: `actif=off` installs nothing; `rejoin_barrier=off` turns off
//! this fix alone (`on` by default). Addresses of both exes of 2026-11-05, checked at startup.

use crate::{log, patch, win};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const BASE: u64 = 0x1_4000_0000;

struct Exe {
    name: &'static str,
    /// The controller's barrier update: starts the sync check once.
    update: u64,
    /// The controller's "game turn ended": enters the barrier.
    turn_end: u64,
    /// The sync manager's "start check", read only to confirm `net_manager` and `lookup`.
    start_check: u64,
    /// Network manager method: session of an ID.
    lookup: u64,
    /// Globals: background jobs, sync manager, network manager.
    jobs: u64,
    sync: u64,
    net_manager: u64,
}

const EXES: [Exe; 2] = [
    Exe {
        name: "DX12",
        update: 0x1_400f_7f70,
        turn_end: 0x1_400f_62e0,
        start_check: 0x1_407f_8c10,
        lookup: 0x1_407f_b740,
        jobs: 0x1_4230_e728,
        sync: 0x1_4239_2ee8,
        net_manager: 0x1_4239_2ef8,
    },
    Exe {
        name: "DX11",
        update: 0x1_400f_75b0,
        turn_end: 0x1_400f_5920,
        start_check: 0x1_407f_6d20,
        lookup: 0x1_407f_9960,
        jobs: 0x1_422d_70a8,
        sync: 0x1_4235_b868,
        net_manager: 0x1_4235_b878,
    },
];

/// The whole barrier update, replaced by `update`. `X` marks the two RIP displacements (jobs, sync):
/// `cmp byte [rcx+4],0 ; je ; mov rax,[jobs] ; test rax,rax ; je ; cmp dword [rax+40h],0 ; seta al ;
/// test al,al ; jne ret ; cmp byte [rcx+9],0 ; jne ret ; mov byte [rcx+9],1 ; mov rcx,[sync] ;
/// mov rax,[rcx] ; jmp [rax+18h] ; ret`
const X: u8 = 0;
const UPDATE: [u8; 0x36] = [
    0x80, 0x79, 0x04, 0x00, 0x74, 0x17, 0x48, 0x8b, 0x05, X, X, X, X, 0x48, 0x85, 0xc0, 0x74, 0x0b, 0x83, 0x78, 0x40, 0x00, 0x0f, 0x97,
    0xc0, 0x84, 0xc0, 0x75, 0x18, 0x80, 0x79, 0x09, 0x00, 0x75, 0x12, 0xc6, 0x41, 0x09, 0x01, 0x48, 0x8b, 0x0d, X, X, X, X, 0x48, 0x8b,
    0x01, 0x48, 0xff, 0x60, 0x18, 0xc3,
];
const UPDATE_JOBS_AT: usize = 9;
const UPDATE_SYNC_AT: usize = 0x2a;
/// `sub rsp,28h ; mov byte [rcx+8],1 ; mov word [rcx+0Eh],100h`: moved to the trampoline as is.
const TURN_END: [u8; 14] = [0x48, 0x83, 0xec, 0x28, 0xc6, 0x41, 0x08, 0x01, 0x66, 0xc7, 0x41, 0x0e, 0x00, 0x01];
const LOOKUP: [u8; 16] = [0x44, 0x8b, 0x49, 0x18, 0x44, 0x8b, 0xc2, 0x4c, 0x8b, 0x51, 0x10, 0x33, 0xd2, 0x41, 0x8b, 0xc0];
/// In "start check": `mov rcx,[net_manager]` at + 0x11, `call lookup` at + 0x18.
const START_CHECK_MANAGER_AT: usize = 0x11;
const START_CHECK_CALL_AT: usize = 0x18;

// Controller: uses background jobs, in the barrier, check started.
const CTL_JOBS: usize = 4;
const CTL_IN_BARRIER: usize = 8;
const CTL_CHECK_STARTED: usize = 9;
const JOBS_PENDING: usize = 0x40;
// Sync manager: session ID, "start check" entry of its vtable. Session: "is host" entry.
const SYNC_SESSION: usize = 8;
const SYNC_START_CHECK: usize = 0x18;
const SESSION_IS_HOST: usize = 0x1c0;

static MODULE: AtomicUsize = AtomicUsize::new(0);
static TABLE: AtomicUsize = AtomicUsize::new(usize::MAX);
static TURN_END_ORIG: AtomicUsize = AtomicUsize::new(0);
/// The barrier was entered by a turn end (not by a load) and its check has not started yet.
static AFTER_TURN_END: AtomicBool = AtomicBool::new(false);

/// The `rejoin_barrier` setting (last occurrence wins; `on` by default).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("rejoin_barrier=")).last().map(str::trim) != Some("off")
}

/// A new game or a loaded snapshot (the in-game interface loads).
pub fn new_game() {
    AFTER_TURN_END.store(false, Ordering::Relaxed);
}

/// Leave the barrier without a check? Only one entered by a load, on a machine known to be a client.
pub fn skip(after_turn_end: bool, host: Option<bool>) -> bool {
    !after_turn_end && host == Some(false)
}

unsafe fn image(address: u64) -> usize {
    MODULE.load(Ordering::Relaxed) + (address - BASE) as usize
}

unsafe fn read<T: Copy>(address: usize) -> T {
    std::ptr::read_unaligned(address as *const T)
}

/// Is this machine the host of the session the sync manager belongs to? `None` without a session.
unsafe fn is_host(t: &Exe) -> Option<bool> {
    let sync: usize = read(image(t.sync));
    let manager: usize = read(image(t.net_manager));
    if sync == 0 || manager == 0 {
        return None;
    }
    let lookup: unsafe extern "C-unwind" fn(usize, u32) -> usize = std::mem::transmute(image(t.lookup));
    let session = lookup(manager, read(sync + SYNC_SESSION));
    if session == 0 {
        return None;
    }
    let f: unsafe extern "C-unwind" fn(usize) -> u8 = std::mem::transmute(read::<usize>(read::<usize>(session) + SESSION_IS_HOST));
    Some(f(session) != 0)
}

/// Hooked "game turn ended": the barrier that follows is a real one.
unsafe extern "C-unwind" fn turn_end(ctl: *mut u8) {
    AFTER_TURN_END.store(true, Ordering::Relaxed);
    let orig: unsafe extern "C-unwind" fn(*mut u8) = std::mem::transmute(TURN_END_ORIG.load(Ordering::Relaxed));
    orig(ctl)
}

/// Replaces the barrier update: the game's code, plus leaving a barrier entered by a load on a client.
unsafe extern "C-unwind" fn update(ctl: *mut u8) {
    let Some(t) = EXES.get(TABLE.load(Ordering::Relaxed)) else { return };
    if *ctl.add(CTL_CHECK_STARTED) != 0 {
        return;
    }
    if skip(AFTER_TURN_END.load(Ordering::Relaxed), is_host(t)) {
        *ctl.add(CTL_IN_BARRIER) = 0;
        log("rejoin: turn barrier after loading left without a sync check (client)");
        return;
    }
    if *ctl.add(CTL_JOBS) != 0 {
        let jobs: usize = read(image(t.jobs));
        if jobs != 0 && read::<u32>(jobs + JOBS_PENDING) != 0 {
            return;
        }
    }
    *ctl.add(CTL_CHECK_STARTED) = 1;
    AFTER_TURN_END.store(false, Ordering::Relaxed);
    let sync: usize = read(image(t.sync));
    let start: unsafe extern "C-unwind" fn(usize) = std::mem::transmute(read::<usize>(read::<usize>(sync) + SYNC_START_CHECK));
    start(sync)
}

/// Are this exe's bytes and globals where the table says? `read(address, n)` returns n bytes.
fn fingerprint<'a>(t: &Exe, read: impl Fn(u64, usize) -> &'a [u8]) -> Result<(), String> {
    let rip = |b: &[u8], at: usize, next: u64| next.wrapping_add_signed(i64::from(i32::from_le_bytes(b[at..at + 4].try_into().unwrap())));
    let u = read(t.update, UPDATE.len());
    let masked = |i: usize| (UPDATE_JOBS_AT..UPDATE_JOBS_AT + 4).contains(&i) || (UPDATE_SYNC_AT..UPDATE_SYNC_AT + 4).contains(&i);
    if u.len() != UPDATE.len() || u.iter().zip(UPDATE).enumerate().any(|(i, (a, b))| !masked(i) && *a != b) {
        return Err(format!("barrier update: unexpected bytes at {:#x}", t.update));
    }
    if rip(u, UPDATE_JOBS_AT, t.update + UPDATE_JOBS_AT as u64 + 4) != t.jobs || rip(u, UPDATE_SYNC_AT, t.update + UPDATE_SYNC_AT as u64 + 4) != t.sync {
        return Err("barrier update: does not read the expected globals".into());
    }
    if read(t.turn_end, TURN_END.len()) != TURN_END {
        return Err(format!("turn end: unexpected bytes at {:#x}", t.turn_end));
    }
    if read(t.lookup, LOOKUP.len()) != LOOKUP {
        return Err(format!("session lookup: unexpected bytes at {:#x}", t.lookup));
    }
    let s = read(t.start_check, START_CHECK_CALL_AT + 5);
    if s.len() != START_CHECK_CALL_AT + 5 || s[START_CHECK_MANAGER_AT..START_CHECK_MANAGER_AT + 3] != [0x48, 0x8b, 0x0d] || s[START_CHECK_CALL_AT] != 0xe8 {
        return Err(format!("start check: unexpected bytes at {:#x}", t.start_check));
    }
    if rip(s, START_CHECK_MANAGER_AT + 3, t.start_check + START_CHECK_MANAGER_AT as u64 + 7) != t.net_manager
        || rip(s, START_CHECK_CALL_AT + 1, t.start_check + START_CHECK_CALL_AT as u64 + 5) != t.lookup
    {
        return Err("start check: does not lead to the network manager and its lookup".into());
    }
    Ok(())
}

/// Called by DllMain: no exe code has run yet.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("rejoin: not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log("rejoin: turned off (rejoin_barrier=off)");
        return;
    }
    let module = win::GetModuleHandleW(std::ptr::null()) as usize;
    let at = |address: u64| module + (address - BASE) as usize;
    let mut reasons = Vec::new();
    let Some(i) = EXES.iter().position(|t| {
        fingerprint(t, |address, n| std::slice::from_raw_parts(at(address) as *const u8, n)).map_err(|e| reasons.push(format!("{}: {e}", t.name))).is_ok()
    }) else {
        log(&format!("rejoin: unknown exe ({}), fix not installed", reasons.join("; ")));
        return;
    };
    let t = &EXES[i];
    MODULE.store(module, Ordering::Relaxed);
    TABLE.store(i, Ordering::Relaxed);
    // The turn-end hook first: without it, the update would take every barrier for one after a load.
    let turn_end_at = at(t.turn_end);
    match patch::install_at("rejoin turn end", turn_end_at, &TURN_END, turn_end_at + TURN_END.len(), turn_end as *const () as usize) {
        Ok(tramp) => TURN_END_ORIG.store(tramp, Ordering::Relaxed),
        Err(e) => {
            log(&format!("rejoin: {e}, fix not installed"));
            return;
        }
    }
    let update_at = at(t.update);
    match patch::install_at("rejoin barrier update", update_at, &[], update_at, update as *const () as usize) {
        Ok(_) => log(&format!("rejoin: no turn barrier after loading on clients ({})", t.name)),
        Err(e) => log(&format!("rejoin: {e}, fix not installed")),
    }
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
            for (j, t) in EXES.iter().enumerate() {
                let r = fingerprint(t, read);
                if i == j {
                    assert_eq!(r, Ok(()), "{} on {name}", t.name);
                } else {
                    assert!(r.is_err(), "the {} table must not match {name}", t.name);
                }
            }
        }
    }

    /// The 14-byte jump fits in the replaced function and in the moved instructions.
    #[test]
    fn sites_hold_the_jump() {
        assert!(UPDATE.len() >= 14);
        assert_eq!(UPDATE[UPDATE.len() - 1], 0xc3, "the update ends at its ret");
        assert_eq!(TURN_END.len(), 14);
    }

    #[test]
    fn only_a_client_after_a_load_skips() {
        assert!(skip(false, Some(false)), "client, barrier entered by a load");
        assert!(!skip(true, Some(false)), "client, real turn end");
        assert!(!skip(false, Some(true)), "host after a load");
        assert!(!skip(true, Some(true)));
        assert!(!skip(false, None), "no session: the game's own behaviour");
    }

    #[test]
    fn new_game_forgets_the_turn_end() {
        AFTER_TURN_END.store(true, Ordering::Relaxed);
        new_game();
        assert!(!AFTER_TURN_END.load(Ordering::Relaxed));
    }

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("rejoin_barrier=off\r\n"));
        assert!(enabled("rejoin_barrier=off\nrejoin_barrier=on\n"), "the last occurrence wins");
    }
}
