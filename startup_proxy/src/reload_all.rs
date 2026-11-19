//! When a client's game has objects the host's has not (or the other way round), the game reloads
//! that client alone, and a game loaded alone drifts away from the running ones (state the sync
//! check does not cover). Instead, at that same turn-end sync check, the host reloads everyone.
//!
//! The host starts the reload itself (`start`, called by `partie.lua`): it sends the game's own
//! "this player waits for the game" request for every player in one go, itself last, so the game
//! makes one copy and sends it to all of them. The turn then starts only once every player has
//! loaded that copy and its check of the loaded game has been compared:
//! - the host forgets who had already answered the turn-end sync round, its own answer included,
//!   so the round stays open and checks wait until the host has loaded too;
//! - until a reloading player says its load is over, the host drops that player's answers and
//!   checks (they are from before its load: a client answers the old round as soon as its copy
//!   arrives), and its own request for a copy, which would send it a second one;
//! - every machine ignores the game orders of a reloading player: that player never applies them
//!   itself, and the copy does not hold them;
//! - a reloading client keeps the check of its loaded game (`rejoin.rs`): the host waits for it;
//! - a player leaving ends the wait for it (`recheck`); the game only looks again when an answer
//!   arrives.
//! A check that still finds objects on one side after the reload starts one more reload of that
//! turn, once nobody is loading the first copy any more; otherwise, and after that second reload,
//! the game's own reload of that player is left alone.
//!
//! The host only does this when every player runs this version of BGE (the lobby's verdict): a
//! client of an older version might skip the check the host now waits for. A client whose hooks
//! here could not be installed keeps every check after a load, for the same reason.
//!
//! Needs `rejoin.rs` (session, barrier) and, on the host, `sync_notes.rs`. Settings: `reload_all=off`
//! keeps the game's own reload when this machine hosts. Addresses of both exes of 2026-11-05, checked at startup.

use crate::exe::rd;
use crate::{log, patch, win};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

/// Base of the exe's Ghidra addresses.
const BASE: u64 = 0x1_4000_0000;

struct Exe {
    name: &'static str,
    /// NetSnapshotRequest: vtable, process, constructor (message, player, reason).
    request_vtable: u64,
    request_process: u64,
    request_ctor: u64,
    /// NetGameSyncProcessed (a player's answer to the sync round): vtable, process.
    ack_vtable: u64,
    ack_process: u64,
    /// NetGameSyncCheck (a client's check): vtable, process.
    check_vtable: u64,
    check_process: u64,
    /// In the session's receive: the call that tests "the local player waits for its copy", and
    /// that test. The message is in r15 there.
    ignore_call: u64,
    waiting_test: u64,
}

const EXES: [Exe; 4] = [
    Exe {
        name: "DX12",
        request_vtable: 0x1_4105_0c30,
        request_process: 0x1_4081_10a0,
        request_ctor: 0x1_4080_dfe0,
        ack_vtable: 0x1_4105_0ca8,
        ack_process: 0x1_4080_fc20,
        check_vtable: 0x1_4104_ea30,
        check_process: 0x1_4080_fb50,
        ignore_call: 0x1_4084_3cd8,
        waiting_test: 0x1_4084_16f0,
    },
    Exe {
        name: "DX11",
        request_vtable: 0x1_4104_17c0,
        request_process: 0x1_4080_f330,
        request_ctor: 0x1_4080_c2c0,
        ack_vtable: 0x1_4104_1838,
        ack_process: 0x1_4080_deb0,
        check_vtable: 0x1_4103_f5a0,
        check_process: 0x1_4080_dde0,
        ignore_call: 0x1_4084_20f8,
        waiting_test: 0x1_4083_fb10,
    },
    Exe {
        name: "DX12 1.0.12.74",
        request_vtable: 0x1_4105_0cd0,
        request_process: 0x1_4081_10e0,
        request_ctor: 0x1_4080_e020,
        ack_vtable: 0x1_4105_0d48,
        ack_process: 0x1_4080_fc60,
        check_vtable: 0x1_4104_ead0,
        check_process: 0x1_4080_fb90,
        ignore_call: 0x1_4084_3fb8,
        waiting_test: 0x1_4084_19d0,
    },
    Exe {
        name: "DX11 1.0.12.74",
        request_vtable: 0x1_4104_1790,
        request_process: 0x1_4081_0060,
        request_ctor: 0x1_4080_cfc0,
        ack_vtable: 0x1_4104_1808,
        ack_process: 0x1_4080_ebe0,
        check_vtable: 0x1_4103_f590,
        check_process: 0x1_4080_eb10,
        ignore_call: 0x1_4084_1c88,
        waiting_test: 0x1_4083_f6a0,
    },
];

/// The "process" entry of message vtables (destructor, text, write, read, process).
const PROCESS: u64 = 4;
/// Starts of the three process functions, of the constructor and of the waiting test.
const REQUEST_START: [u8; 16] = [0x48, 0x85, 0xd2, 0x74, 0x60, 0x48, 0x89, 0x5c, 0x24, 0x10, 0x57, 0x48, 0x83, 0xec, 0x20, 0x48];
const ACK_START: [u8; 16] = [0x48, 0x83, 0xec, 0x28, 0x4c, 0x8b, 0xc2, 0x8b, 0x51, 0x38, 0x83, 0xfa, 0xff, 0x74, 0x27, 0x49];
const CHECK_START: [u8; 16] = [0x40, 0x55, 0x48, 0x8d, 0x6c, 0x24, 0xa9, 0x48, 0x81, 0xec, 0x90, 0x00, 0x00, 0x00, 0x0f, 0xb6];
const WAITING_START: [u8; 16] = [0x48, 0x89, 0x5c, 0x24, 0x08, 0x57, 0x48, 0x83, 0xec, 0x20, 0x48, 0x8b, 0x19, 0x48, 0x8b, 0xf9];
/// The constructor up to `mov edx,4e7h` (the message type), then `lea rax,[vtable]` at + 0x24.
const CTOR_START: [u8; 28] = [
    0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xec, 0x20, 0x41, 0x8b, 0xf0, 0x8b, 0xda, 0x45, 0x33,
    0xc0, 0xba, 0xe7, 0x04, 0x00, 0x00,
];
const CTOR_VTABLE_AT: usize = 0x24;
/// Before the call: `call [rbx+158h] ; mov rcx,r14`. After it: `test al,al ; je ; cmp byte [r15+8],0 ;
/// jne ; test byte [r15+0Ch],4`.
const IGNORE_BEFORE: [u8; 9] = [0xff, 0x93, 0x58, 0x01, 0x00, 0x00, 0x49, 0x8b, 0xce];
const IGNORE_AFTER: [u8; 24] = [
    0x84, 0xc0, 0x0f, 0x84, 0x41, 0x03, 0x00, 0x00, 0x41, 0x80, 0x7f, 0x08, 0x00, 0x0f, 0x85, 0x36, 0x03, 0x00, 0x00, 0x41, 0xf6, 0x47,
    0x0c, 0x04,
];

// Message: type, sender, first field. Requests: player, reason. Answers: player. Checks: test flag.
const MSG_TYPE: usize = 0x18;
const MSG_SENDER: usize = 0x24;
const MSG_PLAYER: usize = 0x38;
const MSG_REASON: usize = 0x3c;
const MSG_TEST: usize = 0x38;
/// Type of the message that carries a game order.
const TYPE_GAME_ORDER: u32 = 3;
/// Reason of a request a player makes for itself, and the one used here.
const REASON_RESYNC: i32 = 0;
// Session: broadcast, local player, host player, network player by ID.
const SESSION_BROADCAST: usize = 0xb8;
const SESSION_LOCAL: usize = 0x148;
const SESSION_HOST: usize = 0x150;
const SESSION_PLAYER: usize = 0x158;
// Network player: waits for the game, the game was sent to it, has answered the sync round.
const PLAYER_WAITING: usize = 0x1a;
const PLAYER_SENT: usize = 0x1b;
const PLAYER_ANSWERED: usize = 0x1f;
// Sync manager: its "an answer arrived" entry, and "a sync round is running".
const SYNC_ON_ANSWER: usize = 0x30;
const SYNC_ROUND: usize = 0x3c;
const MAX_PLAYERS: i32 = 64;
/// Reloads of everyone started for one turn before the game's own reload is left alone.
const MAX_PER_TURN: u32 = 2;

/// `start` and `state` results, for `partie.lua`.
const NOT_POSSIBLE: i32 = 0;
const CAN_START: i32 = 1;
const RUNNING: i32 = 2;
const GAVE_UP: i32 = 3;
const OTHERS_RELOADING: i32 = 4;
const MIXED_VERSIONS: i32 = 5;

static TABLE: AtomicUsize = AtomicUsize::new(usize::MAX);
static MODULE: AtomicUsize = AtomicUsize::new(0);
static REQUEST_ORIG: AtomicUsize = AtomicUsize::new(0);
static ACK_ORIG: AtomicUsize = AtomicUsize::new(0);
static CHECK_ORIG: AtomicUsize = AtomicUsize::new(0);
static WAITING_ORIG: AtomicUsize = AtomicUsize::new(0);
/// Players reloading in a reload of everyone (one bit per player ID).
static RELOADING: AtomicU64 = AtomicU64::new(0);
/// Among them, those whose request this machine has processed: their "waits for the game" mark is
/// up until their load is over.
static MARKED: AtomicU64 = AtomicU64::new(0);
/// This machine reloads because the host reloads everyone: the check of the game it loads is kept
/// (`rejoin.rs`), however long the load takes.
static KEEP: AtomicBool = AtomicBool::new(false);
/// The turn of the last reload of everyone, and how many were started for it.
static LAST: Mutex<(i32, u32)> = Mutex::new((-1, 0));

type Process = unsafe extern "system" fn(usize, usize);
type WaitingTest = unsafe extern "system" fn(usize) -> u8;
type WaitingTestHook = unsafe extern "system" fn(usize, usize) -> u8;

/// The `reload_all` setting (last occurrence wins; `on` by default).
fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("reload_all=")).last().map(str::trim) != Some("off")
}

/// The `reload_orders` setting: ignore the game orders of a reloading player (`on` by default).
fn orders_enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("reload_orders=")).last().map(str::trim) != Some("off")
}

fn installed() -> bool {
    TABLE.load(Ordering::Relaxed) != usize::MAX
}

/// Can this machine, as host, reload everyone? It must see the objects on one side only.
fn host_ready(cfg: &str) -> bool {
    !crate::salon::actif_off(cfg) && enabled(cfg) && crate::sync_notes::installed() && installed()
}

fn bit(player: i32) -> u64 {
    if (0..MAX_PLAYERS).contains(&player) {
        1 << player
    } else {
        0
    }
}

/// Which reload of a turn this would be, or `None` after `MAX_PER_TURN`.
fn attempt(last: (i32, u32), turn: i32) -> Option<u32> {
    let n = if last.0 == turn { last.1 + 1 } else { 1 };
    (n <= MAX_PER_TURN).then_some(n)
}

/// Is this player still reloading? `waiting` is its "waits for the game" mark, `None` when it has
/// left. Forgets a player whose load is over.
fn still_reloading(player: i32, waiting: Option<bool>) -> bool {
    let b = bit(player);
    if RELOADING.load(Ordering::Relaxed) & b == 0 {
        return false;
    }
    let over = match waiting {
        None => true,
        Some(w) => !w && MARKED.load(Ordering::Relaxed) & b != 0,
    };
    if over {
        RELOADING.fetch_and(!b, Ordering::Relaxed);
        MARKED.fetch_and(!b, Ordering::Relaxed);
    }
    !over
}

unsafe fn method<F: Copy>(object: usize, offset: usize) -> F {
    std::mem::transmute_copy(&rd::<usize>(rd::<usize>(object) + offset))
}

unsafe fn local_player(session: usize) -> i32 {
    method::<unsafe extern "system" fn(usize) -> i32>(session, SESSION_LOCAL)(session)
}

unsafe fn host_player(session: usize) -> i32 {
    method::<unsafe extern "system" fn(usize) -> i32>(session, SESSION_HOST)(session)
}

unsafe fn net_player(session: usize, player: i32) -> usize {
    method::<unsafe extern "system" fn(usize, i32) -> usize>(session, SESSION_PLAYER)(session, player)
}

unsafe fn reloading(session: usize, player: i32) -> bool {
    if RELOADING.load(Ordering::Relaxed) & bit(player) == 0 {
        return false;
    }
    let p = net_player(session, player);
    still_reloading(player, (p != 0).then(|| rd::<u8>(p + PLAYER_WAITING) != 0))
}

/// Can the host reload everyone now, for `player`'s request? Only inside its turn-end sync round,
/// and not while other players are still loading the last copy.
unsafe fn state(turn: i32, player: i32) -> i32 {
    if !installed() {
        return NOT_POSSIBLE;
    }
    let Some((session, sync, true)) = crate::rejoin::network() else { return NOT_POSSIBLE };
    if rd::<u8>(sync + SYNC_ROUND) == 0 {
        return NOT_POSSIBLE;
    }
    // A client of an older version may not send the check the host would now wait for.
    if !crate::salon::pol_actifs() {
        return MIXED_VERSIONS;
    }
    if reloading(session, player) {
        return RUNNING;
    }
    if (0..MAX_PLAYERS).any(|p| reloading(session, p)) {
        return OTHERS_RELOADING;
    }
    match attempt(*LAST.lock().unwrap_or_else(|e| e.into_inner()), turn) {
        Some(_) => CAN_START,
        None => GAVE_UP,
    }
}

/// The host reloads everyone: one request per player, itself last. A client that already got its
/// copy keeps the game's own reload.
unsafe fn start(turn: i32, player: i32) -> i32 {
    let s = state(turn, player);
    if s != CAN_START {
        return s;
    }
    let Some(t) = EXES.get(TABLE.load(Ordering::Relaxed)) else { return NOT_POSSIBLE };
    let Some((session, _, true)) = crate::rejoin::network() else { return NOT_POSSIBLE };
    let local = local_player(session);
    let ctor: unsafe extern "system" fn(*mut u64, i32, i32) -> usize = std::mem::transmute(MODULE.load(Ordering::Relaxed) + (t.request_ctor - BASE) as usize);
    let broadcast: unsafe extern "system" fn(usize, *mut u64, u32, u32) = method(session, SESSION_BROADCAST);
    let (mut asked, mut alone) = (Vec::new(), Vec::new());
    for player in (0..MAX_PLAYERS).filter(|p| *p != local).chain([local]) {
        let p = net_player(session, player);
        if p == 0 {
            continue;
        }
        if player != local && rd::<u8>(p + PLAYER_WAITING) != 0 && rd::<u8>(p + PLAYER_SENT) != 0 {
            alone.push(player.to_string());
            continue;
        }
        RELOADING.fetch_or(bit(player), Ordering::Relaxed);
        MARKED.fetch_and(!bit(player), Ordering::Relaxed);
        *((p + PLAYER_ANSWERED) as *mut u8) = 0;
        let mut message = [0u64; 8];
        ctor(message.as_mut_ptr(), player, REASON_RESYNC);
        broadcast(session, message.as_mut_ptr(), 1, 0);
        asked.push(player.to_string());
    }
    let n = {
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        *last = (turn, attempt(*last, turn).unwrap_or(MAX_PER_TURN));
        last.1
    };
    let alone = if alone.is_empty() { String::new() } else { format!("; already loading alone: {}", alone.join(", ")) };
    log(&format!("reload: turn {turn}, reload {n} of everyone started for player(s) {}{alone}", asked.join(", ")));
    CAN_START
}

/// A player left: the host looks again whether everyone still there has answered the sync round.
unsafe fn recheck() {
    if !installed() {
        return;
    }
    let Some((_, sync, true)) = crate::rejoin::network() else { return };
    if rd::<u8>(sync + SYNC_ROUND) != 0 {
        method::<unsafe extern "system" fn(usize)>(sync, SYNC_ON_ANSWER)(sync);
    }
}

/// A sync round is over on this machine (`rejoin.rs`). On the host every player has answered, so
/// nobody is reloading; on a client, the same at a turn end.
pub fn round_done(host: bool, after_turn_end: bool) {
    KEEP.store(false, Ordering::Relaxed);
    if host || after_turn_end {
        RELOADING.store(0, Ordering::Relaxed);
        MARKED.store(0, Ordering::Relaxed);
    }
}

/// Is the check of the game this machine loads next owed to a host that reloads everyone? Read once.
pub fn take_keep() -> bool {
    KEEP.swap(false, Ordering::Relaxed)
}

/// What a machine does with a request for `player`'s copy sent by `sender`.
#[derive(Debug, PartialEq, Eq)]
enum Request {
    /// Not part of a reload of everyone.
    Plain,
    /// The host's request for a player: that player reloads; `keep` when it is this machine.
    Reload { keep: bool },
    /// A reloading player's own request, on the host: it would get a second copy.
    Drop,
}

fn classify(host: bool, local: i32, host_id: i32, sender: i32, player: i32, reason: i32, reloading: bool) -> Request {
    if host {
        return match (reloading, sender == local) {
            (true, true) => Request::Reload { keep: false },
            (true, false) if sender == player => Request::Drop,
            _ => Request::Plain,
        };
    }
    if sender == host_id && reason == REASON_RESYNC && (player != host_id || reloading) {
        return Request::Reload { keep: player == local };
    }
    Request::Plain
}

/// Replaces `NetSnapshotRequest::Process`.
unsafe extern "system" fn on_request(msg: usize, session: usize) {
    let original: Process = std::mem::transmute(REQUEST_ORIG.load(Ordering::Relaxed));
    if session == 0 {
        return original(msg, session);
    }
    let (sender, player, reason) = (rd::<i32>(msg + MSG_SENDER), rd::<i32>(msg + MSG_PLAYER), rd::<i32>(msg + MSG_REASON));
    let (local, host_id) = (local_player(session), host_player(session));
    match classify(local == host_id, local, host_id, sender, player, reason, reloading(session, player)) {
        Request::Plain => original(msg, session),
        Request::Drop => log(&format!("reload: player {player}'s own request dropped, its copy is on its way")),
        Request::Reload { keep } => {
            // On a client, the host reloads too: its own request follows.
            RELOADING.fetch_or(bit(player) | bit(host_id), Ordering::Relaxed);
            original(msg, session);
            MARKED.fetch_or(bit(player), Ordering::Relaxed);
            if keep {
                KEEP.store(true, Ordering::Relaxed);
                log("reload: the host reloads everyone; the check after this game's load is kept");
            }
        }
    }
}

/// Replaces `NetGameSyncProcessed::Process`: on the host, not from a player still reloading.
unsafe extern "system" fn on_ack(msg: usize, session: usize) {
    let player = rd::<i32>(msg + MSG_PLAYER);
    if session != 0 && local_player(session) == host_player(session) && player != local_player(session) && reloading(session, player) {
        log(&format!("reload: player {player}'s answer from before its load dropped"));
        return;
    }
    let original: Process = std::mem::transmute(ACK_ORIG.load(Ordering::Relaxed));
    original(msg, session)
}

/// Replaces `NetGameSyncCheck::Process`: on the host, not from a player still reloading.
unsafe extern "system" fn on_check(msg: usize, session: usize) {
    let sender = rd::<i32>(msg + MSG_SENDER);
    if session != 0
        && rd::<u8>(msg + MSG_TEST) == 0
        && local_player(session) == host_player(session)
        && sender != local_player(session)
        && reloading(session, sender)
    {
        log(&format!("reload: player {sender}'s check from before its load dropped"));
        return;
    }
    let original: Process = std::mem::transmute(CHECK_ORIG.load(Ordering::Relaxed));
    original(msg, session)
}

/// Replaces the receive path's "the local player waits for its copy" test, which makes the game
/// ignore a game message: also a game order from a player that reloads and whose request has been
/// seen here.
unsafe extern "system" fn on_waiting_test(session: usize, msg: usize) -> u8 {
    let original: WaitingTest = std::mem::transmute(WAITING_ORIG.load(Ordering::Relaxed));
    let waiting = original(session);
    if waiting != 0 || msg == 0 || rd::<u32>(msg + MSG_TYPE) != TYPE_GAME_ORDER {
        return waiting;
    }
    let sender = rd::<i32>(msg + MSG_SENDER);
    if MARKED.load(Ordering::Relaxed) & bit(sender) != 0 && reloading(session, sender) {
        log(&format!("reload: game order from player {sender} ignored, sent before its load"));
        return 1;
    }
    0
}

/// The integer argument at `index`.
unsafe fn int_arg(l: *mut c_void, index: i32) -> Option<i32> {
    let h = crate::options::hks()?;
    let mut len = 0usize;
    let text = (h.to_lstring)(l, index, &mut len);
    if text.is_null() {
        return None;
    }
    std::str::from_utf8(std::slice::from_raw_parts(text.cast::<u8>(), len)).ok()?.trim().parse::<f64>().ok().map(|t| t as i32)
}

/// (turn, player) of `Civ6Proxy_ReloadAllState` and `Civ6Proxy_ReloadAllStart`.
unsafe fn args(l: *mut c_void) -> Option<(i32, i32)> {
    Some((int_arg(l, 1)?, int_arg(l, 2)?))
}

/// `Civ6Proxy_ReloadAllReady()` → 1 if this machine, as host, reloads everyone after a desync.
pub(crate) unsafe extern "C-unwind" fn lua_ready(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    (h.push_integer)(l, host_ready(&cfg) as i32);
    1
}

/// `Civ6Proxy_ReloadAllState(turn, player)`, for that player's request → 0 not possible now, 1 can
/// start, 2 the player is in the reload that is running, 3 this turn's reloads did not hold, 4 other
/// players are still loading the last copy, 5 not every player has this version of BGE.
pub(crate) unsafe extern "C-unwind" fn lua_state(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    (h.push_integer)(l, args(l).map_or(NOT_POSSIBLE, |(turn, player)| state(turn, player)));
    1
}

/// `Civ6Proxy_ReloadAllStart(turn, player)` → same values; 1 when the reload of everyone was started.
pub(crate) unsafe extern "C-unwind" fn lua_start(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    (h.push_integer)(l, args(l).map_or(NOT_POSSIBLE, |(turn, player)| start(turn, player)));
    1
}

/// `Civ6Proxy_ReloadAllRecheck()`: a player left.
pub(crate) unsafe extern "C-unwind" fn lua_recheck(_l: *mut c_void) -> i32 {
    recheck();
    0
}

/// Checks this table against an exe; `read(address, n)` returns n bytes (fewer when out of the
/// image), `delta` is added to the addresses stored in the image (0 in the file).
fn fingerprint<'a>(t: &Exe, delta: u64, read: impl Fn(u64, usize) -> &'a [u8]) -> Result<(), String> {
    for (name, vtable, process, start) in [
        ("snapshot request", t.request_vtable, t.request_process, &REQUEST_START),
        ("sync answer", t.ack_vtable, t.ack_process, &ACK_START),
        ("sync check", t.check_vtable, t.check_process, &CHECK_START),
    ] {
        let slot = read(vtable + 8 * PROCESS, 8);
        if slot.len() != 8 || u64::from_le_bytes(slot.try_into().unwrap()) != process.wrapping_add(delta) {
            return Err(format!("{name}: the vtable at {vtable:#x} does not hold {process:#x}"));
        }
        if read(process, start.len()) != start {
            return Err(format!("{name}: unexpected bytes at {process:#x}"));
        }
    }
    let c = read(t.request_ctor, CTOR_VTABLE_AT + 7);
    if c.len() != CTOR_VTABLE_AT + 7 || c[..CTOR_START.len()] != CTOR_START || c[CTOR_VTABLE_AT..CTOR_VTABLE_AT + 3] != [0x48, 0x8d, 0x05] {
        return Err(format!("request constructor: unexpected bytes at {:#x}", t.request_ctor));
    }
    let rel = i32::from_le_bytes(c[CTOR_VTABLE_AT + 3..].try_into().unwrap());
    if (t.request_ctor + CTOR_VTABLE_AT as u64 + 7).wrapping_add_signed(i64::from(rel)) != t.request_vtable {
        return Err("request constructor: builds another message".into());
    }
    if read(t.ignore_call - IGNORE_BEFORE.len() as u64, IGNORE_BEFORE.len()) != IGNORE_BEFORE || read(t.ignore_call + 5, IGNORE_AFTER.len()) != IGNORE_AFTER {
        return Err(format!("waiting test call: unexpected bytes around {:#x}", t.ignore_call));
    }
    let call = read(t.ignore_call, 5);
    if call.len() != 5 || call[0] != 0xe8 || (t.ignore_call + 5).wrapping_add_signed(i64::from(i32::from_le_bytes(call[1..].try_into().unwrap()))) != t.waiting_test {
        return Err(format!("waiting test call: {:#x} does not call {:#x}", t.ignore_call, t.waiting_test));
    }
    if read(t.waiting_test, WAITING_START.len()) != WAITING_START {
        return Err(format!("waiting test: unexpected bytes at {:#x}", t.waiting_test));
    }
    Ok(())
}

/// Puts `hook` in a vtable's process entry.
unsafe fn hook_process(module: usize, vtable: u64, hook: Process) -> bool {
    let slot = (module + (vtable + 8 * PROCESS - BASE) as usize) as *mut usize;
    let mut old = 0u32;
    if win::VirtualProtect(slot.cast(), 8, win::PAGE_EXECUTE_READWRITE, &mut old) == 0 {
        return false;
    }
    *slot = hook as usize;
    win::VirtualProtect(slot.cast(), 8, old, &mut old);
    true
}

/// Called by DllMain after `rejoin::install`, before the game runs.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("reload: not installed (actif=off)");
        return;
    }
    // `reload_all=off` only stops this machine from starting a reload as host: as a client it must
    // still tell a reload of everyone from its own.
    if !crate::rejoin::installed() {
        log("reload: not installed (needs the rejoin fix)");
        return;
    }
    let module = win::GetModuleHandleW(std::ptr::null()) as usize;
    let image = |address: u64| module + (address - BASE) as usize;
    let delta = (module as u64).wrapping_sub(BASE);
    let mut reasons = Vec::new();
    let Some(i) = EXES.iter().position(|t| {
        fingerprint(t, delta, |address, n| std::slice::from_raw_parts(image(address) as *const u8, n)).map_err(|e| reasons.push(format!("{}: {e}", t.name))).is_ok()
    }) else {
        crate::rejoin::never_skip();
        log(&format!("reload: unknown exe ({}), not installed; every check after a load is kept", reasons.join("; ")));
        return;
    };
    let t = &EXES[i];
    // The relay of the receive path's call first: nothing is changed if it cannot be built.
    // `mov rdx,r15 ; jmp [rip+0] ; hook`: the hook gets the message as second argument.
    let orders = orders_enabled(&cfg);
    let mut rel = None;
    if orders {
        let Some(page) = patch::alloc_near(module) else {
            crate::rejoin::never_skip();
            log("reload: no memory within 2 GB of the exe, not installed; every check after a load is kept");
            return;
        };
        let mut code = vec![0x49, 0x8b, 0xd7, 0xff, 0x25, 0, 0, 0, 0];
        code.extend_from_slice(&(on_waiting_test as WaitingTestHook as usize as u64).to_le_bytes());
        std::ptr::copy_nonoverlapping(code.as_ptr(), page as *mut u8, code.len());
        rel = patch::rel32(image(t.ignore_call), page);
        if rel.is_none() {
            crate::rejoin::never_skip();
            log("reload: relay out of range, not installed; every check after a load is kept");
            return;
        }
    }
    MODULE.store(module, Ordering::Relaxed);
    WAITING_ORIG.store(image(t.waiting_test), Ordering::Relaxed);
    // Each original is stored before its entry is replaced: the entry is live at once.
    for (name, vtable, process, hook, original) in [
        ("snapshot request", t.request_vtable, t.request_process, on_request as Process, &REQUEST_ORIG),
        ("sync answer", t.ack_vtable, t.ack_process, on_ack as Process, &ACK_ORIG),
        ("sync check", t.check_vtable, t.check_process, on_check as Process, &CHECK_ORIG),
    ] {
        original.store(image(process), Ordering::Relaxed);
        if !hook_process(module, vtable, hook) {
            crate::rejoin::never_skip();
            log(&format!("reload: VirtualProtect failed for the {name}, not installed; every check after a load is kept"));
            return;
        }
    }
    if let Some(rel) = rel {
        patch::write_rel32(image(t.ignore_call), rel);
    }
    TABLE.store(i, Ordering::Relaxed);
    let host = if enabled(&cfg) { "" } else { "; reload_all=off, not started by this machine" };
    log(&format!("reload: the host starts the reload of everyone and waits for every loaded game ({}){}{host}", t.name, if orders { "" } else { "; reload_orders=off" }));
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";
    const FILES: [&str; 4] = ["CivilizationVI_DX12.exe", "CivilizationVI.exe", "CivilizationVI_DX12.exe", "CivilizationVI.exe"];

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
            let Ok(file) = std::fs::read(crate::test_exe(EXE_DIR, i, name)) else {
                eprintln!("{name} missing, test skipped");
                continue;
            };
            let read = |address: u64, n: usize| offset(&file, address).map_or(&[][..], |o| &file[o..][..n]);
            for (j, t) in EXES.iter().enumerate() {
                let r = fingerprint(t, 0, read);
                if i == j {
                    assert_eq!(r, Ok(()), "{} on {name}", t.name);
                } else {
                    assert!(r.is_err(), "the {} table must not match {name}", t.name);
                }
            }
        }
    }

    #[test]
    fn settings() {
        assert!(enabled(""));
        assert!(!enabled("reload_all=off\r\n"));
        assert!(enabled("reload_all=off\nreload_all=on\n"), "the last occurrence wins");
        assert!(orders_enabled(""));
        assert!(!orders_enabled("reload_orders=off\n"));
    }

    #[test]
    fn two_reloads_per_turn() {
        assert_eq!(attempt((-1, 0), 90), Some(1));
        assert_eq!(attempt((90, 1), 90), Some(2), "one more when the first did not hold");
        assert_eq!(attempt((90, 2), 90), None, "then the game's own reload");
        assert_eq!(attempt((90, 2), 91), Some(1), "a new turn starts over");
    }

    #[test]
    fn requests_on_the_host() {
        // Host 0. Its own broadcast for a reloading player marks that player.
        assert_eq!(classify(true, 0, 0, 0, 3, 0, true), Request::Reload { keep: false });
        assert_eq!(classify(true, 0, 0, 0, 0, 0, true), Request::Reload { keep: false }, "the host's own");
        // A reloading client's own request would send it a second copy.
        assert_eq!(classify(true, 0, 0, 3, 3, 0, true), Request::Drop);
        // Outside a reload of everyone: the game's own handling (desync, remap, launch).
        assert_eq!(classify(true, 0, 0, 3, 3, 0, false), Request::Plain);
        assert_eq!(classify(true, 0, 0, 0, 0, 0, false), Request::Plain, "the host's own load at a remap");
        assert_eq!(classify(true, 0, 0, 0, 3, 2, false), Request::Plain, "saved-game launch");
    }

    #[test]
    fn requests_on_a_client() {
        // Client 3, host 0. The host asks for a client's copy: a reload of everyone.
        assert_eq!(classify(false, 3, 0, 0, 3, 0, false), Request::Reload { keep: true });
        assert_eq!(classify(false, 3, 0, 0, 4, 0, false), Request::Reload { keep: false });
        assert_eq!(classify(false, 3, 0, 0, 0, 0, true), Request::Reload { keep: false }, "the host's own, after the clients'");
        // The host's own request alone (remap), a client's own, a launch: not one.
        assert_eq!(classify(false, 3, 0, 0, 0, 0, false), Request::Plain);
        assert_eq!(classify(false, 3, 0, 4, 4, 0, false), Request::Plain);
        assert_eq!(classify(false, 3, 0, 3, 3, 0, false), Request::Plain);
        assert_eq!(classify(false, 3, 0, 0, 3, 2, false), Request::Plain);
    }

    #[test]
    fn a_player_reloads_until_its_load_is_over() {
        round_done(true, false);
        assert!(!still_reloading(5, Some(true)), "not in a reload of everyone");
        RELOADING.fetch_or(bit(5) | bit(6), Ordering::Relaxed);
        assert!(still_reloading(5, Some(false)), "request not processed here yet: the mark is not up");
        MARKED.fetch_or(bit(5), Ordering::Relaxed);
        assert!(still_reloading(5, Some(true)), "waits for the game");
        assert!(!still_reloading(5, Some(false)), "its load is over");
        assert!(!still_reloading(5, Some(true)), "forgotten: a later wait is not this reload");
        assert!(!still_reloading(6, None), "left the game");
        assert_eq!(RELOADING.load(Ordering::Relaxed), 0);
        // The end of a round: on the host always, on a client at a turn end.
        RELOADING.store(bit(1), Ordering::Relaxed);
        round_done(false, false);
        assert_eq!(RELOADING.load(Ordering::Relaxed), bit(1), "a client's check after its load");
        round_done(false, true);
        assert_eq!(RELOADING.load(Ordering::Relaxed), 0);
        assert_eq!(bit(-1) | bit(64), 0, "no such players");
        // The kept check: read once, forgotten when a round ends without a load.
        assert!(!take_keep());
        KEEP.store(true, Ordering::Relaxed);
        assert!(take_keep() && !take_keep(), "the next load only");
        KEEP.store(true, Ordering::Relaxed);
        round_done(false, false);
        assert!(!take_keep(), "a round that ended first");
    }
}
