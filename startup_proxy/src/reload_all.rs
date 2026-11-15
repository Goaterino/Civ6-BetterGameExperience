//! The first time a client's game must be reloaded after a desync, the host reloads everyone
//! instead: it loads its own game, then every client loads it (Multiplayer Helper's `.mph_ui_snap`,
//! as in `remap.rs`). A freshly loaded game differs from a running one in state the sync check does
//! not cover (AI team counts, plot yields), so a player reloaded alone drifts from the host again;
//! games that all loaded the host's game stay identical.
//!
//! Once per connection of a player; later reloads of that player are the game's own. Needs MPH. The
//! Lua side is in `partie.lua`. Setting `reload_all=off` keeps the game's own reload.

use std::collections::HashSet;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// MPH's VotePanel.lua was seen this session (set by `remap.rs`).
static MPH: AtomicBool = AtomicBool::new(false);
/// When the host asked for its own load for a reload of everyone. Kept in this DLL, which stays
/// loaded across that load.
static PENDING: Mutex<Option<Instant>> = Mutex::new(None);
/// Players who already had a reload of everyone since their last connection.
static USED: Mutex<Option<HashSet<i32>>> = Mutex::new(None);

/// MPH's VotePanel.lua has the clients' handler of `.mph_ui_snap` (a client loads the host's game).
pub fn note_vote_panel(text: &str) {
    if text.contains(r#"string.lower(text) == ".mph_ui_snap""#) && text.contains("Network.RequestSnapshot()") {
        MPH.store(true, Ordering::Relaxed);
    }
}

/// The `reload_all` setting (last occurrence wins; `on` by default).
fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("reload_all=")).last().map(str::trim) != Some("off")
}

fn used() -> std::sync::MutexGuard<'static, Option<HashSet<i32>>> {
    USED.lock().unwrap_or_else(|e| e.into_inner())
}

/// Can everyone be reloaded for this player now?
fn ready(player: i32, cfg: &str) -> bool {
    !crate::salon::actif_off(cfg) && enabled(cfg) && MPH.load(Ordering::Relaxed) && !used().as_ref().is_some_and(|u| u.contains(&player))
}

/// The host is about to load its own game for this player.
fn mark(player: i32) {
    used().get_or_insert_with(HashSet::new).insert(player);
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
}

/// Seconds since `mark`, and the request cleared; `None` if there is none.
fn take() -> Option<u64> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).take().map(|t| t.elapsed().as_secs())
}

/// The player connected again: a reload of everyone is possible again for them.
fn connected(player: i32) {
    if let Some(u) = used().as_mut() {
        u.remove(&player);
    }
}

fn arg_player(l: *mut c_void) -> Option<i32> {
    // SAFETY: reads a Lua argument through the game's API.
    unsafe { crate::salon::arg_str(l, 1) }.and_then(|v| v.trim().split('.').next().and_then(|x| x.parse().ok()))
}

/// `Civ6Proxy_ReloadAllReady(player)` → 1 if everyone can be reloaded for this player, otherwise 0.
pub(crate) unsafe extern "C-unwind" fn lua_ready(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    let ok = arg_player(l).is_some_and(|p| ready(p, &cfg));
    (h.push_integer)(l, ok as i32);
    1
}

/// `Civ6Proxy_ReloadAllMark(player)`: the host is about to load its own game for this player.
pub(crate) unsafe extern "C-unwind" fn lua_mark(l: *mut c_void) -> i32 {
    if let Some(p) = arg_player(l) {
        mark(p);
        crate::log(&format!("reload_all: player {p} out of sync; the host loads its own game, then every client"));
    }
    0
}

/// `Civ6Proxy_ReloadAllTake()` → seconds since `Civ6Proxy_ReloadAllMark`, or -1; clears the request.
pub(crate) unsafe extern "C-unwind" fn lua_take(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    (h.push_integer)(l, take().map_or(-1, |s| s.min(i32::MAX as u64) as i32));
    1
}

/// `Civ6Proxy_ReloadAllConnected(player)`: the player connected again.
pub(crate) unsafe extern "C-unwind" fn lua_connected(l: *mut c_void) -> i32 {
    if let Some(p) = arg_player(l) {
        connected(p);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("reload_all=off\r\n"));
        assert!(enabled("reload_all=off\nreload_all=on\n"), "the last occurrence wins");
    }

    #[test]
    fn once_per_connection_and_only_with_mph() {
        MPH.store(false, Ordering::Relaxed);
        assert!(!ready(7, ""), "never without MPH");
        note_vote_panel("if b_ishost == true and hostID ~= localID and (string.lower(text) == \".mph_ui_snap\")  then\n\t\tNetwork.RequestSnapshot()");
        assert!(ready(7, ""));
        assert!(!ready(7, "reload_all=off"));
        mark(7);
        assert!(!ready(7, ""), "once per connection");
        assert!(ready(8, ""), "other players unaffected");
        assert_eq!(take(), Some(0));
        assert_eq!(take(), None, "taken once");
        connected(7);
        assert!(ready(7, ""), "possible again after a new connection");
    }
}
