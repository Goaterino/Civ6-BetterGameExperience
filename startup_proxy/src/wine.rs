//! Wine (Proton) detection and conversion between Windows and Unix paths.
//!
//! Under Wine, the game runs in a prefix where the Windows installer is useless: updates go through
//! the Linux installer (`updater_linux/`), placed outside the prefix in
//! `$XDG_DATA_HOME/BetterGameExperience` (`~/.local/share/` by default). Wine passes the Unix
//! environment to its processes and exposes the whole filesystem as drive `Z:`.

use crate::win::{wide, GetModuleHandleW, GetProcAddress};
use std::ffi::{c_char, CStr};
use std::path::{Path, PathBuf};

/// Wine adds `wine_get_version` to `ntdll.dll`: the detection method it documents.
pub fn is_wine() -> bool {
    unsafe {
        let ntdll = GetModuleHandleW(wide("ntdll.dll").as_ptr());
        !ntdll.is_null() && !GetProcAddress(ntdll, b"wine_get_version\0".as_ptr()).is_null()
    }
}

/// `$XDG_DATA_HOME/BetterGameExperience`, otherwise `$HOME/.local/share/BetterGameExperience`.
fn unix_data_dir(xdg: Option<&str>, home: Option<&str>) -> Option<String> {
    let base = match xdg.filter(|d| d.starts_with('/')) {
        Some(dir) => dir.trim_end_matches('/').to_string(),
        None => format!("{}/.local/share", home.filter(|h| h.starts_with('/'))?.trim_end_matches('/')),
    };
    Some(format!("{base}/BetterGameExperience"))
}

/// Unix path → Windows path on drive `Z:`.
fn unix_to_dos(path: &str) -> PathBuf {
    PathBuf::from(format!("Z:{}", path.replace('/', "\\")))
}

/// Path on drive `Z:` → Unix path.
fn dos_to_unix_z(path: &str) -> Option<String> {
    let rest = path.strip_prefix("Z:").or_else(|| path.strip_prefix("z:"))?;
    let unix = rest.replace('\\', "/");
    unix.starts_with('/').then_some(unix)
}

/// Unix home directory from `WINEHOMEDIR` (`\??\Z:\home\a`), which Wine sets even when `HOME` is
/// not passed through or is not a Unix path.
fn wine_home() -> Option<String> {
    let dir = std::env::var("WINEHOMEDIR").ok()?;
    dos_to_unix_z(dir.strip_prefix(r"\??\").unwrap_or(&dir))
}

/// The Unix home directory: `HOME` if it is a Unix path, otherwise from `WINEHOMEDIR`. Its last
/// component is the Linux user name, removed from bug reports (`rapport::anonymiser_unix`).
pub fn unix_home() -> Option<String> {
    std::env::var("HOME").ok().filter(|h| h.starts_with('/')).or_else(wine_home)
}

/// The Linux installer's data directory, as seen from Wine.
pub fn data_dir() -> Option<PathBuf> {
    let xdg = std::env::var("XDG_DATA_HOME").ok();
    unix_data_dir(xdg.as_deref(), unix_home().as_deref()).map(|d| unix_to_dos(&d))
}

/// `XDG_DESKTOP_DIR` of `user-dirs.dirs` (`"$HOME/Bureau"` or an absolute path), as a Unix path.
fn desktop_from_user_dirs(text: &str, home: &str) -> Option<String> {
    let value = text.lines().find_map(|l| l.trim().strip_prefix("XDG_DESKTOP_DIR="))?;
    let value = value.trim().trim_matches('"');
    let dir = match value.strip_prefix("$HOME") {
        Some(rest) => format!("{}{rest}", home.trim_end_matches('/')),
        None => value.to_string(),
    };
    // `$HOME/` alone means no desktop directory, per xdg-user-dirs.
    (dir.starts_with('/') && dir.trim_end_matches('/') != home.trim_end_matches('/')).then_some(dir)
}

/// The Linux user's desktop, as seen from Wine (`Z:`): `XDG_DESKTOP_DIR` of
/// `$XDG_CONFIG_HOME/user-dirs.dirs` (`~/.config` by default), otherwise `~/Desktop`. Under Proton,
/// the Windows desktop is a folder inside the prefix, where nobody looks.
pub fn desktop_dir() -> Option<PathBuf> {
    let home = unix_home()?;
    let config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|d| d.starts_with('/'))
        .unwrap_or_else(|| format!("{}/.config", home.trim_end_matches('/')));
    let configured = std::fs::read_to_string(unix_to_dos(&format!("{config}/user-dirs.dirs")))
        .ok()
        .and_then(|text| desktop_from_user_dirs(&text, &home));
    let dir = configured.unwrap_or_else(|| format!("{}/Desktop", home.trim_end_matches('/')));
    Some(unix_to_dos(&dir)).filter(|d| d.is_dir())
}

/// Wine's version and the host system (`wine_get_version` and `wine_get_host_version` of ntdll),
/// e.g. `("9.0", "Linux 6.12.1")`; `None` outside Wine. The host is empty if Wine does not say.
pub fn versions() -> Option<(String, String)> {
    type GetVersion = unsafe extern "C" fn() -> *const c_char;
    type GetHostVersion = unsafe extern "C" fn(*mut *const c_char, *mut *const c_char);
    let text = |p: *const c_char| if p.is_null() { String::new() } else { unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned() };
    unsafe {
        let ntdll = GetModuleHandleW(wide("ntdll.dll").as_ptr());
        if ntdll.is_null() {
            return None;
        }
        let version = GetProcAddress(ntdll, b"wine_get_version\0".as_ptr());
        if version.is_null() {
            return None;
        }
        let version: GetVersion = std::mem::transmute(version);
        let wine = text(version());
        let host = GetProcAddress(ntdll, b"wine_get_host_version\0".as_ptr());
        let host = if host.is_null() {
            String::new()
        } else {
            let host: GetHostVersion = std::mem::transmute(host);
            let (mut sysname, mut release) = (std::ptr::null(), std::ptr::null());
            host(&mut sysname, &mut release);
            format!("{} {}", text(sysname), text(release)).trim().to_string()
        };
        Some((wine, host))
    }
}

/// Unix path of a Windows path: drive `Z:` is converted by hand, other drives through kernel32's
/// `wine_get_unix_file_name`.
pub fn to_unix(path: &Path) -> Option<String> {
    let text = path.to_str()?;
    if let Some(unix) = dos_to_unix_z(text) {
        return Some(unix);
    }
    type GetUnixFileName = unsafe extern "system" fn(*const u16) -> *mut c_char;
    unsafe {
        let kernel32 = GetModuleHandleW(wide("kernel32.dll").as_ptr());
        let function = GetProcAddress(kernel32, b"wine_get_unix_file_name\0".as_ptr());
        if function.is_null() {
            return None;
        }
        let function: GetUnixFileName = std::mem::transmute(function);
        // Buffer allocated by Wine: called rarely (update launch, bug report), so it is not freed.
        let unix = function(wide(text).as_ptr());
        if unix.is_null() {
            return None;
        }
        CStr::from_ptr(unix).to_str().ok().map(str::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dossier_de_donnees() {
        assert_eq!(unix_data_dir(Some("/data"), Some("/home/a")).as_deref(), Some("/data/BetterGameExperience"));
        assert_eq!(unix_data_dir(None, Some("/home/a/")).as_deref(), Some("/home/a/.local/share/BetterGameExperience"));
        assert_eq!(unix_data_dir(Some("relatif"), Some("/home/a")).as_deref(), Some("/home/a/.local/share/BetterGameExperience"));
        assert_eq!(unix_data_dir(None, None), None);
    }

    #[test]
    fn chemins() {
        assert_eq!(unix_to_dos("/home/a/b"), PathBuf::from(r"Z:\home\a\b"));
        assert_eq!(dos_to_unix_z(r"Z:\home\a\b").as_deref(), Some("/home/a/b"));
        assert_eq!(dos_to_unix_z(r"z:\x").as_deref(), Some("/x"));
        assert_eq!(dos_to_unix_z(r"C:\Program Files"), None);
    }

    #[test]
    fn bureau_linux() {
        let dirs = "# comment\nXDG_DOWNLOAD_DIR=\"$HOME/Downloads\"\nXDG_DESKTOP_DIR=\"$HOME/Bureau\"\n";
        assert_eq!(desktop_from_user_dirs(dirs, "/home/a").as_deref(), Some("/home/a/Bureau"));
        assert_eq!(desktop_from_user_dirs("XDG_DESKTOP_DIR=\"/data/desk\"", "/home/a").as_deref(), Some("/data/desk"));
        // `$HOME/` disables the desktop directory.
        assert_eq!(desktop_from_user_dirs("XDG_DESKTOP_DIR=\"$HOME/\"", "/home/a"), None);
        assert_eq!(desktop_from_user_dirs("XDG_DOWNLOAD_DIR=\"$HOME/Downloads\"", "/home/a"), None);
    }
}
