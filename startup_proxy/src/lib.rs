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
//! (`relink`), re-sends pings left unanswered, which would otherwise stay frozen at "0 ms" in the
//! player list (`pings`), and refuses the host's second snapshot when it arrives after the end of
//! the load, against the double load when entering a game (`snapshots`). It also writes the team
//! ping artwork and audio next to the game (`medias`), before the game scans its mods. Log shared by both proxies:
//! `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log`.
//!
//! Sous macOS, le crate se réduit à [`macos`] : voir analyses/macos/README.md.


#[cfg(windows)]
include!("lib_windows.rs");

#[cfg(target_os = "macos")]
mod macos;
