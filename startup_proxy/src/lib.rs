//! Proxy de démarrage : DatabaseDLL_FinalRelease.dll pour Civilization VI.
//!
//! L'exe importe statiquement cette DLL du jeu, posée à côté de lui dans `Base\Binaries\Win64Steam`.
//! Elle est donc chargée au lancement, avant le menu principal, alors que le proxy de GameCore ne
//! l'est qu'au lancement d'une partie. L'originale est renommée DatabaseDLL_FinalRelease_orig.dll ;
//! ses 5 exports sont renvoyés vers elle par le chargeur (voir build.rs).
//!
//! Ce proxy ajoute à l'écran d'options un onglet qui édite les réglages du proxy de GameCore
//! (`options`), et annonce sa version aux autres joueurs du salon multijoueur (`salon`). Journal
//! commun aux deux proxys : `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log`.

#![allow(non_snake_case)] // le nom de la DLL produite, DatabaseDLL_FinalRelease, est imposé.

// Modules communs avec le proxy de GameCore, qui y trouvent `crate::win` et `crate::patch`.
#[allow(dead_code)]
#[path = "../../proxy/src/exe.rs"]
mod exe;
#[allow(dead_code)]
#[path = "../../proxy/src/patch.rs"]
mod patch;
#[allow(dead_code)]
#[path = "../../proxy/src/win.rs"]
mod win;

mod options;
mod maj;
mod salon;
#[cfg(feature = "diag")]
mod zones;
#[path = "../../proxy/src/version.rs"]
mod version;

use std::ffi::c_void;
use std::io::Write;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

static SELF_MODULE: AtomicUsize = AtomicUsize::new(0);
static STATUS: AtomicU32 = AtomicU32::new(0);

/// Export de diagnostic, et marque que `install.ps1` cherche dans le fichier : DllMain appelée (1).
#[no_mangle]
pub extern "system" fn StartupProxyStatus() -> u32 {
    STATUS.load(Ordering::Relaxed)
}

/// Export lu par le proxy de GameCore avant chaque correctif POL : 1 s'ils sont actifs pour la
/// partie en cours (voir `salon`).
#[no_mangle]
pub extern "system" fn BgdPolActifs() -> u32 {
    salon::pol_actifs() as u32
}

pub(crate) fn log(msg: &str) {
    let Some(dir) = std::env::var_os("LOCALAPPDATA").map(|d| std::path::Path::new(&d).join("Civ6RolloverProxy")) else {
        return;
    };
    let _ = std::fs::create_dir_all(&dir);
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("RolloverProxy.log")) {
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

/// Dossier du proxy, celui de l'exe.
pub(crate) fn self_dir() -> Option<std::path::PathBuf> {
    let mut buf = [0u16; 1024];
    let n = unsafe {
        win::GetModuleFileNameW(SELF_MODULE.load(Ordering::Relaxed) as win::HMODULE, buf.as_mut_ptr(), buf.len() as u32)
    } as usize;
    let path = std::path::PathBuf::from(String::from_utf16_lossy(&buf[..n]));
    path.parent().map(|p| p.to_path_buf())
}

#[no_mangle]
pub unsafe extern "system" fn DllMain(module: win::HMODULE, reason: u32, _reserved: *mut c_void) -> i32 {
    if reason == win::DLL_PROCESS_ATTACH {
        SELF_MODULE.store(module as usize, Ordering::Relaxed);
        STATUS.fetch_or(1, Ordering::Relaxed);
        win::DisableThreadLibraryCalls(module);
        // Sous le verrou du chargeur : seulement des appels à kernel32, déjà initialisée. Hors de
        // l'exe du jeu (tests), on ne pose rien.
        let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
        log(&format!("démarrage : proxy DatabaseDLL chargé dans {exe}, BetterGameDLLs {}", version::VERSION));
        let name = exe.rsplit(['/', '\\']).next().unwrap_or("").to_ascii_lowercase();
        if name.starts_with("civilizationvi") {
            options::install();
            #[cfg(feature = "diag")]
            zones::install();
        }
    }
    1
}
