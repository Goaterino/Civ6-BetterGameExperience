//! Proxy de démarrage : winmm.dll pour Civilization VI.
//!
//! L'exe importe `WINMM.dll` de Windows, qui n'est pas une KnownDLL et que le jeu ne livre pas : le
//! chargeur la cherche d'abord dans le dossier de l'exe, `Base\Binaries\Win64Steam`, où ce proxy est
//! posé. Il est donc chargé au lancement, avant le menu principal, sans remplacer aucun fichier de
//! Steam. Ses exports sont relayés vers la winmm de System32 (voir build.rs et `relais`).
//!
//! This proxy loads the GameCore proxy instead of the original (`chargeur`), adds to the options
//! screen a tab that edits its settings (`options`), announces its version to the other players in
//! the multiplayer lobby (`salon`), adds to the Escape menu a button that sends a bug report
//! (`rapport`), bounds the counts read by the network message deserializer, against the freeze when
//! a player disconnects (`messages`), re-links pairs of players relayed through the host
//! (`reliaison`), and re-sends pings left unanswered, which would otherwise stay frozen at "0 ms" in
//! the player list (`pings`). Log shared by both proxies:
//! `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log`.

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
#[path = "../../proxy/src/noms.rs"]
mod noms;

mod chargeur;
mod relais;
mod options;
mod maj;
mod salon;
mod rapport;
mod messages;
mod reliaison;
mod pings;
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
        // Sous le verrou du chargeur : des appels à kernel32, déjà initialisée, et le chargement de
        // la winmm de System32, avant tout le reste (les modules qui importent winmm passent après).
        let relais = relais::init();
        let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
        log(&format!("démarrage : proxy winmm chargé dans {exe}, BetterGameExperience {}", version::VERSION));
        match relais {
            Ok((path, 0)) => log(&format!("démarrage : exports relayés vers {path}")),
            Ok((path, n)) => log(&format!("démarrage : exports relayés vers {path}, {n} absents de cette winmm (rendent 0)")),
            Err(e) => log(&format!("démarrage : relais de winmm impossibles : {e}")),
        }
        // Hors de l'exe du jeu (tests), on ne pose rien.
        let name = exe.rsplit(['/', '\\']).next().unwrap_or("").to_ascii_lowercase();
        if name.starts_with("civilizationvi") {
            chargeur::install(win::GetModuleHandleW(std::ptr::null()) as usize);
            options::install();
            messages::install();
            reliaison::install();
            pings::install();
            #[cfg(feature = "diag")]
            zones::install();
        }
    }
    1
}
