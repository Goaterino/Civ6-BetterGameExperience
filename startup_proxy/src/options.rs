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
//! - **Limiteur d'images.** La liste du jeu (`tickInterval_options` d'`Options.lua` : désactivé,
//!   20, 30, 60) gagne 45, 75, 90 et 100 images/s. L'exe applique `TickIntervalInMS` tel quel
//!   (`Sleep` du reste de l'intervalle, en millisecondes entières, minuteur à 1 ms) : une image dure
//!   l'intervalle plus 0,5 à 1,5 ms. Comme le jeu (49, 32, 16), on retient donc 1000 / n − 1, arrondi.
//!
//! Les réglages ne sont lus par le proxy de GameCore qu'à son chargement, au lancement de la
//! première partie (`gamecore_config_read`). Avant, un changement prend effet à la prochaine
//! partie ; après, au redémarrage du jeu, et la confirmation affiche alors le message du jeu
//! « redémarrage nécessaire ». L'onglet le dit, en français si le jeu l'est, en anglais sinon.

use crate::exe::{self, iat_slot, is_file, replace_buffer, write_slot, GameBuf, ReadXml};
use crate::{log, win};
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicUsize, Ordering};

pub const LUA_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("options_proxy.lua"));
const XML_TAB: &str = include_str!("options_proxy_tab.xml");
const XML_PANEL: &str = include_str!("options_proxy_panel.xml");

/// Points d'insertion dans Options.xml : après le bouton d'onglet `AppTab`, et avant le panneau
/// des raccourcis clavier, dans le conteneur des panneaux.
const TAB_ANCHOR: &str = r#"<GridButton ID="AppTab""#;
const PANEL_ANCHOR: &str = r#"<Container ID="KeyBindings""#;
/// Bouton « Réinitialiser BGE », juste après le bouton Confirmer, sous les panneaux : caché, sauf sur
/// l'onglet BGE, où options_proxy.lua place les deux côte à côte. Sans Confirmer, pas de bouton.
const CONFIRM_ANCHOR: &str = r#"<GridButton ID="ConfirmButton""#;
const XML_RESET: &str = r#"<GridButton ID="ProxyResetButton" Style="ButtonRed" AnchorSide="I,O" Anchor="c,b" Size="300,41" String="" Hidden="1"/>"#;

pub const CFG_NAME: &str = "rollover_proxy.cfg";

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
    let confirm_end = text[panel..]
        .find(CONFIRM_ANCHOR)
        .and_then(|c| text[panel + c..].find("/>").map(|e| panel + c + e + 2));
    let mut out = String::with_capacity(text.len() + XML_TAB.len() + XML_PANEL.len() + XML_RESET.len() + 16);
    out.push_str(&text[..tab_end]);
    out.push_str("\r\n");
    out.push_str(XML_TAB.trim_end());
    out.push_str(&text[tab_end..panel]);
    out.push_str(XML_PANEL.trim_end());
    out.push_str("\r\n");
    match confirm_end {
        Some(end) => {
            out.push_str(&text[panel..end]);
            out.push_str("\r\n\t\t\t\t");
            out.push_str(XML_RESET);
            out.push_str(&text[end..]);
        }
        None => out.push_str(&text[panel..]),
    }
    Ok(out.into_bytes())
}

/// Ligne du bloc qui dit s'il montre les réglages de diagnostic (`diag = true`) : réécrite en
/// `true` quand le proxy est compilé avec la feature `diag` (builds locaux, pas la release).
const DIAG_LINE: &str = "local PROXY_DIAG = false;";

/// Ligne du bloc qui porte la version affichée dans le titre de l'onglet : réécrite avec celle de la
/// release (`version::RELEASE`), ou celle du salon (`version::VERSION`) pour une compilation locale.
const VERSION_LINE: &str = "local PROXY_VERSION = nil;";

/// Le bloc de l'onglet : version dans le titre, réglages de diagnostic compris avec la feature `diag`.
fn lua_block() -> String {
    let version = crate::version::RELEASE.unwrap_or(crate::version::VERSION);
    let block = LUA_BLOCK.replacen(VERSION_LINE, &format!("local PROXY_VERSION = \"{version}\";"), 1);
    if cfg!(feature = "diag") {
        block.replacen(DIAG_LINE, "local PROXY_DIAG = true;", 1)
    } else {
        block
    }
}

// Scope the extra actions to the stock key-binding screen. Other consumers of Input
// (including standalone mods) keep the engine object.
const KEY_BINDING_INIT: &[u8] = b"function InitializeKeyBinding()";
const PING_BINDINGS: &str = concat!(
    "\nlocal Input = (function(engine)\n",
    include_str!("../../proxy/src/mods/pings/catalog.lua"), "\n",
    include_str!("../../proxy/src/mods/pings/bindings.lua"), "\n",
    include_str!("ping_bindings.lua"), "\nend)(Input)\n"
);

/// Options.lua suivi du bloc de l'onglet. Octets laissés tels quels : les fichiers du jeu ne sont
/// pas tous en UTF-8.
pub fn patch_options_lua(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    if let Some(at) = src.windows(KEY_BINDING_INIT.len()).position(|w| w == KEY_BINDING_INIT) {
        let cut = at + KEY_BINDING_INIT.len();
        out.extend_from_slice(&src[..cut]);
        out.extend_from_slice(PING_BINDINGS.as_bytes());
        out.extend_from_slice(&src[cut..]);
    } else {
        out.extend_from_slice(src);
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(lua_block().as_bytes());
    out
}

/// Entrées ajoutées au limiteur d'images, après celle du jeu qu'elles suivent : (ancre, ajout).
const TICK_ENTRIES: [(&str, &str); 2] = [
    (r#"{"LOC_OPTIONS_TICK_INTERVAL_30_FPS", 32},"#, r#" {"45 FPS", 21},"#),
    (r#"{"LOC_OPTIONS_TICK_INTERVAL_60_FPS", 16},"#, r#" {"75 FPS", 12}, {"90 FPS", 10}, {"100 FPS", 9},"#),
];

/// Options.lua avec les entrées de `TICK_ENTRIES` dans la liste du limiteur d'images, ou `None` si
/// une ancre manque (autre version du jeu) : la liste reste alors celle du jeu.
pub fn patch_tick_intervals(src: &[u8]) -> Option<Vec<u8>> {
    let mut out = src.to_vec();
    for (anchor, extra) in TICK_ENTRIES {
        let at = out.windows(anchor.len()).position(|w| w == anchor.as_bytes())? + anchor.len();
        out.splice(at..at, extra.bytes());
    }
    Some(out)
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

/// Le fichier de réglages sans les lignes des `keys` (une par ligne) : sans elles, le proxy prend
/// leur valeur par défaut. Les autres lignes (commentaires, clés que l'onglet ne connaît pas) restent.
pub fn reset_config(existing: &str, keys: &str) -> Result<String, String> {
    let keys: Vec<&str> = keys.lines().map(str::trim).filter(|k| !k.is_empty()).collect();
    if let Some(k) = keys.iter().find(|k| !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')) {
        return Err(format!("clé invalide : {k}"));
    }
    let kept: Vec<&str> = existing
        .lines()
        .filter(|line| !line.split_once('=').is_some_and(|(k, _)| keys.contains(&k.trim())))
        .collect();
    if kept.iter().all(|l| l.trim().is_empty()) {
        return Ok(String::new());
    }
    let mut text = kept.join("\r\n");
    text.push_str("\r\n");
    Ok(text)
}

// ---------------------------------------------------------------------------------------------
// Fichiers.

/// Dossier du proxy de GameCore, `DLC\Expansion2\Binaries\Win64`, depuis celui de l'exe
/// (`Base\Binaries\Win64Steam`), où est posé ce proxy.
pub(crate) fn gamecore_dir() -> Option<std::path::PathBuf> {
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
        let m = win::GetModuleHandleW(win::wide(crate::noms::GAMECORE_PROXY).as_ptr());
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
    let installed = dir.join(crate::noms::GAMECORE_PROXY).exists();
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

/// Retire du fichier les lignes des `keys` ; un fichier qui n'existe pas est déjà par défaut.
fn reset_config_file(keys: &str) -> Result<(), String> {
    let path = gamecore_dir().ok_or("dossier du jeu introuvable")?.join(CFG_NAME);
    let existing = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("lecture de {CFG_NAME} impossible : {e}")),
    };
    let text = reset_config(&existing, keys)?;
    std::fs::write(&path, text).map_err(|e| format!("écriture de {} impossible : {e}", path.display()))?;
    log(&format!("options : {CFG_NAME} réinitialisé ({})", keys.lines().collect::<Vec<_>>().join(", ")));
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
    lua_edit(l, write_config)
}

/// `Civ6Proxy_ReinitialiserReglages(cles)`, une clé par ligne → comme `Civ6Proxy_EcrireReglages`.
unsafe extern "C-unwind" fn lua_reset(l: *mut c_void) -> i32 {
    lua_edit(l, reset_config_file)
}

/// Applique `edit` à la chaîne en argument ; rend 1 et 1|0 selon que le proxy de GameCore a déjà
/// lu le fichier, ou nil et le message d'erreur.
unsafe fn lua_edit(l: *mut c_void, edit: fn(&str) -> Result<(), String>) -> i32 {
    let Some(h) = hks() else { return 0 };
    let mut len = 0usize;
    let p = (h.to_lstring)(l, 1, &mut len);
    let result = if p.is_null() {
        Err("argument attendu : une chaîne".to_string())
    } else {
        edit(&String::from_utf8_lossy(std::slice::from_raw_parts(p as *const u8, len)))
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

const OPTIONS_NATIVES: Natives = &[
    (lua_read, b"Civ6Proxy_LireReglages\0"),
    (lua_write, b"Civ6Proxy_EcrireReglages\0"),
    (lua_reset, b"Civ6Proxy_ReinitialiserReglages\0"),
];

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
            let src = patch_tick_intervals(src).unwrap_or_else(|| {
                log("options : liste du limiteur d'images introuvable, entrées non ajoutées");
                src.to_vec()
            });
            (patch_options_lua(&src), "options : onglet des proxys et entrées du limiteur d'images ajoutés")
        }
        crate::salon::SCRIPT => (crate::salon::prepare(l, src), "salon : annonce de version ajoutée"),
        crate::salon::SCRIPT_PARTIE => {
            let text = crate::salon::prepare_partie(l, src);
            (crate::rapport::prepare_suivi(l, text), "partie : relais du verdict et suivi des rapports de bug ajoutés")
        }
        crate::salon::SCRIPT_CHARGEMENT => {
            (crate::salon::prepare_chargement(l, src), "chargement : correctifs POL des parties locales ajoutés")
        }
        crate::maj::SCRIPT => (crate::maj::prepare(l, src), "maj : notification de mise à jour ajoutée"),
        crate::salon::SCRIPT_OPTION => (crate::salon::prepare_option(l, src), "option : « No First/Last moves (BGE) » ajoutée"),
        crate::rapport::SCRIPT => (crate::rapport::prepare(l, src), "rapport : bouton « Signaler un bug » du menu Échap ajouté"),
        _ => return real(l, buf, size, name),
    };
    let r = real(l, text.as_ptr(), text.len(), name);
    let (prefix, rest) = what.split_once(" : ").unwrap_or(("", what));
    if r == 0 {
        log(&format!("{prefix} : {chunk} : {rest}, compilation réussie"));
        return r;
    }
    // Le script modifié ne compile pas (par exemple trop de locales au niveau du fichier) : on
    // retire le message d'erreur et on compile l'original, pour ne jamais casser l'écran du jeu.
    let erreur = match hks() {
        Some(h) => {
            let p = (h.to_lstring)(l, -1, std::ptr::null_mut());
            let e = if p.is_null() { String::new() } else { CStr::from_ptr(p).to_string_lossy().into_owned() };
            (h.set_top)(l, -2);
            e
        }
        None => String::new(),
    };
    let r2 = real(l, buf, size, name);
    log(&format!(
        "{prefix} : {chunk} : {rest}, compilation en échec ({r} : {erreur}) ; original compilé à la place ({})",
        if r2 == 0 { "réussi".to_string() } else { format!("en échec {r2}") }
    ));
    r2
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
    if is_file(&name, crate::salon::XML) {
        let b = &mut *buf;
        let Some(src) = b.text() else { return r };
        match crate::salon::patch_staging_xml(src) {
            Ok(text) if replace_buffer(b, &text) => log(&format!("salon : {name} : bouton BGE Check et minuteur ajoutés")),
            Ok(_) => log(&format!("salon : {name} : allocation impossible, laissé tel quel")),
            Err(e) => log(&format!("salon : {name} laissé tel quel : {e}")),
        }
        return r;
    }
    if is_file(&name, crate::rapport::XML_PARTIE) {
        let b = &mut *buf;
        let Some(src) = b.text() else { return r };
        match crate::rapport::patch_in_game_xml(src) {
            Ok(text) if replace_buffer(b, &text) => log(&format!("rapport : {name} : minuteur du suivi ajouté")),
            Ok(_) => log(&format!("rapport : {name} : allocation impossible, laissé tel quel")),
            Err(e) => log(&format!("rapport : {name} laissé tel quel : {e}")),
        }
        return r;
    }
    if is_file(&name, crate::rapport::XML) {
        let b = &mut *buf;
        let Some(src) = b.text() else { return r };
        match crate::rapport::patch_menu_xml(src) {
            Ok(text) if replace_buffer(b, &text) => log(&format!("rapport : {name} : bouton « Signaler un bug » ajouté sous Options")),
            Ok(_) => log(&format!("rapport : {name} : allocation impossible, laissé tel quel")),
            Err(e) => log(&format!("rapport : {name} laissé tel quel : {e}")),
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
        // Le bouton de réinitialisation suit Confirmer, dans la même grille.
        let (confirm, reset) = (new.find(CONFIRM_ANCHOR).unwrap(), new.find(XML_RESET).unwrap());
        assert!(confirm < reset && new[confirm..reset].matches('<').count() == 1);
        assert_eq!(new.matches(r#"ID="ProxyResetButton""#).count(), 1);
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
    fn reglages_de_diagnostic() {
        assert_eq!(LUA_BLOCK.matches(DIAG_LINE).count(), 1);
        assert_eq!(LUA_BLOCK.matches(VERSION_LINE).count(), 1);
        assert!(lua_block().contains(&format!("local PROXY_VERSION = \"{}\";", crate::version::RELEASE.unwrap_or(crate::version::VERSION))));
        assert_eq!(lua_block().contains("local PROXY_DIAG = true;"), cfg!(feature = "diag"));
    }

    #[test]
    fn limiteur_d_images() {
        let Some(src) = read(format!(r"{GAME}\Base\Assets\UI\Options.lua")) else { return };
        let new = String::from_utf8_lossy(&patch_tick_intervals(&src).unwrap()).into_owned();
        let start = new.find("local tickInterval_options").unwrap();
        let table = &new[start..start + new[start..].find("};").unwrap()];
        // Valeurs dans l'ordre de la liste : désactivé, 20, 30, 45, 60, 75, 90, 100 images/s.
        let ms: Vec<u32> = table.split("\", ").skip(1).map(|s| s.split('}').next().unwrap().trim().parse().unwrap()).collect();
        assert_eq!(ms, [0, 49, 32, 21, 16, 12, 10, 9]);
        assert_eq!(new.len() - src.len(), TICK_ENTRIES.iter().map(|(_, e)| e.len()).sum::<usize>());
        assert!(patch_tick_intervals(b"x = 1").is_none());
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

    #[test]
    fn reinitialisation_du_fichier() {
        let old = "mode=off\r\n# note\r\nui_boutons=off\r\nmerveilles_file=on\r\nmode=count\r\n";
        assert_eq!(reset_config(old, "mode\nui_boutons\n").unwrap(), "# note\r\nmerveilles_file=on\r\n", "toutes les lignes d'une clé");
        assert_eq!(reset_config("mode=off\r\n\r\n", "mode").unwrap(), "", "rien d'autre : fichier vide");
        assert_eq!(reset_config(old, "").unwrap(), old);
        assert!(reset_config(old, "a b").is_err());
    }

    /// Chaque clé de l'onglet est lue par le proxy de GameCore (`read_config`, dans proxy/src/lib.rs),
    /// et chaque réglage qu'il lit figure dans l'onglet.
    #[test]
    fn cles_communes_avec_le_proxy() {
        // Clés lues par le proxy de GameCore, et par celui-ci (verif_maj).
        let mut lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../proxy/src/lib.rs")).unwrap();
        lib += include_str!("maj.rs");
        lib += include_str!("zones.rs");
        lib += include_str!("messages.rs");
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

    /// Chaque réglage de l'onglet a son nom et son aide dans les textes (textes/en_US.xml), et
    /// chaque clé LOC_ que le bloc emploie existe.
    #[test]
    fn textes_de_l_onglet() {
        let en = include_str!("../../textes/en_US.xml");
        let tab: Vec<&str> = LUA_BLOCK.split("cle = \"").skip(1).map(|s| s.split('"').next().unwrap()).collect();
        for k in &tab {
            for suffixe in ["NOM", "AIDE"] {
                let tag = format!("Tag=\"LOC_BGD_OPT_{}_{suffixe}\"", k.to_uppercase());
                assert!(en.contains(&tag), "{tag} manque dans textes/en_US.xml");
            }
        }
        let bloc = include_str!("options_proxy.lua");
        for cle in bloc.split('"').filter(|s| s.starts_with("LOC_BGD_") && !s.ends_with('_')) {
            assert!(en.contains(&format!("Tag=\"{cle}\"")), "{cle} manque dans textes/en_US.xml");
        }
    }
}
