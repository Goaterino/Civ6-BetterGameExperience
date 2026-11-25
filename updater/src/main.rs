//! BetterGameExperience : installe, met à jour, répare ou retire les proxys de BetterGameExperience
//! dans Civilization VI, à partir des releases GitHub (voir updater/README.md). Une seule crate pour
//! les trois systèmes :
//!
//! - commun : la release (`release` : manifeste signé, fichiers contrôlés par leur SHA-256), la pose
//!   des proxys (`install`), les langues (`lang`) et les textes (`textes`, générés) ;
//! - Windows (`windows/`) : `BetterGameExperience_V<version>.exe`, fenêtres TaskDialog, WinHTTP,
//!   élévation en administrateur pour un jeu sous Program Files ;
//! - Linux (`linux/`) : binaire statique pour Steam Play (Proton), en ligne de commande, installé par
//!   install.sh ; ne modifie jamais l'option de lancement Steam, dit seulement comment la régler ;
//! - macOS (`macos/`) : le lanceur Mac (`BetterGameExperience.app`, bundle dans updater/macos/),
//!   fenêtres NSAlert, ureq et le TLS du système ; les dylibs vont dans
//!   `~/Library/Application Support/BetterGameExperience/dylib`, et l'option de lancement Steam, qui
//!   y est indispensable (le lanceur l'écrit), injecte le chargeur au lancement du jeu.
//!
//! `cfg(unix)` couvre aussi macOS : le code Linux est sous `target_os = "linux"`.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
compile_error!("l'installeur vise Windows, Linux et macOS");

mod install;
mod lang;
#[path = "../../proxy/src/noms.rs"]
mod noms;
mod release;
mod textes;
// Lecteur des fichiers VDF de Steam : ne sert qu'à Linux, mais n'a rien de propre au système ; compilé
// partout pour que ses tests tournent aussi sous Windows et macOS.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[path = "linux/vdf.rs"]
mod vdf;

#[cfg(windows)]
#[path = "windows/app.rs"]
mod app;
#[cfg(windows)]
#[path = "windows/game.rs"]
mod game;
#[cfg(windows)]
#[path = "windows/gui.rs"]
mod gui;
#[cfg(windows)]
#[path = "windows/http.rs"]
mod http;
#[cfg(windows)]
#[path = "windows/win.rs"]
mod win;

#[cfg(target_os = "linux")]
#[path = "linux/app.rs"]
mod app;
#[cfg(target_os = "linux")]
#[path = "linux/game.rs"]
mod game;
#[cfg(target_os = "linux")]
#[path = "linux/http.rs"]
mod http;
#[cfg(target_os = "linux")]
#[path = "linux/launch.rs"]
mod launch;
#[cfg(target_os = "linux")]
#[path = "linux/sys.rs"]
mod sys;
#[cfg(target_os = "linux")]
#[path = "linux/ui.rs"]
mod ui;

#[cfg(target_os = "macos")]
#[path = "macos/app.rs"]
mod app;
#[cfg(target_os = "macos")]
#[path = "macos/game.rs"]
mod game;
#[cfg(target_os = "macos")]
#[path = "macos/gui.rs"]
mod gui;
#[cfg(target_os = "macos")]
#[path = "macos/http.rs"]
mod http;
#[cfg(target_os = "macos")]
#[path = "macos/sys.rs"]
mod sys;

fn main() {
    app::main();
}
