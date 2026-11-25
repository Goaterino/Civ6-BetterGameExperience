//! Vision des cités-États reprises après une guerre : le partage suspendu par une guerre finie est
//! levé. Correctif POL : il change la simulation (visibilité, donc ce que voient les joueurs et
//! l'IA), il n'est appliqué que si tous les joueurs ont le proxy (voir `pol`). Analyse :
//! vision_cites_etats/README.md.
//!
//! Chaque instance de visibilité d'un joueur tient deux listes à compteur de joueurs (adresses du
//! GameCore_XP2 1.0.12.72, traduites pour les autres builds par `patch`) :
//! - ceux avec qui elle partage sa vision (`+0x368`), qu'alimentent la suzeraineté (`1802edf10`),
//!   les alliances, les urgences ;
//! - ceux avec qui ce partage est suspendu par une guerre (`+0x340`, entrées `{i32 joueur, i32
//!   compteur}` en `+0x350`, nombre en `+0x360`). Le partage n'ajoute la vision que si le
//!   destinataire n'y est pas (`180357b30`), et chaque déplacement d'unité la filtre de même.
//!
//! Seul l'état guerre/paix entre deux joueurs (`1801d31b0`, appelé par la déclaration de guerre
//! `1802a7100` avec `(déclarant, cible)` et par la paix `1802a7710` avec `(celui qui fait la paix,
//! l'autre)`) touche à la seconde liste, par trois appels `(p3,p2)`, `(p3,p2)`, `(p2,p3)` à la
//! guerre (`18035a4b0`, inscription) comme à la paix (`18035d8c0`, retrait qui rend la vision au
//! dernier exemplaire). Deux cas laissent une inscription après la paix :
//! - la paix faite par le camp qui n'a pas déclaré la guerre ;
//! - une guerre déclarée plusieurs fois à un joueur déjà combattu. `CanDeclareWar` (`1802a0180`)
//!   ne teste pas « déjà en guerre » pour le type suzerain (`0x5064ce64`), et la branche équipes de
//!   `1802a1f90` déclare à chaque membre : en Teamer, une cité-État déclare la guerre à chaque
//!   ennemi de son suzerain autant de fois que l'équipe a de membres, mais ne fait la paix qu'une
//!   fois au changement de suzeraineté.
//!
//! L'inscription restante bloque pour toujours le partage vers ce joueur : il ne voit plus les
//! unités de la cité-État quand il en redevient suzerain.
//!
//! Deux détours :
//! - **retrait** (`18035d8c0`) : à la paix, l'entrée est retirée en entier, quel que soit son
//!   compteur. Le retrait d'origine est appelé autant de fois que le compteur : le dernier appel
//!   retire l'entrée et rend la vision, exactement comme le jeu le fait pour une entrée à 1 ;
//! - **début de tour** (`BeginTurn`, `180049390`, sur toutes les machines au même moment) : avant
//!   le tour, chaque entrée d'un joueur vivant envers un joueur vivant avec qui il n'est pas en
//!   guerre est retirée de la même façon. C'est la réparation des parties déjà touchées ; un
//!   rechargement n'y suffirait pas, car un joueur qui revient charge seul, sans l'hôte.
//!
//! Seules les fonctions du jeu écrivent : son suivi des changements (`18072a920`) voit tout, et
//! chaque machine fait la même chose dans le même ordre. Pas de réglage joueur : seuls les builds
//! de diagnostic (feature `diag`) lisent `vision_guerre=on|off` (on par défaut).

use crate::exe::rd;
use crate::patch::{self, Site};
use crate::{log, pol};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// ------------------------------------------------------------------ sites (GameCore_XP2 1.0.12.72)

/// Retrait d'un joueur de la liste suspendue d'un autre (gestionnaire de visibilité, joueur dont
/// c'est la liste, joueur retiré) ; rend la vision quand l'entrée disparaît.
const RETRAIT: Site = Site {
    name: "war suspension removal",
    address: 0x1_8035_d8c0,
    // mov r11,rsp ; push r13 ; push r14 ; sub rsp,0xb8
    expected: &[0x4C, 0x8B, 0xDC, 0x41, 0x55, 0x41, 0x56, 0x48, 0x81, 0xEC, 0xB8, 0x00, 0x00, 0x00],
    copy: (0, 14),
    resume: 14,
};
/// `Game::Processor::BeginTurn`, sans argument ; seul appelant : `18004afc0`.
const DEBUT_TOUR: Site = Site {
    name: "BeginTurn",
    address: 0x1_8004_9390,
    // push rbp ; push r15 ; lea rbp,[rsp-0x1a8] ; sub rsp,0x2a8
    expected: &[
        0x40, 0x55, 0x41, 0x57, 0x48, 0x8D, 0xAC, 0x24, 0x58, 0xFE, 0xFF, 0xFF, 0x48, 0x81, 0xEC, 0xA8, 0x02, 0x00, 0x00,
    ],
    copy: (0, 19),
    resume: 19,
};
/// En guerre (diplomatie du jeu, joueur, joueur) -> vrai : la table de `+0x198` ne vaut pas -1.
/// Appelée, pas détournée : ses octets sont seulement vérifiés.
const EN_GUERRE: Site = Site {
    name: "game diplomacy at war",
    address: 0x1_801d_1ed0,
    // mov r9,rcx ; cmp edx,r8d ; jne +3 ; xor al,al ; ret ; mov eax,[rcx+0xb8] ; lea ecx,[rax-1]
    expected: &[0x4C, 0x8B, 0xC9, 0x41, 0x3B, 0xD0, 0x75, 0x03, 0x32, 0xC0, 0xC3, 0x8B, 0x81, 0xB8, 0x00, 0x00, 0x00, 0x8D, 0x48, 0xFF],
    copy: (0, 0),
    resume: 0,
};
const SITES: [Site; 2] = [RETRAIT, DEBUT_TOUR];

/// La diplomatie du jeu (rendue par `1801d0220`). Seuls ses 8 premiers octets sont vérifiés :
/// `push rbx ; sub rsp,0x20 ; mov ebx,[rip+…]`, le déplacement différant d'un build à l'autre.
const DIPLOMATIE: u64 = 0x1_801d_0220;
const DIPLOMATIE_DEBUT: [u8; 8] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x1D];
/// `mov rax,[rip+…] ; ret` : rend le gestionnaire de visibilité, global `GESTIONNAIRE`.
const GESTIONNAIRE_LU: u64 = 0x1_8035_b360;
const GESTIONNAIRE: u64 = 0x1_80b8_e1f8;
/// Les joueurs : `+0x20` le tableau des `Player*` par id, `+0x28` sa fin.
const JOUEURS: u64 = 0x1_80b8_e140;

/// Joueur : vivant (le test de `1801d31b0` avant de toucher aux listes).
const J_VIVANT: usize = 0x258;
/// Gestionnaire de visibilité : `+8` l'indice de l'instance de chaque joueur (-1 sans instance),
/// `+0x30` le tableau des instances.
const G_INDICES: usize = 0x8;
const G_INSTANCES: usize = 0x30;
/// Instance : liste suspendue, entrées puis nombre.
const I_SUSPENDUS: usize = 0x350;
const I_SUSPENDUS_NOMBRE: usize = 0x360;
/// Ids de joueurs des listes (`MAX_PLAYERS`).
const MAX_JOUEURS: u32 = 64;

// ------------------------------------------------------------------ état

static ENABLED: AtomicBool = AtomicBool::new(true);
static BASE: AtomicUsize = AtomicUsize::new(0);
static TRAMPS: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

fn global(address: u64) -> usize {
    BASE.load(Ordering::Relaxed) + patch::rva(address)
}

type RetraitFn = unsafe extern "system-unwind" fn(usize, u32, u32) -> u64;
type DebutTourFn = unsafe extern "system-unwind" fn();
type DiplomatieFn = unsafe extern "system-unwind" fn() -> usize;
type EnGuerreFn = unsafe extern "system-unwind" fn(usize, i32, i32) -> u64;

fn actif() -> bool {
    ENABLED.load(Ordering::Relaxed) && pol::actifs()
}

/// L'instance de visibilité du joueur, 0 sans elle.
unsafe fn instance(gestionnaire: usize, joueur: u32) -> usize {
    if gestionnaire == 0 || joueur >= MAX_JOUEURS {
        return 0;
    }
    let indices: usize = rd(gestionnaire + G_INDICES);
    let instances: usize = rd(gestionnaire + G_INSTANCES);
    if indices == 0 || instances == 0 {
        return 0;
    }
    let i: i32 = rd(indices + joueur as usize * 4);
    if i < 0 {
        return 0;
    }
    rd(instances + i as usize * 8)
}

/// Les entrées `(joueur, compteur)` de la liste suspendue de l'instance.
unsafe fn suspendus(inst: usize) -> Vec<(u32, u32)> {
    let (entrees, n): (usize, u32) = (rd(inst + I_SUSPENDUS), rd(inst + I_SUSPENDUS_NOMBRE));
    if entrees == 0 || n > MAX_JOUEURS {
        return Vec::new();
    }
    (0..n as usize).map(|k| (rd::<u32>(entrees + k * 8), rd::<u32>(entrees + k * 8 + 4))).collect()
}

/// Compteur de `retire` dans la liste suspendue de `joueur`, 0 s'il n'y est pas.
unsafe fn compteur(gestionnaire: usize, joueur: u32, retire: u32) -> u32 {
    let inst = instance(gestionnaire, joueur);
    if inst == 0 {
        return 0;
    }
    suspendus(inst).into_iter().find(|e| e.0 == retire).map_or(0, |e| e.1)
}

/// Le joueur existe et vit.
unsafe fn vivant(joueur: u32) -> bool {
    let mgr: usize = rd(global(JOUEURS));
    if mgr == 0 || joueur >= MAX_JOUEURS {
        return false;
    }
    let (debut, fin): (usize, usize) = (rd(mgr + 0x20), rd(mgr + 0x28));
    if debut + joueur as usize * 8 >= fin {
        return false;
    }
    let p: usize = rd(debut + joueur as usize * 8);
    p != 0 && rd::<u8>(p + J_VIVANT) != 0
}

/// Appels du retrait d'origine pour vider l'entrée : autant que son compteur, un au moins.
fn appels_pour_vider(compteur: u32) -> u32 {
    compteur.clamp(1, MAX_JOUEURS)
}

// ------------------------------------------------------------------ détours

unsafe extern "system-unwind" fn retrait_hook(gestionnaire: usize, joueur: u32, retire: u32) -> u64 {
    let orig: RetraitFn = std::mem::transmute(TRAMPS[0].load(Ordering::Relaxed));
    if !actif() {
        return orig(gestionnaire, joueur, retire);
    }
    let n = compteur(gestionnaire, joueur, retire);
    let mut r = orig(gestionnaire, joueur, retire);
    for _ in 1..appels_pour_vider(n) {
        r = orig(gestionnaire, joueur, retire);
    }
    if n > 1 {
        log(&format!("war suspension: peace, entry of player {retire} in player {joueur}'s list had count {n}, removed whole"));
    }
    r
}

unsafe extern "system-unwind" fn debut_tour_hook() {
    let orig: DebutTourFn = std::mem::transmute(TRAMPS[1].load(Ordering::Relaxed));
    if actif() {
        reparer();
    }
    orig();
}

/// Retire toute entrée suspendue entre deux joueurs vivants qui ne sont pas en guerre.
unsafe fn reparer() {
    let retrait: RetraitFn = std::mem::transmute(TRAMPS[0].load(Ordering::Relaxed));
    let diplomatie: DiplomatieFn = std::mem::transmute(global(DIPLOMATIE));
    let en_guerre: EnGuerreFn = std::mem::transmute(global(EN_GUERRE.address));
    let gestionnaire: usize = rd(global(GESTIONNAIRE));
    let diplo = diplomatie();
    if gestionnaire == 0 || diplo == 0 {
        return;
    }
    let mut retirees: Vec<(u32, u32, u32)> = Vec::new();
    for joueur in 0..MAX_JOUEURS {
        let inst = instance(gestionnaire, joueur);
        if inst == 0 || !vivant(joueur) {
            continue;
        }
        // Copie : chaque retrait modifie la liste.
        for (autre, n) in suspendus(inst) {
            if autre == joueur || autre >= MAX_JOUEURS || !vivant(autre) {
                continue;
            }
            if en_guerre(diplo, joueur as i32, autre as i32) & 0xff != 0 {
                continue;
            }
            for _ in 0..appels_pour_vider(n) {
                retrait(gestionnaire, joueur, autre);
            }
            retirees.push((joueur, autre, n));
        }
    }
    if !retirees.is_empty() {
        let detail: Vec<String> = retirees.iter().map(|(j, a, n)| format!("{a} in {j}'s list (x{n})")).collect();
        log(&format!("war suspension: turn start, {} stale entr(ies) between players at peace removed: {}", retirees.len(), detail.join(", ")));
    }
}

// ------------------------------------------------------------------ pose

/// Le gestionnaire de visibilité est bien le global lu par `18035b360` (`mov rax,[rip+d] ; ret`).
unsafe fn gestionnaire_attendu(base: usize) -> bool {
    let at = base + patch::rva(GESTIONNAIRE_LU);
    let code = std::slice::from_raw_parts(at as *const u8, 8);
    if code[..3] != [0x48, 0x8B, 0x05] || code[7] != 0xC3 {
        return false;
    }
    let d = i32::from_le_bytes(code[3..7].try_into().unwrap());
    (at + 7).wrapping_add(d as isize as usize) == base + patch::rva(GESTIONNAIRE)
}

unsafe fn diplomatie_attendue(base: usize) -> bool {
    std::slice::from_raw_parts((base + patch::rva(DIPLOMATIE)) as *const u8, 8) == DIPLOMATIE_DEBUT
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    if !ENABLED.load(Ordering::Relaxed) {
        log("war suspension: off (vision_guerre=off), no hooks");
        return;
    }
    if let Err(e) = patch::check(base, &SITES).and_then(|_| patch::check(base, &[EN_GUERRE])) {
        log(&format!("war suspension: unexpected game version, no hooks (POL fix unavailable): {e}"));
        return;
    }
    if !gestionnaire_attendu(base) || !diplomatie_attendue(base) {
        log("war suspension: visibility manager or game diplomacy not where expected, no hooks (POL fix unavailable)");
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    let hooks = [retrait_hook as *const () as usize, debut_tour_hook as *const () as usize];
    // Les trampolines d'abord : le début de tour appelle celui du retrait.
    for (i, s) in SITES.iter().enumerate() {
        match patch::install(base, s, hooks[i]) {
            Ok(t) => TRAMPS[i].store(t, Ordering::Relaxed),
            Err(e) => {
                log(&format!("war suspension: failed to hook {}: {e}; hooks removed", s.name));
                for s in &SITES[..i] {
                    patch::restore(base, s);
                }
                BASE.store(0, Ordering::Relaxed);
                return;
            }
        }
    }
    crate::STATUS.fetch_or(crate::ST_POL, Ordering::Relaxed);
    log("war suspension: 2 hooks installed (peace removes whole entries, stale ones repaired at turn start)");
}

pub unsafe fn on_proxy_unload() {
    let base = BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        for s in &SITES {
            patch::restore(base, s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une entrée absente donne quand même un appel (le jeu en fait toujours un) ; un compteur
    /// gonflé, autant d'appels que le compteur.
    #[test]
    fn appels_du_retrait() {
        assert_eq!(appels_pour_vider(0), 1);
        assert_eq!(appels_pour_vider(1), 1);
        assert_eq!(appels_pour_vider(3), 3);
        assert_eq!(appels_pour_vider(u32::MAX), MAX_JOUEURS, "borne contre une liste corrompue");
    }

    /// Chaque site a ses octets attendus dans le GameCore installé, quel que soit son build, et les
    /// deux accès lus à l'installation y sont (test sauté sans lui).
    #[test]
    fn sites_match_installed_gamecore() {
        let path = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\DLC\Expansion2\Binaries\Win64\GameCore_XP2_FinalRelease.dll";
        let Ok(file) = std::fs::read(path) else {
            eprintln!("GameCore missing, test skipped");
            return;
        };
        let rd32 = |o: usize| u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
        let rd16 = |o: usize| u16::from_le_bytes(file[o..o + 2].try_into().unwrap()) as usize;
        let nt = rd32(0x3c);
        let sections = nt + 24 + rd16(nt + 20);
        let offset = |addr: u64| {
            let rva = patch::rva(addr);
            (0..rd16(nt + 6))
                .map(|i| sections + 40 * i)
                .find_map(|h| {
                    let (va, size, raw) = (rd32(h + 12), rd32(h + 16), rd32(h + 20));
                    (rva >= va && rva < va + size).then(|| rva - va + raw)
                })
                .unwrap_or_else(|| panic!("{addr:#x}: not in the file"))
        };
        for s in SITES.iter().chain([&EN_GUERRE]) {
            let o = offset(s.address);
            assert_eq!(patch::as_known(s.address, &file[o..o + s.expected.len()]), s.expected, "{}", s.name);
        }
        let o = offset(DIPLOMATIE);
        assert_eq!(file[o..o + 8], DIPLOMATIE_DEBUT);
        let o = offset(GESTIONNAIRE_LU);
        assert_eq!(file[o..o + 3], [0x48, 0x8B, 0x05]);
        let d = i32::from_le_bytes(file[o + 3..o + 7].try_into().unwrap());
        assert_eq!((patch::rva(GESTIONNAIRE_LU) + 7).wrapping_add(d as isize as usize), patch::rva(GESTIONNAIRE));
    }
}
