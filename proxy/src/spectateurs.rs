//! Spectateurs de Better Spectator Mod (BSM) : hors du tourisme et du vote du congrès. Correctif POL :
//! il change la simulation, il n'est appliqué que si tous les joueurs ont le proxy (voir `pol`).
//!
//! Un spectateur de BSM est un majeur humain vivant dont le leader est `LEADER_SPECTATOR` : il figure
//! dans toutes les listes de majeurs de GameCore (investigation : spectateurs/README.md).
//!
//! - **Tourisme.** `18027ce10` (touristes que A prend à P) divise le tourisme cumulé de A vers P par
//!   200, puis par `N`, le nombre de majeurs ayant vécu ; `18027e960` (la demande totale sur P, qui
//!   plafonne le résultat par les touristes domestiques de P) fait la même division. Chaque
//!   spectateur grossit `N`. Les deux sont réécrites à l'identique, `N` compté sans les spectateurs.
//!   Le reste n'a pas besoin de changer : sans ville ni culture, un spectateur n'a pas de touristes
//!   domestiques à céder, ni de tourisme à exercer.
//! - **Vote.** `1803c0ba0` (vote sur une résolution) et `1803c0d40` (sur une proposition ou une
//!   urgence) refusent le vote d'un spectateur, avant tout débit de faveur. Au dépouillement,
//!   `1803c5f50` donne d'office 1 voix au choix A à tout majeur vivant qui n'a pas voté : pendant
//!   l'appel, un spectateur y figure avec 1 voix et le choix 0 (ce que le dépouillement ne compte pas
//!   et que la voix d'office ne touche pas), puis voix et choix sont remis à 0, ce qui le laisse aussi
//!   hors de la rétribution de faveur qui suit (`1803bff40`).
//!
//! Les cibles « joueur » des résolutions (`WorldCongress.lua`) et la fenêtre du congrès en lecture
//! seule pour le spectateur sont dans `ui`, avec `ui_congres_cibles.lua` et `ui_congres.lua`.

use crate::patch::{self, Site};
use crate::{log, pol};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};

/// Hachage de `LEADER_SPECTATOR` (`~CRC32`, comme `DB.MakeHash`).
const LEADER_SPECTATOR: u32 = 0xf4e7_df50;

// Données et fonctions de GameCore, base 0x180000000.
const PARAMETRES: u64 = 0x1_80b8_aa70; // GlobalParameters
const CONFIG_PARTIE: u64 = 0x1_80b8_aa78; // configuration de partie
const GESTION_JOUEURS: u64 = 0x1_80b8_e140; // PlayerManager (ce que rend 180306b80)
const CONFIG_JOUEUR: u64 = 0x1_8016_4d60; // (configuration de partie, joueur) -> sa configuration
const TYPE_LEADER: u64 = 0x1_8016_a000; // (configuration du joueur) -> hachage de LEADER_TYPE
const A_RENCONTRE: u64 = 0x1_802a_6be0; // (diplomatie, joueur) -> l'a rencontré

// Décalages.
const P_CULTURE_PAR_CITOYEN: usize = 0x6d0; // TOURISM_CULTURE_PER_CITIZEN
const P_TOURISME_PAR_TOURISTE: usize = 0x6e0; // TOURISM_TOURISM_TO_MOVE_CITIZEN
const GJ_MAJEURS_VIVANTS: usize = 0xb50; // pointeurs de joueurs, [début, fin)
const GJ_ID_MAJEURS_VIVANTS: usize = 0xfa0; // ID, i32
const GJ_MAJEURS_AYANT_VECU: usize = 0x10d0; // pointeurs de joueurs
const J_ID: usize = 0xd8;
const J_DIPLOMATIE: usize = 0x6e0;
const J_CULTURE: usize = 0x6f0;
const C_JOUEUR: usize = 0x90;
const C_TOURISME_VERS: usize = 0x490; // i32 en virgule fixe 24.8, par joueur visé
const C_CULTURE_CUMULEE: usize = 0x598; // 24.8
const R_CHOIX: usize = 0x4; // i32 par joueur : 0 aucun, 1 A, 2 B
const R_VOIX: usize = 0x104; // i32 par joueur

const TOURISTES: Site = Site {
    name: "touristes pris à un joueur",
    address: 0x1_8027_ce10,
    // push rbx / push rdi / sub rsp,28h / mov rax,[rip+disp] (GlobalParameters, refait en absolu).
    expected: &[0x40, 0x53, 0x57, 0x48, 0x83, 0xec, 0x28, 0x48, 0x8b, 0x05, 0x52, 0xdc, 0x90, 0x00],
    copy: (0, 7),
    resume: 14,
};
const DEMANDE: Site = Site {
    name: "demande de touristes sur un joueur",
    address: 0x1_8027_e960,
    // push rdi / push r14 / push r15 / sub rsp,20h / mov rdi,rdx / xor r15d,r15d.
    expected: &[0x40, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xec, 0x20, 0x48, 0x8b, 0xfa, 0x45, 0x33, 0xff],
    copy: (0, 16),
    resume: 16,
};
const VOTE: Site = Site {
    name: "vote sur une résolution",
    address: 0x1_803c_0ba0,
    // mov [rsp+10h],rbx / mov [rsp+18h],rbp / mov [rsp+20h],rsi.
    expected: &[0x48, 0x89, 0x5c, 0x24, 0x10, 0x48, 0x89, 0x6c, 0x24, 0x18, 0x48, 0x89, 0x74, 0x24, 0x20],
    copy: (0, 15),
    resume: 15,
};
const VOTE_DISCUSSION: Site = Site {
    name: "vote sur une proposition",
    address: 0x1_803c_0d40,
    // push rbp / push rsi / push r15 / sub rsp,30h / movsxd rbp,edx / mov r15d,r9d.
    expected: &[0x40, 0x55, 0x56, 0x41, 0x57, 0x48, 0x83, 0xec, 0x30, 0x48, 0x63, 0xea, 0x45, 0x8b, 0xf9],
    copy: (0, 15),
    resume: 15,
};
const DEPOUILLEMENT: Site = Site {
    name: "dépouillement d'une résolution",
    address: 0x1_803c_5f50,
    // mov [rsp+8],rcx / push rbp / push rbx / push rsi / push rdi / push r12 / push r13 / push r14.
    expected: &[0x48, 0x89, 0x4c, 0x24, 0x08, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56],
    copy: (0, 15),
    resume: 15,
};
const SITES: [Site; 5] = [TOURISTES, DEMANDE, VOTE, VOTE_DISCUSSION, DEPOUILLEMENT];

static BASE: AtomicUsize = AtomicUsize::new(0);
static POSES: AtomicBool = AtomicBool::new(false);
static T_TOURISTES: AtomicUsize = AtomicUsize::new(0);
static T_DEMANDE: AtomicUsize = AtomicUsize::new(0);
static T_VOTE: AtomicUsize = AtomicUsize::new(0);
static T_DISCUSSION: AtomicUsize = AtomicUsize::new(0);
static T_DEPOUILLEMENT: AtomicUsize = AtomicUsize::new(0);
/// Dernier (N du jeu, N corrigé) journalisé, en `N << 16 | N'` ; -1 rien encore.
static DERNIER_N: AtomicI32 = AtomicI32::new(-1);
static VOTES_REFUSES: AtomicU32 = AtomicU32::new(0);
static VOIX_RETIREES: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

/// Touristes que A prend à P (`18027ce10`), entiers comme le jeu : `tourisme_vers` est le tourisme
/// cumulé de A vers P et `culture_p` la culture cumulée de P, tous deux en virgule fixe 24.8.
/// `demande` est la somme des mêmes quotients sur tous les majeurs qui visent P (`demande`).
pub fn touristes(tourisme_vers: i32, culture_p: i32, demande: i32, par_citoyen: i32, par_touriste: i32, n: i32) -> i32 {
    let domestiques = (culture_p >> 8).wrapping_div(par_citoyen);
    let t = quotient(tourisme_vers, par_touriste, n);
    if domestiques < demande {
        // Plafond : les domestiques de P, répartis au prorata (au moins 1 au dénominateur).
        let d = if demande >= 1 { demande } else { 1 };
        return (t.wrapping_mul(domestiques) as i64 / d as i64) as i32;
    }
    t
}

/// La part d'un visiteur dans la demande : `(tourisme >> 8) / par_touriste / n`, divisions entières.
pub fn quotient(tourisme_vers: i32, par_touriste: i32, n: i32) -> i32 {
    (tourisme_vers >> 8).wrapping_div(par_touriste).wrapping_div(n)
}

unsafe fn rd<T: Copy>(at: usize) -> T {
    std::ptr::read_unaligned(at as *const T)
}

unsafe fn adresse(base: usize, a: u64) -> usize {
    base + (a - patch::IMAGE_BASE) as usize
}

unsafe fn est_spectateur(base: usize, id: u32) -> bool {
    if id >= 64 {
        return false;
    }
    let config_partie: usize = rd(adresse(base, CONFIG_PARTIE));
    if config_partie == 0 {
        return false;
    }
    let config_joueur: unsafe extern "system" fn(usize, u32) -> usize = std::mem::transmute(adresse(base, CONFIG_JOUEUR));
    let type_leader: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(adresse(base, TYPE_LEADER));
    let cfg = config_joueur(config_partie, id);
    cfg != 0 && type_leader(cfg) == LEADER_SPECTATOR
}

/// Les éléments `[début, fin)` d'une liste du PlayerManager.
unsafe fn liste<T: Copy>(gj: usize, decalage: usize) -> Vec<T> {
    let debut: usize = rd(gj + decalage);
    let fin: usize = rd(gj + decalage + 8);
    if fin <= debut {
        return Vec::new();
    }
    (debut..fin).step_by(std::mem::size_of::<T>()).map(|a| rd::<T>(a)).collect()
}

/// `N` sans les spectateurs : les majeurs ayant vécu, moins ceux dont le leader est spectateur.
unsafe fn n_corrige(base: usize, gj: usize) -> i32 {
    let joueurs: Vec<usize> = liste(gj, GJ_MAJEURS_AYANT_VECU);
    let n = joueurs.len() as i32;
    let corrige = joueurs.iter().filter(|&&j| !est_spectateur(base, rd(j + J_ID))).count() as i32;
    let cle = n << 16 | corrige;
    if DERNIER_N.swap(cle, Ordering::Relaxed) != cle && corrige != n {
        log(&format!("spectateurs : tourisme divisé par {corrige} majeurs au lieu de {n} ({} spectateurs)", n - corrige));
    }
    corrige
}

/// `18027e960` réécrite avec `n` : somme, sur les majeurs vivants X ≠ P qui ont rencontré P, de la
/// part de X. Ordre de lecture du jeu, `par_touriste` relu à chaque visiteur.
unsafe fn demande(base: usize, p: usize, n: i32) -> i32 {
    let params: usize = rd(adresse(base, PARAMETRES));
    let gj: usize = rd(adresse(base, GESTION_JOUEURS));
    let a_rencontre: unsafe extern "system" fn(usize, u32) -> u8 = std::mem::transmute(adresse(base, A_RENCONTRE));
    let id_p: i32 = rd(p + J_ID);
    let mut somme = 0i32;
    for x in liste::<usize>(gj, GJ_MAJEURS_VIVANTS) {
        let par_touriste: i32 = rd(params + P_TOURISME_PAR_TOURISTE);
        let id_x: u32 = rd(x + J_ID);
        if id_x == id_p as u32 || a_rencontre(rd(p + J_DIPLOMATIE), id_x) == 0 || par_touriste < 0 {
            continue;
        }
        let vers = if (id_p as u32) < 22 { rd::<i32>(rd::<usize>(rd::<usize>(x + J_CULTURE) + C_TOURISME_VERS) + id_p as usize * 4) } else { 0 };
        somme = somme.wrapping_add(quotient(vers, par_touriste, n));
    }
    somme
}

unsafe extern "system" fn hook_touristes(culture: usize, p: usize) -> u64 {
    let base = BASE.load(Ordering::Relaxed);
    let params: usize = rd(adresse(base, PARAMETRES));
    let (par_citoyen, par_touriste): (i32, i32) = (rd(params + P_CULTURE_PAR_CITOYEN), rd(params + P_TOURISME_PAR_TOURISTE));
    if !pol::actifs() || par_citoyen <= 0 || par_touriste <= 0 {
        let original: unsafe extern "system" fn(usize, usize) -> u64 = std::mem::transmute(T_TOURISTES.load(Ordering::Relaxed));
        return original(culture, p);
    }
    let n = n_corrige(base, rd(adresse(base, GESTION_JOUEURS)));
    if n <= 0 {
        return 0;
    }
    let culture_p: i32 = rd(rd::<usize>(p + J_CULTURE) + C_CULTURE_CUMULEE);
    let dem = demande(base, p, n);
    let id_p: i32 = rd(p + J_ID);
    let vers = if (id_p as u32) < 22 {
        let joueur_a: usize = rd(culture + C_JOUEUR);
        rd::<i32>(rd::<usize>(rd::<usize>(joueur_a + J_CULTURE) + C_TOURISME_VERS) + id_p as usize * 4)
    } else {
        0
    };
    touristes(vers, culture_p, dem, par_citoyen, par_touriste, n) as u32 as u64
}

unsafe extern "system" fn hook_demande(culture: usize, p: usize) -> u64 {
    let base = BASE.load(Ordering::Relaxed);
    let params: usize = rd(adresse(base, PARAMETRES));
    // Diviseur nul : l'originale ferait une division par zéro, on lui laisse ce cas.
    if !pol::actifs() || rd::<i32>(params + P_TOURISME_PAR_TOURISTE) == 0 {
        let original: unsafe extern "system" fn(usize, usize) -> u64 = std::mem::transmute(T_DEMANDE.load(Ordering::Relaxed));
        return original(culture, p);
    }
    let n = n_corrige(base, rd(adresse(base, GESTION_JOUEURS)));
    if n < 1 {
        return 0;
    }
    demande(base, p, n) as u32 as u64
}

fn note_refus(joueur: u32, quoi: &str) {
    let k = VOTES_REFUSES.fetch_add(1, Ordering::Relaxed) + 1;
    if k <= MAX_LOGS {
        log(&format!("spectateurs : vote du spectateur {joueur} sur {quoi} refusé ({k})"));
    }
}

unsafe extern "system" fn hook_vote(congres: usize, joueur: u32, resolution: u32, voix: i32) -> u64 {
    if pol::actifs() && est_spectateur(BASE.load(Ordering::Relaxed), joueur) {
        note_refus(joueur, &format!("la résolution {resolution:08x}"));
        return 0;
    }
    let original: unsafe extern "system" fn(usize, u32, u32, i32) -> u64 = std::mem::transmute(T_VOTE.load(Ordering::Relaxed));
    original(congres, joueur, resolution, voix)
}

unsafe extern "system" fn hook_discussion(congres: usize, joueur: u32, discussion: usize, voix: i32) -> u64 {
    if pol::actifs() && est_spectateur(BASE.load(Ordering::Relaxed), joueur) {
        note_refus(joueur, "une proposition");
        return 0;
    }
    let original: unsafe extern "system" fn(usize, u32, usize, i32) -> u64 = std::mem::transmute(T_DISCUSSION.load(Ordering::Relaxed));
    original(congres, joueur, discussion, voix)
}

unsafe extern "system" fn hook_depouillement(resolution: usize) -> u64 {
    let original: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(T_DEPOUILLEMENT.load(Ordering::Relaxed));
    let base = BASE.load(Ordering::Relaxed);
    if !pol::actifs() {
        return original(resolution);
    }
    let gj: usize = rd(adresse(base, GESTION_JOUEURS));
    let spectateurs: Vec<usize> = liste::<i32>(gj, GJ_ID_MAJEURS_VIVANTS)
        .into_iter()
        .filter(|&id| (0..64).contains(&id) && est_spectateur(base, id as u32))
        .map(|id| id as usize)
        .collect();
    let ecrire = |id: usize, choix: i32, voix: i32| {
        std::ptr::write_unaligned((resolution + R_CHOIX + id * 4) as *mut i32, choix);
        std::ptr::write_unaligned((resolution + R_VOIX + id * 4) as *mut i32, voix);
    };
    for &id in &spectateurs {
        ecrire(id, 0, 1);
    }
    let r = original(resolution);
    for &id in &spectateurs {
        ecrire(id, 0, 0);
    }
    if !spectateurs.is_empty() {
        let k = VOIX_RETIREES.fetch_add(1, Ordering::Relaxed) + 1;
        if k <= MAX_LOGS {
            log(&format!("spectateurs : dépouillement sans la voix d'office des spectateurs {spectateurs:?} ({k})"));
        }
    }
    r
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    if let Err(e) = patch::check(base, &SITES) {
        log(&format!("spectateurs : version inattendue, aucun détour : {e}"));
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    // 18027ce10 lit GlobalParameters relativement à RIP : le trampoline le refait en absolu
    // (movabs rax,[adresse] : 48 A1 imm64).
    let at = base + (TOURISTES.address - patch::IMAGE_BASE) as usize;
    let mut prologue = TOURISTES.expected[TOURISTES.copy.0..TOURISTES.copy.1].to_vec();
    prologue.extend_from_slice(&[0x48, 0xa1]);
    prologue.extend_from_slice(&(adresse(base, PARAMETRES) as u64).to_le_bytes());
    let poses = [
        patch::install_at(TOURISTES.name, at, &prologue, at + TOURISTES.resume, hook_touristes as *const () as usize).map(|t| T_TOURISTES.store(t, Ordering::Relaxed)),
        patch::install(base, &DEMANDE, hook_demande as *const () as usize).map(|t| T_DEMANDE.store(t, Ordering::Relaxed)),
        patch::install(base, &VOTE, hook_vote as *const () as usize).map(|t| T_VOTE.store(t, Ordering::Relaxed)),
        patch::install(base, &VOTE_DISCUSSION, hook_discussion as *const () as usize).map(|t| T_DISCUSSION.store(t, Ordering::Relaxed)),
        patch::install(base, &DEPOUILLEMENT, hook_depouillement as *const () as usize).map(|t| T_DEPOUILLEMENT.store(t, Ordering::Relaxed)),
    ];
    match poses.iter().find_map(|r| r.as_ref().err()) {
        None => {
            POSES.store(true, Ordering::Relaxed);
            log("spectateurs : détours posés (tourisme, votes, dépouillement)");
        }
        Some(e) => log(&format!("spectateurs : échec de la pose d'un détour : {e}")),
    }
}

/// Les cinq détours sont posés : les scripts du congrès peuvent être patchés (`ui`).
pub fn poses() -> bool {
    POSES.load(Ordering::Relaxed)
}

/// Ce que rend `Civ6RolloverProxy_Pol()` aux scripts du congrès : détours posés et correctifs POL
/// actifs. GameCore et les scripts décident ainsi de la même façon.
pub fn lua_actif() -> bool {
    poses() && pol::actifs()
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet ses octets d'origine.
pub unsafe fn on_proxy_unload() {
    POSES.store(false, Ordering::Relaxed);
    let base = BASE.swap(0, Ordering::Relaxed);
    if base == 0 {
        return;
    }
    for (site, tramp) in SITES.iter().zip([&T_TOURISTES, &T_DEMANDE, &T_VOTE, &T_DISCUSSION, &T_DEPOUILLEMENT]) {
        if tramp.swap(0, Ordering::Relaxed) != 0 {
            patch::restore(base, site);
        }
    }
}

/// Nouvelle partie : les journaux recommencent.
pub fn reset() {
    DERNIER_N.store(-1, Ordering::Relaxed);
    VOTES_REFUSES.store(0, Ordering::Relaxed);
    VOIX_RETIREES.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La formule du jeu, transcrite de l'assembleur de 18027ce10 (idiv 32 bits, imul 32 bits).
    fn jeu(vers: i32, culture: i32, demande: i32, par_citoyen: i32, par_touriste: i32, n: i32) -> i32 {
        let ebp = (culture >> 8) / par_citoyen;
        let eax = ((vers >> 8) / par_touriste) / n;
        if ebp >= demande {
            return eax;
        }
        let d = if demande >= 1 { demande } else { 1 };
        eax.wrapping_mul(ebp) / d
    }

    #[test]
    fn meme_formule_que_le_jeu() {
        let mut x: u64 = 0x2545_f491_4f6c_dd1d;
        let mut alea = |m: i64| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % m as u64) as i32
        };
        for _ in 0..100_000 {
            let (vers, culture) = (alea(1 << 28) - (1 << 20), alea(1 << 28));
            let (demande, n) = (alea(5000) - 10, alea(12) + 1);
            assert_eq!(touristes(vers, culture, demande, 100, 200, n), jeu(vers, culture, demande, 100, 200, n));
        }
    }

    #[test]
    fn exemples() {
        // 12 000 de tourisme vers P, 6 majeurs : 12000 / 200 / 6 = 10 touristes.
        assert_eq!(touristes(12_000 << 8, 5_000 << 8, 10, 100, 200, 6), 10);
        // Avec 2 spectateurs en plus, le jeu divise par 8 : 7.
        assert_eq!(touristes(12_000 << 8, 5_000 << 8, 10, 100, 200, 8), 7);
        // Plafond : P n'a que 20 domestiques (2 000 de culture) pour 40 demandés.
        assert_eq!(touristes(12_000 << 8, 2_000 << 8, 40, 100, 200, 6), 5);
        // Spectateur visé : aucune culture, donc rien à prendre.
        assert_eq!(touristes(12_000 << 8, 0, 10, 100, 200, 6), 0);
    }
}
