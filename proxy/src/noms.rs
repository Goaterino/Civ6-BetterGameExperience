//! Noms des fichiers posés dans le jeu, communs aux deux proxys et à l'installeur (inclus par
//! `#[path]`).
//!
//! Aucun fichier du dépôt Steam n'est remplacé : une vérification des fichiers ou une mise à jour du
//! jeu par Steam ne les retire pas.
//! - Le proxy de démarrage est `winmm.dll`, posé à côté de l'exe : l'exe l'importe de System32, et
//!   le chargeur la cherche d'abord dans son dossier. Il relaie les exports de la vraie.
//! - Le proxy de GameCore est `BGD_GameCore_XP2.dll`, à côté de l'original : le proxy de démarrage
//!   détourne le `LoadLibrary` de l'exe qui demande l'original vers lui, et il charge l'original
//!   sous son vrai nom.
#![allow(dead_code)]

/// GameCore de Gathering Storm, `DLC\Expansion2\Binaries\Win64`.
pub const GAMECORE: &str = "GameCore_XP2_FinalRelease.dll";
/// Proxy de GameCore, dans le même dossier.
pub const GAMECORE_PROXY: &str = "BGD_GameCore_XP2.dll";
/// Proxy de démarrage, `Base\Binaries\Win64Steam`.
pub const DEMARRAGE: &str = "winmm.dll";

/// Jusqu'à la v0.3.0 : chaque proxy prenait le nom de l'original, renommé `<nom>_orig.dll`.
/// L'installeur remet ces originaux en place.
pub const ANCIENS: [(&str, &str); 2] = [("DLC/Expansion2/Binaries/Win64", GAMECORE), ("Base/Binaries/Win64Steam", "DatabaseDLL_FinalRelease.dll")];
