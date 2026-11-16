//! Les fonctions du système utilisées sous macOS, l'équivalent de win.rs : dossier de Steam, liste
//! des processus (libproc), ouverture d'une adresse `steam://` (NSWorkspace), langue préférée
//! (NSLocale), presse-papiers (NSPasteboard).

use std::ffi::c_void;
use std::path::{Path, PathBuf};

use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString, NSWorkspace};
use objc2_foundation::{NSLocale, NSString, NSURL};

extern "C" {
    // libproc.h, dans libSystem.
    fn proc_listallpids(buffer: *mut c_void, size: i32) -> i32;
    fn proc_name(pid: i32, buffer: *mut c_void, size: u32) -> i32;
}

/// `~`, le dossier de l'utilisateur.
pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from)
}

/// Dossier de Steam : `~/Library/Application Support/Steam`.
pub fn steam_path() -> Option<PathBuf> {
    let dir = home()?.join("Library/Application Support/Steam");
    dir.is_dir().then_some(dir)
}

/// Noms des processus en cours (`Civ6_Exe`, `Civ6_Exe_Child`…), par libproc : pas de `ps` lancé.
pub fn process_names() -> Vec<String> {
    let count = unsafe { proc_listallpids(std::ptr::null_mut(), 0) };
    if count <= 0 {
        return Vec::new();
    }
    // De la marge : des processus peuvent naître entre les deux appels.
    let mut pids = vec![0i32; count as usize + 64];
    let size = (pids.len() * std::mem::size_of::<i32>()) as i32;
    let n = unsafe { proc_listallpids(pids.as_mut_ptr().cast(), size) };
    pids.truncate(n.max(0) as usize);
    let mut names = Vec::new();
    for pid in pids {
        let mut buf = [0u8; 256];
        let len = unsafe { proc_name(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
        if len > 0 {
            names.push(String::from_utf8_lossy(&buf[..len as usize]).into_owned());
        }
    }
    names
}

/// Ouvre `target` (une adresse `steam://`) comme un clic, sans attendre.
pub fn open(target: &str) {
    let Some(url) = NSURL::URLWithString(&NSString::from_str(target)) else { return };
    NSWorkspace::sharedWorkspace().openURL(&url);
}

/// Steam est ouvert (`steam_osx`).
pub fn steam_running() -> bool {
    process_names().iter().any(|n| n == "steam_osx")
}

/// Demande à Steam de quitter, comme son menu Steam > Quitter, sans attendre.
pub fn quit_steam() {
    open("steam://exit");
}

/// Rouvre Steam, sur la bibliothèque.
pub fn start_steam() {
    open("steam://open/games");
}

/// Copie `text` dans le presse-papiers.
pub fn copy_to_clipboard(text: &str) {
    let board = NSPasteboard::generalPasteboard();
    board.clearContents();
    board.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
}

/// Langues préférées de l'utilisateur, la première d'abord (`fr-FR`, `zh-Hans-CN`…).
pub fn preferred_languages() -> Vec<String> {
    NSLocale::preferredLanguages().iter().map(|l| l.to_string()).collect()
}

/// Le bundle `.app` qui contient `exe` (`…/X.app/Contents/MacOS/X`), s'il y en a un.
pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    let ok = macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && app.extension().is_some_and(|e| e.eq_ignore_ascii_case("app"));
    ok.then(|| app.to_path_buf())
}

/// Copie récursive d'un dossier (un bundle `.app`), droits compris : `fs::copy` garde le bit
/// d'exécution, et la signature ad hoc est dans les fichiers copiés. Les liens symboliques d'un
/// bundle sont recréés tels quels.
pub fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if kind.is_symlink() {
            std::os::unix::fs::symlink(std::fs::read_link(entry.path())?, &dest)?;
        } else if kind.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le processus du test doit être dans la liste, sous son nom (tronqué par le noyau).
    #[test]
    fn processus_en_cours() {
        let me = std::env::current_exe().unwrap();
        let me = me.file_name().unwrap().to_string_lossy().to_string();
        let names = process_names();
        assert!(names.len() > 1);
        assert!(names.iter().any(|n| me.starts_with(n.as_str()) && !n.is_empty()), "{me} absent de {names:?}");
    }

    #[test]
    fn bundle() {
        let exe = Path::new("/Applications/BetterGameExperience.app/Contents/MacOS/BetterGameExperience");
        assert_eq!(bundle_of(exe).as_deref(), Some(Path::new("/Applications/BetterGameExperience.app")));
        assert_eq!(bundle_of(Path::new("/tmp/target/release/BetterGameExperience")), None);
    }

    #[test]
    fn langues() {
        assert!(!preferred_languages().is_empty());
    }
}
