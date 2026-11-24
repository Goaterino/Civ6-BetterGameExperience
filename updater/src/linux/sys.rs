//! Linux system calls, in place of `win.rs`: Steam roots, process list (`/proc`), desktop
//! notification and error dialog.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Directory of an XDG variable: the variable if it is an absolute path, otherwise
/// `$HOME/<default>` (`HOME` absolute too). A relative or empty value is ignored, as the XDG Base
/// Directory spec requires, and as the startup proxy does (startup_proxy/src/wine.rs): both must
/// find the same data directory.
pub fn xdg_dir(var: &str, default: &str, env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let absolute = |name: &str| env(name).map(PathBuf::from).filter(|p| p.is_absolute());
    match absolute(var) {
        Some(dir) => Some(dir),
        None => Some(absolute("HOME")?.join(default)),
    }
}

pub fn env(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

/// Possible Steam roots, in order of preference. No registry on Linux: known locations are tried.
/// (Flatpak and Snap are not handled.)
pub fn steam_roots() -> Vec<PathBuf> {
    steam_roots_with(&env)
}

pub fn steam_roots_with(env: &dyn Fn(&str) -> Option<OsString>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(data) = xdg_dir("XDG_DATA_HOME", ".local/share", env) {
        roots.push(data.join("Steam"));
    }
    if let Some(home) = env("HOME").map(PathBuf::from).filter(|p| p.is_absolute()) {
        roots.push(home.join(".local/share/Steam"));
        roots.push(home.join(".steam/steam"));
    }
    roots
}

/// Names (`comm`) of the processes in `proc`, except process `own` (the installer itself). `comm` is
/// truncated to 15 characters: only the start of a name is reliable.
pub fn process_names_in(proc: &Path, own: u32) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(proc) else { return Vec::new() };
    entries
        .flatten()
        .filter(|e| e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()).is_some_and(|pid| pid != own))
        .filter_map(|e| std::fs::read_to_string(e.path().join("comm")).ok())
        .map(|name| name.trim_end().to_string())
        .collect()
}

pub fn process_names() -> Vec<String> {
    process_names_in(Path::new("/proc"), std::process::id())
}

/// Runs `program` without input or output and waits for it: `None` if it is not installed,
/// otherwise whether it succeeded.
fn quiet(program: &str, args: &[&str]) -> Option<bool> {
    let status = Command::new(program).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    match status {
        Ok(s) => Some(s.success()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => Some(false),
    }
}

/// Desktop notification, if `notify-send` exists; no effect otherwise.
pub fn notify(title: &str, body: &str) {
    let _ = quiet("notify-send", &["--app-name=BetterGameExperience", title, body]);
}

/// Error dialog, for the installer launched by the game (no terminal): `zenity`, otherwise
/// `kdialog`. Without either, or if the dialog could not open (no display), an urgent desktop
/// notification. Waits for the dialog to be closed.
pub fn error(title: &str, body: &str) {
    let shown = quiet("zenity", &["--error", "--no-markup", "--title", title, "--text", body])
        .or_else(|| quiet("kdialog", &["--title", title, "--error", body]));
    if shown != Some(true) {
        let _ = quiet("notify-send", &["--app-name=BetterGameExperience", "--urgency=critical", title, body]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |name| vars.iter().find(|(k, _)| *k == name).map(|(_, v)| OsString::from(*v))
    }

    #[test]
    fn xdg() {
        let env = fake(&[("HOME", "/home/u"), ("XDG_DATA_HOME", "/data")]);
        assert_eq!(xdg_dir("XDG_DATA_HOME", ".local/share", &env), Some(PathBuf::from("/data")));
        // Empty or relative: as if it were not set.
        let env = fake(&[("HOME", "/home/u"), ("XDG_DATA_HOME", "")]);
        assert_eq!(xdg_dir("XDG_DATA_HOME", ".local/share", &env), Some(PathBuf::from("/home/u/.local/share")));
        let env = fake(&[("HOME", "/home/u"), ("XDG_DATA_HOME", "relatif")]);
        assert_eq!(xdg_dir("XDG_DATA_HOME", ".local/share", &env), Some(PathBuf::from("/home/u/.local/share")));
        // A relative HOME gives no directory.
        assert_eq!(xdg_dir("XDG_DATA_HOME", ".local/share", &fake(&[("HOME", "u")])), None);
        assert_eq!(xdg_dir("XDG_DATA_HOME", ".local/share", &fake(&[])), None);
    }

    #[test]
    fn racines_de_steam() {
        let env = fake(&[("HOME", "/home/u"), ("XDG_DATA_HOME", "/data")]);
        assert_eq!(
            steam_roots_with(&env),
            [PathBuf::from("/data/Steam"), "/home/u/.local/share/Steam".into(), "/home/u/.steam/steam".into()]
        );
    }

    #[test]
    fn processus_sauf_soi_meme() {
        let proc = std::env::temp_dir().join(format!("bgd_proc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&proc);
        for (pid, name) in [("10", "CivilizationVI_\n"), ("11", "bash\n"), ("12", "CivilizationVI\n")] {
            std::fs::create_dir_all(proc.join(pid)).unwrap();
            std::fs::write(proc.join(pid).join("comm"), name).unwrap();
        }
        std::fs::create_dir_all(proc.join("self")).unwrap();
        let mut names = process_names_in(&proc, 12);
        names.sort();
        assert_eq!(names, ["CivilizationVI_", "bash"]);
        std::fs::remove_dir_all(&proc).unwrap();
    }
}
