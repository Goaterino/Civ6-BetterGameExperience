//! Pings stuck at "0 ms" in the player list.
//!
//! The ping shown in the player list (`Network.GetPingTime`) is measured by the Steam transport
//! thread (`FSteamTransportThread`, `140864130` in the DX12 exe): every 5 s it sends each player an
//! "SP" packet and waits for the "Sp" answer; the ping is the time between the two. While an answer
//! is missing, it sends that player no new request and writes instead the time elapsed since the
//! start of the measuring round, which it has just reset: 0 ms. A single unanswered request
//! therefore freezes the link at 0 ms until a reconnection (the code meant to ask again after 10 s,
//! but measures from the wrong moment and never gets there).
//!
//! The other player ignores a request from a Steam account whose connection it has not registered
//! yet ("Received a Steam transport ping request from steamID without a registered connection"):
//! this happens when joining a game, while the links open one after the other. The link works; but
//! players rejoin "to fix the 0 ms", and a reconnection often costs a desync. Seen on 2026-11-11:
//! 0 ms to the host (who can never be relayed) and two directly linked players, 127 ms to a third.
//!
//! The fix removes the "answer pending" jump (`jnz` after `cmp qword [rdi+30h],0`): every measuring
//! round sends a new request, and the ping updates at the next answer. The other players need
//! nothing: they already answer every request from a registered account. Only the machine with BGE
//! sees correct pings.
//!
//! Settings in `rollover_proxy.cfg`: `actif=off` installs nothing (BGE's main switch);
//! `resend_pings=off` turns off this fix alone (`on` by default).

use crate::{log, win};
use std::ffi::c_void;

const BASE: u64 = 0x1_4000_0000;

/// An exe and the address of the "answer pending" comparison in its Steam transport thread.
struct Exe {
    name: &'static str,
    site: u64,
}

/// Exes of 2026-11-05: same code in both, at different addresses.
const EXES: [Exe; 2] = [Exe { name: "DX12", site: 0x1_4086_4ad1 }, Exe { name: "DX11", site: 0x1_4086_2931 }];

/// `cmp qword [rdi+30h],0 ; jnz +19h ; mov dl,1 ; mov byte [rbp+291h],'P' ; xor ecx,ecx ; call`
const EXPECTED: [u8; 19] = [
    0x48, 0x83, 0x7f, 0x30, 0x00, 0x75, 0x19, 0xb2, 0x01, 0xc6, 0x85, 0x91, 0x02, 0x00, 0x00, 0x50, 0x33, 0xc9, 0xe8,
];
/// Offset of the `jnz` in these bytes: replaced by two `nop`.
const JUMP: usize = 5;
const NOPS: [u8; 2] = [0x90, 0x90];

/// The `resend_pings` setting (last occurrence wins; `on` by default).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("resend_pings=")).last().map(str::trim) != Some("off")
}

/// Checks that the expected bytes are at this exe's address; `read(address, n)` returns n bytes.
fn fingerprint<'a>(exe: &Exe, read: impl Fn(u64, usize) -> &'a [u8]) -> Result<(), String> {
    let found = read(exe.site, EXPECTED.len());
    if found != EXPECTED {
        return Err(format!("unexpected bytes at {:#x}: {found:02x?}", exe.site));
    }
    Ok(())
}

unsafe fn write_code(at: usize, bytes: &[u8]) -> bool {
    let mut old = 0u32;
    if win::VirtualProtect(at as *mut c_void, bytes.len(), win::PAGE_EXECUTE_READWRITE, &mut old) == 0 {
        return false;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), at as *mut u8, bytes.len());
    win::VirtualProtect(at as *mut c_void, bytes.len(), old, &mut old);
    win::FlushInstructionCache(win::GetCurrentProcess(), at as *const c_void, bytes.len());
    true
}

/// Called by DllMain: the Steam transport thread is not running yet.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("pings: not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log("pings: turned off (resend_pings=off)");
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
        log(&format!("pings: unknown exe ({}), fix not installed", reasons.join("; ")));
        return;
    };
    if write_code(image(exe.site) + JUMP, &NOPS) {
        log(&format!("pings: resend of unanswered pings installed ({})", exe.name));
    } else {
        log("pings: VirtualProtect failed, fix not installed");
    }
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

    /// Each address matches its own installed exe, and only that one; the bytes occur once in it.
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
                    assert!(r.is_err(), "the {} address must not match {name}", exe.name);
                }
            }
            let count = file.windows(EXPECTED.len()).filter(|w| *w == EXPECTED).count();
            assert_eq!(count, 1, "the comparison bytes must be unique in {name}");
        }
    }

    #[test]
    fn replaced_jump() {
        assert_eq!(EXPECTED[JUMP], 0x75, "short jnz");
        assert_eq!(EXPECTED[JUMP + 1], 0x19, "to the code that writes the 0 ping");
    }

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("resend_pings=off\r\n"));
        assert!(enabled("resend_pings=off\nresend_pings=on\n"), "the last occurrence wins");
    }
}
