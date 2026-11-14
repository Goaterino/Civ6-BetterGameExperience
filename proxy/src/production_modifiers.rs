//! Dérive des modificateurs de production d'unités du joueur (Ilkum, notamment).
//! +30 % ajoute 76/256, mais -30 % retire 77/256. Sous POL, une variation négative
//! retire exactement la valeur qu'aurait ajoutée son opposé. Les ajouts restent ceux du jeu.
//! Pas de réparation des sauvegardes, ni de changement des autres familles de modificateurs.

use crate::patch::{self, Site};
use crate::{log, pol};
use std::sync::atomic::{AtomicUsize, Ordering};

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

static BASE: AtomicUsize = AtomicUsize::new(0);
static TRAMP: AtomicUsize = AtomicUsize::new(0);
type Change = unsafe extern "system" fn(usize, i32, i32);
type Edit = unsafe extern "system" fn(usize) -> *mut *mut i32;
const EDIT: usize = 0x72a920;

/// Opposé de la conversion positive du jeu : multiplication f32 par 0,01, puis
/// troncature en 24.8. Multiplier par 256 est exact. Hors plage, garder l'originale.
fn retrait(percent: i32) -> Option<i32> {
    let magnitude = -(percent as f32) * 0.01 * 256.0;
    (percent < 0 && magnitude < 2_147_483_648.0).then(|| -(magnitude as i32))
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

pub unsafe fn on_gamecore_loaded(base: usize) {
    if let Err(e) = patch::check(base, &[CHANGE]) {
        crate::STATUS.fetch_and(!crate::ST_POL, Ordering::Relaxed);
        log(&format!("production : aucun détour : {e}"));
        return;
    }
    match patch::install(base, &CHANGE, hook as *const () as usize) {
        Ok(t) => {
            TRAMP.store(t, Ordering::Relaxed);
            BASE.store(base, Ordering::Relaxed);
            log("production : retrait symétrique des modificateurs d'unités posé (POL)");
        }
        Err(e) => {
            crate::STATUS.fetch_and(!crate::ST_POL, Ordering::Relaxed);
            log(&format!("production : échec du détour : {e}"));
        }
    }
}

pub unsafe fn on_proxy_unload() {
    let base = BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        patch::restore(base, &CHANGE);
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
}
