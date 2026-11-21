//! Un seul chemin de calcul en virgule flottante pour tous les CPU tant que les correctifs POL sont
//! actifs (`maths_communes=on`, par défaut).
//!
//! GameCore importe `exp`, `expf`, `logf`, `pow` et `powf` du `ucrtbase.dll` du système (par
//! `api-ms-win-crt-math-l1-1-0.dll` ; jeux 1.0.12.72 et 1.0.12.74). Au démarrage, l'UCRT choisit
//! pour elles l'un de deux chemins : les instructions FMA3 si le CPU les a, sinon du SSE2 simple.
//! Les deux n'arrondissent pas toujours de la même façon (`exp` diffère sur environ 0,14 % des
//! entrées) : deux joueurs aux CPU différents peuvent obtenir des résultats différents dans la même
//! partie.
//!
//! Quand les correctifs POL deviennent actifs (tous les humains du salon ont cette version, voir
//! `salon`), le module fait passer l'UCRT sur son chemin sans FMA par `_set_FMA3_enable(0)` : chaque
//! machine calcule alors comme un CPU sans FMA3. Quand ils deviennent inactifs, il remet la valeur
//! choisie par l'UCRT au démarrage : avec un joueur sans BGE dans le salon, chaque machine garde le
//! comportement du jeu d'origine, et personne ne diverge de ce joueur à cause de ce module. Le
//! drapeau vaut pour tout le processus et il est lu à chaque appel : le changement compte dès
//! l'appel suivant, et le verdict du salon tombe avant le chargement de la partie.
//!
//! Non couvert : des joueurs avec des versions différentes de `ucrtbase.dll` (autre piste : nos
//! propres implémentations de ces fonctions), et macOS/Linux, où BGE ne tourne pas.
//!
//! Pas de réglage joueur. Les builds de diagnostic (feature `diag`) lisent `maths_communes=on|off`
//! dans `rollover_proxy.cfg` (`on` par défaut) : `off` laisse
//! l'UCRT tel quel ; un changement déjà fait est quand même défait quand les correctifs POL
//! deviennent inactifs.

use crate::{log, win};
use std::sync::atomic::{AtomicI32, Ordering};

/// Valeur de l'UCRT au démarrage, lue avant le premier changement : -1 pas encore lue, -2 UCRT
/// indisponible.
static STARTUP: AtomicI32 = AtomicI32::new(-1);

type SetFma3 = unsafe extern "C" fn(i32) -> i32;
type GetFma3 = unsafe extern "C" fn() -> i32;

/// Le réglage `maths_communes` dans le texte des réglages (dernière occurrence ; `on` par défaut).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("maths_communes=")).last().map_or(true, |v| v.trim() != "off")
}

/// La valeur à poser : 0 tant que les correctifs POL sont actifs, sinon celle de l'UCRT au
/// démarrage.
pub fn target(pol: bool, startup: i32) -> i32 {
    if pol {
        0
    } else {
        startup
    }
}

/// Les deux exports de l'UCRT. Hors de DllMain : charger `ucrtbase.dll` ici est sans risque, et
/// c'est l'instance à laquelle GameCore est lié (une par processus).
unsafe fn ucrt() -> Option<(SetFma3, GetFma3)> {
    let name: Vec<u16> = "ucrtbase.dll\0".encode_utf16().collect();
    let mut m = win::GetModuleHandleW(name.as_ptr());
    if m.is_null() {
        m = win::LoadLibraryW(name.as_ptr());
    }
    if m.is_null() {
        return None;
    }
    let set = win::GetProcAddress(m, b"_set_FMA3_enable\0".as_ptr());
    let get = win::GetProcAddress(m, b"_get_FMA3_enable\0".as_ptr());
    if set.is_null() || get.is_null() {
        return None;
    }
    Some((std::mem::transmute::<_, SetFma3>(set), std::mem::transmute::<_, GetFma3>(get)))
}

/// Appelé par `salon` quand l'état POL change (verdict du salon, partie locale, retour au salon).
pub fn pol_changed(pol: bool) {
    let switched = STARTUP.load(Ordering::Relaxed) >= 0;
    // Correctif POL sans réglage joueur : `maths_communes=off` n'est lu que par les builds de diagnostic.
    let on = !cfg!(feature = "diag") || enabled(&crate::options::read_config().map(|(t, _)| t).unwrap_or_default());
    // Rien à défaire, rien à faire.
    if !switched && (!pol || !on) {
        return;
    }
    unsafe {
        let Some((set, get)) = ucrt() else {
            if STARTUP.swap(-2, Ordering::Relaxed) != -2 {
                log("math: ucrtbase.dll or _set_FMA3_enable not found, math path left alone");
            }
            return;
        };
        if !switched {
            STARTUP.store(get(), Ordering::Relaxed);
        }
        let startup = STARTUP.load(Ordering::Relaxed);
        let want = if on || !pol { target(pol, startup) } else { startup };
        let before = get();
        if before != want {
            set(want);
        }
        log(&format!(
            "math: POL {}: UCRT FMA3 path {before} -> {} (startup value {startup})",
            if pol { "active" } else { "inactive" },
            get()
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(enabled("maths_communes=on\n"));
        assert!(!enabled("maths_communes=off\r\n"));
        assert!(enabled("maths_communes=off\nmaths_communes=on\n"), "the last occurrence counts");
    }

    #[test]
    fn targets() {
        assert_eq!(target(true, 1), 0);
        assert_eq!(target(false, 1), 1);
        assert_eq!(target(false, 0), 0);
    }
}
