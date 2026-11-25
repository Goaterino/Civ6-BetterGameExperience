//! État des correctifs POL sous macOS, le pendant de la partie POL de salon.rs : le proxy de GameCore
//! (proxy/src/pol_mac.rs) lit l'export `BgdPolActifs` de ce chargeur avant chaque correctif POL.
//!
//! Comme sous Windows : le salon (macos_salon.rs) le met à 0 à chaque ouverture puis au verdict de
//! l'hôte ; une partie locale (solo, hotseat) l'active au chargement, par `chargement.lua` ajouté à
//! la fin de `LoadScreen.lua`.

use super::journal::log;
use super::options::{argument_n, publier, LuaCFn};
use std::ffi::{c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) use super::salon_texte::SCRIPT_CHARGEMENT;
use super::salon_texte::CHARGEMENT_BLOCK;

static POL: AtomicBool = AtomicBool::new(false);

/// Lu par le proxy de GameCore avant chaque correctif POL : 1 s'ils sont actifs pour la partie en
/// cours.
#[no_mangle]
pub extern "C" fn BgdPolActifs() -> u32 {
    POL.load(Ordering::Relaxed) as u32
}

/// Un booléen Lua passé en nombre : `1`, que HavokScript peut rendre `1.0` en chaîne.
fn vaut_un(v: &str) -> bool {
    v.trim().parse::<f64>().is_ok_and(|x| x == 1.0)
}

/// `Civ6Proxy_FixerPol(actif, raison)`, actif 1 ou 0. Journalisé quand il change (le salon
/// l'appelle à chaque publication).
pub(crate) unsafe extern "C-unwind" fn fixer_pol(l: *mut c_void) -> c_int {
    let actif = vaut_un(&argument_n(l, 1));
    if POL.swap(actif, Ordering::Relaxed) != actif {
        let raison = argument_n(l, 2);
        log(&format!("pol : correctifs POL {} ({raison})", if actif { "actifs" } else { "désactivés" }));
    }
    // Gros morceaux réseau : jamais sans les correctifs POL (macos_debit.rs).
    if !actif {
        super::debit::pol_off();
    }
    0
}

const NATIVES: [(LuaCFn, &str); 3] =
    [(super::salon::journal, "Civ6Proxy_Journal\0"), (fixer_pol, "Civ6Proxy_FixerPol\0"), super::absent::NATIVES_CHARGEMENT[0]];

/// LoadScreen.lua suivi de `chargement.lua`, si les natives sont posées.
pub unsafe fn chargement_lua(l: *mut c_void, src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    if !publier(l, &NATIVES) {
        log("chargement : natives non posées, correctifs POL coupés");
        return out;
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(CHARGEMENT_BLOCK.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booleen() {
        assert!(vaut_un("1") && vaut_un("1.0") && vaut_un(" 1 "));
        assert!(!vaut_un("0") && !vaut_un("") && !vaut_un("oui"));
    }

    #[test]
    fn natives_utilisees() {
        for (_, nom) in NATIVES {
            let nom = nom.trim_end_matches('\0');
            assert!(CHARGEMENT_BLOCK.contains(nom), "{nom} posée mais pas utilisée");
        }
        assert_eq!(super::super::options_texte::script_name(r"C:\Emu\AppAssets\Base\Assets\UI\LoadScreen.lua"), SCRIPT_CHARGEMENT);
    }
}
