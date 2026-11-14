//! Rejoining at a turn end: the host answers the client's sync check after loading at once.
//!
//! A client that loads an end-of-turn snapshot sends a sync check for that turn. The host, already
//! out of that turn's sync round, keeps it for its next round and compares it one turn late
//! (`rejoin.rs` makes the client skip it instead). Here the host compares it on arrival, while its
//! game is still the one it sent: between its last sync round and the next turn start. The client is
//! repaired before the next turn is played.
//!
//! Host side, four hooks (`EXES`):
//! - "set waiting for a snapshot": a player has loaded the host's snapshot while the host is between
//!   turns; its check is expected (`Expected`);
//! - the turn start (GameUpdate): held until that check is answered, `HOLD` at most;
//! - the check handler: an expected check is compared now, with the game's own code;
//! - the acknowledgement handler: the acknowledgement of such an answer, outside a sync round, does
//!   not count for the next round.
//! Anything else is left to the game: a check that is not expected, a round already running, a
//! turn started since.
//!
//! Only for players whose BGE says it sends that check (`BGD_SYNC` in their player configuration,
//! written by `salon.lua`, passed by `partie.lua`); a client sends it only to a host that says it
//! answers (`chargement.lua`, `rejoin::set_host_answers`).
//!
//! Setting `postload_answer=off` in `rollover_proxy.cfg` installs nothing and announces nothing.
//! Addresses of both exes of 2026-11-05, checked at startup.

use crate::exe::rd;
use crate::{log, patch, win};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const BASE: u64 = 0x1_4000_0000;

struct Exe {
    name: &'static str,
    /// "Set waiting for a snapshot" (network player, waiting, reason).
    set_waiting: u64,
    /// The host's handler of one client's sync check.
    handler: u64,
    /// `NetGameSyncProcessed::Process` (message, session).
    ack: u64,
    /// The controller's GameUpdate send (controller, initial turn processing).
    game_update: u64,
    /// Global: the sync manager.
    sync: u64,
}

const EXES: [Exe; 2] = [
    Exe { name: "DX12", set_waiting: 0x1_4081_79e0, handler: 0x1_407f_65b0, ack: 0x1_4080_fc20, game_update: 0x1_400f_7330, sync: 0x1_4239_2ee8 },
    Exe { name: "DX11", set_waiting: 0x1_4081_6210, handler: 0x1_407f_46c0, ack: 0x1_4080_deb0, game_update: 0x1_400f_6970, sync: 0x1_4235_b868 },
];

/// The whole function, replaced: `mov [rcx+1ah],dl ; test dl,dl ; jne ; mov byte [rcx+1ch],1 ; ret ;
/// mov [rcx+28h],r8d ; ret`.
const SET_WAITING: [u8; 17] = [0x88, 0x51, 0x1a, 0x84, 0xd2, 0x75, 0x05, 0xc6, 0x41, 0x1c, 0x01, 0xc3, 0x44, 0x89, 0x41, 0x28, 0xc3];
/// `mov [rsp+10h],rdx ; mov [rsp+8],rcx ; push rbp ; push rsi ; push rdi ; push r14`, moved to the
/// trampoline, then `lea rbp,[rsp-388h]`.
const HANDLER: [u8; 23] = [
    0x48, 0x89, 0x54, 0x24, 0x10, 0x48, 0x89, 0x4c, 0x24, 0x08, 0x55, 0x56, 0x57, 0x41, 0x56, 0x48, 0x8d, 0xac, 0x24, 0x78, 0xfc, 0xff, 0xff,
];
const HANDLER_MOVED: usize = 15;
/// The whole function, replaced by `on_ack`. `X`: the RIP displacement of the sync manager.
/// `sub rsp,28h ; mov r8,rdx ; mov edx,[rcx+38h] ; cmp edx,-1 ; je ; mov rax,[r8] ; mov rcx,r8 ;
/// call [rax+158h] ; test rax,rax ; je ; mov byte [rax+1fh],1 ; mov rcx,[sync] ; mov rax,[rcx] ;
/// add rsp,28h ; jmp [rax+30h] ; add rsp,28h ; ret`
const X: u8 = 0;
const ACK: [u8; 0x3b] = [
    0x48, 0x83, 0xec, 0x28, 0x4c, 0x8b, 0xc2, 0x8b, 0x51, 0x38, 0x83, 0xfa, 0xff, 0x74, 0x27, 0x49, 0x8b, 0x00, 0x49, 0x8b, 0xc8, 0xff, 0x90,
    0x58, 0x01, 0x00, 0x00, 0x48, 0x85, 0xc0, 0x74, 0x04, 0xc6, 0x40, 0x1f, 0x01, 0x48, 0x8b, 0x0d, X, X, X, X, 0x48, 0x8b, 0x01, 0x48, 0x83,
    0xc4, 0x28, 0x48, 0xff, 0x60, 0x30, 0x48, 0x83, 0xc4, 0x28, 0xc3,
];
const ACK_SYNC_AT: usize = 0x27;
/// `mov [rsp+18h],rbx ; push rbp ; sub rsp,70h ; movzx ebx,dl ; mov rbp,rcx`, moved to the
/// trampoline, then `test dl,dl ; je ; mov byte [rcx+7],0`.
const GAME_UPDATE: [u8; 24] = [
    0x48, 0x89, 0x5c, 0x24, 0x18, 0x55, 0x48, 0x83, 0xec, 0x70, 0x0f, 0xb6, 0xda, 0x48, 0x8b, 0xe9, 0x84, 0xd2, 0x74, 0x04, 0xc6, 0x41, 0x07,
    0x00,
];
const GAME_UPDATE_MOVED: usize = 16;

// Network player: waiting for a snapshot, has processed one, reason of its request, acknowledged
// the sync round.
const PLAYER_WAITING: usize = 0x1a;
const PLAYER_PROCESSED: usize = 0x1c;
const PLAYER_REASON: usize = 0x28;
const PLAYER_ACK: usize = 0x1f;
// Controller: "start the next turn" pending, in the turn barrier.
const CTL_START_PENDING: usize = 7;
const CTL_IN_BARRIER: usize = 8;
// Sync manager: a round is running; "acknowledgement received" entry of its vtable.
const SYNC_ROUND: usize = 0x3c;
const SYNC_ON_ACK: usize = 0x30;
// Session vtable: local player ID, network player by ID.
const SESSION_LOCAL: usize = 0x148;
const SESSION_PLAYER: usize = 0x158;
// Messages: the check's player and test flag, the acknowledgement's player.
const CHECK_PLAYER: usize = 0x24;
const CHECK_TEST: usize = 0x38;
const ACK_PLAYER: usize = 0x38;

/// How long the turn start waits for an expected check after the player's load.
const HOLD: Duration = Duration::from_secs(20);
const MAX_PLAYERS: i32 = 64;

/// A check expected from a player that loaded the host's between-turns game.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Expected {
    player: i32,
    /// The host's turn counter when the player finished loading.
    turn: u32,
    loaded: Instant,
    /// The hold has been logged.
    said: bool,
}

#[derive(Default)]
struct State {
    expected: Vec<Expected>,
    /// Players whose acknowledgement of an immediate answer has not arrived yet.
    acks: Vec<i32>,
}

impl State {
    /// A player has loaded the host's game: its check is expected if the host is between turns and
    /// the player's BGE sends it.
    fn loaded(&mut self, player: i32, turn: u32, between_turns: bool, sends: bool, now: Instant) -> bool {
        self.expected.retain(|e| e.player != player);
        if !(between_turns && sends) {
            return false;
        }
        self.expected.push(Expected { player, turn, loaded: now, said: false });
        true
    }

    /// The expected check of `player` has arrived: compare it now? Only in the same between-turns
    /// period, with no round running.
    fn take(&mut self, player: i32, turn: u32, between_turns: bool) -> bool {
        let Some(i) = self.expected.iter().position(|e| e.player == player) else { return false };
        let e = self.expected.remove(i);
        e.turn == turn && between_turns
    }

    /// Must the turn start wait? `present(player)`: still connected. Returns a player to log once.
    fn hold(&mut self, now: Instant, present: impl Fn(i32) -> bool) -> (bool, Option<i32>) {
        self.expected.retain(|e| now.duration_since(e.loaded) < HOLD && present(e.player));
        match self.expected.first_mut() {
            Some(e) => {
                let first = !e.said;
                e.said = true;
                (true, first.then_some(e.player))
            }
            None => (false, None),
        }
    }

    /// A turn starts: nothing older is expected.
    fn turn_started(&mut self) {
        self.expected.clear();
    }
}

static MODULE: AtomicUsize = AtomicUsize::new(0);
static TABLE: AtomicUsize = AtomicUsize::new(usize::MAX);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static HANDLER_ORIG: AtomicUsize = AtomicUsize::new(0);
static GAME_UPDATE_ORIG: AtomicUsize = AtomicUsize::new(0);
/// The game controller, as last seen by a hook.
static CONTROLLER: AtomicUsize = AtomicUsize::new(0);
/// Turn starts seen (GameUpdate with initial turn processing).
static TURN: AtomicU32 = AtomicU32::new(0);
/// Bit per player ID: its BGE sends the check after loading (`partie.lua`).
static SENDERS: AtomicU64 = AtomicU64::new(0);
static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(State::default))
}

/// The `postload_answer` setting (last occurrence wins; `on` by default).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("postload_answer=")).last().map(str::trim) != Some("off")
}

/// Are the hooks in place? What this machine announces (`BGD_SYNC`).
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Relaxed)
}

/// A new game or a loaded snapshot on this machine (the in-game interface loads).
pub fn new_game() {
    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = None;
    CONTROLLER.store(0, Ordering::Relaxed);
    SENDERS.store(0, Ordering::Relaxed);
}

/// The controller, from the hooks of `rejoin.rs` that receive it.
pub fn controller_seen(ctl: usize) {
    CONTROLLER.store(ctl, Ordering::Relaxed);
}

/// "1,3,5" → bit per player ID.
pub fn parse_players(list: &str) -> u64 {
    list.split(',').filter_map(|p| p.trim().parse::<u32>().ok()).filter(|p| *p < 64).fold(0, |m, p| m | 1 << p)
}

/// `Civ6Proxy_PostLoad()` → 1 if this machine answers and sends the check after loading, else 0.
pub(crate) unsafe extern "C-unwind" fn lua_installed(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    (h.push_integer)(l, installed() as i32);
    1
}

/// `Civ6Proxy_PostLoadPlayers("1,3")`: the players whose BGE sends the check after loading (host).
pub(crate) unsafe extern "C-unwind" fn lua_players(l: *mut c_void) -> i32 {
    let mask = crate::salon::arg_str(l, 1).map_or(0, |s| parse_players(&s));
    if SENDERS.swap(mask, Ordering::Relaxed) != mask {
        log(&format!("postload: players sending the check after loading: {mask:#x}"));
    }
    0
}

/// `Civ6Proxy_HostAnswers(0|1)`: the host of the game being loaded answers that check (client).
pub(crate) unsafe extern "C-unwind" fn lua_host_answers(l: *mut c_void) -> i32 {
    let on = installed() && crate::salon::arg_str(l, 1).is_some_and(|v| v.trim() == "1");
    crate::rejoin::set_host_answers(on);
    0
}

unsafe fn image(address: u64) -> usize {
    MODULE.load(Ordering::Relaxed) + (address - BASE) as usize
}

unsafe fn sync_manager() -> usize {
    match EXES.get(TABLE.load(Ordering::Relaxed)) {
        Some(t) => rd::<usize>(image(t.sync)),
        None => 0,
    }
}

/// Is the host between its last sync round and the next turn start?
unsafe fn between_turns(sync: usize) -> bool {
    let ctl = CONTROLLER.load(Ordering::Relaxed);
    ctl != 0 && sync != 0 && rd::<u8>(ctl + CTL_START_PENDING) == 1 && rd::<u8>(ctl + CTL_IN_BARRIER) == 0 && rd::<u8>(sync + SYNC_ROUND) == 0
}

unsafe fn session_player(session: usize, id: i32) -> usize {
    let f: unsafe extern "system" fn(usize, i32) -> usize = std::mem::transmute(rd::<usize>(rd::<usize>(session) + SESSION_PLAYER));
    f(session, id)
}

unsafe fn local_player(session: usize) -> usize {
    let f: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(rd::<usize>(rd::<usize>(session) + SESSION_LOCAL));
    session_player(session, f(session))
}

/// Replaces "set waiting for a snapshot": the game's code, and on the host, a player that has just
/// loaded the between-turns game is expected to send its check.
unsafe extern "system" fn on_set_waiting(player: usize, waiting: usize, reason: usize) {
    let waiting = waiting as u8;
    *((player + PLAYER_WAITING) as *mut u8) = waiting;
    if waiting != 0 {
        *((player + PLAYER_REASON) as *mut u32) = reason as u32;
        return;
    }
    *((player + PLAYER_PROCESSED) as *mut u8) = 1;
    let Some(session) = crate::rejoin::host_session() else { return };
    let Some(id) = (0..MAX_PLAYERS).find(|id| session_player(session, *id) == player) else { return };
    if player == local_player(session) {
        return;
    }
    let between = between_turns(sync_manager());
    let sends = SENDERS.load(Ordering::Relaxed) & (1 << id) != 0;
    if with_state(|s| s.loaded(id, TURN.load(Ordering::Relaxed), between, sends, Instant::now())) {
        log(&format!("postload: player {id} has loaded the game between turns, its check is expected"));
    }
}

type Handler = unsafe extern "system" fn(usize, usize) -> u64;

/// Replaces the host's handler of one client's check: an expected one is compared now.
unsafe extern "system" fn on_check(sync: usize, message: usize) -> u64 {
    let original: Handler = std::mem::transmute(HANDLER_ORIG.load(Ordering::Relaxed));
    if rd::<u8>(message + CHECK_TEST) != 0 || rd::<u8>(sync + SYNC_ROUND) != 0 {
        return original(sync, message);
    }
    let Some(session) = crate::rejoin::host_session() else { return original(sync, message) };
    let id = rd::<i32>(message + CHECK_PLAYER);
    let me = local_player(session);
    if me == 0 || rd::<u8>(me + PLAYER_ACK) != 0 {
        return original(sync, message);
    }
    if !with_state(|s| s.take(id, TURN.load(Ordering::Relaxed), between_turns(sync))) {
        return original(sync, message);
    }
    // The game compares a check when the host has acknowledged its own round: said so for this call.
    *((me + PLAYER_ACK) as *mut u8) = 1;
    let handled = original(sync, message);
    *((me + PLAYER_ACK) as *mut u8) = 0;
    if handled as u8 != 0 {
        with_state(|s| s.acks.push(id));
        log(&format!("postload: check of player {id} after loading answered at once"));
    } else {
        log(&format!("postload: check of player {id} after loading not compared, left to the next round"));
    }
    handled
}

/// Replaces `NetGameSyncProcessed::Process`: the game's code, except that on the host the
/// acknowledgement of an immediate answer, outside a round, is not kept for the next round.
unsafe extern "system" fn on_ack(message: usize, session: usize) -> u64 {
    let id = rd::<i32>(message + ACK_PLAYER);
    if id == -1 {
        return 0;
    }
    let player = session_player(session, id);
    let sync = sync_manager();
    if player != 0 {
        let outside = sync != 0 && rd::<u8>(sync + SYNC_ROUND) == 0;
        let ours = outside && crate::rejoin::host_session() == Some(session) && with_state(|s| {
            let i = s.acks.iter().position(|p| *p == id);
            i.map(|i| s.acks.remove(i)).is_some()
        });
        if ours {
            log(&format!("postload: acknowledgement of player {id} outside a round not kept"));
        } else {
            *((player + PLAYER_ACK) as *mut u8) = 1;
        }
    }
    if sync == 0 {
        return 0;
    }
    let f: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(rd::<usize>(rd::<usize>(sync) + SYNC_ON_ACK));
    f(sync)
}

/// Hooked GameUpdate send: on the host, the turn start waits for an expected check.
unsafe extern "system" fn on_game_update(ctl: usize, initial: usize) -> u64 {
    CONTROLLER.store(ctl, Ordering::Relaxed);
    if initial as u8 != 0 {
        if let Some(session) = crate::rejoin::host_session() {
            let (hold, first) = with_state(|s| s.hold(Instant::now(), |id| session_player(session, id) != 0));
            if let Some(id) = first {
                log(&format!("postload: turn start held for the check of player {id}"));
            }
            if hold {
                return 0;
            }
        }
        TURN.fetch_add(1, Ordering::Relaxed);
        with_state(State::turn_started);
    }
    let original: unsafe extern "system" fn(usize, usize) -> u64 = std::mem::transmute(GAME_UPDATE_ORIG.load(Ordering::Relaxed));
    original(ctl, initial)
}

/// Are this exe's bytes where the table says? `read(address, n)` returns n bytes.
fn fingerprint<'a>(t: &Exe, read: impl Fn(u64, usize) -> &'a [u8]) -> Result<(), String> {
    if read(t.set_waiting, SET_WAITING.len()) != SET_WAITING {
        return Err(format!("set waiting for a snapshot: unexpected bytes at {:#x}", t.set_waiting));
    }
    if read(t.handler, HANDLER.len()) != HANDLER {
        return Err(format!("check handler: unexpected bytes at {:#x}", t.handler));
    }
    if read(t.game_update, GAME_UPDATE.len()) != GAME_UPDATE {
        return Err(format!("GameUpdate send: unexpected bytes at {:#x}", t.game_update));
    }
    let a = read(t.ack, ACK.len());
    let masked = |i: usize| (ACK_SYNC_AT..ACK_SYNC_AT + 4).contains(&i);
    if a.len() != ACK.len() || a.iter().zip(ACK).enumerate().any(|(i, (x, y))| !masked(i) && *x != y) {
        return Err(format!("acknowledgement handler: unexpected bytes at {:#x}", t.ack));
    }
    let rel = i32::from_le_bytes(a[ACK_SYNC_AT..ACK_SYNC_AT + 4].try_into().unwrap());
    if (t.ack + ACK_SYNC_AT as u64 + 4).wrapping_add_signed(i64::from(rel)) != t.sync {
        return Err("acknowledgement handler: does not read the sync manager".into());
    }
    Ok(())
}

/// Called by DllMain after `rejoin`, `snapshots` and `sync_notes`, which read the same functions.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("postload: not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log("postload: turned off (postload_answer=off)");
        return;
    }
    if !crate::rejoin::installed() {
        log("postload: not installed (needs the rejoin fix)");
        return;
    }
    let module = win::GetModuleHandleW(std::ptr::null()) as usize;
    let at = |address: u64| module + (address - BASE) as usize;
    let mut reasons = Vec::new();
    let Some(i) = EXES.iter().position(|t| {
        fingerprint(t, |address, n| std::slice::from_raw_parts(at(address) as *const u8, n)).map_err(|e| reasons.push(format!("{}: {e}", t.name))).is_ok()
    }) else {
        log(&format!("postload: unknown exe ({}), not installed", reasons.join("; ")));
        return;
    };
    let t = &EXES[i];
    MODULE.store(module, Ordering::Relaxed);
    TABLE.store(i, Ordering::Relaxed);
    // The two trampolines first: once they exist, the other two hooks cannot fail half-way.
    let (handler, update) = (at(t.handler), at(t.game_update));
    let tramps = patch::install_at("postload check handler", handler, &HANDLER[..HANDLER_MOVED], handler + HANDLER_MOVED, on_check as *const () as usize)
        .and_then(|h| {
            HANDLER_ORIG.store(h, Ordering::Relaxed);
            patch::install_at("postload GameUpdate", update, &GAME_UPDATE[..GAME_UPDATE_MOVED], update + GAME_UPDATE_MOVED, on_game_update as *const () as usize)
        })
        .and_then(|u| {
            GAME_UPDATE_ORIG.store(u, Ordering::Relaxed);
            patch::install_at("postload acknowledgement", at(t.ack), &[], at(t.ack), on_ack as *const () as usize)
        })
        .and_then(|_| patch::install_at("postload set waiting", at(t.set_waiting), &[], at(t.set_waiting), on_set_waiting as *const () as usize));
    match tramps {
        Ok(_) => {
            INSTALLED.store(true, Ordering::Relaxed);
            log(&format!("postload: the host answers the sync check after loading at once ({})", t.name));
        }
        Err(e) => log(&format!("postload: {e}, not announced")),
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

    /// The 14-byte jump fits in each replaced function and in the moved instructions.
    #[test]
    fn sites_hold_the_jump() {
        assert!(SET_WAITING.len() >= 14 && ACK.len() >= 14);
        assert!(HANDLER_MOVED >= 14 && GAME_UPDATE_MOVED >= 14);
        assert_eq!(ACK[ACK.len() - 1], 0xc3);
        assert_eq!(SET_WAITING[SET_WAITING.len() - 1], 0xc3);
    }

    #[test]
    fn only_an_expected_check_of_the_same_period_is_compared() {
        let now = Instant::now();
        let mut s = State::default();
        assert!(!s.take(3, 5, true), "nothing expected");
        assert!(s.loaded(3, 5, true, true, now));
        assert!(s.take(3, 5, true), "expected, same turn, between turns");
        assert!(!s.take(3, 5, true), "once");
        assert!(!s.loaded(3, 5, false, true, now), "the host was not between turns");
        assert!(!s.loaded(3, 5, true, false, now), "that player's BGE does not send the check");
        assert!(!s.take(3, 5, true));
        s.loaded(3, 5, true, true, now);
        assert!(!s.take(3, 5, false), "a round started, or the turn did");
        s.loaded(3, 5, true, true, now);
        assert!(!s.take(3, 6, true), "another turn");
        s.loaded(3, 5, true, true, now);
        s.turn_started();
        assert!(!s.take(3, 5, true), "forgotten when a turn starts");
    }

    #[test]
    fn the_turn_start_waits_for_an_expected_check_only() {
        let now = Instant::now();
        let mut s = State::default();
        assert_eq!(s.hold(now, |_| true), (false, None));
        s.loaded(3, 5, true, true, now);
        assert_eq!(s.hold(now, |_| true), (true, Some(3)), "held, logged once");
        assert_eq!(s.hold(now + Duration::from_secs(5), |_| true), (true, None));
        assert_eq!(s.hold(now + HOLD, |_| true), (false, None), "not past the limit");
        s.loaded(3, 5, true, true, now);
        assert_eq!(s.hold(now, |_| false), (false, None), "the player left");
        s.loaded(3, 5, true, true, now);
        assert!(s.take(3, 5, true));
        assert_eq!(s.hold(now, |_| true), (false, None), "answered");
    }

    #[test]
    fn players_list() {
        assert_eq!(parse_players(""), 0);
        assert_eq!(parse_players("1,3"), 0b1010);
        assert_eq!(parse_players("0, 63,64,x"), 1 | 1 << 63);
    }

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("postload_answer=off\r\n"));
        assert!(enabled("postload_answer=off\npostload_answer=on\n"), "the last occurrence wins");
    }
}
