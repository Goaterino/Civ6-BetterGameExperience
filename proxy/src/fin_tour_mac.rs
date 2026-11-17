//! Diplomatie reçue pendant le passage de tour, sous macOS : le pendant de fin_tour.rs (voir son
//! en-tête pour le défaut et le correctif). Correctif POL.
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image :
//! - `EndTurn` (`0x6428a0`, chaîne `Game Turn Ended`) lève le drapeau de fin de tour (`0x13936e5`),
//!   `BeginTurn` (`0x642338`, `Game Turn: %i`) le baisse, `0x643808` le lit (`ldrb w0` puis `ret`) ;
//! - l'`Execute` des messages est la cinquième entrée de leur vtable, retrouvée par leur nom RTTI
//!   (`N8GameCore8Messages15DiplomacyActionE`…). `GameUpdate::Execute` (`0x84761c`) lit le drapeau
//!   avant l'étape de mise à jour (`0x42d2c0`, `Game::Processor::Update` en `0x642e9c` puis deux
//!   autres) ; les trois `Execute` de diplomatie finissent par un saut vers elle sans le lire :
//!   `DiplomacyAction` en `0x846138`, `DiplomacyDeal` en `0x8464c4`, `DiplomacySession` en
//!   `0x846a60`, chacun juste après `bl 0x42aea8` (le jeu, rendu dans `x0`) et la restauration de
//!   la pile (deux instructions pour `DiplomacySession`).
//!
//! Les trois sauts sont redirigés vers des relais (`mov w1, #quel` puis saut absolu vers
//! `etape_apres_diplomatie`), placés à moins de 128 Mo pour rester à portée d'un `b`.

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]

use super::journal::log;
use super::{patch, pol};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const ETAPE: usize = 0x42d2c0;
/// `stp x29, x30, [sp, #-0x10]!` puis `mov x29, sp`.
const ETAPE_OCTETS: [u8; 8] = [0xfd, 0x7b, 0xbf, 0xa9, 0xfd, 0x03, 0x00, 0x91];
const TOUR_FINI: usize = 0x643808;
/// `adrp x8, …` / `ldrb w0, [x8, #0x6e5]` / `ret`.
const TOUR_FINI_OCTETS: [u8; 12] = [0x88, 0x6a, 0x00, 0x90, 0x00, 0x95, 0x5b, 0x39, 0xc0, 0x03, 0x5f, 0xd6];
/// Rend le jeu (`adrp`, `add`, `ldr x0`, `ret`), appelée juste avant chaque site.
const JEU: usize = 0x42aea8;

struct Appel {
    message: &'static str,
    /// `b ETAPE`, précédé de la restauration de la pile (une ou deux instructions)…
    site: usize,
    /// … elle-même précédée de `bl JEU`, à cet écart avant le site.
    jeu: usize,
}

const APPELS: [Appel; 3] = [
    Appel { message: "DiplomacyAction", site: 0x846138, jeu: 8 },
    Appel { message: "DiplomacyDeal", site: 0x8464c4, jeu: 8 },
    Appel { message: "DiplomacySession", site: 0x846a60, jeu: 12 },
];

/// Écart entre deux relais.
const RELAIS: usize = 32;

static BASE: AtomicUsize = AtomicUsize::new(0);
static SAUTEES: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

fn gc(addr: usize) -> usize {
    BASE.load(Ordering::Relaxed) + addr
}

unsafe fn mot(addr: usize) -> u32 {
    std::ptr::read_unaligned(addr as *const u32)
}

unsafe fn octets(addr: usize, n: usize) -> &'static [u8] {
    std::slice::from_raw_parts(addr as *const u8, n)
}

/// Remplace l'étape de mise à jour à la fin de l'`Execute` du message `quel` (indice dans `APPELS`).
unsafe extern "C" fn etape_apres_diplomatie(jeu: usize, quel: u32) {
    let tour_fini: unsafe extern "C" fn() -> u8 = std::mem::transmute(gc(TOUR_FINI));
    if tour_fini() != 0 && pol::actifs() {
        let k = SAUTEES.fetch_add(1, Ordering::Relaxed) + 1;
        if k <= MAX_LOGS {
            let message = APPELS.get(quel as usize).map_or("?", |a| a.message);
            log(&format!("fin de tour : {message} exécuté après la fin du tour, le tour suivant attend le contrôle de synchro ({k})"));
        }
        return;
    }
    let etape: unsafe extern "C" fn(usize) = std::mem::transmute(gc(ETAPE));
    etape(jeu);
}

unsafe fn site_attendu(a: &Appel) -> bool {
    let at = gc(a.site);
    patch::cible_branche(at, mot(at)) == Some(gc(ETAPE))
        && mot(at) & 0x8000_0000 == 0
        && patch::cible_branche(at - a.jeu, mot(at - a.jeu)) == Some(gc(JEU))
        && mot(at - a.jeu) & 0x8000_0000 != 0
}

/// Relais `quel` : `mov w1, #quel`, puis saut absolu vers l'accroche.
fn relais(quel: u32) -> Vec<u8> {
    let mut v = patch::relais(&[0x5280_0001 | (quel << 5)], etape_apres_diplomatie as *const () as usize);
    v.resize(RELAIS, 0);
    v
}

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    BASE.store(base, Ordering::Relaxed);
    if octets(gc(TOUR_FINI), TOUR_FINI_OCTETS.len()) != TOUR_FINI_OCTETS
        || octets(gc(ETAPE), ETAPE_OCTETS.len()) != ETAPE_OCTETS
        || !APPELS.iter().all(|a| site_attendu(a))
    {
        return Err("version inattendue de GameCore, aucun appel redirigé".into());
    }
    let code: Vec<u8> = (0..APPELS.len() as u32).flat_map(relais).collect();
    let page = patch::executable_near(&code, gc(APPELS[0].site), 120 << 20)?;
    let mut sauts = Vec::with_capacity(APPELS.len());
    for (i, a) in APPELS.iter().enumerate() {
        sauts.push(patch::branche(gc(a.site), page + i * RELAIS).ok_or("relais hors de portée, aucun appel redirigé")?);
    }
    let mut methode = "";
    for (a, saut) in APPELS.iter().zip(sauts) {
        methode = patch::write_code(gc(a.site), &saut)?;
    }
    Ok(format!("mise à jour après DiplomacyAction/Deal/Session suspendue pendant le passage de tour (3 appels redirigés, {methode})"))
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
        let r = relais(2);
        assert_eq!(&r[..12], &[0x41, 0x00, 0x80, 0x52, 0x50, 0x00, 0x00, 0x58, 0x00, 0x02, 0x1f, 0xd6]);
        assert_eq!(r.len(), RELAIS);
    }

    /// Les octets vérifiés au chargement correspondent au GameCore arm64 installé, s'il est là.
    #[test]
    fn sites_du_jeu() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let Some(t) = super::super::macho::tranche_arm64(&b) else { panic!("pas de tranche arm64") };
        // Image : __TEXT en 0, au début de la tranche.
        let mot = |a: usize| u32::from_le_bytes(t[a..a + 4].try_into().unwrap());
        assert_eq!(&t[TOUR_FINI..TOUR_FINI + 12], &TOUR_FINI_OCTETS);
        assert_eq!(&t[ETAPE..ETAPE + 8], &ETAPE_OCTETS);
        for a in &APPELS {
            assert_eq!(patch::cible_branche(a.site, mot(a.site)), Some(ETAPE), "{}", a.message);
            assert_eq!(patch::cible_branche(a.site - a.jeu, mot(a.site - a.jeu)), Some(JEU), "{}", a.message);
        }
    }
}
