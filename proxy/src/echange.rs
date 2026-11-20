//! Échange d'unités (`UNITOPERATION_SWAP_UNITS`) qui laisse une unité en chemin, parfois sur une
//! case déjà occupée. Correctif POL : il change la simulation, il n'est appliqué que si tous les
//! joueurs ont le proxy (voir `pol`). Pas de réglage : bug pur.
//!
//! `Unit_Operations_SwapUnits.cpp` (vtable `180a2cde8`) :
//! - `18067c350` (vtable `+0xc0`) : trouve la partenaire ; les deux cases sont voisines et un
//!   chemin A* existe pour chacune vers la case de l'autre, quel que soit son nombre de pas ;
//! - `18067c860` (vtable `+0x40`, emplacement `180a2ce28`) : exécute l'échange par **un** appel à
//!   `1801a82a0` (un pas du chemin) pour chaque unité, la première avec `IGNORE_STACKING`.
//!
//! Quand l'A* passe par une case voisine (zone de contrôle, rivière, route), la première unité
//! s'arrête sur cette case, occupée ou non (rapport BGE-DAA7E7 : un épéiste posé sur un tigre
//! accroupi). Le détour de l'emplacement de vtable appelle l'original, puis fait continuer chaque
//! unité qui n'est pas arrivée, pas à pas (`1801a82a0`, mêmes drapeaux), tant qu'elle avance. Si
//! elle reste malgré tout empilée, elle est placée sur sa case d'arrivée comme le fait le filet de
//! `MoveTo` (`1803b11b0`, mode 1 : case de repli si elle est prise).
//! Investigation : analyses/unites_superposees/README.md.

use crate::{log, pol};
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// Emplacement de l'exécution dans la vtable de l'opération, et ce qu'il contient.
const EMPLACEMENT: u64 = 0x1_80a2_ce28;
const EXECUTER: u64 = 0x1_8067_c860; // (opération, unité, requête, résultat)
/// `(formation, case {x, y}, drapeaux) -> état` : un pas du chemin vers la case.
const UN_PAS: u64 = 0x1_801a_82a0;
/// `(unité, case {x, y}, mode, 0)` : place l'unité ; mode 1 : contrôle, case de repli au besoin.
const PLACER: u64 = 0x1_803b_11b0;
/// `(unité, unités d'une case, 0) -> nombre` : unités qui comptent pour l'empilement, elle comprise.
const COMPTER: u64 = 0x1_803b_0370;
/// `(carte+0xb8, x, y, couche) -> unités de la case`.
const UNITES_DE_LA_CASE: u64 = 0x1_8005_6a20;
/// `(unité) -> couche de la carte`.
const COUCHE: u64 = 0x1_8006_7990;
/// Pointeur global dont `+0x248` est la carte (lu par `180044d40`).
const JEU: u64 = 0x1_80b8_aa68;
/// `GlobalParameters` ; `+0x58c` : limite d'empilement.
const PARAMETRES: u64 = 0x1_80b8_aa70;

/// Premiers octets de chaque fonction appelée (sans adresse relative), vérifiés avant de poser.
const SIGNATURES: [(u64, &[u8]); 6] = [
    (EXECUTER, &[0x40, 0x55, 0x53, 0x56, 0x41, 0x55, 0x41, 0x57, 0x48, 0x8d, 0x6c, 0x24, 0xc9]),
    (UN_PAS, &[0x44, 0x89, 0x44, 0x24, 0x18, 0x48, 0x89, 0x54, 0x24, 0x10, 0x55, 0x53]),
    (PLACER, &[0x40, 0x55, 0x57, 0x41, 0x54, 0x41, 0x55, 0x48, 0x8d, 0xac, 0x24]),
    (COMPTER, &[0x48, 0x89, 0x6c, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18]),
    (UNITES_DE_LA_CASE, &[0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x7c, 0x24, 0x10]),
    (COUCHE, &[0x48, 0x89, 0x5c, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57]),
];

const U_FORMATION: usize = 0xbb8;
const U_X: usize = 0xf8;
const U_Y: usize = 0xfc;
const U_PROPRIETAIRE: usize = 0x128;
const U_ID: usize = 0xb0;
const F_MOUVEMENTS: usize = 0xf0;
const R_DONNEES: usize = 8; // requête : données de l'opération ; `+8` drapeaux de déplacement
const VT_DESTINATION: usize = 0x88; // (opération, données) -> objet, case en `+0x10/+0x14`
const VT_PARTENAIRE: usize = 0xc0; // (opération, unité, &case) -> partenaire
const IGNORE_STACKING: u32 = 4;
const PAS_MAX: usize = 8;

type Executer = unsafe extern "system" fn(usize, usize, usize, usize);

static BASE: AtomicUsize = AtomicUsize::new(0);
static PATCHED_BASE: AtomicUsize = AtomicUsize::new(0);
static ORIGINALE: AtomicUsize = AtomicUsize::new(0);
static CORRECTIONS: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 30;

fn gc(addr: u64) -> usize {
    BASE.load(Ordering::Relaxed) + crate::patch::rva(addr)
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

unsafe fn vcall(objet: usize, decalage: usize) -> usize {
    rd::<usize>(rd::<usize>(objet) + decalage)
}

unsafe fn position(unite: usize) -> (i32, i32) {
    (rd(unite + U_X), rd(unite + U_Y))
}

/// L'unité compte-t-elle plus d'unités sur sa case que la limite d'empilement ?
unsafe fn empilee(unite: usize) -> bool {
    let jeu: usize = rd(gc(JEU));
    let parametres: usize = rd(gc(PARAMETRES));
    if jeu == 0 || parametres == 0 {
        return false;
    }
    let carte: usize = rd(jeu + 0x248);
    if carte == 0 {
        return false;
    }
    let couche: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(gc(COUCHE));
    let grille: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(vcall(carte, 0xb8));
    let unites: unsafe extern "system" fn(usize, i32, i32, i32) -> usize = std::mem::transmute(gc(UNITES_DE_LA_CASE));
    let compter: unsafe extern "system" fn(usize, usize, usize) -> u32 = std::mem::transmute(gc(COMPTER));
    let (x, y) = position(unite);
    let liste = unites(grille(carte), x, y, couche(unite));
    liste != 0 && compter(unite, liste, 0) > rd::<u32>(parametres + 0x58c)
}

/// Fait finir à l'unité le chemin vers `arrivee`. Rend vrai si elle a bougé.
unsafe fn poursuivre(unite: usize, arrivee: [i32; 2], drapeaux: u32) -> bool {
    let formation: usize = rd(unite + U_FORMATION);
    if formation == 0 {
        return false;
    }
    let un_pas: unsafe extern "system" fn(usize, *const [i32; 2], u32) -> i32 = std::mem::transmute(gc(UN_PAS));
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

unsafe extern "system" fn executer(operation: usize, unite: usize, requete: usize, resultat: usize) {
    let originale: Executer = std::mem::transmute(ORIGINALE.load(Ordering::Relaxed));
    let donnees: usize = if requete == 0 { 0 } else { rd(requete + R_DONNEES) };
    if operation == 0 || unite == 0 || donnees == 0 || !pol::actifs() {
        return originale(operation, unite, requete, resultat);
    }
    // Comme l'original : la case visée, puis la partenaire qui s'y trouve.
    let destination: unsafe extern "system" fn(usize, usize) -> usize = std::mem::transmute(vcall(operation, VT_DESTINATION));
    let objet = destination(operation, donnees);
    let mut case_b = if objet == 0 { [-9999, -9999] } else { [rd::<i32>(objet + 0x10), rd::<i32>(objet + 0x14)] };
    let trouver: unsafe extern "system" fn(usize, usize, *mut [i32; 2]) -> usize = std::mem::transmute(vcall(operation, VT_PARTENAIRE));
    let b = trouver(operation, unite, &mut case_b);
    let drapeaux: u32 = rd(donnees + 8);
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
    let placer: unsafe extern "system" fn(usize, *const [i32; 2], u32, u32) -> u64 = std::mem::transmute(gc(PLACER));
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

pub unsafe fn on_gamecore_loaded(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    for (addr, octets) in SIGNATURES {
        let at = gc(addr);
        if crate::patch::as_known(addr, std::slice::from_raw_parts(at as *const u8, octets.len())) != octets {
            log(&format!("echange : version inattendue en {addr:#x}, échange d'unités non corrigé"));
            return;
        }
    }
    let emplacement = gc(EMPLACEMENT);
    if rd::<usize>(emplacement) != gc(EXECUTER) {
        log(&format!("echange : emplacement {EMPLACEMENT:#x} inattendu, échange d'unités non corrigé"));
        return;
    }
    ORIGINALE.store(gc(EXECUTER), Ordering::Relaxed);
    if !ecrire(emplacement, executer as *const () as usize) {
        log("echange : VirtualProtect a échoué, échange d'unités non corrigé");
        return;
    }
    PATCHED_BASE.store(base, Ordering::Relaxed);
    log(&format!("echange : exécution de SWAP_UNITS détournée (vtable {EMPLACEMENT:#x})"));
}

unsafe fn ecrire(emplacement: usize, valeur: usize) -> bool {
    let mut old = 0u32;
    if crate::win::VirtualProtect(emplacement as *mut c_void, 8, crate::win::PAGE_READWRITE, &mut old) == 0 {
        return false;
    }
    std::ptr::write_unaligned(emplacement as *mut usize, valeur);
    crate::win::VirtualProtect(emplacement as *mut c_void, 8, old, &mut old);
    true
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet l'entrée d'origine.
pub unsafe fn on_proxy_unload() {
    let base = PATCHED_BASE.swap(0, Ordering::Relaxed);
    if base == 0 {
        return;
    }
    ecrire(base + crate::patch::rva(EMPLACEMENT), base + crate::patch::rva(EXECUTER));
}

/// Nouvelle partie : les journaux recommencent.
pub fn reset() {
    CORRECTIONS.store(0, Ordering::Relaxed);
}
