//! Salon multijoueur : chaque joueur annonce sa version de BetterGameDLLs, et l'hôte établit si tous
//! les humains ont la même (voir `salon.lua`).
//!
//! L'accroche `luaL_loadbuffer` d'`options` ajoute `salon.lua` à la fin de `StagingRoom.lua`, ou de
//! son remplaçant par un mod (MPH en a un, du même nom), après avoir posé dans la table globale du
//! contexte quatre fonctions natives : `Civ6Proxy_Version()`, `Civ6Proxy_Jeton()`,
//! `Civ6Proxy_Journal(texte)` et `Civ6Proxy_FixerPol(actif, raison)`.
//!
//! **Les correctifs POL** (« proxy-only lobbies ») changent la simulation : le proxy de GameCore ne
//! les applique que si `BgdPolActifs` (export de ce proxy, voir lib.rs) rend 1. Cet état est tenu
//! par l'interface :
//! - dans le salon, `salon.lua` le met à 0 à chaque ouverture, puis au verdict de l'hôte ;
//! - au chargement d'une partie locale (solo, hotseat), `chargement.lua`, ajouté à la fin de
//!   `LoadScreen.lua`, le met à 1 : une seule machine, rien à désynchroniser ;
//! - en cours de partie, l'hôte relaie le verdict à qui rejoint ou se reconnecte : `partie.lua`,
//!   ajouté à la fin de `InGame.lua`, y ajoute son jeton sans changer l'état.
//!
//! Une machine qui n'appliquerait pas ces correctifs ne doit pas s'annoncer : `Civ6Proxy_Version()`
//! rend alors nil et la raison (proxy de GameCore absent, désactivé, ou correctifs non posés).

use crate::options::{hks, publish_natives, push_str, Natives};
use crate::version::VERSION;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub const LUA_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("salon.lua"));
pub const SCRIPT: &str = "stagingroom.lua";
pub const CHARGEMENT_BLOCK: &str = include_str!("chargement.lua");
pub const SCRIPT_CHARGEMENT: &str = "loadscreen.lua";
pub const PARTIE_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("partie.lua"));
pub const SCRIPT_PARTIE: &str = "ingame.lua";
/// Option de partie « No First/Last moves (BGD) » (départ et fin de tour simultanés, depart_tour/README.md).
pub const OPTION_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("option_partie.lua"));
pub const SCRIPT_OPTION: &str = "setupparameters.lua";

/// Correctifs POL actifs pour la partie en cours (voir l'en-tête). Désactivés tant que l'interface
/// n'a rien dit.
static POL: AtomicBool = AtomicBool::new(false);

pub fn pol_actifs() -> bool {
    POL.load(Ordering::Relaxed)
}

/// Bits de `RolloverProxyStatus`, l'export d'état du proxy de GameCore (proxy/src/lib.rs).
const ST_ORIGINAL: u32 = 2;
const ST_POL: u32 = 16;

/// `actif=off` dans le fichier de réglages, dernière occurrence comme `read_config`.
pub fn actif_off(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("actif=")).last().is_some_and(|v| v.trim() == "off")
}

/// Pourquoi cette machine n'appliquerait pas les correctifs POL, ou None si elle les appliquera.
/// Proxy de GameCore déjà chargé : il le dit lui-même (bit ST_POL). Sinon, il le sera au lancement
/// de la partie, avec le fichier de réglages tel qu'il est.
fn pol_indisponible() -> Option<String> {
    unsafe {
        let m = crate::win::GetModuleHandleW(crate::win::wide(crate::noms::GAMECORE_PROXY).as_ptr());
        let p = if m.is_null() { std::ptr::null_mut() } else { crate::win::GetProcAddress(m, b"RolloverProxyStatus\0".as_ptr()) };
        if !p.is_null() {
            let status: extern "system" fn() -> u32 = std::mem::transmute(p);
            let st = status();
            if st & ST_ORIGINAL != 0 {
                return (st & ST_POL == 0)
                    .then(|| "le proxy de GameCore chargé n'a pas posé les correctifs POL (actif=off, ou jeu d'une autre version)".to_string());
            }
        }
    }
    match crate::options::read_config() {
        Ok((_, false)) => Some("proxy de GameCore non installé".into()),
        Ok((cfg, true)) if actif_off(&cfg) => Some("proxys désactivés (actif=off)".into()),
        Ok(_) => None,
        Err(e) => Some(e),
    }
}

/// StagingRoom.lua suivi du bloc du salon. Octets laissés tels quels, comme pour Options.lua.
pub fn patch_staging_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(LUA_BLOCK.as_bytes());
    out
}

/// Jeton d'une ouverture du salon : l'heure en nanosecondes, l'identifiant du processus et un
/// compteur, en hexadécimal. Il suffit qu'il diffère d'une ouverture à l'autre et d'une machine à
/// l'autre ; ce n'est pas un secret.
pub fn new_token() -> String {
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}{:x}{n:x}", std::process::id())
}

/// `Civ6Proxy_Version()` → la version ; ou nil et la raison si cette machine n'appliquerait pas les
/// correctifs POL.
unsafe extern "C-unwind" fn lua_version(l: *mut std::ffi::c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    match pol_indisponible() {
        None => {
            push_str(h, l, VERSION);
            1
        }
        Some(raison) => {
            (h.push_nil)(l);
            push_str(h, l, &raison);
            2
        }
    }
}

unsafe fn arg_str(l: *mut std::ffi::c_void, i: i32) -> Option<String> {
    let h = hks()?;
    let mut len = 0usize;
    let p = (h.to_lstring)(l, i, &mut len);
    (!p.is_null()).then(|| String::from_utf8_lossy(std::slice::from_raw_parts(p as *const u8, len)).into_owned())
}

/// `Civ6Proxy_FixerPol(actif, raison)`, actif 1 ou 0 : l'état lu par le proxy de GameCore. Journalisé
/// quand il change.
unsafe extern "C-unwind" fn lua_fix_pol(l: *mut std::ffi::c_void) -> i32 {
    let actif = arg_str(l, 1).is_some_and(|v| v.trim() == "1");
    if POL.swap(actif, Ordering::Relaxed) != actif {
        let raison = arg_str(l, 2).unwrap_or_default();
        crate::log(&format!("pol : correctifs POL {} ({raison})", if actif { "actifs" } else { "désactivés" }));
    }
    0
}

/// `Civ6Proxy_PolActifs()` → 1 si les correctifs POL sont actifs sur cette machine, sinon 0.
unsafe extern "C-unwind" fn lua_pol_state(l: *mut std::ffi::c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    (h.push_integer)(l, pol_actifs() as i32);
    1
}

unsafe extern "C-unwind" fn lua_token(l: *mut std::ffi::c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    push_str(h, l, &new_token());
    1
}

pub(crate) unsafe extern "C-unwind" fn lua_log(l: *mut std::ffi::c_void) -> i32 {
    if let Some(t) = arg_str(l, 1) {
        crate::log(&t);
    }
    0
}

pub const NATIVES: Natives = &[
    (lua_version, b"Civ6Proxy_Version\0"),
    (lua_token, b"Civ6Proxy_Jeton\0"),
    (lua_log, b"Civ6Proxy_Journal\0"),
    (lua_fix_pol, b"Civ6Proxy_FixerPol\0"),
];

const CHARGEMENT_NATIVES: Natives = &[(lua_log, b"Civ6Proxy_Journal\0"), (lua_fix_pol, b"Civ6Proxy_FixerPol\0")];
const PARTIE_NATIVES: Natives = &[(lua_log, b"Civ6Proxy_Journal\0"), (lua_pol_state, b"Civ6Proxy_PolActifs\0")];
const OPTION_NATIVES: Natives = &[(lua_log, b"Civ6Proxy_Journal\0")];

/// Appelée par l'accroche sur StagingRoom.lua : pose les natives, puis rend le texte à compiler.
pub unsafe fn prepare(l: *mut std::ffi::c_void, src: &[u8]) -> Vec<u8> {
    publish_natives(l, NATIVES, "salon");
    patch_staging_lua(src)
}

/// LoadScreen.lua suivi de `chargement.lua`.
pub fn patch_load_screen_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(CHARGEMENT_BLOCK.as_bytes());
    out
}

/// Appelée par l'accroche sur LoadScreen.lua.
pub unsafe fn prepare_chargement(l: *mut std::ffi::c_void, src: &[u8]) -> Vec<u8> {
    publish_natives(l, CHARGEMENT_NATIVES, "chargement");
    patch_load_screen_lua(src)
}

/// InGame.lua suivi de `partie.lua`.
pub fn patch_in_game_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(PARTIE_BLOCK.as_bytes());
    out
}

/// Appelée par l'accroche sur InGame.lua.
pub unsafe fn prepare_partie(l: *mut std::ffi::c_void, src: &[u8]) -> Vec<u8> {
    publish_natives(l, PARTIE_NATIVES, "partie");
    patch_in_game_lua(src)
}

/// SetupParameters.lua suivi de `option_partie.lua`.
pub fn patch_setup_parameters_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(OPTION_BLOCK.as_bytes());
    out
}

/// Appelée par l'accroche sur SetupParameters.lua, inclus par chaque écran de réglage de partie.
pub unsafe fn prepare_option(l: *mut std::ffi::c_void, src: &[u8]) -> Vec<u8> {
    publish_natives(l, OPTION_NATIVES, "option");
    patch_setup_parameters_lua(src)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jetons_distincts() {
        let (a, b) = (new_token(), new_token());
        assert_ne!(a, b);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()), "{a}");
    }

    #[test]
    fn bloc_ajoute() {
        let out = patch_staging_lua(b"Initialize();");
        assert!(out.starts_with(b"Initialize();\r\n-- "));
        assert!(LUA_BLOCK.contains("BGD - "));
        for &(_, name) in NATIVES {
            let name = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
            assert!(LUA_BLOCK.contains(name), "{name} posée mais pas utilisée par le bloc");
        }
    }

    #[test]
    fn interrupteur_general() {
        assert!(actif_off("mode=coalesce\r\nactif=off\r\n"));
        assert!(!actif_off("actif=off\nactif=on\n"), "la dernière occurrence compte");
        assert!(!actif_off("mode=coalesce\n"), "absent : actif");
    }

    #[test]
    fn bloc_de_chargement() {
        assert!(patch_load_screen_lua(b"Initialize();").starts_with(b"Initialize();\r\n-- "));
        assert!(patch_in_game_lua(b"Initialize();").starts_with(b"Initialize();\r\n-- "));
        for &(_, name) in PARTIE_NATIVES {
            let name = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
            assert!(PARTIE_BLOCK.contains(name), "{name} posée mais pas utilisée");
        }
        assert!(patch_setup_parameters_lua(b"x = 1").starts_with(b"x = 1\r\n-- "));
        for &(_, name) in OPTION_NATIVES {
            let name = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
            assert!(OPTION_BLOCK.contains(name), "{name} posée mais pas utilisée");
        }
        for &(_, name) in CHARGEMENT_NATIVES {
            let name = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
            assert!(CHARGEMENT_BLOCK.contains(name), "{name} posée mais pas utilisée");
        }
    }

    #[test]
    fn version_lisible() {
        // Annonce « <version>@<jeton>#<jeton> », verdict « <version>@<jeton>:<1|0>:<jetons> » : la
        // version ne doit contenir aucun de ces séparateurs.
        assert!(!VERSION.is_empty() && !VERSION.contains(['@', '#', ':', ',']), "{VERSION}");
    }
}
