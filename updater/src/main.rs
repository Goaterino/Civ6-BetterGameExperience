//! BetterGameExperience_V<version>.exe : installe, met à jour, répare ou retire les proxys de
//! BetterGameExperience dans Civilization VI, à partir des releases GitHub (voir updater/README.md).
//!
//!   BetterGameExperience.exe                          fenêtre d'installation
//!   BetterGameExperience.exe --game-dir <dossier>     idem, pour un jeu hors des bibliothèques Steam
//!   BetterGameExperience.exe %command%                (option de lancement Steam, plus proposée) met
//!                                                     à jour, puis lance le jeu
//!   BetterGameExperience.exe --after-game [<dossier>] (notification du jeu) attend la fermeture du
//!                                                     jeu, puis met à jour
//!
//! Sous macOS, le même programme est le lanceur Mac (`BetterGameExperience.app`, voir macos/) :
//! les dylibs vont dans `~/Library/Application Support/BetterGameExperience/dylib`, et l'option de
//! lancement Steam, qui y est indispensable, injecte le chargeur au lancement du jeu.
#![windows_subsystem = "windows"]

#[cfg(not(any(windows, target_os = "macos")))]
compile_error!("l'installeur vise Windows et macOS");

mod game;
#[cfg(windows)]
mod gui;
#[cfg(target_os = "macos")]
#[path = "gui_mac.rs"]
mod gui;
#[cfg(windows)]
mod http;
#[cfg(target_os = "macos")]
#[path = "http_mac.rs"]
mod http;
mod install;
mod lang;
#[cfg(target_os = "macos")]
mod mac;
#[path = "../../proxy/src/noms.rs"]
mod noms;
mod release;
mod textes;
#[cfg(windows)]
mod win;

/// Fonctions du système : win.rs ou mac.rs.
#[cfg(target_os = "macos")]
use mac as sys;
#[cfg(windows)]
use win as sys;

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
/// macOS : copie l'option de lancement Steam dans le presse-papiers.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const CONFIGURE_STEAM: i32 = 104;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const RESTART_STEAM: i32 = 105;

/// Civilization VI sur Steam.
const STEAM_RUN: &str = "steam://rungameid/289070";

fn main() {
    release::migrate_data_dir();
    lang::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        // Relance en administrateur par install::run.
        #[cfg(windows)]
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
            Choice::ConfigureSteam => configure_steam(),
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
                install::run(action, &install::root(&game), &work)
            })?;
            release::remember_installed(None);
            // L'option de lancement restée dans Steam ne gêne pas (le lanceur lance alors le jeu
            // tel quel), mais on la retire si Steam est fermé : ouvert, il la réécrirait en quittant.
            #[cfg(target_os = "macos")]
            if !sys::steam_running() {
                let _ = game::write_launch_option(None);
            }
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
    /// macOS : écrire l'option de lancement dans Steam, puis revenir à la fenêtre.
    ConfigureSteam,
    Close,
}

/// Fenêtre principale, pour le manifeste du canal choisi.
fn ask_action(game: &Path, manifest: &Result<release::Manifest, String>) -> Choice {
    let states = install::status(&install::root(game), manifest.as_ref().ok());
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
        // Installeur de test (updater/macos/test) : visible au premier coup d'œil, non traduit.
        if cfg!(feature = "cle-de-test") {
            content.insert_str(0, "INSTALLEUR DE TEST (clé de test) : ne pas distribuer.\n\n");
        }
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
        // macOS : sans l'option de lancement Steam, les dylibs posées ne sont pas chargées.
        let launch_option = cfg!(target_os = "macos") && ours > 0;
        let steam_ok = launch_option && launch_option_set();
        if launch_option {
            content += launch_option_text(steam_ok);
        }
        let mut buttons = Vec::new();
        if !install_text.is_empty() {
            buttons.push((INSTALL, install_text.as_str()));
        }
        if launch_option && !steam_ok {
            buttons.push((CONFIGURE_STEAM, t(tx::MAC_CONFIGURE_STEAM)));
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
            CONFIGURE_STEAM => Choice::ConfigureSteam,
            _ => Choice::Close,
        };
    }
}

fn install_release(m: &release::Manifest, game: &Path, work: &Path) -> Result<(), String> {
    gui::progress(&f(tx::INSTALLING_BETTERGAMEDLLS_X, &[&m.version]), || {
        release::fetch_files(m, work)?;
        install::run(Action::Install, &install::root(game), work)
    })?;
    release::remember_installed(Some(&m.version));
    copy_self();
    configure_steam();
    Ok(())
}

/// macOS : l'option de lancement de Civilization VI désigne le lanceur, dans Steam.
fn launch_option_set() -> bool {
    #[cfg(target_os = "macos")]
    return game::launch_option_set();
    #[cfg(not(target_os = "macos"))]
    false
}

/// macOS : état de l'option de lancement Steam, à la suite du contenu de la fenêtre.
fn launch_option_text(set: bool) -> &'static str {
    t(if set { tx::MAC_LAUNCH_OPTION_IN_PLACE } else { tx::MAC_LAUNCH_OPTION_MISSING })
}

/// Option de lancement Steam qui passe par le lanceur, à son emplacement fixe.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn launch_option() -> Option<String> {
    Some(format!("\"{}\" %command%", stable_exe()?.display()))
}

/// macOS : écrit l'option de lancement de Civilization VI dans Steam, pour que Steam lance le jeu
/// par le lanceur. Steam relit sa configuration au démarrage et la réécrit en quittant : ouvert, il
/// est fermé puis rouvert, si le joueur l'accepte. En cas d'échec, l'option est copiée pour être
/// collée à la main.
fn configure_steam() {
    #[cfg(target_os = "macos")]
    {
        let Some(exe) = stable_exe() else { return };
        if game::launch_option_set() {
            return;
        }
        let running = sys::steam_running();
        if running && !ask_restart_steam() {
            return;
        }
        if running {
            sys::quit_steam();
        }
        let result = gui::progress(t(tx::MAC_CONFIGURING_STEAM), || {
            // Steam enregistre sa configuration en quittant : on écrit après lui.
            for _ in 0..120 {
                if !sys::steam_running() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
            if sys::steam_running() {
                return Err(t(tx::MAC_STEAM_DID_NOT_CLOSE).to_string());
            }
            game::write_launch_option(Some(&exe)).map(drop)
        });
        if running {
            sys::start_steam();
        }
        if let Err(e) = result {
            let option = launch_option().unwrap_or_default();
            sys::copy_to_clipboard(&option);
            let content = format!("{e}\n\n{}", f(tx::MAC_PASTE_IT_IN_STEAM_X, &[&option]));
            gui::message(Icon::Warning, t(tx::MAC_STEAM_NOT_CONFIGURED), &content);
        }
    }
}

/// macOS : Steam est ouvert, et doit redémarrer pour prendre l'option de lancement.
#[cfg(target_os = "macos")]
fn ask_restart_steam() -> bool {
    loop {
        let page = Page {
            icon: Icon::Info,
            instruction: t(tx::MAC_STEAM_MUST_RESTART),
            content: t(tx::MAC_STEAM_RESTART_TEXT),
            details: None,
            buttons: &[(RESTART_STEAM, t(tx::MAC_RESTART_STEAM))],
            checkbox: None,
        };
        match gui::ask(&page, &|_| false) {
            LANGUAGE_CHANGED => continue,
            choice => return choice == RESTART_STEAM,
        }
    }
}

/// Emplacement fixe de l'installeur : le proxy de démarrage l'y cherche pour la notification de
/// mise à jour (startup_proxy/src/maj.rs), et l'option de lancement Steam le désigne.
#[cfg(windows)]
fn stable_exe() -> Option<PathBuf> {
    Some(release::data_dir()?.join("BetterGameExperience.exe"))
}

/// macOS : l'exécutable du bundle copié à son emplacement fixe.
#[cfg(target_os = "macos")]
fn stable_exe() -> Option<PathBuf> {
    Some(stable_bundle()?.join("Contents/MacOS/BetterGameExperience"))
}

#[cfg(target_os = "macos")]
fn stable_bundle() -> Option<PathBuf> {
    Some(release::data_dir()?.join("BetterGameExperience.app"))
}

/// macOS : copie le bundle `.app` lancé à son emplacement fixe, s'il n'y tourne pas déjà. La copie
/// remplace l'ancienne d'un coup (dossier temporaire, puis renommage) : l'option de lancement Steam
/// ne désigne jamais un bundle à moitié copié. Lancé hors d'un bundle (`cargo run`), rien n'est
/// copié.
#[cfg(target_os = "macos")]
fn copy_self() {
    let (Ok(me), Some(stable)) = (std::env::current_exe().and_then(std::fs::canonicalize), stable_bundle()) else { return };
    let Some(bundle) = sys::bundle_of(&me) else { return };
    if std::fs::canonicalize(&stable).ok().as_deref() == Some(bundle.as_path()) {
        return;
    }
    let parent = stable.parent().unwrap();
    let staging = parent.join(format!(".BetterGameExperience.app.{}", std::process::id()));
    let old = parent.join(format!(".BetterGameExperience.app.old.{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    if std::fs::create_dir_all(parent).is_err() || sys::copy_dir(&bundle, &staging).is_err() {
        let _ = std::fs::remove_dir_all(&staging);
        return;
    }
    let had_old = std::fs::rename(&stable, &old).is_ok();
    if std::fs::rename(&staging, &stable).is_err() {
        if had_old {
            let _ = std::fs::rename(&old, &stable);
        }
        let _ = std::fs::remove_dir_all(&staging);
        return;
    }
    let _ = std::fs::remove_dir_all(&old);
}

/// Copie l'installeur à son emplacement fixe, s'il n'y tourne pas déjà : la copie suit ainsi le
/// dernier installeur lancé.
#[cfg(windows)]
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
/// propose de le relancer. Sous macOS, dit si Steam lance le jeu par le lanceur, sinon propose de le
/// configurer.
fn installed(version: &str, relaunch: bool) {
    loop {
        let instruction = f(tx::BETTERGAMEDLLS_X_IS_INSTALLED, &[&version]);
        let mut content = t(tx::YOU_CAN_CLOSE_THIS_WINDOW).to_string();
        let mut buttons = Vec::new();
        if cfg!(target_os = "macos") {
            let set = launch_option_set();
            content += launch_option_text(set);
            if !set {
                buttons.push((CONFIGURE_STEAM, t(tx::MAC_CONFIGURE_STEAM)));
            }
        }
        if relaunch {
            buttons.push((RELAUNCH, t(tx::RESTART_CIVILIZATION_VI_THROUGH_STEAM)));
        }
        let page = Page {
            icon: Icon::Info,
            instruction: &instruction,
            content: &content,
            details: None,
            buttons: &buttons,
            checkbox: None,
        };
        match gui::ask(&page, &|_| false) {
            LANGUAGE_CHANGED => continue,
            RELAUNCH => {
                sys::open(STEAM_RUN);
            }
            CONFIGURE_STEAM => {
                configure_steam();
                continue;
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
    let mut game = Command::new(&command[0]);
    game.args(&command[1..]);
    #[cfg(target_os = "macos")]
    inject(&mut game, Path::new(&command[0]));
    match game.status() {
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
    let root = install::root(&game);
    if install::status(&root, Some(&m)).iter().all(|(_, s)| *s == State::UpToDate) {
        return Ok(());
    }
    gui::progress(&f(tx::UPDATING_BETTERGAMEDLLS_TO_VERSION_X, &[&m.version]), || {
        release::fetch_files(&m, &work)?;
        install::run(Action::Install, &root, &work)
    })?;
    release::remember_installed(Some(&m.version));
    Ok(())
}

/// macOS : injecte le chargeur dans le jeu par `DYLD_INSERT_LIBRARIES`, que ses droits autorisent
/// (`allow-dyld-environment-variables`), si nos deux dylibs sont posées. `BGE_DYLIBS` dit au chargeur
/// où trouver le proxy de GameCore. Sans elles (désinstallation, pose échouée), le jeu se lance tel
/// quel : l'option de lancement restée dans Steam ne gêne pas.
#[cfg(target_os = "macos")]
fn inject(command: &mut Command, exe: &Path) {
    let game = game::from_command(exe).or_else(game::find).unwrap_or_default();
    let root = install::root(&game);
    let ours = install::status(&root, None).iter().all(|(_, s)| matches!(s, State::Installed | State::UpToDate));
    let Some(loader) = install::target_path(&root, noms::MAC_CHARGEUR).filter(|_| ours) else { return };
    let mut libraries = loader.into_os_string();
    if let Some(other) = std::env::var_os("DYLD_INSERT_LIBRARIES").filter(|v| !v.is_empty()) {
        libraries.push(":");
        libraries.push(other);
    }
    command.env("DYLD_INSERT_LIBRARIES", libraries);
    command.env("BGE_DYLIBS", root.join(noms::MAC_DOSSIER_DYLIBS));
}

#[cfg(all(test, target_os = "macos"))]
mod tests_mac {
    use super::*;

    fn env(command: &Command, name: &str) -> Option<String> {
        command.get_envs().find(|(k, _)| *k == name).and_then(|(_, v)| v).map(|v| v.to_string_lossy().into_owned())
    }

    /// Le chargeur n'est injecté que si nos deux dylibs sont posées (seul test à définir
    /// `BGE_DATA_DIR`).
    #[test]
    fn injection_des_dylibs() {
        let data = std::env::temp_dir().join(format!("bgd_inject_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        std::env::set_var("BGE_DATA_DIR", &data);
        let dylibs = data.join(noms::MAC_DOSSIER_DYLIBS);

        let mut sans = Command::new("/bin/true");
        inject(&mut sans, Path::new("/nulle/part"));
        assert_eq!(env(&sans, "DYLD_INSERT_LIBRARIES"), None, "rien de posé : pas d'injection");

        std::fs::create_dir_all(&dylibs).unwrap();
        std::fs::write(dylibs.join(noms::MAC_CHARGEUR), b"chargeur StartupProxyStatus").unwrap();
        let mut moitie = Command::new("/bin/true");
        inject(&mut moitie, Path::new("/nulle/part"));
        assert_eq!(env(&moitie, "DYLD_INSERT_LIBRARIES"), None, "une seule dylib : pas d'injection");

        std::fs::write(dylibs.join(noms::MAC_GAMECORE_PROXY), b"proxy RolloverProxyStatus").unwrap();
        let mut avec = Command::new("/bin/true");
        inject(&mut avec, Path::new("/nulle/part"));
        let injected = env(&avec, "DYLD_INSERT_LIBRARIES").unwrap();
        assert_eq!(injected.split(':').next().unwrap(), dylibs.join(noms::MAC_CHARGEUR).to_string_lossy());
        assert_eq!(env(&avec, "BGE_DYLIBS").unwrap(), dylibs.to_string_lossy());
        assert_eq!(launch_option().unwrap(), format!("\"{}\" %command%", data.join("BetterGameExperience.app/Contents/MacOS/BetterGameExperience").display()));

        std::env::remove_var("BGE_DATA_DIR");
        std::fs::remove_dir_all(&data).unwrap();
    }
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
