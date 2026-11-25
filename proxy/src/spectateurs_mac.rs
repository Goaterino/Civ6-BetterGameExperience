//! Spectateurs de Better Spectator Mod, sous macOS : le pendant de spectateurs.rs (voir son
//! en-tête). Correctif POL. Formule du tourisme commune avec Windows (spectateurs_calcul.rs) ; le
//! correctif de `WorldCongress.lua` est posé par le chargeur (startup_proxy/src/macos_lua.rs), qui
//! lit `BgdSpectateursPol` (proxy/src/macos.rs) pour décider comme GameCore.
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image :
//! - touristes pris `0x8e8c78(culture, P)` et demande `0x8e96e4(culture, P)`, lues par les deux
//!   `GlobalParameters` du tourisme (`[0x1390220]`, `+0x6d0` et `+0x6e0`, comme Windows) ;
//! - vote sur une résolution `0xc1de04(congrès, joueur, résolution, voix)` et sur une proposition
//!   `0xc1e0a0(congrès, joueur, proposition, voix)`, appelés par l'exécution des opérations
//!   `WORLD_CONGRESS_RESOLUTION_VOTE` (`0x9f2a18`) et `WORLD_CONGRESS_DISCUSSION_VOTE` (`0x9f2d2c`,
//!   la proposition étant rendue par `0xc1ef34`) de `0x9f1538`. `0xc1eb30`, appelée en `0x9f2bbc`,
//!   est `WORLD_CONGRESS_DISCUSSION_SELECTION` (proposer un sujet), que Windows laisse aux
//!   spectateurs ;
//! - dépouillement `0xc28d78(résolution)` : voix d'office aux majeurs vivants (`PlayerManager+0xfa0`)
//!   sans voix, choix en `+4`, voix en `+0x104` (comme Windows) ;
//! - PlayerManager rendu par `0x9eba84()` ; joueur : ID `+0xe0`, diplomatie `+0x6e8`, culture
//!   `+0x6f8` ; culture : joueur `+0x98`, tourisme vers `+0x498`, culture cumulée `+0x5a0` ; a
//!   rencontré `0x91fb20(diplomatie, id)` ;
//! - spectateur : configuration `0x383328([0x1390240], id)`, hachage du leader `0x38a0cc(config)`
//!   (ce qu'appelle la méthode Lua `GetLeaderTypeID`).
//!
//! Les mêmes détours de vote rendent la faveur d'un second vote (retour au vote, congres_mac.rs).
//!
//! En x86_64 (Mac Intel, Rosetta), compilation à part, mêmes fonctions et mêmes décalages de
//! structure, retrouvées par leurs constantes et leurs appelants :
//! - touristes `0x8d0600` et demande `0x8d1110` (seuls `idivl` par `+0x6d0`/`+0x6e0` des
//!   `GlobalParameters`, `[0x13dc0b8]`, avec `+0x10d0` du PlayerManager) ;
//! - vote `0xc38740` et proposition `0xc38a00` (hachage `0xab763662`, résolutions de `0x218`
//!   octets ; appelés par l'exécution des opérations `0x9ea5e0` en `0x9eb2a7` et `0x9eaf3b`) ;
//! - dépouillement `0xc44150` (seul appelant `0xc376b3`, boucle sur les éléments de `0x218`) ;
//! - PlayerManager `0x9e4ec0()` (`[0x13e00e0]`) ; a rencontré `0x90c9a0` (rend un booléen dans
//!   `al`) ; configuration `0x3743b0([0x13dc0d8], id)` et hachage du leader `0x37b370(config)`
//!   (méthode Lua `GetLeaderTypeID`, `0x7cc810`).
//! Le saut posé (`jmp [rip+0]`) prend 14 octets : les trampolines recopient les `push` et le
//! `sub rsp` du prologue (14, 17 ou 20 octets selon la fonction, aucune adresse relative).

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]
// Les relevés x86_64 ne servent, en arm64, qu'aux tests qui lisent le jeu installé.
#![cfg_attr(target_arch = "aarch64", allow(dead_code))]

use super::journal::log;
use super::spectateurs_calcul::{quotient, touristes};
use super::{patch, pol};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};

const LEADER_SPECTATOR: u32 = 0xf4e7_df50;

struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    empreinte: u64,
}

const TOURISTES: Fonction = Fonction { nom: "touristes", adresse: 0x8e8c78, taille: 256, empreinte: 0x9277f3e8e7318705 };
const DEMANDE: Fonction = Fonction { nom: "demande", adresse: 0x8e96e4, taille: 232, empreinte: 0xf60ebff3c4ea3e2c };
const VOTE: Fonction = Fonction { nom: "vote", adresse: 0xc1de04, taille: 668, empreinte: 0x913f465c58a7868a };
const DISCUSSION: Fonction = Fonction { nom: "vote sur une proposition", adresse: 0xc1e0a0, taille: 316, empreinte: 0xd825ff978aac3b63 };
const DEPOUILLEMENT: Fonction = Fonction { nom: "dépouillement", adresse: 0xc28d78, taille: 7696, empreinte: 0xa11df123f04288ce };
/// Prologue recopié dans les trampolines : quatre instructions de pile, sans adresse relative.
const PROLOGUE: usize = 16;

/// x86_64 : les mêmes fonctions, et la longueur de leur prologue recopié (instructions entières,
/// sans adresse relative, au moins les 14 octets du saut).
const X86_TOURISTES: Fonction = Fonction { nom: "touristes", adresse: 0x8d0600, taille: 0x110, empreinte: 0x578b_f926_14c4_93f6 };
const X86_DEMANDE: Fonction = Fonction { nom: "demande", adresse: 0x8d1110, taille: 0xf0, empreinte: 0xe201_5301_fdf2_b489 };
const X86_VOTE: Fonction = Fonction { nom: "vote", adresse: 0xc38740, taille: 0x2c0, empreinte: 0x9582_3621_5e7a_b40b };
const X86_DISCUSSION: Fonction = Fonction { nom: "vote sur une proposition", adresse: 0xc38a00, taille: 0x140, empreinte: 0x6493_e0aa_3da6_7c59 };
const X86_DEPOUILLEMENT: Fonction = Fonction { nom: "dépouillement", adresse: 0xc44150, taille: 0x2320, empreinte: 0x1d5b_ceb3_deac_4722 };
/// push rbp ; mov rbp, rsp ; push r15 ; push r14 ; push r13 ; push r12 ; push rbx (13 octets).
const X86_PUSH: [u8; 13] = [0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53];
/// Suite du prologue : `push rax` (touristes, demande), `sub rsp, 0x28` (vote), `sub rsp, 0x18`
/// (proposition), `sub rsp, 0x208` (dépouillement).
const X86_SUITE: [&[u8]; 5] = [&[0x50], &[0x50], &[0x48, 0x83, 0xec, 0x28], &[0x48, 0x83, 0xec, 0x18], &[0x48, 0x81, 0xec, 0x08, 0x02, 0x00, 0x00]];
/// Fonctions appelées par le proxy en x86_64 : vérifiées aussi, une autre version ne les appelle pas.
const X86_APPELEES: [Fonction; 4] = [
    Fonction { nom: "PlayerManager", adresse: 0x9e4ec0, taille: 0x10, empreinte: 0xfb90_c246_417c_5244 },
    Fonction { nom: "configuration du joueur", adresse: 0x3743b0, taille: 0x30, empreinte: 0xefc4_e548_df8d_f7be },
    Fonction { nom: "type de leader", adresse: 0x37b370, taille: 0x40, empreinte: 0x1e56_9c64_dfc5_71dd },
    Fonction { nom: "a rencontré", adresse: 0x90c9a0, taille: 0x60, empreinte: 0x809e_95b6_7252_5599 },
];

/// Adresses x86_64 (voir l'en-tête).
const X86_PARAMETRES: usize = 0x13dc0b8;
const X86_CONFIGS_JOUEURS: usize = 0x13dc0d8;
const X86_GESTION_JOUEURS: usize = 0x9e4ec0;
const X86_CONFIG_JOUEUR: usize = 0x3743b0;
const X86_TYPE_LEADER: usize = 0x37b370;
const X86_A_RENCONTRE: usize = 0x90c9a0;
const PARAMETRES: usize = if cfg!(target_arch = "x86_64") { X86_PARAMETRES } else { 0x1390220 };
const CONFIGS_JOUEURS: usize = if cfg!(target_arch = "x86_64") { X86_CONFIGS_JOUEURS } else { 0x1390240 };
const GESTION_JOUEURS: usize = if cfg!(target_arch = "x86_64") { X86_GESTION_JOUEURS } else { 0x9eba84 };
const CONFIG_JOUEUR: usize = if cfg!(target_arch = "x86_64") { X86_CONFIG_JOUEUR } else { 0x383328 };
const TYPE_LEADER: usize = if cfg!(target_arch = "x86_64") { X86_TYPE_LEADER } else { 0x38a0cc };
const A_RENCONTRE: usize = if cfg!(target_arch = "x86_64") { X86_A_RENCONTRE } else { 0x91fb20 };

const P_CULTURE_PAR_CITOYEN: usize = 0x6d0;
const P_TOURISME_PAR_TOURISTE: usize = 0x6e0;
const GJ_MAJEURS_VIVANTS: usize = 0xb50;
const GJ_ID_MAJEURS_VIVANTS: usize = 0xfa0;
const GJ_MAJEURS_AYANT_VECU: usize = 0x10d0;
const J_ID: usize = 0xe0;
const J_DIPLOMATIE: usize = 0x6e8;
const J_CULTURE: usize = 0x6f8;
const C_JOUEUR: usize = 0x98;
const C_TOURISME_VERS: usize = 0x498;
const C_CULTURE_CUMULEE: usize = 0x5a0;
const R_CHOIX: usize = 0x4;
const R_VOIX: usize = 0x104;

static BASE: AtomicUsize = AtomicUsize::new(0);
static POSES: AtomicBool = AtomicBool::new(false);
static T_TOURISTES: AtomicUsize = AtomicUsize::new(0);
static T_DEMANDE: AtomicUsize = AtomicUsize::new(0);
static T_VOTE: AtomicUsize = AtomicUsize::new(0);
static T_DISCUSSION: AtomicUsize = AtomicUsize::new(0);
static T_DEPOUILLEMENT: AtomicUsize = AtomicUsize::new(0);
static DERNIER_N: AtomicI32 = AtomicI32::new(-1);
static VOTES_REFUSES: AtomicU32 = AtomicU32::new(0);
static VOIX_RETIREES: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

fn gc(addr: usize) -> usize {
    BASE.load(Ordering::Relaxed) + addr
}

unsafe fn rd<T: Copy>(at: usize) -> T {
    std::ptr::read_unaligned(at as *const T)
}

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

unsafe fn gestion_joueurs() -> usize {
    let f: unsafe extern "C" fn() -> usize = std::mem::transmute(gc(GESTION_JOUEURS));
    f()
}

unsafe fn est_spectateur(id: u32) -> bool {
    if id >= 64 {
        return false;
    }
    let configs: usize = rd(gc(CONFIGS_JOUEURS));
    if configs == 0 {
        return false;
    }
    let config_joueur: unsafe extern "C" fn(usize, u32) -> usize = std::mem::transmute(gc(CONFIG_JOUEUR));
    let type_leader: unsafe extern "C" fn(usize) -> u32 = std::mem::transmute(gc(TYPE_LEADER));
    let cfg = config_joueur(configs, id);
    cfg != 0 && type_leader(cfg) == LEADER_SPECTATOR
}

unsafe fn liste<T: Copy>(gj: usize, decalage: usize) -> impl Iterator<Item = T> {
    let debut: usize = rd(gj + decalage);
    let fin: usize = rd(gj + decalage + 8);
    (debut..fin.max(debut)).step_by(std::mem::size_of::<T>()).map(|a| unsafe { rd::<T>(a) })
}

unsafe fn n_corrige(gj: usize) -> i32 {
    let (mut n, mut corrige) = (0i32, 0i32);
    for j in liste::<usize>(gj, GJ_MAJEURS_AYANT_VECU) {
        n += 1;
        corrige += !est_spectateur(rd(j + J_ID)) as i32;
    }
    let cle = n << 16 | corrige;
    if DERNIER_N.load(Ordering::Relaxed) != cle && DERNIER_N.swap(cle, Ordering::Relaxed) != cle && corrige != n {
        log(&format!("spectateurs : tourisme divisé par {corrige} majeurs au lieu de {n} ({} spectateurs)", n - corrige));
    }
    corrige
}

/// `0x8e96e4` réécrite avec `n` : somme, sur les majeurs vivants X ≠ P qui ont rencontré P, de la
/// part de X. Ordre de lecture du jeu, `par_touriste` relu à chaque visiteur.
unsafe fn demande(p: usize, n: i32) -> i32 {
    let params: usize = rd(gc(PARAMETRES));
    let gj = gestion_joueurs();
    let a_rencontre: unsafe extern "C" fn(usize, u32) -> u32 = std::mem::transmute(gc(A_RENCONTRE));
    let id_p: i32 = rd(p + J_ID);
    let mut somme = 0i32;
    for x in liste::<usize>(gj, GJ_MAJEURS_VIVANTS) {
        let id_x: u32 = rd(x + J_ID);
        if id_x == id_p as u32 {
            continue;
        }
        let par_touriste: i32 = rd(params + P_TOURISME_PAR_TOURISTE);
        if a_rencontre(rd(p + J_DIPLOMATIE), id_x) & 0xff == 0 || par_touriste < 0 {
            continue;
        }
        let vers = if (id_p as u32) < 22 { rd::<i32>(rd::<usize>(rd::<usize>(x + J_CULTURE) + C_TOURISME_VERS) + id_p as usize * 4) } else { 0 };
        somme = somme.wrapping_add(quotient(vers, par_touriste, n));
    }
    somme
}

unsafe extern "C" fn hook_touristes(culture: usize, p: usize) -> i32 {
    let params: usize = rd(gc(PARAMETRES));
    let (par_citoyen, par_touriste): (i32, i32) = (rd(params + P_CULTURE_PAR_CITOYEN), rd(params + P_TOURISME_PAR_TOURISTE));
    if !pol::actifs() || par_citoyen <= 0 || par_touriste <= 0 {
        let original: unsafe extern "C" fn(usize, usize) -> i32 = std::mem::transmute(T_TOURISTES.load(Ordering::Relaxed));
        return original(culture, p);
    }
    let n = n_corrige(gestion_joueurs());
    if n <= 0 {
        return 0;
    }
    let culture_p: i32 = rd(rd::<usize>(p + J_CULTURE) + C_CULTURE_CUMULEE);
    let dem = demande(p, n);
    let id_p: i32 = rd(p + J_ID);
    let vers = if (id_p as u32) < 22 {
        let joueur_a: usize = rd(culture + C_JOUEUR);
        rd::<i32>(rd::<usize>(rd::<usize>(joueur_a + J_CULTURE) + C_TOURISME_VERS) + id_p as usize * 4)
    } else {
        0
    };
    touristes(vers, culture_p, dem, par_citoyen, par_touriste, n)
}

unsafe extern "C" fn hook_demande(culture: usize, p: usize) -> i32 {
    let params: usize = rd(gc(PARAMETRES));
    if !pol::actifs() || rd::<i32>(params + P_TOURISME_PAR_TOURISTE) == 0 {
        let original: unsafe extern "C" fn(usize, usize) -> i32 = std::mem::transmute(T_DEMANDE.load(Ordering::Relaxed));
        return original(culture, p);
    }
    let n = n_corrige(gestion_joueurs());
    if n < 1 {
        return 0;
    }
    demande(p, n)
}

fn note_refus(joueur: u32, quoi: &str) {
    let k = VOTES_REFUSES.fetch_add(1, Ordering::Relaxed) + 1;
    if k <= MAX_LOGS {
        log(&format!("spectateurs : vote du spectateur {joueur} sur {quoi} refusé ({k})"));
    }
}

unsafe extern "C" fn hook_vote(congres: usize, joueur: u32, resolution: u32, voix: i32) -> u32 {
    if pol::actifs() && est_spectateur(joueur) {
        note_refus(joueur, &format!("la résolution {resolution:08x}"));
        return 0;
    }
    let original: unsafe extern "C" fn(usize, u32, u32, i32) -> u32 = std::mem::transmute(T_VOTE.load(Ordering::Relaxed));
    let rendu = super::congres::avant_vote(congres, joueur, resolution);
    let r = original(congres, joueur, resolution, voix);
    if let (0, Some(rendu)) = (r & 1, rendu) {
        super::congres::reprendre(rendu);
    }
    r
}

unsafe extern "C" fn hook_discussion(congres: usize, joueur: u32, discussion: usize, voix: i32) -> u32 {
    if pol::actifs() && est_spectateur(joueur) {
        note_refus(joueur, "une proposition");
        return 0;
    }
    let original: unsafe extern "C" fn(usize, u32, usize, i32) -> u32 = std::mem::transmute(T_DISCUSSION.load(Ordering::Relaxed));
    let rendu = super::congres::avant_discussion(congres, joueur, discussion);
    let r = original(congres, joueur, discussion, voix);
    if let (0, Some(rendu)) = (r & 1, rendu) {
        super::congres::reprendre(rendu);
    }
    r
}

unsafe extern "C" fn hook_depouillement(resolution: usize) -> usize {
    let original: unsafe extern "C" fn(usize) -> usize = std::mem::transmute(T_DEPOUILLEMENT.load(Ordering::Relaxed));
    if !pol::actifs() {
        return original(resolution);
    }
    let spectateurs: Vec<usize> = liste::<i32>(gestion_joueurs(), GJ_ID_MAJEURS_VIVANTS)
        .filter(|&id| (0..64).contains(&id) && est_spectateur(id as u32))
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

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    let poses: [(Fonction, &AtomicUsize, usize); 5] = [
        (TOURISTES, &T_TOURISTES, hook_touristes as *const () as usize),
        (DEMANDE, &T_DEMANDE, hook_demande as *const () as usize),
        (VOTE, &T_VOTE, hook_vote as *const () as usize),
        (DISCUSSION, &T_DISCUSSION, hook_discussion as *const () as usize),
        (DEPOUILLEMENT, &T_DEPOUILLEMENT, hook_depouillement as *const () as usize),
    ];
    for (f, _, _) in &poses {
        if fnv(std::slice::from_raw_parts((base + f.adresse) as *const u8, f.taille)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    BASE.store(base, Ordering::Relaxed);
    let mut methode = "";
    for (f, tramp, accroche) in poses {
        methode = patch::hook_with_trampoline_publie(base + f.adresse, PROLOGUE, accroche, tramp)?;
    }
    POSES.store(true, Ordering::Relaxed);
    Ok(format!("détours posés (tourisme, votes, dépouillement, {methode})"))
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    let poses: [(Fonction, &AtomicUsize, usize); 5] = [
        (X86_TOURISTES, &T_TOURISTES, hook_touristes as *const () as usize),
        (X86_DEMANDE, &T_DEMANDE, hook_demande as *const () as usize),
        (X86_VOTE, &T_VOTE, hook_vote as *const () as usize),
        (X86_DISCUSSION, &T_DISCUSSION, hook_discussion as *const () as usize),
        (X86_DEPOUILLEMENT, &T_DEPOUILLEMENT, hook_depouillement as *const () as usize),
    ];
    let octets = |f: &Fonction| std::slice::from_raw_parts((base + f.adresse) as *const u8, f.taille);
    for f in poses.iter().map(|(f, _, _)| f).chain(X86_APPELEES.iter()) {
        if fnv(octets(f)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    for ((f, _, _), suite) in poses.iter().zip(X86_SUITE) {
        let o = octets(f);
        if o[..13] != X86_PUSH || &o[13..13 + suite.len()] != suite {
            return Err(format!("prologue inattendu ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    BASE.store(base, Ordering::Relaxed);
    let mut methode = "";
    for ((f, tramp, accroche), suite) in poses.into_iter().zip(X86_SUITE) {
        methode = patch::hook_with_trampoline_publie(base + f.adresse, 13 + suite.len(), accroche, tramp)?;
    }
    POSES.store(true, Ordering::Relaxed);
    Ok(format!("détours posés (tourisme, votes, dépouillement, {methode})"))
}

/// Détours posés (l'état POL se décide ensuite, en partie).
pub fn poses() -> bool {
    POSES.load(Ordering::Relaxed)
}

/// Ce que rend `Civ6RolloverProxy_Pol()` aux scripts du congrès : détours posés et correctifs POL
/// actifs, pour que GameCore et les scripts décident de la même façon.
pub fn lua_actif() -> bool {
    POSES.load(Ordering::Relaxed) && pol::actifs()
}

/// Tranche x86_64 d'un binaire universel (macho.rs n'a que `tranche_arm64`), pour les tests de
/// spectateurs_mac.rs, congres_mac.rs et citoyens_mac.rs.
#[cfg(test)]
pub(crate) fn tranche_x86_64(b: &[u8]) -> Option<&[u8]> {
    const CPU_X86_64: u32 = 0x0100_0007;
    let be = |o: usize| b.get(o..o + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap()));
    let le = |o: usize| b.get(o..o + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()));
    if le(0)? == 0xfeed_facf {
        return (le(4)? == CPU_X86_64).then_some(b);
    }
    if be(0)? != 0xcafe_babe {
        return None;
    }
    (0..be(4)? as usize).find_map(|i| {
        let e = 8 + 20 * i;
        (be(e)? == CPU_X86_64).then(|| b.get(be(e + 8)? as usize..(be(e + 8)? + be(e + 12)?) as usize)).flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonctions_du_jeu() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::super::macho::tranche_arm64(&b).expect("tranche arm64");
        for f in [TOURISTES, DEMANDE, VOTE, DISCUSSION, DEPOUILLEMENT] {
            assert_eq!(fnv(&t[f.adresse..f.adresse + f.taille]), f.empreinte, "{} : empreinte {:#x}", f.nom, fnv(&t[f.adresse..f.adresse + f.taille]));
            // Prologue sans adresse relative : sub sp / stp / add x29 (pas d'adrp, b, bl, ldr littéral).
            for k in 0..4 {
                let m = u32::from_le_bytes(t[f.adresse + 4 * k..f.adresse + 4 * k + 4].try_into().unwrap());
                let relatif = m & 0x9f00_0000 == 0x9000_0000 || m & 0x9f00_0000 == 0x1000_0000 || m & 0x7c00_0000 == 0x1400_0000 || m & 0x3b00_0000 == 0x1800_0000;
                assert!(!relatif, "{} : instruction relative en +{}", f.nom, 4 * k);
            }
        }
    }

    /// Cible d'un `lea`/`mov reg, [rip + disp32]` dont le disp32 est en `at` (instruction finie juste après).
    fn rip(t: &[u8], at: usize) -> usize {
        (at as i64 + 4 + i32::from_le_bytes(t[at..at + 4].try_into().unwrap()) as i64) as usize
    }

    #[test]
    fn fonctions_du_jeu_x86_64() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::tranche_x86_64(&b).expect("tranche x86_64");
        let poses = [X86_TOURISTES, X86_DEMANDE, X86_VOTE, X86_DISCUSSION, X86_DEPOUILLEMENT];
        for f in poses.iter().chain(X86_APPELEES.iter()) {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
        }
        for (f, suite) in poses.iter().zip(X86_SUITE) {
            assert_eq!(t[f.adresse..f.adresse + 13], X86_PUSH, "{} : push", f.nom);
            assert_eq!(&t[f.adresse + 13..f.adresse + 13 + suite.len()], suite, "{} : suite du prologue", f.nom);
            // `jmp [rip+0]` et son adresse : 14 octets (patch_mac.rs, `jump` en x86_64).
            assert!(13 + suite.len() >= 14, "{} : prologue plus court que le saut", f.nom);
        }
        let appel = |a: usize| patch::cible_rel32_x86(a, &t[a..a + 5]);
        // Données lues par le proxy, relevées dans le code du jeu : `lea r15, [rip + …]` des
        // touristes, `lea rax, [rip + …]` du coût des voix (configurations).
        assert_eq!(rip(&t, 0x8d0611), X86_PARAMETRES);
        assert_eq!(rip(&t, 0xaea28c), X86_CONFIGS_JOUEURS);
        assert_eq!(appel(0x8d063b), Some(X86_GESTION_JOUEURS));
        assert_eq!(appel(0x8d11b1), Some(X86_A_RENCONTRE));
        assert_eq!(appel(0x8d067a), Some(X86_DEMANDE.adresse));
        // GetLeaderTypeID (Lua) : configuration du joueur, puis hachage du leader.
        assert_eq!(appel(0x7cc82b), Some(X86_TYPE_LEADER));
        // Appels des votes par l'exécution des opérations, du dépouillement par le congrès.
        assert_eq!(appel(0x9eb2a7), Some(X86_VOTE.adresse));
        assert_eq!(appel(0x9eaf3b), Some(X86_DISCUSSION.adresse));
        assert_eq!(appel(0xc376b3), Some(X86_DEPOUILLEMENT.adresse));
        // Décalages relus : +0x6d0 / +0x6e0 des paramètres, +0x10d0 / +0xb50 / +0xfa0 du
        // PlayerManager, choix +4 et voix +0x104 de la résolution.
        let contient = |a: usize, n: usize, motif: &[u8]| t[a..a + n].windows(motif.len()).any(|w| w == motif);
        assert!(contient(0x8d0600, 0x110, &[0x83, 0xb8, 0xd0, 0x06, 0, 0, 0]), "cmp [rax+0x6d0], 0");
        assert!(contient(0x8d0600, 0x110, &[0x48, 0x2b, 0x88, 0xd0, 0x10, 0, 0]), "sub rcx, [rax+0x10d0]");
        assert!(contient(0x8d1110, 0xf0, &[0x4c, 0x8b, 0xa0, 0x50, 0x0b, 0, 0]), "mov r12, [rax+0xb50]");
        assert!(contient(0xc44150, 0x100, &[0x48, 0x8b, 0x98, 0xa0, 0x0f, 0, 0]), "mov rbx, [rax+0xfa0]");
        assert!(contient(0xc44150, 0x100, &[0x41, 0xc7, 0x44, 0xb7, 0x04, 1, 0, 0, 0]), "mov [r15+rsi*4+4], 1");
    }
}
