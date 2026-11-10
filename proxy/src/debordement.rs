//! Production en réserve d'une ville (débordement), montrée par l'interface (BET-21).
//!
//! GameCore garde dans la file de production (`ville + 0x10d0`) la production qui reste à verser
//! dans le prochain élément : `file + 0x1f4`, en virgule fixe 24.8 (`m_xProductionOverflow`, nom
//! tiré de la sérialisation `180106510`). Elle vient de deux chemins de `1800f5080` (ajout de la
//! production du tour) :
//! - un élément terminé : min(production du tour, production de base) − ce qui manquait à
//!   l'élément, au moins 0 ;
//! - une file vide : la production du tour s'y ajoute, puis `180266120` remet à zéro les villes
//!   à file vide sur un événement de tour, d'où un plafond d'un tour.
//! Au tour suivant, `1800f5080` ajoute la réserve à la production du tour pour l'élément en tête,
//! puis la remet à zéro (ou au nouveau débordement). Relevé en jeu le 2026-11-10 : des villes à 2 à
//! 4 éléments en file gardaient 13, 33 et 77 points ; la réserve n'est donc pas versée au choix d'un
//! élément.
//!
//! La valeur n'est pas dans le cache de l'interface (`Cache_City_BuildQueue`). On la relève quand
//! le cache est reconstruit : dans `18006b860`, le `call 1800fd160` en `18006b8d1` (rcx = la file)
//! est redirigé vers un relais qui note la réserve de la ville puis appelle l'original. Lecture
//! seule, dans le fil qui construit le cache : rien ne change dans la partie. `tours` redirige un
//! autre appel de la même fonction (`18006b8dc`), sans conflit.
//!
//! Le Lua lit le relevé par `BGD_Debordement(joueur, ville)` → points de production, ou rien si
//! la ville n'a pas encore été vue (`ui_debordement.lua`, posé par `ui`).
//!
//! Réglage `ui_debordement=on|off` dans rollover_proxy.cfg (on par défaut).

use crate::log;
use crate::patch::{alloc_near, rel32, write_rel32, IMAGE_BASE};
use crate::win;
use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

static ENABLED: AtomicBool = AtomicBool::new(true);
static BASE: AtomicUsize = AtomicUsize::new(0);
static PATCHED_BASE: AtomicUsize = AtomicUsize::new(0);

/// Réserve relevée par (joueur, ville), en virgule fixe 24.8.
static RESERVES: Mutex<Option<HashMap<(i32, i32), i32>>> = Mutex::new(None);
static TRACES: AtomicUsize = AtomicUsize::new(0);
const MAX_TRACES: usize = 20;

/// Site du `call` redirigé, et la fonction qu'il appelle.
const SITE: u64 = 0x1_8006_b8d1;
const CALLEE: u64 = 0x1_800f_d160;

// Champs de GameCore.
const FILE_VILLE: usize = 0x1e8; // file -> ville
const FILE_TAILLE: usize = 0x1e0; // nombre d'éléments
const FILE_RESERVE: usize = 0x1f4; // m_xProductionOverflow, 24.8
const VILLE_ID: usize = 0xa8; // identifiant de la ville (celui de pCity:GetID())
const VILLE_JOUEUR: usize = 0xd8; // joueur propriétaire

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// Le relevé est posé : `BGD_Debordement` rend des valeurs.
pub fn pose() -> bool {
    PATCHED_BASE.load(Ordering::Relaxed) != 0
}

fn gc(addr: u64) -> usize {
    BASE.load(Ordering::Relaxed) + (addr - IMAGE_BASE) as usize
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read(addr as *const T)
}

/// Relais : note la réserve de la ville de la file, puis appelle `1800fd160` (rcx = file).
unsafe extern "system" fn relais(queue: usize) -> usize {
    if queue != 0 {
        let city = rd::<usize>(queue + FILE_VILLE);
        if city != 0 {
            let (joueur, id, reserve) = (rd::<i32>(city + VILLE_JOUEUR), rd::<i32>(city + VILLE_ID), rd::<i32>(queue + FILE_RESERVE));
            if (0..64).contains(&joueur) {
                let avant = {
                    let mut g = RESERVES.lock().unwrap_or_else(|e| e.into_inner());
                    g.get_or_insert_with(HashMap::new).insert((joueur, id), reserve)
                };
                if avant != Some(reserve) && TRACES.fetch_add(1, Ordering::Relaxed) < MAX_TRACES {
                    log(&format!(
                        "debordement : joueur {joueur}, ville {id} : {:.2} (file de {} éléments)",
                        reserve as f64 / 256.0,
                        rd::<i32>(queue + FILE_TAILLE)
                    ));
                }
            }
        }
    }
    let f: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(gc(CALLEE));
    f(queue)
}

/// Réserve relevée, en points de production.
fn reserve(joueur: i32, ville: i32) -> Option<f64> {
    let g = RESERVES.lock().unwrap_or_else(|e| e.into_inner());
    g.as_ref()?.get(&(joueur, ville)).map(|&v| v as f64 / 256.0)
}

// ---------------------------------------------------------------------------------------------
// Lua.

type LuaCFn = unsafe extern "C-unwind" fn(*mut c_void) -> i32;

struct Hks {
    push_closure: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, i32, *const c_char, i32),
    set_field: unsafe extern "C-unwind" fn(*mut c_void, i32, *const c_char),
    cpcall: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, *mut c_void) -> i32,
    set_top: unsafe extern "C-unwind" fn(*mut c_void, i32),
    to_integer: unsafe extern "C-unwind" fn(*mut c_void, i32) -> i32,
    push_number: unsafe extern "C-unwind" fn(*mut c_void, f64),
}

static HKS: OnceLock<Option<Hks>> = OnceLock::new();

unsafe fn hks() -> Option<&'static Hks> {
    HKS.get_or_init(|| {
        let m = win::GetModuleHandleW(win::wide("HavokScript_FinalRelease.dll").as_ptr());
        if m.is_null() {
            return None;
        }
        let get = |name: &[u8]| {
            let p = win::GetProcAddress(m, name.as_ptr());
            (!p.is_null()).then_some(p)
        };
        Some(Hks {
            push_closure: std::mem::transmute(get(b"?hks_pushnamedcclosure@@YAXPEAUlua_State@@P6AH0@ZHPEBDH@Z\0")?),
            set_field: std::mem::transmute(get(b"?lua_setfield@@YAXPEAUlua_State@@HPEBD@Z\0")?),
            cpcall: std::mem::transmute(get(b"?lua_cpcall@@YAHPEAUlua_State@@P6AH0@ZPEAX@Z\0")?),
            set_top: std::mem::transmute(get(b"?lua_settop@@YAXPEAUlua_State@@H@Z\0")?),
            to_integer: std::mem::transmute(get(b"?lua_tointeger@@YAHPEAUlua_State@@H@Z\0")?),
            push_number: std::mem::transmute(get(b"?lua_pushnumber@@YAXPEAUlua_State@@N@Z\0")?),
        })
    })
    .as_ref()
}

const LUA_GLOBALSINDEX: i32 = -10002;
const NOM: &[u8] = b"BGD_Debordement\0";

unsafe extern "C-unwind" fn lua_debordement(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let (joueur, ville) = ((h.to_integer)(l, 1), (h.to_integer)(l, 2));
    match reserve(joueur, ville) {
        Some(v) => {
            (h.push_number)(l, v);
            1
        }
        None => 0,
    }
}

unsafe extern "C-unwind" fn installer(l: *mut c_void) -> i32 {
    if let Some(h) = hks() {
        (h.push_closure)(l, lua_debordement, 0, NOM.as_ptr() as *const c_char, 0);
        (h.set_field)(l, LUA_GLOBALSINDEX, NOM.as_ptr() as *const c_char);
    }
    0
}

static PUBLIE_LOG: AtomicBool = AtomicBool::new(false);

/// Pose `BGD_Debordement` dans la table globale de l'état qui compile le script. Rend vrai si
/// elle y est : sans elle, le bloc Lua ne fait rien.
pub unsafe fn publier(l: *mut c_void) -> bool {
    let r = match hks() {
        Some(h) => {
            let r = (h.cpcall)(l, installer, std::ptr::null_mut());
            if r != 0 {
                (h.set_top)(l, -2);
            }
            r
        }
        None => -1,
    };
    if r != 0 || !PUBLIE_LOG.swap(true, Ordering::Relaxed) {
        log(&if r == 0 { "debordement : BGD_Debordement posée".to_string() } else { format!("debordement : BGD_Debordement non posée ({r})") });
    }
    r == 0
}

// ---------------------------------------------------------------------------------------------
// Pose.

/// Appelé quand l'originale vient d'être chargée.
pub unsafe fn on_gamecore_loaded(base: usize) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    let at = gc(SITE);
    if rd::<u8>(at) != 0xe8 || rel32(at, gc(CALLEE)) != Some(rd::<i32>(at + 1)) {
        log(&format!("debordement : version inattendue en {SITE:#x}, réserve de production non relevée"));
        return;
    }
    let Some(page) = alloc_near(base) else {
        log("debordement : pas de mémoire à moins de 2 Go de GameCore, réserve non relevée");
        return;
    };
    // Relais : jmp [rip+0] puis l'adresse de `relais`.
    std::ptr::copy_nonoverlapping([0xffu8, 0x25, 0, 0, 0, 0].as_ptr(), page as *mut u8, 6);
    std::ptr::write_unaligned((page + 6) as *mut u64, relais as *const () as usize as u64);
    match rel32(at, page) {
        Some(rel) => write_rel32(at, rel),
        None => {
            log("debordement : relais hors de portée, réserve non relevée");
            return;
        }
    }
    PATCHED_BASE.store(base, Ordering::Relaxed);
    log("debordement : réserve de production relevée à chaque mise à jour du cache de la file (18006b8d1)");
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet l'appel d'origine.
pub unsafe fn on_proxy_unload() {
    let base = PATCHED_BASE.swap(0, Ordering::Relaxed);
    if base == 0 {
        return;
    }
    let at = base + (SITE - IMAGE_BASE) as usize;
    if let Some(rel) = rel32(at, base + (CALLEE - IMAGE_BASE) as usize) {
        if rd::<u8>(at) == 0xe8 && rd::<i32>(at + 1) != rel {
            write_rel32(at, rel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releve_en_points() {
        RESERVES.lock().unwrap().get_or_insert_with(HashMap::new).insert((0, 65536), 13 * 256 + 128);
        assert_eq!(reserve(0, 65536), Some(13.5));
        assert_eq!(reserve(0, 1), None);
    }
}
