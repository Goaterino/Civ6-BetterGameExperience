//! BetterGameDLLs.exe : installe, met à jour, répare ou retire les proxys de BetterGameDLLs dans
//! Civilization VI, à partir des releases GitHub (voir updater/README.md).
//!
//!   BetterGameDLLs.exe                          fenêtre d'installation
//!   BetterGameDLLs.exe --game-dir <dossier>     idem, pour un jeu hors des bibliothèques Steam
//!   BetterGameDLLs.exe %command%                (options de lancement Steam) met à jour, puis lance le jeu
//!   BetterGameDLLs.exe --after-game [<dossier>] (notification du jeu) attend la fermeture du jeu, puis
//!                                               met à jour
#![windows_subsystem = "windows"]

mod game;
mod gui;
mod install;
mod lang;
#[path = "../../proxy/src/noms.rs"]
mod noms;
mod release;
mod win;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use gui::{Icon, Page, LANGUAGE_CHANGED};
use install::{Action, State};
use lang::{f, t};

const INSTALL: i32 = 100;
const UNINSTALL: i32 = 101;
const COPY: i32 = 102;
const RELAUNCH: i32 = 103;

/// Civilization VI sur Steam.
const STEAM_RUN: &str = "steam://rungameid/289070";

fn main() {
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
    gui::message(Icon::Error, t(["L'opération a échoué", "The operation failed"]), e);
    1
}

fn interactive(game: Option<PathBuf>) -> i32 {
    // Dès l'ouverture, pas seulement après une installation : un joueur déjà à jour, qui ouvre un
    // installeur plus récent, met aussi à jour celui que la notification du jeu lance.
    copy_self();
    match menu(game) {
        Ok(()) => 0,
        Err(e) => failed(&e),
    }
}

fn find_game(dir: Option<PathBuf>) -> Result<PathBuf, String> {
    match dir {
        Some(dir) => game::from_command(&dir).ok_or_else(|| {
            f(
                ["{} n'est pas le dossier de Civilization VI (Gathering Storm).", "{} is not the Civilization VI (Gathering Storm) folder."],
                &[&dir.display()],
            )
        }),
        None => game::find().ok_or_else(|| {
            t([
                "Civilization VI est introuvable dans les bibliothèques Steam.\n\nRelance l'installeur avec --game-dir \"<dossier du jeu>\".",
                "Civilization VI was not found in the Steam libraries.\n\nRun the installer again with --game-dir \"<game folder>\".",
            ])
            .to_string()
        }),
    }
}

fn menu(game: Option<PathBuf>) -> Result<(), String> {
    let game = find_game(game)?;
    let work = release::work_dir();
    let manifest = release::fetch_manifest(&work);
    let states = install::status(&game, manifest.as_ref().ok());
    // Proxys posés, et ancienne installation (proxys à la place des originaux) à remplacer.
    let ours = states.iter().filter(|(_, s)| matches!(s, State::Installed | State::UpToDate)).count();
    let old = states.iter().any(|(_, s)| *s == State::Old);
    let up_to_date = states.iter().all(|(_, s)| *s == State::UpToDate);

    // Reconstruite à chaque changement de langue.
    let action = loop {
        let mut content = f(["Jeu : {}", "Game: {}"], &[&game.display()]);
        let (icon, instruction, install_text) = match &manifest {
            Err(e) => {
                content += &f(["\n\nImpossible de lire la dernière version : {}", "\n\nCould not read the latest version: {}"], &[e]);
                (Icon::Warning, t(["Dernière version inconnue", "Latest version unknown"]).to_string(), String::new())
            }
            Ok(m) if up_to_date => (
                Icon::Info,
                f(["BetterGameDLLs {} est installé et à jour", "BetterGameDLLs {} is installed and up to date"], &[&m.version]),
                String::new(),
            ),
            Ok(m) if ours == 0 && !old => (
                Icon::Info,
                t(["BetterGameDLLs n'est pas installé", "BetterGameDLLs is not installed"]).to_string(),
                f(["Installer\nTélécharge et installe la version {}.", "Install\nDownloads and installs version {}."], &[&m.version]),
            ),
            Ok(m) if old => (
                Icon::Info,
                f(["La version {} de BetterGameDLLs est disponible", "BetterGameDLLs version {} is available"], &[&m.version]),
                f(
                    [
                        "Mettre à jour\nInstalle la version {}, qui ne remplace plus aucun fichier du jeu : Steam ne la retire plus.",
                        "Update\nInstalls version {}, which no longer replaces any game file: Steam no longer removes it.",
                    ],
                    &[&m.version],
                ),
            ),
            // Une partie seulement : un fichier a été retiré à la main.
            Ok(m) if ours < install::TARGETS.len() => (
                Icon::Warning,
                t(["BetterGameDLLs doit être réparé", "BetterGameDLLs needs to be repaired"]).to_string(),
                f(["Réparer\nRéinstalle la version {}.", "Repair\nReinstalls version {}."], &[&m.version]),
            ),
            Ok(m) => (
                Icon::Info,
                f(["La version {} de BetterGameDLLs est disponible", "BetterGameDLLs version {} is available"], &[&m.version]),
                f(["Mettre à jour\nInstalle la version {}.", "Update\nInstalls version {}."], &[&m.version]),
            ),
        };
        let details: Vec<String> = states
            .iter()
            .map(|(name, state)| {
                let text = match state {
                    State::Absent => t(["pas installé", "not installed"]),
                    State::Foreign => t(["fichier d'un autre programme", "file from another program"]),
                    State::Old => t(["ancien proxy à la place de l'original", "old proxy in place of the original"]),
                    State::Installed => t(["proxy installé, autre version", "proxy installed, other version"]),
                    State::UpToDate => t(["proxy à jour", "proxy up to date"]),
                };
                format!("{name} : {text}")
            })
            .collect();
        let mut buttons = Vec::new();
        if !install_text.is_empty() {
            buttons.push((INSTALL, install_text.as_str()));
        }
        if ours > 0 || old {
            buttons.push((UNINSTALL, t(["Désinstaller\nRemet les fichiers d'origine du jeu.", "Uninstall\nRestores the game's original files."])));
        }
        let details = details.join("\n");
        let page = Page { icon, instruction: &instruction, content: &content, details: Some(&details), buttons: &buttons };
        match gui::ask(&page, &|_| false) {
            LANGUAGE_CHANGED => continue,
            INSTALL => break Action::Install,
            UNINSTALL => break Action::Uninstall,
            _ => return Ok(()),
        }
    };
    if game::running() {
        return Err(t(["Ferme Civilization VI, puis relance l'installeur.", "Close Civilization VI, then run the installer again."]).into());
    }
    match action {
        Action::Install => {
            let m = manifest.unwrap();
            install_release(&m, &game, &work)?;
            installed(&m.version, false);
        }
        Action::Uninstall => {
            gui::progress(t(["Désinstallation de BetterGameDLLs…", "Uninstalling BetterGameDLLs…"]), || {
                install::run(action, &game, &work)
            })?;
            gui::message(
                Icon::Info,
                t(["BetterGameDLLs est désinstallé", "BetterGameDLLs is uninstalled"]),
                t(["Le jeu a retrouvé ses fichiers d'origine.", "The game has its original files back."]),
            );
        }
    }
    Ok(())
}

fn install_release(m: &release::Manifest, game: &Path, work: &Path) -> Result<(), String> {
    gui::progress(&f(["Installation de BetterGameDLLs {}…", "Installing BetterGameDLLs {}…"], &[&m.version]), || {
        release::fetch_files(m, work)?;
        install::run(Action::Install, game, work)
    })?;
    copy_self();
    Ok(())
}

/// Emplacement fixe de l'installeur : le proxy de démarrage l'y cherche pour la notification de
/// mise à jour (startup_proxy/src/maj.rs), et l'option de lancement Steam le désigne.
fn stable_exe() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("BetterGameDLLs\\BetterGameDLLs.exe"))
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

/// Fin d'installation : propose l'option de lancement Steam qui garde le proxy à jour, et, après
/// une mise à jour lancée depuis le jeu, de le relancer.
fn installed(version: &str, relaunch: bool) {
    let exe = stable_exe().filter(|p| p.is_file()).or_else(|| std::env::current_exe().ok()).unwrap_or_default();
    let option = format!("\"{}\" %command%", exe.display());
    loop {
        let content = f(
            [
                "Pour qu'il se mette à jour et se répare tout seul, ajoute cette option de lancement à Civilization VI dans Steam (Propriétés > Général > Options de lancement) :\n\n{}",
                "To keep it updated and repaired automatically, add this launch option to Civilization VI in Steam (Properties > General > Launch options):\n\n{}",
            ],
            &[&option],
        );
        let instruction = f(["BetterGameDLLs {} est installé", "BetterGameDLLs {} is installed"], &[&version]);
        let mut buttons = Vec::new();
        if relaunch {
            buttons.push((RELAUNCH, t(["Relancer Civilization VI\nPar Steam.", "Restart Civilization VI\nThrough Steam."])));
        }
        buttons.push((
            COPY,
            t(["Copier l'option de lancement\nÀ coller dans les options de lancement de Steam.", "Copy the launch option\nTo paste into Steam's launch options."]),
        ));
        let page = Page { icon: Icon::Info, instruction: &instruction, content: &content, details: None, buttons: &buttons };
        match gui::ask(&page, &|id| id == COPY && win::copy_to_clipboard(&option)) {
            LANGUAGE_CHANGED => continue,
            RELAUNCH => {
                let _ = Command::new("explorer.exe").arg(STEAM_RUN).spawn();
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
        return failed(t([
            "Civilization VI ne s'est pas fermé. Ferme-le, puis relance l'installeur.",
            "Civilization VI did not close. Close it, then run the installer again.",
        ]));
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
            t(["BetterGameDLLs n'a pas pu se mettre à jour", "BetterGameDLLs could not update"]),
            &f(["{}\n\nLe jeu va se lancer sans mise à jour.", "{}\n\nThe game will start without the update."], &[&e]),
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
        .ok_or_else(|| t(["Civilization VI introuvable.", "Civilization VI not found."]).to_string())?;
    let work = release::work_dir();
    let m = release::fetch_manifest(&work)?;
    if install::status(&game, Some(&m)).iter().all(|(_, s)| *s == State::UpToDate) {
        return Ok(());
    }
    gui::progress(&f(["Mise à jour de BetterGameDLLs vers la version {}…", "Updating BetterGameDLLs to version {}…"], &[&m.version]), || {
        release::fetch_files(&m, &work)?;
        install::run(Action::Install, &game, &work)
    })
}
