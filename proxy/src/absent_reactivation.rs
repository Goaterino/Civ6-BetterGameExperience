//! The turn of a player who left, ended by another machine, stays ended: GameCore no longer makes it
//! active again. POL fix: it changes the simulation, so it only applies when every player has the
//! proxy (`pol`), and only in a live network game (`pol::partie_reseau`).
//!
//! **The bug (BGE-4EB5B0).** A player who drops keeps a human slot (`drop_slot`), and the host ends
//! its turn with the game's own `PlayerTurnComplete` (the startup proxy's `absent_turn`, called by
//! `partie.lua`). Executing that message, GameCore runs the end-of-turn automation of the player's
//! units (`Player_Processor.cpp`). A unit left with movement after it ("Player %i Unit %i has
//! movement remaining after completing automation. Attempting to turn active player.") makes the
//! game call `AppRequestTurnUnready` (request `0x5006dda`): the absent player is active again, and
//! nobody is there to end the turn. The host ends it only once per turn, so the rollover never ends.
//!
//! **The marker.** The decision must be the same on every machine, a player who rejoins or reloads
//! included, so it can only rest on the message being executed: drop_slot's `AWAY` is per-process
//! memory and a machine that rejoins does not have it. Here the marker is "the `PlayerTurnComplete`
//! of player P was sent by another player": GameCore's receive function gets the sender with the
//! message, the same on every machine. `partie.lua` only sends one for a player who is not
//! connected. That the game itself never sends one for a connected player is an observation, not a
//! reversed fact: in the 15 reports read for this fix, the 23 such messages out of about 13,000 all
//! came from BGE ending the turn of a player who was not connected (host migration included: the
//! new host ends the absent players' turns, BGE-141E15 20:34:32). A paused game is left out by the
//! game: `PlayerTurnComplete` is ignored while paused.
//!
//! **The other reactivations.** In the same reports, the 4,266 `AppRequestTurnUnready` are either
//! the player's own `PlayerTurnUnready` (3,144, request `0xa99d06c`, which an absent player cannot
//! send) or the two requests of the call redirected here (764 `0x5006dda`, automation; 358
//! `0xd60b7e54`). The gate's two other callers were never seen reactivating a player.
//!
//! **The fix.** For the duration of such a message only, on the thread that executes it, the gate
//! that `Player_Processor` calls just before `AppRequestTurnUnready` answers "no" for that player.
//! This is the game's own path when that gate is closed: the turn stays complete, the player's
//! "automation done" flag is set, and the following update steps of the turn leave the player
//! alone. The gate is untouched for any other player, any other message, and the gate's two other
//! callers.
//!
//! **Sites, reversed on the 2020 GameCore only** (`GameCore_XP2_FinalRelease.dll`, link time
//! `5ee10d21`, from the macOS Steam zip). The supported builds (1.0.12.58, .72, .74) were not
//! available when this was written: `SITES` is `None` and nothing is installed until their addresses
//! are filled in. `signatures_in_the_installed_game` (Windows, installed game) finds each signature
//! below in the installed GameCore and prints its address. Then:
//! 1. check in Ghidra that each match is the function described here (strings and callers below);
//! 2. put the 1.0.12.72 addresses in `SITES` and add them to `patch::MOVED` with their 1.0.12.74
//!    counterparts (1.0.12.58 has the 1.0.12.72 code byte for byte);
//! 3. if a signature does not match, the compiler laid out the code differently: reverse the
//!    function again and adapt the signature and the checks, do not loosen them;
//! 4. on the six exes, list the callers of the controller's "send turn complete" (`absent_turn`'s
//!    `send`): the marker holds only if the game never sends it for another, connected human.
//!
//! **Limits.**
//! - `pol::partie_reseau` is per-machine state, set by `chargement.lua` at the load screen. A
//!   machine where that script is not added would make the player active again, alone, and stall
//!   the turn: the same dependency as POL's own "launch data" correction.
//! - Until `SITES` is filled in, the host still ends an absent player's turn once per turn
//!   (`partie.lua`), and the game can make it active again.
//!
//! 2020 addresses (reference only, they are not those of any supported build):
//! - receive (`180033080`, `Messages_Manager.cpp`): `(sender, stream)`, logs `GameCore RECV (%i): %s`,
//!   sets the byte `180b13958` around the message's `Execute`. Callers: the message queue
//!   (`180032820`) and the exe's API (`180570410`).
//! - `PlayerTurnComplete::Execute` (`180034de0`): the message's player at `+0x18`, its reason at
//!   `+0x1c`; ignores the message when the game is paused (`game+0x218`) or the turn already
//!   complete, else sets the turn complete (`180059680`) and runs an update step.
//! - the gate's call (`180057dc1`) in the end-of-turn automation (`1800578e0`), whose string is
//!   the one quoted above. The gate (`1800569c0`) ignores its argument; its other callers are
//!   `180058ce0` and `180058d30` (`RequestTurnUnready`).

use crate::patch::{alloc_near, rel32, write_rel32, Site};
use crate::{log, pol};
use std::cell::Cell;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// 1.0.12.72 addresses of the three sites. `None`: not reversed yet, nothing is installed.
struct Sites {
    receive: u64,
    turn_complete: u64,
    /// The `call` to the gate (the `e8` byte).
    gate_call: u64,
}
const SITES: Option<Sites> = None;

/// `push rbp ; push rbx ; push r12 ; push r13 ; push r14 ; lea rbp,[rsp-5B0h]`, no relative address.
const RECEIVE_PROLOGUE: [u8; 17] = [0x40, 0x55, 0x53, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x48, 0x8d, 0xac, 0x24, 0x50, 0xfa, 0xff, 0xff];
/// `mov rax,rsp ; push rbp ; push rbx ; lea rbp,[rax-1B8h] ; sub rsp,2A8h`, no relative address.
const TURN_COMPLETE_PROLOGUE: [u8; 19] = [0x48, 0x8b, 0xc4, 0x55, 0x53, 0x48, 0x8d, 0xa8, 0x48, 0xfe, 0xff, 0xff, 0x48, 0x81, 0xec, 0xa8, 0x02, 0x00, 0x00];

/// Byte patterns of the 2020 build, `??` for a relative address, and where the site is in each.
/// Checked at the site before anything is changed, and searched by the test.
const RECEIVE_SIGNATURE: (&str, usize) = (
    // prologue ; sub rsp,6B0h ; mov rax,[rdx] ; lea r8,[rbp+5E8h] ; xor r13d,r13d ; mov qword [rsp+50h],-1
    "40 55 53 41 54 41 55 41 56 48 8d ac 24 50 fa ff ff 48 81 ec b0 06 00 00 48 8b 02 4c 8d 85 e8 05 00 00 45 33 ed 48 c7 44 24 50 ff ff ff ff",
    0,
);
const TURN_COMPLETE_SIGNATURE: (&str, usize) = (
    // prologue ; mov rbx,rcx ; mov ecx,[rcx+18h] ; cmp ecx,40h ; jae ; mov [rax+18h],rsi ;
    // mov [rax-18h],rdi ; mov [rax-20h],r14 ; call
    "48 8b c4 55 53 48 8d a8 48 fe ff ff 48 81 ec a8 02 00 00 48 8b d9 8b 49 18 83 f9 40 0f 83 ?? ?? ?? ?? 48 89 70 18 48 89 78 e8 4c 89 70 e0 e8",
    0,
);
const GATE_CALL_SIGNATURE: (&str, usize) = (
    // mov ecx,[r15+0D8h] ; call gate ; test al,al ; jz ; mov rcx,[app] ; mov r8d,ebx ;
    // mov edx,[r15+0D8h] ; mov rax,[rcx] ; call [rax+68h] (AppRequestTurnUnready) ; test al,al ; jz
    "41 8b 8f d8 00 00 00 e8 ?? ?? ?? ?? 84 c0 74 34 48 8b 0d ?? ?? ?? ?? 44 8b c3 41 8b 97 d8 00 00 00 48 8b 01 ff 50 68 84 c0 74 19",
    7,
);
/// The gate: `mov eax,[rip+…] ; cmp eax,-1 ; jz`.
const GATE_SIGNATURE: (&str, usize) = ("8b 05 ?? ?? ?? ?? 83 f8 ff 74", 0);

/// The message's player.
const MSG_PLAYER: usize = 0x18;

static BASE: AtomicUsize = AtomicUsize::new(0);
static T_RECEIVE: AtomicUsize = AtomicUsize::new(0);
static T_TURN_COMPLETE: AtomicUsize = AtomicUsize::new(0);
static GATE: AtomicUsize = AtomicUsize::new(0);
static GATE_SITE: AtomicUsize = AtomicUsize::new(0);
thread_local! {
    /// Sender of the message this thread is receiving.
    static SENDER: Cell<Option<i32>> = const { Cell::new(None) };
    /// Player whose reactivation is refused while this thread executes its message.
    static REFUSED: Cell<Option<i32>> = const { Cell::new(None) };
}
static LOGGED: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

/// Whether a `PlayerTurnComplete` for `player`, received from `sender` (`None`: not received through
/// GameCore's receive function), keeps the player's turn ended.
fn keeps_ended(sender: Option<i32>, player: i32, pol: bool, network: bool) -> bool {
    let valid = |p: i32| (0..64).contains(&p);
    pol && network && valid(player) && sender.is_some_and(|s| valid(s) && s != player)
}

type Receive = unsafe extern "system" fn(i32, usize) -> u64;
type TurnComplete = unsafe extern "system" fn(*const u8) -> u64;
type Gate = unsafe extern "system" fn(i32) -> u64;

unsafe extern "system" fn hook_receive(sender: i32, stream: usize) -> u64 {
    let original: Receive = std::mem::transmute(T_RECEIVE.load(Ordering::Relaxed));
    let previous = SENDER.replace(Some(sender));
    let r = original(sender, stream);
    SENDER.set(previous);
    r
}

unsafe extern "system" fn hook_turn_complete(msg: *const u8) -> u64 {
    let original: TurnComplete = std::mem::transmute(T_TURN_COMPLETE.load(Ordering::Relaxed));
    let player = std::ptr::read_unaligned(msg.add(MSG_PLAYER).cast::<i32>());
    if !keeps_ended(SENDER.get(), player, pol::actifs(), pol::partie_reseau()) {
        return original(msg);
    }
    let previous = REFUSED.replace(Some(player));
    let r = original(msg);
    REFUSED.set(previous);
    r
}

unsafe extern "system" fn hook_gate(player: i32) -> u64 {
    if REFUSED.get() == Some(player) {
        let k = LOGGED.fetch_add(1, Ordering::Relaxed) + 1;
        if k <= MAX_LOGS {
            log(&format!("absent: player {player}'s turn was ended by another player; the game wanted to make it active again, it stays ended ({k})"));
        }
        return 0;
    }
    let gate: Gate = std::mem::transmute(GATE.load(Ordering::Relaxed));
    gate(player)
}

/// `(pattern, site offset)` as bytes, `None` for `??`.
fn pattern(signature: (&str, usize)) -> Vec<Option<u8>> {
    signature.0.split_whitespace().map(|b| u8::from_str_radix(b, 16).ok()).collect()
}

fn matches(pattern: &[Option<u8>], bytes: &[u8]) -> bool {
    pattern.len() == bytes.len() && pattern.iter().zip(bytes).all(|(p, b)| p.is_none_or(|p| p == *b))
}

/// The signature is at `address` (a 1.0.12.72 address) in the loaded GameCore.
unsafe fn signature_at(base: usize, address: u64, signature: (&str, usize)) -> bool {
    let p = pattern(signature);
    // The site's own shift in 1.0.12.74 (`patch::MOVED`), not that of an address before it.
    let read = std::slice::from_raw_parts((base + crate::patch::rva(address) - signature.1) as *const u8, p.len());
    matches(&p, &crate::patch::as_known(address - signature.1 as u64, read))
}

fn site(name: &'static str, address: u64, prologue: &'static [u8]) -> Site {
    Site { name, address, expected: prologue, copy: (0, prologue.len()), resume: prologue.len() }
}

/// The two detoured functions: receive, then `PlayerTurnComplete::Execute`.
fn sites(s: &Sites) -> [Site; 2] {
    [site("receive", s.receive, &RECEIVE_PROLOGUE), site("player turn complete", s.turn_complete, &TURN_COMPLETE_PROLOGUE)]
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    let Some(s) = SITES else {
        log("absent: GameCore addresses not reversed for this build yet, a turn ended for an absent player can still be reactivated");
        return;
    };
    let call = base + crate::patch::rva(s.gate_call);
    let gate = call.wrapping_add(5).wrapping_add_signed(std::ptr::read_unaligned((call + 1) as *const i32) as isize);
    let gate_ok = signature_at(base, s.gate_call, GATE_CALL_SIGNATURE)
        && matches(&pattern(GATE_SIGNATURE), std::slice::from_raw_parts(gate as *const u8, pattern(GATE_SIGNATURE).len()));
    if !signature_at(base, s.receive, RECEIVE_SIGNATURE) || !signature_at(base, s.turn_complete, TURN_COMPLETE_SIGNATURE) || !gate_ok {
        log("absent: unexpected GameCore bytes, nothing installed");
        return;
    }
    let [receive, turn_complete] = sites(&s);
    if let Err(e) = crate::patch::check(base, &[receive, turn_complete]) {
        log(&format!("absent: unexpected version, nothing installed: {e}"));
        return;
    }
    // Relay near GameCore for the gate's `call rel32`: jmp [rip+0] to `hook_gate`.
    let Some(page) = alloc_near(base) else {
        log("absent: no memory within 2 GB of GameCore, nothing installed");
        return;
    };
    let Some(rel) = rel32(call, page) else {
        log("absent: relay out of range, nothing installed");
        return;
    };
    let mut code = vec![0xff, 0x25, 0, 0, 0, 0];
    code.extend_from_slice(&(hook_gate as *const () as usize as u64).to_le_bytes());
    std::ptr::copy_nonoverlapping(code.as_ptr(), page as *mut u8, code.len());
    BASE.store(base, Ordering::Relaxed);
    GATE.store(gate, Ordering::Relaxed);
    let [receive, turn_complete] = sites(&s);
    // The receive detour first: the other one reads the sender it records.
    let done = crate::patch::install(base, &receive, hook_receive as *const () as usize)
        .map(|t| T_RECEIVE.store(t, Ordering::Relaxed))
        .and_then(|_| crate::patch::install(base, &turn_complete, hook_turn_complete as *const () as usize))
        .map(|t| T_TURN_COMPLETE.store(t, Ordering::Relaxed));
    if let Err(e) = done {
        log(&format!("absent: a detour could not be installed: {e}"));
        return;
    }
    write_rel32(call, rel);
    GATE_SITE.store(call, Ordering::Relaxed);
    log("absent: a turn that another player ended for an absent player stays ended (POL, network games)");
}

/// The proxy is unloaded while the original stays in memory: its bytes are put back.
pub unsafe fn on_proxy_unload() {
    let base = BASE.swap(0, Ordering::Relaxed);
    if base == 0 {
        return;
    }
    let Some(s) = SITES else { return };
    let call = GATE_SITE.swap(0, Ordering::Relaxed);
    if call != 0 {
        if let Some(rel) = rel32(call, GATE.load(Ordering::Relaxed)) {
            write_rel32(call, rel);
        }
    }
    let [receive, turn_complete] = sites(&s);
    if T_TURN_COMPLETE.swap(0, Ordering::Relaxed) != 0 {
        crate::patch::restore(base, &turn_complete);
    }
    if T_RECEIVE.swap(0, Ordering::Relaxed) != 0 {
        crate::patch::restore(base, &receive);
    }
}

/// A new game: the log starts again.
pub fn reset() {
    LOGGED.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_another_players_message_in_a_pol_network_game() {
        assert!(keeps_ended(Some(1), 2, true, true), "the host ends an absent player's turn");
        assert!(!keeps_ended(Some(2), 2, true, true), "the player's own end of turn");
        assert!(!keeps_ended(None, 2, true, true), "not received through GameCore's receive function");
        assert!(!keeps_ended(Some(1), 2, false, true), "not every player has BGE");
        assert!(!keeps_ended(Some(1), 2, true, false), "local game (solo, hotseat) or PlayByCloud");
        assert!(!keeps_ended(Some(-1), 2, true, true) && !keeps_ended(Some(64), 2, true, true), "no such sender");
        assert!(!keeps_ended(Some(1), -1, true, true) && !keeps_ended(Some(1), 64, true, true), "no such player");
    }

    #[test]
    fn signatures_are_well_formed() {
        for (sig, prologue) in [(RECEIVE_SIGNATURE, &RECEIVE_PROLOGUE[..]), (TURN_COMPLETE_SIGNATURE, &TURN_COMPLETE_PROLOGUE[..])] {
            let p = pattern(sig);
            assert!(matches(&p[..prologue.len()], prologue), "the prologue opens the signature");
            assert!(prologue.len() >= 14, "room for the jump");
        }
        let call = pattern(GATE_CALL_SIGNATURE);
        assert_eq!(call[GATE_CALL_SIGNATURE.1], Some(0xe8), "the site is the call");
        assert_eq!(pattern(("41 ?? zz", 0)), vec![Some(0x41), None, None]);
    }

    /// Windows, installed game: where each signature is in the installed GameCore. Each must be
    /// found exactly once; with `SITES` filled in, at its address.
    #[test]
    fn signatures_in_the_installed_game() {
        let path = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\DLC\Expansion2\Binaries\Win64\GameCore_XP2_FinalRelease.dll";
        let Ok(b) = std::fs::read(path) else { return };
        let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize;
        let pe = u32_at(0x3c);
        let n = u16::from_le_bytes(b[pe + 6..pe + 8].try_into().unwrap()) as usize;
        let opt = u16::from_le_bytes(b[pe + 20..pe + 22].try_into().unwrap()) as usize;
        let address = |offset: usize| {
            (0..n).find_map(|i| {
                let s = pe + 24 + opt + i * 40;
                let (va, size, raw) = (u32_at(s + 12), u32_at(s + 16), u32_at(s + 20));
                (raw..raw + size).contains(&offset).then(|| crate::patch::IMAGE_BASE + (offset - raw + va) as u64)
            })
        };
        let mut found = Vec::new();
        for (name, sig) in [("receive", RECEIVE_SIGNATURE), ("player turn complete", TURN_COMPLETE_SIGNATURE), ("gate call", GATE_CALL_SIGNATURE)] {
            let p = pattern(sig);
            let at: Vec<u64> = (0..b.len() - p.len()).filter(|&o| matches(&p, &b[o..o + p.len()])).filter_map(|o| address(o + sig.1)).collect();
            eprintln!("{name}: {at:#x?}");
            found.push((name, at));
        }
        for (name, at) in &found {
            assert_eq!(at.len(), 1, "{name}: the signature must be found exactly once, reverse the function again");
        }
        if let Some(s) = SITES {
            // The table's addresses are those of 1.0.12.72: compare in that build only.
            let wanted = [s.receive, s.turn_complete, s.gate_call].map(|a| crate::patch::IMAGE_BASE + crate::patch::rva(a) as u64);
            for ((name, at), w) in found.iter().zip(wanted) {
                assert_eq!(at[0], w, "{name}");
            }
        }
    }
}
