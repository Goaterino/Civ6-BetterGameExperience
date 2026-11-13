//! Multiplayer Helper's remap: the host loads its own game before the clients load it, so every
//! machine runs a loaded game. MPH's single `Network.SendChat(".mph_ui_snap",-2,-1)` in
//! `ui/Additions/VotePanel.lua` becomes `BGD_RemapSnap()` ([remap.lua](remap.lua)). Setting
//! `remap_host_load=off` leaves MPH's remap as it is.

use crate::log;
use crate::options::{publish_natives, Natives};
use std::sync::Mutex;
use std::time::Instant;

/// MPH's script, as the Lua hook names it (`options::script_name`).
pub const SCRIPT: &str = "votepanel.lua";
/// The host's `.mph_ui_snap`, sent once its restarted game is loaded: exactly once in the file.
const SEND: &str = r#"Network.SendChat(".mph_ui_snap",-2,-1)"#;
/// Only in MPH's remap code.
const MARK: &str = "GAME_HOST_IS_JUST_RELOADING";
const CALL: &str = "BGD_RemapSnap()";
const BLOCK: &str = include_str!("remap.lua");

const NATIVES: Natives = &[
    (crate::salon::lua_log, b"Civ6Proxy_Journal\0"),
    (lua_mark, b"Civ6Proxy_RemapMark\0"),
    (lua_take, b"Civ6Proxy_RemapTake\0"),
];

/// When the host asked for its own load. Kept in this DLL, which stays loaded across that load
/// (the game's configuration does not keep a value set just before it).
static ASKED: Mutex<Option<Instant>> = Mutex::new(None);

fn mark() {
    *ASKED.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
}

/// Seconds since the host asked, and the request cleared; `None` if there is none.
fn take() -> Option<u64> {
    ASKED.lock().unwrap_or_else(|e| e.into_inner()).take().map(|t| t.elapsed().as_secs())
}

/// `Civ6Proxy_RemapMark()`: the host is about to load its own game.
unsafe extern "C-unwind" fn lua_mark(_l: *mut std::ffi::c_void) -> i32 {
    mark();
    0
}

/// `Civ6Proxy_RemapTake()` → seconds since `Civ6Proxy_RemapMark()`, or -1; clears the request.
unsafe extern "C-unwind" fn lua_take(l: *mut std::ffi::c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    (h.push_integer)(l, take().map_or(-1, |s| s.min(i32::MAX as u64) as i32));
    1
}

/// The `remap_host_load` setting (last occurrence wins; `on` by default).
fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("remap_host_load=")).last().map(str::trim) != Some("off")
}

/// MPH's VotePanel.lua with the host's own load before `.mph_ui_snap`; `None` if it is not the
/// expected file.
pub fn patch_vote_panel(src: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(src).ok()?;
    if !text.contains(MARK) || text.matches(SEND).count() != 1 || text.contains(CALL) {
        return None;
    }
    let mut out = text.replacen(SEND, CALL, 1);
    out.push_str("\r\n");
    out.push_str(BLOCK);
    Some(out.into_bytes())
}

/// Called by the Lua hook for VotePanel.lua: the text to compile, or `None` to compile the original.
pub unsafe fn prepare(l: *mut std::ffi::c_void, src: &[u8]) -> Option<Vec<u8>> {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) || !enabled(&cfg) {
        return None;
    }
    let Some(text) = patch_vote_panel(src) else {
        log("remap : VotePanel.lua is not Multiplayer Helper's expected one, left as it is");
        return None;
    };
    publish_natives(l, NATIVES, "remap");
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MPH: &str = r"C:\Program Files (x86)\Steam\steamapps\workshop\content\289070\3307282026\ui\Additions\VotePanel.lua";

    #[test]
    fn request_survives_until_taken_once() {
        assert_eq!(take(), None);
        mark();
        assert_eq!(take(), Some(0));
        assert_eq!(take(), None, "taken once");
    }

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("remap_host_load=off\r\n"));
        assert!(enabled("remap_host_load=off\nremap_host_load=on\n"), "the last occurrence wins");
    }

    #[test]
    fn installed_mph() {
        let Ok(src) = std::fs::read(MPH) else {
            eprintln!("MPH not installed, test skipped");
            return;
        };
        let new = String::from_utf8(patch_vote_panel(&src).expect("MPH's VotePanel.lua is patched")).unwrap();
        assert!(new.ends_with(BLOCK));
        let mph = &new[..new.len() - BLOCK.len()];
        assert!(!mph.contains(SEND), "MPH's send is replaced");
        assert_eq!(mph.matches(CALL).count(), 1, "by one call");
        assert!(BLOCK.contains(r#"Network.SendChat(".mph_ui_snap", -2, -1)"#), "the block sends it itself");
        assert!(BLOCK.contains("function BGD_RemapSnap()"));
        for (_, name) in NATIVES {
            let name = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
            assert!(BLOCK.contains(name), "{name} used by the block");
        }
        assert_eq!(patch_vote_panel(new.as_bytes()), None, "never patched twice");
    }

    #[test]
    fn other_files_left_alone() {
        assert_eq!(patch_vote_panel(b"-- some other VotePanel.lua"), None);
        let twice = format!("{MARK}\n{SEND}\n{SEND}\n");
        assert_eq!(patch_vote_panel(twice.as_bytes()), None, "not the expected version");
        let once = format!("{MARK}\n\t\t\t{SEND}\n");
        let out = String::from_utf8(patch_vote_panel(once.as_bytes()).unwrap()).unwrap();
        assert!(out.starts_with(&format!("{MARK}\n\t\t\t{CALL}\n")));
    }
}
