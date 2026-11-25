//! Échange d'unités (`UNITOPERATION_SWAP_UNITS`) qui laisse une unité en chemin, sous macOS : le
//! pendant d'echange.rs (voir son en-tête pour le défaut et le correctif). Correctif POL.
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image :
//! - vtable de `N8GameCore4Unit10Operations9SwapUnitsE` en `0x1305740` (deux destructeurs en tête,
//!   ABI Itanium : chaque entrée Windows d'indice `i` est ici en `i + 1`) : exécution `0xbe5e68`
//!   (entrée 9, `+0x40` sous Windows), destination `0xbe65c8` (entrée 18, appelée en `+0x90`,
//!   `mov x0, x1`), partenaire `0xbe62a8` (entrée 25, appelée en `+0xc8`) ;
//! - l'exécution lit la case visée en `destination+0xc/+0x10`, les drapeaux en
//!   `[requête+8]+8`, puis fait **un** pas `0x55be68(formation, &case, drapeaux)` pour chaque unité,
//!   la première avec `IGNORE_STACKING` (4), comme Windows ;
//! - le filet de `MoveTo` (`0xbd1e54`, entrée 5 de la vtable `0x1303790`) donne les fonctions du
//!   contrôle d'empilement : carte `0x3bbe1c()` (`[0x1390218]+0x250`, appelée par le relais
//!   `0x82c80c`), grille par sa méthode `+0xc0`, couche `0xb91184(unité)`, unités de la case
//!   `0x83796c(grille, x, y, couche)`, compte `0xbeac58(unité, liste, 0)` comparé à
//!   `[0x1390220]+0x58c` (limite d'empilement des `GlobalParameters`), et le placement
//!   `0xbe86d0(unité, &case, 1, 0)`.
//!
//! En x86_64 (Mac Intel, Rosetta), même code et mêmes décalages : vtable `0x1353060`, exécution
//! `0xbfbdf0`, destination `0xbfc5d0`, partenaire `0xbfc280`, pas `0x5547f0`, carte `0x3ae6f0`
//! (`[0x13dc0b0]+0x250`, relais `0x808440`), couche `0xba2370`, unités de la case `0x813fc0`, compte
//! `0xc010e0`, `GlobalParameters` en `[0x13dc0b8]`, placement `0xbfe7c0` (relevés dans `MoveTo`,
//! `0xbe6ed0`).
//!
//! Unité (Windows plus 8) : formation `+0xbc0`, case `+0x100/+0x104`, id `+0xb8`, propriétaire
//! `+0x130` ; formation : mouvements restants `+0xf8`.
//!
//! L'exécution est détournée par un trampoline sur son début (instructions de pile, sans adresse
//! relative), plutôt que par l'entrée de vtable : même effet, une seule entrée l'appelle.

use super::journal::log;
use super::{patch, pol};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    empreinte: u64,
}

/// Ce que le correctif lit ou appelle dans une architecture.
struct Adresses {
    /// Détournée : son prologue (`PROLOGUE` octets) est recopié dans le trampoline.
    executer: Fonction,
    /// Appelées directement.
    appelees: [Fonction; 6],
    /// Appelées par la vtable : vérifiées, et comparées aux entrées de la vtable.
    destination: Fonction,
    partenaire: Fonction,
    vtable: usize,
    /// Pointeur vers les `GlobalParameters`.
    parametres: usize,
    prologue: usize,
}

#[cfg_attr(not(target_arch = "aarch64"), allow(dead_code))]
const ARM64: Adresses = Adresses {
    executer: Fonction { nom: "exécution de SWAP_UNITS", adresse: 0xbe5e68, taille: 0x20c, empreinte: 0xa779_b868_db60_fa7e },
    appelees: [
        Fonction { nom: "pas du chemin", adresse: 0x55be68, taille: 0xc7c, empreinte: 0x818f_3ea3_d4bd_d1e8 },
        Fonction { nom: "placement", adresse: 0xbe86d0, taille: 0x1238, empreinte: 0x26bb_027e_97e6_3c34 },
        Fonction { nom: "compte d'empilement", adresse: 0xbeac58, taille: 0x100, empreinte: 0x64a8_ad78_8674_15ca },
        Fonction { nom: "unités de la case", adresse: 0x83796c, taille: 0x8c, empreinte: 0x7615_b215_b8ea_c394 },
        Fonction { nom: "couche", adresse: 0xb91184, taille: 0xa0, empreinte: 0x8aeb_4bb3_11ca_703c },
        Fonction { nom: "carte", adresse: 0x3bbe1c, taille: 0x1c, empreinte: 0xee0d_f415_5c57_fe05 },
    ],
    destination: Fonction { nom: "destination", adresse: 0xbe65c8, taille: 0x8, empreinte: 0x5b03_64bf_8a1c_463b },
    partenaire: Fonction { nom: "partenaire", adresse: 0xbe62a8, taille: 0x310, empreinte: 0x00d1_877e_772b_6a83 },
    vtable: 0x1305740,
    parametres: 0x1390220,
    // sub sp ; stp ; stp ; stp
    prologue: 16,
};

#[cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]
const X86_64: Adresses = Adresses {
    executer: Fonction { nom: "exécution de SWAP_UNITS", adresse: 0xbfbdf0, taille: 0x250, empreinte: 0x9173_b009_7c19_ef10 },
    appelees: [
        Fonction { nom: "pas du chemin", adresse: 0x5547f0, taille: 0xec0, empreinte: 0xf791_9117_169a_43f3 },
        Fonction { nom: "placement", adresse: 0xbfe7c0, taille: 0x14b0, empreinte: 0x802f_33d7_fbc9_6be5 },
        Fonction { nom: "compte d'empilement", adresse: 0xc010e0, taille: 0x120, empreinte: 0xecad_d905_24af_b1c0 },
        Fonction { nom: "unités de la case", adresse: 0x813fc0, taille: 0x90, empreinte: 0x0345_241f_54f2_4405 },
        Fonction { nom: "couche", adresse: 0xba2370, taille: 0x90, empreinte: 0x1c62_c456_d9e4_d99d },
        Fonction { nom: "carte", adresse: 0x3ae6f0, taille: 0x20, empreinte: 0xa986_3289_9213_8b6f },
    ],
    destination: Fonction { nom: "destination", adresse: 0xbfc5d0, taille: 0x10, empreinte: 0x36f3_c9dd_79c6_a237 },
    partenaire: Fonction { nom: "partenaire", adresse: 0xbfc280, taille: 0x320, empreinte: 0xebb9_7df3_5d0f_a50b },
    vtable: 0x1353060,
    parametres: 0x13dc0b8,
    // push rbp ; mov rbp, rsp ; push r15 ; push r14 ; push r13 ; push r12 ; push rbx ; sub rsp, 0x98
    prologue: 20,
};

#[cfg(target_arch = "aarch64")]
const A: &Adresses = &ARM64;
#[cfg(target_arch = "x86_64")]
const A: &Adresses = &X86_64;

const UN_PAS: usize = 0;
const PLACER: usize = 1;
const COMPTER: usize = 2;
const UNITES_DE_LA_CASE: usize = 3;
const COUCHE: usize = 4;
const CARTE: usize = 5;

/// Entrées de la vtable de l'opération (indices de 8 octets).
const E_EXECUTER: usize = 9;
const E_DESTINATION: usize = 18;
const E_PARTENAIRE: usize = 25;
const VT_DESTINATION: usize = E_DESTINATION * 8; // (opération, données) -> objet, case en +0xc/+0x10
const VT_PARTENAIRE: usize = E_PARTENAIRE * 8; // (opération, unité, &case) -> partenaire
const VT_GRILLE: usize = 0xc0; // carte -> grille des unités
const P_LIMITE_EMPILEMENT: usize = 0x58c;

const U_FORMATION: usize = 0xbc0;
const U_X: usize = 0x100;
const U_Y: usize = 0x104;
const U_PROPRIETAIRE: usize = 0x130;
const U_ID: usize = 0xb8;
const F_MOUVEMENTS: usize = 0xf8;
const R_DONNEES: usize = 8; // requête : données de l'opération ; `+8` drapeaux de déplacement
const D_DRAPEAUX: usize = 8;
const O_CASE: usize = 0xc;
const IGNORE_STACKING: u32 = 4;
const PAS_MAX: usize = 8;

type Executer = unsafe extern "C" fn(usize, usize, usize, usize);

static BASE: AtomicUsize = AtomicUsize::new(0);
static T_EXECUTER: AtomicUsize = AtomicUsize::new(0);
static CORRECTIONS: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 30;

fn gc(addr: usize) -> usize {
    BASE.load(Ordering::Relaxed) + addr
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

unsafe fn vcall(objet: usize, decalage: usize) -> usize {
    rd::<usize>(rd::<usize>(objet) + decalage)
}

unsafe fn position(unite: usize) -> (i32, i32) {
    (rd(unite + U_X), rd(unite + U_Y))
}

/// L'unité compte-t-elle plus d'unités sur sa case que la limite d'empilement ? (le test du filet de
/// `MoveTo`)
unsafe fn empilee(unite: usize) -> bool {
    let carte: unsafe extern "C" fn() -> usize = std::mem::transmute(gc(A.appelees[CARTE].adresse));
    let parametres: usize = rd(gc(A.parametres));
    let carte = carte();
    if carte == 0 || parametres == 0 {
        return false;
    }
    let grille: unsafe extern "C" fn(usize) -> usize = std::mem::transmute(vcall(carte, VT_GRILLE));
    let couche: unsafe extern "C" fn(usize) -> i32 = std::mem::transmute(gc(A.appelees[COUCHE].adresse));
    let unites: unsafe extern "C" fn(usize, i32, i32, i32) -> usize = std::mem::transmute(gc(A.appelees[UNITES_DE_LA_CASE].adresse));
    let compter: unsafe extern "C" fn(usize, usize, u32) -> u32 = std::mem::transmute(gc(A.appelees[COMPTER].adresse));
    let (x, y) = position(unite);
    let liste = unites(grille(carte), x, y, couche(unite));
    liste != 0 && compter(unite, liste, 0) > rd::<u32>(parametres + P_LIMITE_EMPILEMENT)
}

/// Fait finir à l'unité le chemin vers `arrivee`. Rend vrai si elle a bougé.
unsafe fn poursuivre(unite: usize, arrivee: [i32; 2], drapeaux: u32) -> bool {
    let formation: usize = rd(unite + U_FORMATION);
    if formation == 0 {
        return false;
    }
    let un_pas: unsafe extern "C" fn(usize, *const [i32; 2], u32) -> i32 = std::mem::transmute(gc(A.appelees[UN_PAS].adresse));
    let mut a_bouge = false;
    for _ in 0..PAS_MAX {
        let avant = position(unite);
        if avant == (arrivee[0], arrivee[1]) {
            break;
        }
        un_pas(formation, &arrivee, drapeaux);
        if position(unite) == avant {
            break;
        }
        a_bouge = true;
    }
    a_bouge
}

unsafe fn journal(unite: usize, depart: (i32, i32), arrivee: [i32; 2], quoi: &str) {
    let k = CORRECTIONS.fetch_add(1, Ordering::Relaxed) + 1;
    if k > MAX_LOGS {
        return;
    }
    let formation: usize = rd(unite + U_FORMATION);
    let mouvements = if formation == 0 { -1 } else { rd::<i32>(formation + F_MOUVEMENTS) };
    let (x, y) = position(unite);
    log(&format!(
        "echange : joueur {} unité {} de ({},{}) vers ({},{}) : {quoi}, finit en ({x},{y}), mouvements {:.2} ({k})",
        rd::<i32>(unite + U_PROPRIETAIRE),
        rd::<i32>(unite + U_ID),
        depart.0,
        depart.1,
        arrivee[0],
        arrivee[1],
        mouvements as f64 / 256.0
    ));
}

/// L'exécution de l'échange, comme echange.rs : l'original, puis chaque unité finit son chemin.
unsafe extern "C" fn executer(operation: usize, unite: usize, requete: usize, resultat: usize) {
    let originale: Executer = std::mem::transmute(T_EXECUTER.load(Ordering::Acquire));
    let donnees: usize = if requete == 0 { 0 } else { rd(requete + R_DONNEES) };
    if operation == 0 || unite == 0 || donnees == 0 || !pol::actifs() {
        return originale(operation, unite, requete, resultat);
    }
    // Comme l'original : la case visée, puis la partenaire qui s'y trouve.
    let destination: unsafe extern "C" fn(usize, usize) -> usize = std::mem::transmute(vcall(operation, VT_DESTINATION));
    let objet = destination(operation, donnees);
    let mut case_b = if objet == 0 { [-9999, -9999] } else { [rd::<i32>(objet + O_CASE), rd::<i32>(objet + O_CASE + 4)] };
    let trouver: unsafe extern "C" fn(usize, usize, *mut [i32; 2]) -> usize = std::mem::transmute(vcall(operation, VT_PARTENAIRE));
    let b = trouver(operation, unite, &mut case_b);
    let drapeaux: u32 = rd(donnees + D_DRAPEAUX);
    let a_depart = position(unite);
    let b_depart = if b == 0 { (0, 0) } else { position(b) };

    originale(operation, unite, requete, resultat);

    if b == 0 {
        return;
    }
    let parcours = [
        (unite, a_depart, [b_depart.0, b_depart.1], drapeaux | IGNORE_STACKING),
        (b, b_depart, [a_depart.0, a_depart.1], drapeaux),
    ];
    // Deux tours : la case d'arrivée de l'une n'est libre qu'une fois l'autre partie.
    let mut pas_faits = [false; 2];
    for _ in 0..2 {
        for (i, &(u, _, arrivee, d)) in parcours.iter().enumerate() {
            if poursuivre(u, arrivee, d) {
                pas_faits[i] = true;
            }
        }
    }
    let placer: unsafe extern "C" fn(usize, *const [i32; 2], u32, u32) -> u64 = std::mem::transmute(gc(A.appelees[PLACER].adresse));
    for (i, &(u, depart, arrivee, _)) in parcours.iter().enumerate() {
        let arrivee_atteinte = position(u) == (arrivee[0], arrivee[1]);
        if !arrivee_atteinte && empilee(u) {
            placer(u, &arrivee, 1, 0);
            journal(u, depart, arrivee, "restée empilée en chemin, placée");
        } else if pas_faits[i] {
            journal(u, depart, arrivee, if arrivee_atteinte { "arrêtée en chemin, chemin repris" } else { "chemin repris sans arriver" });
        }
    }
}

/// Toutes les empreintes et les trois entrées de la vtable, dans l'image chargée en `base`.
unsafe fn verifier(base: usize) -> Result<(), String> {
    let toutes = std::iter::once(&A.executer).chain(A.appelees.iter()).chain([&A.destination, &A.partenaire]);
    for f in toutes {
        if fnv(std::slice::from_raw_parts((base + f.adresse) as *const u8, f.taille)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), échange d'unités non corrigé", f.nom, f.adresse));
        }
    }
    for (entree, f) in [(E_EXECUTER, &A.executer), (E_DESTINATION, &A.destination), (E_PARTENAIRE, &A.partenaire)] {
        let at = base + A.vtable + entree * 8;
        if rd::<usize>(at) != base + f.adresse {
            return Err(format!("vtable de SwapUnits inattendue (entrée {entree}, {}), échange d'unités non corrigé", f.nom));
        }
    }
    Ok(())
}

pub unsafe fn installer(base: usize) -> Result<String, String> {
    verifier(base)?;
    BASE.store(base, Ordering::Relaxed);
    let methode = patch::hook_with_trampoline_publie(base + A.executer.adresse, A.prologue, executer as *const () as usize, &T_EXECUTER)?;
    Ok(format!("exécution de SWAP_UNITS détournée en {:#x} : chaque unité finit son chemin ({methode})", A.executer.adresse))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Empreintes et entrées de vtable d'une tranche (les segments y sont à leur adresse).
    fn verifier_tranche(a: &Adresses, t: &[u8]) {
        let toutes = std::iter::once(&a.executer).chain(a.appelees.iter()).chain([&a.destination, &a.partenaire]);
        for f in toutes {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
        }
        let q = |at: usize| u64::from_le_bytes(t[at..at + 8].try_into().unwrap()) as usize;
        for (entree, f) in [(E_EXECUTER, &a.executer), (E_DESTINATION, &a.destination), (E_PARTENAIRE, &a.partenaire)] {
            assert_eq!(q(a.vtable + entree * 8), f.adresse, "entrée {entree} ({})", f.nom);
        }
    }

    /// Page visée par un `adrp` en `at`.
    fn adrp(at: usize, m: u32) -> usize {
        assert_eq!(m & 0x9f00_0000, 0x9000_0000, "adrp attendu en {at:#x}");
        let imm = (((m >> 5) & 0x7ffff) << 2 | (m >> 29) & 3) as i64;
        let imm = if imm & (1 << 20) != 0 { imm - (1 << 21) } else { imm };
        ((at as i64 & !0xfff) + (imm << 12)) as usize
    }

    #[test]
    fn fonctions_du_jeu_arm64() {
        let Some(b) = jeu() else { return };
        let t = super::super::macho::tranche_arm64(&b).expect("tranche arm64");
        verifier_tranche(&ARM64, t);
        let mot = |a: usize| u32::from_le_bytes(t[a..a + 4].try_into().unwrap());
        // Prologue sans adresse relative (pas d'adrp, adr, b, bl, ldr littéral).
        for k in 0..ARM64.prologue / 4 {
            let m = mot(ARM64.executer.adresse + 4 * k);
            let relatif = m & 0x9f00_0000 == 0x9000_0000 || m & 0x9f00_0000 == 0x1000_0000 || m & 0x7c00_0000 == 0x1400_0000 || m & 0x3b00_0000 == 0x1800_0000;
            assert!(!relatif, "instruction relative en +{}", 4 * k);
        }
        // L'exécution : destination (+0x90), partenaire (+0xc8), un pas par unité, la première
        // avec IGNORE_STACKING (orr w2, w8, #4).
        assert_eq!(mot(0xbe5eb0), 0xf940_4908, "ldr x8, [x8, #0x90]");
        assert_eq!(mot(0xbe5ebc), 0x2941_a408, "ldp w8, w9, [x0, #0xc]");
        assert_eq!(mot(0xbe5ed4), 0xf940_6508, "ldr x8, [x8, #0xc8]");
        assert_eq!(mot(0xbe5f84), 0x321e_0102, "orr w2, w8, #0x4");
        for site in [0xbe5f8c, 0xbe5ffc] {
            assert_eq!(patch::cible_branche(site, mot(site)), Some(ARM64.appelees[UN_PAS].adresse), "pas en {site:#x}");
        }
        // Le filet de MoveTo : carte (relais 0x82c80c -> 0x3bbe1c), couche, unités, compte, limite
        // en +0x58c des paramètres [0x1390220], placement en mode 1.
        assert_eq!(patch::cible_branche(0xbd2210, mot(0xbd2210)), Some(0x82c80c));
        assert_eq!(patch::cible_branche(0x82c80c, mot(0x82c80c)), Some(ARM64.appelees[CARTE].adresse));
        assert_eq!(mot(0xbd2218), 0xf940_6108, "ldr x8, [x8, #0xc0] (grille)");
        for (site, i) in [(0xbd2228, COUCHE), (0xbd223c, UNITES_DE_LA_CASE), (0xbd224c, COMPTER), (0xbd2340, PLACER)] {
            assert_eq!(patch::cible_branche(site, mot(site)), Some(ARM64.appelees[i].adresse), "appel en {site:#x}");
        }
        assert_eq!(adrp(0xbd2254, mot(0xbd2254)) + 0x220, ARM64.parametres, "adrp x8 (page de 0x1390220)");
        assert_eq!(mot(0xbd2258), 0x9108_8108, "add x8, x8, #0x220");
        assert_eq!(mot(0xbd2260), 0xb945_8d16, "ldr w22, [x8, #0x58c]");
    }

    #[test]
    fn fonctions_du_jeu_x86_64() {
        let Some(b) = jeu() else { return };
        let t = tranche_x86_64(&b).expect("tranche x86_64");
        verifier_tranche(&X86_64, t);
        let ex = X86_64.executer.adresse;
        // push rbp ; mov rbp, rsp ; push r15..rbx ; sub rsp, 0x98 : 20 octets, rien de relatif.
        assert_eq!(
            t[ex..ex + X86_64.prologue],
            [0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec, 0x98, 0, 0, 0]
        );
        assert!(patch::jump(0).len() <= X86_64.prologue);
        let appel = |a: usize| patch::cible_rel32_x86(a, &t[a..a + 5]);
        let rip = |a: usize| (a as i64 + 4 + i32::from_le_bytes(t[a..a + 4].try_into().unwrap()) as i64) as usize;
        // L'exécution : destination (+0x90), partenaire (+0xc8), un pas par unité.
        assert_eq!(t[0xbfbe2d..0xbfbe33], [0xff, 0x90, 0x90, 0, 0, 0], "call [rax+0x90]");
        assert_eq!(t[0xbfbe33..0xbfbe39], [0x8b, 0x48, 0x0c, 0x8b, 0x40, 0x10], "case en +0xc/+0x10");
        assert_eq!(t[0xbfbe61..0xbfbe67], [0xff, 0x90, 0xc8, 0, 0, 0], "call [rax+0xc8]");
        assert_eq!(t[0xbfbf37..0xbfbf3a], [0x83, 0xca, 0x04], "or edx, 4");
        for site in [0xbfbf41, 0xbfbfd4] {
            assert_eq!(appel(site), Some(X86_64.appelees[UN_PAS].adresse), "pas en {site:#x}");
        }
        // Le filet de MoveTo (0xbe6ed0).
        assert_eq!(appel(0xbe7309), Some(0x808440));
        assert_eq!(t[0x808445], 0xe9);
        assert_eq!(appel(0x808445), Some(X86_64.appelees[CARTE].adresse));
        assert_eq!(t[0xbe7314..0xbe731a], [0xff, 0x91, 0xc0, 0, 0, 0], "call [rcx+0xc0] (grille)");
        for (site, i) in [(0xbe7320, COUCHE), (0xbe7338, UNITES_DE_LA_CASE), (0xbe7345, COMPTER), (0xbe7429, PLACER)] {
            assert_eq!(appel(site), Some(X86_64.appelees[i].adresse), "appel en {site:#x}");
        }
        assert_eq!(rip(0xbe7350), X86_64.parametres, "lea rax, [rip + paramètres]");
        assert_eq!(t[0xbe7357..0xbe735e], [0x44, 0x8b, 0xa8, 0x8c, 0x05, 0, 0], "mov r13d, [rax+0x58c]");
    }
}
