//! Journal `AI_ChokePoint.csv` pendant la génération de la carte (`journal_chokepoints=off|on`, off
//! par défaut : graphe non écrit).
//!
//! `TerrainBuilder.AnalyzeChokepoints` construit un graphe de Voronoï (`1805c9750`). Après chaque
//! étape, et à chaque tour de la boucle de fusion des cellules, il écrit **tout** le graphe dans le
//! canal de journal `0x94ee4ab3` : une ligne par cellule, formatée par `1805c8fe0`. Cela fait
//! 11,6 Mo et environ 0,35 s par génération (analyses/chargement/README.md, section 3).
//!
//! Le canal ne doit pas être rendu nul : `1805c9750` ne lance la fusion des cellules (`1805cb5e0`)
//! que si le canal existe (désassemblé : `test rdi,rdi / jz 1805c9944` avant « Merging cells »).
//! Un canal nul changerait donc les régions et les goulets, donc la carte. On coupe plutôt
//! `1805c8fe0`, qui ne fait qu'écrire : elle formate la cellule dans des chaînes locales puis appelle
//! l'écriture du canal (vtable +0x10), sans rien modifier de la cellule ni du graphe, et
//! `1805c9750` est son seul appelant. Les en-têtes (« Voronoi Graph »…) et les lignes « Choke point »
//! et « Merging » restent : quelques lignes par génération.

use crate::log;
use crate::patch::{self, Site};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// `test rdx,rdx / jz rel32 / push rbp / push r12 / push r14`. Le détour ne rappelle jamais
/// l'originale : aucune instruction recopiée, le trampoline n'est pas utilisé (le `jz` relatif ne
/// se recopierait pas).
const ECRITURE_CELLULE: Site = Site {
    name: "écriture d'une cellule du graphe de Voronoï",
    address: 0x1_805c_8fe0,
    expected: &[0x48, 0x85, 0xd2, 0x0f, 0x84, 0x3f, 0x03, 0x00, 0x00, 0x55, 0x41, 0x54, 0x41, 0x56],
    copy: (0, 0),
    resume: 0,
};

/// Journal du graphe écrit (`on`) ; off par défaut.
static JOURNAL: AtomicBool = AtomicBool::new(false);
static HOOKED_BASE: AtomicUsize = AtomicUsize::new(0);

pub fn set_journal(on: bool) {
    JOURNAL.store(on, Ordering::Relaxed);
}

/// Remplace `1805c8fe0(cellule, canal)` : rien n'est écrit.
unsafe extern "system" fn sans_ecriture(_cellule: usize, _canal: usize) {}

pub unsafe fn on_gamecore_loaded(base: usize) {
    if JOURNAL.load(Ordering::Relaxed) {
        log("journal_chokepoints : graphe de Voronoï écrit dans AI_ChokePoint.csv (réglage on)");
        return;
    }
    if let Err(e) = patch::check(base, &[ECRITURE_CELLULE]) {
        log(&format!("journal_chokepoints : version inattendue, journal laissé tel quel : {e}"));
        return;
    }
    match patch::install(base, &ECRITURE_CELLULE, sans_ecriture as *const () as usize) {
        Ok(_) => {
            HOOKED_BASE.store(base, Ordering::Relaxed);
            log("journal_chokepoints : graphe de Voronoï non écrit dans AI_ChokePoint.csv");
        }
        Err(e) => log(&format!("journal_chokepoints : échec de la pose du détour : {e}")),
    }
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet ses octets d'origine.
pub unsafe fn on_proxy_unload() {
    let base = HOOKED_BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        patch::restore(base, &ECRITURE_CELLULE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryExW(name: *const u16, file: *mut std::ffi::c_void, flags: u32) -> crate::win::HMODULE;
    }
    const DLL: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\DLC\Expansion2\Binaries\Win64\GameCore_XP2_FinalRelease.dll";

    static APPELS_CANAL: AtomicU32 = AtomicU32::new(0);

    /// Toutes les entrées de la vtable du faux canal : compte l'appel et rend 0 (« canal coupé »
    /// pour +0x38, ce qui fait sortir l'originale aussitôt).
    unsafe extern "system" fn entree(_canal: usize) -> usize {
        APPELS_CANAL.fetch_add(1, Ordering::Relaxed);
        0
    }

    /// Sur l'originale chargée sans DllMain : sans détour, `1805c8fe0` interroge le canal ; avec, elle
    /// rend la main sans y toucher.
    #[test]
    fn la_cellule_n_est_plus_ecrite() {
        if !std::path::Path::new(DLL).exists() {
            eprintln!("absent, test sauté : {DLL}");
            return;
        }
        unsafe {
            let h = LoadLibraryExW(crate::win::wide(DLL).as_ptr(), std::ptr::null_mut(), 1) as usize;
            assert_ne!(h, 0);
            patch::check(h, &[ECRITURE_CELLULE]).expect("octets attendus");
            let vtable = [entree as *const () as usize; 16];
            let canal = [vtable.as_ptr() as usize];
            let cellule = [0u32; 0x1e];
            let f: unsafe extern "system" fn(usize, usize) =
                std::mem::transmute(h + (ECRITURE_CELLULE.address - patch::IMAGE_BASE) as usize);
            f(cellule.as_ptr() as usize, canal.as_ptr() as usize);
            assert_eq!(APPELS_CANAL.swap(0, Ordering::Relaxed), 1, "l'originale interroge le canal");
            patch::install(h, &ECRITURE_CELLULE, sans_ecriture as *const () as usize).expect("détour");
            f(cellule.as_ptr() as usize, canal.as_ptr() as usize);
            assert_eq!(APPELS_CANAL.load(Ordering::Relaxed), 0, "le canal n'est plus touché");
            patch::restore(h, &ECRITURE_CELLULE);
        }
    }
}
