//! Ends the turn of a human player who is not connected.
//!
//! With BGE in every player's game, a player who drops keeps a human slot (the GameCore proxy's
//! `drop_slot`), so the game's AI never plays for it. Nobody is left to end that player's turn, and
//! the game turn only ends once no player has an active turn. The host ends it with the game's own
//! "send turn complete" of the game controller, which sends the synchronized `PlayerTurnComplete`
//! for any player whose turn is active. `partie.lua` decides when: every connected human has ended
//! its turn.
//!
//! Addresses of the four known exes, checked at startup. `actif=off` installs nothing.

use crate::exe::rd;
use crate::{log, win};
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Base of the exe's Ghidra addresses.
const BASE: u64 = 0x1_4000_0000;

struct Exe {
    name: &'static str,
    /// The controller's "send turn complete" (controller, player, parameters).
    send: u64,
    /// The function that returns the game controller, creating it when there is none.
    controller: u64,
}

const EXES: [Exe; 4] = [
    Exe { name: "DX12 1.0.12.74", send: 0x1_400f_77f0, controller: 0x1_400f_4ce0 },
    Exe { name: "DX11 1.0.12.74", send: 0x1_400f_6ab0, controller: 0x1_400f_3f90 },
    Exe { name: "DX12 1.0.12.58", send: 0x1_400f_2ff0, controller: 0x1_400f_04d0 },
    Exe { name: "DX11 1.0.12.58", send: 0x1_400f_1720, controller: 0x1_400e_ec00 },
];

/// `mov [rsp+18h],rbx ; push rsi ; push rdi ; push r14 ; mov eax,2090h ; call`.
const SEND_START: [u8; 15] = [0x48, 0x89, 0x5c, 0x24, 0x18, 0x56, 0x57, 0x41, 0x56, 0xb8, 0x90, 0x20, 0x00, 0x00, 0xe8];
/// `sub rsp,38h ; mov rax,[controller global]`, then `test rax,rax ; jne`.
const CONTROLLER_START: [u8; 7] = [0x48, 0x83, 0xec, 0x38, 0x48, 0x8b, 0x05];
const CONTROLLER_AFTER: [u8; 5] = [0x48, 0x85, 0xc0, 0x75, 0x54];
const CONTROLLER_GLOBAL_AT: usize = 7;

static SEND: AtomicUsize = AtomicUsize::new(0);
/// Address of the pointer to the game controller.
static CONTROLLER: AtomicUsize = AtomicUsize::new(0);

/// The global the controller function reads, or why this is not that function.
fn fingerprint<'a>(t: &Exe, read: impl Fn(u64, usize) -> &'a [u8]) -> Result<u64, String> {
    if read(t.send, SEND_START.len()) != SEND_START {
        return Err(format!("send turn complete: unexpected bytes at {:#x}", t.send));
    }
    let c = read(t.controller, CONTROLLER_GLOBAL_AT + 4 + CONTROLLER_AFTER.len());
    if c.len() != CONTROLLER_GLOBAL_AT + 4 + CONTROLLER_AFTER.len() || c[..CONTROLLER_GLOBAL_AT] != CONTROLLER_START || c[CONTROLLER_GLOBAL_AT + 4..] != CONTROLLER_AFTER {
        return Err(format!("game controller: unexpected bytes at {:#x}", t.controller));
    }
    let rel = i32::from_le_bytes(c[CONTROLLER_GLOBAL_AT..CONTROLLER_GLOBAL_AT + 4].try_into().unwrap());
    Ok((t.controller + CONTROLLER_GLOBAL_AT as u64 + 4).wrapping_add_signed(i64::from(rel)))
}

/// Ends `player`'s turn if it is active. False without a game controller.
unsafe fn end_turn(player: i32) -> bool {
    let (send, global) = (SEND.load(Ordering::Relaxed), CONTROLLER.load(Ordering::Relaxed));
    if send == 0 || global == 0 || !(0..64).contains(&player) {
        return false;
    }
    let controller: usize = rd(global);
    if controller == 0 {
        return false;
    }
    let send: unsafe extern "system" fn(usize, i32, usize) -> u64 = std::mem::transmute(send);
    send(controller, player, 0) as u8 != 0
}

/// `Civ6Proxy_EndTurnFor(player)` → 1 if the player's turn was active and its end was sent.
pub(crate) unsafe extern "C-unwind" fn lua_end_turn(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let mut len = 0usize;
    let text = (h.to_lstring)(l, 1, &mut len);
    let player = if text.is_null() {
        None
    } else {
        std::str::from_utf8(std::slice::from_raw_parts(text.cast::<u8>(), len)).ok().and_then(|t| t.trim().parse::<f64>().ok())
    };
    let sent = player.is_some_and(|p| end_turn(p as i32));
    (h.push_integer)(l, sent as i32);
    1
}

/// Called by DllMain, before the game runs. Nothing in the exe is changed.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("absent turn: not installed (actif=off)");
        return;
    }
    let module = win::GetModuleHandleW(std::ptr::null()) as usize;
    let image = |address: u64| module + (address - BASE) as usize;
    let mut reasons = Vec::new();
    let found = EXES.iter().filter(|t| crate::exe::for_this_game(t.name)).find_map(|t| {
        fingerprint(t, |address, n| std::slice::from_raw_parts(image(address) as *const u8, n)).map_err(|e| reasons.push(format!("{}: {e}", t.name))).ok().map(|global| (t, global))
    });
    let Some((t, global)) = found else {
        log(&format!("absent turn: unknown exe ({}), not installed", crate::exe::no_table(&reasons)));
        return;
    };
    CONTROLLER.store(image(global), Ordering::Relaxed);
    SEND.store(image(t.send), Ordering::Relaxed);
    log(&format!("absent turn: the host can end the turn of a player who is not connected ({})", t.name));
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";
    const FILES: [&str; 4] = ["CivilizationVI_DX12.exe", "CivilizationVI.exe", "CivilizationVI_DX12.exe", "CivilizationVI.exe"];

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

    /// Each table matches its own exe, and only that one; the controller global is in `.data`.
    #[test]
    fn bytes_in_installed_exes() {
        for (i, name) in FILES.iter().enumerate() {
            let Ok(file) = std::fs::read(crate::test_exe(EXE_DIR, i, name)) else {
                eprintln!("{name} missing, test skipped");
                continue;
            };
            let read = |address: u64, n: usize| offset(&file, address).map_or(&[][..], |o| &file[o..][..n]);
            for (j, t) in EXES.iter().enumerate() {
                let r = fingerprint(t, read);
                if i == j {
                    let global = r.unwrap_or_else(|e| panic!("{} on {name}: {e}", t.name));
                    assert!(global > t.controller + 0x100_0000, "{}: the controller global {global:#x} is not in the data", t.name);
                } else {
                    assert!(r.is_err(), "the {} table must not match {name}", t.name);
                }
            }
        }
    }

    #[test]
    fn nothing_without_a_game() {
        assert!(!unsafe { end_turn(3) }, "not installed");
        assert!(!unsafe { end_turn(-1) } && !unsafe { end_turn(64) }, "no such players");
    }
}
