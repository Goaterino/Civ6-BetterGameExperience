//! Langues de l'installeur : celles de Better Balanced Game. Les textes sont dans `textes`, généré
//! par `textes/gen_textes.py` depuis `textes/<langue>.xml` : un tableau `Texts` par texte, une entrée
//! par langue de `LANGUAGES`, dans le même ordre. Le choix du joueur (liens en bas des fenêtres sous
//! Windows et macOS, commande `lang` sous Linux) est gardé dans `langue.txt` du dossier de données
//! (`release::data_dir`) ; sans choix, la langue du système (Windows : langue de l'interface ; Linux :
//! `LC_ALL`, puis `LC_MESSAGES`, puis `LANG` ; macOS : la première langue préférée), sinon l'anglais.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Language {
    /// Code gardé dans `langue.txt`.
    pub code: &'static str,
    /// Nom de la langue, dans cette langue.
    pub label: &'static str,
    /// Langue principale Windows (`PRIMARYLANGID`).
    #[cfg_attr(not(windows), allow(dead_code))]
    primary: u16,
}

pub const LANGUAGES: [Language; 11] = [
    Language { code: "en", label: "English", primary: 0x09 },
    Language { code: "fr", label: "Français", primary: 0x0C },
    Language { code: "de", label: "Deutsch", primary: 0x07 },
    Language { code: "es", label: "Español", primary: 0x0A },
    Language { code: "it", label: "Italiano", primary: 0x10 },
    Language { code: "ja", label: "日本語", primary: 0x11 },
    Language { code: "ko", label: "한국어", primary: 0x12 },
    Language { code: "pl", label: "Polski", primary: 0x15 },
    Language { code: "pt", label: "Português (Brasil)", primary: 0x16 },
    Language { code: "ru", label: "Русский", primary: 0x19 },
    Language { code: "zh", label: "简体中文", primary: 0x04 },
];

const DEFAULT: usize = 0;

pub type Texts = [&'static str; LANGUAGES.len()];

static CURRENT: AtomicUsize = AtomicUsize::new(DEFAULT);

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}

fn choice_file() -> Option<PathBuf> {
    Some(crate::release::data_dir()?.join("langue.txt"))
}

/// Langue de l'interface de Windows.
#[cfg(windows)]
fn system_language() -> Option<usize> {
    let primary = unsafe { GetUserDefaultUILanguage() } & 0x3FF;
    LANGUAGES.iter().position(|l| l.primary == primary)
}

/// Linux : langue de la locale POSIX.
#[cfg(target_os = "linux")]
fn system_language() -> Option<usize> {
    system_locale(&|name| std::env::var_os(name)).and_then(|l| from_locale(&l))
}

/// Langue d'une locale POSIX (`fr_FR.UTF-8`, `pt_BR`, `zh_CN@x`) : le préfixe avant `_`, `.` ou `@`.
/// Compilé partout, pour que ses tests tournent aussi sous Windows et macOS.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn from_locale(locale: &str) -> Option<usize> {
    let prefix = locale.split(['_', '.', '@']).next()?.to_ascii_lowercase();
    LANGUAGES.iter().position(|l| l.code == prefix)
}

/// Première variable de locale définie et non vide, dans l'ordre de priorité POSIX.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn system_locale(env: &dyn Fn(&str) -> Option<std::ffi::OsString>) -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|v| env(v).filter(|s| !s.is_empty()))
        .map(|s| s.to_string_lossy().into_owned())
}

/// Première des langues préférées de macOS que l'installeur connaît (`fr-FR` → `fr`).
#[cfg(target_os = "macos")]
fn system_language() -> Option<usize> {
    crate::sys::preferred_languages().iter().find_map(|tag| from_tag(tag))
}

/// Rang dans `LANGUAGES` d'une étiquette BCP 47 (`pt-BR`, `zh-Hans-CN`).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn from_tag(tag: &str) -> Option<usize> {
    let code = tag.split(['-', '_']).next()?.to_ascii_lowercase();
    LANGUAGES.iter().position(|l| l.code == code)
}

pub fn init() {
    let saved = choice_file().and_then(|p| std::fs::read_to_string(p).ok());
    let index = match saved {
        Some(code) => LANGUAGES.iter().position(|l| l.code == code.trim()),
        None => system_language(),
    };
    CURRENT.store(index.unwrap_or(DEFAULT), Ordering::Relaxed);
}

pub fn current() -> usize {
    CURRENT.load(Ordering::Relaxed)
}

pub fn set(index: usize) {
    CURRENT.store(index, Ordering::Relaxed);
    if let Some(path) = choice_file() {
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let _ = std::fs::write(path, LANGUAGES[index].code);
    }
}

/// Le texte dans la langue choisie.
pub fn t(texts: Texts) -> &'static str {
    texts[current()]
}

/// Le texte dans la langue choisie, chaque `{n}` remplacé par le n-ième argument (à partir de 1) :
/// une traduction peut les placer dans un autre ordre.
pub fn f(texts: Texts, args: &[&dyn std::fmt::Display]) -> String {
    fill(t(texts), args)
}

fn fill(text: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let end = after.find('}');
        let n = end.and_then(|e| after[..e].parse::<usize>().ok());
        match (end, n) {
            (Some(e), Some(n)) if n >= 1 && n <= args.len() => {
                out.push_str(&args[n - 1].to_string());
                rest = &after[e + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn langues_dans_l_ordre_des_textes() {
        let codes: Vec<&str> = crate::textes::LANGUES.iter().map(|l| &l[..2]).collect();
        let ours: Vec<&str> = LANGUAGES.iter().map(|l| l.code).collect();
        assert_eq!(codes, ours);
    }

    #[test]
    fn locale_vers_langue() {
        assert_eq!(from_locale("fr_FR.UTF-8"), Some(1));
        assert_eq!(from_locale("pt_BR"), Some(8));
        assert_eq!(from_locale("zh_CN@x"), Some(10));
        assert_eq!(from_locale("de"), Some(2));
        assert_eq!(from_locale("C"), None);
        assert_eq!(from_locale(""), None);
    }

    #[test]
    fn ordre_des_variables_de_locale() {
        let env = |v: &str| match v {
            "LC_ALL" => Some("".into()),
            "LC_MESSAGES" => Some("it_IT.UTF-8".into()),
            "LANG" => Some("fr_FR".into()),
            _ => None,
        };
        assert_eq!(system_locale(&env).as_deref(), Some("it_IT.UTF-8"));
    }

    #[test]
    fn etiquettes_du_systeme() {
        assert_eq!(from_tag("fr-FR"), Some(1));
        assert_eq!(from_tag("pt-BR"), Some(8));
        assert_eq!(from_tag("zh-Hans-CN"), Some(10));
        assert_eq!(from_tag("nl-NL"), None);
    }

    #[test]
    fn parametres_numerotes() {
        assert_eq!(fill("{2} puis {1}", &[&"a", &"b"]), "b puis a");
        assert_eq!(fill("{1} {3} {x}", &[&1]), "1 {3} {x}");
    }

    #[test]
    fn memes_parametres_dans_chaque_langue() {
        for texts in [crate::textes::DOWNLOAD_OF_X_X, crate::textes::GAME_X] {
            let en: Vec<usize> = (1..=9).filter(|n| texts[0].contains(&format!("{{{n}}}"))).collect();
            for text in texts {
                let other: Vec<usize> = (1..=9).filter(|n| text.contains(&format!("{{{n}}}"))).collect();
                assert_eq!(en, other, "{text}");
            }
        }
    }
}
