//! The turn of a player who left, ended by another machine, stays ended, sous macOS : le pendant
//! d'absent_reactivation.rs (voir son en-tête : le bogue BGE-4EB5B0, le marqueur, le correctif).
//! Correctif POL, en partie réseau seulement. Règle commune avec Windows
//! (absent_reactivation_regle.rs).
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image :
//! - réception `0x848528(émetteur, flux)` (`Messages_Manager`, chaîne « GameCore RECV (%i): %s ») :
//!   lit le message, met l'octet `0x1393a40` à 1 autour de son `Execute` (`vtable + 0x20`), comme
//!   Windows. Deux appelants, comme Windows : la file des messages (`bl` en `0x848c18`, émetteur
//!   `[x20 + 0x10]`) et l'API de l'exe (`b` en `0x522c68`, après `mov x0, x1 ; mov x1, x2`) ;
//! - `PlayerTurnComplete::Execute` `0x84b3f4`, entrée 4 de la vtable `0x12e5b28` de
//!   `N8GameCore8Messages18PlayerTurnCompleteE`, seul pointeur vers elle (`0x12e5b48`) : joueur en
//!   `+0x18` (refusé au-delà de 63), raison en `+0x1c`, pause lue en `jeu + 0x228` (Windows
//!   `+0x220`), « déjà fini » par `0x9f5e10(joueur)`, tour fini par `0x9f5ab0(joueur, 1, raison)` ;
//!   chaînes « PlayerTurnComplete ignored due to game paused. » et « … already turn complete. » ;
//! - l'automatisation de fin de tour `0x9f54cc(joueur)` (chaîne « Player %i Unit %i has movement
//!   remaining after completing automation. Attempting to turn active player. ») : un seul appel
//!   de `AppRequestTurnUnready` (`[[0x1390228]] + 0x68`, `blr` en `0x9f56cc`, requête dans `x21` :
//!   `0x5006dda` automatisation, `0xd60b7e54`, `0xfc731e25`), précédé de la porte `bl 0x8ae624` en
//!   `0x9f56a8` (après `ldr w0, [x19, #0xe0]`, l'ID du joueur ; `cbz w0` ensuite). C'est ce `bl`
//!   qui est redirigé. La porte est `ANNULABLE` de congres_mac.rs (réécrite par lui, d'où
//!   l'empreinte ou son saut acceptés) ; ses deux autres appelants, `0x9f5b7c` et `0x9f5dd4`, ne
//!   sont pas touchés.
//!
//! En x86_64 (Mac Intel, Rosetta), le même code : réception `0x826930` (appelants `0x827067`,
//! `call`, et `0x51767a`, `jmp` de l'API de l'exe), `PlayerTurnComplete::Execute` `0x829940`
//! (vtable `0x13334f0`, entrée 4 en `0x1333510` ; pause en `jeu + 0x228`), automatisation
//! `0x9ee520`, porte `call 0x891aa0` en `0x9eeb0e` (après `mov edi, [rbx + 0xe0]`, suivie de
//! `test al, al ; je`, puis `AppRequestTurnUnready` `call [rax + 0x68]` en `0x9eeb2d`, requête dans
//! `edx`) ; autres appelants de la porte `0x9eecfc` et `0x9eef82`.
//!
//! `pol::partie_reseau` n'existe pas encore dans pol_mac.rs : `partie_reseau()` ci-dessous lit
//! l'export `BgdPartieReseau` du chargeur (startup_proxy/src/macos_absent.rs), posé par
//! `chargement.lua` à chaque écran de chargement. Export absent : non, et le correctif ne joue pas.

#[path = "absent_reactivation_regle.rs"]
mod regle;

use super::journal::log;
use super::{patch, pol};
use regle::keeps_ended;
use std::cell::Cell;
use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// Une fonction entière (jusqu'au début de la suivante, `LC_FUNCTION_STARTS`), et la longueur de
/// son prologue recopié dans le trampoline (instructions entières, sans adresse relative ; 0 pour
/// une fonction qui n'est pas détournée).
struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    empreinte: u64,
    prologue: usize,
}

/// L'appel de la porte dans l'automatisation, et les octets qui l'entourent.
struct Appel {
    site: usize,
    avant: &'static [u8],
    apres: &'static [u8],
}

const ARM_RECEPTION: Fonction = Fonction { nom: "réception", adresse: 0x848528, taille: 0x344, empreinte: 0xf638_f876_001b_230d, prologue: 16 };
const ARM_FIN_DE_TOUR: Fonction =
    Fonction { nom: "PlayerTurnComplete::Execute", adresse: 0x84b3f4, taille: 0x474, empreinte: 0x35c3_1140_8e38_533c, prologue: 16 };
const ARM_AUTOMATISATION: Fonction =
    Fonction { nom: "automatisation de fin de tour", adresse: 0x9f54cc, taille: 0x530, empreinte: 0x42f5_c6e4_0d07_ca74, prologue: 0 };
const ARM_PORTE: Fonction = Fonction { nom: "porte de réactivation", adresse: 0x8ae624, taille: 88, empreinte: 0xadfc_fb48_8f13_5b92, prologue: 0 };
/// `ldr w0, [x19, #0xe0]` ; `bl porte` ; `cbz w0, 0x9f56e8`.
const ARM_APPEL: Appel = Appel { site: 0x9f56a8, avant: &[0x60, 0xe2, 0x40, 0xb9], apres: &[0xe0, 0x01, 0x00, 0x34] };

/// x86_64 : `push rbp ; mov rbp, rsp ; push r15 ; push r14 ; push r13 ; push r12 ; push rbx ;
/// sub rsp, imm32` (20 octets) pour les deux fonctions détournées.
const X86_RECEPTION: Fonction = Fonction { nom: "réception", adresse: 0x826930, taille: 0x3a0, empreinte: 0x4552_5f35_8bd9_227b, prologue: 20 };
const X86_FIN_DE_TOUR: Fonction =
    Fonction { nom: "PlayerTurnComplete::Execute", adresse: 0x829940, taille: 0x510, empreinte: 0x9af8_2924_467d_d6ea, prologue: 20 };
const X86_AUTOMATISATION: Fonction =
    Fonction { nom: "automatisation de fin de tour", adresse: 0x9ee520, taille: 0x670, empreinte: 0x9625_cd09_6bc4_305b, prologue: 0 };
const X86_PORTE: Fonction = Fonction { nom: "porte de réactivation", adresse: 0x891aa0, taille: 0x60, empreinte: 0xf3f0_6f4d_440c_d2f3, prologue: 0 };
/// `mov edi, [rbx + 0xe0]` ; `call porte` ; `test al, al ; je`.
const X86_APPEL: Appel = Appel { site: 0x9eeb0e, avant: &[0x8b, 0xbb, 0xe0, 0x00, 0x00, 0x00], apres: &[0x84, 0xc0, 0x74, 0x3c] };

const X86: bool = cfg!(target_arch = "x86_64");
const RECEPTION: Fonction = if X86 { X86_RECEPTION } else { ARM_RECEPTION };
const FIN_DE_TOUR: Fonction = if X86 { X86_FIN_DE_TOUR } else { ARM_FIN_DE_TOUR };
const AUTOMATISATION: Fonction = if X86 { X86_AUTOMATISATION } else { ARM_AUTOMATISATION };
const PORTE: Fonction = if X86 { X86_PORTE } else { ARM_PORTE };
const APPEL: Appel = if X86 { X86_APPEL } else { ARM_APPEL };

const MSG_JOUEUR: usize = 0x18;

static T_RECEPTION: AtomicUsize = AtomicUsize::new(0);
static T_FIN_DE_TOUR: AtomicUsize = AtomicUsize::new(0);
static GATE: AtomicUsize = AtomicUsize::new(0);
thread_local! {
    /// Sender of the message this thread is receiving.
    static SENDER: Cell<Option<i32>> = const { Cell::new(None) };
    /// Player whose reactivation is refused while this thread executes its message.
    static REFUSED: Cell<Option<i32>> = const { Cell::new(None) };
}
static LOGGED: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

/// Export `BgdPartieReseau` du chargeur : 0 pas encore cherché, 1 absent, sinon son adresse.
static RESEAU: AtomicUsize = AtomicUsize::new(0);

extern "C" {
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}
const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;

/// Partie en réseau en direct (ni solo, ni hotseat, ni PlayByCloud), selon l'export
/// `BgdPartieReseau` du chargeur. Chargeur absent ou sans cet export : non.
pub fn partie_reseau() -> bool {
    let mut p = RESEAU.load(Ordering::Relaxed);
    if p == 0 {
        let f = unsafe { dlsym(RTLD_DEFAULT, c"BgdPartieReseau".as_ptr()) };
        p = if f.is_null() { 1 } else { f as usize };
        if p == 1 {
            log("absent : export BgdPartieReseau absent du chargeur, correctif inactif");
        }
        RESEAU.store(p, Ordering::Relaxed);
    }
    if p == 1 {
        return false;
    }
    let f: extern "C" fn() -> u32 = unsafe { std::mem::transmute(p) };
    f() != 0
}

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

unsafe extern "C" fn hook_reception(sender: i32, stream: usize) -> u64 {
    let original: unsafe extern "C" fn(i32, usize) -> u64 = std::mem::transmute(T_RECEPTION.load(Ordering::Relaxed));
    let previous = SENDER.replace(Some(sender));
    let r = original(sender, stream);
    SENDER.set(previous);
    r
}

unsafe extern "C" fn hook_fin_de_tour(msg: *const u8) -> u64 {
    let original: unsafe extern "C" fn(*const u8) -> u64 = std::mem::transmute(T_FIN_DE_TOUR.load(Ordering::Relaxed));
    let player = std::ptr::read_unaligned(msg.add(MSG_JOUEUR).cast::<i32>());
    if !keeps_ended(SENDER.get(), player, pol::actifs(), partie_reseau()) {
        return original(msg);
    }
    let previous = REFUSED.replace(Some(player));
    let r = original(msg);
    REFUSED.set(previous);
    r
}

/// La porte, pour l'appel de l'automatisation seulement. Rend 0 ou 1 sur 32 bits (`cbz w0`, ou
/// `test al, al` en x86_64).
unsafe extern "C" fn hook_porte(player: i32) -> u32 {
    if REFUSED.get() == Some(player) {
        let k = LOGGED.fetch_add(1, Ordering::Relaxed) + 1;
        if k <= MAX_LOGS {
            log(&format!("absent: player {player}'s turn was ended by another player; the game wanted to make it active again, it stays ended ({k})"));
        }
        return 0;
    }
    let gate: unsafe extern "C" fn(i32) -> u32 = std::mem::transmute(GATE.load(Ordering::Relaxed));
    gate(player)
}

/// A new game: the log starts again. Not called yet on macOS (see the report): at most
/// `MAX_LOGS` lines per process.
#[allow(dead_code)]
pub fn reset() {
    LOGGED.store(0, Ordering::Relaxed);
}

/// Partie fixe du saut de `patch::jump` (avant l'adresse) : `ldr x16, #8 ; br x16`, ou
/// `jmp [rip + 0]`.
fn debut_du_saut() -> Vec<u8> {
    let s = patch::jump(0);
    s[..s.len() - 8].to_vec()
}

/// La porte telle que l'empreinte la connaît, ou déjà réécrite par congres_mac.rs (son saut).
fn porte_attendue(octets: &[u8]) -> bool {
    fnv(octets) == PORTE.empreinte || octets.starts_with(&debut_du_saut())
}

/// Cible de l'appel `at` (un `bl` arm64, un `call rel32` x86_64), ou None.
unsafe fn cible_appel(at: usize) -> Option<usize> {
    if X86 {
        patch::cible_rel32_x86(at, std::slice::from_raw_parts(at as *const u8, 5)).filter(|_| *(at as *const u8) == 0xe8)
    } else {
        let mot = std::ptr::read_unaligned(at as *const u32);
        (mot & 0xfc00_0000 == 0x9400_0000).then(|| patch::cible_branche(at, mot)).flatten()
    }
}

/// L'appel redirigé vers `relais` : un `bl` (4 octets) ou un `call rel32` (5 octets).
fn appel_vers(site: usize, relais: usize) -> Option<Vec<u8>> {
    if X86 {
        patch::rel32_x86(0xe8, site, relais).map(|c| c.to_vec())
    } else {
        let mut bl = patch::branche(site, relais)?;
        bl[3] |= 0x80; // b → bl
        Some(bl.to_vec())
    }
}

pub unsafe fn installer(base: usize) -> Result<String, String> {
    for f in [&RECEPTION, &FIN_DE_TOUR, &AUTOMATISATION] {
        if fnv(std::slice::from_raw_parts((base + f.adresse) as *const u8, f.taille)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    if !porte_attendue(std::slice::from_raw_parts((base + PORTE.adresse) as *const u8, PORTE.taille)) {
        return Err(format!("version inattendue ({} en {:#x}), aucun détour", PORTE.nom, PORTE.adresse));
    }
    let site = base + APPEL.site;
    let longueur = if X86 { 5 } else { 4 };
    let octets = |a: usize, n: usize| std::slice::from_raw_parts(a as *const u8, n);
    if cible_appel(site) != Some(base + PORTE.adresse)
        || octets(site - APPEL.avant.len(), APPEL.avant.len()) != APPEL.avant
        || octets(site + longueur, APPEL.apres.len()) != APPEL.apres
    {
        return Err(format!("version inattendue (appel de la porte en {:#x}), aucun détour", APPEL.site));
    }
    // Relais à portée de l'appel : saut vers `hook_porte`.
    let portee = if X86 { 1 << 30 } else { 120 << 20 };
    let page = patch::executable_near(&patch::jump(hook_porte as *const () as usize), site, portee)?;
    let appel = appel_vers(site, page).ok_or("relais hors de portée, aucun détour")?;
    GATE.store(base + PORTE.adresse, Ordering::Relaxed);
    // La réception d'abord : l'autre détour lit l'émetteur qu'elle note.
    patch::hook_with_trampoline_publie(base + RECEPTION.adresse, RECEPTION.prologue, hook_reception as *const () as usize, &T_RECEPTION)?;
    patch::hook_with_trampoline_publie(base + FIN_DE_TOUR.adresse, FIN_DE_TOUR.prologue, hook_fin_de_tour as *const () as usize, &T_FIN_DE_TOUR)?;
    let methode = patch::write_code(site, &appel)?;
    Ok(format!("un tour fini par un autre joueur pour un absent reste fini (POL, parties en réseau) ({methode})"))
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
        for f in [&ARM_RECEPTION, &ARM_FIN_DE_TOUR, &ARM_AUTOMATISATION, &ARM_PORTE] {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
        }
        for f in [&ARM_RECEPTION, &ARM_FIN_DE_TOUR] {
            assert!(f.prologue >= 16, "{} : place pour le saut", f.nom);
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
        // L'appel redirigé : bl vers la porte, entre la lecture de l'ID et le test du résultat.
        let s = ARM_APPEL.site;
        assert_eq!(mot(s) & 0xfc00_0000, 0x9400_0000, "un bl");
        assert_eq!(patch::cible_branche(s, mot(s)), Some(ARM_PORTE.adresse));
        assert_eq!(&t[s - 4..s], ARM_APPEL.avant, "ldr w0, [x19, #0xe0]");
        assert_eq!(&t[s + 4..s + 8], ARM_APPEL.apres, "cbz w0, 0x9f56e8");
        // Puis AppRequestTurnUnready : ldr x8, [x8, #0x68] ; mov x2, x21 ; blr x8.
        assert_eq!([mot(0x9f56c4), mot(0x9f56c8), mot(0x9f56cc)], [0xf940_3508, 0xaa15_03e2, 0xd63f_0100]);
        // La porte a trois appelants, dont celui-ci.
        for site in [0x9f56a8, 0x9f5b7c, 0x9f5dd4] {
            assert_eq!(patch::cible_branche(site, mot(site)), Some(ARM_PORTE.adresse), "appel en {site:#x}");
        }
        // Vtable de PlayerTurnComplete, entrée 4 (pointeur chaîné : 36 bits bas).
        let ptr = u64::from_le_bytes(t[0x12e5b48..0x12e5b50].try_into().unwrap());
        assert_eq!(ptr & 0xf_ffff_ffff, ARM_FIN_DE_TOUR.adresse as u64);
        // Le joueur du message : ldr w0, [x0, #0x18].
        assert_eq!(mot(0x84b420), 0xb940_1800);
        // Les deux appelants de la réception.
        assert_eq!(patch::cible_branche(0x848c18, mot(0x848c18)), Some(ARM_RECEPTION.adresse));
        assert_eq!(patch::cible_branche(0x522c68, mot(0x522c68)), Some(ARM_RECEPTION.adresse));
    }

    #[test]
    fn fonctions_du_jeu_x86_64() {
        let Some(b) = gamecore() else { return };
        let t = tranche_x86_64(&b).expect("tranche x86_64");
        for f in [&X86_RECEPTION, &X86_FIN_DE_TOUR, &X86_AUTOMATISATION, &X86_PORTE] {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
        }
        // Prologues recopiés : push rbp ; mov rbp, rsp ; push r15 … rbx ; sub rsp, imm32.
        for (f, pile) in [(&X86_RECEPTION, 0x658u32), (&X86_FIN_DE_TOUR, 0x258)] {
            let mut attendu = vec![0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec];
            attendu.extend_from_slice(&pile.to_le_bytes());
            assert_eq!(attendu.len(), f.prologue);
            assert_eq!(&t[f.adresse..f.adresse + f.prologue], &attendu[..], "{}", f.nom);
        }
        let appel = |a: usize| (t[a] == 0xe8 || t[a] == 0xe9).then(|| patch::cible_rel32_x86(a, &t[a..a + 5])).flatten();
        // L'appel redirigé.
        let s = X86_APPEL.site;
        assert_eq!(t[s], 0xe8, "un call");
        assert_eq!(appel(s), Some(X86_PORTE.adresse));
        assert_eq!(&t[s - 6..s], X86_APPEL.avant, "mov edi, [rbx + 0xe0]");
        assert_eq!(&t[s + 5..s + 9], X86_APPEL.apres, "test al, al ; je");
        // Puis AppRequestTurnUnready : mov esi, [rbx + 0xe0] … mov edx, r15d ; call [rax + 0x68].
        assert_eq!(&t[0x9eeb2a..0x9eeb30], &[0x44, 0x89, 0xfa, 0xff, 0x50, 0x68]);
        for site in [0x9eeb0e, 0x9eecfc, 0x9eef82] {
            assert_eq!(appel(site), Some(X86_PORTE.adresse), "appel en {site:#x}");
        }
        // Entrée 4 de la vtable de PlayerTurnComplete ; le joueur du message : mov edi, [rdi + 0x18].
        assert_eq!(u64::from_le_bytes(t[0x1333510..0x1333518].try_into().unwrap()) & 0xf_ffff_ffff, X86_FIN_DE_TOUR.adresse as u64);
        assert_eq!(&t[0x829965..0x829968], &[0x8b, 0x7f, 0x18]);
        // Les deux appelants de la réception : la file (call), l'API de l'exe (jmp).
        assert_eq!(appel(0x827067), Some(X86_RECEPTION.adresse));
        assert_eq!(appel(0x51767a), Some(X86_RECEPTION.adresse));
    }

    #[test]
    fn porte_reecrite_acceptee() {
        let mut octets = patch::jump(0x1234_5678);
        octets.resize(PORTE.taille, 0);
        assert!(porte_attendue(&octets), "la porte réécrite par congres_mac.rs");
        assert!(!porte_attendue(&vec![0; PORTE.taille]));
    }
}
