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
//! Host side: the host never records that its launch broadcast (flag 0x10) reached the players, so
//! the second snapshot goes to everyone still loading. When the host processes its own copy of the
//! broadcast, every other player still waiting on the launch request is marked "snapshot sent"
//! (network player + 0x1b): no second save, no second snapshot. Any later snapshot request clears
//! the mark (rejoin, resync, remap). Setting `launch_guard=off` turns off this host part alone.
//!
//! Addresses of both exes of 2026-11-05 (`EXES`), checked byte by byte at startup; DX11 was found
//! from the vtables next to the ones of `relink` and from the bytes of the "game view" test (in DX11
//! the current view is at + 0x5400, not + 0x5420). Settings: `actif=off` installs nothing (BGE's
//! main switch); a pure bug fix, without its own setting.

use crate::{log, win};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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
/// NetSnapshot flags; 0x10 only on the launch broadcast.
const MSG_FLAGS: usize = 0x38;
const FLAG_LAUNCH: u32 = 0x10;
/// Session vtable: "is host". Network player: snapshot sent; settled (in the session before the
/// launch); reason of the last snapshot request (2: launch).
const SESSION_IS_HOST: usize = 0x1c0;
const PLAYER_SENT: usize = 0x1b;
const PLAYER_SETTLED: usize = 0x1c;
const PLAYER_REASON: usize = 0x28;
const REASON_LAUNCH: i32 = 2;
/// Player IDs covered by the game's own broadcast.
const PLAYER_IDS: i32 = 64;

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

const X: Option<u8> = None;
const fn b(x: u8) -> Option<u8> {
    Some(x)
}

/// NetSnapshotRequest::Process: `call [rax+158h]`, `call set_waiting` (rel32 at 0x3b),
/// `mov byte [rsi+1bh],0`.
const REQUEST: [Option<u8>; 0x66] = [
    b(0x48), b(0x85), b(0xd2), b(0x74), b(0x60), b(0x48), b(0x89), b(0x5c), b(0x24), b(0x10), b(0x57), b(0x48), b(0x83), b(0xec), b(0x20), b(0x48),
    b(0x8b), b(0x02), b(0x48), b(0x8b), b(0xda), b(0x8b), b(0x51), b(0x38), b(0x48), b(0x8b), b(0xf9), b(0x48), b(0x8b), b(0xcb), b(0x48), b(0x89),
    b(0x74), b(0x24), b(0x30), b(0xff), b(0x90), b(0x58), b(0x01), b(0x00), b(0x00), b(0x48), b(0x8b), b(0xf0), b(0x48), b(0x85), b(0xc0), b(0x74),
    b(0x25), b(0x44), b(0x8b), b(0x47), b(0x3c), b(0xb2), b(0x01), b(0x48), b(0x8b), b(0xc8), b(0xe8), X, X, X, X, b(0xc6),
    b(0x46), b(0x1b), b(0x00), b(0x48), b(0x8b), b(0xcb), b(0x48), b(0x8b), b(0x03), b(0x44), b(0x8b), b(0x47), b(0x3c), b(0x8b), b(0x57), b(0x38),
    b(0xff), b(0x90), b(0xa0), b(0x03), b(0x00), b(0x00), b(0x48), b(0x8b), b(0x74), b(0x24), b(0x30), b(0x48), b(0x8b), b(0x5c), b(0x24), b(0x38),
    b(0x48), b(0x83), b(0xc4), b(0x20), b(0x5f), b(0xc3),
];
const REQUEST_CALL_AT: u64 = 0x3b;

/// "Set waiting for a snapshot" (player, waiting, reason): `mov [rcx+1ah],dl ; test dl,dl ; jne ;
/// mov byte [rcx+1ch],1 ; ret ; mov [rcx+28h],r8d ; ret`.
const SET_WAITING: [u8; 17] = [0x88, 0x51, 0x1a, 0x84, 0xd2, 0x75, 0x05, 0xc6, 0x41, 0x1c, 0x01, 0xc3, 0x44, 0x89, 0x41, 0x28, 0xc3];

/// HandleSnapshots, up to its per-player test: `call [rax+1c0h]` (host), then per player
/// `cmp byte [rax+1ah],0 ; je`, `cmp byte [rax+1bh],0 ; jne`, `call [rax+0d8h]`. Calls at 0x73, 0xaa.
const HANDLE: [Option<u8>; 0x138] = [
    b(0x40), b(0x55), b(0x41), b(0x54), b(0x48), b(0x8d), b(0xac), b(0x24), b(0x18), b(0xfe), b(0xff), b(0xff), b(0x48), b(0x81), b(0xec), b(0xe8),
    b(0x02), b(0x00), b(0x00), b(0x48), b(0x8b), b(0x01), b(0x4c), b(0x8b), b(0xe1), b(0xff), b(0x90), b(0xc0), b(0x01), b(0x00), b(0x00), b(0x84),
    b(0xc0), b(0x0f), b(0x84), b(0x6e), b(0x05), b(0x00), b(0x00), b(0x41), b(0x80), b(0xbc), b(0x24), b(0xe0), b(0x01), b(0x00), b(0x00), b(0x00),
    b(0x0f), b(0x85), b(0x5f), b(0x05), b(0x00), b(0x00), b(0x49), b(0x8b), b(0x4c), b(0x24), b(0x18), b(0x48), b(0x8b), b(0x01), b(0xff), b(0x90),
    b(0xd0), b(0x00), b(0x00), b(0x00), b(0x84), b(0xc0), b(0x0f), b(0x84), b(0x49), b(0x05), b(0x00), b(0x00), b(0x49), b(0x8b), b(0x4c), b(0x24),
    b(0x18), b(0x48), b(0x8b), b(0x01), b(0xff), b(0x90), b(0xc8), b(0x00), b(0x00), b(0x00), b(0x84), b(0xc0), b(0x0f), b(0x84), b(0x33), b(0x05),
    b(0x00), b(0x00), b(0x49), b(0x8b), b(0x04), b(0x24), b(0x49), b(0x8b), b(0xcc), b(0xff), b(0x90), b(0x70), b(0x02), b(0x00), b(0x00), b(0x48),
    b(0x8b), b(0xc8), b(0xe8), X, X, X, X, b(0x84), b(0xc0), b(0x0f), b(0x85), b(0x16), b(0x05), b(0x00), b(0x00), b(0x49),
    b(0x8b), b(0x04), b(0x24), b(0x0f), b(0x57), b(0xc0), b(0x0f), b(0x57), b(0xc9), b(0x66), b(0x0f), b(0x7f), b(0x45), b(0x80), b(0x49), b(0x8b),
    b(0xcc), b(0xf3), b(0x0f), b(0x7f), b(0x4c), b(0x24), b(0x70), b(0xff), b(0x90), b(0x70), b(0x02), b(0x00), b(0x00), b(0x48), b(0x8b), b(0xc8),
    b(0x4c), b(0x8d), b(0x44), b(0x24), b(0x70), b(0x48), b(0x8d), b(0x55), b(0x80), b(0xe8), X, X, X, X, b(0x66), b(0x0f),
    b(0x6f), b(0x45), b(0x80), b(0x48), b(0x8b), b(0x4c), b(0x24), b(0x70), b(0x66), b(0x48), b(0x0f), b(0x7e), b(0xc0), b(0x66), b(0x0f), b(0x7f),
    b(0x44), b(0x24), b(0x50), b(0x48), b(0x3b), b(0xc1), b(0x0f), b(0x84), b(0xc9), b(0x04), b(0x00), b(0x00), b(0x48), b(0x89), b(0x9c), b(0x24),
    b(0xe0), b(0x02), b(0x00), b(0x00), b(0x48), b(0x8b), b(0x5c), b(0x24), b(0x50), b(0x4c), b(0x89), b(0xac), b(0x24), b(0xc8), b(0x02), b(0x00),
    b(0x00), b(0x41), b(0xbd), b(0x17), b(0x01), b(0x00), b(0x00), b(0x4c), b(0x89), b(0xbc), b(0x24), b(0xb8), b(0x02), b(0x00), b(0x00), b(0x4c),
    b(0x8b), b(0x7c), b(0x24), b(0x58), b(0x48), b(0x89), b(0xb4), b(0x24), b(0xd8), b(0x02), b(0x00), b(0x00), b(0x48), b(0x89), b(0xbc), b(0x24),
    b(0xd0), b(0x02), b(0x00), b(0x00), b(0x4c), b(0x89), b(0xb4), b(0x24), b(0xc0), b(0x02), b(0x00), b(0x00), b(0x0f), b(0x1f), b(0x40), b(0x00),
    b(0x48), b(0x8b), b(0x43), b(0x08), b(0x80), b(0x78), b(0x1a), b(0x00), b(0x0f), b(0x84), b(0x3e), b(0x02), b(0x00), b(0x00), b(0x80), b(0x78),
    b(0x1b), b(0x00), b(0x0f), b(0x85), b(0x34), b(0x02), b(0x00), b(0x00), b(0x49), b(0x8b), b(0x4c), b(0x24), b(0x18), b(0x8b), b(0x13), b(0x48),
    b(0x8b), b(0x01), b(0xff), b(0x90), b(0xd8), b(0x00), b(0x00), b(0x00),
];

/// Launch callback: `lea r9d,[r12+10h]` (flags 0x10), `call` NetSnapshot constructor (rel32 at 0x0d),
/// then the session broadcast `call [rax+0b8h]`.
const LAUNCH: [Option<u8>; 0x2f] = [
    b(0x45), b(0x8d), b(0x4c), b(0x24), b(0x10), b(0x44), b(0x8b), b(0xc3), b(0x48), b(0x8d), b(0x4d), b(0x80), b(0xe8), X, X, X,
    X, b(0x48), b(0x8b), b(0x05), X, X, X, X, b(0x48), b(0x8d), b(0x55), b(0x80), b(0x45), b(0x33), b(0xc9), b(0x41),
    b(0xb0), b(0x01), b(0x48), b(0x8b), b(0x48), b(0x08), b(0x48), b(0x8b), b(0x01), b(0xff), b(0x90), b(0xb8), b(0x00), b(0x00), b(0x00),
];
const LAUNCH_CALL_AT: u64 = 0x0d;

/// NetSnapshot constructor: type 0x4e8, flags stored at + 0x38, vtable `lea` (rel32 at 0x32).
const CTOR: [Option<u8>; 0x3e] = [
    b(0x48), b(0x89), b(0x5c), b(0x24), b(0x08), b(0x48), b(0x89), b(0x6c), b(0x24), b(0x10), b(0x56), b(0x57), b(0x41), b(0x56), b(0x48), b(0x83),
    b(0xec), b(0x20), b(0x49), b(0x63), b(0xf0), b(0x48), b(0x8b), b(0xea), b(0x45), b(0x33), b(0xc0), b(0xba), b(0xe8), b(0x04), b(0x00), b(0x00),
    b(0x41), b(0x8b), b(0xd9), b(0x48), b(0x8b), b(0xf9), b(0xe8), X, X, X, X, b(0xc6), b(0x47), b(0x08), b(0x01), b(0x48),
    b(0x8d), b(0x05), X, X, X, X, b(0x48), b(0x89), b(0x07), b(0x33), b(0xc0), b(0x89), b(0x5f), b(0x38),
];
const CTOR_VTABLE_AT: u64 = 0x32;

/// Everything that depends on the exe (Ghidra addresses, base `0x140000000`).
struct Exe {
    name: &'static str,
    /// `NetSnapshot` vtable and its five entries.
    snapshot: (u64, [u64; 5]),
    /// The "game view" test, and the offset of the current view in the state manager.
    in_game: u64,
    current_view: u16,
    /// Host part: NetSnapshotRequest vtable and its "process" entry, "set waiting for a snapshot",
    /// HandleSnapshots (the second snapshot), the launch broadcast site, the NetSnapshot constructor.
    request: (u64, u64),
    set_waiting: u64,
    handle: u64,
    launch: u64,
    ctor: u64,
}

const EXES: [Exe; 2] = [
    Exe {
        name: "DX12",
        snapshot: (0x1_4105_0c58, [0x1_4080_e510, 0x1_4080_f6c0, 0x1_4081_1530, 0x1_4081_1f20, 0x1_4081_0fa0]),
        in_game: 0x1_400f_1ab0,
        current_view: 0x5420,
        request: (0x1_4105_0c30, 0x1_4081_10a0),
        set_waiting: 0x1_4081_79e0,
        handle: 0x1_4084_0260,
        launch: 0x1_4027_d106,
        ctor: 0x1_4080_def0,
    },
    Exe {
        name: "DX11",
        snapshot: (0x1_4104_17e8, [0x1_4080_c7a0, 0x1_4080_d950, 0x1_4080_f810, 0x1_4081_01b0, 0x1_4080_f230]),
        in_game: 0x1_400f_0d60,
        current_view: 0x5400,
        request: (0x1_4104_17c0, 0x1_4080_f330),
        set_waiting: 0x1_4081_6210,
        handle: 0x1_4083_e680,
        launch: 0x1_4027_af66,
        ctor: 0x1_4080_c1d0,
    },
];

type Process = unsafe extern "C-unwind" fn(*mut u8, *mut c_void);
type LocalPlayer = unsafe extern "C" fn(*mut c_void) -> i32;
type PlayerById = unsafe extern "C" fn(*mut c_void, i32) -> usize;
type InGame = unsafe extern "C" fn() -> u8;

static ORIG: AtomicUsize = AtomicUsize::new(0);
static IN_GAME: AtomicUsize = AtomicUsize::new(0);
/// The host part is installed.
static LAUNCH_GUARD: AtomicBool = AtomicBool::new(false);

/// The `launch_guard` setting (last occurrence wins; `on` by default).
pub fn launch_guard_enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("launch_guard=")).last().map(str::trim) != Some("off")
}

/// What the host part relies on. An error leaves only the host part out.
fn launch_fingerprint<'a>(t: &Exe, read: impl Fn(u64, usize) -> &'a [u8], entry: impl Fn(u64) -> u64) -> Result<(), String> {
    let same = |at: u64, want: &[Option<u8>]| {
        let f = read(at, want.len());
        f.len() == want.len() && want.iter().zip(f).all(|(w, f)| w.map_or(true, |w| w == *f))
    };
    let rel = |at: u64| at.wrapping_add(4).wrapping_add_signed(i64::from(i32::from_le_bytes(read(at, 4).try_into().unwrap_or_default())));
    let (vtable, request) = t.request;
    if entry(vtable + 8 * PROCESS as u64) != request || !same(request, &REQUEST) || rel(request + REQUEST_CALL_AT) != t.set_waiting {
        return Err(format!("NetSnapshotRequest::Process: unexpected bytes at {request:#x}"));
    }
    if read(t.set_waiting, SET_WAITING.len()) != SET_WAITING {
        return Err(format!("set waiting for a snapshot: unexpected bytes at {:#x}", t.set_waiting));
    }
    if !same(t.handle, &HANDLE) {
        return Err(format!("HandleSnapshots: unexpected bytes at {:#x}", t.handle));
    }
    if !same(t.launch, &LAUNCH) || rel(t.launch + LAUNCH_CALL_AT) != t.ctor {
        return Err(format!("launch broadcast: unexpected bytes at {:#x}", t.launch));
    }
    if !same(t.ctor, &CTOR) || rel(t.ctor + CTOR_VTABLE_AT) != t.snapshot.0 {
        return Err(format!("NetSnapshot constructor: unexpected bytes at {:#x}", t.ctor));
    }
    Ok(())
}

/// Mark this network player as already sent? Only one still waiting on the launch request, sent
/// nothing since, and in the session before the launch.
pub fn launch_recipient(waiting: bool, sent: bool, settled: bool, reason: i32) -> bool {
    waiting && !sent && settled && reason == REASON_LAUNCH
}

/// Host, processing its own copy of the launch broadcast: marks the other recipients as sent.
unsafe fn mark_launch_recipients(s: *mut c_void) -> Vec<i32> {
    let vt: usize = std::ptr::read(s as *const usize);
    if vt == 0 {
        return Vec::new();
    }
    let method = |off: usize| std::ptr::read((vt + off) as *const usize);
    let is_host: unsafe extern "C" fn(*mut c_void) -> u8 = std::mem::transmute(method(SESSION_IS_HOST));
    if is_host(s) == 0 {
        return Vec::new();
    }
    let local: LocalPlayer = std::mem::transmute(method(SESSION_LOCAL));
    let player_by_id: PlayerById = std::mem::transmute(method(SESSION_PLAYER));
    let me = local(s);
    let mut marked = Vec::new();
    for id in (0..PLAYER_IDS).filter(|&id| id != me) {
        let p = player_by_id(s, id);
        if p == 0 {
            continue;
        }
        let flag = |off: usize| std::ptr::read((p + off) as *const u8) != 0;
        let reason = std::ptr::read_unaligned((p + PLAYER_REASON) as *const i32);
        if launch_recipient(flag(PLAYER_WAITING), flag(PLAYER_SENT), flag(PLAYER_SETTLED), reason) {
            std::ptr::write((p + PLAYER_SENT) as *mut u8, 1);
            marked.push(id);
        }
    }
    marked
}

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
        let flags = std::ptr::read_unaligned(msg.add(MSG_FLAGS) as *const u32);
        orig(msg, s);
        if size > 0 && flags & FLAG_LAUNCH != 0 && LAUNCH_GUARD.load(Ordering::Relaxed) {
            let marked = mark_launch_recipients(s);
            if !marked.is_empty() {
                let ids = marked.iter().map(i32::to_string).collect::<Vec<_>>().join(", ");
                log(&format!("snapshots: saved game sent to players {ids}, no second snapshot after the host's load"));
            }
        }
        return;
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
    if !launch_guard_enabled(&cfg) {
        log("snapshots: host launch guard turned off (launch_guard=off)");
        return;
    }
    let image = |address: u64| module + (address - BASE) as usize;
    match launch_fingerprint(
        t,
        |address, n| std::slice::from_raw_parts(image(address) as *const u8, n),
        |address| (*(image(address) as *const usize)).wrapping_sub(module) as u64 + BASE,
    ) {
        Ok(()) => {
            LAUNCH_GUARD.store(true, Ordering::Relaxed);
            log("snapshots: as host, a saved game is sent once (no second snapshot after the host's load)");
        }
        Err(e) => log(&format!("snapshots: {e}, host launch guard not installed")),
    }
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

    #[test]
    fn who_is_marked_at_launch() {
        // Received the launch broadcast, still loading: marked.
        assert!(launch_recipient(true, false, true, 2));
        // Acknowledged already, or already sent a snapshot.
        assert!(!launch_recipient(false, false, true, 2));
        assert!(!launch_recipient(true, true, true, 2));
        // Joined or rejoined after the launch started (new network player).
        assert!(!launch_recipient(true, false, false, -1));
        assert!(!launch_recipient(true, false, false, 2));
        // Asked for its own snapshot (resync, Network.RequestSnapshot, remap).
        assert!(!launch_recipient(true, false, true, 0));
    }

    #[test]
    fn launch_guard_setting() {
        assert!(launch_guard_enabled(""));
        assert!(!launch_guard_enabled("launch_guard=off\r\n"));
        assert!(launch_guard_enabled("launch_guard=off\nlaunch_guard=on\n"), "the last occurrence wins");
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
                    let read = |address: u64, n: usize| offset(&file, address).map_or(&[][..], |o| &file[o..][..n]);
                    let entry = |address: u64| u64::from_le_bytes(read(address, 8).try_into().unwrap_or_default());
                    assert_eq!(launch_fingerprint(t, read, entry), Ok(()), "host tables {} on {name}", t.name);
                } else {
                    assert!(r.is_err(), "the {} table must not match {name}", t.name);
                }
            }
        }
    }
}
