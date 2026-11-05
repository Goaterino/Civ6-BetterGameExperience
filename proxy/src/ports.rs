//! Téléportation vers une ville (`UNITOPERATION_TELEPORT_TO_CITY`) au départ d'un port. Correctif
//! POL : il change la simulation, il n'est appliqué que si tous les joueurs ont le proxy (voir `pol`).
//!
//! `Unit_Operations_TeleportToCity.cpp` (vtable `180a2d050`) :
//! - `18067d5c0` : l'unité a droit à l'opération (marchand, ou propriété `CAN_TELEPORT_TO_CITY`) ;
//! - `18067d290` (`CanStart`) : il lui reste des mouvements, et le district sous elle est un
//!   `DISTRICT_CITY_CENTER` (`18005e770`) qui lui appartient ;
//! - `18067d650` (destination) : une ville à elle, ou, pour une unité navale (`unité+0x2a0 == 0`,
//!   `DOMAIN_SEA`), n'importe quel district à elle (port, parc aquatique…) ;
//! - `18067d900` (exécution) : place l'unité, sans regarder l'origine.
//!
//! Une unité navale peut donc arriver dans un port, mais pas en repartir. On redirige le seul
//! `call 18005e770` de `18067d290` (en `18067d315`) vers un relais qui ajoute l'unité (`r14`) en
//! second argument : le départ est aussi accepté pour une unité navale posée sur un district,
//! quel qu'il soit (port, parc aquatique, canal…), la règle même de la destination. Le contrôle
//! de propriétaire qui suit l'appel reste en place, comme les règles de destination.
//! Investigation : analyses/teleportation/README.md.

use crate::patch::{alloc_near, rel32, write_rel32, IMAGE_BASE};
use crate::{log, pol};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// `call 18005e770` dans `18067d290`, précédé de `mov rcx,rax` et suivi de `test al,al`.
const SITE: u64 = 0x1_8067_d315;
const EST_CENTRE_VILLE: u64 = 0x1_8005_e770; // (objet du district sous l'unité) -> bool
const AVANT: [u8; 3] = [0x48, 0x8b, 0xc8];
const APRES: [u8; 2] = [0x84, 0xc0];

const U_PROPRIETAIRE: usize = 0x128;
const U_DOMAINE: usize = 0x2a0;
const DOMAIN_SEA: i32 = 0;
/// Objet que `18005e770` reçoit : identifiant du district en `+0x10`, propriétaire en `+0x1c`, et
/// la recherche d'un objet par identifiant `{propriétaire, genre 3 = district, id}` en vtable `+8`.
const O_ID: usize = 0x10;
const O_PROPRIETAIRE: usize = 0x1c;
const GENRE_DISTRICT: i16 = 3;

static BASE: AtomicUsize = AtomicUsize::new(0);
static PATCHED_BASE: AtomicUsize = AtomicUsize::new(0);
static DEPARTS: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

fn gc(addr: u64) -> usize {
    BASE.load(Ordering::Relaxed) + (addr - IMAGE_BASE) as usize
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

/// District désigné par l'objet, retrouvé comme le fait `18005e770`.
unsafe fn district(objet: usize) -> usize {
    #[repr(C)]
    struct Id {
        proprietaire: i16,
        genre: i16,
        id: u32,
    }
    let cle = Id { proprietaire: rd::<i8>(objet + O_PROPRIETAIRE) as i16, genre: GENRE_DISTRICT, id: rd::<u32>(objet + O_ID) };
    let vtable: usize = rd(objet);
    let chercher: unsafe extern "system" fn(usize, *const Id) -> usize = std::mem::transmute(rd::<usize>(vtable + 8));
    chercher(objet, &cle)
}

/// Remplace `18005e770` dans `CanStart` : centre-ville, ou tout district pour une unité navale,
/// comme à l'arrivée (`18067d650`).
unsafe extern "system" fn depart_valide(objet: usize, unite: usize) -> bool {
    let original: unsafe extern "system" fn(usize) -> bool = std::mem::transmute(gc(EST_CENTRE_VILLE));
    if original(objet) {
        return true;
    }
    if objet == 0 || unite == 0 || rd::<i32>(unite + U_DOMAINE) != DOMAIN_SEA || !pol::actifs() {
        return false;
    }
    // Pas de condition de domaine sur le district : l'arrivée n'en pose pas, le départ suit la même
    // règle (le parc aquatique, qui n'est pas `DOMAIN_SEA`, était refusé au départ).
    if district(objet) == 0 {
        return false;
    }
    let k = DEPARTS.fetch_add(1, Ordering::Relaxed) + 1;
    if k <= MAX_LOGS {
        log(&format!(
            "ports : départ accepté depuis un district hors centre-ville (joueur {}, district {:#x}) ({k})",
            rd::<i32>(unite + U_PROPRIETAIRE),
            rd::<u32>(objet + O_ID)
        ));
    }
    true
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    let at = gc(SITE);
    let ok = std::slice::from_raw_parts((at - 3) as *const u8, 3) == AVANT
        && rd::<u8>(at) == 0xe8
        && rel32(at, gc(EST_CENTRE_VILLE)) == Some(rd::<i32>(at + 1))
        && std::slice::from_raw_parts((at + 5) as *const u8, 2) == APRES;
    if !ok {
        log(&format!("ports : version inattendue en {SITE:#x}, aucun appel redirigé"));
        return;
    }
    let Some(stub) = alloc_near(base) else {
        log("ports : pas de mémoire à moins de 2 Go de GameCore, aucun appel redirigé");
        return;
    };
    // Relais : mov rdx,r14 (l'unité) puis jmp [rip+0] vers `depart_valide`. Un saut, pas un appel :
    // la pile reste celle de l'appel d'origine.
    let mut code = vec![0x4c, 0x89, 0xf2, 0xff, 0x25, 0, 0, 0, 0];
    code.extend_from_slice(&(depart_valide as *const () as usize as u64).to_le_bytes());
    std::ptr::copy_nonoverlapping(code.as_ptr(), stub as *mut u8, code.len());
    let Some(rel) = rel32(at, stub) else {
        log("ports : relais hors de portée, aucun appel redirigé");
        return;
    };
    write_rel32(at, rel);
    PATCHED_BASE.store(base, Ordering::Relaxed);
    log("ports : départ de TELEPORT_TO_CITY accepté depuis un district (appel redirigé en 0x18067d315)");
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet l'appel d'origine.
pub unsafe fn on_proxy_unload() {
    let base = PATCHED_BASE.swap(0, Ordering::Relaxed);
    if base == 0 {
        return;
    }
    let at = base + (SITE - IMAGE_BASE) as usize;
    if let Some(rel) = rel32(at, base + (EST_CENTRE_VILLE - IMAGE_BASE) as usize) {
        write_rel32(at, rel);
    }
}

/// Nouvelle partie : les journaux recommencent.
pub fn reset() {
    DEPARTS.store(0, Ordering::Relaxed);
}
