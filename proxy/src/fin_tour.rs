//! Diplomatie reçue pendant le passage de tour : le tour suivant ne démarre plus avant le contrôle
//! de synchro. Correctif POL : il change la simulation, il n'est appliqué que si tous les joueurs
//! ont le proxy (voir `pol`). Analyse : depart_tour/README.md, « Diplomatie reçue pendant le
//! passage de tour ».
//!
//! `EndTurn` (`180044b20`, journal `Game Turn Ended`) lève le drapeau de fin de tour (`180b13a81`,
//! lu par `180045dc0`). Le prochain `Game::Processor::Update` (`180046200`) le trouve levé et
//! appelle aussitôt `BeginTurn` (`180044700`, journal `Game Turn: %i`), quel qu'en soit l'appelant.
//! L'attente des `GameTurnComplete` et le contrôle de synchro sont dans l'exe, qui envoie ensuite
//! `GameUpdate` avec `InitialTurnProcessing=1` pour démarrer le tour. `GameUpdate::Execute`
//! (`1800318d0`) ignore toute autre mise à jour tant que le drapeau est levé, mais les messages de
//! diplomatie lancent l'étape de mise à jour (`18060eca0`) sans le regarder :
//! - `DiplomacyAction::Execute` (`1800308b0`), sans condition ;
//! - `DiplomacyDeal::Execute` (`180030b70`), sans condition ;
//! - `DiplomacySession::Execute` (`180030fe0`), pour le type `0x1baabc8b` quand aucun joueur n'a
//!   son tour actif, ce qui est le cas après `EndTurn`.
//!
//! Reçu dans ce créneau, un tel message fait démarrer le tour chez l'hôte avant le contrôle de
//! synchro, et les `GameTurnComplete` du tour fini arrivés ensuite comptent pour le nouveau tour
//! (`GameTurnComplete::Execute`, `180031710`, ne les ignore que drapeau levé) : leurs joueurs
//! commencent le tour « tour fini » (BGE-FB5126).
//!
//! On redirige ces trois appels vers un relais qui saute l'étape de mise à jour quand le drapeau
//! est levé, comme `GameUpdate::Execute` : l'action est appliquée, et le `GameUpdate` initial qui
//! suit le contrôle de synchro lance la mise à jour. Drapeau baissé (un joueur qui a passé son tour
//! puis agit, tour encore en cours), rien ne change.

use crate::patch::{alloc_near, rel32, write_rel32, IMAGE_BASE};
use crate::{log, pol};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// Étape de mise à jour du jeu : `Game::Processor::Update` puis deux autres mises à jour.
const ETAPE: u64 = 0x1_8060_eca0;
/// `movzx eax, byte [rip+…]` (drapeau de fin de tour `180b13a81`) puis `ret`.
const TOUR_FINI: u64 = 0x1_8004_5dc0;
const TOUR_FINI_OCTETS: [u8; 8] = [0x0f, 0xb6, 0x05, 0xba, 0xdc, 0xac, 0x00, 0xc3];

struct Appel {
    message: &'static str,
    /// `jmp rel32` (appel terminal) ou `call rel32` vers `ETAPE`.
    site: u64,
    opcode: u8,
    /// Instruction qui précède le site.
    avant: &'static [u8],
}

const APPELS: [Appel; 3] = [
    // add rsp,28h / jmp 18060eca0
    Appel { message: "DiplomacyAction", site: 0x1_8003_08cf, opcode: 0xe9, avant: &[0x48, 0x83, 0xc4, 0x28] },
    // add rsp,38h / jmp 18060eca0
    Appel { message: "DiplomacyDeal", site: 0x1_8003_0b98, opcode: 0xe9, avant: &[0x48, 0x83, 0xc4, 0x38] },
    // mov rcx,rax / call 18060eca0
    Appel { message: "DiplomacySession", site: 0x1_8003_10d2, opcode: 0xe8, avant: &[0x48, 0x8b, 0xc8] },
];

/// Écart entre deux relais dans la page.
const RELAIS: usize = 32;

static BASE: AtomicUsize = AtomicUsize::new(0);
static PATCHED_BASE: AtomicUsize = AtomicUsize::new(0);
static SAUTEES: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

fn gc(addr: u64) -> usize {
    BASE.load(Ordering::Relaxed) + (addr - IMAGE_BASE) as usize
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

/// Remplace l'étape de mise à jour à la fin de l'`Execute` du message `quel` (indice dans
/// `APPELS`). `jeu` est l'argument que lui passent les trois appels.
unsafe extern "system" fn etape_apres_diplomatie(jeu: usize, quel: u32) {
    let tour_fini: unsafe extern "system" fn() -> u8 = std::mem::transmute(gc(TOUR_FINI));
    if tour_fini() != 0 && pol::actifs() {
        let k = SAUTEES.fetch_add(1, Ordering::Relaxed) + 1;
        if k <= MAX_LOGS {
            let message = APPELS.get(quel as usize).map_or("?", |a| a.message);
            log(&format!("fin de tour : {message} exécuté après la fin du tour, le tour suivant attend le contrôle de synchro ({k})"));
        }
        return;
    }
    let etape: unsafe extern "system" fn(usize) = std::mem::transmute(gc(ETAPE));
    etape(jeu);
}

unsafe fn site_attendu(a: &Appel) -> bool {
    let at = gc(a.site);
    std::slice::from_raw_parts((at - a.avant.len()) as *const u8, a.avant.len()) == a.avant
        && rd::<u8>(at) == a.opcode
        && rel32(at, gc(ETAPE)) == Some(rd::<i32>(at + 1))
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    let tour_fini = std::slice::from_raw_parts(gc(TOUR_FINI) as *const u8, TOUR_FINI_OCTETS.len());
    if tour_fini != TOUR_FINI_OCTETS || !APPELS.iter().all(|a| site_attendu(a)) {
        log("fin de tour : version inattendue de GameCore, aucun appel redirigé");
        return;
    }
    let Some(page) = alloc_near(base) else {
        log("fin de tour : pas de mémoire à moins de 2 Go de GameCore, aucun appel redirigé");
        return;
    };
    // Relais : mov edx,quel puis jmp [rip+0] vers `etape_apres_diplomatie`. Un saut : la pile est
    // celle de l'appel d'origine (ou, pour un appel terminal, celle de l'appelant de l'`Execute`).
    let mut rels = Vec::with_capacity(APPELS.len());
    for (i, a) in APPELS.iter().enumerate() {
        let relais = page + i * RELAIS;
        let mut code = vec![0xba];
        code.extend_from_slice(&(i as u32).to_le_bytes());
        code.extend_from_slice(&[0xff, 0x25, 0, 0, 0, 0]);
        code.extend_from_slice(&(etape_apres_diplomatie as *const () as usize as u64).to_le_bytes());
        debug_assert!(code.len() <= RELAIS);
        std::ptr::copy_nonoverlapping(code.as_ptr(), relais as *mut u8, code.len());
        let Some(rel) = rel32(gc(a.site), relais) else {
            log("fin de tour : relais hors de portée, aucun appel redirigé");
            return;
        };
        rels.push(rel);
    }
    for (a, rel) in APPELS.iter().zip(rels) {
        write_rel32(gc(a.site), rel);
    }
    PATCHED_BASE.store(base, Ordering::Relaxed);
    log("fin de tour : mise à jour après DiplomacyAction/Deal/Session suspendue pendant le passage de tour (3 appels redirigés)");
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet les appels d'origine.
pub unsafe fn on_proxy_unload() {
    let base = PATCHED_BASE.swap(0, Ordering::Relaxed);
    if base == 0 {
        return;
    }
    let etape = base + (ETAPE - IMAGE_BASE) as usize;
    for a in &APPELS {
        let at = base + (a.site - IMAGE_BASE) as usize;
        if let Some(rel) = rel32(at, etape) {
            write_rel32(at, rel);
        }
    }
}

/// Nouvelle partie : les journaux recommencent.
pub fn reset() {
    SAUTEES.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les octets vérifiés au chargement correspondent au GameCore installé, s'il est là.
    #[test]
    fn sites_du_jeu() {
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
        let o = fichier(TOUR_FINI).unwrap();
        assert_eq!(&b[o..o + TOUR_FINI_OCTETS.len()], &TOUR_FINI_OCTETS);
        for a in &APPELS {
            let o = fichier(a.site).unwrap();
            assert_eq!(&b[o - a.avant.len()..o], a.avant, "{}", a.message);
            assert_eq!(b[o], a.opcode, "{}", a.message);
            let rel = i32::from_le_bytes(b[o + 1..o + 5].try_into().unwrap());
            assert_eq!(a.site as i64 + 5 + rel as i64, ETAPE as i64, "{}", a.message);
        }
    }
}
