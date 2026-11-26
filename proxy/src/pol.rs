//! État des correctifs POL (« proxy-only lobbies ») : ils changent la simulation, ils ne s'appliquent
//! que si l'export `BgdPolActifs` du proxy de démarrage rend 1 (salon où tous les humains ont le
//! proxy à la même version, ou partie locale). Proxy de démarrage absent : correctifs coupés.

use crate::{log, win};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

/// Export `BgdPolActifs` : 0 pas encore cherché, 1 absent, sinon son adresse.
static EXPORT: AtomicUsize = AtomicUsize::new(0);
/// Export `BgdPartieReseau` : 0 pas encore cherché, 1 absent, sinon son adresse.
static RESEAU: AtomicUsize = AtomicUsize::new(0);
/// Export `BgdDepartsActifs` : 0 pas encore cherché, 1 absent, sinon son adresse.
static DEPARTS: AtomicUsize = AtomicUsize::new(0);
/// Dernier état journalisé : 2 inconnu, puis 0 ou 1.
static LAST: AtomicU8 = AtomicU8::new(2);
/// Dernier état de la protection des départs journalisé : 2 inconnu, puis 0 ou 1.
static LAST_DEPARTS: AtomicU8 = AtomicU8::new(2);

/// Le proxy de démarrage, `winmm.dll` à côté de l'exe : cherché par son chemin complet, car la winmm
/// de System32 qu'il relaie porte le même nom.
pub(crate) unsafe fn startup_module() -> win::HMODULE {
    let mut buf = [0u16; 1024];
    let n = win::GetModuleFileNameW(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32) as usize;
    let exe = std::path::PathBuf::from(String::from_utf16_lossy(&buf[..n]));
    match exe.parent() {
        Some(dir) if n > 0 => win::GetModuleHandleW(win::wide(&dir.join(crate::noms::DEMARRAGE).display().to_string()).as_ptr()),
        _ => std::ptr::null_mut(),
    }
}

pub fn actifs() -> bool {
    let mut p = EXPORT.load(Ordering::Relaxed);
    if p == 0 {
        p = unsafe {
            let m = startup_module();
            let f = if m.is_null() { std::ptr::null_mut() } else { win::GetProcAddress(m, b"BgdPolActifs\0".as_ptr()) };
            if f.is_null() { 1 } else { f as usize }
        };
        if p == 1 {
            log("pol : proxy de démarrage absent, correctifs POL coupés");
        }
        EXPORT.store(p, Ordering::Relaxed);
    }
    if p == 1 {
        return false;
    }
    let f: extern "system" fn() -> u32 = unsafe { std::mem::transmute(p) };
    let on = f() != 0;
    // Lecture d'abord : appelée depuis les fils de l'IA, une écriture à chaque appel ferait
    // passer la ligne de cache d'un cœur à l'autre.
    if LAST.load(Ordering::Relaxed) != on as u8 && LAST.swap(on as u8, Ordering::Relaxed) != on as u8 {
        log(&format!("pol : correctifs POL {}", if on { "actifs" } else { "coupés" }));
    }
    on
}

/// Protection des départs (`drop_slot`, `absent_reactivation`) : avec les correctifs POL, ou quand
/// tous les humains annoncent le même `NIVEAU_DEPARTS` sans avoir la même version (export
/// `BgdDepartsActifs` du proxy de démarrage, voir startup_proxy/src/salon.lua). Proxy de démarrage
/// absent : coupée.
pub fn departs() -> bool {
    let pol = actifs();
    let on = pol || {
        let mut p = DEPARTS.load(Ordering::Relaxed);
        if p == 0 {
            p = unsafe {
                let m = startup_module();
                let f = if m.is_null() { std::ptr::null_mut() } else { win::GetProcAddress(m, b"BgdDepartsActifs\0".as_ptr()) };
                if f.is_null() { 1 } else { f as usize }
            };
            DEPARTS.store(p, Ordering::Relaxed);
        }
        p != 1 && {
            let f: extern "system" fn() -> u32 = unsafe { std::mem::transmute(p) };
            f() != 0
        }
    };
    if LAST_DEPARTS.load(Ordering::Relaxed) != on as u8 && LAST_DEPARTS.swap(on as u8, Ordering::Relaxed) != on as u8 {
        log(&format!(
            "drops: drop protection level {} {}",
            crate::version::NIVEAU_DEPARTS,
            match (on, pol) {
                (false, _) => "off",
                (true, true) => "on (POL)",
                (true, false) => "on (same drop level, POL off)",
            }
        ));
    }
    on
}

/// Partie en réseau en direct (ni solo, ni hotseat, ni PlayByCloud), selon l'export
/// `BgdPartieReseau` du proxy de démarrage, posé à chaque écran de chargement (`chargement.lua`).
/// Proxy de démarrage absent ou d'une version sans cet export : non.
pub fn partie_reseau() -> bool {
    let mut p = RESEAU.load(Ordering::Relaxed);
    if p == 0 {
        p = unsafe {
            let m = startup_module();
            let f = if m.is_null() { std::ptr::null_mut() } else { win::GetProcAddress(m, b"BgdPartieReseau\0".as_ptr()) };
            if f.is_null() { 1 } else { f as usize }
        };
        RESEAU.store(p, Ordering::Relaxed);
    }
    if p == 1 {
        return false;
    }
    let f: extern "system" fn() -> u32 = unsafe { std::mem::transmute(p) };
    f() != 0
}
