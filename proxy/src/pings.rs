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
    include_str!("mods/pings/limits.lua"),
    "\n",
    include_str!("mods/pings/map.lua")
);
const FILTER: &str = include_str!("mods/pings/filter.lua");
const CHAT: &str = include_str!("mods/pings/chat.lua");

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
