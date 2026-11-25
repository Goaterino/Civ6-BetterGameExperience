//! Proxy de GameCore_XP2_FinalRelease.dll pour Civilization VI.
//!
//! Posé à côté de l'originale sous le nom `BGD_GameCore_XP2.dll`, sans la remplacer : le proxy de
//! démarrage (winmm.dll) le fait charger à sa place par l'exe. Il relaie ses 157 exports
//! (`forward`) et, au premier appel, la charge sous son vrai nom puis pose ses détours.
//!
//! Réglage : fichier `rollover_proxy.cfg` à côté de la DLL, lignes `actif=on|off` (interrupteur
//! général, on par défaut), `mode=coalesce|count|off`
//! (coalesce par défaut), `portee=cascade|message` (message par défaut),
//! `ameliorations=off|count|verify|fast|quiet` et `rendements=off|count|verify|fast|quiet` (quiet par
//! défaut), `wmd_accel=<facteur>|off` (20 par défaut, voir `wmd`), `ui_boutons`, `ui_recentrage` et
//! `ui_journal`, `ui_mesure_epingles`, `ui_epingles`, `ui_merveilles`, `ui_diag_accords` (voir `ui`), `ui_carte_entiere=on|off`
//! (on par défaut, voir `carte`), `ui_rendements=on|off`
//! (on par défaut, voir `rendements_empire`), `tours_production=on|off` (on par défaut, voir `tours`), `ui_debordement=on|off` (on, voir
//! `debordement`), `ui_banniere_annuler`, `ui_reduction_quartiers`, `ui_formations`, `ui_logement`, `ui_espions`, `ui_accord_quantites` et `ui_avancement` (on, voir `ui`),
//! `ui_charges`, `ui_routes`, `ui_notifications`, `ui_bandeau`, `ui_politiques`, `ui_rapports` (on
//! par défaut, voir `mods_ui`), `journal_chokepoints=on|off` (off, voir `journal_chokepoints`),
//! `delaunay_rapide=off|on|verifier` (on, voir `delaunay`), `depart_signaux=on|off` (off, voir `depart`), `ui_mesure_mouvements=on|off` (off, voir `ui`),
//! `relecture_fidele=on|off` (on, voir `relecture` ; côté client, pas POL).
//! Les réglages de diagnostic `ui_journal`, `ui_mesure_epingles` et `ui_diag_accords` ne sont lus
//! qu'avec la feature `diag` (builds locaux) : la release l'omet, ils y restent sur off. De même pour
//! `unites_promises=on|count|off`, `visibilite_chargee`, `ecouteurs_charges`,
//! `preferences_chargees` et `vision_guerre` (correctifs POL, toujours actifs en release, sans réglage joueur). Journal :
//! `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log`, hors du dossier Logs du jeu.
//!
//! Sous macOS, le crate se réduit à [`macos`] : voir analyses/macos/README.md.

#![allow(non_snake_case)] // le nom de la DLL produite, BGD_GameCore_XP2.

#[cfg(windows)]
include!("lib_windows.rs");

#[cfg(target_os = "macos")]
mod macos;
