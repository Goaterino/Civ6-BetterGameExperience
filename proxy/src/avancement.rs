//! Science et culture en excédent du joueur local, montrées par le compteur d'avancement
//! (`ui_avancement.lua`, réglage `ui_avancement`, voir `ui`).
//!
//! Quand une technologie (un dogme) est terminée avec plus que son coût, GameCore garde le surplus
//! dans `m_xResearchOverflow` (`m_xCulturalOverflow`) et le verse plus tard dans la recherche en
//! cours. Ces champs ne sont pas dans le cache de l'interface ni exposés au Lua. Relevé dans
//! GameCore_XP2 (2026-11-10) :
//! - joueur de jeu : `180306b80` rend le gestionnaire des joueurs (`mov rax, [180b8e140]`),
//!   pointeurs de joueurs en `+0x20`, table de validité en `+0x2b48` (i32, -1 si absent), comme le
//!   fait `1806e8120` pour le Lua de jeu ;
//! - `PlayerTechs` en `joueur + 0x718` (`lGetTechs`, `1806edf40`), `PlayerCulture` en
//!   `joueur + 0x6f0` (`lGetCulture`, `1806eda40`) ;
//! - chaque champ sérialisé est `{vtable, propriétaire, valeur}` : `m_xResearchOverflow` en
//!   `PlayerTechs + 0xf8` (constructeur `180331aa0`), `m_xCulturalOverflow` en
//!   `PlayerCulture + 0x420` (constructeur `180273c30`), valeur en virgule fixe 24.8 à `+0x10`.
//!
//! À chaque lecture, le champ doit porter la vtable des valeurs 24.8 (`1809ef168`) et pointer vers
//! son objet : sinon (autre version), rien n'est rendu. Lecture seule, du fil de l'interface.
//!
//! Le Lua lit le relevé par `BGD_Excedent(joueur, 0 science | 1 culture)` → points, ou rien.

use crate::debordement::{hks, LUA_GLOBALSINDEX};
use crate::log;
use crate::patch::IMAGE_BASE;
use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static BASE: AtomicUsize = AtomicUsize::new(0);

const JOUEURS: u64 = 0x1_8030_6b80; // () -> gestionnaire des joueurs
const GET_TECHS: u64 = 0x1_806e_df40; // lGetTechs (jeu)
const GET_CULTURE: u64 = 0x1_806e_da40; // lGetCulture (jeu)
const VTABLE_FIXE: u64 = 0x1_809e_f168; // vtable des champs 24.8 sérialisés

const J_POINTEURS: usize = 0x20;
const J_VALIDES: usize = 0x2b48;
const J_MAX: i32 = 0x41;
const JOUEUR_TECHS: usize = 0x718;
const JOUEUR_CULTURE: usize = 0x6f0;
const TECHS_EXCEDENT: usize = 0xf8;
const CULTURE_EXCEDENT: usize = 0x420;

/// Octets attendus : (adresse, décalage, octets).
const SIGNATURES: [(u64, usize, &[u8]); 3] = [
    (JOUEURS, 0, &[0x48, 0x8b, 0x05, 0xb9, 0x75, 0x88, 0x00, 0xc3]),
    (GET_TECHS, 0x13, &[0x48, 0x8b, 0x90, 0x18, 0x07, 0x00, 0x00]),
    (GET_CULTURE, 0x13, &[0x48, 0x8b, 0x90, 0xf0, 0x06, 0x00, 0x00]),
];

fn gc(addr: u64) -> usize {
    BASE.load(Ordering::Relaxed) + (addr - IMAGE_BASE) as usize
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read(addr as *const T)
}

/// Valeur d'un champ 24.8 sérialisé de `objet` en `champ`, si sa vtable et son propriétaire sont
/// ceux attendus.
unsafe fn champ_fixe(objet: usize, champ: usize) -> Option<i32> {
    if objet == 0 || rd::<usize>(objet + champ) != gc(VTABLE_FIXE) || rd::<usize>(objet + champ + 8) != objet {
        return None;
    }
    Some(rd::<i32>(objet + champ + 0x10))
}

/// Excédent du joueur, en points : science (`culture` faux) ou culture.
unsafe fn excedent(joueur: i32, culture: bool) -> Option<f64> {
    if BASE.load(Ordering::Relaxed) == 0 || !(0..J_MAX).contains(&joueur) {
        return None;
    }
    let f: unsafe extern "system" fn() -> usize = std::mem::transmute(gc(JOUEURS));
    let gestion = f();
    if gestion == 0 {
        return None;
    }
    let valides = rd::<usize>(gestion + J_VALIDES);
    if valides == 0 || rd::<i32>(valides + joueur as usize * 4) == -1 {
        return None;
    }
    let pointeurs = rd::<usize>(gestion + J_POINTEURS);
    let p = if pointeurs == 0 { 0 } else { rd::<usize>(pointeurs + joueur as usize * 8) };
    if p == 0 {
        return None;
    }
    let (objet, champ) = if culture {
        (rd::<usize>(p + JOUEUR_CULTURE), CULTURE_EXCEDENT)
    } else {
        (rd::<usize>(p + JOUEUR_TECHS), TECHS_EXCEDENT)
    };
    champ_fixe(objet, champ).map(points)
}

/// Virgule fixe 24.8 → points.
fn points(v: i32) -> f64 {
    v as f64 / 256.0
}

// ---------------------------------------------------------------------------------------------
// Lua.

const NOM: &[u8] = b"BGD_Excedent\0";

unsafe extern "C-unwind" fn lua_excedent(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let (joueur, quoi) = ((h.to_integer)(l, 1), (h.to_integer)(l, 2));
    match excedent(joueur, quoi == 1) {
        Some(v) => {
            (h.push_number)(l, v);
            1
        }
        None => 0,
    }
}

unsafe extern "C-unwind" fn installer(l: *mut c_void) -> i32 {
    if let Some(h) = hks() {
        (h.push_closure)(l, lua_excedent, 0, NOM.as_ptr() as *const c_char, 0);
        (h.set_field)(l, LUA_GLOBALSINDEX, NOM.as_ptr() as *const c_char);
    }
    0
}

static PUBLIE_LOG: AtomicBool = AtomicBool::new(false);

/// Pose `BGD_Excedent` dans la table globale de l'état qui compile le script. Rend vrai si elle y
/// est ; sans elle, le compteur n'affiche pas d'excédent.
pub unsafe fn publier(l: *mut c_void) -> bool {
    if BASE.load(Ordering::Relaxed) == 0 {
        return false;
    }
    let r = match hks() {
        Some(h) => {
            let r = (h.cpcall)(l, installer, std::ptr::null_mut());
            if r != 0 {
                (h.set_top)(l, -2);
            }
            r
        }
        None => -1,
    };
    if r != 0 || !PUBLIE_LOG.swap(true, Ordering::Relaxed) {
        log(&if r == 0 { "avancement : BGD_Excedent posée".to_string() } else { format!("avancement : BGD_Excedent non posée ({r})") });
    }
    r == 0
}

/// Appelé quand l'originale vient d'être chargée : vérifie la version, rien n'est modifié.
pub unsafe fn on_gamecore_loaded(base: usize) {
    for (addr, decalage, octets) in SIGNATURES {
        let at = base + (addr - IMAGE_BASE) as usize + decalage;
        if std::slice::from_raw_parts(at as *const u8, octets.len()) != octets {
            log(&format!("avancement : version inattendue en {addr:#x}, excédents de science et de culture non lus"));
            return;
        }
    }
    BASE.store(base, Ordering::Relaxed);
}

/// Le proxy est déchargé : plus de lecture.
pub fn on_proxy_unload() {
    BASE.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_en_virgule_fixe() {
        assert_eq!(points(13 * 256 + 128), 13.5);
        assert_eq!(points(0), 0.0);
    }

    #[test]
    fn champ_controle() {
        unsafe {
            BASE.store(0x10_0000, Ordering::Relaxed);
            let mut objet = [0usize; 8];
            let o = objet.as_mut_ptr();
            let p = o as usize;
            *o.add(2) = gc(VTABLE_FIXE);
            *o.add(3) = p;
            *o.add(4) = (40 * 256 + 64) as usize;
            assert_eq!(champ_fixe(p, 0x10), Some(40 * 256 + 64));
            *o.add(3) = p + 8; // autre propriétaire
            assert_eq!(champ_fixe(p, 0x10), None);
            *o.add(3) = p;
            *o.add(2) = 0; // autre vtable
            assert_eq!(champ_fixe(p, 0x10), None);
            assert_eq!(champ_fixe(0, 0x10), None);
            BASE.store(0, Ordering::Relaxed);
        }
    }

    /// Les signatures relevées correspondent au GameCore installé, s'il est là.
    #[test]
    fn signatures_du_jeu() {
        let path = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\DLC\Expansion2\Binaries\Win64\GameCore_XP2_FinalRelease.dll";
        let Ok(b) = std::fs::read(path) else { return };
        let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize;
        let pe = u32_at(0x3c);
        let n = u16::from_le_bytes(b[pe + 6..pe + 8].try_into().unwrap()) as usize;
        let opt = u16::from_le_bytes(b[pe + 20..pe + 22].try_into().unwrap()) as usize;
        let fichier = |addr: u64| {
            let rva = (addr - IMAGE_BASE) as usize;
            (0..n).find_map(|i| {
                let s = pe + 24 + opt + i * 40;
                let (taille, va, brut) = (u32_at(s + 8), u32_at(s + 12), u32_at(s + 20));
                (va..va + taille).contains(&rva).then(|| brut + rva - va)
            })
        };
        for (addr, decalage, octets) in SIGNATURES {
            let o = fichier(addr).unwrap() + decalage;
            assert_eq!(&b[o..o + octets.len()], octets, "{addr:#x}");
        }
    }
}
