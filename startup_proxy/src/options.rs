//! Onglet « BetterGameDLLs » de l'écran d'options du jeu : il édite `rollover_proxy.cfg`, le fichier de
//! réglages du proxy de GameCore.
//!
//! L'écran d'options est le même contexte `Options.xml` / `Options.lua` au menu principal et en jeu.
//! Le proxy le modifie au moment où l'exe lit ses fichiers, avec les accroches communes de
//! `proxy/src/exe.rs`, posées dès DllMain :
//! - **XML.** Après la lecture d'`Options.xml`, on ajoute un bouton d'onglet `ProxyTab` sous
//!   `AppTab`, et un panneau `ProxyOptions` à côté des autres (`options_proxy.xml`). Les lignes de
//!   réglage réutilisent les instances du fichier (`PullDownParameterInstance`, `KeyBindingCategory`).
//! - **Lua.** On ajoute `options_proxy.lua` à la fin d'`Options.lua`, pour lire ses variables
//!   locales (`m_tabs`). Avant de le compiler, on pose dans la table globale du contexte deux
//!   fonctions natives : `Civ6Proxy_LireReglages()` et `Civ6Proxy_EcrireReglages(lignes)`.
//!
//! Les réglages ne sont lus par le proxy de GameCore qu'à son chargement, au lancement de la
//! première partie (`gamecore_config_read`). Avant, un changement prend effet à la prochaine
//! partie ; après, au redémarrage du jeu, et la confirmation affiche alors le message du jeu
//! « redémarrage nécessaire ». L'onglet le dit, en français si le jeu l'est, en anglais sinon.

use crate::exe::{self, iat_slot, is_file, replace_buffer, write_slot, GameBuf, ReadXml};
use crate::{log, win};
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicUsize, Ordering};

pub const LUA_BLOCK: &str = include_str!("options_proxy.lua");
const XML_TAB: &str = include_str!("options_proxy_tab.xml");
const XML_PANEL: &str = include_str!("options_proxy_panel.xml");

/// Points d'insertion dans Options.xml : après le bouton d'onglet `AppTab`, et avant le panneau
/// des raccourcis clavier, dans le conteneur des panneaux.
const TAB_ANCHOR: &str = r#"<GridButton ID="AppTab""#;
const PANEL_ANCHOR: &str = r#"<Container ID="KeyBindings""#;

pub const CFG_NAME: &str = "rollover_proxy.cfg";
const GAMECORE_ORIGINAL: &str = "GameCore_XP2_FinalRelease_orig.dll";

static REAL_LOADBUFFER: AtomicUsize = AtomicUsize::new(0);
static XML_TRAMP: AtomicUsize = AtomicUsize::new(0);

// ---------------------------------------------------------------------------------------------
// Textes, sans état : testés sur les vrais fichiers (voir les tests en bas).

fn script_name(chunk: &str) -> String {
    chunk.rsplit(['/', '\\']).next().unwrap_or("").trim_start_matches('@').to_ascii_lowercase()
}

/// Options.xml avec le bouton d'onglet et le panneau.
pub fn patch_options_xml(src: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(src).map_err(|_| "texte non UTF-8".to_string())?;
    if text.contains(r#"ID="ProxyTab""#) {
        return Err("onglet déjà présent".into());
    }
    let tab = text.find(TAB_ANCHOR).ok_or("bouton AppTab introuvable")?;
    let tab_end = text[tab..].find("/>").ok_or("bouton AppTab non fermé")? + tab + 2;
    let panel = text.find(PANEL_ANCHOR).ok_or("panneau KeyBindings introuvable")?;
    if panel < tab_end {
        return Err("panneau KeyBindings avant les onglets".into());
    }
    let mut out = String::with_capacity(text.len() + XML_TAB.len() + XML_PANEL.len() + 8);
    out.push_str(&text[..tab_end]);
    out.push_str("\r\n");
    out.push_str(XML_TAB.trim_end());
    out.push_str(&text[tab_end..panel]);
    out.push_str(XML_PANEL.trim_end());
    out.push_str("\r\n");
    out.push_str(&text[panel..]);
    Ok(out.into_bytes())
}

/// Options.lua suivi du bloc de l'onglet. Octets laissés tels quels : les fichiers du jeu ne sont
/// pas tous en UTF-8.
pub fn patch_options_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(LUA_BLOCK.as_bytes());
    out
}

/// Le fichier de réglages avec les `changes` (`clé=valeur`, une par ligne) : chaque clé prend sa
/// nouvelle valeur à sa première ligne et perd ses lignes suivantes, les autres lignes restent
/// telles quelles, et les clés absentes sont ajoutées à la fin.
pub fn merge_config(existing: &str, changes: &str) -> Result<String, String> {
    let mut pending: Vec<(String, String)> = Vec::new();
    for line in changes.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (k, v) = line.split_once('=').ok_or_else(|| format!("ligne sans « = » : {line}"))?;
        let (k, v) = (k.trim(), v.trim());
        if k.is_empty() || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(format!("clé invalide : {k}"));
        }
        if !v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.') {
            return Err(format!("valeur invalide pour {k} : {v}"));
        }
        pending.retain(|(pk, _)| pk != k);
        pending.push((k.to_string(), v.to_string()));
    }
    // read_config garde la dernière occurrence d'une clé : les suivantes de chaque clé changée sont
    // retirées.
    let mut done: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for line in existing.lines() {
        let key = line.split_once('=').map(|(k, _)| k.trim());
        if key.is_some_and(|k| done.iter().any(|d| d == k)) {
            continue;
        }
        match key.and_then(|k| pending.iter().position(|(pk, _)| pk == k)) {
            Some(i) => {
                let (k, v) = pending.remove(i);
                out.push(format!("{k}={v}"));
                done.push(k);
            }
            None => out.push(line.to_string()),
        }
    }
    out.extend(pending.into_iter().map(|(k, v)| format!("{k}={v}")));
    let mut text = out.join("\r\n");
    text.push_str("\r\n");
    Ok(text)
}

// ---------------------------------------------------------------------------------------------
// Fichiers.

/// Dossier du proxy de GameCore, `DLC\Expansion2\Binaries\Win64`, depuis celui de l'exe
/// (`Base\Binaries\Win64Steam`), où est posé ce proxy.
fn gamecore_dir() -> Option<std::path::PathBuf> {
    let exe_dir = crate::self_dir()?;
    Some(exe_dir.parent()?.parent()?.parent()?.join("DLC").join("Expansion2").join("Binaries").join("Win64"))
}

/// Le proxy de GameCore est-il chargé, et a-t-il déjà lu le fichier ? Il ne le lit qu'une fois, au
/// lancement de la première partie (`forward::real_module`), puis passe à `RolloverProxyStatus`
/// le bit 2 (originale chargée). Avant, un changement sera lu par la prochaine partie ; après, il
/// attend le redémarrage du jeu. L'originale seule n'a pas cet export.
pub fn gamecore_config_read() -> bool {
    const ST_ORIGINAL: u32 = 2;
    unsafe {
        let m = win::GetModuleHandleW(win::wide("GameCore_XP2_FinalRelease.dll").as_ptr());
        if m.is_null() {
            return false;
        }
        let p = win::GetProcAddress(m, b"RolloverProxyStatus\0".as_ptr());
        if p.is_null() {
            return false;
        }
        let status: extern "system" fn() -> u32 = std::mem::transmute(p);
        status() & ST_ORIGINAL != 0
    }
}

pub(crate) fn read_config() -> Result<(String, bool), String> {
    let dir = gamecore_dir().ok_or("dossier du jeu introuvable")?;
    let installed = dir.join(GAMECORE_ORIGINAL).exists();
    match std::fs::read_to_string(dir.join(CFG_NAME)) {
        Ok(text) => Ok((text, installed)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok((String::new(), installed)),
        Err(e) => Err(format!("lecture de {CFG_NAME} impossible : {e}")),
    }
}

fn write_config(changes: &str) -> Result<(), String> {
    let path = gamecore_dir().ok_or("dossier du jeu introuvable")?.join(CFG_NAME);
    let existing = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("lecture de {CFG_NAME} impossible : {e}")),
    };
    let text = merge_config(&existing, changes)?;
    std::fs::write(&path, text).map_err(|e| format!("écriture de {} impossible : {e}", path.display()))?;
    log(&format!("options : {CFG_NAME} enregistré ({})", changes.lines().collect::<Vec<_>>().join(", ")));
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Fonctions natives posées dans le contexte Lua des options.

/// HavokScript n'exporte ni `lua_pushlstring` ni `lua_pushboolean` : les chaînes passent par
/// `lua_pushfstring("%s")`, les booléens par `lua_pushinteger`.
const LUA_GLOBALSINDEX: i32 = -10002;
pub(crate) type LuaCFn = unsafe extern "C-unwind" fn(*mut c_void) -> i32;

pub(crate) struct Hks {
    push_fstring: unsafe extern "C-unwind" fn(*mut c_void, *const c_char, ...) -> *const c_char,
    pub(crate) push_integer: unsafe extern "C-unwind" fn(*mut c_void, i32),
    pub(crate) push_nil: unsafe extern "C-unwind" fn(*mut c_void),
    pub(crate) to_lstring: unsafe extern "C-unwind" fn(*mut c_void, i32, *mut usize) -> *const c_char,
    push_closure: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, i32, *const c_char, i32),
    set_field: unsafe extern "C-unwind" fn(*mut c_void, i32, *const c_char),
    cpcall: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, *mut c_void) -> i32,
    set_top: unsafe extern "C-unwind" fn(*mut c_void, i32),
}

static HKS: std::sync::OnceLock<Option<Hks>> = std::sync::OnceLock::new();

pub(crate) unsafe fn hks() -> Option<&'static Hks> {
    HKS.get_or_init(|| {
        let m = win::GetModuleHandleW(win::wide("HavokScript_FinalRelease.dll").as_ptr());
        if m.is_null() {
            return None;
        }
        let get = |name: &[u8]| {
            let p = win::GetProcAddress(m, name.as_ptr());
            (!p.is_null()).then_some(p)
        };
        Some(Hks {
            push_fstring: std::mem::transmute(get(b"?lua_pushfstring@@YAPEBDPEAUlua_State@@PEBDZZ\0")?),
            push_integer: std::mem::transmute(get(b"?lua_pushinteger@@YAXPEAUlua_State@@H@Z\0")?),
            push_nil: std::mem::transmute(get(b"?lua_pushnil@@YAXPEAUlua_State@@@Z\0")?),
            to_lstring: std::mem::transmute(get(b"?lua_tolstring@@YAPEBDPEAUlua_State@@HPEA_K@Z\0")?),
            push_closure: std::mem::transmute(get(b"?hks_pushnamedcclosure@@YAXPEAUlua_State@@P6AH0@ZHPEBDH@Z\0")?),
            set_field: std::mem::transmute(get(b"?lua_setfield@@YAXPEAUlua_State@@HPEBD@Z\0")?),
            cpcall: std::mem::transmute(get(b"?lua_cpcall@@YAHPEAUlua_State@@P6AH0@ZPEAX@Z\0")?),
            set_top: std::mem::transmute(get(b"?lua_settop@@YAXPEAUlua_State@@H@Z\0")?),
        })
    })
    .as_ref()
}

pub(crate) unsafe fn push_str(h: &Hks, l: *mut c_void, s: &str) {
    let c = std::ffi::CString::new(s.replace('\0', "")).unwrap_or_default();
    (h.push_fstring)(l, b"%s\0".as_ptr() as *const c_char, c.as_ptr());
}

/// `Civ6Proxy_LireReglages()` → texte du fichier (vide s'il n'existe pas), 1|0 selon que le proxy
/// de GameCore est installé, 1|0 selon qu'il a déjà lu le fichier ; ou nil et le message d'erreur.
unsafe extern "C-unwind" fn lua_read(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    match read_config() {
        Ok((text, installed)) => {
            push_str(h, l, &text);
            (h.push_integer)(l, installed as i32);
            (h.push_integer)(l, gamecore_config_read() as i32);
            3
        }
        Err(e) => {
            (h.push_nil)(l);
            push_str(h, l, &e);
            2
        }
    }
}

/// `Civ6Proxy_EcrireReglages(lignes)`, lignes `clé=valeur` → 1 et 1|0 selon que le proxy de
/// GameCore a déjà lu le fichier (redémarrage nécessaire) ; ou nil et le message d'erreur.
unsafe extern "C-unwind" fn lua_write(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let mut len = 0usize;
    let p = (h.to_lstring)(l, 1, &mut len);
    let result = if p.is_null() {
        Err("argument attendu : une chaîne".to_string())
    } else {
        write_config(&String::from_utf8_lossy(std::slice::from_raw_parts(p as *const u8, len)))
    };
    match result {
        Ok(()) => {
            (h.push_integer)(l, 1);
            (h.push_integer)(l, gamecore_config_read() as i32);
            2
        }
        Err(e) => {
            log(&format!("options : {e}"));
            (h.push_nil)(l);
            push_str(h, l, &e);
            2
        }
    }
}

/// Fonctions natives d'un contexte : la fonction et son nom global, terminé par un zéro.
pub(crate) type Natives = &'static [(LuaCFn, &'static [u8])];

const OPTIONS_NATIVES: Natives =
    &[(lua_read, b"Civ6Proxy_LireReglages\0"), (lua_write, b"Civ6Proxy_EcrireReglages\0")];

thread_local! {
    /// Les fonctions à poser, le temps d'un `lua_cpcall` : sa fonction ne reçoit que l'état Lua.
    static PENDING: std::cell::Cell<Natives> = const { std::cell::Cell::new(&[]) };
}

unsafe extern "C-unwind" fn install_natives(l: *mut c_void) -> i32 {
    if let Some(h) = hks() {
        for &(f, name) in PENDING.with(|p| p.get()) {
            (h.push_closure)(l, f, 0, name.as_ptr() as *const c_char, 0);
            (h.set_field)(l, LUA_GLOBALSINDEX, name.as_ptr() as *const c_char);
        }
    }
    0
}

/// Pose les fonctions natives dans la table globale du contexte, par `lua_cpcall` : si la table
/// refuse l'écriture, l'erreur reste dans Lua, et le bloc qui s'en sert le signale. `what` préfixe
/// le journal.
pub(crate) unsafe fn publish_natives(l: *mut c_void, natives: Natives, what: &str) -> bool {
    let Some(h) = hks() else {
        log(&format!("{what} : fonctions HavokScript introuvables, natives non posées"));
        return false;
    };
    PENDING.with(|p| p.set(natives));
    let r = (h.cpcall)(l, install_natives, std::ptr::null_mut());
    PENDING.with(|p| p.set(&[]));
    if r != 0 {
        // lua_cpcall laisse l'erreur sur la pile.
        (h.set_top)(l, -2);
        log(&format!("{what} : fonctions natives non posées ({r})"));
    }
    r == 0
}

// ---------------------------------------------------------------------------------------------
// Accroches.

type LoadBuffer = unsafe extern "C" fn(*mut c_void, *const u8, usize, *const c_char) -> i32;

unsafe extern "C" fn hook_loadbuffer(l: *mut c_void, buf: *const u8, size: usize, name: *const c_char) -> i32 {
    let real: LoadBuffer = std::mem::transmute(REAL_LOADBUFFER.load(Ordering::Relaxed));
    if name.is_null() || buf.is_null() {
        return real(l, buf, size, name);
    }
    // Premier script de l'exe, bien avant le menu principal : la vérification a le temps d'aboutir.
    crate::maj::start();
    let chunk = CStr::from_ptr(name).to_string_lossy();
    let src = std::slice::from_raw_parts(buf, size);
    let (text, what) = match script_name(&chunk).as_str() {
        "options.lua" => {
            publish_natives(l, OPTIONS_NATIVES, "options");
            (patch_options_lua(src), "options : onglet des proxys ajouté")
        }
        crate::salon::SCRIPT => (crate::salon::prepare(l, src), "salon : annonce de version ajoutée"),
        crate::salon::SCRIPT_PARTIE => (crate::salon::prepare_partie(l, src), "partie : relais du verdict ajouté"),
        crate::salon::SCRIPT_CHARGEMENT => {
            (crate::salon::prepare_chargement(l, src), "chargement : correctifs POL des parties locales ajoutés")
        }
        crate::maj::SCRIPT => (crate::maj::prepare(l, src), "maj : notification de mise à jour ajoutée"),
        crate::salon::SCRIPT_OPTION => (crate::salon::prepare_option(l, src), "option : « No first moves (BGD) » ajoutée"),
        _ => return real(l, buf, size, name),
    };
    let r = real(l, text.as_ptr(), text.len(), name);
    let (prefix, rest) = what.split_once(" : ").unwrap_or(("", what));
    log(&format!(
        "{prefix} : {chunk} : {rest}, compilation {}",
        if r == 0 { "réussie".to_string() } else { format!("en échec ({r})") }
    ));
    r
}

unsafe extern "C" fn hook_read_xml(path: *const u16, buf: *mut GameBuf) -> i32 {
    let real: ReadXml = std::mem::transmute(XML_TRAMP.load(Ordering::Relaxed));
    let r = real(path, buf);
    if r != 0 || path.is_null() || buf.is_null() {
        return r;
    }
    let n = (0..).take_while(|&i| *path.add(i) != 0).count();
    let name = String::from_utf16_lossy(std::slice::from_raw_parts(path, n));
    if is_file(&name, crate::maj::XML) {
        let b = &mut *buf;
        let Some(src) = b.text() else { return r };
        match crate::maj::patch_main_menu_xml(src) {
            Ok(None) => {}
            Ok(Some(text)) if replace_buffer(b, &text) => log(&format!("maj : {name} : popup du menu ajouté")),
            Ok(Some(_)) => log(&format!("maj : {name} : allocation impossible, laissé tel quel")),
            Err(e) => log(&format!("maj : {name} laissé tel quel : {e}")),
        }
        return r;
    }
    if !is_file(&name, "Options.xml") {
        return r;
    }
    let b = &mut *buf;
    let Some(src) = b.text() else { return r };
    match patch_options_xml(src) {
        Ok(text) if replace_buffer(b, &text) => log(&format!("options : {name} : onglet des proxys ajouté")),
        Ok(_) => log(&format!("options : {name} : allocation impossible, laissé tel quel")),
        Err(e) => log(&format!("options : {name} laissé tel quel : {e}")),
    }
    r
}

/// Pose les deux accroches. Appelée par DllMain : l'exe et ses imports sont déjà projetés et
/// résolus, mais son code n'a pas encore tourné.
pub unsafe fn install() {
    let exe = win::GetModuleHandleW(std::ptr::null()) as usize;
    let Some(slot) = iat_slot(exe, "HavokScript_FinalRelease.dll", "luaL_loadbuffer") else {
        log("options : import luaL_loadbuffer introuvable dans l'exe, pas d'onglet");
        return;
    };
    // L'entrée doit déjà viser HavokScript : sinon la table d'imports n'est pas encore résolue, et
    // le chargeur écraserait l'accroche.
    let havok = win::GetModuleHandleW(win::wide("HavokScript_FinalRelease.dll").as_ptr());
    let expected = if havok.is_null() {
        0
    } else {
        win::GetProcAddress(havok, b"?luaL_loadbuffer@@YAHPEAUlua_State@@PEBD_K1@Z\0".as_ptr()) as usize
    };
    let current = *(slot as *const usize);
    if expected == 0 || current != expected {
        log(&format!("options : import luaL_loadbuffer pas encore résolu ({current:#x}, attendu {expected:#x}), pas d'onglet"));
        return;
    }
    match exe::hook_read_xml(exe, hook_read_xml as *const () as usize) {
        Ok(h) => XML_TRAMP.store(h.tramp, Ordering::Relaxed),
        Err(e) => {
            log(&format!("options : {e}, pas d'onglet"));
            return;
        }
    }
    REAL_LOADBUFFER.store(current, Ordering::Relaxed);
    if !write_slot(slot, hook_loadbuffer as *const () as usize) {
        log("options : VirtualProtect a échoué sur la table d'imports, onglet sans réglages");
        return;
    }
    log("options : accroches posées (lecture XML, luaL_loadbuffer)");
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI";

    fn read(path: String) -> Option<Vec<u8>> {
        let r = std::fs::read(&path).ok();
        if r.is_none() {
            eprintln!("absent, test sauté : {path}");
        }
        r
    }

    #[test]
    fn xml_du_jeu() {
        let Some(src) = read(format!(r"{GAME}\Base\Assets\UI\Options.xml")) else { return };
        let new = String::from_utf8(patch_options_xml(&src).unwrap()).unwrap();
        // Le bouton suit AppTab, dans la pile des onglets ; le panneau précède KeyBindings.
        let (app, tab, stack_end) = (new.find(TAB_ANCHOR).unwrap(), new.find(r#"ID="ProxyTab""#).unwrap(), new.find("</Stack>").unwrap());
        assert!(app < tab && tab < stack_end);
        assert!(new.find(r#"ID="ProxyOptions""#).unwrap() < new.find(PANEL_ANCHOR).unwrap());
        for (open, close) in [("<Container", "</Container>"), ("<ScrollPanel", "</ScrollPanel>"), ("<Stack", "</Stack>")] {
            let old = String::from_utf8_lossy(&src);
            assert_eq!(new.matches(open).count() - old.matches(open).count(), new.matches(close).count() - old.matches(close).count(), "{open}");
        }
        assert!(patch_options_xml(&new.into_bytes()).is_err(), "pas deux fois");
        assert!(patch_options_xml(b"<Context></Context>").is_err());
    }

    #[test]
    fn noms_des_scripts() {
        assert_eq!(script_name(r"C:\x\Base\Assets\UI\Options.lua"), "options.lua");
        assert_eq!(script_name("@Options.lua"), "options.lua");
        assert_ne!(script_name("C:/x/InGameTopOptionsMenu.lua"), "options.lua");
        assert!(patch_options_lua(b"x = 1").starts_with(b"x = 1\r\n-- "));
    }

    #[test]
    fn fusion_du_fichier() {
        let old = "mode=coalesce\r\n# note\r\nui_boutons=on\r\nwmd_accel=20\r\n";
        let new = merge_config(old, "ui_boutons=off\nnouveau=on\n").unwrap();
        assert_eq!(new, "mode=coalesce\r\n# note\r\nui_boutons=off\r\nwmd_accel=20\r\nnouveau=on\r\n");
        assert_eq!(merge_config("", "a=1").unwrap(), "a=1\r\n");
        assert_eq!(merge_config("a=1\nb=0\na=2\n", "a=3").unwrap(), "a=3\r\nb=0\r\n", "les doublons d'une clé changée disparaissent");
        assert_eq!(merge_config("b=0\nb=1\n", "a=3").unwrap(), "b=0\r\nb=1\r\na=3\r\n", "ceux des autres restent");
        assert!(merge_config(old, "sans_egal").is_err());
        assert!(merge_config(old, "a b=1").is_err());
        assert!(merge_config(old, "a=1\nb=x;y").is_err());
        assert_eq!(merge_config(old, "").unwrap(), old);
    }

    /// Chaque clé de l'onglet est lue par le proxy de GameCore (`read_config`, dans proxy/src/lib.rs),
    /// et chaque réglage qu'il lit figure dans l'onglet.
    #[test]
    fn cles_communes_avec_le_proxy() {
        // Clés lues par le proxy de GameCore, et par celui-ci (verif_maj).
        let mut lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../proxy/src/lib.rs")).unwrap();
        lib += include_str!("maj.rs");
        let read: Vec<&str> = lib.split("strip_prefix(\"").skip(1).map(|s| s.split("=\")").next().unwrap()).collect();
        let tab: Vec<&str> = LUA_BLOCK.split("cle = \"").skip(1).map(|s| s.split('"').next().unwrap()).collect();
        assert!(read.len() >= 12, "{read:?}");
        for k in &read {
            assert!(tab.contains(k), "{k} lu par le proxy, absent de l'onglet");
        }
        for k in &tab {
            assert!(read.contains(k), "{k} dans l'onglet, inconnu du proxy");
        }
    }
}
