//! A player who drops keeps a human slot: the game's AI does not play for it.
//! It changes the simulation, so it only applies when every machine does it: with the POL fixes
//! (every player has the proxy at the same version), or when every player announces the same drop
//! protection level, whatever their version (`pol::departs`, `version::NIVEAU_DEPARTS`). A change
//! of what this module does needs a new level.
//!
//! When a player leaves, the host sends `PlayerConnectionChange`. Its handler (`1800383e0`) sets
//! the player's SLOT_STATUS to "computer" through `18016b840`, and "is human" is that value: from
//! then on every machine runs its own AI for the player, without any network message, until the
//! player is back. The machines' AIs do not decide alike, and the games differ.
//!
//! Here the slot stays "taken" when the player was not kicked. The rest of the handler runs as
//! before (notification, events, game update). The game sends the message twice per departure and
//! only acts on the first, since the slot is then a computer's: the second is skipped here the same
//! way. A kick still hands the player to the AI. The host ends the absent player's turns (the
//! startup proxy's `absent_turn`).

use crate::patch::{self, Site};
use crate::{log, pol};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// `PlayerConnectionChange::Execute`: `mov r11,rsp ; push rbp ; push rdi ; lea rbp,[r11-0D8h] ;
/// sub rsp,1C8h`, no relative address.
const HANDLER: Site = Site {
    name: "player connection change",
    address: 0x1_8003_83e0,
    expected: &[0x4c, 0x8b, 0xdc, 0x55, 0x57, 0x49, 0x8d, 0xab, 0x28, 0xff, 0xff, 0xff, 0x48, 0x81, 0xec, 0xc8, 0x01, 0x00, 0x00],
    copy: (0, 19),
    resume: 19,
};
/// Set slot status (player entry, status): `mov [rsp+8],rbx ; push rdi ; sub rsp,20h ; mov rbx,rcx ;
/// mov edi,edx`, no relative address.
const SET_STATUS: Site = Site {
    name: "slot status",
    address: 0x1_8016_b840,
    expected: &[0x48, 0x89, 0x5c, 0x24, 0x08, 0x57, 0x48, 0x83, 0xec, 0x20, 0x48, 0x8b, 0xd9, 0x8b, 0xfa],
    copy: (0, 15),
    resume: 15,
};
const SITES: [&Site; 2] = [&HANDLER, &SET_STATUS];

// The message: player, connecting, kicked.
const MSG_PLAYER: usize = 0x18;
const MSG_CONNECTING: usize = 0x1c;
const MSG_KICKED: usize = 0x1d;
const SLOT_COMPUTER: i32 = 1;
const SLOT_TAKEN: i32 = 3;

static BASE: AtomicUsize = AtomicUsize::new(0);
static T_HANDLER: AtomicUsize = AtomicUsize::new(0);
static T_SET_STATUS: AtomicUsize = AtomicUsize::new(0);
/// The handler is running for a departure whose slot stays human.
static LEAVING: AtomicBool = AtomicBool::new(false);
/// Players whose departure was handled and who are not back (one bit per player).
static AWAY: AtomicU64 = AtomicU64::new(0);

type Handler = unsafe extern "system" fn(*const u8);
type SetStatus = unsafe extern "system" fn(usize, i32);

/// What to do with a connection change.
#[derive(Debug, PartialEq, Eq)]
enum Change {
    /// The game's handler, untouched.
    Game,
    /// The game's handler, with the slot kept human.
    KeepHuman,
    /// Nothing: the second message of a departure already handled.
    Skip,
}

fn decide(on: bool, connecting: bool, kicked: bool, away: bool) -> Change {
    match (on, connecting, kicked, away) {
        (false, ..) | (_, true, ..) | (_, _, true, _) => Change::Game,
        (true, false, false, false) => Change::KeepHuman,
        (true, false, false, true) => Change::Skip,
    }
}

fn bit(player: u32) -> u64 {
    if player < 64 {
        1 << player
    } else {
        0
    }
}

/// The status the slot gets: "taken" in place of "computer" during a kept departure.
fn kept(leaving: bool, status: i32) -> i32 {
    if leaving && status == SLOT_COMPUTER {
        SLOT_TAKEN
    } else {
        status
    }
}

unsafe extern "system" fn hook_handler(msg: *const u8) {
    let original: Handler = std::mem::transmute(T_HANDLER.load(Ordering::Relaxed));
    let player = std::ptr::read_unaligned(msg.add(MSG_PLAYER).cast::<u32>());
    let (connecting, kicked) = (*msg.add(MSG_CONNECTING) != 0, *msg.add(MSG_KICKED) != 0);
    let b = bit(player);
    let change = decide(pol::departs(), connecting, kicked, AWAY.load(Ordering::Relaxed) & b != 0);
    if connecting || kicked {
        AWAY.fetch_and(!b, Ordering::Relaxed);
    }
    match change {
        Change::Game => original(msg),
        Change::Skip => {}
        Change::KeepHuman => {
            AWAY.fetch_or(b, Ordering::Relaxed);
            LEAVING.store(true, Ordering::Relaxed);
            original(msg);
            LEAVING.store(false, Ordering::Relaxed);
            log(&format!("drop: player {player} left, its slot stays human (no AI takeover)"));
        }
    }
}

unsafe extern "system" fn hook_set_status(entry: usize, status: i32) {
    let original: SetStatus = std::mem::transmute(T_SET_STATUS.load(Ordering::Relaxed));
    original(entry, kept(LEAVING.load(Ordering::Relaxed), status))
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    for site in SITES {
        if let Err(e) = patch::check(base, std::slice::from_ref(site)) {
            log(&format!("drop: unexpected version, no detour: {e}"));
            return;
        }
    }
    BASE.store(base, Ordering::Relaxed);
    // The setter first: the handler's detour relies on it.
    let done = [
        patch::install(base, &SET_STATUS, hook_set_status as *const () as usize).map(|t| T_SET_STATUS.store(t, Ordering::Relaxed)),
        patch::install(base, &HANDLER, hook_handler as *const () as usize).map(|t| T_HANDLER.store(t, Ordering::Relaxed)),
    ];
    match done.iter().find_map(|r| r.as_ref().err()) {
        None => log("drop: a player who leaves keeps a human slot when every player has BGE with the same drop protection level"),
        Some(e) => log(&format!("drop: a detour could not be installed: {e}")),
    }
}

/// The proxy is unloaded while the original stays in memory: its bytes are put back.
pub unsafe fn on_proxy_unload() {
    let base = BASE.swap(0, Ordering::Relaxed);
    if base == 0 {
        return;
    }
    for (site, tramp) in SITES.iter().zip([&T_HANDLER, &T_SET_STATUS]) {
        if tramp.swap(0, Ordering::Relaxed) != 0 {
            patch::restore(base, site);
        }
    }
}

/// A new game.
pub fn reset() {
    AWAY.store(0, Ordering::Relaxed);
    LEAVING.store(false, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_plain_departure_keeps_the_slot() {
        assert_eq!(decide(true, false, false, false), Change::KeepHuman, "a player drops");
        assert_eq!(decide(true, false, false, true), Change::Skip, "the second message of that departure");
        assert_eq!(decide(true, false, true, false), Change::Game, "a kick hands the player to the AI");
        assert_eq!(decide(true, false, true, true), Change::Game, "kicked while away");
        assert_eq!(decide(true, true, false, true), Change::Game, "the player is back");
        for away in [false, true] {
            for kicked in [false, true] {
                for connecting in [false, true] {
                    assert_eq!(decide(false, connecting, kicked, away), Change::Game, "drop protection off");
                }
            }
        }
    }

    #[test]
    fn only_computer_becomes_taken() {
        assert_eq!(kept(true, SLOT_COMPUTER), SLOT_TAKEN);
        assert_eq!(kept(false, SLOT_COMPUTER), SLOT_COMPUTER, "outside a kept departure");
        for status in [0, 2, 3, 4, 5] {
            assert_eq!(kept(true, status), status);
        }
        assert_eq!(bit(64), 0);
    }

    #[test]
    fn sites_hold_the_jump() {
        for site in SITES {
            assert!(site.expected.len() >= 14 && site.copy == (0, site.expected.len()) && site.resume == site.expected.len(), "{}", site.name);
        }
    }
}
