//! Minimap et caméra sur la carte entière dès le début (`ui_carte_entiere=on|off`, on par défaut).
//!
//! L'exe borne la caméra et la minimap au rectangle des cases révélées par le joueur local, plus
//! ses épingles et celles qu'on lui partage (`14007c1b0`, recalculé à chaque image par
//! `GameViewState_PreAsyncSetup`). Il ne prend la carte entière que si rien n'est révélé, ou si la
//! largeur calculée manque de moins de 640 unités du monde à celle de la carte. D'où l'habitude de
//! poser des épingles tout en haut et tout en bas de la carte. Voir minimap/README.md.
//!
//! Le correctif : le `jnc` qui garde le rectangle calculé (`73 69`) devient deux `nop`. Le `jmp`
//! qui suit mène alors toujours à la branche « carte entière ». Le site est trouvé par une
//! signature, unique dans les exe DX12 et DX11. Rien ne change dans GameCore ni sur le réseau :
//! c'est local, sans effet sur les autres joueurs.

use crate::depart::find_sig;
use crate::exe::{rd, text_section};
use crate::{log, win};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(true);
/// Adresse du `jnc` remplacé, pour le remettre au déchargement.
static SITE: AtomicUsize = AtomicUsize::new(0);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// Fin de `14007c1b0` (projet Ghidra civ6exe ; `14007c4b3` dans l'exe DX12 installé, `14007b963`
/// en DX11) : largeur de la carte (`[r14+2cc] - [r14+2c0]`) moins celle du rectangle calculé, fois
/// 1/64, comparée à 10.0.
const SIG: [Option<u8>; 47] = {
    const fn b(x: u8) -> Option<u8> {
        Some(x)
    }
    const N: Option<u8> = None;
    [
        // movss xmm1,[r14+0x2cc]
        b(0xf3), b(0x41), b(0x0f), b(0x10), b(0x8e), b(0xcc), b(0x02), b(0x00), b(0x00),
        // subss xmm1,[r14+0x2c0]
        b(0xf3), b(0x41), b(0x0f), b(0x5c), b(0x8e), b(0xc0), b(0x02), b(0x00), b(0x00),
        // movss xmm0,[rbp+0x20]
        b(0xf3), b(0x0f), b(0x10), b(0x45), b(0x20),
        // subss xmm0,[rbp+0x18]
        b(0xf3), b(0x0f), b(0x5c), b(0x45), b(0x18),
        // subss xmm1,xmm0
        b(0xf3), b(0x0f), b(0x5c), b(0xc8),
        // mulss xmm1,[rip+…] (1/64)
        b(0xf3), b(0x0f), b(0x59), b(0x0d), N, N, N, N,
        // comiss xmm1,[rip+…] (10.0)
        b(0x0f), b(0x2f), b(0x0d), N, N, N, N,
    ]
};
/// Décalage du `jnc` depuis le début de la signature.
const SAUT: usize = SIG.len();
/// `jnc +0x69` (garde le rectangle calculé) puis `jmp +0x0e` (carte entière).
const ORIGINAL: [u8; 4] = [0x73, 0x69, 0xeb, 0x0e];
const NOPS: [u8; 2] = [0x90, 0x90];

/// Le `jnc`, d'origine ou déjà remplacé (proxy rechargé sans avoir pu le remettre).
unsafe fn trouver(exe: usize) -> Result<usize, String> {
    let (start, size) = text_section(exe).ok_or("section .text de l'exe introuvable")?;
    let text = std::slice::from_raw_parts(start as *const u8, size);
    let at = match &find_sig(text, &SIG)[..] {
        [i] if i + SAUT + ORIGINAL.len() <= size => start + i + SAUT,
        v => return Err(format!("bornes de la vue trouvées {} fois au lieu d'une", v.len())),
    };
    let have = rd::<[u8; 4]>(at);
    let tete_connue = have[..2] == ORIGINAL[..2] || have[..2] == NOPS;
    if !tete_connue || have[2..] != ORIGINAL[2..] {
        return Err(format!("octets inattendus après la comparaison : {have:02x?}"));
    }
    Ok(at)
}

unsafe fn ecrire(at: usize, octets: &[u8; 2]) -> bool {
    let mut old = 0u32;
    if win::VirtualProtect(at as *mut c_void, octets.len(), win::PAGE_EXECUTE_READWRITE, &mut old) == 0 {
        return false;
    }
    std::ptr::copy_nonoverlapping(octets.as_ptr(), at as *mut u8, octets.len());
    win::VirtualProtect(at as *mut c_void, octets.len(), old, &mut old);
    win::FlushInstructionCache(win::GetCurrentProcess(), at as *const c_void, octets.len());
    true
}

pub unsafe fn on_gamecore_loaded() {
    let exe = win::GetModuleHandleW(std::ptr::null()) as usize;
    let at = match trouver(exe) {
        Ok(at) => at,
        Err(e) => {
            log(&format!("carte : minimap et caméra laissées telles quelles : {e}"));
            return;
        }
    };
    let on = ENABLED.load(Ordering::Relaxed);
    let voulu = if on { NOPS } else { [ORIGINAL[0], ORIGINAL[1]] };
    if rd::<[u8; 2]>(at) != voulu && !ecrire(at, &voulu) {
        log(&format!("carte : écriture impossible en exe+{:#x}, erreur {}", at - exe, win::GetLastError()));
        return;
    }
    if on {
        SITE.store(at, Ordering::Relaxed);
        log(&format!("carte : minimap et caméra sur la carte entière (exe+{:#x})", at - exe));
    } else {
        log("carte : réglage coupé (ui_carte_entiere=off), bornes d'origine");
    }
}

/// Le réglage est relu au prochain chargement : on remet le `jnc` d'origine.
pub unsafe fn on_proxy_unload() {
    let at = SITE.swap(0, Ordering::Relaxed);
    if at != 0 {
        ecrire(at, &[ORIGINAL[0], ORIGINAL[1]]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXES: [&str; 2] = [
        r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam\CivilizationVI_DX12.exe",
        r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam\CivilizationVI.exe",
    ];

    #[test]
    fn signature_unique_dans_les_deux_exe() {
        assert_eq!(SAUT, 0x2f);
        for path in EXES {
            let Ok(data) = std::fs::read(path) else {
                eprintln!("absent, test sauté : {path}");
                continue;
            };
            let hits = find_sig(&data, &SIG);
            assert_eq!(hits.len(), 1, "{path}");
            assert_eq!(&data[hits[0] + SAUT..hits[0] + SAUT + 4], &ORIGINAL, "{path}");
        }
    }
}
