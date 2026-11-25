//! A player who drops keeps a human slot, sous macOS : le pendant de drop_slot.rs (voir son
//! en-tête). Correctif POL. Règle commune avec Windows (drop_slot_regle.rs).
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image :
//! - `PlayerConnectionChange::Execute` `0x849b14`, entrée 4 (`+0x20`, celle qu'appelle la réception
//!   `0x848528`) de la vtable `0x12e5920` de `N8GameCore8Messages22PlayerConnectionChangeE`, seul
//!   pointeur vers elle (`0x12e5940`). Message : joueur `+0x18` (refusé au-delà de 63), arrivée
//!   `+0x1c`, exclusion `+0x1d`, comme Windows. L'entrée du joueur est
//!   `[[0x1390210] + 0xdd0] + 32 × joueur` ; son état `0x38ad5c(entrée)` (2 par défaut). Arrivée :
//!   état 3 (`0x389144(entrée, 3)` en `0x849b70`) sauf s'il vaut 4. Départ : rien si l'état est 4
//!   ou 1 (le second message d'un départ, comme Windows), sinon notification (texte selon
//!   l'exclusion), `0x389144(entrée, 1)` en `0x849e44`, événements, mise à jour de la partie ;
//! - le setter de SLOT_STATUS `0x389144(entrée, état)`, 21 appelants, clé `[0x13901c4]` comme le
//!   getter `0x38ad5c` ;
//! - `DllCreateGameContext` (`0x416698`, `b 0x521f58`) : son corps `0x521f58`, sans argument, rend
//!   le contexte ; seul appelant ce `b`. Détourné ici pour remettre la mémoire des départs à zéro à
//!   chaque partie (sous Windows, `forward.rs` appelle `reset` ; le relais de proxy/src/macos.rs est
//!   un saut nu, sans place pour un appel).
//!
//! En x86_64 (Mac Intel, Rosetta), le même code : gestionnaire `0x827fc0` (vtable `0x13332e8`,
//! entrée 4 en `0x1333308`), entrée `[[0x13dc0a8] + 0xdd0] + 32 × joueur`, getter `0x37bde0`,
//! setter `0x37a6e0` (clé `[0x13dc05c]`, 21 appelants), appelé en `0x82801b` (état 3) et
//! `0x8283c3` (état 1) ; `DllCreateGameContext` `0x40ecc0` (`push rbp ; mov rbp, rsp ; pop rbp ;
//! jmp 0x516970`), corps `0x516970`.
//!
//! Comme sous Windows : le setter est détourné avant le gestionnaire ; pendant un départ gardé (et
//! seulement là), un état « ordinateur » devient « pris ». L'hôte termine ensuite les tours du
//! joueur absent (startup_proxy/src/macos_absent.rs, `Civ6Proxy_EndTurnFor`).

#[path = "drop_slot_regle.rs"]
mod regle;

use super::journal::log;
use super::{patch, pol};
use regle::{bit, decide, kept, Change};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// Une fonction entière (jusqu'au début de la suivante, `LC_FUNCTION_STARTS`), et la longueur de
/// son prologue recopié dans le trampoline (instructions entières, sans adresse relative).
struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    empreinte: u64,
    prologue: usize,
}

const ARM_GESTIONNAIRE: Fonction =
    Fonction { nom: "PlayerConnectionChange::Execute", adresse: 0x849b14, taille: 0x45c, empreinte: 0x63cd_27d5_6fd2_41a4, prologue: 16 };
const ARM_ETAT: Fonction = Fonction { nom: "état du slot", adresse: 0x389144, taille: 0x194, empreinte: 0xca69_3a77_c544_72da, prologue: 16 };
const ARM_NOUVELLE_PARTIE: Fonction =
    Fonction { nom: "DllCreateGameContext", adresse: 0x521f58, taille: 0x138, empreinte: 0x5b42_5395_0192_4bc7, prologue: 16 };
const ARM_EXPORT: usize = 0x416698;

/// x86_64 : prologues `push rbp ; mov rbp, rsp ; push …` jusqu'au `sub rsp` compris (gestionnaire,
/// 18 octets ; corps de `DllCreateGameContext`, 15), et jusqu'à `mov rdi, [rdi + 0x10]` pour le
/// setter (17), l'instruction suivante lisant `[rip + …]`.
const X86_GESTIONNAIRE: Fonction =
    Fonction { nom: "PlayerConnectionChange::Execute", adresse: 0x827fc0, taille: 0x500, empreinte: 0x3377_1dfb_3dbf_8279, prologue: 18 };
const X86_ETAT: Fonction = Fonction { nom: "état du slot", adresse: 0x37a6e0, taille: 0x140, empreinte: 0x2a56_4880_dffe_e3e3, prologue: 17 };
const X86_NOUVELLE_PARTIE: Fonction =
    Fonction { nom: "DllCreateGameContext", adresse: 0x516970, taille: 0x130, empreinte: 0xe7ef_d8c4_c061_0ed4, prologue: 15 };
const X86_EXPORT: usize = 0x40ecc0;
/// `push rbp ; mov rbp, rsp ; pop rbp ; jmp rel32` : l'export, le saut en +5.
const X86_EXPORT_OCTETS: [u8; 5] = [0x55, 0x48, 0x89, 0xe5, 0x5d];

const X86: bool = cfg!(target_arch = "x86_64");
const GESTIONNAIRE: Fonction = if X86 { X86_GESTIONNAIRE } else { ARM_GESTIONNAIRE };
const ETAT: Fonction = if X86 { X86_ETAT } else { ARM_ETAT };
const NOUVELLE_PARTIE: Fonction = if X86 { X86_NOUVELLE_PARTIE } else { ARM_NOUVELLE_PARTIE };

const MSG_JOUEUR: usize = 0x18;
const MSG_ARRIVEE: usize = 0x1c;
const MSG_EXCLU: usize = 0x1d;

static T_GESTIONNAIRE: AtomicUsize = AtomicUsize::new(0);
static T_ETAT: AtomicUsize = AtomicUsize::new(0);
static T_NOUVELLE_PARTIE: AtomicUsize = AtomicUsize::new(0);
/// The handler is running for a departure whose slot stays human.
static LEAVING: AtomicBool = AtomicBool::new(false);
/// Players whose departure was handled and who are not back (one bit per player).
static AWAY: AtomicU64 = AtomicU64::new(0);

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

unsafe extern "C" fn hook_gestionnaire(msg: *const u8) -> u64 {
    let original: unsafe extern "C" fn(*const u8) -> u64 = std::mem::transmute(T_GESTIONNAIRE.load(Ordering::Relaxed));
    let player = std::ptr::read_unaligned(msg.add(MSG_JOUEUR).cast::<u32>());
    let (connecting, kicked) = (*msg.add(MSG_ARRIVEE) != 0, *msg.add(MSG_EXCLU) != 0);
    let b = bit(player);
    let change = decide(pol::actifs(), connecting, kicked, AWAY.load(Ordering::Relaxed) & b != 0);
    if connecting || kicked {
        AWAY.fetch_and(!b, Ordering::Relaxed);
    }
    match change {
        Change::Game => original(msg),
        Change::Skip => 0,
        Change::KeepHuman => {
            AWAY.fetch_or(b, Ordering::Relaxed);
            LEAVING.store(true, Ordering::Relaxed);
            let r = original(msg);
            LEAVING.store(false, Ordering::Relaxed);
            log(&format!("drop: player {player} left, its slot stays human (no AI takeover)"));
            r
        }
    }
}

unsafe extern "C" fn hook_etat(entree: usize, etat: i32) -> u64 {
    let original: unsafe extern "C" fn(usize, i32) -> u64 = std::mem::transmute(T_ETAT.load(Ordering::Relaxed));
    original(entree, kept(LEAVING.load(Ordering::Relaxed), etat))
}

/// `DllCreateGameContext` : une nouvelle partie, puis l'original.
unsafe extern "C" fn hook_nouvelle_partie() -> usize {
    reset();
    super::depart::reset();
    let original: unsafe extern "C" fn() -> usize = std::mem::transmute(T_NOUVELLE_PARTIE.load(Ordering::Relaxed));
    original()
}

/// A new game.
pub fn reset() {
    AWAY.store(0, Ordering::Relaxed);
    LEAVING.store(false, Ordering::Relaxed);
}

/// L'export `DllCreateGameContext` saute au corps détourné ici.
unsafe fn export_attendu(base: usize) -> bool {
    if X86 {
        let at = base + X86_EXPORT;
        std::slice::from_raw_parts(at as *const u8, 5) == X86_EXPORT_OCTETS
            && patch::cible_rel32_x86(at + 5, std::slice::from_raw_parts((at + 5) as *const u8, 5)) == Some(base + NOUVELLE_PARTIE.adresse)
            && *((at + 5) as *const u8) == 0xe9
    } else {
        let at = base + ARM_EXPORT;
        let mot = std::ptr::read_unaligned(at as *const u32);
        mot & 0xfc00_0000 == 0x1400_0000 && patch::cible_branche(at, mot) == Some(base + NOUVELLE_PARTIE.adresse)
    }
}

pub unsafe fn installer(base: usize) -> Result<String, String> {
    if !export_attendu(base) {
        return Err("version inattendue (DllCreateGameContext), aucun détour".into());
    }
    for f in [&NOUVELLE_PARTIE, &ETAT, &GESTIONNAIRE] {
        if fnv(std::slice::from_raw_parts((base + f.adresse) as *const u8, f.taille)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    // La remise à zéro, puis le setter : le détour du gestionnaire compte sur lui.
    let poses: [(&Fonction, &AtomicUsize, usize); 3] = [
        (&NOUVELLE_PARTIE, &T_NOUVELLE_PARTIE, hook_nouvelle_partie as *const () as usize),
        (&ETAT, &T_ETAT, hook_etat as *const () as usize),
        (&GESTIONNAIRE, &T_GESTIONNAIRE, hook_gestionnaire as *const () as usize),
    ];
    let mut methode = "";
    for (f, tramp, accroche) in poses {
        methode = patch::hook_with_trampoline_publie(base + f.adresse, f.prologue, accroche, tramp)?;
    }
    Ok(format!("un joueur qui part garde un slot humain quand tous ont BGE (POL) ({methode})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gamecore() -> Option<Vec<u8>> {
        let home = std::env::var_os("HOME")?;
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        std::fs::read(p).ok()
    }

    /// Tranche x86_64 d'un binaire universel.
    fn tranche_x86_64(b: &[u8]) -> Option<&[u8]> {
        let be = |o: usize| b.get(o..o + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap()));
        if be(0)? != 0xcafe_babe {
            return None;
        }
        (0..be(4)? as usize).find_map(|i| {
            let e = 8 + 20 * i;
            (be(e)? == 0x0100_0007).then(|| b.get(be(e + 8)? as usize..(be(e + 8)? + be(e + 12)?) as usize)).flatten()
        })
    }

    #[test]
    fn fonctions_du_jeu() {
        let Some(b) = gamecore() else { return };
        let t = super::super::macho::tranche_arm64(&b).expect("tranche arm64");
        let mot = |a: usize| u32::from_le_bytes(t[a..a + 4].try_into().unwrap());
        for f in [&ARM_NOUVELLE_PARTIE, &ARM_ETAT, &ARM_GESTIONNAIRE] {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
            assert!(f.prologue >= 16, "{} : place pour le saut", f.nom);
            // Prologue sans adresse relative (adr/adrp, b/bl, cbz/tbz, b.cond, ldr littéral).
            for k in 0..f.prologue / 4 {
                let m = mot(f.adresse + 4 * k);
                let relatif = m & 0x1f00_0000 == 0x1000_0000
                    || m & 0x7c00_0000 == 0x1400_0000
                    || m & 0x7c00_0000 == 0x3400_0000
                    || m & 0xfe00_0000 == 0x5400_0000
                    || m & 0x3b00_0000 == 0x1800_0000;
                assert!(!relatif, "{} : instruction relative en +{}", f.nom, 4 * k);
            }
        }
        // DllCreateGameContext : `b` vers son corps.
        assert_eq!(mot(ARM_EXPORT) & 0xfc00_0000, 0x1400_0000);
        assert_eq!(patch::cible_branche(ARM_EXPORT, mot(ARM_EXPORT)), Some(ARM_NOUVELLE_PARTIE.adresse));
        // Le gestionnaire est l'entrée 4 de la vtable du message (pointeur chaîné : 36 bits bas).
        let ptr = u64::from_le_bytes(t[0x12e5940..0x12e5948].try_into().unwrap());
        assert_eq!(ptr & 0xf_ffff_ffff, ARM_GESTIONNAIRE.adresse as u64);
        // Les deux appels du setter dans le gestionnaire : état 3 (arrivée), puis 1 (départ).
        assert_eq!(patch::cible_branche(0x849b70, mot(0x849b70)), Some(ARM_ETAT.adresse));
        assert_eq!(mot(0x849b6c), 0x5280_0061, "mov w1, #3");
        assert_eq!(patch::cible_branche(0x849e44, mot(0x849e44)), Some(ARM_ETAT.adresse));
        assert_eq!(mot(0x849e40), 0x5280_0021, "mov w1, #1");
        // Champs du message : ldr w8, [x0, #0x18] ; ldrb w22, [x0, #0x1c] ; ldrb w8, [x19, #0x1d].
        assert_eq!(mot(0x849b2c), 0xb940_1808);
        assert_eq!(mot(0x849b50), 0x3940_7016);
        assert_eq!(mot(0x849cb8), 0x3940_7668);
    }

    #[test]
    fn fonctions_du_jeu_x86_64() {
        let Some(b) = gamecore() else { return };
        let t = tranche_x86_64(&b).expect("tranche x86_64");
        for f in [&X86_NOUVELLE_PARTIE, &X86_ETAT, &X86_GESTIONNAIRE] {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
            assert!(f.prologue >= 14, "{} : place pour le saut", f.nom);
        }
        // Les prologues recopiés, instruction par instruction.
        let prologues: [(&Fonction, &[u8]); 3] = [
            // push rbp ; mov rbp, rsp ; push r15 ; push r14 ; push r12 ; push rbx ; sub rsp, 0x190
            (&X86_GESTIONNAIRE, &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec, 0x90, 0x01, 0x00, 0x00]),
            // push rbp ; mov rbp, rsp ; push r14 ; push rbx ; mov r14d, esi ; mov rbx, rdi ; mov rdi, [rdi + 0x10]
            (&X86_ETAT, &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x56, 0x53, 0x41, 0x89, 0xf6, 0x48, 0x89, 0xfb, 0x48, 0x8b, 0x7f, 0x10]),
            // push rbp ; mov rbp, rsp ; push r15 ; push r14 ; push r12 ; push rbx ; sub rsp, 0x10
            (&X86_NOUVELLE_PARTIE, &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x54, 0x53, 0x48, 0x83, 0xec, 0x10]),
        ];
        for (f, octets) in prologues {
            assert_eq!(octets.len(), f.prologue, "{}", f.nom);
            assert_eq!(&t[f.adresse..f.adresse + f.prologue], octets, "{}", f.nom);
        }
        // DllCreateGameContext : saut vers son corps.
        assert_eq!(&t[X86_EXPORT..X86_EXPORT + 5], &X86_EXPORT_OCTETS);
        assert_eq!(t[X86_EXPORT + 5], 0xe9);
        assert_eq!(patch::cible_rel32_x86(X86_EXPORT + 5, &t[X86_EXPORT + 5..X86_EXPORT + 10]), Some(X86_NOUVELLE_PARTIE.adresse));
        // Entrée 4 de la vtable du message.
        assert_eq!(u64::from_le_bytes(t[0x1333308..0x1333310].try_into().unwrap()) & 0xf_ffff_ffff, X86_GESTIONNAIRE.adresse as u64);
        // Les deux appels du setter : mov esi, 3 puis mov esi, 1.
        for (site, etat) in [(0x82801b, 3u8), (0x8283c3, 1)] {
            assert_eq!(patch::cible_rel32_x86(site, &t[site..site + 5]), Some(X86_ETAT.adresse), "{site:#x}");
            assert_eq!(&t[site - 5..site], &[0xbe, etat, 0, 0, 0], "{site:#x}");
        }
        // Champs du message : mov r14d, [rdi + 0x18] ; movzx r12d, byte [rdi + 0x1c] ; cmp byte [rbx + 0x1d], 0.
        assert_eq!(&t[0x827fd2..0x827fd6], &[0x44, 0x8b, 0x77, 0x18]);
        assert_eq!(&t[0x827ff8..0x827ffd], &[0x44, 0x0f, 0xb6, 0x67, 0x1c]);
        assert_eq!(&t[0x8281b0..0x8281b4], &[0x80, 0x7b, 0x1d, 0x00]);
    }
}
