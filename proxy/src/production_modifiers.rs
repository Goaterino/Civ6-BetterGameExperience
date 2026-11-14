//! Dérive des modificateurs de production d'unités (`EFFECT_ADJUST_UNIT_PRODUCTION`) : Ilkum au
//! niveau du joueur, et les modificateurs de ville (`MODIFIER_PLAYER_CITIES_ADJUST_UNIT_PRODUCTION`,
//! `MODIFIER_SINGLE_CITY_ADJUST_UNIT_PRODUCTION`).
//! Le jeu convertit un pourcentage en 24.8 par `floorf` : +30 % ajoute 76/256, mais -30 % retire
//! 77/256. Sous POL, une variation négative retire exactement la valeur qu'aurait ajoutée son
//! opposé. Les variations positives, et les négatives sans dérive (multiples de 25 %), restent
//! celles du jeu. Pas de réparation des sauvegardes.
//!
//! - Joueur : `1809497a0` (tableau `+0x200`), appelée par l'ajout `180944610` et le retrait
//!   `180973d50` de l'effet.
//! - Ville : l'effet (vtable `180a8d0f0`, ajout `180880b40`, retrait `180881800`) appelle
//!   `1809444e0` (ajout) ou `180973c20` (retrait) quand sa cible est une ville. Le calcul y est
//!   recopié dans une boucle sur les types d'unités, écrite dans `(ville+0x10c0)+0x2c0`. Le détour
//!   reprend la boucle seulement si l'un des montants dérive, sinon l'originale s'exécute.
//! - `EFFECT_ADJUST_CITY_PRODUCTION_UNIT` (`MODIFIER_PLAYER_CAPITAL_CITY_…`,
//!   `MODIFIER_PLAYER_DISTRICTS_…`) ajoute et retire un entier (`1808a1050` / `1808a16f0`) : pas de
//!   dérive, pas de détour.

use crate::patch::{self, Site};
use crate::{log, pol};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const CHANGE: Site = Site {
    name: "modificateur de production des unités",
    address: 0x1_8094_97a0,
    // Corps entier : vérifie aussi les offsets du tableau et l'appel à son éditeur FAutoArchive.
    expected: &[
        0x48, 0x89, 0x5c, 0x24, 0x08, 0x57, 0x48, 0x83, 0xec, 0x40, 0x48, 0x63, 0xda, 0x48, 0x8b, 0xf9,
        0x3b, 0x99, 0x20, 0x02, 0x00, 0x00, 0x73, 0x5d, 0x0f, 0x29, 0x74, 0x24, 0x30, 0x0f, 0x29, 0x7c,
        0x24, 0x20, 0x66, 0x41, 0x0f, 0x6e, 0xf8, 0x0f, 0x5b, 0xff, 0xf3, 0x0f, 0x59, 0x3d, 0xa6, 0xf1,
        0x07, 0x00, 0x0f, 0x28, 0xc7, 0xe8, 0xe4, 0x77, 0x06, 0x00, 0x48, 0x8d, 0x8f, 0x00, 0x02, 0x00,
        0x00, 0x0f, 0x28, 0xf0, 0xe8, 0x37, 0x11, 0xde, 0xff, 0xf3, 0x0f, 0x5c, 0xfe, 0x48, 0x8b, 0x10,
        0xf3, 0x0f, 0x59, 0x3d, 0x80, 0x83, 0x07, 0x00, 0xf3, 0x0f, 0x2c, 0xc7, 0x0f, 0x28, 0x7c, 0x24,
        0x20, 0x0f, 0xb6, 0xc8, 0xf3, 0x0f, 0x2c, 0xc6, 0x0f, 0x28, 0x74, 0x24, 0x30, 0xc1, 0xe0, 0x08,
        0x0b, 0xc8, 0x01, 0x0c, 0x9a, 0x48, 0x8b, 0x5c, 0x24, 0x50, 0x48, 0x83, 0xc4, 0x40, 0x5f, 0xc3,
    ],
    copy: (0, 16), // Instructions entières, sans adressage relatif.
    resume: 16,
};

/// Corps entier de `1809444e0` : offsets de l'effet, de la ville et du joueur, appels à l'éditeur
/// FAutoArchive (`18072a920`), au joueur propriétaire (`180044f00`) et à `floorf` (`1809b0fbe`).
const VILLE_AJOUT: Site = Site {
    name: "modificateur de production des unités (ville, ajout)",
    address: 0x1_8094_44e0,
    expected: &[
        0x48, 0x89, 0x5c, 0x24, 0x20, 0x57, 0x48, 0x83, 0xec, 0x50, 0x83, 0x79, 0x30, 0x00, 0x48, 0x8b,
        0xfa, 0x48, 0x8b, 0xd9, 0x77, 0x0d, 0x32, 0xc0, 0x48, 0x8b, 0x5c, 0x24, 0x78, 0x48, 0x83, 0xc4,
        0x50, 0x5f, 0xc3, 0x48, 0x89, 0x6c, 0x24, 0x60, 0x48, 0x8d, 0x8a, 0xc0, 0x10, 0x00, 0x00, 0x4c,
        0x89, 0x74, 0x24, 0x70, 0xe8, 0x07, 0x64, 0xde, 0xff, 0x8b, 0x8f, 0xd8, 0x00, 0x00, 0x00, 0x4c,
        0x8b, 0xf0, 0xe8, 0xd9, 0x09, 0x70, 0xff, 0x33, 0xff, 0x48, 0x8b, 0xe8, 0x39, 0x7b, 0x18, 0x0f,
        0x86, 0xb6, 0x00, 0x00, 0x00, 0x0f, 0x29, 0x7c, 0x24, 0x30, 0xf3, 0x0f, 0x10, 0x3d, 0x36, 0x44,
        0x08, 0x00, 0x44, 0x0f, 0x29, 0x44, 0x24, 0x20, 0xf3, 0x44, 0x0f, 0x10, 0x05, 0x27, 0xd6, 0x07,
        0x00, 0x48, 0x89, 0x74, 0x24, 0x68, 0x0f, 0x29, 0x74, 0x24, 0x40, 0x0f, 0x1f, 0x44, 0x00, 0x00,
        0x48, 0x8b, 0x4b, 0x08, 0x48, 0x63, 0x04, 0xb9, 0x85, 0xc0, 0x78, 0x63, 0x48, 0x8b, 0xd0, 0x48,
        0x8b, 0x85, 0xc8, 0x06, 0x00, 0x00, 0x48, 0x8b, 0x88, 0x28, 0x01, 0x00, 0x00, 0x48, 0x63, 0x34,
        0x91, 0x83, 0xfe, 0xff, 0x74, 0x49, 0x3b, 0x7b, 0x30, 0x73, 0x0a, 0x48, 0x8b, 0x43, 0x20, 0x48,
        0x8d, 0x0c, 0xb8, 0xeb, 0x04, 0x48, 0x8b, 0x4b, 0x20, 0x66, 0x0f, 0x6e, 0x31, 0x0f, 0x5b, 0xf6,
        0xf3, 0x0f, 0x59, 0xf7, 0x0f, 0x28, 0xc6, 0xe8, 0x12, 0xca, 0x06, 0x00, 0x49, 0x8b, 0x96, 0xc0,
        0x02, 0x00, 0x00, 0xf3, 0x0f, 0x5c, 0xf0, 0xf3, 0x41, 0x0f, 0x59, 0xf0, 0xf3, 0x0f, 0x2c, 0xc6,
        0x0f, 0xb6, 0xc8, 0xf3, 0x0f, 0x2c, 0xc0, 0xc1, 0xe0, 0x08, 0x0b, 0xc8, 0x01, 0x0c, 0xb2, 0xff,
        0xc7, 0x3b, 0x7b, 0x18, 0x72, 0x8a, 0x44, 0x0f, 0x28, 0x44, 0x24, 0x20, 0x0f, 0x28, 0x7c, 0x24,
        0x30, 0x0f, 0x28, 0x74, 0x24, 0x40, 0x48, 0x8b, 0x74, 0x24, 0x68, 0x4c, 0x8b, 0x74, 0x24, 0x70,
        0xb0, 0x01, 0x48, 0x8b, 0x6c, 0x24, 0x60, 0x48, 0x8b, 0x5c, 0x24, 0x78, 0x48, 0x83, 0xc4, 0x50,
        0x5f, 0xc3,
    ],
    // mov [rsp+20],rbx ; push rdi ; sub rsp,50 ; cmp dword [rcx+30],0 : sans adressage relatif. Le
    // `ja` qui suit lit les drapeaux du `cmp`, que les deux `mov` et le saut de retour préservent.
    copy: (0, 14),
    resume: 14,
};

/// Corps entier de `180973c20` : identique à l'ajout, montant négé (`neg eax`) avant la conversion.
const VILLE_RETRAIT: Site = Site {
    name: "modificateur de production des unités (ville, retrait)",
    address: 0x1_8097_3c20,
    expected: &[
        0x48, 0x89, 0x5c, 0x24, 0x20, 0x57, 0x48, 0x83, 0xec, 0x50, 0x83, 0x79, 0x30, 0x00, 0x48, 0x8b,
        0xfa, 0x48, 0x8b, 0xd9, 0x77, 0x0d, 0x32, 0xc0, 0x48, 0x8b, 0x5c, 0x24, 0x78, 0x48, 0x83, 0xc4,
        0x50, 0x5f, 0xc3, 0x48, 0x89, 0x6c, 0x24, 0x60, 0x48, 0x8d, 0x8a, 0xc0, 0x10, 0x00, 0x00, 0x4c,
        0x89, 0x74, 0x24, 0x70, 0xe8, 0xc7, 0x6c, 0xdb, 0xff, 0x8b, 0x8f, 0xd8, 0x00, 0x00, 0x00, 0x4c,
        0x8b, 0xf0, 0xe8, 0x99, 0x12, 0x6d, 0xff, 0x33, 0xff, 0x48, 0x8b, 0xe8, 0x39, 0x7b, 0x18, 0x0f,
        0x86, 0xba, 0x00, 0x00, 0x00, 0x0f, 0x29, 0x7c, 0x24, 0x30, 0xf3, 0x0f, 0x10, 0x3d, 0xf6, 0x4c,
        0x05, 0x00, 0x44, 0x0f, 0x29, 0x44, 0x24, 0x20, 0xf3, 0x44, 0x0f, 0x10, 0x05, 0xe7, 0xde, 0x04,
        0x00, 0x48, 0x89, 0x74, 0x24, 0x68, 0x0f, 0x29, 0x74, 0x24, 0x40, 0x0f, 0x1f, 0x44, 0x00, 0x00,
        0x48, 0x8b, 0x4b, 0x08, 0x48, 0x63, 0x04, 0xb9, 0x85, 0xc0, 0x78, 0x67, 0x48, 0x8b, 0xd0, 0x48,
        0x8b, 0x85, 0xc8, 0x06, 0x00, 0x00, 0x48, 0x8b, 0x88, 0x28, 0x01, 0x00, 0x00, 0x48, 0x63, 0x34,
        0x91, 0x83, 0xfe, 0xff, 0x74, 0x4d, 0x3b, 0x7b, 0x30, 0x73, 0x0a, 0x48, 0x8b, 0x43, 0x20, 0x48,
        0x8d, 0x0c, 0xb8, 0xeb, 0x04, 0x48, 0x8b, 0x4b, 0x20, 0x8b, 0x01, 0xf7, 0xd8, 0x66, 0x0f, 0x6e,
        0xf0, 0x0f, 0x5b, 0xf6, 0xf3, 0x0f, 0x59, 0xf7, 0x0f, 0x28, 0xc6, 0xe8, 0xce, 0xd2, 0x03, 0x00,
        0x49, 0x8b, 0x96, 0xc0, 0x02, 0x00, 0x00, 0xf3, 0x0f, 0x5c, 0xf0, 0xf3, 0x41, 0x0f, 0x59, 0xf0,
        0xf3, 0x0f, 0x2c, 0xc6, 0x0f, 0xb6, 0xc8, 0xf3, 0x0f, 0x2c, 0xc0, 0xc1, 0xe0, 0x08, 0x0b, 0xc8,
        0x01, 0x0c, 0xb2, 0xff, 0xc7, 0x3b, 0x7b, 0x18, 0x72, 0x86, 0x44, 0x0f, 0x28, 0x44, 0x24, 0x20,
        0x0f, 0x28, 0x7c, 0x24, 0x30, 0x0f, 0x28, 0x74, 0x24, 0x40, 0x48, 0x8b, 0x74, 0x24, 0x68, 0x4c,
        0x8b, 0x74, 0x24, 0x70, 0xb0, 0x01, 0x48, 0x8b, 0x6c, 0x24, 0x60, 0x48, 0x8b, 0x5c, 0x24, 0x78,
        0x48, 0x83, 0xc4, 0x50, 0x5f, 0xc3,
    ],
    copy: (0, 14),
    resume: 14,
};

const SITES: [Site; 3] = [CHANGE, VILLE_AJOUT, VILLE_RETRAIT];

static BASE: AtomicUsize = AtomicUsize::new(0);
static TRAMP: AtomicUsize = AtomicUsize::new(0);
static TRAMP_AJOUT: AtomicUsize = AtomicUsize::new(0);
static TRAMP_RETRAIT: AtomicUsize = AtomicUsize::new(0);
/// Détours posés, un bit par entrée de `SITES` : seuls ceux-là sont retirés au déchargement.
static POSES: AtomicU32 = AtomicU32::new(0);
type Change = unsafe extern "system" fn(usize, i32, i32);
type Edit = unsafe extern "system" fn(usize) -> *mut *mut i32;
type EditObjet = unsafe extern "system" fn(usize) -> usize;
type Joueur = unsafe extern "system" fn(i32) -> usize;
type Ville = unsafe extern "system" fn(usize, usize) -> bool;
const EDIT: usize = 0x72a920;
const JOUEUR: usize = 0x44f00;

// Effet : types d'unités (`+0x08`, i32) et leur nombre (`+0x18`), montants (`+0x20`, i32) et leur
// nombre (`+0x30`) ; au-delà du nombre de montants, le premier sert pour tous.
const E_TYPES: usize = 0x08;
const E_NB_TYPES: usize = 0x18;
const E_MONTANTS: usize = 0x20;
const E_NB_MONTANTS: usize = 0x30;
/// Ville : propriétaire (i32), et objet FAutoArchive dont `+0x2c0` porte le tableau 24.8.
const V_PROPRIETAIRE: usize = 0xd8;
const V_OBJET: usize = 0x10c0;
const O_TABLEAU: usize = 0x2c0;
/// Joueur : unités en `+0x6c8`, dont `+0x128` donne l'index de chaque type d'unité (-1 : aucun).
const J_UNITES: usize = 0x6c8;
const U_INDEX: usize = 0x128;

/// Opposé de la conversion positive du jeu : multiplication f32 par 0,01, puis
/// troncature en 24.8. Multiplier par 256 est exact. Hors plage, garder l'originale.
fn retrait(percent: i32) -> Option<i32> {
    let magnitude = -(percent as f32) * 0.01 * 256.0;
    (percent < 0 && magnitude < 2_147_483_648.0).then(|| -(magnitude as i32))
}

/// Conversion du jeu, instruction pour instruction : `x = p * 0,01f`, `floorf`, fraction × 256
/// tronquée sur un octet, partie entière décalée de 8.
fn conversion_jeu(percent: i32) -> i32 {
    let x = percent as f32 * 0.01;
    let entier = x.floor();
    (((x - entier) * 256.0) as i32 & 0xff) | (entier as i32).wrapping_shl(8)
}

/// Conversion sous POL : celle du jeu, sauf une variation négative qui dérive.
fn conversion(percent: i32) -> i32 {
    retrait(percent).unwrap_or_else(|| conversion_jeu(percent))
}

fn derive(percent: i32) -> bool {
    conversion(percent) != conversion_jeu(percent)
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

unsafe fn changer(units: usize, index: i32, percent: i32, actif: bool) {
    if let Some(delta) = retrait(percent).filter(|_| actif) {
        // Même comparaison non signée et même accès en écriture que l'originale :
        // préserver le suivi des modifications de FAutoArchive, pas d'écriture brute à +0x210.
        if (index as u32) < *((units + 0x220) as *const u32) {
            let edit: Edit = std::mem::transmute(BASE.load(Ordering::Relaxed) + EDIT);
            let entry = (*edit(units + 0x200)).add(index as usize);
            *entry = (*entry).wrapping_add(delta);
        }
    } else {
        let original: Change = std::mem::transmute(TRAMP.load(Ordering::Relaxed));
        original(units, index, percent);
    }
}

unsafe extern "system" fn hook(units: usize, index: i32, percent: i32) {
    changer(units, index, percent, percent < 0 && pol::actifs());
}

/// Montant de la `i`-ième entrée de l'effet, comme le lit la boucle du jeu, négé au retrait.
unsafe fn montant(effet: usize, i: u32, retire: bool) -> i32 {
    let base: usize = rd(effet + E_MONTANTS);
    let p: i32 = if i < rd::<u32>(effet + E_NB_MONTANTS) { rd(base + 4 * i as usize) } else { rd(base) };
    if retire { p.wrapping_neg() } else { p }
}

/// L'un des montants de l'effet dérive-t-il ? Sinon la boucle du jeu donne déjà le bon résultat.
unsafe fn effet_derive(effet: usize, retire: bool) -> bool {
    rd::<u32>(effet + E_NB_MONTANTS) != 0 && (0..rd::<u32>(effet + E_NB_TYPES)).any(|i| derive(montant(effet, i, retire)))
}

/// Boucle de `1809444e0` / `180973c20` après ses deux appels, avec la conversion corrigée.
unsafe fn boucle(effet: usize, objet: usize, joueur: usize, retire: bool) {
    for i in 0..rd::<u32>(effet + E_NB_TYPES) {
        let t: i32 = rd(rd::<usize>(effet + E_TYPES) + 4 * i as usize);
        if t < 0 {
            continue;
        }
        let index: i32 = rd(rd::<usize>(rd::<usize>(joueur + J_UNITES) + U_INDEX) + 4 * t as usize);
        if index == -1 {
            continue;
        }
        let entree = (rd::<usize>(objet + O_TABLEAU) as isize + 4 * index as isize) as *mut i32;
        *entree = (*entree).wrapping_add(conversion(montant(effet, i, retire)));
    }
}

unsafe fn ville(effet: usize, cite: usize, retire: bool) -> bool {
    if !(effet_derive(effet, retire) && pol::actifs()) {
        let tramp = if retire { &TRAMP_RETRAIT } else { &TRAMP_AJOUT };
        let original: Ville = std::mem::transmute(tramp.load(Ordering::Relaxed));
        return original(effet, cite);
    }
    // Mêmes appels, dans le même ordre, que l'originale : l'éditeur marque l'objet modifié.
    let base = BASE.load(Ordering::Relaxed);
    let edit: EditObjet = std::mem::transmute(base + EDIT);
    let joueur: Joueur = std::mem::transmute(base + JOUEUR);
    let objet = edit(cite + V_OBJET);
    let proprietaire = joueur(rd(cite + V_PROPRIETAIRE));
    boucle(effet, objet, proprietaire, retire);
    true
}

unsafe extern "system" fn hook_ville_ajout(effet: usize, cite: usize) -> bool {
    ville(effet, cite, false)
}

unsafe extern "system" fn hook_ville_retrait(effet: usize, cite: usize) -> bool {
    ville(effet, cite, true)
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    if let Err(e) = patch::check(base, &SITES) {
        crate::STATUS.fetch_and(!crate::ST_POL, Ordering::Relaxed);
        log(&format!("production : aucun détour : {e}"));
        return;
    }
    // Le trampoline et la base sont publiés avant le saut : `install` écrit le saut en dernier.
    BASE.store(base, Ordering::Relaxed);
    let poses: [(&AtomicUsize, usize); 3] = [
        (&TRAMP, hook as *const () as usize),
        (&TRAMP_AJOUT, hook_ville_ajout as *const () as usize),
        (&TRAMP_RETRAIT, hook_ville_retrait as *const () as usize),
    ];
    for (i, (site, (tramp, detour))) in SITES.iter().zip(poses).enumerate() {
        match patch::install(base, site, detour) {
            Ok(t) => {
                tramp.store(t, Ordering::Relaxed);
                POSES.fetch_or(1 << i, Ordering::Relaxed);
            }
            Err(e) => {
                crate::STATUS.fetch_and(!crate::ST_POL, Ordering::Relaxed);
                log(&format!("production : échec du détour : {e}"));
                return;
            }
        }
    }
    log("production : retrait symétrique des modificateurs d'unités posé, joueur et villes (POL)");
}

pub unsafe fn on_proxy_unload() {
    let base = BASE.swap(0, Ordering::Relaxed);
    let poses = POSES.swap(0, Ordering::Relaxed);
    if base != 0 {
        for (i, site) in SITES.iter().enumerate() {
            if poses & (1 << i) != 0 {
                patch::restore(base, site);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrait_symetrique() {
        // Ajouts relevés dans la routine native, dont les valeurs exactement représentables.
        for (percent, added) in [(5, 12), (10, 25), (15, 38), (25, 64), (30, 76), (50, 128)] {
            assert_eq!(retrait(-percent), Some(-added));
        }
        assert_eq!(retrait(0), None);
        assert_eq!(retrait(30), None);
        assert_eq!(retrait(i32::MIN), None);
    }

    #[test]
    fn conversion_du_jeu() {
        // Valeurs de la routine native : arrondi vers -∞ pour un négatif.
        assert_eq!(conversion_jeu(30), 76);
        assert_eq!(conversion_jeu(-30), -77);
        assert_eq!(conversion_jeu(-50), -128);
        assert_eq!(conversion_jeu(150), 384);
        for p in 1..=100_000 {
            assert_eq!(conversion(p), conversion_jeu(p), "positif inchangé : {p}");
            assert_eq!(conversion(-p), -conversion_jeu(p), "retrait symétrique : {p}");
            assert_eq!(derive(-p), conversion_jeu(-p) != -conversion_jeu(p));
        }
        assert!(!derive(-25) && !derive(-50) && !derive(-100) && derive(-30) && !derive(30));
        assert_eq!(conversion(i32::MIN), conversion_jeu(i32::MIN));
    }

    /// Effet, joueur et objet de ville factices aux offsets du jeu.
    struct Monde {
        effet: Vec<u8>,
        joueur: Vec<u8>,
        unites: Vec<u8>,
        objet: Vec<u8>,
        _types: Vec<i32>,
        _montants: Vec<i32>,
        _index: Vec<i32>,
        tableau: Vec<i32>,
    }

    fn ecrire<T>(buf: &mut [u8], off: usize, v: T) {
        unsafe { std::ptr::write_unaligned(buf.as_mut_ptr().add(off) as *mut T, v) }
    }

    fn monde(types: Vec<i32>, montants: Vec<i32>, index: Vec<i32>, cases: usize) -> Monde {
        let mut m = Monde {
            effet: vec![0; 0x40],
            joueur: vec![0; 0x6d0],
            unites: vec![0; 0x130],
            objet: vec![0; 0x2c8],
            _types: types,
            _montants: montants,
            _index: index,
            tableau: vec![256; cases],
        };
        let (nt, nm) = (m._types.len() as u32, m._montants.len() as u32);
        let (pt, pm, pi, pa) = (m._types.as_ptr() as usize, m._montants.as_ptr() as usize, m._index.as_ptr() as usize, m.tableau.as_mut_ptr() as usize);
        ecrire(&mut m.effet, E_TYPES, pt);
        ecrire(&mut m.effet, E_NB_TYPES, nt);
        ecrire(&mut m.effet, E_MONTANTS, pm);
        ecrire(&mut m.effet, E_NB_MONTANTS, nm);
        ecrire(&mut m.unites, U_INDEX, pi);
        let pu = m.unites.as_ptr() as usize;
        ecrire(&mut m.joueur, J_UNITES, pu);
        ecrire(&mut m.objet, O_TABLEAU, pa);
        m
    }

    #[test]
    fn boucle_de_ville() {
        // Types 0 à 3 : le 1 n'a pas d'index chez le joueur, -1 est ignoré ; un seul montant (30 %)
        // sert pour tous les types, comme dans la boucle du jeu.
        let m = monde(vec![0, 1, -1, 2], vec![30], vec![2, -1, 0], 3);
        let (e, o, j) = (m.effet.as_ptr() as usize, m.objet.as_ptr() as usize, m.joueur.as_ptr() as usize);
        unsafe {
            assert!(!effet_derive(e, false));
            assert!(effet_derive(e, true));
            boucle(e, o, j, false);
            assert_eq!(m.tableau, [256 + 76, 256, 256 + 76]);
            boucle(e, o, j, true);
        }
        assert_eq!(m.tableau, [256, 256, 256], "ajout puis retrait : retour exact");
        // Montant par type ; -50 % est exact, -30 % dérive.
        let m2 = monde(vec![0, 2], vec![-50, 25], vec![0, 0, 1], 2);
        unsafe {
            let e2 = m2.effet.as_ptr() as usize;
            assert!(!effet_derive(e2, false) && !effet_derive(e2, true));
            let m3 = monde(vec![0], vec![-30], vec![0], 1);
            assert!(effet_derive(m3.effet.as_ptr() as usize, false));
            assert!(!effet_derive(m3.effet.as_ptr() as usize, true));
        }
    }
}
