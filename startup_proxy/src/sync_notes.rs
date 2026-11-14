//! Host only: notes which major objects differed for a client at a turn-end sync check, for the
//! chat lines of `partie.lua` (a player reloading after a desync, back in sync, a large repair).
//! Nothing the host sends is changed.
//!
//! Hooks (`EXES`): the call to the host's check handler (which client) and the handler's call that
//! adds one differing archive to the repair (which object). AI caches, visibility and modifiers are
//! repaired silently by the game every few turns: they are not noted. Test checks are left alone.
//!
//! Setting `sync_notes=off` in `rollover_proxy.cfg` installs nothing. Addresses of both exes of
//! 2026-11-05, checked at startup.

use crate::exe::rd;
use crate::{log, patch, win};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Base of the exe's Ghidra addresses.
const BASE: u64 = 0x1_4000_0000;

struct Exe {
    name: &'static str,
    /// The call to the host's handler in the loop over received checks, and the handler.
    check_call: u64,
    handler: u64,
    /// The handler's call that adds one archive to the repair, and that function.
    repair_call: u64,
    add_repair: u64,
}

const EXES: [Exe; 2] = [
    Exe { name: "DX12", check_call: 0x1_407f_9cf6, handler: 0x1_407f_65b0, repair_call: 0x1_407f_717d, add_repair: 0x1_407f_89f0 },
    Exe { name: "DX11", check_call: 0x1_407f_7e06, handler: 0x1_407f_46c0, repair_call: 0x1_407f_528d, add_repair: 0x1_407f_6b00 },
];

/// Around the handler call: `mov rdx,rdi ; mov rcx,rsi`, then `test al,al ; jz`.
const CHECK_BEFORE: [u8; 6] = [0x48, 0x8b, 0xd7, 0x48, 0x8b, 0xce];
const CHECK_AFTER: [u8; 4] = [0x84, 0xc0, 0x0f, 0x84];
/// Before the add call, its arguments: `mov rax,[rbx+8] ; lea r9,[r14+10h] ; mov rsi,[rbp+3b0h] ;
/// lea r8,[rbp-20h] ; mov [rsp+28h],rax ; lea rdx,[rbp+230h] ; mov rax,[rbx] ; mov rcx,rsi ;
/// mov [rsp+20h],rax`. After it, `mov rdi,[rbp+3b8h]`: its result is not used.
const REPAIR_BEFORE: [u8; 42] = [
    0x48, 0x8b, 0x43, 0x08, 0x4d, 0x8d, 0x4e, 0x10, 0x48, 0x8b, 0xb5, 0xb0, 0x03, 0x00, 0x00, 0x4c, 0x8d, 0x45, 0xe0, 0x48, 0x89,
    0x44, 0x24, 0x28, 0x48, 0x8d, 0x95, 0x30, 0x02, 0x00, 0x00, 0x48, 0x8b, 0x03, 0x48, 0x8b, 0xce, 0x48, 0x89, 0x44, 0x24, 0x20,
];
const REPAIR_AFTER: [u8; 7] = [0x48, 0x8b, 0xbd, 0xb8, 0x03, 0x00, 0x00];
/// First bytes of the handler and of the add function.
const HANDLER_START: [u8; 15] = [0x48, 0x89, 0x54, 0x24, 0x10, 0x48, 0x89, 0x4c, 0x24, 0x08, 0x55, 0x56, 0x57, 0x41, 0x56];
const ADD_START: [u8; 16] = [0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x6c, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57];

/// The check message: the client's player ID, and the test flag.
const MSG_PLAYER: usize = 0x24;
const MSG_TEST: usize = 0x38;
/// The archive: begin and end of its type name.
const ARCHIVE_NAME: usize = 0x60;
const ARCHIVE_NAME_END: usize = 0x68;
/// Notes kept while `partie.lua` does not read them.
const MAX_NOTES: usize = 64;

/// The `sync_notes` setting (last occurrence wins; `on` by default).
fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("sync_notes=")).last().map(str::trim) != Some("off")
}

/// What a differing archive is called in the chat, or `None` for those the game repairs silently
/// all the time (AI caches, visibility, modifiers).
pub fn category(type_name: &str) -> Option<&'static str> {
    if type_name.starts_with("AI::") || type_name.starts_with("Visibility") || type_name.starts_with("DynamicModifier") {
        return None;
    }
    Some(match type_name {
        "Unit" | "Formation" | "Player::Units" => "units",
        "Control::Contract" => "contracts",
        "Map" => "map",
        t if t.starts_with("City") || t.starts_with("District") || t == "Player::Cities" => "cities",
        t if t.starts_with("Player") => "players",
        _ => "game state",
    })
}

#[derive(Default)]
struct Notes {
    /// The check being handled: the client, its major archives, their categories.
    current: Option<(i32, u32, Vec<&'static str>)>,
    /// Per client, checks in a row with a major difference.
    streaks: HashMap<i32, u32>,
    /// For `partie.lua`: "player|major archives|checks in a row|category, category".
    notes: Vec<String>,
}

impl Notes {
    fn begin(&mut self, player: i32) {
        self.current = Some((player, 0, Vec::new()));
    }

    fn differs(&mut self, type_name: &str) {
        let Some((_, major, categories)) = self.current.as_mut() else { return };
        if let Some(c) = category(type_name) {
            *major += 1;
            if !categories.contains(&c) {
                categories.push(c);
            }
        }
    }

    /// The check is over; `handled` is false when the handler kept it queued (nothing compared).
    fn end(&mut self, handled: bool) {
        let Some((player, major, categories)) = self.current.take() else { return };
        if !handled {
            return;
        }
        if major == 0 {
            self.streaks.remove(&player);
            return;
        }
        let streak = self.streaks.entry(player).or_insert(0);
        *streak += 1;
        if self.notes.len() < MAX_NOTES {
            self.notes.push(format!("{player}|{major}|{streak}|{}", categories.join(", ")));
        }
    }

    fn take(&mut self) -> String {
        std::mem::take(&mut self.notes).join(";")
    }
}

static HANDLER: AtomicUsize = AtomicUsize::new(0);
static ADD_REPAIR: AtomicUsize = AtomicUsize::new(0);
static NOTES: Mutex<Option<Notes>> = Mutex::new(None);

fn with_notes<R>(f: impl FnOnce(&mut Notes) -> R) -> R {
    let mut guard = NOTES.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(Notes::default))
}

/// A new game (the in-game interface loads).
pub fn new_game() {
    *NOTES.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// `Civ6Proxy_SyncNotes()` → the notes since the last call, separated by ";" (empty when none).
/// Only the host has any.
pub(crate) unsafe extern "C-unwind" fn lua_notes(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    crate::options::push_str(h, l, &with_notes(Notes::take));
    1
}

type Handler = unsafe extern "system" fn(usize, usize) -> u64;
type AddRepair = unsafe extern "system" fn(usize, usize, usize, usize, u64, usize);

/// The archive's type name, "Control::Contract" for example.
unsafe fn type_name(archive: usize) -> Option<String> {
    if archive == 0 {
        return None;
    }
    let begin = rd::<usize>(archive + ARCHIVE_NAME);
    let end = rd::<usize>(archive + ARCHIVE_NAME_END);
    if begin == 0 || end < begin || end - begin > 128 {
        return None;
    }
    Some(String::from_utf8_lossy(std::slice::from_raw_parts(begin as *const u8, end - begin)).into_owned())
}

/// Replaces the loop's call to the host's handler of one client's check.
unsafe extern "system" fn on_check(sync: usize, message: usize) -> u64 {
    let original: Handler = std::mem::transmute(HANDLER.load(Ordering::Relaxed));
    if rd::<u8>(message + MSG_TEST) != 0 {
        return original(sync, message);
    }
    with_notes(|n| n.begin(rd::<i32>(message + MSG_PLAYER)));
    let result = original(sync, message);
    with_notes(|n| n.end(result as u8 != 0));
    result
}

/// Replaces the handler's call that adds one differing archive to the repair.
unsafe extern "system" fn on_repair(sync: usize, repair: usize, serializer: usize, entry: usize, key: u64, archive: usize) {
    let original: AddRepair = std::mem::transmute(ADD_REPAIR.load(Ordering::Relaxed));
    if let Some(name) = type_name(archive) {
        with_notes(|n| n.differs(&name));
    }
    original(sync, repair, serializer, entry, key, archive);
}

/// Checks that each call and each function is where and what this table says; `read(address, n)`
/// returns n bytes (fewer when out of the image).
fn fingerprint<'a>(exe: &Exe, read: impl Fn(u64, usize) -> &'a [u8]) -> Result<(), String> {
    let calls: [(&str, u64, u64, &[u8], &[u8]); 2] = [
        ("check call", exe.check_call, exe.handler, &CHECK_BEFORE, &CHECK_AFTER),
        ("repair call", exe.repair_call, exe.add_repair, &REPAIR_BEFORE, &REPAIR_AFTER),
    ];
    for (name, call, target, before, after) in calls {
        if read(call - before.len() as u64, before.len()) != before {
            return Err(format!("{name}: unexpected bytes before {call:#x}"));
        }
        let bytes = read(call, 5);
        if bytes.len() != 5 || bytes[0] != 0xe8 {
            return Err(format!("{name}: no call at {call:#x}"));
        }
        let rel = i32::from_le_bytes(bytes[1..5].try_into().unwrap());
        if (call + 5).wrapping_add_signed(i64::from(rel)) != target {
            return Err(format!("{name}: {call:#x} does not call {target:#x}"));
        }
        if read(call + 5, after.len()) != after {
            return Err(format!("{name}: unexpected bytes after {call:#x}"));
        }
    }
    for (name, at, start) in [("handler", exe.handler, &HANDLER_START[..]), ("add function", exe.add_repair, &ADD_START[..])] {
        if read(at, start.len()) != start {
            return Err(format!("{name}: unexpected bytes at {at:#x}"));
        }
    }
    Ok(())
}

/// Called by DllMain, before the game runs: redirects both calls.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("sync notes: not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log("sync notes: turned off (sync_notes=off)");
        return;
    }
    let module = win::GetModuleHandleW(std::ptr::null()) as usize;
    let image = |address: u64| module + (address - BASE) as usize;
    let mut reasons = Vec::new();
    let Some(exe) = EXES.iter().find(|exe| {
        fingerprint(exe, |address, n| std::slice::from_raw_parts(image(address) as *const u8, n))
            .map_err(|e| reasons.push(format!("{}: {e}", exe.name)))
            .is_ok()
    }) else {
        log(&format!("sync notes: unknown exe ({}), not installed", reasons.join("; ")));
        return;
    };
    let Some(page) = patch::alloc_near(module) else {
        log("sync notes: no memory within 2 GB of the exe, not installed");
        return;
    };
    let calls = [(exe.check_call, on_check as *const () as usize), (exe.repair_call, on_repair as *const () as usize)];
    let mut rels = [0i32; 2];
    for (i, (call, hook)) in calls.iter().enumerate() {
        // Relay: jmp [rip+0] ; hook. A jump, not a call: the hook gets the original call's stack.
        let relay = page + i * 0x20;
        let mut code = vec![0xff, 0x25, 0, 0, 0, 0];
        code.extend_from_slice(&(*hook as u64).to_le_bytes());
        std::ptr::copy_nonoverlapping(code.as_ptr(), relay as *mut u8, code.len());
        let Some(rel) = patch::rel32(image(*call), relay) else {
            log("sync notes: relay out of range, not installed");
            return;
        };
        rels[i] = rel;
    }
    HANDLER.store(image(exe.handler), Ordering::Relaxed);
    ADD_REPAIR.store(image(exe.add_repair), Ordering::Relaxed);
    for ((call, _), rel) in calls.iter().zip(rels) {
        patch::write_rel32(image(*call), rel);
    }
    log(&format!("sync notes: installed ({})", exe.name));
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";
    const FILES: [&str; 2] = ["CivilizationVI_DX12.exe", "CivilizationVI.exe"];

    /// File offset of a virtual address, from the PE section table.
    fn offset(file: &[u8], address: u64) -> Option<usize> {
        let rd32 = |o: usize| u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
        let rd16 = |o: usize| u16::from_le_bytes(file[o..o + 2].try_into().unwrap()) as usize;
        let nt = rd32(0x3c);
        let sections = nt + 24 + rd16(nt + 20);
        let rva = (address - BASE) as usize;
        (0..rd16(nt + 6)).map(|i| sections + 40 * i).find_map(|s| {
            let (va, size, raw) = (rd32(s + 12), rd32(s + 16), rd32(s + 20));
            (rva >= va && rva < va + size).then(|| rva - va + raw)
        })
    }

    /// Each table matches its own installed exe, and only that one.
    #[test]
    fn bytes_in_installed_exes() {
        for (i, name) in FILES.iter().enumerate() {
            let Ok(file) = std::fs::read(format!(r"{EXE_DIR}\{name}")) else {
                eprintln!("{name} missing, test skipped");
                continue;
            };
            let read = |address: u64, n: usize| offset(&file, address).map_or(&[][..], |o| &file[o..][..n]);
            for (j, exe) in EXES.iter().enumerate() {
                let r = fingerprint(exe, read);
                if i == j {
                    assert_eq!(r, Ok(()), "{} on {name}", exe.name);
                } else {
                    assert!(r.is_err(), "the {} table must not match {name}", exe.name);
                }
            }
        }
    }

    #[test]
    fn silent_and_major_archives() {
        for silent in ["AI::CityBuild", "AI::CultureAi", "Visibility::Instance", "DynamicModifier"] {
            assert_eq!(category(silent), None, "{silent}");
        }
        assert_eq!(category("Unit"), Some("units"));
        assert_eq!(category("Player::Units"), Some("units"));
        assert_eq!(category("Control::Contract"), Some("contracts"));
        assert_eq!(category("City"), Some("cities"));
        assert_eq!(category("PlayerCulture"), Some("players"));
        assert_eq!(category("Game::Instance"), Some("game state"));
    }

    #[test]
    fn notes_only_major_differences() {
        let mut n = Notes::default();
        n.begin(3);
        n.differs("AI::CityBuild");
        n.differs("Visibility::Instance");
        n.end(true);
        assert_eq!(n.take(), "", "silent repairs are not noted");
        n.begin(3);
        n.differs("Unit");
        n.differs("Unit");
        n.differs("Control::Contract");
        n.differs("AI::Operation");
        n.end(true);
        n.begin(3);
        n.differs("City");
        n.end(true);
        assert_eq!(n.take(), "3|3|1|units, contracts;3|1|2|cities");
        n.begin(3);
        n.differs("Unit");
        n.end(false);
        assert_eq!(n.take(), "", "a check kept queued was not compared");
        n.begin(3);
        n.end(true);
        n.begin(3);
        n.differs("Unit");
        n.end(true);
        assert_eq!(n.take(), "3|1|1|units", "a clean check ends the streak");
    }

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("sync_notes=off\r\n"));
        assert!(enabled("sync_notes=off\nsync_notes=on\n"), "the last occurrence wins");
    }
}
