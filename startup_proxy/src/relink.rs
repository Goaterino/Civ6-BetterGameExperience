//! Re-linking pairs relayed through the host: two players whose direct link failed when one of them
//! arrived go through the host for the rest of the game. BGE rebuilds the direct link during the
//! simultaneous turn start, without anyone leaving the game.
//!
//! What Civ6 does (Ghidra project `civ6-netcode`, DX12 exe of 2026-11-05):
//! - an arriving player receives from the host one `NetPeerInfoSteam(player, steamID)` per player
//!   already there, and opens a pending connection to each (the session's queue, one at a time). The
//!   other side accepts any connection carrying the session key, then each side sends its
//!   `NetIntroductionSteam` on the new link: the queue drops the pending connection, and
//!   `CreateNetPlayer` (`140812f90`) binds the player to the connection;
//! - with no answer for 20 s, the connection fails: `NetPeerFailure` to the host, which broadcasts
//!   `NetRelay(new, existing)` to everyone. Each machine records the pair in the relay manager
//!   (session + 0x60, table player → partners); messages between the two go through the host.
//!   Nothing retries the link. When the host leaves, each machine disconnects every player in its
//!   table (`14084f5d0`);
//! - Civ6's own path cannot link a pair that is already relayed: on the introduction of a relayed
//!   player it skips the queue, and `CreateNetPlayer` replaces the existing player ("Disconnecting
//!   the old connection"), which publishes `DelayedPlayerLeft`: the game would believe the player
//!   left and came back (pause, MPH's disconnect handling, desync).
//!
//! What BGE does (both players of the pair have BGE: the simultaneous turn start requires it):
//! - `start(partner, steamID)` opens a pending connection to the partner, as on arrival (the
//!   `NetPeerInfoSteam` handler is given a made-up message);
//! - the `NetIntroductionSteam` handler is hooked: an introduction from a player relayed with us,
//!   arriving on a connection that no player has yet, is the re-link. We bind the existing player
//!   to that connection in place (its connection at + 0x10, the connection → player table), drop
//!   our pending connection to it, and remove the pair from the relay table: Civ6 now sends to it
//!   directly. No departure is published;
//! - the `NetRelay` handler is hooked: a `NetRelay` for a pair already relayed is ignored. That is
//!   what the host sends back when our attempt fails; Civ6 would handle it by resending the
//!   introductions through the relay, and the partner would replace the player (departure published);
//! - a failed pending connection stays at the front of the queue (the host, which relays, does not
//!   answer `NetPeerFailure`): we pop it ourselves, otherwise it blocks the next attempt;
//! - `forget(a, b)` removes the pair from the local table, on every player with BGE, when the
//!   initiator announces the success (`partie.lua`): otherwise, when the host leaves, they would
//!   disconnect both players.
//!
//! The Lua side (`partie.lua`) picks the moment: the simultaneous turn start (orders held, nobody
//! plays, nothing in flight between the two). It says in the chat that it found a link through the
//! host and is fixing it, then the result, and holds the "ready" signal until the attempt is over.
//! Addresses of both exes of 2026-11-05 (`EXES`), checked byte by byte at startup.

use crate::{log, win};
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

/// Base of the exe's Ghidra addresses.
const BASE: u64 = 0x1_4000_0000;

/// The "process" entry of message vtables (destructor, text, write, read, process).
const PROCESS: usize = 4;

/// Everything that depends on the exe (Ghidra addresses, base `0x140000000`).
struct Exe {
    name: &'static str,
    /// Vtables of `NetIntroductionSteam`, `NetRelay` and `NetPeerInfoSteam`, and their five entries.
    intro: (u64, [u64; 5]),
    relay: (u64, [u64; 5]),
    peer_info: (u64, [u64; 5]),
    /// Vtables of the messages a player sends on every new link, in the order of `REPEATED`.
    repeated: [(u64, [u64; 5]); 3],
    /// The network manager (a global pointer): connections at + 0x88, sessions at + 0x10.
    net_manager: u64,
    /// Offset, in the `NetIntroductionSteam` handler, of the `mov rcx,[rip+…]` that reads it.
    net_manager_read_at: u64,
    queue_front: u64,
    queue_pop: u64,
    connection: u64,
    connection_set_steam: u64,
    player_find: u64,
    map_slot: u64,
    relay_erase: u64,
    relay_contains: u64,
}

/// First bytes of the called functions, the same in both exes, in this order: queue_front,
/// queue_pop, connection, connection_set_steam, player_find, map_slot, relay_erase, relay_contains.
const PROLOGUES: [(&str, [u8; 12]); 8] = [
    ("queue front", [0x48, 0x8b, 0x41, 0x18, 0x48, 0x2b, 0x41, 0x10, 0x48, 0xa9, 0xf8, 0xff]),
    ("queue pop", [0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xec, 0x20, 0x48, 0x8b]),
    ("connection", [0x4c, 0x8b, 0x01, 0x49, 0x8b, 0xc8, 0x49, 0x8b, 0x40, 0x08, 0x80, 0x78]),
    ("connection steamID", [0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48]),
    ("player", [0x40, 0x53, 0x48, 0x83, 0xec, 0x10, 0x44, 0x8b, 0x59, 0x28, 0x44, 0x8b]),
    ("connection table slot", [0x48, 0x83, 0xec, 0x58, 0x44, 0x8b, 0x02, 0x33, 0xd2, 0x44, 0x8b, 0x49]),
    ("relayed player erase", [0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x6c, 0x24, 0x10, 0x48, 0x89]),
    ("relayed pair?", [0x48, 0x83, 0xec, 0x18, 0x44, 0x8b, 0x51, 0x10, 0x44, 0x8b, 0xca, 0x4c]),
];

/// The DX11 addresses were found function by function as for the network message guard (same bytes
/// except relative displacements, same size in `.pdata`); tool: `Tools\civ6-dx11-map\relais_map.py`.
const EXES: [Exe; 2] = [
    Exe {
        name: "DX12",
        intro: (0x1_4105_7460, [0x1_4085_1500, 0x1_4085_16d0, 0x1_4085_1ba0, 0x1_4085_1c00, 0x1_4085_1730]),
        relay: (0x1_4105_0b90, [0x1_4080_a9a0, 0x1_4080_f610, 0x1_4080_af90, 0x1_4080_afe0, 0x1_4081_0c60]),
        peer_info: (0x1_4105_7438, [0x1_4085_1500, 0x1_4085_1700, 0x1_4085_1ba0, 0x1_4085_1c00, 0x1_4085_1aa0]),
        repeated: [
            (0x1_4105_0be0, [0x1_4080_e4c0, 0x1_4080_f5b0, 0x1_4081_14c0, 0x1_4081_1e80, 0x1_4081_0930]),
            (0x1_4105_0c08, [0x1_4080_a9a0, 0x1_4080_f180, 0x1_4081_15a0, 0x1_4081_1760, 0x1_4080_f850]),
            (0x1_4105_0c80, [0x1_4080_a9a0, 0x1_4080_f6e0, 0x1_4081_15a0, 0x1_4081_1760, 0x1_4081_1020]),
        ],
        net_manager: 0x1_4239_2ef8,
        net_manager_read_at: 0x52,
        queue_front: 0x1_4084_d580,
        queue_pop: 0x1_4084_d5d0,
        connection: 0x1_4083_7ed0,
        connection_set_steam: 0x1_4084_ecb0,
        player_find: 0x1_4081_35d0,
        map_slot: 0x1_4081_28f0,
        relay_erase: 0x1_4084_f9d0,
        relay_contains: 0x1_4084_f3e0,
    },
    Exe {
        name: "DX11",
        intro: (0x1_4104_7fd0, [0x1_4084_f860, 0x1_4084_fa30, 0x1_4084_ff00, 0x1_4084_ff60, 0x1_4084_fa90]),
        relay: (0x1_4104_1720, [0x1_4080_9350, 0x1_4080_d8a0, 0x1_4080_f780, 0x1_4081_0080, 0x1_4080_eef0]),
        peer_info: (0x1_4104_7fa8, [0x1_4084_f860, 0x1_4084_fa60, 0x1_4084_ff00, 0x1_4084_ff60, 0x1_4084_fe00]),
        repeated: [
            (0x1_4104_1770, [0x1_4080_c750, 0x1_4080_d840, 0x1_4080_f750, 0x1_4081_0050, 0x1_4080_ebc0]),
            (0x1_4104_1798, [0x1_4080_9350, 0x1_4080_d410, 0x1_4080_f470, 0x1_4081_02c0, 0x1_4080_dae0]),
            (0x1_4104_1810, [0x1_4080_9350, 0x1_4080_d970, 0x1_4080_f470, 0x1_4081_02c0, 0x1_4080_f2b0]),
        ],
        net_manager: 0x1_4235_b878,
        net_manager_read_at: 0x52,
        queue_front: 0x1_4084_b990,
        queue_pop: 0x1_4084_b9e0,
        connection: 0x1_4083_64e0,
        connection_set_steam: 0x1_4084_d030,
        player_find: 0x1_4081_1b80,
        map_slot: 0x1_4081_0bd0,
        relay_erase: 0x1_4084_dd50,
        relay_contains: 0x1_4084_d760,
    },
];

// Layout of the network objects, the same in both exes (read from the session's getters:
// `mov rax,[rcx+48h] ; ret` etc., checked every time the session is used).
const SESSION_PLAYERS: usize = 0x40;
const SESSION_QUEUE: usize = 0x48;
const SESSION_RELAYS: usize = 0x60;
/// The session's getters (entries of its vtable) and their bytes: queue, players, relays.
const GETTERS: [(usize, [u8; 5]); 3] =
    [(0x268, [0x48, 0x8b, 0x41, 0x48, 0xc3]), (0x270, [0x48, 0x8b, 0x41, 0x40, 0xc3]), (0x288, [0x48, 0x8b, 0x41, 0x60, 0xc3])];
/// Player manager: our player ID, table connection → player (table object).
const PLAYERS_LOCAL: usize = 0x08;
const PLAYERS_CONNECTIONS: usize = 0x40;
/// Network player: its connection (0 while relayed).
const PLAYER_CONNECTION: usize = 0x10;
/// Pending connection: target player, state (0xb: failed).
const PENDING_PLAYER: usize = 0x50;
const PENDING_STATE: usize = 0x5c;
const STATE_FAILED: i32 = 0xb;
/// Network message: arrival connection; `NetIntroductionSteam` and `NetPeerInfoSteam`: player,
/// steamID; `NetRelay`: new, existing.
const MSG_CONNECTION: usize = 0x10;
const MSG_PLAYER: usize = 0x38;
const MSG_STEAM: usize = 0x3c;
const MSG_RELAY_EXISTING: usize = 0x3c;
/// Size of the made-up `NetPeerInfoSteam` message (its handler only reads + 0x38 to + 0x44).
const MSG_SIZE: usize = 0x80;

/// An attempt is given up after this delay, answer or not (the transport gives up at 20 s).
pub const ATTEMPT_MAX_MS: u64 = 45_000;

/// What each player sends on a link that opens, right after its introduction (seen in the log of
/// 2026-11-10, 20:33:06: introduction, `NetRegister`, `NetAppRegister`, `NetSnapshotProcessed`
/// infoOnly). On a re-link, the partner is already registered with us (the same network player, its
/// flags kept): these messages tell nothing new, and `NetRegister` would call the application back
/// ("player registered") for a player who never left. They are dropped when they arrive on the
/// re-linked connection, for `REPEATED_MS`.
const REPEATED: [&str; 3] = ["NetRegister", "NetAppRegister", "NetSnapshotProcessed"];
const REPEATED_MS: u64 = 30_000;

type Process = unsafe extern "C-unwind" fn(*mut u8, *mut c_void);

static MODULE: AtomicUsize = AtomicUsize::new(0);
static TABLE: AtomicUsize = AtomicUsize::new(usize::MAX);
static SESSION: AtomicUsize = AtomicUsize::new(0);
static ORIG_INTRO: AtomicUsize = AtomicUsize::new(0);
static ORIG_RELAY: AtomicUsize = AtomicUsize::new(0);
static ORIG_REPEATED: [AtomicUsize; 3] = [AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0)];
/// Recently re-linked connections, and until when their repeated messages are dropped.
static RECENT: Mutex<Vec<(u32, u64)>> = Mutex::new(Vec::new());

/// The attempt in progress: partner and start time (ms).
struct Attempt {
    partner: i32,
    start_ms: u64,
}
static ATTEMPT: Mutex<Option<Attempt>> = Mutex::new(None);
/// Bit n: player n was linked directly by BGE since the last attempt started (`state`).
static RELINKED: AtomicU64 = AtomicU64::new(0);

fn table() -> Option<&'static Exe> {
    EXES.get(TABLE.load(Ordering::Relaxed))
}

fn now_ms() -> u64 {
    win::qpc_ms(win::qpc()) as u64
}

unsafe fn image(address: u64) -> usize {
    MODULE.load(Ordering::Relaxed) + (address - BASE) as usize
}

unsafe fn read<T: Copy>(address: usize) -> T {
    std::ptr::read_unaligned(address as *const T)
}

/// A valid Civ6 session (its getters have the expected bytes)?
unsafe fn session_valid(s: usize) -> bool {
    if s == 0 {
        return false;
    }
    let vt: usize = read(s);
    if vt == 0 {
        return false;
    }
    GETTERS.iter().all(|(entry, bytes)| {
        let f: usize = read(vt + entry);
        f != 0 && std::slice::from_raw_parts(f as *const u8, bytes.len()) == bytes
    })
}

/// The game's session: the one seen with the last handled message, otherwise the first one of the
/// network manager (table session → object at + 0x10, number of buckets at + 0x18).
unsafe fn session() -> Option<usize> {
    let s = SESSION.load(Ordering::Relaxed);
    if session_valid(s) {
        return Some(s);
    }
    let t = table()?;
    let manager: usize = read(image(t.net_manager));
    if manager == 0 {
        return None;
    }
    let buckets: usize = read(manager + 0x10);
    let n: u32 = read(manager + 0x18);
    if buckets == 0 || n == 0 || n > 4096 {
        return None;
    }
    for i in 0..n as usize {
        let mut e: usize = read(buckets + 8 * i);
        let mut guard = 0;
        while e != 0 && guard < 64 {
            let s: usize = read(e + 8);
            if session_valid(s) {
                SESSION.store(s, Ordering::Relaxed);
                return Some(s);
            }
            e = read(e + 0x10);
            guard += 1;
        }
    }
    None
}

unsafe fn object(s: usize, offset: usize) -> Option<usize> {
    let o: usize = read(s + offset);
    (o != 0).then_some(o)
}

unsafe fn local(s: usize) -> Option<i32> {
    Some(read(object(s, SESSION_PLAYERS)? + PLAYERS_LOCAL))
}

unsafe fn relayed(s: usize, a: i32, b: i32) -> bool {
    let (Some(t), Some(r)) = (table(), object(s, SESSION_RELAYS)) else { return false };
    let f: unsafe extern "C-unwind" fn(usize, u32, i32) -> u8 = std::mem::transmute(image(t.relay_contains));
    f(r, a as u32, b) != 0
}

/// Does a Civ6 integer → value table (buckets at + 8, number of buckets at + 0x10; entry: key + 0,
/// value + 8, next + 0x10) contain the key?
unsafe fn map_contains(t: usize, key: u32) -> bool {
    let buckets: usize = read(t + 8);
    let n: u32 = read(t + 0x10);
    if buckets == 0 || n == 0 {
        return false;
    }
    let end: usize = read(buckets + 8 * n as usize);
    let mut e: usize = read(buckets + 8 * (key % n) as usize);
    let mut guard = 0;
    while e != 0 && e != end && guard < 4096 {
        if read::<u32>(e) == key {
            return true;
        }
        e = read(e + 0x10);
        guard += 1;
    }
    false
}

/// Removes `v` from a partner list (vector of integers); returns how many are left.
pub fn remove_from(list: &mut Vec<i32>, v: i32) -> usize {
    list.retain(|&x| x != v);
    list.len()
}

/// Removes `b` from the partners of `a` in the relay table (entry: key + 0, vector + 8 / + 0x10,
/// next + 0x20; buckets at + 8, count at + 0x10); erases the entry of `a` if it becomes empty.
unsafe fn remove_partner(r: usize, a: i32, b: i32) {
    let Some(t) = table() else { return };
    let buckets: usize = read(r + 8);
    let n: u32 = read(r + 0x10);
    if buckets == 0 || n == 0 {
        return;
    }
    let mut e: usize = read(buckets + 8 * (a as u32 % n) as usize);
    let mut guard = 0;
    while e != 0 && guard < 4096 {
        if read::<i32>(e) == a {
            let begin: usize = read(e + 8);
            let end: usize = read(e + 0x10);
            if begin != 0 && end >= begin {
                let mut list: Vec<i32> = std::slice::from_raw_parts(begin as *const i32, (end - begin) / 4).to_vec();
                let left = remove_from(&mut list, b);
                if left < (end - begin) / 4 {
                    std::ptr::copy_nonoverlapping(list.as_ptr(), begin as *mut i32, left);
                    *((e + 0x10) as *mut usize) = begin + 4 * left;
                }
                if left == 0 {
                    let erase: unsafe extern "C-unwind" fn(usize, *const u32) -> i32 = std::mem::transmute(image(t.relay_erase));
                    let key = a as u32;
                    erase(r, &key);
                }
            }
            return;
        }
        e = read(e + 0x20);
        guard += 1;
    }
}

/// Removes the pair on both sides; true if it was in the table.
unsafe fn remove_pair(s: usize, a: i32, b: i32) -> bool {
    let Some(r) = object(s, SESSION_RELAYS) else { return false };
    let before = relayed(s, a, b) || relayed(s, b, a);
    remove_partner(r, a, b);
    remove_partner(r, b, a);
    before
}

/// The pairs of the relay table, each once (a < b).
unsafe fn pairs(s: usize) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    let Some(r) = object(s, SESSION_RELAYS) else { return out };
    let buckets: usize = read(r + 8);
    let n: u32 = read(r + 0x10);
    if buckets == 0 || n == 0 || n > 4096 {
        return out;
    }
    let end: usize = read(buckets + 8 * n as usize);
    for i in 0..n as usize {
        let mut e: usize = read(buckets + 8 * i);
        let mut guard = 0;
        while e != 0 && e != end && guard < 4096 {
            let a: i32 = read(e);
            let begin: usize = read(e + 8);
            let end_v: usize = read(e + 0x10);
            if begin != 0 && end_v >= begin && end_v - begin < 4 * 64 {
                for &b in std::slice::from_raw_parts(begin as *const i32, (end_v - begin) / 4) {
                    let p = (a.min(b), a.max(b));
                    if !out.contains(&p) {
                        out.push(p);
                    }
                }
            }
            e = read(e + 0x20);
            guard += 1;
        }
    }
    out.sort();
    out
}

/// The pairs as text for Lua: "a-b,c-d".
pub fn pairs_text(p: &[(i32, i32)]) -> String {
    p.iter().map(|(a, b)| format!("{a}-{b}")).collect::<Vec<_>>().join(",")
}

/// The pending connection at the front of the queue: (address, player, state).
unsafe fn queue_front(s: usize) -> Option<(usize, i32, i32)> {
    let t = table()?;
    let q = object(s, SESSION_QUEUE)?;
    let f: unsafe extern "C-unwind" fn(usize) -> usize = std::mem::transmute(image(t.queue_front));
    let p = f(q);
    (p != 0).then(|| (p, read(p + PENDING_PLAYER), read(p + PENDING_STATE)))
}

unsafe fn pop_front(s: usize, succeeded: bool) {
    let (Some(t), Some(q)) = (table(), object(s, SESSION_QUEUE)) else { return };
    let f: unsafe extern "C-unwind" fn(usize, u8) = std::mem::transmute(image(t.queue_pop));
    f(q, succeeded as u8);
}

/// Hooked introduction. Re-link: a player relayed with us introduces itself on a connection that no
/// player has, and its network player has no connection. Otherwise, the original handler.
unsafe extern "C-unwind" fn intro(msg: *mut u8, s: *mut c_void) {
    let s = s as usize;
    if session_valid(s) {
        SESSION.store(s, Ordering::Relaxed);
        if !msg.is_null() && relink(msg as usize, s) {
            return;
        }
    }
    let orig: Process = std::mem::transmute(ORIG_INTRO.load(Ordering::Relaxed));
    orig(msg, s as *mut c_void)
}

unsafe fn relink(msg: usize, s: usize) -> bool {
    let Some(t) = table() else { return false };
    let Some(me) = local(s) else { return false };
    let player: i32 = read(msg + MSG_PLAYER);
    let conn: u32 = read(msg + MSG_CONNECTION);
    if player < 0 || player == me || conn == 0 || !relayed(s, me, player) {
        return false;
    }
    let Some(pm) = object(s, SESSION_PLAYERS) else { return false };
    if map_contains(pm + PLAYERS_CONNECTIONS, conn) {
        return false; // arrived through the relay (the host's connection)
    }
    let find: unsafe extern "C-unwind" fn(usize, u32) -> usize = std::mem::transmute(image(t.player_find));
    let np = find(pm, player as u32);
    if np == 0 || read::<u32>(np + PLAYER_CONNECTION) != 0 {
        return false;
    }
    // As the original handler does: steamID on the connection, and our pending connection to the
    // player removed (succeeded).
    let steam: u64 = read(msg + MSG_STEAM);
    let manager: usize = read(image(t.net_manager));
    if manager != 0 {
        let connection: unsafe extern "C-unwind" fn(usize, u32) -> usize = std::mem::transmute(image(t.connection));
        let c = connection(manager + 0x88, conn);
        if c != 0 {
            let set_steam: unsafe extern "C-unwind" fn(usize, u64, u8) = std::mem::transmute(image(t.connection_set_steam));
            set_steam(c, steam, 0);
        }
    }
    if let Some((_, target, _)) = queue_front(s) {
        if target == player {
            pop_front(s, true);
        }
    }
    // The existing network player takes the connection: no replacement, no departure published.
    *((np + PLAYER_CONNECTION) as *mut u32) = conn;
    let slot: unsafe extern "C-unwind" fn(usize, *const u32) -> *mut usize = std::mem::transmute(image(t.map_slot));
    let v = slot(pm + PLAYERS_CONNECTIONS, &conn);
    if !v.is_null() {
        *v = np;
    }
    remove_pair(s, me, player);
    if (0..64).contains(&player) {
        RELINKED.fetch_or(1u64 << player, Ordering::Relaxed);
    }
    {
        let mut recent = RECENT.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();
        recent.retain(|&(c, until)| c != conn && until > now);
        recent.push((conn, now + REPEATED_MS));
    }
    log(&format!("relink: player {player} linked directly (connection {conn}), pair {me}-{player} removed from the relay"));
    true
}

/// Hooked `NetRelay`: a pair already relayed is ignored (the host sends it back when an attempt
/// fails; handling it would resend the introductions and publish a departure).
unsafe extern "C-unwind" fn relay(msg: *mut u8, s: *mut c_void) {
    let s = s as usize;
    if session_valid(s) && !msg.is_null() {
        SESSION.store(s, Ordering::Relaxed);
        let a: i32 = read(msg as usize + MSG_PLAYER);
        let b: i32 = read(msg as usize + MSG_RELAY_EXISTING);
        if relayed(s, a, b) {
            log(&format!("relink: NetRelay {a}-{b} already known, ignored"));
            return;
        }
        log(&format!("relink: NetRelay {a}-{b} (direct link failed, relayed through the host)"));
    }
    let orig: Process = std::mem::transmute(ORIG_RELAY.load(Ordering::Relaxed));
    orig(msg, s as *mut c_void)
}

/// A repeated message (`REPEATED[i]`) that arrived on a re-linked connection less than
/// `REPEATED_MS` ago?
unsafe fn is_repeat(msg: *mut u8) -> bool {
    if msg.is_null() {
        return false;
    }
    let conn: u32 = read(msg as usize + MSG_CONNECTION);
    let now = now_ms();
    RECENT.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|&(c, until)| c == conn && now < until)
}

unsafe fn process_repeated(i: usize, msg: *mut u8, s: *mut c_void) {
    if is_repeat(msg) {
        log(&format!("relink: {} repeated on the re-linked connection, dropped", REPEATED[i]));
        return;
    }
    let orig: Process = std::mem::transmute(ORIG_REPEATED[i].load(Ordering::Relaxed));
    orig(msg, s)
}
unsafe extern "C-unwind" fn repeated_register(msg: *mut u8, s: *mut c_void) {
    process_repeated(0, msg, s)
}
unsafe extern "C-unwind" fn repeated_app_register(msg: *mut u8, s: *mut c_void) {
    process_repeated(1, msg, s)
}
unsafe extern "C-unwind" fn repeated_snapshot(msg: *mut u8, s: *mut c_void) {
    process_repeated(2, msg, s)
}

/// Where an attempt stands, seen from the front of the queue and the relay table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    InProgress,
    Relinked,
    Failed,
}

/// `front`: player and state of the pending connection at the front of the queue; `still_relayed`:
/// the pair is still in the table; `relinked`: the introduction was handled by `relink`.
pub fn state(partner: i32, front: Option<(i32, i32)>, still_relayed: bool, relinked: bool, elapsed_ms: u64) -> State {
    if relinked {
        return State::Relinked;
    }
    if !still_relayed {
        // Out of the table without our re-link: the partner left.
        return State::Failed;
    }
    match front {
        Some((p, s)) if p == partner && s == STATE_FAILED => State::Failed,
        Some((p, _)) if p == partner && elapsed_ms < ATTEMPT_MAX_MS => State::InProgress,
        Some((p, _)) if p == partner => State::Failed,
        _ if elapsed_ms < 2_000 => State::InProgress,
        _ => State::Failed,
    }
}

/// Opens a pending connection to `partner`, as when a player arrives.
unsafe fn start(partner: i32, steam: u64) -> Result<(), String> {
    let t = table().ok_or("unknown exe")?;
    let s = session().ok_or("no session")?;
    let me = local(s).ok_or("no local player")?;
    if partner < 0 || partner == me {
        return Err(format!("invalid partner {partner}"));
    }
    if !relayed(s, me, partner) {
        return Err(format!("{me}-{partner} is not relayed"));
    }
    if steam == 0 {
        return Err("unknown steamID".into());
    }
    let mut attempt = ATTEMPT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(current) = attempt.as_ref() {
        if now_ms().saturating_sub(current.start_ms) < ATTEMPT_MAX_MS {
            return Err(format!("attempt to {} in progress", current.partner));
        }
    }
    // A failed connection left at the front (when the player arrived, or our previous attempt)
    // would block the queue: pop it, like the answer the host does not send.
    if let Some((_, p, st)) = queue_front(s) {
        if st == STATE_FAILED {
            pop_front(s, false);
            log(&format!("relink: failed connection to {p} popped from the front of the queue"));
        } else {
            return Err(format!("queue busy (player {p}, state {st})"));
        }
    }
    let mut msg = [0u8; MSG_SIZE];
    std::ptr::write_unaligned(msg.as_mut_ptr().add(MSG_PLAYER) as *mut i32, partner);
    std::ptr::write_unaligned(msg.as_mut_ptr().add(MSG_STEAM) as *mut u64, steam);
    RELINKED.fetch_and(!(1u64 << (partner & 63)), Ordering::Relaxed);
    *attempt = Some(Attempt { partner, start_ms: now_ms() });
    drop(attempt);
    let f: Process = std::mem::transmute(image(t.peer_info.1[PROCESS]));
    f(msg.as_mut_ptr(), s as *mut c_void);
    log(&format!("relink: direct link attempt {me}-{partner} (steamID {steam})"));
    Ok(())
}

/// The state of the attempt to `partner`; a failed connection at the front is popped.
unsafe fn follow(partner: i32) -> State {
    let Some(s) = session() else { return State::Failed };
    let Some(me) = local(s) else { return State::Failed };
    let started = ATTEMPT.lock().unwrap_or_else(|e| e.into_inner()).as_ref().filter(|a| a.partner == partner).map(|a| a.start_ms);
    let Some(started) = started else { return State::Failed };
    let front = queue_front(s);
    let relinked = RELINKED.load(Ordering::Relaxed) & (1u64 << (partner & 63)) != 0;
    let st = state(partner, front.map(|(_, p, st)| (p, st)), relayed(s, me, partner), relinked, now_ms().saturating_sub(started));
    if st != State::InProgress {
        if st == State::Failed {
            if let Some((_, p, _)) = front {
                if p == partner {
                    pop_front(s, false);
                }
            }
        }
        *ATTEMPT.lock().unwrap_or_else(|e| e.into_inner()) = None;
        log(&format!("relink: attempt {me}-{partner}: {}", if st == State::Relinked { "direct link" } else { "failed, the pair stays relayed" }));
    }
    st
}

/// `Civ6Proxy_RelayedPairs()` → the game's relayed pairs, "a-b,c-d" (empty when none).
pub(crate) unsafe extern "C-unwind" fn lua_pairs(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let text = session().map(|s| pairs_text(&pairs(s))).unwrap_or_default();
    crate::options::push_str(h, l, &text);
    1
}

fn arg_int(l: *mut c_void, i: i32) -> Option<i64> {
    // SAFETY: reads a Lua argument through the game's API.
    unsafe { crate::salon::arg_str(l, i) }.and_then(|v| v.trim().split('.').next().and_then(|x| x.parse().ok()))
}

/// `Civ6Proxy_RelinkStart(partner, steamID)` → 1 if the attempt started, otherwise 0.
pub(crate) unsafe extern "C-unwind" fn lua_start(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let partner = arg_int(l, 1).unwrap_or(-1) as i32;
    let steam = crate::salon::arg_str(l, 2).and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0);
    let r = start(partner, steam);
    if let Err(e) = &r {
        log(&format!("relink: no attempt to {partner}: {e}"));
    }
    (h.push_integer)(l, r.is_ok() as i32);
    1
}

/// `Civ6Proxy_RelinkState(partner)` → 0 in progress, 1 linked directly, 2 failed.
pub(crate) unsafe extern "C-unwind" fn lua_state(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let partner = arg_int(l, 1).unwrap_or(-1) as i32;
    let st = match follow(partner) {
        State::InProgress => 0,
        State::Relinked => 1,
        State::Failed => 2,
    };
    (h.push_integer)(l, st);
    1
}

/// `Civ6Proxy_RelayForgetPair(a, b)` → 1 if the pair was in the local table (removed), otherwise 0.
/// Never a pair we belong to: the announcement can arrive before the partner's introduction, and
/// `relink` would no longer recognise the re-link (Civ6's own path would publish a departure). Our
/// pair leaves the table when the introduction arrives.
pub(crate) unsafe extern "C-unwind" fn lua_forget(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let (Some(a), Some(b)) = (arg_int(l, 1), arg_int(l, 2)) else { return 0 };
    let third_party = |s: usize| local(s).is_some_and(|me| me != a as i32 && me != b as i32);
    let removed = session().is_some_and(|s| third_party(s) && remove_pair(s, a as i32, b as i32));
    if removed {
        log(&format!("relink: pair {a}-{b} removed (announced as linked)"));
    }
    (h.push_integer)(l, removed as i32);
    1
}

/// Are all the table's bytes there? `read(address, n)` reads the image, `entry(address)` a vtable
/// entry brought back to base `0x140000000`. Shared by the loaded exe and the files (tests).
fn fingerprint<'a>(t: &Exe, read: impl Fn(u64, usize) -> &'a [u8], entry: impl Fn(u64) -> u64) -> Result<(), String> {
    let vtables = [
        ("NetIntroductionSteam", &t.intro),
        ("NetRelay", &t.relay),
        ("NetPeerInfoSteam", &t.peer_info),
        (REPEATED[0], &t.repeated[0]),
        (REPEATED[1], &t.repeated[1]),
        (REPEATED[2], &t.repeated[2]),
    ];
    for (name, (vtable, entries)) in vtables {
        for (k, expected) in entries.iter().enumerate() {
            if entry(vtable + 8 * k as u64) != *expected {
                return Err(format!("{name}: unexpected vtable at {vtable:#x}"));
            }
        }
    }
    let functions = [t.queue_front, t.queue_pop, t.connection, t.connection_set_steam, t.player_find, t.map_slot, t.relay_erase, t.relay_contains];
    for ((name, bytes), address) in PROLOGUES.iter().zip(functions) {
        if read(address, bytes.len()) != bytes {
            return Err(format!("{name}: unexpected bytes at {address:#x}"));
        }
    }
    // `mov rcx,[rip+d]` in the introduction handler: it reads the network manager.
    let at = t.intro.1[PROCESS] + t.net_manager_read_at;
    let m = read(at, 7);
    if m.len() < 7 || m[..3] != [0x48, 0x8b, 0x0d] {
        return Err(format!("network manager read missing at {at:#x}"));
    }
    let d = i32::from_le_bytes(m[3..7].try_into().unwrap());
    if (at + 7).wrapping_add_signed(i64::from(d)) != t.net_manager {
        return Err(format!("network manager: does not lead to {:#x}", t.net_manager));
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

/// Replaces the "process" entry of a message vtable with `hook`, keeping the original in `orig`.
unsafe fn hook_slot(module: usize, vtable: u64, hook: usize, orig: &AtomicUsize) -> Result<(), String> {
    let slot = (module + (vtable - BASE) as usize + 8 * PROCESS) as *mut usize;
    let mut old = 0u32;
    if win::VirtualProtect(slot as *mut c_void, 8, win::PAGE_EXECUTE_READWRITE, &mut old) == 0 {
        return Err("VirtualProtect failed".into());
    }
    orig.store(*slot, Ordering::Relaxed);
    *slot = hook;
    win::VirtualProtect(slot as *mut c_void, 8, old, &mut old);
    Ok(())
}

// Test mode (diagnostic builds only): `relink_test=<player>` in the rollover_proxy.cfg of the
// player who joins makes their link to that player fail, through Civ6's real failure path (the
// pending connection goes to state 10: the queue calls the failure, `NetPeerFailure`, the host
// relays the pair). A way to try the re-link without waiting for a real failure. Once per launch;
// never the link to the host (its failure would cancel the join).
#[cfg(feature = "diag")]
static TEST_TARGET: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);
#[cfg(feature = "diag")]
static ORIG_PEER_INFO: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "diag")]
const STATE_ABANDONED: i32 = 10;
#[cfg(feature = "diag")]
const PENDING_IS_HOST: usize = 0x58;

/// The `relink_test` setting (last occurrence wins): the player whose link must fail.
#[cfg_attr(not(feature = "diag"), allow(dead_code))]
pub fn test_target(cfg: &str) -> Option<i32> {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("relink_test=")).last().and_then(|v| v.trim().parse().ok()).filter(|&p| p >= 0)
}

#[cfg(feature = "diag")]
unsafe extern "C-unwind" fn peer_info(msg: *mut u8, s: *mut c_void) {
    let orig: Process = std::mem::transmute(ORIG_PEER_INFO.load(Ordering::Relaxed));
    orig(msg, s);
    let target = TEST_TARGET.load(Ordering::Relaxed);
    let s = s as usize;
    if target < 0 || msg.is_null() || read::<i32>(msg as usize + MSG_PLAYER) != target || !session_valid(s) {
        return;
    }
    let Some(q) = object(s, SESSION_QUEUE) else { return };
    let (begin, end): (usize, usize) = (read(q + 0x10), read(q + 0x18));
    if end < begin + 8 {
        return;
    }
    let pending: usize = read(end - 8);
    if pending == 0 || read::<i32>(pending + PENDING_PLAYER) != target || read::<u8>(pending + PENDING_IS_HOST) != 0 {
        return;
    }
    *((pending + PENDING_STATE) as *mut i32) = STATE_ABANDONED;
    TEST_TARGET.store(-1, Ordering::Relaxed);
    log(&format!("relink: test: link to player {target} marked failed, the host will relay the pair"));
}

/// The `relink` setting in `rollover_proxy.cfg` (last occurrence wins): `off` installs nothing.
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("relink=")).last().map(str::trim) != Some("off")
}

/// Called by DllMain: no exe code has run yet.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("relink: not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log("relink: turned off (relink=off)");
        return;
    }
    let module = win::GetModuleHandleW(std::ptr::null()) as usize;
    let mut reasons = Vec::new();
    let Some(i) = EXES.iter().position(|t| verify(module, t).map_err(|e| reasons.push(format!("{}: {e}", t.name))).is_ok()) else {
        log(&format!("relink: unknown exe ({}), re-link not installed", reasons.join("; ")));
        return;
    };
    MODULE.store(module, Ordering::Relaxed);
    TABLE.store(i, Ordering::Relaxed);
    let t = &EXES[i];
    let repeated: [Process; 3] = [repeated_register, repeated_app_register, repeated_snapshot];
    let r = hook_slot(module, t.relay.0, relay as Process as usize, &ORIG_RELAY)
        .and_then(|_| (0..3).try_for_each(|i| hook_slot(module, t.repeated[i].0, repeated[i] as usize, &ORIG_REPEATED[i])))
        .and_then(|_| hook_slot(module, t.intro.0, intro as Process as usize, &ORIG_INTRO));
    match r {
        Ok(()) => log(&format!("relink: re-link of relayed pairs installed ({})", t.name)),
        Err(e) => {
            log(&format!("relink: {e}, re-link not installed"));
            return;
        }
    }
    #[cfg(feature = "diag")]
    if let Some(target) = test_target(&cfg) {
        TEST_TARGET.store(target, Ordering::Relaxed);
        match hook_slot(module, t.peer_info.0, peer_info as Process as usize, &ORIG_PEER_INFO) {
            Ok(()) => log(&format!("relink: test: the link to player {target} will fail on arrival")),
            Err(e) => log(&format!("relink: test: {e}")),
        }
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

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("relink=off\r\n"));
        assert!(enabled("relink=off\nrelink=on\n"), "the last occurrence wins");
    }

    #[test]
    fn test_setting() {
        assert_eq!(test_target(""), None);
        assert_eq!(test_target("relink_test=8\r\n"), Some(8));
        assert_eq!(test_target("relink_test=8\nrelink_test=\n"), None, "emptied: no more test");
        assert_eq!(test_target("relink_test=-1\n"), None);
    }

    #[test]
    fn pairs_as_text() {
        assert_eq!(pairs_text(&[]), "");
        assert_eq!(pairs_text(&[(6, 8), (7, 8)]), "6-8,7-8");
    }

    #[test]
    fn removing_a_partner() {
        let mut v = vec![8, 3, 8];
        assert_eq!(remove_from(&mut v, 8), 1);
        assert_eq!(v, vec![3]);
        assert_eq!(remove_from(&mut v, 5), 1, "missing: nothing changes");
        assert_eq!(remove_from(&mut v, 3), 0);
    }

    #[test]
    fn following_an_attempt() {
        // Introduction handled by `relink`: linked.
        assert_eq!(state(8, Some((8, 2)), true, true, 1_000), State::Relinked);
        assert_eq!(state(8, None, false, true, 1_000), State::Relinked);
        // Pair out of the table without a re-link (partner left): failed, not "fixed".
        assert_eq!(state(8, None, false, false, 1_000), State::Failed);
        // At the front, in progress.
        assert_eq!(state(8, Some((8, 2)), true, false, 5_000), State::InProgress);
        // At the front, failed (transport: 20 s without an answer).
        assert_eq!(state(8, Some((8, STATE_FAILED)), true, false, 21_000), State::Failed);
        // Too long.
        assert_eq!(state(8, Some((8, 2)), true, false, ATTEMPT_MAX_MS), State::Failed);
        // No longer at the front and still relayed: removed without a link (the host's answer).
        assert_eq!(state(8, None, true, false, 3_000), State::Failed);
        // Just started: the queue may not have taken it yet.
        assert_eq!(state(8, None, true, false, 500), State::InProgress);
    }
}
