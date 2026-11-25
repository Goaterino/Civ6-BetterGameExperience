//! Joueurs absents sous macOS, le pendant d'absent_turn.rs et de la partie « réseau » de salon.rs :
//!
//! - `Civ6Proxy_EndTurnFor(player)` pour `partie.lua` (bloc commun, salon_texte.rs) : l'hôte termine
//!   le tour d'un joueur humain qui n'est plus connecté (son slot reste humain : le proxy de GameCore,
//!   proxy/src/drop_slot_mac.rs). Comme sous Windows, par le « send turn complete » du contrôleur de
//!   partie de l'exe, qui envoie le `PlayerTurnComplete` synchronisé.
//! - `BgdPartieReseau`, lu par proxy/src/absent_reactivation_mac.rs, et `Civ6Proxy_PartieReseau(actif)`
//!   pour `chargement.lua`, qui le pose à chaque écran de chargement.
//!
//! Les natives macOS rendent une seule chaîne (macos_options.rs) : `Civ6Proxy_MacEndTurnFor` rend
//! `1` ou `0`, et `CALE_PARTIE`, placée devant le bloc de `partie.lua`, rend la fonction qu'il attend.
//!
//! Relevé dans Civ6_Exe_Child (Civ6 Aspyr 1.4.6), adresses de l'exe (base `0x100000000`) :
//! - arm64 : « send turn complete » `0x10137c864(contrôleur, joueur, paramètres)` (chaîne
//!   « SendTurnComplete() ignored request because player %i was not turn active. », rend 1 si la
//!   fin de tour est envoyée ; paramètres nuls admis : `vtable + 0x1f8` au lieu de `+0x200`), trois
//!   appelants comme sous Windows (le joueur local `0x10137e54c`, la répartition des événements de
//!   l'interface `0x101363050`, le minuteur de tour `0x10137c578`, « Turn Timer Sending
//!   TurnComplete »). Contrôleur rendu (et créé au besoin, 0x4b0 octets) par `0x101380000`, rangé
//!   dans `[0x10e429150]` (`adrp x21` / `ldr x8, [x21, #0x150]`), lu ici sans le créer ;
//! - x86_64 : « send turn complete » `0x101421f30` (appelants `0x101407c95`, `0x101421c83`,
//!   `0x101423cb8`), contrôleur `0x1014258f0`, rangé dans `[0x10f2473d0]` (`cmp qword [rip + …], 0`).

use super::journal::log;
use super::macho;
use super::options::{argument_n, lire, rendre, LuaCFn};
use super::salon_texte::actif_off;
use std::ffi::{c_int, c_void};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Base des adresses de l'exe.
const BASE_EXE: usize = 0x1_0000_0000;

struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    empreinte: u64,
}

/// Fonctions entières (jusqu'au début de la suivante, `LC_FUNCTION_STARTS`).
const ARM_ENVOI: Fonction = Fonction { nom: "send turn complete", adresse: 0x1_0137_c864, taille: 0x430, empreinte: 0x61f2_af69_2bce_e36d };
const ARM_CONTROLEUR: Fonction = Fonction { nom: "contrôleur de partie", adresse: 0x1_0138_0000, taille: 0x90, empreinte: 0x9ce1_b3c6_96e9_8fe9 };
const ARM_GLOBALE: usize = 0x1_0e42_9150;
const X86_ENVOI: Fonction = Fonction { nom: "send turn complete", adresse: 0x1_0142_1f30, taille: 0x430, empreinte: 0x6b01_168e_12c0_b2eb };
const X86_CONTROLEUR: Fonction = Fonction { nom: "contrôleur de partie", adresse: 0x1_0142_58f0, taille: 0x90, empreinte: 0xf1bb_f8a2_8bb8_2c7c };
const X86_GLOBALE: usize = 0x1_0f24_73d0;

const X86: bool = cfg!(target_arch = "x86_64");
const ENVOI: Fonction = if X86 { X86_ENVOI } else { ARM_ENVOI };
const CONTROLEUR: Fonction = if X86 { X86_CONTROLEUR } else { ARM_CONTROLEUR };
const GLOBALE: usize = if X86 { X86_GLOBALE } else { ARM_GLOBALE };

static SEND: AtomicUsize = AtomicUsize::new(0);
/// Adresse du pointeur vers le contrôleur de partie.
static CONTROLLER: AtomicUsize = AtomicUsize::new(0);

/// La partie chargée est en réseau, en direct (ni solo, ni hotseat, ni PlayByCloud). Posé par
/// `chargement.lua` à chaque écran de chargement. Non tant que l'interface n'a rien dit.
static RESEAU: AtomicBool = AtomicBool::new(false);

/// Lu par le proxy de GameCore (proxy/src/absent_reactivation_mac.rs).
#[no_mangle]
pub extern "C" fn BgdPartieReseau() -> u32 {
    RESEAU.load(Ordering::Relaxed) as u32
}

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

/// Un nombre Lua passé en chaîne (`3`, ou `3.0`).
fn nombre(v: &str) -> Option<f64> {
    v.trim().parse::<f64>().ok()
}

/// `Civ6Proxy_PartieReseau(actif)`, actif 1 ou 0. Journalisé quand il change.
pub(crate) unsafe extern "C-unwind" fn partie_reseau(l: *mut c_void) -> c_int {
    let actif = nombre(&argument_n(l, 1)) == Some(1.0);
    if RESEAU.swap(actif, Ordering::Relaxed) != actif {
        log(&format!("chargement : partie {}", if actif { "en réseau" } else { "locale ou PlayByCloud" }));
    }
    0
}

/// Termine le tour de `player` s'il est actif. Faux sans contrôleur de partie.
unsafe fn end_turn(player: i32) -> bool {
    let (send, global) = (SEND.load(Ordering::Relaxed), CONTROLLER.load(Ordering::Relaxed));
    if send == 0 || global == 0 || !(0..64).contains(&player) {
        return false;
    }
    let controller = std::ptr::read_unaligned(global as *const usize);
    if controller == 0 {
        return false;
    }
    let send: unsafe extern "C" fn(usize, i32, usize) -> u64 = std::mem::transmute(send);
    send(controller, player, 0) as u8 != 0
}

/// `Civ6Proxy_MacEndTurnFor(player)` → `1` si le tour du joueur était actif et que sa fin a été
/// envoyée, sinon `0`.
pub(crate) unsafe extern "C-unwind" fn mac_end_turn(l: *mut c_void) -> c_int {
    let sent = nombre(&argument_n(l, 1)).is_some_and(|p| p.fract() == 0.0 && end_turn(p as i32));
    rendre(l, if sent { "1" } else { "0" })
}

/// Natives de `partie.lua`, à poser avec celles de macos_salon.rs (`NATIVES_PARTIE`).
pub(crate) const NATIVES_PARTIE: [(LuaCFn, &str); 1] = [(mac_end_turn, "Civ6Proxy_MacEndTurnFor\0")];
/// Native de `chargement.lua`, à poser avec celles de macos_pol.rs (`NATIVES`).
pub(crate) const NATIVES_CHARGEMENT: [(LuaCFn, &str); 1] = [(partie_reseau, "Civ6Proxy_PartieReseau\0")];

/// Cale placée devant le bloc de `partie.lua` (avec `CALE_PARTIE` de macos_salon.rs) : la fonction
/// qu'il attend, sur la native macOS.
pub(crate) const CALE_PARTIE: &str = r#"
-- macOS : natives à une seule chaîne (startup_proxy/src/macos_absent.rs).
if Civ6Proxy_MacEndTurnFor ~= nil then
	Civ6Proxy_EndTurnFor = function(player) return tonumber(Civ6Proxy_MacEndTurnFor(player)) or 0; end
end
"#;

/// Vérifie les deux fonctions de l'exe et note leurs adresses. Rien dans l'exe n'est modifié.
pub unsafe fn installer(_mh: *const macho::MachHeader64, slide: isize) -> Result<(), String> {
    if lire().is_ok_and(|cfg| actif_off(&cfg)) {
        return Err("non posé (actif=off)".into());
    }
    let image = |a: usize| (a as isize + slide) as usize;
    for f in [&ENVOI, &CONTROLEUR] {
        if fnv(std::slice::from_raw_parts(image(f.adresse) as *const u8, f.taille)) != f.empreinte {
            return Err(format!("exe inattendu ({} en {:#x}), non posé", f.nom, f.adresse));
        }
    }
    CONTROLLER.store(image(GLOBALE), Ordering::Relaxed);
    SEND.store(image(ENVOI.adresse), Ordering::Relaxed);
    log(&format!("absent : l'hôte peut terminer le tour d'un joueur déconnecté (exe {:#x})", ENVOI.adresse - BASE_EXE));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exe() -> Option<Vec<u8>> {
        let home = std::env::var_os("HOME")?;
        let p = std::path::Path::new(&home).join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/Civ6_Exe_Child");
        std::fs::read(p).ok()
    }

    /// Tranche `cpu` d'un binaire universel.
    fn tranche(b: &[u8], cpu: u32) -> Option<&[u8]> {
        let be = |o: usize| b.get(o..o + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap()));
        if be(0)? != 0xcafe_babe {
            return None;
        }
        (0..be(4)? as usize).find_map(|i| {
            let e = 8 + 20 * i;
            (be(e)? == cpu).then(|| b.get(be(e + 8)? as usize..(be(e + 8)? + be(e + 12)?) as usize)).flatten()
        })
    }

    /// `__TEXT` commence au début du fichier : décalage = adresse - base.
    fn fonction(t: &[u8], a: usize, n: usize) -> &[u8] {
        &t[a - BASE_EXE..a - BASE_EXE + n]
    }

    #[test]
    fn exe_arm64() {
        let Some(b) = exe() else { return };
        let t = tranche(&b, 0x0100_000c).expect("tranche arm64");
        for f in [&ARM_ENVOI, &ARM_CONTROLEUR] {
            let e = fnv(fonction(t, f.adresse, f.taille));
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
        }
        // La globale du contrôleur : adrp x21, page ; ldr x8, [x21, #off].
        let mot = |a: usize| u32::from_le_bytes(fonction(t, a, 4).try_into().unwrap());
        let (adrp, ldr) = (mot(ARM_CONTROLEUR.adresse + 0x10), mot(ARM_CONTROLEUR.adresse + 0x14));
        assert_eq!(adrp & 0x9f00_001f, 0x9000_0015, "adrp x21");
        let imm = (((adrp >> 29) & 3) | (((adrp >> 5) & 0x7ffff) << 2)) as i64;
        let imm = if imm & (1 << 20) != 0 { imm - (1 << 21) } else { imm };
        let page = ((ARM_CONTROLEUR.adresse as i64 + 0x10) & !0xfff) + (imm << 12);
        assert_eq!(ldr & 0xffc0_03ff, 0xf940_02a8, "ldr x8, [x21, #…]");
        assert_eq!(page as usize + (((ldr >> 10) & 0xfff) << 3) as usize, ARM_GLOBALE);
        // Les trois appelants de l'envoi (deux `b`, un `bl`).
        for (site, bl) in [(0x1_0137_e54c, false), (0x1_0136_3050, false), (0x1_0137_c578, true)] {
            let m = mot(site);
            assert_eq!(m & 0xfc00_0000, if bl { 0x9400_0000 } else { 0x1400_0000 }, "{site:#x}");
            let imm = ((m & 0x03ff_ffff) << 6) as i32 >> 6;
            assert_eq!((site as i64 + imm as i64 * 4) as usize, ARM_ENVOI.adresse, "{site:#x}");
        }
    }

    #[test]
    fn exe_x86_64() {
        let Some(b) = exe() else { return };
        let t = tranche(&b, 0x0100_0007).expect("tranche x86_64");
        for f in [&X86_ENVOI, &X86_CONTROLEUR] {
            let e = fnv(fonction(t, f.adresse, f.taille));
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
        }
        // cmp qword [rip + disp32], 0 en +0xa (8 octets).
        let a = X86_CONTROLEUR.adresse + 0xa;
        let i = fonction(t, a, 8);
        assert_eq!(&i[..3], &[0x48, 0x83, 0x3d]);
        let disp = i32::from_le_bytes(i[3..7].try_into().unwrap());
        assert_eq!((a as i64 + 8 + disp as i64) as usize, X86_GLOBALE);
        // Les trois appelants de l'envoi (call ou jmp rel32).
        for site in [0x1_0140_7c95usize, 0x1_0142_1c83, 0x1_0142_3cb8] {
            let i = fonction(t, site, 5);
            assert!(i[0] == 0xe8 || i[0] == 0xe9, "{site:#x}");
            let rel = i32::from_le_bytes(i[1..5].try_into().unwrap());
            assert_eq!((site as i64 + 5 + rel as i64) as usize, X86_ENVOI.adresse, "{site:#x}");
        }
    }

    #[test]
    fn natives_attendues_par_les_blocs() {
        use super::super::salon_texte::{CHARGEMENT_BLOCK, PARTIE_BLOCK};
        assert!(PARTIE_BLOCK.contains("Civ6Proxy_EndTurnFor(id)"), "partie.lua appelle Civ6Proxy_EndTurnFor");
        assert!(CALE_PARTIE.contains("Civ6Proxy_EndTurnFor =") && CALE_PARTIE.contains(NATIVES_PARTIE[0].1.trim_end_matches('\0')));
        assert!(CHARGEMENT_BLOCK.contains(NATIVES_CHARGEMENT[0].1.trim_end_matches('\0')), "chargement.lua appelle Civ6Proxy_PartieReseau");
        assert_eq!(nombre("1"), Some(1.0));
        assert_eq!(nombre("1.0"), Some(1.0));
        assert_eq!(nombre(" 3 "), Some(3.0));
        assert_eq!(nombre("x"), None);
    }

    #[test]
    fn rien_sans_controleur() {
        assert!(!unsafe { end_turn(3) }, "non posé");
        assert!(!unsafe { end_turn(-1) } && !unsafe { end_turn(64) }, "pas de tels joueurs");
        assert_eq!(BgdPartieReseau(), 0, "non tant que l'interface n'a rien dit");
    }
}
