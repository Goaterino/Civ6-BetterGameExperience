//! Compteur de tours de production affiché : prise en compte des bonus plats de gouvernement.
//!
//! Les prédicteurs de `City_BuildQueue` (`1800fe8b0` unités, `1800fe420` bâtiments et merveilles,
//! `1800fe610` districts, `1800fe780` projets) oublient le facteur `1 + 0,01 × GetFlatBonus` que la
//! progression réelle applique (fascisme +50 % aux unités, autocratie +10 % aux merveilles,
//! république marchande +15 % aux districts). Analyse : production_turns/README.md.
//!
//! Les prédicteurs servent aussi à l'IA (`1803e2aa0`, `1803fa5e0`, `180552cc0`…), qui calcule ses
//! choix de la même façon chez tous les joueurs : les modifier changerait la partie et
//! désynchroniserait un joueur sans proxy. On ne touche donc qu'aux quatre appels du cache de
//! l'interface (`Cache_City_BuildQueue`), qui ne fait que lire l'état de jeu :
//! - `18006b8dc` : construction du cache, `1800fe9b0` (élément en tête de file) ;
//! - `18006c8d3` : `GetTurnsLeft()` sous verrou, `1800fe9b0` ;
//! - `18006c6ad` et `18006c7b5` : `GetTurnsLeft(hash[, formation])` sous verrou, `1800fe300`.
//!
//! Chaque `call rel32` est redirigé vers un relais posé à moins de 2 Go de GameCore, qui saute à
//! une réécriture de `1800fe9b0` ou `1800fe300`. Celle-ci appelle les prédicteurs d'origine avec
//! l'argument qu'ils offrent déjà (bonus additif ou multiplicateur, à 0 chez tous les appelants),
//! choisi pour que le taux prédit inclue le facteur :
//! - unités : bonus additif `m × F >> 8 − m`, avec `m` = modificateur + bonus du joueur, comme dans
//!   `180105850` (le calcul est alors exact) ;
//! - bâtiments : idem sur le modificateur du bâtiment, avec le bonus des merveilles ajouté au
//!   facteur pour une merveille, comme dans `1801049b0` ;
//! - districts : multiplicateur final `F` (`180104f40` multiplie aussi, plus tôt dans la chaîne) ;
//! - projets : multiplicateur `(arg + 256)` porté à `F` (`180105270`).
//!
//! `F` est en virgule fixe 24.8, converti depuis le flottant exactement comme le jeu. Sans bonus,
//! `F` vaut 256 et les arguments restent à 0 : le résultat est celui de l'original.
//!
//! GameCore est le même pour l'exe DX11 et l'exe DX12 : rien ici ne dépend de l'exe.
//!
//! Réglage `tours_production=on|off` dans rollover_proxy.cfg (on par défaut).

use crate::patch::{alloc_near, rel32, write_rel32};
use crate::log;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(true);
static BASE: AtomicUsize = AtomicUsize::new(0);
static PATCHED_BASE: AtomicUsize = AtomicUsize::new(0);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// Appels redirigés : site du `call`, fonction appelée d'origine.
const CALLS: [(u64, u64); 4] = [
    (0x1_8006_b8dc, HEAD),
    (0x1_8006_c8d3, HEAD),
    (0x1_8006_c6ad, BY_HASH),
    (0x1_8006_c7b5, BY_HASH),
];

// Fonctions de l'originale, adresses Ghidra.
const HEAD: u64 = 0x1_800f_e9b0; // (file) -> tours de l'élément en tête
const BY_HASH: u64 = 0x1_800f_e300; // (file, hash, formation) -> tours
const PREDICT_UNIT: u64 = 0x1_800f_e8b0; // (file, index, formation, &bonus)
const PREDICT_BUILDING: u64 = 0x1_800f_e420; // (file, index, &bonus, -1, -1)
const PREDICT_DISTRICT: u64 = 0x1_800f_e610; // (file, index, -1, -1, &multiplicateur, 0)
const PREDICT_PROJECT: u64 = 0x1_800f_e780; // (file, index, &bonus du multiplicateur)
const BUILDINGS: u64 = 0x1_807d_04a0; // () -> table des bâtiments (vecteur de lignes)
const UNITS: u64 = 0x1_807d_1b00;
const DISTRICTS: u64 = 0x1_807d_08e0;
const PROJECTS: u64 = 0x1_807d_13a0;
const FIND_BUILDING: u64 = 0x1_8022_09a0; // (table, hash) -> ligne ou nul ; index en +0x14
const FIND_UNIT: u64 = 0x1_8006_78d0;
const FIND_INDEXED: u64 = 0x1_801a_eec0; // districts et projets
const PLAYERS: u64 = 0x1_8030_6b80; // () -> joueurs ; +0x20 : tableau de pointeurs
const PLAYER: u64 = 0x1_8004_4f00; // (id joueur) -> joueur (bonus par unité en +0x6c8)
const BONUSES: u64 = 0x1_8026_10b0; // (joueur) -> Player_Bonuses
const FLAT_BONUS: u64 = 0x1_8026_10f0; // (bonus, hash) -> GetFlatBonus

// Hachages des bonus plats lus par la progression (noms déduits, voir l'analyse).
const OVERALL: u32 = 0xc1a0_05d2;
const UNIT: u32 = 0x3d5f_f1a2;
const WONDER: u32 = 0x5953_bf98;
const DISTRICT: u32 = 0x0112_d393;

fn gc(addr: u64) -> usize {
    BASE.load(Ordering::Relaxed) + crate::patch::rva(addr)
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read(addr as *const T)
}

unsafe fn call0(f: u64) -> usize {
    let f: unsafe extern "system" fn() -> usize = std::mem::transmute(gc(f));
    f()
}

unsafe fn call2(f: u64, a: usize, b: usize) -> usize {
    let f: unsafe extern "system" fn(usize, usize) -> usize = std::mem::transmute(gc(f));
    f(a, b)
}

/// Conversion du jeu d'un flottant en virgule fixe 24.8 : partie entière décalée, fraction
/// tronquée à 8 bits.
fn fixed(x: f32) -> i32 {
    let whole = x.floor();
    (((x - whole) * 256.0) as i32 & 0xff) | (whole as i32) << 8
}

/// Facteur appliqué par la progression : 1 + 0,01 × (somme des bonus).
fn factor(bonus: i32) -> i32 {
    fixed(bonus as f32 * 0.01 + 1.0)
}

/// `m × F >> 8`, en 64 bits puis tronqué comme le jeu.
fn scale(m: i32, f: i32) -> i32 {
    ((m as i64 * f as i64) >> 8) as i32
}

/// Bonus plats du propriétaire de la file.
struct Owner {
    id: i32,
    bonuses: usize,
}

impl Owner {
    unsafe fn of(queue: usize) -> Option<Owner> {
        let city = rd::<usize>(queue + 0x1e8);
        if city == 0 {
            return None;
        }
        let id = rd::<i32>(city + 0xd8);
        let players = call0(PLAYERS);
        if id < 0 || players == 0 {
            return None;
        }
        let player = rd::<usize>(rd::<usize>(players + 0x20) + id as usize * 8);
        if player == 0 {
            return None;
        }
        Some(Owner { id, bonuses: call2(BONUSES, player, 0) })
    }

    unsafe fn flat(&self, hash: u32) -> i32 {
        call2(FLAT_BONUS, self.bonuses, hash as usize) as i32
    }
}

/// Ligne `index` d'une table de règles (vecteur [début, fin) de pointeurs), ou 0.
unsafe fn row(table: usize, index: i32) -> usize {
    let (begin, end) = (rd::<usize>(table), rd::<usize>(table + 8));
    if index < 0 || (index as usize) >= (end - begin) / 8 {
        return 0;
    }
    rd::<usize>(begin + index as usize * 8)
}

unsafe fn unit(queue: usize, index: i32, formation: u32) -> i32 {
    let mut bonus = 0i32;
    // Index négatif : l'original rend 0 sans rien lire.
    if let (Some(o), true) = (Owner::of(queue), index >= 0) {
        let f = factor(o.flat(UNIT) + o.flat(OVERALL));
        if f != 256 {
            // Le modificateur que 180105850 multiplie : celui de la file plus le bonus du joueur.
            let per_unit = rd::<usize>(call2(PLAYER, o.id as usize, 0) + 0x6c8);
            let player_bonus = if (index as u32) < rd::<u32>(per_unit + 0x220) {
                rd::<i32>(rd::<usize>(per_unit + 0x210) + index as usize * 4)
            } else {
                0
            };
            let m = rd::<i32>(rd::<usize>(queue + 0x2c0) + index as usize * 4) + player_bonus;
            bonus = scale(m, f) - m;
        }
    }
    let p: unsafe extern "system" fn(usize, i32, u32, *mut i32) -> i32 = std::mem::transmute(gc(PREDICT_UNIT));
    p(queue, index, formation, &mut bonus)
}

unsafe fn building(queue: usize, index: i32) -> i32 {
    let mut bonus = 0i32;
    let def = row(call0(BUILDINGS), index);
    if let (Some(o), true) = (Owner::of(queue), def != 0) {
        let mut f = factor(o.flat(OVERALL));
        if rd::<u8>(def + 0x158) & 0x40 != 0 {
            // Merveille : 1801049b0 ajoute 0,01 × bonus, converti à part.
            f += fixed(o.flat(WONDER) as f32 * 0.01);
        }
        if f != 256 {
            let m = rd::<i32>(rd::<usize>(queue + 0x2a8) + index as usize * 4);
            bonus = scale(m, f) - m;
        }
    }
    let p: unsafe extern "system" fn(usize, i32, *mut i32, i32, i32) -> i32 = std::mem::transmute(gc(PREDICT_BUILDING));
    p(queue, index, &mut bonus, -1, -1)
}

unsafe fn district(queue: usize, index: i32) -> i32 {
    // 0 : pas de multiplicateur.
    let mut mult = 0i32;
    if let Some(o) = Owner::of(queue) {
        let f = factor(o.flat(DISTRICT) + o.flat(OVERALL));
        if f != 256 {
            mult = f;
        }
    }
    let p: unsafe extern "system" fn(usize, i32, i32, i32, *mut i32, usize) -> i32 = std::mem::transmute(gc(PREDICT_DISTRICT));
    p(queue, index, -1, -1, &mut mult, 0)
}

unsafe fn project(queue: usize, index: i32) -> i32 {
    // Le prédicteur multiplie par (bonus + 256).
    let mut bonus = 0i32;
    if let Some(o) = Owner::of(queue) {
        bonus = factor(o.flat(OVERALL)) - 256;
    }
    let p: unsafe extern "system" fn(usize, i32, *mut i32) -> i32 = std::mem::transmute(gc(PREDICT_PROJECT));
    p(queue, index, &mut bonus)
}

/// Réécriture de `1800fe9b0` : tours de l'élément en tête de file, -1 si la file est vide.
unsafe extern "system" fn head(queue: usize) -> i32 {
    let item = rd::<usize>(queue + 0x1d0);
    if rd::<i32>(queue + 0x1e0) == 0 || item == 0 {
        return -1;
    }
    match rd::<i32>(item + 0x30) {
        0 => unit(queue, rd::<i32>(item + 0x10), 0),
        1 => building(queue, rd::<i32>(item + 0x14)),
        2 => district(queue, rd::<i32>(item + 0x18)),
        3 => project(queue, rd::<i32>(item + 0x1c)),
        _ => -1,
    }
}

/// Réécriture de `1800fe300` : tours d'un type donné par son hash, -1 s'il est inconnu.
unsafe extern "system" fn by_hash(queue: usize, hash: u32, formation: u32) -> i32 {
    let h = hash as usize;
    let def = call2(FIND_BUILDING, call0(BUILDINGS), h);
    if def != 0 {
        return building(queue, rd::<i32>(def + 0x14));
    }
    let def = call2(FIND_UNIT, call0(UNITS), h);
    if def != 0 {
        return unit(queue, rd::<i32>(def + 0x14), formation);
    }
    let def = call2(FIND_INDEXED, call0(DISTRICTS), h);
    if def != 0 {
        return district(queue, rd::<i32>(def + 0x14));
    }
    let def = call2(FIND_INDEXED, call0(PROJECTS), h);
    if def != 0 {
        return project(queue, rd::<i32>(def + 0x14));
    }
    -1
}

/// Appelé quand l'originale vient d'être chargée, avant le premier appel relayé.
pub unsafe fn on_gamecore_loaded(base: usize) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    // Chaque site doit être un `call` vers la fonction attendue : sinon, autre version, rien n'est posé.
    for (site, callee) in CALLS {
        let at = gc(site);
        let ok = rd::<u8>(at) == 0xe8 && rel32(at, gc(callee)) == Some(rd::<i32>(at + 1));
        if !ok {
            log(&format!("tours : version inattendue en {site:#x}, aucun appel redirigé"));
            return;
        }
    }
    let Some(page) = alloc_near(base) else {
        log("tours : pas de mémoire à moins de 2 Go de GameCore, aucun appel redirigé");
        return;
    };
    // Relais : jmp [rip+0] puis l'adresse de la réécriture.
    for (i, target) in [head as *const () as usize, by_hash as *const () as usize].into_iter().enumerate() {
        let stub = page + i * 16;
        std::ptr::copy_nonoverlapping([0xffu8, 0x25, 0, 0, 0, 0].as_ptr(), stub as *mut u8, 6);
        std::ptr::write_unaligned((stub + 6) as *mut u64, target as u64);
    }
    for (site, callee) in CALLS {
        let at = gc(site);
        let stub = page + if callee == HEAD { 0 } else { 16 };
        match rel32(at, stub) {
            Some(rel) => write_rel32(at, rel),
            None => {
                log("tours : relais hors de portée, aucun appel redirigé");
                on_proxy_unload_at(base);
                return;
            }
        }
    }
    PATCHED_BASE.store(base, Ordering::Relaxed);
    log("tours : compteur de tours de production corrigé (4 appels du cache de l'interface)");
}

/// Remet les `call` d'origine (les relais restent alloués, ils ne sont plus visés).
unsafe fn on_proxy_unload_at(base: usize) {
    for (site, callee) in CALLS {
        let at = base + crate::patch::rva(site);
        if rd::<u8>(at) == 0xe8 {
            if let Some(rel) = rel32(at, base + crate::patch::rva(callee)) {
                if rd::<i32>(at + 1) != rel {
                    write_rel32(at, rel);
                }
            }
        }
    }
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet ses appels d'origine.
pub unsafe fn on_proxy_unload() {
    let base = PATCHED_BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        on_proxy_unload_at(base);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facteurs_du_jeu() {
        assert_eq!(factor(0), 256);
        assert_eq!(factor(50), 384); // fascisme, unités
        assert_eq!(factor(15), 294); // république marchande : 1,15f vaut 1,1499999, fraction tronquée
        assert_eq!(fixed(10.0 * 0.01), 25); // autocratie, ajouté au facteur d'une merveille
        assert_eq!(scale(1000, 384) - 1000, 500);
    }
}
