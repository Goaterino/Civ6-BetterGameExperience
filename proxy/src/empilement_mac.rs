//! Unités empilées, sous macOS : le pendant d'empilement.rs (voir son en-tête). Correctif POL. La
//! règle de l'excédent est commune avec Windows (empilement_regle.rs).
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image : les deux
//! compteurs lisent `GlobalParameters+0x1b0` (`COMBAT_FLANKING_BONUS_MODIFIER`, contournement,
//! `0xad947c`) et `+0x234` (`COMBAT_SUPPORT_BONUS_MODIFIER`, soutien, `0xad75c8`). Chacun parcourt
//! les six cases voisines et, pour chaque entrée de la liste d'unités de la case, appelle
//! `0xa65924(conteneur d'unités du joueur, id)` suivi de `cbz x0` : en `0xad9650` (porteur dans
//! `x21`, liste dans `x27`) et en `0xad7760` (porteur `x21`, liste `x26`). Le test de force y est en
//! ligne (`0xba8078(unité, 0)`, `0xba8220`, `0xba8318`).
//!
//! Offsets de l'unité sous macOS (ceux de Windows plus 8) : `UnitInfo` en `+0xa0` (classe de
//! formation, hachée, en `+0x20`), id `+0xb8`, case `+0x100`/`+0x104`, propriétaire `+0x130`,
//! morte `+0x2f0`. Unité d'une entrée `{i16 joueur, i16 genre, i32 id}` : `0x6c3874`.

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]

use super::empilement_regle::{classe_limitee, excedent, meme_groupe, Desc};
use super::journal::log;
use super::{patch, pol};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

struct Appel {
    bonus: &'static str,
    /// `bl TROUVER_UNITE`, précédé de `ldr x0, [x0, #0x6d0]` puis `ldr w1, [xN, #4]`, suivi de `cbz x0`.
    site: usize,
    /// `ldr w1, [xN, #4]` : l'entrée courante.
    avant: u32,
    /// Registre de la liste de la case (le porteur est toujours `x21`).
    liste: u32,
}

const APPELS: [Appel; 2] = [
    Appel { bonus: "contournement", site: 0xad9650, avant: 0xb940_06e1, liste: 27 },
    Appel { bonus: "soutien", site: 0xad7760, avant: 0xb940_0681, liste: 26 },
];
const CONTENEUR: u32 = 0xf943_6800; // ldr x0, [x0, #0x6d0]
const APRES: u32 = 0xb4ff_fe00; // cbz x0, … (mêmes octets aux deux sites)

const TROUVER_UNITE: usize = 0xa65924;
const UNITE_DE_L_ENTREE: usize = 0x6c3874;
const FORCE_MELEE: usize = 0xba8078;
const FORCE_DISTANCE: usize = 0xba8220;
const FORCE_BOMBARDEMENT: usize = 0xba8318;

const U_INFO: usize = 0xa0;
const INFO_CLASSE: usize = 0x20;
const U_ID: usize = 0xb8;
const U_X: usize = 0x100;
const U_Y: usize = 0x104;
const U_PROPRIETAIRE: usize = 0x130;
const U_MORTE: usize = 0x2f0;
const L_ENTREES: usize = 0x50;
const L_NOMBRE: usize = 0x60;

static BASE: AtomicUsize = AtomicUsize::new(0);
static IGNOREES: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

fn gc(addr: usize) -> usize {
    BASE.load(Ordering::Relaxed) + addr
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

/// Le test des deux compteurs : force de mêlée, à distance ou de bombardement.
unsafe fn a_une_force(unite: usize) -> bool {
    let melee: unsafe extern "C" fn(usize, u32) -> i32 = std::mem::transmute(gc(FORCE_MELEE));
    let distance: unsafe extern "C" fn(usize) -> i32 = std::mem::transmute(gc(FORCE_DISTANCE));
    let bombardement: unsafe extern "C" fn(usize) -> i32 = std::mem::transmute(gc(FORCE_BOMBARDEMENT));
    melee(unite, 0) > 0 || distance(unite) > 0 || bombardement(unite) >= 1
}

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
    let entrees: usize = rd(liste + L_ENTREES);
    let n = rd::<u32>(liste + L_NOMBRE) as usize;
    let id: i32 = rd(unite + U_ID);
    let rang = (0..n)
        .find(|&i| {
            let e = entrees + i * 8;
            rd::<i16>(e) as i32 == u.proprietaire && rd::<i32>(e + 4) == id
        })
        .unwrap_or(0);
    let unite_de: unsafe extern "C" fn(usize) -> usize = std::mem::transmute(gc(UNITE_DE_L_ENTREE));
    let precedents = (0..rang).filter_map(|i| {
        let v = unite_de(entrees + i * 8);
        (v != 0 && v != unite).then(|| decrire(v, |d| meme_groupe(&u, d) && d.vivante))
    });
    excedent(&u, p.as_ref(), precedents)
}

unsafe fn filtrer(bonus: &str, conteneur: usize, id: u32, porteur: usize, liste: usize) -> usize {
    let trouver: unsafe extern "C" fn(usize, u32) -> usize = std::mem::transmute(gc(TROUVER_UNITE));
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

unsafe extern "C" fn contournement(conteneur: usize, id: u32, attaquant: usize, liste: usize) -> usize {
    filtrer("contournement", conteneur, id, attaquant, liste)
}

unsafe extern "C" fn soutien(conteneur: usize, id: u32, defenseur: usize, liste: usize) -> usize {
    filtrer("soutien", conteneur, id, defenseur, liste)
}

unsafe fn site_attendu(a: &Appel) -> bool {
    let at = gc(a.site);
    let mot = rd::<u32>(at);
    mot & 0x8000_0000 != 0
        && patch::cible_branche(at, mot) == Some(gc(TROUVER_UNITE))
        && rd::<u32>(at - 4) == a.avant
        && rd::<u32>(at - 8) == CONTENEUR
        && rd::<u32>(at + 4) == APRES
}

/// `mov x2, x21` (le porteur), `mov x3, x<liste>`, puis saut vers l'accroche.
fn relais(a: &Appel, cible: usize) -> Vec<u8> {
    let mut v = patch::relais(&[0xaa15_03e2, 0xaa00_03e3 | (a.liste << 16)], cible);
    v.resize(32, 0);
    v
}

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    BASE.store(base, Ordering::Relaxed);
    // Tout ou rien : un seul des deux bonus corrigé serait pire que le jeu d'origine.
    if let Some(a) = APPELS.iter().find(|a| !site_attendu(a)) {
        return Err(format!("version inattendue en {:#x} ({}), aucun appel redirigé", a.site, a.bonus));
    }
    let cibles = [contournement as *const () as usize, soutien as *const () as usize];
    let code: Vec<u8> = APPELS.iter().zip(cibles).flat_map(|(a, c)| relais(a, c)).collect();
    let page = patch::executable_near(&code, gc(APPELS[0].site), 120 << 20)?;
    let mut bls = Vec::new();
    for (i, a) in APPELS.iter().enumerate() {
        let mut bl = patch::branche(gc(a.site), page + 32 * i).ok_or("relais hors de portée, aucun appel redirigé")?;
        bl[3] |= 0x80; // b → bl
        bls.push(bl);
    }
    let mut methode = "";
    for (a, bl) in APPELS.iter().zip(bls) {
        methode = patch::write_code(gc(a.site), &bl)?;
    }
    Ok(format!(
        "unités en excédent ignorées par le soutien et le contournement (appels redirigés en {:#x} et {:#x}, {methode})",
        APPELS[1].site, APPELS[0].site
    ))
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(_base: usize) -> Result<String, String> {
    Err("pas encore relevé en x86_64".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relais_arm64() {
        let r = relais(&APPELS[0], 0x1122_3344_5566_7788);
        // mov x2, x21 ; mov x3, x27 ; ldr x16, #8 ; br x16
        assert_eq!(&r[..16], &[0xe2, 0x03, 0x15, 0xaa, 0xe3, 0x03, 0x1b, 0xaa, 0x50, 0x00, 0x00, 0x58, 0x00, 0x02, 0x1f, 0xd6]);
        assert_eq!(&relais(&APPELS[1], 0)[4..8], &[0xe3, 0x03, 0x1a, 0xaa], "mov x3, x26");
    }

    #[test]
    fn sites_du_jeu() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::super::macho::tranche_arm64(&b).expect("tranche arm64");
        let mot = |a: usize| u32::from_le_bytes(t[a..a + 4].try_into().unwrap());
        for a in &APPELS {
            assert_eq!(patch::cible_branche(a.site, mot(a.site)), Some(TROUVER_UNITE), "{}", a.bonus);
            assert_eq!((mot(a.site - 8), mot(a.site - 4), mot(a.site + 4)), (CONTENEUR, a.avant, APRES), "{}", a.bonus);
        }
    }
}
