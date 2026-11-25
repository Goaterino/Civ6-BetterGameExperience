//! Textes des mods en jeu : capture de ce qu'UpdateText écrit dans la base de localisation, et
//! remise de ce delta au chargement suivant au lieu de rejouer UpdateText (1,6 s). Utilisé par
//! `bdd`, qui décide quand.
//!
//! La base de localisation (`:memory:`) garde ouvert le savepoint `Origin`, l'état construit depuis
//! `Base/Assets/Text` (textes vanilla) : chaque passage par le salon et chaque `UpdateContent`
//! y reviennent (`ROLLBACK TO Origin`, `1400c1f60`). Le salon y écrit ses propres textes, d'où la
//! remise à chaque chargement, puis UpdateText y réécrit tous les textes en jeu des mods.
//!
//! Capture (chargement complet) : juste après la remise à `Origin`, une empreinte du schéma et
//! des sept autres tables, et une copie de `LocalizedText` en mémoire. Après les composants, si
//! l'empreinte n'a pas changé, la table est relue et comparée à cette copie : les lignes ajoutées
//! ou modifiées (184 428 sur 360 480 avec BBG, extensions comprises : elles sont chargées comme des
//! mods), dans l'ordre de la clé, et les clés supprimées. Pas de table SQL temporaire : créée dans
//! le savepoint `Origin`, jamais validé, elle ferait couper par chaque `ROLLBACK TO Origin` les
//! lectures en cours du jeu et expirer ses requêtes préparées (SQLite 3.13 du DatabaseDLL). Une
//! première version notait les clés touchées par des déclencheurs (0,7 s de plus pendant les
//! composants) ; une deuxième relisait et réinsérait toute la table, 0,6 s de plus au rejeu
//! (capture `20261117-141910-sans-vsync`).
//!
//! Remise (chargement suivant, mêmes actions) : après la remise à `Origin`, les clés supprimées
//! sont effacées, puis les lignes du delta écrites dans l'ordre de la clé, par des requêtes
//! préparées. L'état obtenu est exactement celui de la fin de la capture, ce que vérifie une
//! empreinte (nombre de lignes, longueurs).
//!
//! API des bases du jeu (`DatabaseDLL`), vue depuis l'exe et GameCore : connexion +0x18 préparer
//! (sql, longueur), +0x20 exécuter une instruction, +0xa0 revenir à un savepoint ; requête +0x8
//! libérer, +0x10 exécuter, +0x20 avancer (vrai s'il y a une ligne), +0x38 réinitialiser,
//! +0x90 lier un texte (index, texte, longueur), +0xf0 lire une colonne en texte.

use crate::log;
use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};

// Les NULL passent par char(1) dans les deux sens : `ifnull(x, char(1))` à la lecture,
// `NULLIF(?, char(1))` à l'écriture (l'API ne lie pas de NULL).

pub struct Connexion(pub usize);

impl Connexion {
    /// Première colonne de la première ligne d'une requête, en texte.
    pub unsafe fn valeur(&self, sql: &str) -> Option<CString> {
        let r = self.preparer(sql)?;
        if !r.avancer() {
            return None;
        }
        r.texte(0)
    }

    unsafe fn f(&self, off: usize) -> usize {
        *((*(self.0 as *const usize)) as *const usize).byte_add(off)
    }

    unsafe fn preparer(&self, sql: &str) -> Option<Requete> {
        let c = CString::new(sql).ok()?;
        let f: unsafe extern "C" fn(usize, *const c_char, i32) -> usize = std::mem::transmute(self.f(0x18));
        let r = f(self.0, c.as_ptr(), -1);
        (r != 0).then_some(Requete(r))
    }

    pub unsafe fn revenir_a(&self, savepoint: &CStr) -> bool {
        let f: unsafe extern "C" fn(usize, *const c_char) -> u8 = std::mem::transmute(self.f(0xa0));
        f(self.0, savepoint.as_ptr()) != 0
    }
}

struct Requete(usize);

impl Requete {
    unsafe fn f(&self, off: usize) -> usize {
        *((*(self.0 as *const usize)) as *const usize).byte_add(off)
    }
    unsafe fn avancer(&self) -> bool {
        let f: unsafe extern "C" fn(usize) -> u8 = std::mem::transmute(self.f(0x20));
        f(self.0) != 0
    }
    /// Valeur de retour inconnue (jamais testée par le jeu) : le résultat se vérifie à part.
    unsafe fn executer(&self) {
        let f: unsafe extern "C" fn(usize) = std::mem::transmute(self.f(0x10));
        f(self.0)
    }
    unsafe fn reinitialiser(&self) {
        let f: unsafe extern "C" fn(usize) = std::mem::transmute(self.f(0x38));
        f(self.0)
    }
    /// Valeur de retour inconnue, comme `executer`. Le quatrième argument est la longueur : -1,
    /// jusqu'au zéro final, comme le passe l'exe (`or r9d,-1` avant l'appel, `140b425aa`). Sans
    /// lui, la longueur était un reste de registre (crash du remap du 2026-11-17).
    unsafe fn lier(&self, i: i32, v: &CStr) {
        let f: unsafe extern "C" fn(usize, i32, *const c_char, i32) = std::mem::transmute(self.f(0x90));
        f(self.0, i, v.as_ptr(), -1)
    }
    unsafe fn texte(&self, col: i32) -> Option<CString> {
        let f: unsafe extern "C" fn(usize, i32) -> *const c_char = std::mem::transmute(self.f(0xf0));
        let p = f(self.0, col);
        (!p.is_null()).then(|| CStr::from_ptr(p).to_owned())
    }
}

impl Drop for Requete {
    fn drop(&mut self) {
        unsafe {
            self.reinitialiser();
            let f: unsafe extern "C" fn(usize) = std::mem::transmute(self.f(0x8));
            f(self.0)
        }
    }
}

/// Les sept autres tables du schéma d'origine, chacune avec ses colonnes : leur contenu entre
/// dans l'empreinte « avant / après ». Un mod qui y écrit rend la capture inutilisable.
const AUTRES: [(&str, &str); 7] = [
    ("Languages", "quote(Locale)||quote(Name)||quote(Collator)||quote(PluralRule)||quote(SupportsSmallCaps)"),
    ("AudioLanguages", "quote(AudioLanguage)||quote(Locale)"),
    ("DefaultAudioLanguages", "quote(Locale)||quote(AudioLanguage)"),
    ("SteamLanguages", "quote(SteamLanguage)||quote(Locale)"),
    ("EpicLanguages", "quote(EpicLanguage)||quote(Locale)"),
    ("LanguagePriorities", "quote(PrimaryLanguage)||quote(Language)||quote(Priority)"),
    ("FontStyleSheets", "quote(Language)||quote(StyleSheet)||quote(StyleOverride)||quote(DefaultFont)"),
];

/// Ce qu'UpdateText change dans `LocalizedText` par rapport à `Origin`.
pub struct Delta {
    /// Empreinte de `LocalizedText` à la fin de la capture, vérifiée après la remise.
    empreinte: CString,
    /// Lignes ajoutées ou modifiées, dans l'ordre de la clé primaire : Language, Tag, Text,
    /// Gender, Plurality (char(1) pour NULL).
    lignes: Vec<[CString; 5]>,
    /// Clés (Language, Tag) présentes dans `Origin` et supprimées.
    suppressions: Vec<[CString; 2]>,
}

impl Delta {
    pub fn taille(&self) -> usize {
        self.lignes.len()
    }
    pub fn suppressions(&self) -> usize {
        self.suppressions.len()
    }
}

/// Capture en cours : l'empreinte hors textes et `LocalizedText` à l'état `Origin`, en mémoire
/// (une table SQL temporaire dans le savepoint ouvert gênerait les requêtes du jeu).
pub struct Capture {
    avant: CString,
    origine: HashMap<(CString, CString), [CString; 3]>,
}

/// Nombre de lignes et longueur cumulée des textes : de quoi voir une remise incomplète.
unsafe fn empreinte(c: &Connexion) -> Option<CString> {
    c.valeur(
        "SELECT count(*) || '/' || total(length(Language) + length(Tag) + ifnull(length(Text), 0) \
         + ifnull(length(Gender), 0) + ifnull(length(Plurality), 0)) FROM main.LocalizedText",
    )
}

/// Schéma et contenu des sept autres tables.
unsafe fn hors_textes(c: &Connexion) -> Option<CString> {
    let mut sql = String::from("SELECT (SELECT count(*) || '/' || total(length(sql)) FROM main.sqlite_master)");
    for (t, cols) in AUTRES {
        sql += &format!(" || '|' || ifnull((SELECT group_concat(x, ',') FROM (SELECT {cols} AS x FROM main.{t} ORDER BY rowid)), '')");
    }
    c.valeur(&sql)
}

/// Juste après la remise à `Origin` : l'empreinte de ce qui doit rester inchangé, et une copie de
/// `LocalizedText` en mémoire, à laquelle comparer la table après les composants.
pub unsafe fn commencer_capture(c: &Connexion) -> Option<Capture> {
    let Some(avant) = hors_textes(c) else {
        log("localization: localization schema not readable, mod texts will be rebuilt next time");
        return None;
    };
    let mut origine = HashMap::new();
    let ok = lire_table(c, |[a, b, t, g, p]| {
        origine.insert((a, b), [t, g, p]);
    });
    if !ok {
        log("localization: Origin texts not readable, mod texts will be rebuilt next time");
        return None;
    }
    Some(Capture { avant, origine })
}

/// Toute la table, dans l'ordre de la clé primaire, NULL lus en char(1). Faux si une ligne est
/// illisible.
unsafe fn lire_table(c: &Connexion, mut f: impl FnMut([CString; 5])) -> bool {
    let Some(r) = c.preparer(
        "SELECT Language, Tag, ifnull(Text, char(1)), ifnull(Gender, char(1)), ifnull(Plurality, char(1)) \
         FROM main.LocalizedText ORDER BY Language, Tag",
    ) else {
        return false;
    };
    while r.avancer() {
        let (Some(a), Some(b), Some(t), Some(g), Some(p)) = (r.texte(0), r.texte(1), r.texte(2), r.texte(3), r.texte(4)) else {
            return false;
        };
        f([a, b, t, g, p]);
    }
    true
}

/// Après les composants : ce qui a changé depuis `Origin`, si rien d'autre n'a changé. `None`
/// sinon.
pub unsafe fn lire_capture(c: &Connexion, cap: Capture) -> Option<Delta> {
    if hors_textes(c).as_deref() != Some(cap.avant.as_c_str()) {
        log("localization: a mod changed the localization schema or wrote outside LocalizedText, mod texts will be rebuilt every time");
        return None;
    }
    let mut origine = cap.origine;
    let mut d = Delta { empreinte: empreinte(c)?, lignes: Vec::new(), suppressions: Vec::new() };
    // Lues dans l'ordre de la clé primaire : réécrites dans cet ordre, l'arbre se remplit
    // presque deux fois plus vite qu'en désordre.
    let ok = lire_table(c, |[a, b, t, g, p]| {
        let cle = (a, b);
        let pareil = origine.remove(&cle).is_some_and(|o| o == [t.clone(), g.clone(), p.clone()]);
        if !pareil {
            d.lignes.push([cle.0, cle.1, t, g, p]);
        }
    });
    if !ok {
        log("localization: unreadable row, mod texts will be rebuilt");
        return None;
    }
    d.suppressions = origine.into_keys().map(|(a, b)| [a, b]).collect();
    d.suppressions.sort();
    Some(d)
}

/// Applique le delta à `LocalizedText`, qui vient d'être remise à `Origin`. En cas d'écart
/// d'empreinte, la base est ramenée à `Origin` et l'appelant laisse UpdateText faire.
pub unsafe fn rejouer(c: &Connexion, d: &Delta) -> bool {
    let ok = (|| {
        let sup = c.preparer("DELETE FROM main.LocalizedText WHERE Language = ? AND Tag = ?")?;
        for cle in &d.suppressions {
            sup.lier(1, &cle[0]);
            sup.lier(2, &cle[1]);
            sup.executer();
            sup.reinitialiser();
        }
        drop(sup);
        let ins = c.preparer(
            "INSERT OR REPLACE INTO main.LocalizedText(Language, Tag, Text, Gender, Plurality) \
             VALUES(?, ?, NULLIF(?, char(1)), NULLIF(?, char(1)), NULLIF(?, char(1)))",
        )?;
        for ligne in &d.lignes {
            for (i, v) in ligne.iter().enumerate() {
                ins.lier(i as i32 + 1, v);
            }
            ins.executer();
            ins.reinitialiser();
        }
        drop(ins);
        (empreinte(c)? == d.empreinte).then_some(())
    })()
    .is_some();
    if !ok {
        log("localization: replaying the mod texts failed, back to Origin and UpdateText");
        c.revenir_a(c"Origin");
    }
    ok
}
