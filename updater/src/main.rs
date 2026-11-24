//! BetterGameExperience : installe, met à jour, répare ou retire les proxys de BetterGameExperience
//! dans Civilization VI, à partir des releases GitHub (voir updater/README.md). Une seule crate pour
//! les deux systèmes :
//!
//! - commun : la release (`release` : manifeste signé, fichiers contrôlés par leur SHA-256), la pose
//!   des proxys (`install`), les langues (`lang`) et les textes (`textes`, générés) ;
//! - Windows (`windows/`) : `BetterGameExperience_V<version>.exe`, fenêtres TaskDialog, WinHTTP,
//!   élévation en administrateur pour un jeu sous Program Files ;
//! - Linux (`linux/`) : binaire statique pour Steam Play (Proton), en ligne de commande, installé par
//!   install.sh ; ne modifie jamais l'option de lancement Steam, dit seulement comment la régler.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod install;
mod lang;
#[path = "../../proxy/src/noms.rs"]
mod noms;
mod release;
mod textes;
// Lecteur des fichiers VDF de Steam : ne sert qu'à Linux, mais n'a rien de propre au système ; compilé
// partout pour que ses tests tournent aussi sous Windows.
#[cfg_attr(not(unix), allow(dead_code))]
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

#[cfg(unix)]
#[path = "linux/app.rs"]
mod app;
#[cfg(unix)]
#[path = "linux/game.rs"]
mod game;
#[cfg(unix)]
#[path = "linux/http.rs"]
mod http;
#[cfg(unix)]
#[path = "linux/launch.rs"]
mod launch;
#[cfg(unix)]
#[path = "linux/sys.rs"]
mod sys;
#[cfg(unix)]
#[path = "linux/ui.rs"]
mod ui;

fn main() {
    app::main();
}
