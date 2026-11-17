//! Dérive des modificateurs de production d'unités, sous macOS : le pendant de
//! production_modifiers.rs (voir son en-tête). Correctif POL. Conversion commune avec Windows
//! (production_calcul.rs), pour un résultat identique au bit près.
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6) par la vtable de
//! `N11GameEffects7Effects20AdjustUnitProductionE` : ajout (entrée 5, `0xcbc598`) et retrait
//! (entrée 6, `0xcbc8e0`), qui appellent selon la cible :
//! - joueur : `0xcbcc30(unités, index, pourcentage)`, le retrait passant le pourcentage négé.
//!   Nombre en `+0x228`, tableau 24.8 en `+0x218` ;
//! - ville : `0xcbc754(effet, ville)` (ajout) et `0xcbcaa0` (retrait, montants négés). Boucle sur
//!   les types d'unités de l'effet (`+0x08`, nombre `+0x18` ; montants `+0x20`, nombre `+0x30`),
//!   index chez le propriétaire (`0x3bbebc(ville+0xe0)`, puis `+0x6d0`, `+0x130`), tableau en
//!   `ville+0x1398`.
//!
//! Le compilateur a mis en ligne le suivi des modifications de FAutoArchive (un arbre, pas un appel
//! comme sous Windows) : plutôt que de le recopier, chaque accroche appelle l'originale, puis ajoute
//! à chaque entrée touchée l'écart entre la conversion corrigée et celle du jeu. L'entrée est déjà
//! marquée modifiée par l'originale ; le résultat est celui de Windows.

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]

use super::journal::log;
use super::{patch, pol};
use super::production_calcul::{conversion, derive};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    /// FNV-1a 64 du corps entier : offsets et appels de cette version du jeu.
    empreinte: u64,
}

const JOUEUR: Fonction = Fonction { nom: "joueur", adresse: 0xcbcc30, taille: 276, empreinte: 0xf89d672e4a27be39 };
const VILLE_AJOUT: Fonction = Fonction { nom: "ville, ajout", adresse: 0xcbc754, taille: 396, empreinte: 0xbd41c917c4e0d4d8 };
const VILLE_RETRAIT: Fonction = Fonction { nom: "ville, retrait", adresse: 0xcbcaa0, taille: 400, empreinte: 0xb9eedbbd7e3ed5d6 };
/// Joueur par identifiant.
const PAR_ID: usize = 0x3bbebc;
/// Prologue recopié dans les trampolines : quatre `stp`, sans adresse relative.
const PROLOGUE: usize = 16;

const U_NB: usize = 0x228;
const U_TABLEAU: usize = 0x218;
const E_TYPES: usize = 0x08;
const E_NB_TYPES: usize = 0x18;
const E_MONTANTS: usize = 0x20;
const E_NB_MONTANTS: usize = 0x30;
const V_PROPRIETAIRE: usize = 0xe0;
const V_TABLEAU: usize = 0x1398;
const J_UNITES: usize = 0x6d0;
const U_INDEX: usize = 0x130;

static BASE: AtomicUsize = AtomicUsize::new(0);
static TRAMP_JOUEUR: AtomicUsize = AtomicUsize::new(0);
static TRAMP_AJOUT: AtomicUsize = AtomicUsize::new(0);
static TRAMP_RETRAIT: AtomicUsize = AtomicUsize::new(0);
static CORRECTIONS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

fn journal(quoi: &str, percent: i32) {
    let k = CORRECTIONS.fetch_add(1, Ordering::Relaxed) + 1;
    if k <= MAX_LOGS {
        log(&format!("production : {quoi} : {percent} % corrigé de {:+}/256 ({k})", ecart(percent)));
    }
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

/// Ce qu'il faut ajouter au résultat du jeu pour obtenir la conversion corrigée.
fn ecart(percent: i32) -> i32 {
    conversion(percent).wrapping_sub(super::production_calcul::conversion_jeu(percent))
}

unsafe extern "C" fn joueur(unites: usize, index: i32, percent: i32) {
    let original: unsafe extern "C" fn(usize, i32, i32) = std::mem::transmute(TRAMP_JOUEUR.load(Ordering::Relaxed));
    original(unites, index, percent);
    // Même comparaison non signée que l'originale, qui n'a rien écrit au-delà.
    if percent < 0 && derive(percent) && (index as u32) < rd::<u32>(unites + U_NB) && pol::actifs() {
        let entree = (rd::<usize>(unites + U_TABLEAU) as *mut i32).add(index as usize);
        *entree = (*entree).wrapping_add(ecart(percent));
        journal("joueur", percent);
    }
}

/// Montant de la `i`-ième entrée de l'effet, comme le lit la boucle du jeu, négé au retrait.
unsafe fn montant(effet: usize, i: u32, retire: bool) -> i32 {
    let base: usize = rd(effet + E_MONTANTS);
    let p: i32 = if i < rd::<u32>(effet + E_NB_MONTANTS) { rd(base + 4 * i as usize) } else { rd(base) };
    if retire { p.wrapping_neg() } else { p }
}

/// Après la boucle du jeu : l'écart de chaque entrée qu'elle a touchée.
unsafe fn corriger(effet: usize, ville: usize, retire: bool) {
    if rd::<u32>(effet + E_NB_MONTANTS) == 0 {
        return; // l'originale n'a rien fait
    }
    if !(0..rd::<u32>(effet + E_NB_TYPES)).any(|i| derive(montant(effet, i, retire))) || !pol::actifs() {
        return;
    }
    let par_id: unsafe extern "C" fn(i32) -> usize = std::mem::transmute(BASE.load(Ordering::Relaxed) + PAR_ID);
    let proprietaire = par_id(rd(ville + V_PROPRIETAIRE));
    let index_par_type: usize = rd(rd::<usize>(proprietaire + J_UNITES) + U_INDEX);
    let tableau: usize = rd(ville + V_TABLEAU);
    for i in 0..rd::<u32>(effet + E_NB_TYPES) {
        let t: i32 = rd(rd::<usize>(effet + E_TYPES) + 4 * i as usize);
        if t < 0 {
            continue;
        }
        let index: i32 = rd(index_par_type + 4 * t as usize);
        if index == -1 {
            continue;
        }
        let entree = (tableau as isize + 4 * index as isize) as *mut i32;
        let p = montant(effet, i, retire);
        if derive(p) {
            *entree = (*entree).wrapping_add(ecart(p));
            journal("ville", p);
        }
    }
}

unsafe extern "C" fn ville_ajout(effet: usize, ville: usize) -> u32 {
    let original: unsafe extern "C" fn(usize, usize) -> u32 = std::mem::transmute(TRAMP_AJOUT.load(Ordering::Relaxed));
    let r = original(effet, ville);
    corriger(effet, ville, false);
    r
}

unsafe extern "C" fn ville_retrait(effet: usize, ville: usize) -> u32 {
    let original: unsafe extern "C" fn(usize, usize) -> u32 = std::mem::transmute(TRAMP_RETRAIT.load(Ordering::Relaxed));
    let r = original(effet, ville);
    corriger(effet, ville, true);
    r
}

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    let fonctions = [
        (JOUEUR, &TRAMP_JOUEUR, joueur as *const () as usize),
        (VILLE_AJOUT, &TRAMP_AJOUT, ville_ajout as *const () as usize),
        (VILLE_RETRAIT, &TRAMP_RETRAIT, ville_retrait as *const () as usize),
    ];
    for (f, _, _) in &fonctions {
        if fnv(std::slice::from_raw_parts((base + f.adresse) as *const u8, f.taille)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    BASE.store(base, Ordering::Relaxed);
    let mut methode = "";
    for (f, tramp, accroche) in fonctions {
        // Le trampoline est publié avant que le saut ne soit écrit.
        methode = patch::hook_with_trampoline_publie(base + f.adresse, PROLOGUE, accroche, tramp)?;
    }
    Ok(format!("retrait symétrique des modificateurs d'unités posé, joueur et villes ({methode})"))
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(_base: usize) -> Result<String, String> {
    Err("pas encore relevé en x86_64".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecarts() {
        assert_eq!(ecart(30), 0, "positif : le jeu");
        assert_eq!(ecart(-30), 1, "-30 % : -77 devient -76");
        assert_eq!(ecart(-50), 0, "exact");
        for p in 1..=10_000 {
            assert_eq!(super::super::production_calcul::conversion_jeu(-p).wrapping_add(ecart(-p)), -super::super::production_calcul::conversion_jeu(p));
        }
    }

    #[test]
    fn fonctions_du_jeu() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::super::macho::tranche_arm64(&b).expect("tranche arm64");
        for f in [JOUEUR, VILLE_AJOUT, VILLE_RETRAIT] {
            assert_eq!(fnv(&t[f.adresse..f.adresse + f.taille]), f.empreinte, "{}", f.nom);
            // Quatre `stp` en tête (copiés tels quels dans le trampoline).
            for k in 0..4 {
                let m = u32::from_le_bytes(t[f.adresse + 4 * k..f.adresse + 4 * k + 4].try_into().unwrap());
                assert!((m >> 24) & 0xfe == 0xa8 && (m >> 22) & 1 == 0, "{} : stp attendu en +{}", f.nom, 4 * k);
            }
        }
    }
}
