//! The rules of `reload_all.rs` that do not depend on the exe: settings, who is reloading in a reload
//! of everyone, how a snapshot request is told apart, how many reloads a turn gets, and the kept
//! check. Shared by the Windows module (`reload_all.rs`) and the macOS loader (`macos_reload.rs`).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// Type of the message that carries a game order.
pub const TYPE_GAME_ORDER: u32 = 3;
/// Reason of a request a player makes for itself, and the one used here.
pub const REASON_RESYNC: i32 = 0;
pub const MAX_PLAYERS: i32 = 64;
/// Reloads of everyone started for one turn before the game's own reload is left alone.
pub const MAX_PER_TURN: u32 = 2;

/// `start` and `state` results, for `partie.lua`.
pub const NOT_POSSIBLE: i32 = 0;
pub const CAN_START: i32 = 1;
pub const RUNNING: i32 = 2;
pub const GAVE_UP: i32 = 3;
pub const OTHERS_RELOADING: i32 = 4;
pub const MIXED_VERSIONS: i32 = 5;

/// Players reloading in a reload of everyone (one bit per player ID).
pub static RELOADING: AtomicU64 = AtomicU64::new(0);
/// Among them, those whose request this machine has processed: their "waits for the game" mark is
/// up until their load is over.
pub static MARKED: AtomicU64 = AtomicU64::new(0);
/// This machine reloads because the host reloads everyone: the check of the game it loads is kept
/// (`rejoin.rs`), however long the load takes.
pub static KEEP: AtomicBool = AtomicBool::new(false);
/// The turn of the last reload of everyone, and how many were started for it.
pub static LAST: Mutex<(i32, u32)> = Mutex::new((-1, 0));

/// The `reload_all` setting (last occurrence wins; `on` by default).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("reload_all=")).last().map(str::trim) != Some("off")
}

/// The `reload_orders` setting: ignore the game orders of a reloading player (`on` by default).
pub fn orders_enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("reload_orders=")).last().map(str::trim) != Some("off")
}

pub fn bit(player: i32) -> u64 {
    if (0..MAX_PLAYERS).contains(&player) {
        1 << player
    } else {
        0
    }
}

/// Which reload of a turn this would be, or `None` after `MAX_PER_TURN`.
pub fn attempt(last: (i32, u32), turn: i32) -> Option<u32> {
    let n = if last.0 == turn { last.1 + 1 } else { 1 };
    (n <= MAX_PER_TURN).then_some(n)
}

/// Is this player still reloading? `waiting` is its "waits for the game" mark, `None` when it has
/// left. Forgets a player whose load is over.
pub fn still_reloading(player: i32, waiting: Option<bool>) -> bool {
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

/// Notes a reload of everyone started for `turn`; rends which one of that turn it is.
pub fn note_started(turn: i32) -> u32 {
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    *last = (turn, attempt(*last, turn).unwrap_or(MAX_PER_TURN));
    last.1
}

/// Can a reload of everyone start this turn (`attempt` on the last one)?
pub fn turn_allows(turn: i32) -> bool {
    attempt(*LAST.lock().unwrap_or_else(|e| e.into_inner()), turn).is_some()
}

/// What a machine does with a request for `player`'s copy sent by `sender`.
#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    /// Not part of a reload of everyone.
    Plain,
    /// The host's request for a player: that player reloads; `keep` when it is this machine.
    Reload { keep: bool },
    /// A reloading player's own request, on the host: it would get a second copy.
    Drop,
}

pub fn classify(host: bool, local: i32, host_id: i32, sender: i32, player: i32, reason: i32, reloading: bool) -> Request {
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

/// A Lua number passed as text (`3`, `3.0`), as an integer.
pub fn int_text(text: &str) -> Option<i32> {
    text.trim().parse::<f64>().ok().map(|t| t as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings() {
        assert!(enabled(""));
        assert!(!enabled("reload_all=off\r\n"));
        assert!(enabled("reload_all=off\nreload_all=on\n"), "the last occurrence wins");
        assert!(orders_enabled(""));
        assert!(!orders_enabled("reload_orders=off\n"));
        assert_eq!(int_text(" 3 "), Some(3));
        assert_eq!(int_text("90.0"), Some(90));
        assert_eq!(int_text("x"), None);
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
        // Turns: two reloads, then none.
        assert!(turn_allows(500));
        assert_eq!(note_started(500), 1);
        assert_eq!(note_started(500), 2);
        assert!(!turn_allows(500));
        assert!(turn_allows(501));
    }
}
