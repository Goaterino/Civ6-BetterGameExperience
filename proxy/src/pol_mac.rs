//! État des correctifs POL sous macOS, le pendant de pol.rs : l'export `BgdPolActifs` du chargeur
//! (startup_proxy/src/macos_pol.rs), injecté par `DYLD_INSERT_LIBRARIES`, donc visible de tous.
//! Chargeur absent : correctifs coupés.

use super::journal::log;
use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

/// Export `BgdPolActifs` : 0 pas encore cherché, 1 absent, sinon son adresse.
static EXPORT: AtomicUsize = AtomicUsize::new(0);
/// Dernier état journalisé : 2 inconnu, puis 0 ou 1.
static LAST: AtomicU8 = AtomicU8::new(2);

extern "C" {
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}
const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;

pub fn actifs() -> bool {
    let mut p = EXPORT.load(Ordering::Relaxed);
    if p == 0 {
        let f = unsafe { dlsym(RTLD_DEFAULT, c"BgdPolActifs".as_ptr()) };
        p = if f.is_null() { 1 } else { f as usize };
        if p == 1 {
            log("pol : chargeur absent, correctifs POL coupés");
        }
        EXPORT.store(p, Ordering::Relaxed);
    }
    if p == 1 {
        return false;
    }
    let f: extern "C" fn() -> u32 = unsafe { std::mem::transmute(p) };
    let on = f() != 0;
    // Lecture d'abord : appelée depuis les fils de l'IA, une écriture à chaque appel ferait passer
    // la ligne de cache d'un cœur à l'autre.
    if LAST.load(Ordering::Relaxed) != on as u8 && LAST.swap(on as u8, Ordering::Relaxed) != on as u8 {
        log(&format!("pol : correctifs POL {}", if on { "actifs" } else { "coupés" }));
    }
    on
}
