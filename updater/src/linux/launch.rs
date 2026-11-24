//! Is the Steam launch option set? Read-only: each Steam account's `localconfig.vdf`
//! (`userdata/<account>/config/`) holds Civilization VI's launch options, in
//! `UserLocalConfigStore/Software/Valve/Steam/apps/289070/LaunchOptions`. The installer never writes
//! it (main.rs): it only tells the player when the override is missing.

use std::fs;
use std::path::{Path, PathBuf};

use crate::sys;
use crate::vdf;

const APP_ID: &str = "289070";

#[derive(Debug, PartialEq)]
pub enum Check {
    /// At least one Steam account has the override before `%command%`.
    Set,
    /// Accounts were read, none has it.
    Missing,
    /// No `localconfig.vdf` could be read: nothing is said.
    Unknown,
}

pub fn check() -> Check {
    check_in(&sys::steam_roots())
}

pub fn check_in(roots: &[PathBuf]) -> Check {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut read_any = false;
    for root in roots {
        // `~/.steam/steam` is usually a link to `~/.local/share/Steam`: each root once.
        let Ok(root) = fs::canonicalize(root) else { continue };
        if seen.contains(&root) {
            continue;
        }
        seen.push(root.clone());
        let Ok(accounts) = fs::read_dir(root.join("userdata")) else { continue };
        for account in accounts.flatten() {
            let Some(options) = read_options(&account.path().join("config/localconfig.vdf")) else { continue };
            read_any = true;
            if options.as_deref().is_some_and(has_override) {
                return Check::Set;
            }
        }
    }
    if read_any {
        Check::Missing
    } else {
        Check::Unknown
    }
}

/// `None` if the file cannot be read; `Some(None)` if it has no launch options for the game.
fn read_options(path: &Path) -> Option<Option<String>> {
    let text = fs::read_to_string(path).ok()?;
    let root = vdf::parse(&text).ok()?;
    let mut block: &[(String, vdf::Value)] = &root;
    for key in ["UserLocalConfigStore", "Software", "Valve", "Steam", "apps", APP_ID] {
        match vdf::get_block(block, key) {
            Some(b) => block = b,
            None => return Some(None),
        }
    }
    Some(vdf::get_str(block, "LaunchOptions").map(str::to_string))
}

/// True if `options` sets `WINEDLLOVERRIDES` with `winmm` native (`n`, `n,b`, `native`…) before
/// `%command%`. After `%command%`, or without it, Steam passes the text to the game as arguments,
/// not as environment.
pub fn has_override(options: &str) -> bool {
    let Some(end) = options.find("%command%") else { return false };
    let mut rest = &options[..end];
    while let Some(i) = rest.find("WINEDLLOVERRIDES=") {
        rest = &rest[i + "WINEDLLOVERRIDES=".len()..];
        let value = match rest.chars().next() {
            Some(q @ ('"' | '\'')) => rest[1..].split(q).next().unwrap_or(""),
            _ => rest.split_whitespace().next().unwrap_or(""),
        };
        // `dll1,dll2=mode;dll3=mode`.
        let native = value.split(';').filter_map(|entry| entry.split_once('=')).any(|(dlls, mode)| {
            dlls.split(',').any(|d| d.trim().eq_ignore_ascii_case("winmm")) && mode.trim_start().starts_with(['n', 'N'])
        });
        if native {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_reconnue() {
        assert!(has_override("WINEDLLOVERRIDES=\"winmm=n,b\" %command%"));
        assert!(has_override("MANGOHUD=1 WINEDLLOVERRIDES=\"d3d11=n;winmm=n,b\" %command% -windowed"));
        assert!(has_override("WINEDLLOVERRIDES=winmm=n %command%"));
        assert!(has_override("WINEDLLOVERRIDES='dxgi,winmm=native,builtin' %command%"));
        // After %command%, or without it: arguments for the game.
        assert!(!has_override("%command% WINEDLLOVERRIDES=\"winmm=n,b\""));
        assert!(!has_override("WINEDLLOVERRIDES=\"winmm=n,b\""));
        // Builtin winmm, or another DLL only.
        assert!(!has_override("WINEDLLOVERRIDES=\"winmm=b\" %command%"));
        assert!(!has_override("WINEDLLOVERRIDES=\"winmmx=n\" %command%"));
        assert!(!has_override("PROTON_LOG=1 %command%"));
        assert!(!has_override(""));
    }

    fn compte(root: &Path, id: &str, options: Option<&str>) {
        let dir = root.join("userdata").join(id).join("config");
        fs::create_dir_all(&dir).unwrap();
        let app = match options {
            Some(o) => format!("\"{APP_ID}\"\n{{\n\"LaunchOptions\"\t\t\"{}\"\n}}\n", o.replace('"', "\\\"")),
            None => String::new(),
        };
        let text = format!("\"UserLocalConfigStore\"\n{{\n\"Software\"\n{{\n\"Valve\"\n{{\n\"Steam\"\n{{\n\"apps\"\n{{\n{app}}}\n}}\n}}\n}}\n}}\n");
        fs::write(dir.join("localconfig.vdf"), text).unwrap();
    }

    #[test]
    fn comptes_steam() {
        let root = std::env::temp_dir().join(format!("bgd_launch_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        assert_eq!(check_in(&[root.clone()]), Check::Unknown);
        compte(&root, "1", None);
        compte(&root, "2", Some("PROTON_LOG=1 %command%"));
        assert_eq!(check_in(&[root.clone()]), Check::Missing);
        // One account with the option is enough.
        compte(&root, "3", Some("WINEDLLOVERRIDES=\"winmm=n,b\" %command%"));
        assert_eq!(check_in(&[root.clone(), root.clone()]), Check::Set);
        fs::remove_dir_all(&root).unwrap();
    }
}
