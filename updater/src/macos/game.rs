//! Dossier de Civilization VI sous macOS : bibliothèques Steam, ou chemin de la commande de
//! lancement ; option de lancement Steam du lanceur (`localconfig.vdf`). L'équivalent de
//! windows/game.rs.
//!
//! Le port d'Aspyr : le dossier Steam du jeu contient `Civ6.app`, dont
//! `Contents/MacOS/GameCore_XP2.dll` est le GameCore (un bundle Mach-O, malgré l'extension).

use std::fs;
use std::path::{Path, PathBuf};

use crate::sys;

/// GameCore de Gathering Storm, relatif au dossier du jeu.
const GAMECORE: &str = "Civ6.app/Contents/MacOS/GameCore_XP2.dll";

/// Un dossier est celui du jeu s'il contient le GameCore de Gathering Storm.
pub fn is_game_dir(dir: &Path) -> bool {
    dir.join(GAMECORE).is_file()
}

/// Les bibliothèques Steam lues dans `libraryfolders.vdf`, la bibliothèque de Steam d'abord.
fn libraries(steam: &Path) -> Vec<PathBuf> {
    let mut libraries = vec![steam.to_path_buf()];
    if let Ok(vdf) = fs::read_to_string(steam.join("steamapps").join("libraryfolders.vdf")) {
        libraries.extend(parse_libraries(&vdf));
    }
    libraries
}

/// Chemins `"path"` de `libraryfolders.vdf`, où Steam double les `\` (Windows).
fn parse_libraries(vdf: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for line in vdf.lines() {
        // "path"		"D:\\SteamLibrary"
        let mut parts = line.split('"').filter(|p| !p.trim().is_empty());
        if parts.next() == Some("path") {
            if let Some(path) = parts.next() {
                out.push(PathBuf::from(path.replace("\\\\", "\\")));
            }
        }
    }
    out
}

/// Le jeu, dans l'une des bibliothèques Steam (`steamapps/libraryfolders.vdf`).
pub fn find() -> Option<PathBuf> {
    let steam = sys::steam_path()?;
    libraries(&steam)
        .into_iter()
        .map(|lib| lib.join("steamapps").join("common").join("Sid Meier's Civilization VI"))
        .find(|dir| is_game_dir(dir))
}

/// Le jeu, parmi les dossiers parents de l'exe que Steam lance (`%command%`).
pub fn from_command(exe: &Path) -> Option<PathBuf> {
    exe.ancestors().find(|dir| is_game_dir(dir)).map(Path::to_path_buf)
}

/// Civilization VI est lancé : `Civ6_Exe` (lanceur d'Aspyr) et `Civ6_Exe_Child` (le jeu).
pub fn running() -> bool {
    sys::process_names().iter().any(|n| is_game_process(n))
}

fn is_game_process(name: &str) -> bool {
    name.to_lowercase().starts_with("civ6_exe")
}

/// Bloc des jeux dans `localconfig.vdf` : `UserLocalConfigStore > Software > Valve > Steam > apps`,
/// où `"289070"` (Civilization VI) garde ses `LaunchOptions`.
const APPS: [&str; 5] = ["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"];
const CIV6: &str = "289070";

/// `localconfig.vdf` de chaque compte Steam de la machine (`userdata/<compte>/config`).
fn local_configs() -> Vec<PathBuf> {
    let Some(steam) = sys::steam_path() else { return Vec::new() };
    let Ok(users) = fs::read_dir(steam.join("userdata")) else { return Vec::new() };
    users.flatten().map(|u| u.path().join("config/localconfig.vdf")).filter(|p| p.is_file()).collect()
}

/// L'option de lancement Steam de Civilization VI désigne le lanceur, pour l'un des comptes
/// Steam de la machine.
pub fn launch_option_set() -> bool {
    local_configs().iter().any(|p| fs::read_to_string(p).is_ok_and(|vdf| has_launch_option(&vdf)))
}

/// Écrit (`Some(exe)`) ou retire (`None`) l'option de lancement du lanceur dans la
/// configuration de chaque compte Steam qui a un bloc des jeux. Steam doit être fermé : il relit
/// ce fichier au démarrage et le réécrit en quittant. L'ancien fichier est gardé à côté
/// (`localconfig.vdf.bge`), et le nouveau le remplace d'un coup (fichier temporaire, puis
/// renommage). Rend le nombre de comptes configurés.
pub fn write_launch_option(exe: Option<&Path>) -> Result<usize, String> {
    let exe = exe.map(|e| e.to_string_lossy().into_owned());
    let mut done = 0;
    for path in local_configs() {
        let err = |e: std::io::Error| format!("{}: {e}", path.display());
        let vdf = fs::read_to_string(&path).map_err(err)?;
        let Some(new) = with_launch_option(&vdf, exe.as_deref()) else {
            // Déjà comme il faut, ou compte sans bloc des jeux.
            done += usize::from(exe.is_some() && has_launch_option(&vdf));
            continue;
        };
        fs::write(path.with_extension("vdf.bge"), &vdf).map_err(err)?;
        let tmp = path.with_extension("vdf.bge-tmp");
        fs::write(&tmp, &new).map_err(err)?;
        fs::rename(&tmp, &path).map_err(err)?;
        done += 1;
    }
    if exe.is_some() && done == 0 {
        return Err(format!("{} : {APPS:?} > {CIV6}", "localconfig.vdf"));
    }
    Ok(done)
}

/// Dans le bloc `"289070"` des jeux, une ligne `"LaunchOptions"` qui cite le lanceur.
fn has_launch_option(vdf: &str) -> bool {
    Blocks::of(vdf).options.is_some_and(|(_, v)| v.contains("BetterGameExperience"))
}

/// Les chaînes d'une ligne de VDF (`"clé"  "valeur"`), sans leurs `\` d'échappement.
fn tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut s = String::new();
        while let Some(c) = chars.next() {
            match c {
                '\\' => s.extend(chars.next()),
                '"' => break,
                c => s.push(c),
            }
        }
        out.push(s);
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Repères de `localconfig.vdf`, en numéros de ligne : `LaunchOptions` de Civilization VI et sa
/// valeur, fin du bloc de Civilization VI, fin du bloc des jeux.
#[derive(Default)]
struct Blocks {
    options: Option<(usize, String)>,
    civ6_end: Option<usize>,
    apps_end: Option<usize>,
}

impl Blocks {
    fn of(vdf: &str) -> Blocks {
        let same = |path: &[String], want: &[&str]| path.len() == want.len() && path.iter().zip(want).all(|(a, b)| a.eq_ignore_ascii_case(b));
        let civ6_path: Vec<&str> = APPS.iter().copied().chain([CIV6]).collect();
        let mut b = Blocks::default();
        let mut path: Vec<String> = Vec::new();
        let mut key = String::new();
        for (n, line) in vdf.lines().enumerate() {
            match line.trim() {
                "{" => path.push(std::mem::take(&mut key)),
                "}" => {
                    if same(&path, &civ6_path) && b.civ6_end.is_none() {
                        b.civ6_end = Some(n);
                    } else if same(&path, &APPS) && b.apps_end.is_none() {
                        b.apps_end = Some(n);
                    }
                    path.pop();
                }
                _ => {
                    let t = tokens(line);
                    if t.len() == 1 {
                        key = t[0].clone();
                    } else if t.len() == 2 && t[0].eq_ignore_ascii_case("LaunchOptions") && same(&path, &civ6_path) {
                        b.options = Some((n, t[1].clone()));
                    }
                }
            }
        }
        b
    }
}

/// L'option de lancement `current`, avec le lanceur devant `%command%` (`Some(exe)`) ou sans lui
/// (`None`). Les autres options du joueur restent.
fn launch_option_value(current: &str, exe: Option<&str>) -> String {
    let mut rest = current.to_string();
    // Retire `"…BetterGameExperience…" ` (ancien emplacement compris).
    while let Some(i) = rest.find("BetterGameExperience") {
        let (Some(start), Some(len)) = (rest[..i].rfind('"'), rest[i..].find('"')) else { break };
        let mut end = i + len + 1;
        if rest[end..].starts_with(' ') {
            end += 1;
        }
        rest.replace_range(start..end, "");
    }
    let rest = rest.trim();
    let rest = if rest == "%command%" { "" } else { rest };
    match exe {
        None => rest.to_string(),
        Some(exe) => {
            let ours = format!("\"{exe}\" %command%");
            if rest.is_empty() {
                ours
            } else if rest.contains("%command%") {
                rest.replacen("%command%", &ours, 1)
            } else {
                format!("{ours} {rest}")
            }
        }
    }
}

/// `vdf` avec l'option de lancement de Civilization VI changée, ou `None` s'il n'y a rien à changer
/// (ou pas de bloc des jeux où l'écrire).
fn with_launch_option(vdf: &str, exe: Option<&str>) -> Option<String> {
    let b = Blocks::of(vdf);
    let current = b.options.as_ref().map_or("", |(_, v)| v.as_str());
    let value = launch_option_value(current, exe);
    if value == current {
        return None;
    }
    let mut lines: Vec<String> = vdf.split('\n').map(str::to_string).collect();
    let indent = |n: usize| lines[n].chars().take_while(|c| c.is_whitespace()).collect::<String>();
    let entry = |indent: &str| format!("{indent}\"LaunchOptions\"\t\t\"{}\"", escape(&value));
    match (b.options, b.civ6_end, b.apps_end) {
        (Some((n, _)), _, _) if value.is_empty() => {
            lines.remove(n);
            // Bloc de Civilization VI créé pour l'option, et resté vide : retiré aussi.
            let empty = n >= 2 && lines[n - 1].trim() == "{" && lines.get(n).is_some_and(|l| l.trim() == "}");
            if empty && tokens(&lines[n - 2]) == [CIV6] {
                lines.drain(n - 2..=n);
            }
        }
        (Some((n, _)), _, _) => {
            let i = indent(n);
            lines[n] = entry(&i);
        }
        (None, Some(end), _) => {
            let i = indent(end) + "\t";
            lines.insert(end, entry(&i));
        }
        (None, None, Some(end)) => {
            let i = indent(end) + "\t";
            let block = [format!("{i}\"{CIV6}\""), format!("{i}{{"), entry(&format!("{i}\t")), format!("{i}}}")];
            lines.splice(end..end, block);
        }
        (None, None, None) => return None,
    }
    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `localconfig.vdf` réduit, avec un autre jeu qui a aussi une option citant le lanceur.
    fn config(civ6: &str) -> String {
        format!(
            "\"UserLocalConfigStore\"\n{{\n\t\"Software\"\n\t{{\n\t\t\"Valve\"\n\t\t{{\n\t\t\t\"Steam\"\n\t\t\t{{\n\t\t\t\t\"apps\"\n\t\t\t\t{{\n\t\t\t\t\t\"8930\"\n\t\t\t\t\t{{\n\t\t\t\t\t\t\"LaunchOptions\"\t\t\"BetterGameExperience %command%\"\n\t\t\t\t\t}}\n{civ6}\t\t\t\t}}\n\t\t\t}}\n\t\t}}\n\t}}\n\t\"depots\"\n\t{{\n\t\t\"289070\"\n\t\t{{\n\t\t\t\"LaunchOptions\"\t\t\"BetterGameExperience\"\n\t\t}}\n\t}}\n}}\n"
        )
    }

    fn civ6(lines: &str) -> String {
        format!("\t\t\t\t\t\"289070\"\n\t\t\t\t\t{{\n\t\t\t\t\t\t\"Playtime\"\t\t\"1\"\n{lines}\t\t\t\t\t}}\n")
    }

    const EXE: &str = "/Users/x/Library/Application Support/BetterGameExperience/BetterGameExperience.app/Contents/MacOS/BetterGameExperience";

    #[test]
    fn option_de_lancement_lue() {
        // Seule celle de Civilization VI, dans le bloc des jeux, compte.
        assert!(!has_launch_option(&config(&civ6(""))));
        let avec = config(&civ6(&format!("\t\t\t\t\t\t\"LaunchOptions\"\t\t\"\\\"{EXE}\\\" %command%\"\n")));
        assert!(has_launch_option(&avec));
        assert_eq!(Blocks::of(&avec).options.unwrap().1, format!("\"{EXE}\" %command%"));
    }

    #[test]
    fn option_de_lancement_ecrite_puis_retiree() {
        let sans = config(&civ6(""));
        let avec = with_launch_option(&sans, Some(EXE)).unwrap();
        assert_eq!(avec, config(&civ6(&format!("\t\t\t\t\t\t\"LaunchOptions\"\t\t\"\\\"{EXE}\\\" %command%\"\n"))));
        assert!(has_launch_option(&avec));
        // Déjà en place : rien à réécrire.
        assert_eq!(with_launch_option(&avec, Some(EXE)), None);
        // Retirée : le fichier redevient celui d'avant.
        assert_eq!(with_launch_option(&avec, None).unwrap(), sans);
        assert_eq!(with_launch_option(&sans, None), None);
    }

    #[test]
    fn bloc_de_civ6_cree() {
        let sans_civ6 = config("");
        let avec = with_launch_option(&sans_civ6, Some(EXE)).unwrap();
        assert!(has_launch_option(&avec));
        assert_eq!(with_launch_option(&avec, None).unwrap(), sans_civ6, "bloc vide retiré");
        assert!(avec.contains("\t\t\t\t\t\"289070\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"LaunchOptions\""));
        // Pas de bloc des jeux : rien à écrire.
        assert_eq!(with_launch_option("\"UserLocalConfigStore\"\n{\n}\n", Some(EXE)), None);
    }

    #[test]
    fn options_du_joueur_gardees() {
        let ours = format!("\"{EXE}\" %command%");
        assert_eq!(launch_option_value("", Some(EXE)), ours);
        assert_eq!(launch_option_value("-nosplash", Some(EXE)), format!("{ours} -nosplash"));
        assert_eq!(launch_option_value("FOO=1 %command% -x", Some(EXE)), format!("FOO=1 {ours} -x"));
        // Ancien emplacement remplacé, pas doublé.
        assert_eq!(launch_option_value("\"/ancien/BetterGameExperience.app/x\" %command% -x", Some(EXE)), format!("{ours} -x"));
        assert_eq!(launch_option_value(&format!("FOO=1 {ours} -x"), None), "FOO=1 %command% -x");
        assert_eq!(launch_option_value(&ours, None), "");
    }

    /// Sur un vrai `localconfig.vdf` (une copie) : `BGE_VDF=<fichier> cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn vraie_configuration() {
        let vdf = fs::read_to_string(std::env::var("BGE_VDF").unwrap()).unwrap();
        let avec = with_launch_option(&vdf, Some(EXE)).unwrap();
        assert!(has_launch_option(&avec));
        let ajout: Vec<_> = avec.lines().filter(|l| !vdf.lines().any(|v| v == *l)).collect();
        println!("lignes ajoutées : {ajout:#?}");
        assert_eq!(with_launch_option(&avec, None).unwrap(), vdf);
    }

    #[test]
    fn bibliotheques() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/Users/x/Library/Application Support/Steam\"\n\t\t\"label\"\t\t\"\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n}";
        let libs = parse_libraries(vdf);
        assert_eq!(libs, vec![PathBuf::from("/Users/x/Library/Application Support/Steam"), PathBuf::from("D:\\SteamLibrary")]);
    }

    #[test]
    fn processus_du_jeu() {
        assert!(is_game_process("Civ6_Exe"));
        assert!(is_game_process("Civ6_Exe_Child"));
        assert!(!is_game_process("CivilizationVI.exe"));
        assert!(!is_game_process("Steam"));
    }

    #[test]
    fn dossier_depuis_la_commande() {
        let dir = std::env::temp_dir().join(format!("bgd_game_{}", std::process::id()));
        let core = dir.join(GAMECORE);
        fs::create_dir_all(core.parent().unwrap()).unwrap();
        fs::write(&core, b"x").unwrap();
        let exe = "Civ6.app/Contents/MacOS/Civ6_Exe_Child";
        assert_eq!(from_command(&dir.join(exe)).as_deref(), Some(dir.as_path()));
        assert_eq!(from_command(Path::new("/nulle/part/jeu")), None);
        fs::remove_dir_all(&dir).unwrap();
    }
}
