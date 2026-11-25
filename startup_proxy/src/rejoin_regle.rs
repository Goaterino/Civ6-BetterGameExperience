//! The rules of `rejoin.rs` that do not depend on the exe: which load's check the host waits for,
//! whether a "round complete" event is left over from before a load, the snapshot record and the
//! request to keep the next check. Shared by the Windows module (`rejoin.rs`) and the macOS loader
//! (`macos_reload.rs`): the controller's flags are at the same offsets in both games.

use std::sync::atomic::{AtomicBool, Ordering};

// Controller: uses background jobs, next turn to start, in the barrier, check started.
pub const CTL_JOBS: usize = 4;
pub const CTL_NEXT_TURN: usize = 7;
pub const CTL_IN_BARRIER: usize = 8;
pub const CTL_CHECK_STARTED: usize = 9;
pub const JOBS_PENDING: usize = 0x40;

/// Request reason: the host launched a saved game for everyone.
pub const REASON_LAUNCH: i32 = 2;
/// Snapshot flags: sent to everyone at a launch; sent while the host's sync round was running.
pub const FLAG_LAUNCH: u32 = 0x10;
pub const FLAG_ROUND: u32 = 0x80;

/// After this, a request to keep the check is forgotten.
pub const KEEP_MAX_AGE_S: u64 = 300;
const SNAPSHOT_KNOWN: u64 = 1 << 63;
const SNAPSHOT_ACK: u64 = 1 << 32;

/// The barrier was entered by a turn end (not by a load) and its check has not started yet.
pub static AFTER_TURN_END: AtomicBool = AtomicBool::new(false);
/// The host reloads everyone (`partie.lua`, `.bgd_reload`): the host waits for the check of the
/// next load, whatever the snapshot says. When it was asked; read once by the next load's barrier.
static KEEP_NEXT: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

/// What was last loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Load {
    /// Reason of the local player's last snapshot request: 2 launch, 0 asked by this machine,
    /// -1 none (joined).
    pub reason: i32,
    /// The snapshot's flags (`FLAG_LAUNCH`, `FLAG_ROUND`).
    pub flags: u32,
    /// Our acknowledgement of the host's round was owed and went out with the snapshot.
    pub ack_on_receipt: bool,
}

impl Load {
    /// Is the host not waiting for the check this load would send?
    pub fn stale(&self) -> bool {
        self.reason != REASON_LAUNCH && self.flags & FLAG_LAUNCH == 0 && (self.flags & FLAG_ROUND == 0 || self.ack_on_receipt)
    }
}

/// The `rejoin_barrier` setting (last occurrence wins; `on` by default).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("rejoin_barrier=")).last().map(str::trim) != Some("off")
}

/// The `stale_round_guard` setting (last occurrence wins; `on` by default).
pub fn guard_enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("stale_round_guard=")).last().map(str::trim) != Some("off")
}

/// Is a "round complete" event left over from before this barrier? Only in a barrier entered by a
/// load whose check has not started.
pub fn stale_round(in_barrier: bool, check_started: bool, after_turn_end: bool) -> bool {
    in_barrier && !check_started && !after_turn_end
}

/// The game's handler, unless the round is stale: in a barrier, leave it and start the next turn;
/// otherwise clear both barrier flags. False when the event is ignored.
///
/// # Safety
/// `ctl` points to a game controller (at least `CTL_CHECK_STARTED + 1` bytes).
pub unsafe fn complete_round(ctl: *mut u8, after_turn_end: bool) -> bool {
    let in_barrier = *ctl.add(CTL_IN_BARRIER) != 0;
    if stale_round(in_barrier, *ctl.add(CTL_CHECK_STARTED) != 0, after_turn_end) {
        return false;
    }
    if in_barrier {
        std::ptr::write_unaligned(ctl.add(CTL_NEXT_TURN).cast::<u16>(), 1);
        *ctl.add(CTL_CHECK_STARTED) = 0;
    } else {
        std::ptr::write_unaligned(ctl.add(CTL_IN_BARRIER).cast::<u16>(), 0);
    }
    true
}

/// A new game or a loaded snapshot (the in-game interface loads). The last snapshot is kept: it
/// arrived just before.
pub fn new_game() {
    AFTER_TURN_END.store(false, Ordering::Relaxed);
}

/// Leave the barrier without a check? Only one entered by a load, on a machine known to be a client,
/// when the host is not waiting for that check.
pub fn skip(after_turn_end: bool, host: Option<bool>, load: Option<Load>) -> bool {
    !after_turn_end && host == Some(false) && load.is_some_and(|l| l.stale())
}

/// The host reloads everyone: keep the check of the next load.
pub fn keep_next_check() {
    *KEEP_NEXT.lock().unwrap_or_else(|e| e.into_inner()) = Some(std::time::Instant::now());
}

/// Was the next check asked to be kept (recently)? Clears the request.
pub fn take_keep() -> bool {
    KEEP_NEXT.lock().unwrap_or_else(|e| e.into_inner()).take().is_some_and(|t| t.elapsed().as_secs() <= KEEP_MAX_AGE_S)
}

pub fn pack(flags: u32, ack_on_receipt: bool) -> u64 {
    SNAPSHOT_KNOWN | if ack_on_receipt { SNAPSHOT_ACK } else { 0 } | u64::from(flags)
}

pub fn unpack(packed: u64, reason: i32) -> Option<Load> {
    (packed & SNAPSHOT_KNOWN != 0).then(|| Load { reason, flags: packed as u32, ack_on_receipt: packed & SNAPSHOT_ACK != 0 })
}

pub fn describe(what: &str, load: Option<Load>) -> String {
    match load {
        Some(l) => format!("rejoin: {what} (reason {}, flags {:#x}, acknowledged on receipt: {})", l.reason, l.flags, l.ack_on_receipt),
        None => format!("rejoin: {what} (local player unknown)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flags +7, +8, +9 of a controller after the "round complete" event, and whether it was handled.
    fn round(next_turn: u8, in_barrier: u8, check_started: u8, after_turn_end: bool) -> (bool, u8, u8, u8) {
        let mut ctl = [0xaa_u8; 12];
        (ctl[CTL_NEXT_TURN], ctl[CTL_IN_BARRIER], ctl[CTL_CHECK_STARTED]) = (next_turn, in_barrier, check_started);
        let handled = unsafe { complete_round(ctl.as_mut_ptr(), after_turn_end) };
        assert_eq!((ctl[6], ctl[10]), (0xaa, 0xaa), "neighbours untouched");
        (handled, ctl[CTL_NEXT_TURN], ctl[CTL_IN_BARRIER], ctl[CTL_CHECK_STARTED])
    }

    #[test]
    fn round_complete_as_the_game_except_before_a_load_barrier_check() {
        // The game's handler.
        assert_eq!(round(0, 1, 1, true), (true, 1, 0, 0), "turn-end barrier: left, next turn");
        assert_eq!(round(0, 1, 0, true), (true, 1, 0, 0), "turn-end barrier before its check: as the game");
        assert_eq!(round(0, 1, 1, false), (true, 1, 0, 0), "load barrier whose check ran: left, next turn");
        assert_eq!(round(5, 0, 1, false), (true, 5, 0, 0), "no barrier: flags cleared, next turn untouched");
        // A round that ended before the load: the barrier waits for its own check.
        assert_eq!(round(1, 1, 0, false), (false, 1, 1, 0), "load barrier before its check");
    }

    #[test]
    fn guard_setting() {
        assert!(guard_enabled(""));
        assert!(!guard_enabled("stale_round_guard=off\r\n"));
        assert!(guard_enabled("stale_round_guard=off\nstale_round_guard=on\n"), "the last occurrence wins");
    }

    fn load(reason: i32, flags: u32, ack_on_receipt: bool) -> Option<Load> {
        Some(Load { reason, flags, ack_on_receipt })
    }

    #[test]
    fn only_a_stale_post_load_check_is_skipped() {
        // The host is not waiting for these checks.
        assert!(skip(false, Some(false), load(-1, 0, false)), "rejoin after the host's round");
        assert!(skip(false, Some(false), load(0, FLAG_ROUND, true)), "automatic full resync at a turn end");
        assert!(skip(false, Some(false), load(0, 0, false)), "snapshot with no round running");
        // The host's round waits for these.
        assert!(!skip(false, Some(false), load(2, FLAG_LAUNCH, false)), "saved-game launch");
        assert!(!skip(false, Some(false), load(2, 0, false)), "launch resend");
        assert!(!skip(false, Some(false), load(2, FLAG_ROUND, false)), "launch resend during the host's round");
        assert!(!skip(false, Some(false), load(-1, FLAG_ROUND, false)), "rejoin during the host's round");
        assert!(!skip(false, Some(false), load(0, FLAG_ROUND, false)), "requested during a round, before our answer");
        // Unknown load, real turn end, host, no session: the game's own behaviour.
        assert!(!skip(false, Some(false), None));
        assert!(!skip(true, Some(false), load(-1, 0, false)), "client, real turn end");
        assert!(!skip(false, Some(true), load(-1, 0, false)), "host after a load");
        assert!(!skip(false, None, load(-1, 0, false)), "no session");
    }

    #[test]
    fn last_snapshot_round_trip() {
        assert_eq!(unpack(0, 2), None, "nothing recorded");
        assert_eq!(unpack(pack(FLAG_ROUND | FLAG_LAUNCH, true), 0), load(0, 0x90, true));
        assert_eq!(unpack(pack(0, false), -1), load(-1, 0, false));
    }

    #[test]
    fn new_game_forgets_the_turn_end() {
        AFTER_TURN_END.store(true, Ordering::Relaxed);
        new_game();
        assert!(!AFTER_TURN_END.load(Ordering::Relaxed));
    }

    #[test]
    fn keep_request_is_read_once() {
        assert!(!take_keep(), "nothing asked");
        keep_next_check();
        assert!(take_keep(), "the next load keeps its check");
        assert!(!take_keep(), "only that one");
    }

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("rejoin_barrier=off\r\n"));
        assert!(enabled("rejoin_barrier=off\nrejoin_barrier=on\n"), "the last occurrence wins");
    }
}
