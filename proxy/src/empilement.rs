//! Unités empilées : celles en excédent ne comptent plus dans les bonus de soutien et de
//! contournement. Correctif POL : il change la simulation (résolution du combat, IA), il n'est
//! appliqué que si tous les joueurs ont le proxy (voir `pol`). Pas de réglage : correction d'abus.
//!
//! Deux unités de même classe peuvent finir sur la même case (unité achetée dans une ville déjà
//! occupée, déplacement interrompu, téléportation…). Les deux compteurs du jeu parcourent toutes
//! les unités des cases voisines sans regarder l'empilement, d'où +2 par unité en trop :
//! - contournement `18036c2a0` (`COMBAT_FLANKING_BONUS_MODIFIER`) : unités de l'attaquant voisines
//!   du défenseur ;
//! - soutien `18036d1d0` (`COMBAT_SUPPORT_BONUS_MODIFIER`) : unités du défenseur voisines de lui.
//!
//! Dans chacun, l'unité de l'entrée courante est obtenue par `call 1802c4360` (conteneur d'unités
//! du joueur, id), suivi de `test rax,rax ; jz` : une unité nulle est ignorée. Cet appel (en
//! `18036c3e1` et `18036d301`) est redirigé vers un relais `mov r8,rsi ; mov r9,r14 ; jmp [rip+0]`,
//! qui ajoute le porteur du bonus (`rsi` : l'attaquant, ou le défenseur) et la liste d'unités de la
//! case en cours de parcours (`r14`). `unite_comptee` rend l'unité, ou 0 si elle est en excédent.
//! Le calcul du bonus et l'infobulle de la prévisualisation restent ceux du jeu.
//!
//! Une unité est en excédent si elle est de classe `FORMATION_CLASS_LAND_COMBAT` ou
//! `FORMATION_CLASS_NAVAL` et :
//! - le porteur est sur sa case, au même propriétaire et de la même classe (il occupe la place) ;
//! - ou une unité vivante, avec une force, du même propriétaire et de la même classe la précède
//!   dans la liste de la case. Limite fixe : une par classe, celle du jeu (`GlobalParameters+0x58c`
//!   vaut 1 et n'est changée par aucun mod connu).
//! Les autres classes (soutien, civil, aérien) gardent le comportement du jeu.
//!
//! Investigation : empilement_combat/README.md.

use crate::patch::{alloc_near, rel32, write_rel32, IMAGE_BASE};
use crate::{log, pol};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// Appels redirigés : site du `call 1802c4360`, nom du bonus.
const SITES: [(u64, &str); 2] = [(0x1_8036_c3e1, "contournement"), (0x1_8036_d301, "soutien")];
/// `(conteneur d'unités du joueur, id) -> unité`.
const TROUVER_UNITE: u64 = 0x1_802c_4360;
/// `(entrée {i16 joueur, i16 genre, i32 id}) -> unité`.
const UNITE_DE_L_ENTREE: u64 = 0x1_8006_6b80;
/// `(unité) -> force de mêlée, à distance ou de bombardement > 0`, le test des deux compteurs.
const A_UNE_FORCE: u64 = 0x1_8015_a2b0;
/// `mov edx,[rdi+4] ; mov rcx,[rax+0x6c8]` avant l'appel, `mov rbx,rax ; test rax,rax` après.
const AVANT: [u8; 10] = [0x8b, 0x57, 0x04, 0x48, 0x8b, 0x88, 0xc8, 0x06, 0x00, 0x00];
const APRES: [u8; 6] = [0x48, 0x8b, 0xd8, 0x48, 0x85, 0xc0];

const U_INFO: usize = 0x98;
const INFO_CLASSE: usize = 0x20; // hachage de FormationClass
const U_ID: usize = 0xb0;
const U_X: usize = 0xf8;
const U_Y: usize = 0xfc;
const U_PROPRIETAIRE: usize = 0x128;
const U_MORTE: usize = 0x2e8;
const L_ENTREES: usize = 0x50;
const L_NOMBRE: usize = 0x60;

/// `~CRC32` des noms, comme `DB.MakeHash` (vérifié dans `1807d8a70`).
const FORMATION_CLASS_LAND_COMBAT: u32 = 0x9ad4_85f8;
const FORMATION_CLASS_NAVAL: u32 = 0xfeeb_5b2f;

static BASE: AtomicUsize = AtomicUsize::new(0);
static PATCHED_BASE: AtomicUsize = AtomicUsize::new(0);
static IGNOREES: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

fn gc(addr: u64) -> usize {
    BASE.load(Ordering::Relaxed) + (addr - IMAGE_BASE) as usize
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

/// Ce que la règle regarde d'une unité.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Desc {
    proprietaire: i32,
    classe: u32,
    x: i32,
    y: i32,
    vivante: bool,
    force: bool,
}

/// Seules les classes limitées à une unité par case et qui comptent dans les bonus.
fn classe_limitee(classe: u32) -> bool {
    classe == FORMATION_CLASS_LAND_COMBAT || classe == FORMATION_CLASS_NAVAL
}

fn meme_groupe(u: &Desc, v: &Desc) -> bool {
    classe_limitee(u.classe) && v.proprietaire == u.proprietaire && v.classe == u.classe
}

/// `u` est-elle en excédent ? `precedents` : les unités placées avant elle dans la liste de sa case.
fn excedent(u: &Desc, porteur: Option<&Desc>, precedents: impl IntoIterator<Item = Desc>) -> bool {
    if !classe_limitee(u.classe) {
        return false;
    }
    if let Some(p) = porteur {
        if meme_groupe(u, p) && (p.x, p.y) == (u.x, u.y) {
            return true;
        }
    }
    precedents.into_iter().any(|v| meme_groupe(u, &v) && v.vivante && v.force)
}

/// Description d'une unité ; `force` n'est évaluée (appel au jeu) que si `utile` le demande.
unsafe fn decrire(unite: usize, utile: impl Fn(&Desc) -> bool) -> Desc {
    let info: usize = rd(unite + U_INFO);
    let mut d = Desc {
        proprietaire: rd(unite + U_PROPRIETAIRE),
        classe: if info == 0 { 0 } else { rd(info + INFO_CLASSE) },
        x: rd(unite + U_X),
        y: rd(unite + U_Y),
        vivante: rd::<u8>(unite + U_MORTE) == 0,
        force: false,
    };
    if utile(&d) {
        let a_une_force: unsafe extern "system" fn(usize) -> bool = std::mem::transmute(gc(A_UNE_FORCE));
        d.force = a_une_force(unite);
    }
    d
}

unsafe fn en_excedent(unite: usize, porteur: usize, liste: usize) -> bool {
    let u = decrire(unite, |_| false);
    if !classe_limitee(u.classe) {
        return false;
    }
    let p = if porteur != 0 && porteur != unite { Some(decrire(porteur, |_| false)) } else { None };
    if liste == 0 {
        return excedent(&u, p.as_ref(), []);
    }
    // Les entrées qui précèdent celle de `unite` ; sans elle dans la liste, aucune.
    let entrees: usize = rd(liste + L_ENTREES);
    let n = rd::<u32>(liste + L_NOMBRE) as usize;
    let id: i32 = rd(unite + U_ID);
    let rang = (0..n)
        .find(|&i| {
            let e = entrees + i * 8;
            rd::<i16>(e) as i32 == u.proprietaire && rd::<i32>(e + 4) == id
        })
        .unwrap_or(0);
    let unite_de: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(gc(UNITE_DE_L_ENTREE));
    let precedents = (0..rang).filter_map(|i| {
        let v = unite_de(entrees + i * 8);
        (v != 0 && v != unite).then(|| decrire(v, |d| meme_groupe(&u, d) && d.vivante))
    });
    excedent(&u, p.as_ref(), precedents)
}

unsafe fn filtrer(bonus: &str, conteneur: usize, id: u32, porteur: usize, liste: usize) -> usize {
    let trouver: unsafe extern "system" fn(usize, u32) -> usize = std::mem::transmute(gc(TROUVER_UNITE));
    let unite = trouver(conteneur, id);
    if unite == 0 || !pol::actifs() || !en_excedent(unite, porteur, liste) {
        return unite;
    }
    let k = IGNOREES.fetch_add(1, Ordering::Relaxed) + 1;
    if k <= MAX_LOGS {
        log(&format!(
            "empilement : unité {} du joueur {} en ({}, {}) en excédent, ignorée pour le {bonus} (porteur {}) ({k})",
            rd::<i32>(unite + U_ID),
            rd::<i32>(unite + U_PROPRIETAIRE),
            rd::<i32>(unite + U_X),
            rd::<i32>(unite + U_Y),
            if porteur == 0 { -1 } else { rd::<i32>(porteur + U_ID) }
        ));
    }
    0
}

unsafe extern "system" fn contournement(conteneur: usize, id: u32, attaquant: usize, liste: usize) -> usize {
    filtrer("contournement", conteneur, id, attaquant, liste)
}

unsafe extern "system" fn soutien(conteneur: usize, id: u32, defenseur: usize, liste: usize) -> usize {
    filtrer("soutien", conteneur, id, defenseur, liste)
}

unsafe fn site_attendu(at: usize) -> bool {
    std::slice::from_raw_parts((at - AVANT.len()) as *const u8, AVANT.len()) == AVANT
        && rd::<u8>(at) == 0xe8
        && rel32(at, gc(TROUVER_UNITE)) == Some(rd::<i32>(at + 1))
        && std::slice::from_raw_parts((at + 5) as *const u8, 6) == APRES
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    let at = |site: u64| base + (site - IMAGE_BASE) as usize;
    // Tout ou rien : un seul des deux bonus corrigé serait pire que le jeu d'origine.
    if let Some((site, _)) = SITES.iter().find(|(s, _)| !site_attendu(at(*s))) {
        log(&format!("empilement : version inattendue en {site:#x}, aucun appel redirigé"));
        return;
    }
    let Some(page) = alloc_near(base) else {
        log("empilement : pas de mémoire à moins de 2 Go de GameCore, aucun appel redirigé");
        return;
    };
    let cibles = [contournement as *const () as usize, soutien as *const () as usize];
    let mut rels = [0i32; 2];
    for (i, ((site, _), cible)) in SITES.iter().zip(cibles).enumerate() {
        let stub = page + i * 0x20;
        // Relais : mov r8,rsi (le porteur) ; mov r9,r14 (la liste) ; jmp [rip+0]. Un saut, pas un
        // appel : la pile reste celle de l'appel d'origine, r8 et r9 sont volatils.
        let mut code = vec![0x49, 0x89, 0xf0, 0x4d, 0x89, 0xf1, 0xff, 0x25, 0, 0, 0, 0];
        code.extend_from_slice(&(cible as u64).to_le_bytes());
        std::ptr::copy_nonoverlapping(code.as_ptr(), stub as *mut u8, code.len());
        let Some(rel) = rel32(at(*site), stub) else {
            log("empilement : relais hors de portée, aucun appel redirigé");
            return;
        };
        rels[i] = rel;
    }
    for ((site, _), rel) in SITES.iter().zip(rels) {
        write_rel32(at(*site), rel);
    }
    PATCHED_BASE.store(base, Ordering::Relaxed);
    log("empilement : unités en excédent ignorées par le soutien et le contournement (appels redirigés en 0x18036c3e1 et 0x18036d301)");
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet les appels d'origine.
pub unsafe fn on_proxy_unload() {
    let base = PATCHED_BASE.swap(0, Ordering::Relaxed);
    if base == 0 {
        return;
    }
    for (site, _) in SITES {
        let at = base + (site - IMAGE_BASE) as usize;
        if let Some(rel) = rel32(at, base + (TROUVER_UNITE - IMAGE_BASE) as usize) {
            write_rel32(at, rel);
        }
    }
}

/// Nouvelle partie : les journaux recommencent.
pub fn reset() {
    IGNOREES.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOUTIEN: u32 = 0xf1c3_2241;
    const AIR: u32 = 0x31b7_00cb;

    fn d(proprietaire: i32, classe: u32, x: i32, y: i32) -> Desc {
        Desc { proprietaire, classe, x, y, vivante: true, force: true }
    }

    #[test]
    fn hachages_des_classes() {
        let h = |s: &str| {
            // ~CRC32 (polynôme réfléchi 0xedb88320), comme DB.MakeHash.
            let mut c = 0xffff_ffffu32;
            for b in s.bytes() {
                c ^= b as u32;
                for _ in 0..8 {
                    c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
                }
            }
            c
        };
        assert_eq!(h("FORMATION_CLASS_LAND_COMBAT"), FORMATION_CLASS_LAND_COMBAT);
        assert_eq!(h("FORMATION_CLASS_NAVAL"), FORMATION_CLASS_NAVAL);
        assert_eq!(h("FORMATION_CLASS_SUPPORT"), SOUTIEN);
        assert_eq!(h("FORMATION_CLASS_AIR"), AIR);
    }

    #[test]
    fn premiere_unite_comptee_seconde_en_excedent() {
        let a = d(0, FORMATION_CLASS_LAND_COMBAT, 5, 5);
        assert!(!excedent(&a, None, []));
        assert!(excedent(&a, None, [a]));
    }

    #[test]
    fn precedent_mort_ou_sans_force_ne_compte_pas() {
        let a = d(0, FORMATION_CLASS_LAND_COMBAT, 5, 5);
        assert!(!excedent(&a, None, [Desc { vivante: false, ..a }]));
        assert!(!excedent(&a, None, [Desc { force: false, ..a }]));
    }

    #[test]
    fn autres_proprietaires_et_classes_ne_comptent_pas() {
        let a = d(0, FORMATION_CLASS_LAND_COMBAT, 5, 5);
        assert!(!excedent(&a, None, [d(1, FORMATION_CLASS_LAND_COMBAT, 5, 5)]));
        assert!(!excedent(&a, None, [d(0, FORMATION_CLASS_NAVAL, 5, 5), d(0, SOUTIEN, 5, 5)]));
        let n = d(0, FORMATION_CLASS_NAVAL, 5, 5);
        assert!(excedent(&n, None, [a, n]));
    }

    #[test]
    fn classes_non_limitees_jamais_en_excedent() {
        for c in [SOUTIEN, AIR, 0xc251_0aa7] {
            let u = d(0, c, 5, 5);
            assert!(!excedent(&u, Some(&u), [u, u]));
        }
    }

    #[test]
    fn porteur_sur_la_case_occupe_la_place() {
        let a = d(0, FORMATION_CLASS_LAND_COMBAT, 5, 5);
        // Attaquant sur la même case, même classe : le partenaire ne contourne pas.
        assert!(excedent(&a, Some(&d(0, FORMATION_CLASS_LAND_COMBAT, 5, 5)), []));
        // Ailleurs, ou d'une autre classe : sans effet.
        assert!(!excedent(&a, Some(&d(0, FORMATION_CLASS_LAND_COMBAT, 6, 5)), []));
        assert!(!excedent(&a, Some(&d(0, FORMATION_CLASS_NAVAL, 5, 5)), []));
    }
}
