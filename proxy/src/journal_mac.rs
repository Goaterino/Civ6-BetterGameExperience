//! Journal macOS, commun au chargeur et au proxy de GameCore :
//! `~/Library/Application Support/BetterGameExperience/RolloverProxy.log`, le pendant de
//! `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log` sous Windows.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Dossier de BetterGameExperience : le parent de `BGE_DYLIBS` (posé par le lanceur), sinon
/// `~/Library/Application Support/BetterGameExperience`.
pub fn data_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("BGE_DYLIBS").filter(|d| !d.is_empty()) {
        return Path::new(&d).parent().map(Path::to_path_buf);
    }
    Some(PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support/BetterGameExperience"))
}

pub fn log(msg: &str) {
    let Some(dir) = data_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("RolloverProxy.log")) {
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}
