//! Proxy de GameCore_XP2_FinalRelease.dll pour Civilization VI (prototype).
//!
//! L'originale est renommée GameCore_XP2_FinalRelease_orig.dll ; ce proxy prend son nom, relaie ses
//! 157 exports (`forward`) et, au premier appel, la charge puis pose les détours de `coalesce`.
//!
//! Réglage : fichier `rollover_proxy.cfg` à côté de la DLL, lignes `actif=on|off` (interrupteur
//! général, on par défaut), `mode=coalesce|count|off`
//! (coalesce par défaut), `portee=cascade|message` (cascade par défaut),
//! `ameliorations=off|count|verify|fast|quiet` et `rendements=off|count|verify|fast|quiet` (off par
//! défaut), `wmd_accel=<facteur>|off` (20 par défaut, voir `wmd`), `ui_boutons`, `ui_recentrage` et
//! `ui_journal`, `ui_mesure_epingles`, `ui_epingles`, `ui_merveilles` (voir `ui`), `ui_rendements=on|off`
//! (on par défaut, voir `rendements_empire`), `tours_production=on|off` (on par défaut, voir `tours`),
//! `ui_charges`, `ui_routes`, `ui_notifications`, `ui_bandeau`, `ui_politiques`, `ui_rapports` (off
//! par défaut, voir `mods_ui`). Journal :
//! `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log`, hors du dossier Logs du jeu.

#![allow(non_snake_case)] // le nom de la DLL produite, GameCore_XP2_FinalRelease, est imposé.

mod citoyens;
mod coalesce;
mod exe;
mod forward;
mod improvements;
mod mods_ui;
mod patch;
mod rendements;
mod rendements_empire;
mod tours;
mod ui;
#[allow(dead_code)] // RELEASE ne sert qu'au proxy de démarrage.
mod version;
mod win;
mod wmd;

use std::ffi::c_void;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub(crate) static SELF_MODULE: AtomicUsize = AtomicUsize::new(0);

/// Interrupteur général (`actif=on|off`, on par défaut) : off, le proxy charge l'originale et relaie
/// ses exports sans poser aucun détour, sans qu'il faille le désinstaller.
pub(crate) static ENABLED: AtomicBool = AtomicBool::new(true);

/// Étapes franchies, pour les tests : voir `RolloverProxyStatus`.
pub(crate) static STATUS: AtomicU32 = AtomicU32::new(0);
pub(crate) const ST_DLLMAIN: u32 = 1;
pub(crate) const ST_ORIGINAL: u32 = 2;
pub(crate) const ST_HOOKED: u32 = 8;
/// Correctifs POL posés (`citoyens`) : le proxy de démarrage ne s'annonce dans le salon que si ce
/// bit est là, une fois ce proxy chargé.
pub(crate) const ST_POL: u32 = 16;

/// Export de diagnostic : DllMain appelée (1), originale chargée (2), détours posés (8), correctifs
/// POL posés (16).
#[no_mangle]
pub extern "system" fn RolloverProxyStatus() -> u32 {
    STATUS.load(Ordering::Relaxed)
}

/// Export de test : charge l'originale et pose les détours sans appeler aucune fonction du jeu.
#[no_mangle]
pub extern "system" fn RolloverProxyLoadOriginal() -> usize {
    forward::real_module()
}

fn log_path() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(&std::env::var_os("LOCALAPPDATA")?).join("Civ6RolloverProxy");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("RolloverProxy.log"))
}

pub(crate) fn log(msg: &str) {
    let Some(path) = log_path() else { return };
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

pub(crate) fn self_dir() -> Option<std::path::PathBuf> {
    let mut buf = [0u16; 1024];
    let n = unsafe {
        win::GetModuleFileNameW(SELF_MODULE.load(Ordering::Relaxed) as win::HMODULE, buf.as_mut_ptr(), buf.len() as u32)
    } as usize;
    let path = std::path::PathBuf::from(String::from_utf16_lossy(&buf[..n]));
    path.parent().map(|p| p.to_path_buf())
}

pub(crate) fn read_config() {
    let Some(text) = self_dir().and_then(|d| std::fs::read_to_string(d.join("rollover_proxy.cfg")).ok()) else {
        return;
    };
    for line in text.lines() {
        if let Some(v) = line.trim().strip_prefix("actif=") {
            ENABLED.store(v.trim() != "off", Ordering::Relaxed);
        }
        if let Some(v) = line.trim().strip_prefix("mode=") {
            coalesce::set_mode(match v.trim() {
                "off" => coalesce::Mode::Off,
                "count" => coalesce::Mode::Count,
                _ => coalesce::Mode::Coalesce,
            });
        }
        if let Some(v) = line.trim().strip_prefix("portee=") {
            coalesce::set_scope(match v.trim() {
                "message" => coalesce::Scope::Message,
                _ => coalesce::Scope::Cascade,
            });
        }
        if let Some(v) = line.trim().strip_prefix("ameliorations=") {
            improvements::set_mode(match v.trim() {
                "count" => improvements::Mode::Count,
                "verify" => improvements::Mode::Verify,
                "fast" => improvements::Mode::Fast,
                "quiet" => improvements::Mode::Quiet,
                _ => improvements::Mode::Off,
            });
        }
        if let Some(v) = line.trim().strip_prefix("rendements=") {
            rendements::set_mode(match v.trim() {
                "count" => rendements::Mode::Count,
                "verify" => rendements::Mode::Verify,
                "fast" => rendements::Mode::Fast,
                "quiet" => rendements::Mode::Quiet,
                _ => rendements::Mode::Off,
            });
        }
        if let Some(v) = line.trim().strip_prefix("wmd_accel=") {
            // `off`, 1 ou une valeur illisible : pas d'accélération.
            wmd::set_factor(v.trim().parse::<f32>().ok().filter(|f| f.is_finite() && *f >= 1.0).unwrap_or(1.0));
        }
        if let Some(v) = line.trim().strip_prefix("tours_production=") {
            tours::set_enabled(v.trim() != "off");
        }
        if let Some(v) = line.trim().strip_prefix("ui_boutons=") {
            ui::set_boutons(v.trim() != "off");
        }
        if let Some(v) = line.trim().strip_prefix("ui_recentrage=") {
            ui::set_recentrage(v.trim() == "on");
        }
        if let Some(v) = line.trim().strip_prefix("ui_journal=") {
            ui::set_journal(v.trim() == "on");
        }
        if let Some(v) = line.trim().strip_prefix("ui_mesure_epingles=") {
            ui::set_mesure_epingles(v.trim() == "on");
        }
        if let Some(v) = line.trim().strip_prefix("ui_epingles=") {
            ui::set_epingles(v.trim() != "off");
        }
        if let Some(v) = line.trim().strip_prefix("ui_merveilles=") {
            ui::set_merveilles(v.trim() != "off");
        }
        if let Some(v) = line.trim().strip_prefix("ui_rendements=") {
            rendements_empire::set_enabled(v.trim() != "off");
        }
        if let Some(v) = line.trim().strip_prefix("ui_charges=") {
            mods_ui::set_charges(v.trim() == "on");
        }
        if let Some(v) = line.trim().strip_prefix("ui_routes=") {
            mods_ui::set_routes(v.trim() == "on");
        }
        if let Some(v) = line.trim().strip_prefix("ui_notifications=") {
            mods_ui::set_notifications(v.trim() == "on");
        }
        if let Some(v) = line.trim().strip_prefix("ui_bandeau=") {
            mods_ui::set_bandeau(v.trim() == "on");
        }
        if let Some(v) = line.trim().strip_prefix("ui_politiques=") {
            mods_ui::set_politiques(v.trim() == "on");
        }
        if let Some(v) = line.trim().strip_prefix("ui_rapports=") {
            mods_ui::set_rapports(v.trim() == "on");
        }
    }
}

#[no_mangle]
pub unsafe extern "system" fn DllMain(module: win::HMODULE, reason: u32, reserved: *mut c_void) -> i32 {
    match reason {
        win::DLL_PROCESS_ATTACH => {
            SELF_MODULE.store(module as usize, Ordering::Relaxed);
            STATUS.fetch_or(ST_DLLMAIN, Ordering::Relaxed);
            win::DisableThreadLibraryCalls(module);
        }
        // Déchargement par FreeLibrary (reserved nul), pas fin du processus : l'originale reste
        // chargée, on y remet les octets d'origine pour qu'aucun saut ne vise ce proxy déchargé.
        win::DLL_PROCESS_DETACH if reserved.is_null() => {
            coalesce::on_proxy_unload();
            citoyens::on_proxy_unload();
            improvements::on_proxy_unload();
            rendements::on_proxy_unload();
            rendements_empire::on_proxy_unload();
            tours::on_proxy_unload();
            wmd::on_proxy_unload();
            ui::on_proxy_unload();
        }
        _ => {}
    }
    1
}
