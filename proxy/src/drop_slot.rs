//! A player who drops keeps a human slot: the game's AI does not play for it.
//! POL fix: it changes the simulation, so it only applies when every player has the proxy (`pol`).
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
#[path = "drop_slot_regle.rs"]
mod regle;
use regle::{bit, decide, kept, Change};
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

static BASE: AtomicUsize = AtomicUsize::new(0);
static T_HANDLER: AtomicUsize = AtomicUsize::new(0);
static T_SET_STATUS: AtomicUsize = AtomicUsize::new(0);
/// The handler is running for a departure whose slot stays human.
static LEAVING: AtomicBool = AtomicBool::new(false);
/// Players whose departure was handled and who are not back (one bit per player).
static AWAY: AtomicU64 = AtomicU64::new(0);

type Handler = unsafe extern "system" fn(*const u8);
type SetStatus = unsafe extern "system" fn(usize, i32);

unsafe extern "system" fn hook_handler(msg: *const u8) {
    let original: Handler = std::mem::transmute(T_HANDLER.load(Ordering::Relaxed));
    let player = std::ptr::read_unaligned(msg.add(MSG_PLAYER).cast::<u32>());
    let (connecting, kicked) = (*msg.add(MSG_CONNECTING) != 0, *msg.add(MSG_KICKED) != 0);
    let b = bit(player);
    let change = decide(pol::actifs(), connecting, kicked, AWAY.load(Ordering::Relaxed) & b != 0);
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
        None => log("drop: a player who leaves keeps a human slot when every player has BGE (POL)"),
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

    // The rule's tests are in drop_slot_regle.rs.
    #[test]
    fn sites_hold_the_jump() {
        for site in SITES {
            assert!(site.expected.len() >= 14 && site.copy == (0, site.expected.len()) && site.resume == site.expected.len(), "{}", site.name);
        }
    }
}
