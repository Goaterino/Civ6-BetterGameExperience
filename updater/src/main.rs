//! BetterGameExperience_V<version>.exe : installe, met à jour, répare ou retire les proxys de
//! BetterGameExperience dans Civilization VI, à partir des releases GitHub (voir updater/README.md).
//!
//!   BetterGameExperience.exe                          fenêtre d'installation
//!   BetterGameExperience.exe --game-dir <dossier>     idem, pour un jeu hors des bibliothèques Steam
//!   BetterGameExperience.exe %command%                (option de lancement Steam, plus proposée) met
//!                                                     à jour, puis lance le jeu
//!   BetterGameExperience.exe --after-game [<dossier>] (notification du jeu) attend la fermeture du
//!                                                     jeu, puis met à jour
#![windows_subsystem = "windows"]

mod game;
mod gui;
mod http;
mod install;
mod lang;
#[path = "../../proxy/src/noms.rs"]
mod noms;
mod release;
mod textes;
mod win;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use gui::{Icon, Page, LANGUAGE_CHANGED, TOGGLED};
use install::{Action, State};
use lang::{f, t};
use textes as tx;

const INSTALL: i32 = 100;
const UNINSTALL: i32 = 101;
const RELAUNCH: i32 = 103;

/// Civilization VI sur Steam.
const STEAM_RUN: &str = "steam://rungameid/289070";

fn main() {
    release::migrate_data_dir();
    lang::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        // Relance en administrateur par install::run.
        Some("--apply") if args.len() == 4 => install::apply_elevated(&args[1], Path::new(&args[2]), Path::new(&args[3])),
        Some("--game-dir") if args.len() == 2 => interactive(Some(PathBuf::from(&args[1]))),
        Some("--after-game") => after_game(args.get(1).map(PathBuf::from)),
        Some(arg) if !arg.starts_with("--") => launch(&args),
        _ => interactive(None),
    };
    std::process::exit(code);
}

fn failed(e: &str) -> i32 {
    gui::message(Icon::Error, t(tx::THE_OPERATION_FAILED), e);
    1
}

fn interactive(game: Option<PathBuf>) -> i32 {
    // Dès l'ouverture, pas seulement après une installation : un joueur déjà à jour, qui ouvre un
    // installeur plus récent, met aussi à jour celui que la notification du jeu lance.
    copy_self();
    remove_old_installers();
    match menu(game) {
        Ok(()) => 0,
        Err(e) => failed(&e),
    }
}

fn find_game(dir: Option<PathBuf>) -> Result<PathBuf, String> {
    match dir {
        Some(dir) => game::from_command(&dir).ok_or_else(|| {
            f(
                tx::X_IS_NOT_THE_CIVILIZATION_VI,
                &[&dir.display()],
            )
        }),
        None => game::find().ok_or_else(|| {
            t(tx::CIVILIZATION_VI_WAS_NOT_FOUND_IN)
            .to_string()
        }),
    }
}

fn menu(game: Option<PathBuf>) -> Result<(), String> {
    let game = find_game(game)?;
    let work = release::work_dir();
    let mut manifest = release::fetch_manifest(&work);
    // Refaite à chaque changement de canal (case « Pré-release »).
    let action = loop {
        match ask_action(&game, &manifest) {
            Choice::Act(action) => break action,
            Choice::Close => return Ok(()),
            Choice::Toggle => {
                release::set_prerelease(!release::prerelease());
                let mut fetched = Err(String::new());
                let _ = gui::progress(t(tx::LOOKING_FOR_THE_LATEST_VERSION), || {
                    fetched = release::fetch_manifest(&work);
                    Ok(())
                });
                manifest = fetched;
            }
        }
    };
    if game::running() {
        return Err(t(tx::CLOSE_CIVILIZATION_VI_THEN_RUN_THE).into());
    }
    match action {
        Action::Install => {
            let m = manifest.unwrap();
            install_release(&m, &game, &work)?;
            installed(&m.version, false);
        }
        Action::Uninstall => {
            gui::progress(t(tx::UNINSTALLING_BETTERGAMEDLLS), || {
                install::run(action, &game, &work)
            })?;
            release::remember_installed(None);
            gui::message(
                Icon::Info,
                t(tx::BETTERGAMEDLLS_IS_UNINSTALLED),
                t(tx::THE_GAME_HAS_ITS_ORIGINAL_FILES),
            );
        }
    }
    Ok(())
}

enum Choice {
    Act(Action),
    /// Case « Pré-release » cochée ou décochée : le manifeste est à relire.
    Toggle,
    Close,
}

/// Fenêtre principale, pour le manifeste du canal choisi.
fn ask_action(game: &Path, manifest: &Result<release::Manifest, String>) -> Choice {
    let states = install::status(game, manifest.as_ref().ok());
    // Proxys posés, et ancienne installation (proxys à la place des originaux) à remplacer.
    let ours = states.iter().filter(|(_, s)| matches!(s, State::Installed | State::UpToDate)).count();
    let old = states.iter().any(|(_, s)| *s == State::Old);
    let up_to_date = states.iter().all(|(_, s)| *s == State::UpToDate);
    let prerelease = release::prerelease();
    // Version posée plus récente que celle du canal : une pré-release, que le canal release
    // remplace par la dernière release (retour en arrière).
    let installed = release::installed_version();
    let downgrade = match (manifest, &installed) {
        (Ok(m), Some(v)) if ours > 0 && !up_to_date && release::is_newer(v, &m.version) => Some(v.clone()),
        _ => None,
    };

    // Reconstruite à chaque changement de langue.
    loop {
        let mut content = f(tx::GAME_X, &[&game.display()]);
        if prerelease {
            content += t(tx::CHANNEL_PRE_RELEASE_TEST_VERSIONS_AHEAD);
        }
        let (icon, instruction, install_text) = match manifest {
            Err(e) => {
                content += &f(tx::COULD_NOT_READ_THE_LATEST_VERSION, &[e]);
                (Icon::Warning, t(tx::LATEST_VERSION_UNKNOWN).to_string(), String::new())
            }
            Ok(m) if up_to_date => (
                Icon::Info,
                f(tx::BETTERGAMEDLLS_X_IS_INSTALLED_AND_UP, &[&m.version]),
                String::new(),
            ),
            Ok(m) if ours == 0 && !old => (
                Icon::Info,
                t(tx::BETTERGAMEDLLS_IS_NOT_INSTALLED).to_string(),
                f(tx::INSTALL_DOWNLOADS_AND_INSTALLS_VERSION_X, &[&m.version]),
            ),
            Ok(m) if old => (
                Icon::Info,
                f(tx::BETTERGAMEDLLS_VERSION_X_IS_AVAILABLE, &[&m.version]),
                f(
                    tx::UPDATE_INSTALLS_VERSION_X_WHICH_NO,
                    &[&m.version],
                ),
            ),
            // Une partie seulement : un fichier a été retiré à la main.
            Ok(m) if ours < install::TARGETS.len() => (
                Icon::Warning,
                t(tx::BETTERGAMEDLLS_NEEDS_TO_BE_REPAIRED).to_string(),
                f(tx::REPAIR_REINSTALLS_VERSION_X, &[&m.version]),
            ),
            Ok(m) => match &downgrade {
                Some(v) => (
                    Icon::Info,
                    f(tx::BETTERGAMEDLLS_VERSION_X_IS_INSTALLED, &[v]),
                    f(
                        tx::GO_BACK_TO_VERSION_X_REPLACES,
                        &[&m.version],
                    ),
                ),
                None => (
                    Icon::Info,
                    f(tx::BETTERGAMEDLLS_VERSION_X_IS_AVAILABLE, &[&m.version]),
                    f(tx::UPDATE_INSTALLS_VERSION_X, &[&m.version]),
                ),
            },
        };
        let mut details: Vec<String> = states
            .iter()
            .map(|(name, state)| {
                let text = match state {
                    State::Absent => t(tx::NOT_INSTALLED),
                    State::Foreign => t(tx::FILE_FROM_ANOTHER_PROGRAM),
                    State::Old => t(tx::OLD_PROXY_IN_PLACE_OF_THE),
                    State::Installed => t(tx::PROXY_INSTALLED_OTHER_VERSION),
                    State::UpToDate => t(tx::PROXY_UP_TO_DATE),
                };
                format!("{name} : {text}")
            })
            .collect();
        if let (Some(v), true) = (&installed, ours > 0) {
            details.push(f(tx::VERSION_INSTALLED_BY_THE_INSTALLER_X, &[v]));
        }
        let mut buttons = Vec::new();
        if !install_text.is_empty() {
            buttons.push((INSTALL, install_text.as_str()));
        }
        if ours > 0 || old {
            buttons.push((UNINSTALL, t(tx::UNINSTALL_RESTORES_THE_GAME_S_ORIGINAL)));
        }
        let details = details.join("\n");
        let checkbox = t(tx::PRE_RELEASE_INSTALL_TEST_VERSIONS);
        let page = Page {
            icon,
            instruction: &instruction,
            content: &content,
            details: Some(&details),
            buttons: &buttons,
            checkbox: Some((checkbox, prerelease)),
        };
        return match gui::ask(&page, &|_| false) {
            LANGUAGE_CHANGED => continue,
            TOGGLED => Choice::Toggle,
            INSTALL => Choice::Act(Action::Install),
            UNINSTALL => Choice::Act(Action::Uninstall),
            _ => Choice::Close,
        };
    }
}

fn install_release(m: &release::Manifest, game: &Path, work: &Path) -> Result<(), String> {
    gui::progress(&f(tx::INSTALLING_BETTERGAMEDLLS_X, &[&m.version]), || {
        release::fetch_files(m, work)?;
        install::run(Action::Install, game, work)
    })?;
    release::remember_installed(Some(&m.version));
    copy_self();
    Ok(())
}

/// Emplacement fixe de l'installeur : le proxy de démarrage l'y cherche pour la notification de
/// mise à jour (startup_proxy/src/maj.rs), et l'option de lancement Steam le désigne.
fn stable_exe() -> Option<PathBuf> {
    Some(release::data_dir()?.join("BetterGameExperience.exe"))
}

/// Copie l'installeur à son emplacement fixe, s'il n'y tourne pas déjà : la copie suit ainsi le
/// dernier installeur lancé.
fn copy_self() {
    let (Ok(me), Some(stable)) = (std::env::current_exe(), stable_exe()) else { return };
    if std::fs::canonicalize(&me).ok() == std::fs::canonicalize(&stable).ok() {
        return;
    }
    let _ = std::fs::create_dir_all(stable.parent().unwrap());
    let _ = std::fs::copy(&me, &stable);
}

/// Version de la release qui a produit cet installeur (`BGD_RELEASE`, posé par le workflow de
/// release). Aucune pour une compilation locale.
const RELEASE: Option<&str> = match option_env!("BGD_RELEASE") {
    Some(v) if !v.is_empty() => Some(v),
    _ => None,
};

/// Retire, dans le dossier de l'installeur lancé (d'habitude celui des téléchargements), les
/// installeurs téléchargés avant lui : `BetterGameExperience.exe` et `BetterGameDLLs.exe` (avant les
/// noms versionnés), et les `BetterGameExperience_V<version>.exe` de même version ou plus anciens.
/// Jamais la copie à l'emplacement fixe, que l'option de lancement Steam d'un joueur peut désigner.
fn remove_old_installers() {
    let Some(mine) = RELEASE else { return };
    let Ok(me) = std::env::current_exe().and_then(std::fs::canonicalize) else { return };
    let Some(dir) = me.parent() else { return };
    if release::data_dir().and_then(|d| std::fs::canonicalize(d).ok()).as_deref() == Some(dir) {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let old = path.file_name().and_then(|n| n.to_str()).is_some_and(|n| is_old_installer(n, mine));
        if old && path.is_file() && std::fs::canonicalize(&path).ok().as_deref() != Some(me.as_path()) {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Vrai si `name` est un installeur antérieur à la version `mine`. Le navigateur ajoute « (1) » à un
/// fichier téléchargé deux fois.
fn is_old_installer(name: &str, mine: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let Some(stem) = name.strip_suffix(".exe") else { return false };
    let stem = match stem.rsplit_once(" (") {
        Some((base, n)) if n.strip_suffix(')').is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) => base,
        _ => stem,
    };
    if stem == "bettergameexperience" || stem == "bettergamedlls" {
        return true;
    }
    match stem.strip_prefix("bettergameexperience_v") {
        Some(v) => v == mine || release::is_newer(mine, v),
        None => false,
    }
}

/// Fin d'installation : dit que tout est en place, et, après une mise à jour lancée depuis le jeu,
/// propose de le relancer.
fn installed(version: &str, relaunch: bool) {
    loop {
        let instruction = f(tx::BETTERGAMEDLLS_X_IS_INSTALLED, &[&version]);
        let mut buttons = Vec::new();
        if relaunch {
            buttons.push((RELAUNCH, t(tx::RESTART_CIVILIZATION_VI_THROUGH_STEAM)));
        }
        let page = Page {
            icon: Icon::Info,
            instruction: &instruction,
            content: t(tx::YOU_CAN_CLOSE_THIS_WINDOW),
            details: None,
            buttons: &buttons,
            checkbox: None,
        };
        match gui::ask(&page, &|_| false) {
            LANGUAGE_CHANGED => continue,
            RELAUNCH => {
                win::open(STEAM_RUN);
            }
            _ => {}
        }
        return;
    }
}

/// Lancé par la notification du jeu, qui se ferme : attend sa fin (une minute au plus), met à jour,
/// puis propose de le relancer.
fn after_game(dir: Option<PathBuf>) -> i32 {
    for _ in 0..120 {
        if !game::running() {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    if game::running() {
        return failed(t(tx::CIVILIZATION_VI_DID_NOT_CLOSE_CLOSE));
    }
    let result = (|| {
        let game = find_game(dir)?;
        let work = release::work_dir();
        let m = release::fetch_manifest(&work)?;
        install_release(&m, &game, &work)?;
        Ok::<_, String>(m.version)
    })();
    match result {
        Ok(version) => {
            installed(&version, true);
            0
        }
        Err(e) => failed(&e),
    }
}

/// Options de lancement Steam : met à jour ou répare avant de lancer le jeu, sans fenêtre si tout est
/// à jour. Un échec n'empêche pas de jouer.
fn launch(command: &[String]) -> i32 {
    if let Err(e) = update(Path::new(&command[0])) {
        gui::message(
            Icon::Warning,
            t(tx::BETTERGAMEDLLS_COULD_NOT_UPDATE),
            &f(tx::X_THE_GAME_WILL_START_WITHOUT, &[&e]),
        );
    }
    // L'attente garde le jeu « en cours » pour Steam.
    match Command::new(&command[0]).args(&command[1..]).status() {
        Ok(status) => status.code().unwrap_or(0),
        Err(_) => 1,
    }
}

fn update(exe: &Path) -> Result<(), String> {
    let game = game::from_command(exe)
        .or_else(game::find)
        .ok_or_else(|| t(tx::CIVILIZATION_VI_NOT_FOUND).to_string())?;
    let work = release::work_dir();
    let m = release::fetch_manifest(&work)?;
    if install::status(&game, Some(&m)).iter().all(|(_, s)| *s == State::UpToDate) {
        return Ok(());
    }
    gui::progress(&f(tx::UPDATING_BETTERGAMEDLLS_TO_VERSION_X, &[&m.version]), || {
        release::fetch_files(&m, &work)?;
        install::run(Action::Install, &game, &work)
    })?;
    release::remember_installed(Some(&m.version));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_old_installer;

    #[test]
    fn anciens_installeurs() {
        assert!(is_old_installer("BetterGameExperience.exe", "0.6.0"));
        assert!(is_old_installer("BetterGameExperience (2).exe", "0.6.0"));
        assert!(is_old_installer("BetterGameDLLs.exe", "0.6.0"));
        assert!(is_old_installer("BetterGameExperience_V0.5.1.exe", "0.6.0"));
        assert!(is_old_installer("BetterGameExperience_V0.6.0-rc.1.exe", "0.6.0"));
        assert!(is_old_installer("BetterGameExperience_V0.6.0 (1).exe", "0.6.0"));
        assert!(!is_old_installer("BetterGameExperience_V0.6.1.exe", "0.6.0"));
        assert!(!is_old_installer("BetterGameExperience_V0.7.0-rc.1.exe", "0.6.0"));
        assert!(!is_old_installer("BetterGameExperience_Vtruc.exe", "0.6.0"));
        assert!(!is_old_installer("BetterGameExperience.zip", "0.6.0"));
        assert!(!is_old_installer("CivilizationVI.exe", "0.6.0"));
    }
}
