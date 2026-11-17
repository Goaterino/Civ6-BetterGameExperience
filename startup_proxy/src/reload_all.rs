//! When a client's game has objects the host's has not (or the other way round), the game reloads
//! that client alone, and a game loaded alone drifts away from the running ones (state the sync
//! check does not cover). Instead, at that same turn-end sync check, the host reloads everyone: it
//! asks for its own game and tells every client to ask too, so the game makes one copy and sends it
//! to all of them; games that all loaded the same copy stay identical. `partie.lua` does it, from
//! the counts of `sync_notes.rs`; `rejoin.rs` keeps the clients' check after that load.
//! Setting `reload_all=off` keeps the game's own reload.

use std::ffi::c_void;

/// The `reload_all` setting (last occurrence wins; `on` by default).
fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("reload_all=")).last().map(str::trim) != Some("off")
}

/// Can this machine, as host, reload everyone? It must see the objects on one side only.
fn host_ready(cfg: &str) -> bool {
    !crate::salon::actif_off(cfg) && enabled(cfg) && crate::sync_notes::installed()
}

/// `Civ6Proxy_ReloadAllReady()` → 1 if this machine, as host, reloads everyone after a desync.
pub(crate) unsafe extern "C-unwind" fn lua_ready(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    (h.push_integer)(l, host_ready(&cfg) as i32);
    1
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
}
