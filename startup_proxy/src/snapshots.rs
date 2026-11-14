//! Double load when entering a game: the host's second snapshot, arriving after the client has
//! finished loading, is refused.
//!
//! What Civ6 does (`analyses/double_load/README.md`; DX12 exe of 2026-11-05, project
//! `civ6exe1105`):
//! - when a game is entered, the host sends everyone the same snapshot (the save). As soon as its
//!   own load is over, `HandleSnapshots` (`140840260`) sends a second one, freshly saved, to every
//!   player whose acknowledgement (`NetSnapshotProcessed`) it has not received yet;
//! - the client's only guard is in `ProcessSnapshot` (`140275ad0`): a snapshot received while the
//!   loading screen is the current view is ignored ("ignored snapshot due to load-in-progress").
//!   Outside of it, the client loads it unconditionally.
//!
//! The client starts loading later than the host, by the transfer time of the first snapshot. When
//! the host finishes first, but the second snapshot takes longer to arrive than what the client has
//! left to load, the client is already in the game when it arrives: it reloads the game from the
//! game view, then stays stuck in GameCore's deserialization. Captured on 2026-11-13 (2.9 MB
//! snapshot, 11 s of transfer, second snapshot received 1.7 s after the end of the load).
//!
//! The fix: `NetSnapshot::Process` (`140810fa0`, "process" entry of its vtable) is hooked. A
//! snapshot is refused when **the game view is the current view and the local player is not waiting
//! for a snapshot** (network player + 0x1a, raised by `NetSnapshotRequest`, cleared by
//! `NetSnapshotProcessed`). Every legitimate snapshot still goes through:
//! - entering from the lobby, reconnecting: the menu is the current view;
//! - full resync (`FGameSyncManager`, `1407f84b0`) and `Network.RequestSnapshot()` (`14026deb0`):
//!   they send `NetSnapshotRequest(local, 0)` to everyone, ourselves included, which raises + 0x1a
//!   before the snapshot arrives;
//! - a reload asked by the host: `NetSnapshotRequest(player, 2)` to everyone, same thing;
//! - a second snapshot that arrives during the load: let through, the game ignores it itself.
//!
//! To refuse, the message's data size (+ 0x58) is set to 0 for the duration of the original call,
//! then restored. The original then does everything else as usual (+ 0x1b of the local player,
//! `FGameSyncManager` notified), but does not call `ProcessSnapshot`: the exact equivalent of the
//! "ignored" branch, without calling any internal function. The other machines notice nothing: the
//! host has already received our acknowledgement. Only the machine with BGE is protected.
//!
//! Every snapshot handed to the game is also reported to `rejoin.rs`, which reads its flags.
//!
//! Addresses of both exes of 2026-11-05 (`EXES`), checked byte by byte at startup; DX11 was found
//! from the vtables next to the ones of `relink` and from the bytes of the "game view" test (in DX11
//! the current view is at + 0x5400, not + 0x5420). Settings: `actif=off` installs nothing (BGE's
//! main switch); a pure bug fix, without its own setting.

use crate::{log, win};
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Base of the exe's Ghidra addresses.
const BASE: u64 = 0x1_4000_0000;

/// The "process" entry of message vtables (destructor, text, write, read, process).
const PROCESS: usize = 4;

/// `NetSnapshot`: data size (i32), read by "process" before calling `ProcessSnapshot`.
const MSG_SIZE: usize = 0x58;
/// Session vtable: local player ID, network player by ID (what "process" calls itself).
const SESSION_LOCAL: usize = 0x148;
const SESSION_PLAYER: usize = 0x158;
/// Network player: waiting for a snapshot.
const PLAYER_WAITING: usize = 0x1a;

/// Start of `NetSnapshot::Process`, the same in both exes, up to the call to `ProcessSnapshot`:
/// local player (`call [rbx+148h]`, `call [rbx+158h]`), `mov byte [rax+1bh],1`, the data test
/// (`[rdi+48h]` vs `[rdi+40h]`, `mov r8d,[rdi+58h] ; test r8d,r8d ; jle`) and `call [rax+0e8h]`.
const PROCESS_BYTES: [u8; 103] = [
    0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xec, 0x20, 0x48, 0x8b, 0xf2, 0x48, 0x8b, 0xf9, 0x48, 0x85, 0xd2, 0x74,
    0x52, 0x48, 0x89, 0x5c, 0x24, 0x30, 0x48, 0x8b, 0xca, 0x48, 0x8b, 0x1a, 0xff, 0x93, 0x48, 0x01, 0x00, 0x00, 0x8b, 0xd0,
    0x48, 0x8b, 0xce, 0xff, 0x93, 0x58, 0x01, 0x00, 0x00, 0x48, 0x8b, 0x5c, 0x24, 0x30, 0x48, 0x85, 0xc0, 0x74, 0x04, 0xc6,
    0x40, 0x1b, 0x01, 0x48, 0x8b, 0x47, 0x48, 0x48, 0x3b, 0x47, 0x40, 0x74, 0x1e, 0x44, 0x8b, 0x47, 0x58, 0x45, 0x85, 0xc0,
    0x7e, 0x15, 0x48, 0x8b, 0x4e, 0x18, 0x44, 0x8b, 0x4f, 0x38, 0x48, 0x8b, 0x57, 0x40, 0x48, 0x8b, 0x01, 0xff, 0x90, 0xe8,
    0x00, 0x00, 0x00,
];

/// "Is the game view the current view?" (`1400f1ab0` in DX12), a leaf function called as is:
/// `cmp qword [rip+…],0 ; je ; mov rax,[rip+…] ; mov rcx,[rax+14f8h] ; test rcx,rcx ; je ;
/// cmp rcx,[rax+<current view>] ; jne ; mov eax,1 ; ret ; xor eax,eax ; ret ; xor al,al ; ret`.
/// `None`: a relative displacement, which differs between the exes.
const IN_GAME_BYTES: [Option<u8>; 50] = {
    const fn b(x: u8) -> Option<u8> {
        Some(x)
    }
    const X: Option<u8> = None;
    [
        b(0x48), b(0x83), b(0x3d), X, X, X, X, b(0x00), b(0x74), b(0x25), b(0x48), b(0x8b), b(0x05), X, X, X, X,
        b(0x48), b(0x8b), b(0x88), b(0xf8), b(0x14), b(0x00), b(0x00), b(0x48), b(0x85), b(0xc9), b(0x74), b(0x0f),
        b(0x48), b(0x3b), b(0x88), X, X, b(0x00), b(0x00), b(0x75), b(0x06), b(0xb8), b(0x01), b(0x00), b(0x00),
        b(0x00), b(0xc3), b(0x33), b(0xc0), b(0xc3), b(0x32), b(0xc0), b(0xc3),
    ]
};
/// Offset of the current view's low bytes in `IN_GAME_BYTES` (`cmp rcx,[rax+…]`).
const IN_GAME_VIEW_AT: usize = 32;

/// Everything that depends on the exe (Ghidra addresses, base `0x140000000`).
struct Exe {
    name: &'static str,
    /// `NetSnapshot` vtable and its five entries.
    snapshot: (u64, [u64; 5]),
    /// The "game view" test, and the offset of the current view in the state manager.
    in_game: u64,
    current_view: u16,
}

const EXES: [Exe; 2] = [
    Exe {
        name: "DX12",
        snapshot: (0x1_4105_0c58, [0x1_4080_e510, 0x1_4080_f6c0, 0x1_4081_1530, 0x1_4081_1f20, 0x1_4081_0fa0]),
        in_game: 0x1_400f_1ab0,
        current_view: 0x5420,
    },
    Exe {
        name: "DX11",
        snapshot: (0x1_4104_17e8, [0x1_4080_c7a0, 0x1_4080_d950, 0x1_4080_f810, 0x1_4081_01b0, 0x1_4080_f230]),
        in_game: 0x1_400f_0d60,
        current_view: 0x5400,
    },
];

type Process = unsafe extern "C-unwind" fn(*mut u8, *mut c_void);
type LocalPlayer = unsafe extern "C" fn(*mut c_void) -> i32;
type PlayerById = unsafe extern "C" fn(*mut c_void, i32) -> usize;
type InGame = unsafe extern "C" fn() -> u8;

static ORIG: AtomicUsize = AtomicUsize::new(0);
static IN_GAME: AtomicUsize = AtomicUsize::new(0);

/// Checks one table against an exe; `read(address, n)` returns n bytes, `entry(address)` the
/// address stored at `address`.
fn fingerprint<'a>(t: &Exe, read: impl Fn(u64, usize) -> &'a [u8], entry: impl Fn(u64) -> u64) -> Result<(), String> {
    let (vtable, entries) = t.snapshot;
    for (k, expected) in entries.iter().enumerate() {
        if entry(vtable + 8 * k as u64) != *expected {
            return Err(format!("NetSnapshot: unexpected vtable at {vtable:#x}"));
        }
    }
    if read(entries[PROCESS], PROCESS_BYTES.len()) != PROCESS_BYTES {
        return Err(format!("NetSnapshot::Process: unexpected bytes at {:#x}", entries[PROCESS]));
    }
    let found = read(t.in_game, IN_GAME_BYTES.len());
    let view = t.current_view.to_le_bytes();
    let same = found.len() == IN_GAME_BYTES.len()
        && IN_GAME_BYTES.iter().zip(found).all(|(e, f)| e.map_or(true, |e| e == *f))
        && found[IN_GAME_VIEW_AT..IN_GAME_VIEW_AT + 2] == view;
    if !same {
        return Err(format!("game view test: unexpected bytes at {:#x}", t.in_game));
    }
    Ok(())
}

unsafe fn verify(module: usize, t: &Exe) -> Result<(), String> {
    let image = |address: u64| module + (address - BASE) as usize;
    fingerprint(
        t,
        |address, n| std::slice::from_raw_parts(image(address) as *const u8, n),
        |address| (*(image(address) as *const usize)).wrapping_sub(module) as u64 + BASE,
    )
}

/// What to do with a snapshot: refuse it only in the game view, when we are not waiting for one.
pub fn refuse(in_game: bool, waiting: Option<bool>, size: i32) -> bool {
    in_game && waiting == Some(false) && size > 0
}

/// Is the local player waiting for a snapshot? Asked of the session the way "process" does.
unsafe fn local_waiting(s: *mut c_void) -> Option<bool> {
    let vt: usize = std::ptr::read(s as *const usize);
    if vt == 0 {
        return None;
    }
    let local: LocalPlayer = std::mem::transmute(std::ptr::read((vt + SESSION_LOCAL) as *const usize));
    let player_by_id: PlayerById = std::mem::transmute(std::ptr::read((vt + SESSION_PLAYER) as *const usize));
    let id = local(s);
    let p = player_by_id(s, id);
    (p != 0).then(|| std::ptr::read((p + PLAYER_WAITING) as *const u8) != 0)
}

/// Hooked `NetSnapshot::Process`.
unsafe extern "C-unwind" fn process(msg: *mut u8, s: *mut c_void) {
    let orig: Process = std::mem::transmute(ORIG.load(Ordering::Relaxed));
    if msg.is_null() || s.is_null() {
        return orig(msg, s);
    }
    let in_game: InGame = std::mem::transmute(IN_GAME.load(Ordering::Relaxed));
    let size_at = msg.add(MSG_SIZE) as *mut i32;
    let size = std::ptr::read_unaligned(size_at);
    let game_view = in_game() != 0;
    // The session is only asked in the game view: elsewhere the snapshot always goes through.
    let waiting = if game_view { local_waiting(s) } else { None };
    if !refuse(game_view, waiting, size) {
        if size > 0 {
            crate::rejoin::snapshot_arrives(msg);
        }
        return orig(msg, s);
    }
    std::ptr::write_unaligned(size_at, 0);
    orig(msg, s);
    std::ptr::write_unaligned(size_at, size);
    log(&format!("snapshots: snapshot of {size} bytes received in the game view without being asked for, ignored (double load avoided)"));
}

/// Called by DllMain: no exe code has run yet.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("snapshots: not installed (actif=off)");
        return;
    }
    let module = win::GetModuleHandleW(std::ptr::null()) as usize;
    let mut reasons = Vec::new();
    let Some(t) = EXES.iter().find(|t| verify(module, t).map_err(|e| reasons.push(format!("{}: {e}", t.name))).is_ok()) else {
        log(&format!("snapshots: unknown exe ({}), double load guard not installed", reasons.join("; ")));
        return;
    };
    IN_GAME.store(module + (t.in_game - BASE) as usize, Ordering::Relaxed);
    let slot = (module + (t.snapshot.0 - BASE) as usize + 8 * PROCESS) as *mut usize;
    let mut old = 0u32;
    if win::VirtualProtect(slot as *mut c_void, 8, win::PAGE_EXECUTE_READWRITE, &mut old) == 0 {
        log("snapshots: VirtualProtect failed, double load guard not installed");
        return;
    }
    ORIG.store(*slot, Ordering::Relaxed);
    *slot = process as Process as usize;
    win::VirtualProtect(slot as *mut c_void, 8, old, &mut old);
    log(&format!("snapshots: double load guard installed ({})", t.name));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_refused() {
        // 2026-11-13: game view, acknowledgement processed.
        assert!(refuse(true, Some(false), 2_913_189));
        // Full resync, reload asked by the host: we are waiting.
        assert!(!refuse(true, Some(true), 2_913_189));
        // Entering, reconnecting, during the load: not the game view.
        assert!(!refuse(false, Some(false), 2_913_189));
        assert!(!refuse(false, None, 2_913_189));
        // Unknown local player, empty snapshot: the game decides.
        assert!(!refuse(true, None, 2_913_189));
        assert!(!refuse(true, Some(false), 0));
    }

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

    fn file_fingerprint(t: &Exe, file: &[u8]) -> Result<(), String> {
        let read = |address: u64, n: usize| offset(file, address).map_or(&[][..], |o| &file[o..][..n]);
        fingerprint(t, read, |address| u64::from_le_bytes(read(address, 8).try_into().unwrap_or_default()))
    }

    /// Each table matches its own installed exe, and only that one.
    #[test]
    fn bytes_in_installed_exes() {
        for (i, name) in FILES.iter().enumerate() {
            let Ok(file) = std::fs::read(format!(r"{EXE_DIR}\{name}")) else {
                eprintln!("{name} missing, test skipped");
                continue;
            };
            for (j, t) in EXES.iter().enumerate() {
                let r = file_fingerprint(t, &file);
                if i == j {
                    assert_eq!(r, Ok(()), "table {} on {name}", t.name);
                } else {
                    assert!(r.is_err(), "the {} table must not match {name}", t.name);
                }
            }
        }
    }
}
