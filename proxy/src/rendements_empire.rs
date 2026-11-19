//! Rendements d'empire de l'interface rafraîchis après un gouverneur ou un personnage illustre.
//!
//! Voir empire_yields/README.md. GameCore calcule toujours les bons rendements, mais l'interface
//! lit des instantanés (`Cache_*`) que seuls certains événements refont. Changer un citoyen émet
//! `0xc2f385c0`, qui refait ceux de la ville, de la culture, de la foi, des sciences et du trésor.
//! Une promotion de gouverneur ne refait aucun de ceux-là ; un retrait ou un établissement de
//! gouverneur et un personnage illustre les refont pendant l'action, avant que ses effets ne
//! soient appliqués.
//!
//! Les détours notent le joueur local quand une de ces actions le concerne. Une fois l'action
//! appliquée, on appelle pour lui les gestionnaires de cache de `0xc2f385c0` :
//! - à la sortie de l'action la plus externe, si elle s'exécute hors d'une passe de
//!   `Game_Processor`. C'est le cas des ordres du joueur pendant son tour (promotion, personnage
//!   illustre) : aucune fin de passe ne les a suivis dans le journal du test du 2026-11-01 ;
//! - sinon à la fin de la passe (`coalesce::hook_step`), avant l'envoi des caches à l'interface.
//!
//! Les gestionnaires ne font que lire l'état de jeu et poster un instantané à l'interface. Rien
//! n'est envoyé aux
//! scripts Lua ni sur le réseau : la simulation ne change pas, avec ou sans proxy chez les autres.
//!
//! Réglage `ui_rendements=on|off` (on), qui commande aussi `ui_rendements.lua` (voir `ui`).

use crate::patch::{self, Site};
use crate::log;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

const PROMOTE: Site = Site {
    name: "promotion d'un gouverneur",
    address: 0x1_802d_f340,
    // mov [rsp+8],rbx / mov [rsp+10h],rbp / mov [rsp+18h],rsi
    expected: &[0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x6c, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18],
    copy: (0, 15),
    resume: 15,
};

const UNASSIGN: Site = Site {
    name: "retrait d'un gouverneur",
    address: 0x1_802d_f800,
    // mov [rsp+8],rbx / mov [rsp+10h],rsi / mov [rsp+18h],rdi
    expected: &[0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x48, 0x89, 0x7c, 0x24, 0x18],
    copy: (0, 15),
    resume: 15,
};

const ESTABLISH: Site = Site {
    name: "établissement d'un gouverneur",
    address: 0x1_8025_9b10,
    // mov r11,rsp / mov [r11+20h],rsi / push rbp / lea rbp,[r11-148h]
    expected: &[0x4c, 0x8b, 0xdc, 0x49, 0x89, 0x73, 0x20, 0x55, 0x49, 0x8d, 0xab, 0xb8, 0xfe, 0xff, 0xff],
    copy: (0, 15),
    resume: 15,
};

const GREAT_PERSON: Site = Site {
    name: "activation d'un personnage illustre",
    address: 0x1_8065_5560,
    // mov [rsp+18h],r8 / mov [rsp+8],rcx / push rbp / push rbx / push rsi / push rdi / push r12
    expected: &[0x4c, 0x89, 0x44, 0x24, 0x18, 0x48, 0x89, 0x4c, 0x24, 0x08, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54],
    copy: (0, 16),
    resume: 16,
};

// Fonctions de l'originale, adresses Ghidra.
const LOCAL_PLAYER: u64 = 0x1_8004_4e60; // () -> joueur local, -1 hors partie
const CITIES: u64 = 0x1_8007_2f80; // (événement, joueur) : instantané de toutes ses villes
/// Gestionnaires de `0xc2f385c0` hors villes : culture, foi, sciences, trésor. Chacun prend
/// (événement, enregistrement) et lit le joueur en +0x18 ; le trésor exige aussi une ville en
/// +0x1c (seulement comparée à -1).
const HANDLERS: [u64; 4] = [0x1_800b_42e0, 0x1_800c_0da0, 0x1_800c_4b40, 0x1_800c_6e90];
/// L'événement d'un changement de citoyen, que les gestionnaires repassent à l'interface.
const EVENT: usize = 0xc2f3_85c0;

static ENABLED: AtomicBool = AtomicBool::new(true);
static BASE: AtomicUsize = AtomicUsize::new(0);
static HOOKED_BASE: AtomicUsize = AtomicUsize::new(0);
static TRAMP_PROMOTE: AtomicUsize = AtomicUsize::new(0);
static TRAMP_UNASSIGN: AtomicUsize = AtomicUsize::new(0);
static TRAMP_ESTABLISH: AtomicUsize = AtomicUsize::new(0);
static TRAMP_GREAT_PERSON: AtomicUsize = AtomicUsize::new(0);
/// Joueurs à rafraîchir à la fin de la passe, un bit par joueur (GameCore en compte 64 au plus).
static PENDING: AtomicU64 = AtomicU64::new(0);
/// Rafraîchissements journalisés, pour ne noter que les premiers de chaque partie.
static LOGGED: AtomicU32 = AtomicU32::new(0);
/// Actions détournées en cours (un retrait s'exécute dans une assignation, par exemple).
static ACTION_DEPTH: AtomicU32 = AtomicU32::new(0);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Nouvelle partie : rien à rafraîchir, journal remis à zéro.
pub fn reset() {
    PENDING.store(0, Ordering::Relaxed);
    ACTION_DEPTH.store(0, Ordering::Relaxed);
    LOGGED.store(0, Ordering::Relaxed);
}

fn addr(f: u64) -> usize {
    BASE.load(Ordering::Relaxed) + crate::patch::rva(f)
}

/// Note le joueur s'il est le joueur local : seul son bandeau compte, et c'est le seul que les
/// actions de son tour concernent.
unsafe fn mark(player: i32, cause: &str) {
    let local: unsafe extern "system" fn() -> i32 = std::mem::transmute(addr(LOCAL_PLAYER));
    if (0..64).contains(&player) && player == local() {
        PENDING.fetch_or(1 << player, Ordering::Relaxed);
        if LOGGED.load(Ordering::Relaxed) < 10 {
            let when = if crate::coalesce::in_step() { "fin de passe" } else { "fin de l'action" };
            log(&format!("rendements d'empire : {cause} du joueur {player}, rafraîchi en {when}"));
        }
    }
}

/// Entrée dans une action détournée.
fn enter() {
    ACTION_DEPTH.fetch_add(1, Ordering::Relaxed);
}

/// Sortie d'une action détournée : à la sortie de la plus externe, hors d'une passe, on rafraîchit
/// tout de suite ; dans une passe, `after_step` le fera à sa fin.
unsafe fn leave() {
    if ACTION_DEPTH.fetch_sub(1, Ordering::Relaxed) == 1 && !crate::coalesce::in_step() {
        after_step();
    }
}

/// Joueur des gouverneurs d'un joueur (`Player_Governors`) : joueur en +0x70, son numéro en +0xd8.
unsafe fn governors_player(governors: usize) -> i32 {
    let player = *((governors + 0x70) as *const usize);
    if player == 0 { -1 } else { *((player + 0xd8) as *const i32) }
}

unsafe extern "system" fn hook_promote(governors: usize, governor: usize, promotion: usize) -> usize {
    let f: unsafe extern "system" fn(usize, usize, usize) -> usize = std::mem::transmute(TRAMP_PROMOTE.load(Ordering::Relaxed));
    enter();
    let r = f(governors, governor, promotion);
    if r as u8 != 0 {
        mark(governors_player(governors), "promotion de gouverneur");
    }
    leave();
    r
}

unsafe extern "system" fn hook_unassign(governors: usize, governor: usize, silent: usize, flag: usize) {
    let f: unsafe extern "system" fn(usize, usize, usize, usize) = std::mem::transmute(TRAMP_UNASSIGN.load(Ordering::Relaxed));
    enter();
    f(governors, governor, silent, flag);
    mark(governors_player(governors), "retrait de gouverneur");
    leave();
}

unsafe extern "system" fn hook_establish(governor: usize, established: usize) {
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(TRAMP_ESTABLISH.load(Ordering::Relaxed));
    enter();
    f(governor, established);
    // Gouverneur : joueur propriétaire en +0x18.
    if governor != 0 {
        mark(*((governor + 0x18) as *const i32), "établissement de gouverneur");
    }
    leave();
}

unsafe extern "system" fn hook_great_person(a: usize, unit: usize, c: usize, d: usize) -> usize {
    let f: unsafe extern "system" fn(usize, usize, usize, usize) -> usize =
        std::mem::transmute(TRAMP_GREAT_PERSON.load(Ordering::Relaxed));
    // Unité : joueur propriétaire en +0x128, lu avant l'appel (l'unité peut être consommée).
    let player = if unit == 0 { -1 } else { *((unit + 0x128) as *const i32) };
    enter();
    let r = f(a, unit, c, d);
    mark(player, "personnage illustre");
    leave();
    r
}

/// Enregistrement d'événement tel que le prépare `18007c670` : huit emplacements
/// (joueur, type, identifiant), puis le joueur et une ville placés en +0x18.
fn record(player: i32) -> [u8; 0x60] {
    let mut r = [0u8; 0x60];
    r[..8].copy_from_slice(&0x6447_c33du64.to_le_bytes());
    for (i, kind) in [1i16, 1, 2, 2, 3, 3, 0, 0].iter().enumerate() {
        let at = 8 + i * 8;
        r[at..at + 2].copy_from_slice(&(-1i16).to_le_bytes());
        r[at + 2..at + 4].copy_from_slice(&kind.to_le_bytes());
        r[at + 4..at + 8].copy_from_slice(&(-1i32).to_le_bytes());
    }
    r[0x18..0x1a].copy_from_slice(&(player as i16).to_le_bytes());
    r[0x1c..0x20].copy_from_slice(&0i32.to_le_bytes());
    r
}

/// Sur le thread de GameCore, une fois les actions appliquées : rafraîchit les joueurs notés.
pub unsafe fn after_step() {
    let mut pending = PENDING.swap(0, Ordering::Relaxed);
    if pending == 0 || HOOKED_BASE.load(Ordering::Relaxed) == 0 {
        return;
    }
    while pending != 0 {
        let player = pending.trailing_zeros() as i32;
        pending &= pending - 1;
        // Passe par le site, détourné ou non par `coalesce` : hors portée, il s'exécute aussitôt.
        let cities: unsafe extern "system" fn(usize, usize) = std::mem::transmute(addr(CITIES));
        cities(EVENT, player as usize);
        let rec = record(player);
        for h in HANDLERS {
            let f: unsafe extern "system" fn(usize, *const u8) = std::mem::transmute(addr(h));
            f(EVENT, rec.as_ptr());
        }
        if LOGGED.fetch_add(1, Ordering::Relaxed) < 10 {
            log(&format!("rendements d'empire : instantanés refaits pour le joueur {player}"));
        }
    }
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    if !enabled() {
        return;
    }
    let sites = [PROMOTE, UNASSIGN, ESTABLISH, GREAT_PERSON];
    if let Err(e) = patch::check(base, &sites) {
        log(&format!("rendements d'empire : version inattendue, aucun détour : {e}"));
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    let r = (|| {
        TRAMP_PROMOTE.store(patch::install(base, &PROMOTE, hook_promote as *const () as usize)?, Ordering::Relaxed);
        TRAMP_UNASSIGN.store(patch::install(base, &UNASSIGN, hook_unassign as *const () as usize)?, Ordering::Relaxed);
        TRAMP_ESTABLISH.store(patch::install(base, &ESTABLISH, hook_establish as *const () as usize)?, Ordering::Relaxed);
        TRAMP_GREAT_PERSON
            .store(patch::install(base, &GREAT_PERSON, hook_great_person as *const () as usize)?, Ordering::Relaxed);
        Ok::<(), String>(())
    })();
    HOOKED_BASE.store(base, Ordering::Relaxed);
    match r {
        Ok(()) => log("rendements d'empire : détours posés (gouverneurs, personnages illustres)"),
        Err(e) => log(&format!("rendements d'empire : échec de la pose des détours : {e}")),
    }
}

pub unsafe fn on_proxy_unload() {
    let base = HOOKED_BASE.swap(0, Ordering::Relaxed);
    if base == 0 {
        return;
    }
    for (site, tramp) in [
        (&PROMOTE, &TRAMP_PROMOTE),
        (&UNASSIGN, &TRAMP_UNASSIGN),
        (&ESTABLISH, &TRAMP_ESTABLISH),
        (&GREAT_PERSON, &TRAMP_GREAT_PERSON),
    ] {
        if tramp.load(Ordering::Relaxed) != 0 {
            patch::restore(base, site);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enregistrement() {
        let r = record(6);
        assert_eq!(i16::from_le_bytes([r[0x18], r[0x19]]), 6);
        assert_eq!(i16::from_le_bytes([r[0x1a], r[0x1b]]), 2, "type ville");
        assert_ne!(i32::from_le_bytes(r[0x1c..0x20].try_into().unwrap()), -1, "le trésor exige une ville");
        assert_eq!(i16::from_le_bytes([r[0x08], r[0x09]]), -1);
    }
}
