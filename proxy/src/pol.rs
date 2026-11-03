//! État des correctifs POL (« proxy-only lobbies ») : ils changent la simulation, ils ne s'appliquent
//! que si l'export `BgdPolActifs` du proxy de démarrage rend 1 (salon où tous les humains ont le
//! proxy à la même version, ou partie locale). Proxy de démarrage absent : correctifs coupés.

use crate::{log, win};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

/// Export `BgdPolActifs` : 0 pas encore cherché, 1 absent, sinon son adresse.
static EXPORT: AtomicUsize = AtomicUsize::new(0);
/// Dernier état journalisé : 2 inconnu, puis 0 ou 1.
static LAST: AtomicU8 = AtomicU8::new(2);

pub fn actifs() -> bool {
    let mut p = EXPORT.load(Ordering::Relaxed);
    if p == 0 {
        p = unsafe {
            let m = win::GetModuleHandleW(win::wide("DatabaseDLL_FinalRelease.dll").as_ptr());
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
    if LAST.swap(on as u8, Ordering::Relaxed) != on as u8 {
        log(&format!("pol : correctifs POL {}", if on { "actifs" } else { "coupés" }));
    }
    on
}
