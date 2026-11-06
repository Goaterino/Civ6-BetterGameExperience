//! Pose et retrait des proxys, à côté des fichiers du jeu sans en remplacer aucun (voir
//! proxy/src/noms.rs) : `winmm.dll` à côté de l'exe, `BGD_GameCore_XP2.dll` à côté de GameCore. Une
//! vérification des fichiers ou une mise à jour du jeu par Steam ne les retire pas.
//! `rollover_proxy.cfg` n'est pas touché : sans lui, le proxy prend ses réglages par défaut.
//!
//! Jusqu'à la v0.3.0, chaque proxy prenait le nom de l'original, renommé `<nom>_orig.dll`.
//! L'installation et la désinstallation remettent ces originaux en place.

use std::fs;
use std::path::{Path, PathBuf};

use crate::lang::f;
use crate::textes as tx;
use crate::noms;
use crate::release::{self, Manifest};
use crate::win;

pub struct Target {
    pub name: &'static str,
    dest: &'static str,
    /// Chaîne présente dans le proxy, absente de tout autre fichier de ce nom.
    marker: &'static [u8],
}

pub const TARGETS: [Target; 2] = [
    Target { name: noms::GAMECORE_PROXY, dest: "DLC/Expansion2/Binaries/Win64", marker: b"RolloverProxyStatus" },
    Target { name: noms::DEMARRAGE, dest: "Base/Binaries/Win64Steam", marker: b"StartupProxyStatus" },
];

/// Marques des anciens proxys, posés sous le nom des originaux.
const ANCIENNES_MARQUES: [&[u8]; 2] = [b"RolloverProxyStatus", b"StartupProxyStatus"];

/// Fichier où le processus administrateur laisse son erreur.
const ERROR_FILE: &str = "erreur.txt";

#[derive(Clone, Copy, PartialEq)]
pub enum State {
    /// Proxy pas posé.
    Absent,
    /// Un fichier de ce nom qui n'est pas de BetterGameDLLs (une autre `winmm.dll`, d'un autre mod).
    Foreign,
    /// Ancien proxy posé à la place d'un original (installation de la v0.3.0 ou d'avant).
    Old,
    /// Proxy posé, d'une autre version que la release (ou release inconnue).
    Installed,
    UpToDate,
}

#[derive(Clone, Copy)]
pub enum Action {
    Install,
    Uninstall,
}

fn contains(path: &Path, marker: &[u8]) -> Result<bool, String> {
    let data = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(data.windows(marker.len()).any(|w| w == marker))
}

impl Target {
    fn path(&self, game: &Path) -> PathBuf {
        game.join(self.dest).join(self.name)
    }

    fn state(&self, game: &Path, m: Option<&Manifest>) -> Result<State, String> {
        let path = self.path(game);
        if !path.exists() {
            return Ok(State::Absent);
        }
        if !contains(&path, self.marker)? {
            return Ok(State::Foreign);
        }
        let expected = m.and_then(|m| m.files.iter().find(|f| f.name == self.name));
        let current = release::sha256(&path).ok();
        Ok(if expected.is_some_and(|f| Some(&f.sha256) == current.as_ref()) { State::UpToDate } else { State::Installed })
    }
}

/// Ancienne installation : (original, `_orig`) de chaque fichier concerné.
fn old_files(game: &Path) -> impl Iterator<Item = (PathBuf, PathBuf)> + '_ {
    noms::ANCIENS.iter().map(move |(dest, name)| {
        let dll = game.join(dest).join(name);
        let orig = dll.with_file_name(name.replace(".dll", "_orig.dll"));
        (dll, orig)
    })
}

fn is_old_proxy(dll: &Path) -> Result<bool, String> {
    if !dll.exists() {
        return Ok(false);
    }
    for marker in ANCIENNES_MARQUES {
        if contains(dll, marker)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// État de chaque proxy, puis de chaque ancien proxy encore posé à la place d'un original.
pub fn status(game: &Path, m: Option<&Manifest>) -> Vec<(&'static str, State)> {
    let mut out: Vec<(&'static str, State)> =
        TARGETS.iter().map(|t| (t.name, t.state(game, m).unwrap_or(State::Foreign))).collect();
    for ((_, name), (dll, _)) in noms::ANCIENS.iter().zip(old_files(game)) {
        if is_old_proxy(&dll).unwrap_or(false) {
            out.push((name, State::Old));
        }
    }
    out
}

/// Installe la release téléchargée dans `work`, ou retire les proxys ; en administrateur si le
/// dossier du jeu n'est pas modifiable (Program Files).
pub fn run(action: Action, game: &Path, work: &Path) -> Result<(), String> {
    if writable(game) {
        return apply(action, game, work);
    }
    let _ = fs::remove_file(work.join(ERROR_FILE));
    let arg = match action {
        Action::Install => "install",
        Action::Uninstall => "uninstall",
    };
    let params = format!("--apply {arg} \"{}\" \"{}\"", work.display(), game.display());
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    match win::run_elevated(&exe, &params)? {
        0 => Ok(()),
        code => Err(fs::read_to_string(work.join(ERROR_FILE)).unwrap_or(f(tx::FAILURE_CODE_X, &[&code]))),
    }
}

/// Point d'entrée du processus administrateur : son erreur va dans `work`.
pub fn apply_elevated(action: &str, work: &Path, game: &Path) -> i32 {
    let action = if action == "uninstall" { Action::Uninstall } else { Action::Install };
    match apply(action, game, work) {
        Ok(()) => 0,
        Err(e) => {
            let _ = fs::write(work.join(ERROR_FILE), e);
            1
        }
    }
}

fn apply(action: Action, game: &Path, work: &Path) -> Result<(), String> {
    match action {
        // Relue ici : en administrateur, on ne se fie pas à ce que le processus utilisateur a vérifié.
        Action::Install => install(game, work, &release::load(work)?),
        Action::Uninstall => uninstall(game),
    }
}

fn writable(game: &Path) -> bool {
    TARGETS.iter().all(|t| {
        let probe = game.join(t.dest).join(".bgd_test");
        let ok = fs::write(&probe, b"").is_ok();
        let _ = fs::remove_file(&probe);
        ok
    })
}

/// Remet les originaux d'une installation de la v0.3.0 ou d'avant. Un `_orig` à côté d'un original
/// en place vient d'une installation écrasée par Steam (vérification des fichiers, mise à jour) :
/// l'original remis par Steam fait foi, le `_orig` est retiré.
fn restore_old(game: &Path) -> Result<(), String> {
    for (dll, orig) in old_files(game) {
        if is_old_proxy(&dll)? {
            if !orig.exists() {
                return Err(f(tx::X_IS_MISSING_VERIFY_THE_GAME, &[&orig.display()]));
            }
            fs::remove_file(&dll).map_err(|e| format!("{}: {e}", dll.display()))?;
            fs::rename(&orig, &dll).map_err(|e| format!("{}: {e}", orig.display()))?;
        } else if orig.exists() {
            fs::remove_file(&orig).map_err(|e| format!("{}: {e}", orig.display()))?;
        }
    }
    Ok(())
}

/// Refuse d'écraser un fichier qui n'est pas de BetterGameDLLs.
fn check_foreign(game: &Path) -> Result<(), String> {
    for t in &TARGETS {
        if t.state(game, None)? == State::Foreign {
            return Err(f(
                tx::X_IS_NOT_A_BETTERGAMEDLLS_FILE,
                &[&t.path(game).display()],
            ));
        }
    }
    Ok(())
}

fn install(game: &Path, work: &Path, m: &Manifest) -> Result<(), String> {
    // La release doit poser exactement nos cibles, aux mêmes endroits.
    let known = |f: &release::File| TARGETS.iter().any(|t| t.name == f.name && t.dest == f.dest);
    if m.files.len() != TARGETS.len() || !m.files.iter().all(known) {
        return Err(f(tx::VERSION_X_NEEDS_A_NEWER_INSTALLER, &[&m.version]));
    }
    check_foreign(game)?;
    restore_old(game)?;
    for t in &TARGETS {
        let dll = t.path(game);
        let new = dll.with_extension("dll.new");
        fs::copy(work.join(t.name), &new).map_err(|e| format!("{}: {e}", new.display()))?;
        fs::rename(&new, &dll).map_err(|e| format!("{}: {e}", dll.display()))?;
    }
    Ok(())
}

fn uninstall(game: &Path) -> Result<(), String> {
    restore_old(game)?;
    for t in &TARGETS {
        if matches!(t.state(game, None)?, State::Installed | State::UpToDate) {
            let dll = t.path(game);
            fs::remove_file(&dll).map_err(|e| format!("{}: {e}", dll.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Faux jeu : les deux originaux d'aujourd'hui, et le contenu donné à chaque chemin.
    fn game(name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bgd_install_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for (path, data) in [("DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll", &b"original GameCore"[..]), ("Base/Binaries/Win64Steam/DatabaseDLL_FinalRelease.dll", b"original DatabaseDLL")]
            .iter()
            .chain(files)
        {
            let p = dir.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, data).unwrap();
        }
        dir
    }

    fn read(dir: &Path, path: &str) -> Option<Vec<u8>> {
        fs::read(dir.join(path)).ok()
    }

    #[test]
    fn ancienne_installation_remise() {
        let dir = game(
            "ancienne",
            &[
                ("DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll", b"proxy RolloverProxyStatus"),
                ("DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease_orig.dll", b"vrai GameCore"),
                ("Base/Binaries/Win64Steam/DatabaseDLL_FinalRelease_orig.dll", b"vieil original"),
            ],
        );
        let st = status(&dir, None);
        assert!(st.iter().any(|(n, s)| *n == noms::GAMECORE && *s == State::Old));
        assert!(!st.iter().any(|(n, _)| *n == "DatabaseDLL_FinalRelease.dll"), "original remis par Steam : pas un ancien proxy");
        restore_old(&dir).unwrap();
        assert_eq!(read(&dir, "DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll").unwrap(), b"vrai GameCore");
        assert!(read(&dir, "DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease_orig.dll").is_none());
        assert_eq!(read(&dir, "Base/Binaries/Win64Steam/DatabaseDLL_FinalRelease.dll").unwrap(), b"original DatabaseDLL");
        assert!(read(&dir, "Base/Binaries/Win64Steam/DatabaseDLL_FinalRelease_orig.dll").is_none(), "_orig périmé retiré");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ancien_proxy_sans_orig() {
        let dir = game("sans_orig", &[("Base/Binaries/Win64Steam/DatabaseDLL_FinalRelease.dll", b"proxy StartupProxyStatus")]);
        assert!(restore_old(&dir).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn winmm_d_un_autre_mod() {
        let dir = game("autre", &[("Base/Binaries/Win64Steam/winmm.dll", b"chargeur ASI")]);
        let st = status(&dir, None);
        assert!(st.iter().any(|(n, s)| *n == noms::DEMARRAGE && *s == State::Foreign));
        assert!(st.iter().any(|(n, s)| *n == noms::GAMECORE_PROXY && *s == State::Absent));
        assert!(check_foreign(&dir).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn desinstallation() {
        let dir = game(
            "desinstallation",
            &[("Base/Binaries/Win64Steam/winmm.dll", b"StartupProxyStatus"), ("DLC/Expansion2/Binaries/Win64/BGD_GameCore_XP2.dll", b"RolloverProxyStatus")],
        );
        uninstall(&dir).unwrap();
        assert!(status(&dir, None).iter().all(|(_, s)| *s == State::Absent));
        assert!(read(&dir, "DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll").is_some());
        fs::remove_dir_all(&dir).unwrap();
    }
}
