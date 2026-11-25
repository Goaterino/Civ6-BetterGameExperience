//! Vision des cités-États reprises après une guerre, sous macOS : le pendant de vision_guerre.rs
//! (voir son en-tête et vision_cites_etats/README.md pour le défaut et le correctif). Correctif
//! POL, sans réglage.
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image :
//! - l'état guerre/paix entre deux joueurs (`1801d31b0` sous Windows) est `0x59273c` : il teste
//!   `Player+0x260` (vivant) des deux joueurs, puis appelle trois fois l'inscription `0xa87b74` à la
//!   guerre et trois fois le retrait `0xa87d60` à la paix, `(p3,p2)`, `(p3,p2)`, `(p2,p3)`, chacun
//!   après le gestionnaire de visibilité `0xa890ac()` (`[0x13945e0]`). Ce sont leurs seuls
//!   appelants ;
//! - gestionnaire : `+8` indice de l'instance de chaque joueur, `+0x30` les instances (comme
//!   Windows) ; instance : liste suspendue en `+0x348`, entrées `{i32 joueur, i32 compteur}` en
//!   `+0x358`, nombre en `+0x368` (Windows plus 8, lus par `0xa7e804`, que le retrait appelle) ;
//! - `BeginTurn` `0x642338`, sans argument, seul appelant `0x642f40` (`Game::Processor::Update`) ;
//! - en guerre `0x5910b4(diplomatie, joueur, joueur)` (table `+0x1a0` différente de -1, nombre de
//!   joueurs `+0xc0`), diplomatie du jeu `0x58c16c()` (classe `DiplomaticRelations`) ;
//! - joueurs : `0x9eba84()` (`[0x1394118]`), `Player*` de `+0x20` à `+0x28` (vecteur).
//!
//! En x86_64 (Mac Intel, Rosetta), mêmes décalages : guerre/paix `0x58e3c0`, inscription
//! `0xa8c920`, retrait `0xa8cb20` (liste lue par `0xa82820`), gestionnaire `0xa8dd80()`
//! (`[0x13e05a8]`), `BeginTurn` `0x6420f0` (appelant `0x642d00`), en guerre `0x58cd00`,
//! diplomatie `0x5876d0()`, joueurs `0x9e4ec0()` (`[0x13e00e0]`).
//!
//! Deux détours par trampoline, comme Windows : le retrait vide l'entrée en entier, le début de tour
//! retire les entrées entre joueurs vivants en paix. Seules les fonctions du jeu écrivent.

use super::journal::log;
use super::{patch, pol};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    empreinte: u64,
}

struct Adresses {
    /// Détournées (prologue recopié dans le trampoline).
    retrait: Fonction,
    debut_tour: Fonction,
    /// Appelées.
    en_guerre: Fonction,
    diplomatie: Fonction,
    gestionnaire: Fonction,
    joueurs: Fonction,
    /// Vérifiée seulement : seul appelant des deux fonctions de la liste suspendue.
    guerre_paix: Fonction,
    prologue_retrait: usize,
    prologue_debut_tour: usize,
}

#[cfg_attr(not(target_arch = "aarch64"), allow(dead_code))]
const ARM64: Adresses = Adresses {
    retrait: Fonction { nom: "retrait de la liste suspendue", adresse: 0xa87d60, taille: 0x410, empreinte: 0xb47d_7308_b42d_d254 },
    debut_tour: Fonction { nom: "BeginTurn", adresse: 0x642338, taille: 0x3b4, empreinte: 0xf307_af9d_4378_393c },
    en_guerre: Fonction { nom: "en guerre", adresse: 0x5910b4, taille: 0x4c, empreinte: 0x75f5_a7df_cc69_1af8 },
    diplomatie: Fonction { nom: "diplomatie du jeu", adresse: 0x58c16c, taille: 0x54, empreinte: 0x90cb_b44e_e446_2dc8 },
    gestionnaire: Fonction { nom: "gestionnaire de visibilité", adresse: 0xa890ac, taille: 0xc, empreinte: 0x5a31_6f24_05d2_c5bf },
    joueurs: Fonction { nom: "PlayerManager", adresse: 0x9eba84, taille: 0xc, empreinte: 0xa17b_1a8a_f825_0832 },
    guerre_paix: Fonction { nom: "état guerre/paix", adresse: 0x59273c, taille: 0x50c, empreinte: 0xd2f4_d617_2880_9edb },
    // sub sp ; stp ; stp ; stp
    prologue_retrait: 16,
    // stp ; stp ; stp ; stp
    prologue_debut_tour: 16,
};

#[cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]
const X86_64: Adresses = Adresses {
    retrait: Fonction { nom: "retrait de la liste suspendue", adresse: 0xa8cb20, taille: 0x480, empreinte: 0x4575_d859_ca49_4ceb },
    debut_tour: Fonction { nom: "BeginTurn", adresse: 0x6420f0, taille: 0x400, empreinte: 0x9138_0c22_2591_a693 },
    en_guerre: Fonction { nom: "en guerre", adresse: 0x58cd00, taille: 0x50, empreinte: 0x678a_68f9_5ec8_74b0 },
    diplomatie: Fonction { nom: "diplomatie du jeu", adresse: 0x5876d0, taille: 0x40, empreinte: 0x23b7_f0e9_c786_16bb },
    gestionnaire: Fonction { nom: "gestionnaire de visibilité", adresse: 0xa8dd80, taille: 0x10, empreinte: 0x3eb9_77ae_90d8_1bb4 },
    joueurs: Fonction { nom: "PlayerManager", adresse: 0x9e4ec0, taille: 0x10, empreinte: 0xfb90_c246_417c_5244 },
    guerre_paix: Fonction { nom: "état guerre/paix", adresse: 0x58e3c0, taille: 0x550, empreinte: 0x09c5_3d6e_85b4_bbd4 },
    // push rbp ; mov rbp, rsp ; push r15 ; push r14 ; push r13 ; push r12 ; push rbx ; sub rsp, 0x58
    prologue_retrait: 17,
    // idem, sub rsp, 0x258
    prologue_debut_tour: 20,
};

#[cfg(target_arch = "aarch64")]
const A: &Adresses = &ARM64;
#[cfg(target_arch = "x86_64")]
const A: &Adresses = &X86_64;

/// Joueur : vivant (le test de l'état guerre/paix avant de toucher aux listes).
const J_VIVANT: usize = 0x260;
/// PlayerManager : vecteur des `Player*` par id.
const PM_DEBUT: usize = 0x20;
const PM_FIN: usize = 0x28;
/// Gestionnaire de visibilité : `+8` l'indice de l'instance de chaque joueur (-1 sans instance),
/// `+0x30` le tableau des instances.
const G_INDICES: usize = 0x8;
const G_INSTANCES: usize = 0x30;
/// Instance : liste suspendue, entrées puis nombre.
const I_SUSPENDUS: usize = 0x358;
const I_SUSPENDUS_NOMBRE: usize = 0x368;
/// Ids de joueurs des listes (`MAX_PLAYERS`).
const MAX_JOUEURS: u32 = 64;

static BASE: AtomicUsize = AtomicUsize::new(0);
/// Les deux détours sont posés : sans l'un, l'autre laisse faire le jeu.
static POSES: AtomicBool = AtomicBool::new(false);
static T_RETRAIT: AtomicUsize = AtomicUsize::new(0);
static T_DEBUT_TOUR: AtomicUsize = AtomicUsize::new(0);

fn gc(addr: usize) -> usize {
    BASE.load(Ordering::Relaxed) + addr
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

type RetraitFn = unsafe extern "C" fn(usize, u32, u32) -> u64;
type SansArgFn = unsafe extern "C" fn() -> usize;
type EnGuerreFn = unsafe extern "C" fn(usize, i32, i32) -> u32;

fn actif() -> bool {
    POSES.load(Ordering::Relaxed) && pol::actifs()
}

/// L'instance de visibilité du joueur, 0 sans elle.
unsafe fn instance(gestionnaire: usize, joueur: u32) -> usize {
    if gestionnaire == 0 || joueur >= MAX_JOUEURS {
        return 0;
    }
    let indices: usize = rd(gestionnaire + G_INDICES);
    let instances: usize = rd(gestionnaire + G_INSTANCES);
    if indices == 0 || instances == 0 {
        return 0;
    }
    let i: i32 = rd(indices + joueur as usize * 4);
    if i < 0 {
        return 0;
    }
    rd(instances + i as usize * 8)
}

/// Les entrées `(joueur, compteur)` de la liste suspendue de l'instance.
unsafe fn suspendus(inst: usize) -> Vec<(u32, u32)> {
    let (entrees, n): (usize, u32) = (rd(inst + I_SUSPENDUS), rd(inst + I_SUSPENDUS_NOMBRE));
    if entrees == 0 || n > MAX_JOUEURS {
        return Vec::new();
    }
    (0..n as usize).map(|k| (rd::<u32>(entrees + k * 8), rd::<u32>(entrees + k * 8 + 4))).collect()
}

/// Compteur de `retire` dans la liste suspendue de `joueur`, 0 s'il n'y est pas.
unsafe fn compteur(gestionnaire: usize, joueur: u32, retire: u32) -> u32 {
    let inst = instance(gestionnaire, joueur);
    if inst == 0 {
        return 0;
    }
    suspendus(inst).into_iter().find(|e| e.0 == retire).map_or(0, |e| e.1)
}

/// Le joueur existe et vit.
unsafe fn vivant(joueur: u32) -> bool {
    let joueurs: SansArgFn = std::mem::transmute(gc(A.joueurs.adresse));
    let mgr = joueurs();
    if mgr == 0 || joueur >= MAX_JOUEURS {
        return false;
    }
    let (debut, fin): (usize, usize) = (rd(mgr + PM_DEBUT), rd(mgr + PM_FIN));
    if debut == 0 || debut + joueur as usize * 8 >= fin {
        return false;
    }
    let p: usize = rd(debut + joueur as usize * 8);
    p != 0 && rd::<u8>(p + J_VIVANT) != 0
}

/// Appels du retrait d'origine pour vider l'entrée : autant que son compteur, un au moins (comme
/// vision_guerre.rs).
fn appels_pour_vider(compteur: u32) -> u32 {
    compteur.clamp(1, MAX_JOUEURS)
}

unsafe extern "C" fn retrait_hook(gestionnaire: usize, joueur: u32, retire: u32) -> u64 {
    let orig: RetraitFn = std::mem::transmute(T_RETRAIT.load(Ordering::Acquire));
    if !actif() {
        return orig(gestionnaire, joueur, retire);
    }
    let n = compteur(gestionnaire, joueur, retire);
    let mut r = orig(gestionnaire, joueur, retire);
    for _ in 1..appels_pour_vider(n) {
        r = orig(gestionnaire, joueur, retire);
    }
    if n > 1 {
        log(&format!("war suspension: peace, entry of player {retire} in player {joueur}'s list had count {n}, removed whole"));
    }
    r
}

unsafe extern "C" fn debut_tour_hook() {
    let orig: unsafe extern "C" fn() = std::mem::transmute(T_DEBUT_TOUR.load(Ordering::Acquire));
    if actif() {
        reparer();
    }
    orig();
}

/// Retire toute entrée suspendue entre deux joueurs vivants qui ne sont pas en guerre.
unsafe fn reparer() {
    let retrait: RetraitFn = std::mem::transmute(T_RETRAIT.load(Ordering::Acquire));
    let diplomatie: SansArgFn = std::mem::transmute(gc(A.diplomatie.adresse));
    let gestionnaire: SansArgFn = std::mem::transmute(gc(A.gestionnaire.adresse));
    let en_guerre: EnGuerreFn = std::mem::transmute(gc(A.en_guerre.adresse));
    let (gestionnaire, diplo) = (gestionnaire(), diplomatie());
    if gestionnaire == 0 || diplo == 0 {
        return;
    }
    let mut retirees: Vec<(u32, u32, u32)> = Vec::new();
    for joueur in 0..MAX_JOUEURS {
        let inst = instance(gestionnaire, joueur);
        if inst == 0 || !vivant(joueur) {
            continue;
        }
        // Copie : chaque retrait modifie la liste.
        for (autre, n) in suspendus(inst) {
            if autre == joueur || autre >= MAX_JOUEURS || !vivant(autre) {
                continue;
            }
            // Booléen rendu dans `al` en x86_64 : seul l'octet bas compte.
            if en_guerre(diplo, joueur as i32, autre as i32) & 0xff != 0 {
                continue;
            }
            for _ in 0..appels_pour_vider(n) {
                retrait(gestionnaire, joueur, autre);
            }
            retirees.push((joueur, autre, n));
        }
    }
    if !retirees.is_empty() {
        let detail: Vec<String> = retirees.iter().map(|(j, a, n)| format!("{a} in {j}'s list (x{n})")).collect();
        log(&format!("war suspension: turn start, {} stale entr(ies) between players at peace removed: {}", retirees.len(), detail.join(", ")));
    }
}

fn toutes(a: &Adresses) -> [&Fonction; 7] {
    [&a.retrait, &a.debut_tour, &a.en_guerre, &a.diplomatie, &a.gestionnaire, &a.joueurs, &a.guerre_paix]
}

pub unsafe fn installer(base: usize) -> Result<String, String> {
    for f in toutes(A) {
        if fnv(std::slice::from_raw_parts((base + f.adresse) as *const u8, f.taille)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), vision après une guerre non corrigée", f.nom, f.adresse));
        }
    }
    BASE.store(base, Ordering::Relaxed);
    // Le retrait d'abord : le début de tour appelle son trampoline.
    patch::hook_with_trampoline_publie(base + A.retrait.adresse, A.prologue_retrait, retrait_hook as *const () as usize, &T_RETRAIT)?;
    let methode = patch::hook_with_trampoline_publie(base + A.debut_tour.adresse, A.prologue_debut_tour, debut_tour_hook as *const () as usize, &T_DEBUT_TOUR)?;
    POSES.store(true, Ordering::Relaxed);
    Ok(format!(
        "2 détours posés (retrait {:#x} : la paix vide l'entrée ; BeginTurn {:#x} : entrées entre joueurs en paix retirées) ({methode})",
        A.retrait.adresse, A.debut_tour.adresse
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appels_du_retrait() {
        assert_eq!(appels_pour_vider(0), 1);
        assert_eq!(appels_pour_vider(1), 1);
        assert_eq!(appels_pour_vider(3), 3);
        assert_eq!(appels_pour_vider(u32::MAX), MAX_JOUEURS, "borne contre une liste corrompue");
    }

    fn jeu() -> Option<Vec<u8>> {
        let home = std::env::var_os("HOME")?;
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        std::fs::read(p).ok()
    }

    /// Tranche x86_64 d'un binaire universel (macho.rs n'a que `tranche_arm64`).
    fn tranche_x86_64(b: &[u8]) -> Option<&[u8]> {
        const CPU_X86_64: u32 = 0x0100_0007;
        let be = |o: usize| b.get(o..o + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap()));
        let le = |o: usize| b.get(o..o + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()));
        if le(0)? == 0xfeed_facf {
            return (le(4)? == CPU_X86_64).then_some(b);
        }
        if be(0)? != 0xcafe_babe {
            return None;
        }
        (0..be(4)? as usize).find_map(|i| {
            let e = 8 + 20 * i;
            (be(e)? == CPU_X86_64).then(|| b.get(be(e + 8)? as usize..(be(e + 8)? + be(e + 12)?) as usize)).flatten()
        })
    }

    fn empreintes(a: &Adresses, t: &[u8]) {
        for f in toutes(a) {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
        }
    }

    #[test]
    fn fonctions_du_jeu_arm64() {
        let Some(b) = jeu() else { return };
        let t = super::super::macho::tranche_arm64(&b).expect("tranche arm64");
        empreintes(&ARM64, t);
        let mot = |a: usize| u32::from_le_bytes(t[a..a + 4].try_into().unwrap());
        for (f, n) in [(&ARM64.retrait, ARM64.prologue_retrait), (&ARM64.debut_tour, ARM64.prologue_debut_tour)] {
            for k in 0..n / 4 {
                let m = mot(f.adresse + 4 * k);
                let relatif = m & 0x9f00_0000 == 0x9000_0000 || m & 0x9f00_0000 == 0x1000_0000 || m & 0x7c00_0000 == 0x1400_0000 || m & 0x3b00_0000 == 0x1800_0000;
                assert!(!relatif, "{} : instruction relative en +{}", f.nom, 4 * k);
            }
        }
        let bl = |a: usize| patch::cible_branche(a, mot(a));
        // État guerre/paix : trois inscriptions, trois retraits, chacun après le gestionnaire.
        for site in [0x592aa4, 0x592ab4, 0x592ac4] {
            assert_eq!(bl(site), Some(0xa87b74), "inscription en {site:#x}");
            assert_eq!(bl(site - 12), Some(ARM64.gestionnaire.adresse));
        }
        for site in [0x592bd0, 0x592be0, 0x592bf0] {
            assert_eq!(bl(site), Some(ARM64.retrait.adresse), "retrait en {site:#x}");
            assert_eq!(bl(site - 12), Some(ARM64.gestionnaire.adresse));
        }
        assert_eq!(mot(0x592a38), 0x3949_8108, "ldrb w8, [x8, #0x260] (vivant)");
        assert_eq!(bl(0x642f40), Some(ARM64.debut_tour.adresse), "Update appelle BeginTurn");
        // Gestionnaire : indices en +8, instances en +0x30 ; liste suspendue lue par 0xa7e804.
        assert_eq!(mot(0xa87d8c), 0xf940_0408, "ldr x8, [x0, #0x8]");
        assert_eq!(mot(0xa87dd0), 0xf940_1a8a, "ldr x10, [x20, #0x30]");
        assert_eq!(bl(0xa87e04), Some(0xa7e804));
        assert_eq!(mot(0xa7e8b4), 0xb943_6a68, "ldr w8, [x19, #0x368] (nombre)");
        assert_eq!(mot(0xa7e8c4), 0xf941_ae69, "ldr x9, [x19, #0x358] (entrées)");
        // En guerre : table +0x1a0, valeur différente de -1.
        assert_eq!(mot(0x5910d0), 0xf940_d009, "ldr x9, [x0, #0x1a0]");
        assert_eq!(mot(0x5910f4), 0x3100_051f, "cmn w8, #1");
    }

    #[test]
    fn fonctions_du_jeu_x86_64() {
        let Some(b) = jeu() else { return };
        let t = tranche_x86_64(&b).expect("tranche x86_64");
        empreintes(&X86_64, t);
        let pile = [0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48];
        let r = X86_64.retrait.adresse;
        assert_eq!(t[r..r + 17], [&pile[..], &[0x83, 0xec, 0x58]].concat()[..], "prologue du retrait");
        let d = X86_64.debut_tour.adresse;
        assert_eq!(t[d..d + 20], [&pile[..], &[0x81, 0xec, 0x58, 0x02, 0, 0]].concat()[..], "prologue de BeginTurn");
        assert!(patch::jump(0).len() <= X86_64.prologue_retrait);
        let appel = |a: usize| patch::cible_rel32_x86(a, &t[a..a + 5]);
        for site in [0x58e767, 0x58e779, 0x58e78b] {
            assert_eq!(appel(site), Some(0xa8c920), "inscription en {site:#x}");
        }
        for site in [0x58e894, 0x58e8a6, 0x58e8b8] {
            assert_eq!(appel(site), Some(X86_64.retrait.adresse), "retrait en {site:#x}");
        }
        assert_eq!(appel(0x58e887), Some(X86_64.gestionnaire.adresse));
        assert_eq!(t[0x58e6f2..0x58e6f9], [0x80, 0xb8, 0x60, 0x02, 0, 0, 0], "cmp byte [rax+0x260], 0 (vivant)");
        assert_eq!(appel(0x642d00), Some(X86_64.debut_tour.adresse), "Update appelle BeginTurn");
        assert_eq!(appel(0xa8cbbc), Some(0xa82820));
        assert_eq!(t[0xa828a4..0xa828ab], [0x41, 0x8b, 0x80, 0x68, 0x03, 0, 0], "mov eax, [r8+0x368] (nombre)");
        assert_eq!(t[0xa828b8..0xa828bf], [0x49, 0x8b, 0x88, 0x58, 0x03, 0, 0], "mov rcx, [r8+0x358] (entrées)");
        assert_eq!(t[0x58cd20..0x58cd27], [0x48, 0x8b, 0xbf, 0xa0, 0x01, 0, 0], "mov rdi, [rdi+0x1a0]");
        assert_eq!(t[0x58cd39..0x58cd3d], [0x83, 0x3c, 0xc7, 0xff], "cmp dword [rdi+rax*8], -1");
    }
}
