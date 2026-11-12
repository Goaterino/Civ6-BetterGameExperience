//! Dossier de Civilization VI : bibliothèques Steam, ou chemin de la commande de lancement.

use std::fs;
use std::path::{Path, PathBuf};

use crate::win;

/// Un dossier est celui du jeu s'il contient le GameCore de Gathering Storm.
pub fn is_game_dir(dir: &Path) -> bool {
    dir.join("DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll").is_file()
}

/// Le jeu, dans l'une des bibliothèques Steam (`steamapps/libraryfolders.vdf`).
pub fn find() -> Option<PathBuf> {
    let steam = win::steam_path()?;
    let mut libraries = vec![steam.clone()];
    if let Ok(vdf) = fs::read_to_string(steam.join("steamapps\\libraryfolders.vdf")) {
        for line in vdf.lines() {
            // "path"		"D:\\SteamLibrary"
            let mut parts = line.split('"').filter(|p| !p.trim().is_empty());
            if parts.next() == Some("path") {
                if let Some(path) = parts.next() {
                    libraries.push(PathBuf::from(path.replace("\\\\", "\\")));
                }
            }
        }
    }
    libraries
        .into_iter()
        .map(|lib| lib.join("steamapps\\common\\Sid Meier's Civilization VI"))
        .find(|dir| is_game_dir(dir))
}

/// Le jeu, parmi les dossiers parents de l'exe que Steam lance (`%command%`).
pub fn from_command(exe: &Path) -> Option<PathBuf> {
    exe.ancestors().find(|dir| is_game_dir(dir)).map(Path::to_path_buf)
}

/// Civilization VI est lancé : `CivilizationVI.exe`, `CivilizationVI_DX12.exe`…
pub fn running() -> bool {
    win::process_names().iter().any(|n| n.to_lowercase().starts_with("civilizationvi"))
}
