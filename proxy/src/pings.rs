//! Team pings reuse the negotiated lobby verdict and the game's private chat transport.
//! GenVFX lives under WorldViewControls and owns no input or update handler in the stock game.
use crate::exe::is_file;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

const SUPPORT: &str = concat!(
    include_str!("mods/pings/catalog.lua"),
    "\n",
    include_str!("mods/pings/common.lua")
);
const TEXT: &str = concat!(
    include_str!("../../textes/bgd_textes.lua"),
    "\n",
    include_str!("../../textes/bgd_texte.lua")
);
const MAP: &str = concat!(
    include_str!("mods/pings/bindings.lua"), "\n",
    include_str!("mods/pings/prefs.lua"), "\n",
    include_str!("mods/pings/limits.lua"),
    "\n",
    include_str!("mods/pings/map.lua")
);
const FILTER: &str = include_str!("mods/pings/filter.lua");
const CHAT: &str = concat!(
    include_str!("mods/pings/bindings.lua"), "\n",
    include_str!("mods/pings/prefs.lua"), "\n",
    include_str!("mods/pings/chat.lua")
);

// ---------------------------------------------------------------------------------------------
// Ping volume and keyboard chords kept in rollover_proxy.cfg (prefs.lua): Civ's
// UserConfiguration is emptied by every game load. Only `ping_*` keys with integer values.

const CFG_NAME: &str = "rollover_proxy.cfg";

fn pref_key(key: &str) -> bool {
    key.len() > 5 && key.starts_with("ping_") && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// The value of `key` in the settings text, last occurrence winning as in `read_config`.
pub fn pref(cfg: &str, key: &str) -> Option<i32> {
    if !pref_key(key) {
        return None;
    }
    cfg.lines()
        .filter_map(|l| l.split_once('='))
        .filter(|(k, _)| k.trim() == key)
        .last()
        .and_then(|(_, v)| v.trim().parse().ok())
}

/// The settings text with `changes` (`ping_key=integer`, one per line): each key takes its
/// first line and loses the later ones, other lines stay, missing keys go at the end.
pub fn merge_prefs(existing: &str, changes: &str) -> Result<String, String> {
    let mut pending: Vec<(String, i32)> = Vec::new();
    for line in changes.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (k, v) = line.split_once('=').ok_or_else(|| format!("line without '=': {line}"))?;
        let k = k.trim();
        if !pref_key(k) {
            return Err(format!("not a ping setting: {k}"));
        }
        let v: i32 = v.trim().parse().ok().filter(|v| *v >= 0).ok_or_else(|| format!("invalid value for {k}"))?;
        pending.retain(|(pk, _)| pk != k);
        pending.push((k.to_string(), v));
    }
    let mut done: Vec<String> = Vec::new();
    let mut out: Vec<String> = Vec::new();
    for line in existing.lines() {
        let key = line.split_once('=').map(|(k, _)| k.trim());
        if key.is_some_and(|k| done.iter().any(|d| d == k)) {
            continue;
        }
        match key.and_then(|k| pending.iter().position(|(pk, _)| pk == k)) {
            Some(i) => {
                let (k, v) = pending.remove(i);
                out.push(format!("{k}={v}"));
                done.push(k);
            }
            None => out.push(line.to_string()),
        }
    }
    out.extend(pending.into_iter().map(|(k, v)| format!("{k}={v}")));
    let mut text = out.join("\r\n");
    text.push_str("\r\n");
    Ok(text)
}

fn cfg_path() -> Option<std::path::PathBuf> {
    crate::self_dir().map(|d| d.join(CFG_NAME))
}

pub fn read_pref(key: &str) -> Option<i32> {
    pref(&std::fs::read_to_string(cfg_path()?).ok()?, key)
}

pub fn save_prefs(changes: &str) -> Result<(), String> {
    let path = cfg_path().ok_or("game folder not found")?;
    let existing = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("cannot read {CFG_NAME}: {e}")),
    };
    let text = merge_prefs(&existing, changes)?;
    if text != existing {
        std::fs::write(&path, text).map_err(|e| format!("cannot write {CFG_NAME}: {e}"))?;
        crate::log(&format!("pings : {CFG_NAME} saved ({})", changes.lines().collect::<Vec<_>>().join(", ")));
    }
    Ok(())
}

// Keep the mod's locals out of the host script, including its limit on locals per chunk.
fn block(out: &mut Vec<u8>, body: &str) {
    for part in ["\ndo\nlocal function BGD_InstallPings()\n", SUPPORT, "\n", body,
        "\nend\nlocal ok, err = pcall(BGD_InstallPings)\nif not ok then print('BGE team pings: ' .. tostring(err)) end\nend\n"] {
        out.extend_from_slice(part.as_bytes());
    }
}

pub fn lua(chunk: &str, src: &[u8]) -> Option<Vec<u8>> {
    let map = is_file(chunk, "GenVFX.lua");
    let chat = is_file(chunk, "ChatPanel.lua");
    if !(map || chat || is_file(chunk, "WorldTracker.lua")) || !crate::pol::actifs() {
        return None;
    }
    Some(compose(map, chat, src))
}

fn compose(map: bool, chat: bool, src: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    // ChatLogic subscribes during include(), before ChatPanel.Initialize. Keep a UTF-8 BOM first.
    let src = if let Some(rest) = src.strip_prefix(b"\xef\xbb\xbf") {
        out.extend_from_slice(b"\xef\xbb\xbf");
        rest
    } else {
        src
    };
    if !map {
        block(&mut out, FILTER);
    }
    out.extend_from_slice(src);
    if map || chat {
        out.push(b'\n');
        out.extend_from_slice(TEXT.as_bytes());
        block(&mut out, if map { MAP } else { CHAT });
    }
    out
}

pub fn xml(src: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(src).map_err(|e| e.to_string())?;
    if text.contains("BGD_PingMap") {
        return Err("contrôles déjà présents".into());
    }
    let end = text
        .rfind("</Context>")
        .ok_or("balise </Context> introuvable")?;
    Ok([
        &text[..end],
        include_str!("mods/pings/map.xml"),
        &text[end..],
    ]
    .concat()
    .into_bytes())
}

const SOUNDS: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/ping_audio.rs"));

#[link(name = "winmm")]
extern "system" {
    fn PlaySoundW(sound: *const u8, module: *mut c_void, flags: u32) -> i32;
}
#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> *mut c_void;
    fn GetWindowThreadProcessId(window: *mut c_void, pid: *mut u32) -> u32;
}

// WAVs are standard PCM16. Locate RIFF chunks so editors can add metadata or stereo.
fn with_gain(wav: &[u8], percent: i32) -> Option<Vec<u8>> {
    if wav.get(..4)? != b"RIFF" || wav.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut pos = 12;
    let mut pcm = false;
    while pos + 8 <= wav.len() {
        let tag = &wav[pos..pos + 4];
        let size = u32::from_le_bytes(wav[pos + 4..pos + 8].try_into().ok()?) as usize;
        let start = pos + 8;
        let body = wav.get(start..start.checked_add(size)?)?;
        if tag == b"fmt " {
            pcm = body.get(..2)? == [1, 0] && body.get(14..16)? == [16, 0];
        } else if tag == b"data" && pcm && size > 0 && size % 2 == 0 {
            let mut out = wav.to_vec();
            for s in out[start..start + size].chunks_exact_mut(2) {
                let value = i16::from_le_bytes([s[0], s[1]]) as i32 * percent / 100;
                s.copy_from_slice(&(value as i16).to_le_bytes());
            }
            return Some(out);
        }
        pos = start + size + size % 2;
    }
    None
}

type Cache = Mutex<HashMap<(usize, i32), Vec<u8>>>;
static CACHE: OnceLock<Cache> = OnceLock::new();

pub fn stop() {
    if let Some(cache) = CACHE.get() {
        if let Ok(mut sounds) = cache.lock() {
            if !sounds.is_empty() {
                unsafe {
                    PlaySoundW(std::ptr::null(), std::ptr::null_mut(), 0);
                }
                sounds.clear();
            }
        }
    }
}

pub fn play(name: &[u8], percent: i32, mute_unfocused: bool) {
    if !(1..=100).contains(&percent) {
        return;
    }
    let Some((index, (_, wav))) = SOUNDS
        .iter()
        .enumerate()
        .find(|(_, (id, _))| id.as_bytes() == name)
    else {
        return;
    };
    if mute_unfocused {
        let mut owner = 0;
        unsafe { GetWindowThreadProcessId(GetForegroundWindow(), &mut owner) };
        if owner != std::process::id() {
            return;
        }
    }
    // SND_MEMORY | SND_ASYNC needs immutable storage until playback ends. At most 100
    // gain variants per shipped cue, created only for levels actually used.
    let Ok(mut cache) = CACHE.get_or_init(Default::default).lock() else {
        return;
    };
    let data = match cache.entry((index, percent)) {
        std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::hash_map::Entry::Vacant(entry) => {
            let Some(bytes) = with_gain(wav, percent) else {
                return;
            };
            entry.insert(bytes)
        }
    };
    unsafe {
        PlaySoundW(
            data.as_ptr(),
            std::ptr::null_mut(),
            0x0004 | 0x0001 | 0x0002,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_cues_are_valid_pcm_and_match_the_catalog() {
        let catalog = include_str!("mods/pings/catalog.lua");
        for name in catalog
            .split("sound=\"")
            .skip(1)
            .map(|s| s.split('"').next().unwrap())
        {
            let (_, wav) = SOUNDS
                .iter()
                .find(|(id, _)| *id == name)
                .expect("missing ping WAV");
            assert_eq!(with_gain(wav, 100).as_deref(), Some(*wav));
            assert_ne!(with_gain(wav, 25).unwrap(), *wav);
        }
    }
    #[test]
    fn saved_ping_preferences() {
        let cfg = "actif=on\r\nping_volume=50\r\n# note\r\nping_volume=75\r\n";
        assert_eq!(pref(cfg, "ping_volume"), Some(75), "last occurrence wins");
        assert_eq!(pref(cfg, "ping_key_TeamPingWheel_0"), None);
        assert_eq!(pref(cfg, "actif"), None, "only ping_* keys");
        let merged = merge_prefs(cfg, "ping_volume=25\nping_key_TeamPingWheel_1=267").unwrap();
        assert_eq!(merged, "actif=on\r\nping_volume=25\r\n# note\r\nping_key_TeamPingWheel_1=267\r\n");
        assert_eq!(merge_prefs("", "ping_volume=0").unwrap(), "ping_volume=0\r\n");
        assert!(merge_prefs(cfg, "actif=off").is_err(), "other settings are not the pings' to change");
        assert!(merge_prefs(cfg, "ping_volume=-1").is_err());
        assert!(merge_prefs(cfg, "ping_volume=1\r\nping_x=y").is_err());
    }

    #[test]
    fn preserves_context_and_installs_filters_before_subscribers() {
        let xml = xml(b"<Context><Container ID=\"Stock\"/></Context>").unwrap();
        assert!(
            xml.ends_with(b"</Context>") && xml.starts_with(b"<Context><Container ID=\"Stock\"/>")
        );
        assert!(super::xml(&xml).is_err());
        let chat = String::from_utf8(compose(false, true, b"\xef\xbb\xbf--StockChat")).unwrap();
        assert!(chat.starts_with('\u{feff}'));
        assert!(chat.find("function chat.Add").unwrap() < chat.find("--StockChat").unwrap());
        assert!(
            chat.find("--StockChat").unwrap()
                < chat.find("RegisterCommitCallback(submit)").unwrap()
        );
    }
}
