//! Mods d'interface du Workshop repris dans le proxy (analyse et plan : mods_ui/README.md). Chaque
//! reprise a son réglage, on par défaut ; activée, elle **prime** sur le mod s'il est encore actif :
//! les fichiers du mod sont neutralisés au moment où l'exe les compile ou les lit.
//!
//! - `ui_charges` : charges des bâtisseurs sur leur drapeau (Better Builder Charges Tracking, wltk) ;
//! - `ui_routes` : tours avant la fin de la route d'un négociant, sur son drapeau ;
//! - `ui_notifications` : bouton « tout effacer » des notifications (NotificationClear, 336tarot) ;
//! - `ui_bandeau` : nourriture, production, activités moyennes par ville et luxes dans le bandeau
//!   (TopPanel Extension Pro, 号码菌 / Azukii) ;
//! - `ui_politiques` : effet chiffré des cartes de politique (Extended Policy Cards, Aristos) ;
//! - `ui_rapports` : écran des rapports de Better Report Screen (Infixo).
//!
//! Les scripts passent par l'accroche `luaL_loadbuffer` de `ui` en trois étapes : `remplacer` (le
//! texte entier : fichier du mod neutralisé, ou fichier du jeu remplacé par le nôtre), les correctifs
//! de `ui`, puis `ajouter` (blocs ajoutés ou insérés). Les XML passent par `xml`.
//!
//! Deux fonctions natives sont posées dans la table globale de chaque contexte dont on change un
//! script (`publier`) :
//! - `Civ6Proxy_Charger(nom)` compile un fichier embarqué (`EMBARQUES`) par le vrai
//!   `luaL_loadbuffer`, et rend la fonction, ou le texte d'erreur ;
//! - `Civ6Proxy_FinRoute(joueur origine, ville, joueur destination, ville, joueur négociant, unité)`
//!   rend la fin de la durée minimale de la route commerciale et son tour de départ, lus dans
//!   GameCore (voir `fin_route`), ou rien. La route finit ensuite au retour du négociant chez lui.

use crate::exe::is_file;
use crate::{log, win};
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;

static CHARGES: AtomicBool = AtomicBool::new(true);
static ROUTES: AtomicBool = AtomicBool::new(true);
static NOTIFICATIONS: AtomicBool = AtomicBool::new(true);
static BANDEAU: AtomicBool = AtomicBool::new(true);
static POLITIQUES: AtomicBool = AtomicBool::new(true);
static RAPPORTS: AtomicBool = AtomicBool::new(true);

pub fn set_charges(on: bool) {
    CHARGES.store(on, Ordering::Relaxed);
}
pub fn set_routes(on: bool) {
    ROUTES.store(on, Ordering::Relaxed);
}
pub fn set_notifications(on: bool) {
    NOTIFICATIONS.store(on, Ordering::Relaxed);
}
pub fn set_bandeau(on: bool) {
    BANDEAU.store(on, Ordering::Relaxed);
}
pub fn set_politiques(on: bool) {
    POLITIQUES.store(on, Ordering::Relaxed);
}
pub fn set_rapports(on: bool) {
    RAPPORTS.store(on, Ordering::Relaxed);
}

fn on(flag: &AtomicBool) -> bool {
    flag.load(Ordering::Relaxed)
}

/// Une reprise au moins est active : l'accroche Lua doit être posée.
pub fn any() -> bool {
    [&CHARGES, &ROUTES, &NOTIFICATIONS, &BANDEAU, &POLITIQUES, &RAPPORTS].iter().any(|f| on(f))
}

/// Une reprise a besoin de la lecture XML.
pub fn xml_needed() -> bool {
    on(&BANDEAU) || on(&POLITIQUES) || on(&RAPPORTS)
}

pub fn journal_reglages() -> String {
    let s = |f: &AtomicBool| if on(f) { "on" } else { "off" };
    format!(
        "charges {}, routes {}, notifications {}, bandeau {}, politiques {}, rapports {}",
        s(&CHARGES),
        s(&ROUTES),
        s(&NOTIFICATIONS),
        s(&BANDEAU),
        s(&POLITIQUES),
        s(&RAPPORTS)
    )
}

// ---------------------------------------------------------------------------------------------
// Fichiers embarqués.

/// Fichiers que `Civ6Proxy_Charger` sait compiler : nom, texte.
const EMBARQUES: [(&str, &[u8]); 12] = [
    ("BGD_Commun", include_bytes!("mods/commun.lua")),
    ("BGD_Textes", include_bytes!("mods/textes.lua")),
    ("RealModifierAnalysis", include_bytes!("mods/rapports/RealModifierAnalysis.lua")),
    ("BRSPage_Yields", include_bytes!("mods/rapports/BRSPage_Yields.lua")),
    ("BRSPage_Resources", include_bytes!("mods/rapports/BRSPage_Resources.lua")),
    ("BRSPage_Gossip", include_bytes!("mods/rapports/BRSPage_Gossip.lua")),
    ("BRSPage_CityStatus", include_bytes!("mods/rapports/BRSPage_CityStatus.lua")),
    ("BRSPage_Units", include_bytes!("mods/rapports/BRSPage_Units.lua")),
    ("BRSPage_Deals", include_bytes!("mods/rapports/BRSPage_Deals.lua")),
    ("BRSPage_Policy", include_bytes!("mods/rapports/BRSPage_Policy.lua")),
    ("BRSPage_Minor", include_bytes!("mods/rapports/BRSPage_Minor.lua")),
    ("BRSPage_Cities2", include_bytes!("mods/rapports/BRSPage_Cities2.lua")),
];

const DRAPEAUX: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("mods/drapeaux.lua"));
const NOTIFS: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("mods/notifications.lua"));
const BANDEAU_LUA: &str = include_str!("mods/bandeau.lua");
const BANDEAU_XML: &str = include_str!("mods/bandeau.xml");
const POL_ENTETE: &str = include_str!("mods/politiques_entete.lua");
const POL_CARTE: &str = include_str!("mods/politiques_carte.lua");
const POL_XML: &str = include_str!("mods/politiques.xml");
const RAPPORTS_LUA: &[u8] = include_bytes!("mods/rapports/reportscreen.lua");
const RAPPORTS_XML: &str = include_str!("mods/rapports/reportscreen.xml");
const LISTE: &str = include_str!("mods/rapports/liste.lua");
const TEXTES: &str = include_str!("mods/textes.tsv");

/// Better Spectator Mod : son script du bandeau, que TopPanel Extension Pro évinçait.
const BSM_ID: &str = "c6e5ad32-0600-4a98-a7cd-5854a1abcaaf";

fn has(src: &[u8], pat: &str) -> bool {
    src.windows(pat.len()).any(|w| w == pat.as_bytes())
}

fn find(src: &[u8], pat: &str) -> Vec<usize> {
    let p = pat.as_bytes();
    (0..src.len().saturating_sub(p.len() - 1)).filter(|&i| &src[i..i + p.len()] == p).collect()
}

/// Dossier du jeu : l'exe est dans Base\Binaries\Win64Steam.
fn game_dir() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.parent()?.parent()?.parent()?.to_path_buf())
}

/// Fichier du jeu relu sur le disque, pour remplacer celui d'un mod du même nom.
fn fichier_du_jeu(rel: &str) -> Result<Vec<u8>, String> {
    let path = game_dir().ok_or("dossier du jeu introuvable")?.join(rel);
    std::fs::read(&path).map_err(|e| format!("{} illisible : {e}", path.display()))
}

// ---------------------------------------------------------------------------------------------
// Étape 1 : texte entier remplacé.

/// Le script entier à compiler à la place de celui de l'exe, et ce qu'on a fait (journal).
pub fn remplacer(chunk: &str, src: &[u8]) -> Option<Result<(Vec<u8>, String), String>> {
    let vide = |quoi: &str| Some(Ok((format!("-- BetterGameDLLs : {quoi}, neutralisé par le proxy.\r\n").into_bytes(), format!("{quoi} neutralisé"))));
    if on(&RAPPORTS) {
        if is_file(chunk, "ReportScreen.lua") {
            let quoi = if has(src, "Better Report Screen") && !has(src, "BetterGameDLLs") {
                "Better Report Screen du proxy (fichier du mod neutralisé)"
            } else {
                "Better Report Screen du proxy"
            };
            return Some(Ok((RAPPORTS_LUA.to_vec(), quoi.into())));
        }
        // Comme le ReplaceUIScript de BRS : les surcharges de XP1 et XP2 ne passent plus par-dessus.
        // Les include restent, pour les scripts (BSM) qui incluent la chaîne.
        if is_file(chunk, "ReportScreen_Expansion2.lua") {
            return Some(Ok((b"include(\"ReportScreen_Expansion1\");\r\n".to_vec(), "réduit à son include (Better Report Screen)".into())));
        }
        if is_file(chunk, "ReportScreen_Expansion1.lua") {
            return Some(Ok((b"include(\"ReportScreen\");\r\n".to_vec(), "réduit à son include (Better Report Screen)".into())));
        }
        if is_file(chunk, "ReportsListLoader.lua") && has(src, "ReportsList Loader") {
            return Some(Ok((b"include(\"ReportsList\");\r\n".to_vec(), "ReportsList Loader neutralisé, liste du proxy".into())));
        }
        if is_file(chunk, "RealModifierAnalysis.lua") && !has(src, "BetterGameDLLs") {
            return vide("Real Modifier Analysis du mod");
        }
    }
    if on(&POLITIQUES) && is_file(chunk, "GovernmentScreen.lua") && has(src, "-- ARISTOS") {
        return Some(fichier_du_jeu(r"Base\Assets\UI\Screens\GovernmentScreen.lua").map(|t| (t, "fichier d'Extended Policy Cards remplacé par celui du jeu".into())));
    }
    if on(&BANDEAU) {
        if is_file(chunk, "TopPanelExtension.lua") && has(src, "TPE_BASE_RefreshYields") {
            // Le script de BSM n'est pas importé (seulement donné par ReplaceUIScript, que TPE évince) :
            // include ne le trouve pas. On essaie donc chaque fichier jusqu'à ce que la chaîne soit
            // chargée (Initialize défini), comme TPE.
            let t = format!(
                "-- BetterGameDLLs : TopPanel Extension Pro neutralisé, script du contexte rendu.\r\n\
                 for _, f in ipairs({{ \"TopPanel_Spectator\", \"TopPanel_Expansion2\", \"TopPanel_Expansion1\", \"TopPanel\" }}) do\r\n\
                 \tif f ~= \"TopPanel_Spectator\" or Modding.IsModActive(\"{BSM_ID}\") then\r\n\
                 \t\tinclude(f);\r\n\
                 \t\tif Initialize then break; end\r\n\
                 \tend\r\n\
                 end\r\n"
            );
            return Some(Ok((t.into_bytes(), "TopPanel Extension Pro neutralisé".into())));
        }
        if is_file(chunk, "TopPanel_TT.lua") && has(src, "TopPanelToolTip_") {
            return vide("infobulles de TopPanel Extension Pro");
        }
    }
    if on(&NOTIFICATIONS) && is_file(chunk, "NotificationPanel_QuickClear.lua") && has(src, "NPQC_") {
        return vide("NotificationClear");
    }
    if on(&CHARGES) && is_file(chunk, "UnitFlagManager_BuilderCharges.lua") && has(src, "OnUnitChargesChanged") {
        // Le choix du fichier de base du mod, sans ses surcharges : notre bloc fait le reste.
        let t = "-- BetterGameDLLs : Better Builder Charges Tracking neutralisé.\r\n\
                 for _, file in ipairs({ \"UnitFlagManager_BarbarianClansMode.lua\", \"UnitFlagManager.lua\" }) do\r\n\
                 \tinclude(file);\r\n\
                 \tif Initialize then break; end\r\n\
                 end\r\n";
        return Some(Ok((t.as_bytes().to_vec(), "Better Builder Charges Tracking neutralisé".into())));
    }
    None
}

// ---------------------------------------------------------------------------------------------
// Étape 3 : blocs ajoutés.

/// Le bloc de drapeaux, précédé de ses deux réglages.
pub fn patch_drapeaux(src: &[u8], charges: bool, routes: bool) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(format!("\r\nlocal BGD_Charges, BGD_Routes = {charges}, {routes};\r\n").as_bytes());
    out.extend_from_slice(DRAPEAUX.as_bytes());
    out
}

/// Le bloc des notifications, juste après le joker des fichiers NotificationPanel_.
pub fn patch_notifications(src: &[u8]) -> Result<Vec<u8>, String> {
    const JOKER: &str = "include(\"NotificationPanel_\", true);";
    let at = match find(src, JOKER).as_slice() {
        [at] => at + JOKER.len(),
        v => return Err(format!("{} include(\"NotificationPanel_\", true) au lieu de 1", v.len())),
    };
    let mut out = src[..at].to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(NOTIFS.as_bytes());
    out.extend_from_slice(&src[at..]);
    Ok(out)
}

/// L'effet de la carte : le bloc du proxy à la place de la dernière ligne de RealizePolicyCard.
pub fn patch_politiques(src: &[u8]) -> Result<Vec<u8>, String> {
    const ANCRE: &str = "cardInstance.Draggable:SetToolTipString(cardName .. \"[NEWLINE][NEWLINE]\" .. policy.Description);";
    let at = match find(src, ANCRE).as_slice() {
        [at] => *at,
        v => return Err(format!("{} infobulles de carte au lieu de 1", v.len())),
    };
    // L'en-tête après une éventuelle marque d'ordre d'octets, qui doit rester en tête du fichier.
    let bom = if src.starts_with(b"\xef\xbb\xbf") { 3 } else { 0 };
    let mut out = src[..bom].to_vec();
    out.extend_from_slice(POL_ENTETE.as_bytes());
    out.extend_from_slice(&src[bom..at]);
    out.extend_from_slice(POL_CARTE.trim_start().as_bytes());
    out.extend_from_slice(&src[at + ANCRE.len()..]);
    Ok(out)
}

fn ajoute(src: &[u8], bloc: &str) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(bloc.as_bytes());
    out
}

/// Le script avec nos blocs, et ce qu'on a fait. `entree_bandeau` : c'est le script du contexte
/// TopPanel, le premier compilé après TopPanel.xml (voir `xml_lu`).
pub fn ajouter(chunk: &str, src: &[u8], entree_bandeau: bool) -> Option<Result<(Vec<u8>, String), String>> {
    if (on(&CHARGES) || on(&ROUTES)) && is_file(chunk, "UnitFlagManager.lua") {
        return Some(Ok((patch_drapeaux(src, on(&CHARGES), on(&ROUTES)), "charges et routes sur les drapeaux".into())));
    }
    if on(&NOTIFICATIONS) && is_file(chunk, "NotificationPanel.lua") {
        return Some(patch_notifications(src).map(|t| (t, "bouton d'effacement des notifications".into())));
    }
    if on(&BANDEAU) && entree_bandeau {
        return Some(Ok((ajoute(src, BANDEAU_LUA), "bandeau de TopPanel Extension Pro".into())));
    }
    if on(&POLITIQUES) && is_file(chunk, "GovernmentScreen.lua") {
        return Some(patch_politiques(src).map(|t| (t, "effet des cartes de politique".into())));
    }
    if on(&RAPPORTS) && is_file(chunk, "ReportsList.lua") {
        return Some(Ok((ajoute(src, LISTE), "liste des rapports de Better Report Screen".into())));
    }
    None
}

// ---------------------------------------------------------------------------------------------
// XML.

/// Prochain script compilé : celui du contexte TopPanel (lu juste après son XML).
static ENTREE_BANDEAU: AtomicBool = AtomicBool::new(false);

/// À chaque XML lu, avant `xml`.
pub fn xml_lu(name: &str) {
    if on(&BANDEAU) && is_file(name, "TopPanel.xml") {
        ENTREE_BANDEAU.store(true, Ordering::Relaxed);
    }
}

/// À chaque script compilé : vrai pour le premier après TopPanel.xml.
pub fn prendre_entree_bandeau() -> bool {
    ENTREE_BANDEAU.swap(false, Ordering::Relaxed)
}

/// Les types d'infobulles du bandeau, avant la dernière balise </Context> de TopPanel.xml.
pub fn patch_xml_bandeau(src: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(src).map_err(|_| "texte non UTF-8".to_string())?;
    if text.contains("BGD_TooltipType_TopPanel_Food") {
        return Err("déjà modifié".into());
    }
    let end = text.rfind("</Context>").ok_or("balise </Context> introuvable")?;
    Ok([&text[..end], BANDEAU_XML, &text[end..]].concat().into_bytes())
}

/// Le cadre de l'effet sur la carte de politique, après le label NewIcon.
pub fn patch_xml_politiques(src: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(src).map_err(|_| "texte non UTF-8".to_string())?;
    if text.contains("ID=\"EffectContainer\"") {
        return Err("cadre déjà présent".into());
    }
    let label = match text.match_indices("ID=\"NewIcon\"").map(|(i, _)| i).collect::<Vec<_>>().as_slice() {
        [i] => *i,
        v => return Err(format!("{} labels NewIcon au lieu de 1", v.len())),
    };
    let close = text[label..].find("/>").ok_or("label NewIcon non fermé")? + label + 2;
    let line_end = text[close..].find('\n').map_or(text.len(), |i| close + i + 1);
    Ok([&text[..line_end], POL_XML, &text[line_end..]].concat().into_bytes())
}

/// Textes des mods (textes.tsv) : (clé, langue) → texte.
fn textes() -> &'static HashMap<(&'static str, &'static str), &'static str> {
    static T: OnceLock<HashMap<(&str, &str), &str>> = OnceLock::new();
    T.get_or_init(|| {
        TEXTES
            .lines()
            .filter_map(|l| {
                let mut it = l.splitn(3, '\t');
                Some(((it.next()?, it.next()?), it.next()?))
            })
            .collect()
    })
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Les valeurs d'attribut `"LOC_…"` connues des textes des mods, traduites dans `langue` (anglais
/// à défaut) : sans le mod, la base de localisation ne les a plus.
pub fn traduire_xml(text: &str, langue: &str) -> String {
    let t = textes();
    let mut out = String::with_capacity(text.len() + 1024);
    let mut rest = text;
    while let Some(i) = rest.find("\"LOC_") {
        out.push_str(&rest[..i + 1]);
        rest = &rest[i + 1..];
        let end = rest.find('"').unwrap_or(rest.len());
        let key = &rest[..end];
        match t.get(&(key, langue)).or_else(|| t.get(&(key, "en_US"))) {
            Some(s) => out.push_str(&xml_escape(s)),
            None => out.push_str(key),
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Langue du jeu (`fr_FR`…), lue dans Localization_FinalRelease.dll.
unsafe fn langue() -> String {
    let m = win::GetModuleHandleW(win::wide("Localization_FinalRelease.dll").as_ptr());
    if m.is_null() {
        return "en_US".into();
    }
    let current = win::GetProcAddress(m, b"?GetCurrentLanguage@Localization@@YAAEBVLanguage@1@XZ\0".as_ptr());
    let get_type = win::GetProcAddress(m, b"?GetType@Language@Localization@@QEBAPEBDXZ\0".as_ptr());
    if current.is_null() || get_type.is_null() {
        return "en_US".into();
    }
    let current: unsafe extern "C" fn() -> *const c_void = std::mem::transmute(current);
    let get_type: unsafe extern "C" fn(*const c_void) -> *const c_char = std::mem::transmute(get_type);
    let l = current();
    let t = if l.is_null() { std::ptr::null() } else { get_type(l) };
    if t.is_null() {
        return "en_US".into();
    }
    CStr::from_ptr(t).to_string_lossy().into_owned()
}

/// Le XML à mettre à la place de celui que l'exe vient de lire, et ce qu'on a fait.
pub unsafe fn xml(name: &str, src: &[u8]) -> Option<Result<(Vec<u8>, String), String>> {
    if on(&RAPPORTS) && is_file(name, "ReportScreen.xml") {
        let l = langue();
        return Some(Ok((traduire_xml(RAPPORTS_XML, &l).into_bytes(), format!("écran de Better Report Screen du proxy ({l})"))));
    }
    if on(&BANDEAU) && is_file(name, "TopPanel.xml") {
        return Some(patch_xml_bandeau(src).map(|t| (t, "infobulles de TopPanel Extension Pro".into())));
    }
    if on(&POLITIQUES) && is_file(name, "GovernmentScreen.xml") {
        let base = if has(src, "ARISTOS") {
            match fichier_du_jeu(r"Base\Assets\UI\Screens\GovernmentScreen.xml") {
                Ok(t) => t,
                Err(e) => return Some(Err(e)),
            }
        } else {
            src.to_vec()
        };
        return Some(patch_xml_politiques(&base).map(|t| (t, "cadre de l'effet des cartes de politique".into())));
    }
    None
}

// ---------------------------------------------------------------------------------------------
// Fonctions natives.

type LuaCFn = unsafe extern "C-unwind" fn(*mut c_void) -> i32;

struct Hks {
    push_closure: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, i32, *const c_char, i32),
    set_field: unsafe extern "C-unwind" fn(*mut c_void, i32, *const c_char),
    cpcall: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, *mut c_void) -> i32,
    set_top: unsafe extern "C-unwind" fn(*mut c_void, i32),
    to_lstring: unsafe extern "C-unwind" fn(*mut c_void, i32, *mut usize) -> *const c_char,
    to_integer: unsafe extern "C-unwind" fn(*mut c_void, i32) -> i32,
    push_integer: unsafe extern "C-unwind" fn(*mut c_void, i32),
    push_fstring: unsafe extern "C-unwind" fn(*mut c_void, *const c_char, ...) -> *const c_char,
}

static HKS: OnceLock<Option<Hks>> = OnceLock::new();

unsafe fn hks() -> Option<&'static Hks> {
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
            push_closure: std::mem::transmute(get(b"?hks_pushnamedcclosure@@YAXPEAUlua_State@@P6AH0@ZHPEBDH@Z\0")?),
            set_field: std::mem::transmute(get(b"?lua_setfield@@YAXPEAUlua_State@@HPEBD@Z\0")?),
            cpcall: std::mem::transmute(get(b"?lua_cpcall@@YAHPEAUlua_State@@P6AH0@ZPEAX@Z\0")?),
            set_top: std::mem::transmute(get(b"?lua_settop@@YAXPEAUlua_State@@H@Z\0")?),
            to_lstring: std::mem::transmute(get(b"?lua_tolstring@@YAPEBDPEAUlua_State@@HPEA_K@Z\0")?),
            to_integer: std::mem::transmute(get(b"?lua_tointeger@@YAHPEAUlua_State@@H@Z\0")?),
            push_integer: std::mem::transmute(get(b"?lua_pushinteger@@YAXPEAUlua_State@@H@Z\0")?),
            push_fstring: std::mem::transmute(get(b"?lua_pushfstring@@YAPEBDPEAUlua_State@@PEBDZZ\0")?),
        })
    })
    .as_ref()
}

const LUA_GLOBALSINDEX: i32 = -10002;

unsafe extern "C-unwind" fn lua_charger(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let p = (h.to_lstring)(l, 1, std::ptr::null_mut());
    let nom = if p.is_null() { String::new() } else { CStr::from_ptr(p).to_string_lossy().into_owned() };
    let Some(&(_, text)) = EMBARQUES.iter().find(|(n, _)| n.eq_ignore_ascii_case(&nom)) else {
        let msg = format!("fichier embarqué inconnu : {nom}\0");
        (h.push_fstring)(l, b"%s\0".as_ptr() as *const c_char, msg.as_ptr() as *const c_char);
        return 1;
    };
    let Some(real) = crate::ui::real_loadbuffer() else {
        (h.push_fstring)(l, b"%s\0".as_ptr() as *const c_char, b"luaL_loadbuffer absent\0".as_ptr() as *const c_char);
        return 1;
    };
    let chunk = format!("@BetterGameDLLs\\{nom}.lua\0");
    // Fonction compilée, ou texte d'erreur : les deux restent sur la pile.
    real(l, text.as_ptr(), text.len(), chunk.as_ptr() as *const c_char);
    1
}

unsafe extern "C-unwind" fn lua_fin_route(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let a: [i32; 6] = std::array::from_fn(|i| (h.to_integer)(l, i as i32 + 1));
    match fin_route(a) {
        Some((depart, fin)) => {
            (h.push_integer)(l, fin);
            (h.push_integer)(l, depart);
            2
        }
        None => 0,
    }
}

unsafe extern "C-unwind" fn installer(l: *mut c_void) -> i32 {
    if let Some(h) = hks() {
        for (name, f) in [(&b"Civ6Proxy_Charger\0"[..], lua_charger as LuaCFn), (&b"Civ6Proxy_FinRoute\0"[..], lua_fin_route as LuaCFn)] {
            (h.push_closure)(l, f, 0, name.as_ptr() as *const c_char, 0);
            (h.set_field)(l, LUA_GLOBALSINDEX, name.as_ptr() as *const c_char);
        }
    }
    0
}

static PUBLIE_LOG: AtomicBool = AtomicBool::new(false);

/// Pose les fonctions natives dans la table globale du contexte qui compile le script.
pub unsafe fn publier(l: *mut c_void) {
    let Some(h) = hks() else {
        if !PUBLIE_LOG.swap(true, Ordering::Relaxed) {
            log("mods : fonctions HavokScript introuvables, fichiers embarqués indisponibles");
        }
        return;
    };
    let r = (h.cpcall)(l, installer, std::ptr::null_mut());
    if r != 0 {
        (h.set_top)(l, -2);
    }
    if !PUBLIE_LOG.swap(true, Ordering::Relaxed) {
        log(&if r == 0 { "mods : fonctions natives posées (Civ6Proxy_Charger, Civ6Proxy_FinRoute)".to_string() } else { format!("mods : fonctions natives non posées ({r})") });
    }
}

// ---------------------------------------------------------------------------------------------
// Fin d'une route commerciale, lue dans GameCore.
//
// - `18038f6c0()` rend le gestionnaire de commerce (composant du jeu).
// - `180386480(gestionnaire + 0xc0, &origine, &destination)` cherche la route entre deux villes
//   (clés { 0, joueur, ville } de 12 octets) et rend sa fiche, ou 0 si elle n'est pas active. La
//   fiche porte le négociant en +0x8 (joueur, 16 bits) et +0xc (unité), et le tour de départ en
//   +0x4 : à la création (`1803913f0`), `180394510` y écrit le tour courant, et en +0x0 un numéro
//   d'ordre des routes. Premier essai en jeu (3 novembre 2026) : +0x20 valait -1.
// - `18038f720(gestionnaire, départ)` rend la fin de la durée minimale : départ +
//   ⌊TRADE_ROUTE_TURN_DURATION_BASE × CostMultiplier / 100⌋ + ⌊TradeRouteMinimumEndTurnChange de
//   l'ère courante (Eras_XP2) × CostMultiplier / 100⌋, au moins départ + 1 (mods_ui/README.md).
// Lecture seule, depuis le thread de l'interface : les mêmes fonctions que le jeu, sans rien écrire.

struct Site {
    address: usize,
    expected: [u8; 10],
}

const TRADE_MANAGER: Site = Site { address: 0x1_8038_f6c0, expected: [0x40, 0x53, 0x48, 0x83, 0xec, 0x20, 0x8b, 0x1d, 0x44, 0x59] };
const FIND_ROUTE: Site = Site { address: 0x1_8038_6480, expected: [0x48, 0x89, 0x5c, 0x24, 0x10, 0x57, 0x48, 0x83, 0xec, 0x60] };
const ROUTE_END: Site = Site { address: 0x1_8038_f720, expected: [0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10] };

static GAMECORE: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
struct CityKey {
    zero: u32,
    player: i32,
    id: i32,
}

/// Tour de départ de la route et fin de sa durée minimale ; `a` = joueur et ville d'origine, de
/// destination, joueur et unité du négociant. None si GameCore n'est pas celui attendu, ou si la route n'est pas celle du négociant.
unsafe fn fin_route(a: [i32; 6]) -> Option<(i32, i32)> {
    let base = GAMECORE.load(Ordering::Relaxed);
    if base == 0 {
        return None;
    }
    let at = |s: &Site| base + s.address - crate::patch::IMAGE_BASE as usize;
    let manager: unsafe extern "system" fn() -> usize = std::mem::transmute(at(&TRADE_MANAGER));
    let find: unsafe extern "system" fn(usize, *const CityKey, *const CityKey) -> usize = std::mem::transmute(at(&FIND_ROUTE));
    let end: unsafe extern "system" fn(usize, i32) -> i32 = std::mem::transmute(at(&ROUTE_END));
    // Les premiers appels de chaque partie vont au journal, pour vérifier la lecture en jeu.
    let n = APPELS_ROUTE.fetch_add(1, Ordering::Relaxed);
    let trace = |quoi: String| {
        if n < MAX_TRACES_ROUTE {
            log(&format!("mods : fin de route {a:?} : {quoi}"));
        }
    };
    let mgr = manager();
    if mgr == 0 {
        trace("pas de gestionnaire de commerce".into());
        return None;
    }
    let origine = CityKey { zero: 0, player: a[0], id: a[1] };
    let destination = CityKey { zero: 0, player: a[2], id: a[3] };
    let route = find(mgr + 0xc0, &origine, &destination);
    if route == 0 {
        trace(format!("route introuvable (gestionnaire {mgr:#x})"));
        return None;
    }
    let (joueur, unite, depart) = (*((route + 0x8) as *const i16), *((route + 0xc) as *const i32), *((route + 0x4) as *const i32));
    if joueur as i32 != a[4] || unite != a[5] || depart < 0 {
        let octets = std::slice::from_raw_parts(route as *const u8, 0x40);
        trace(format!("fiche {route:#x} d'un autre négociant ({joueur}, {unite}), départ {depart}, octets {octets:02x?}"));
        return None;
    }
    let fin = end(mgr, depart);
    trace(format!("départ {depart}, fin {fin}"));
    Some((depart, fin))
}

static APPELS_ROUTE: AtomicUsize = AtomicUsize::new(0);
const MAX_TRACES_ROUTE: usize = 30;

pub unsafe fn on_gamecore_loaded(base: usize) {
    if !on(&ROUTES) {
        return;
    }
    for s in [&TRADE_MANAGER, &FIND_ROUTE, &ROUTE_END] {
        let at = base + s.address - crate::patch::IMAGE_BASE as usize;
        let have = std::slice::from_raw_parts(at as *const u8, s.expected.len());
        if have != s.expected {
            log(&format!("mods : octets inattendus en {:#x} ({have:02x?}), pas de tours des routes commerciales", s.address));
            return;
        }
    }
    GAMECORE.store(base, Ordering::Relaxed);
    log("mods : fin des routes commerciales lue dans GameCore (18038f6c0, 180386480, 18038f720)");
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
    fn textes_lus() {
        let t = textes();
        assert!(t.len() > 500, "{}", t.len());
        assert_eq!(t.get(&("LOC_HUD_REPORTS_TAB_DEALS", "en_US")), Some(&"Deals"));
        assert!(t.contains_key(&("LOC_TOPPANEL_FOOD_TOOLTIP_HEADER", "fr_FR")));
    }

    #[test]
    fn xml_des_rapports_traduit() {
        let fr = traduire_xml(RAPPORTS_XML, "fr_FR");
        let en = traduire_xml(RAPPORTS_XML, "en_US");
        // Aucune clé du mod ne reste ; les clés du jeu, elles, restent.
        for (k, _) in textes().keys() {
            assert!(!fr.contains(&format!("\"{k}\"")), "{k} non traduit");
        }
        assert!(en.contains("\"LOC_HUD_REPORTS_") || en.contains("\"LOC_"), "plus aucune clé du jeu ?");
        assert!(fr.len() > 100_000 && en.len() > 100_000);
        assert_eq!(traduire_xml(r#"<A String="LOC_X_INCONNUE" B="LOC_HUD_REPORTS_TAB_DEALS"/>"#, "en_US"), r#"<A String="LOC_X_INCONNUE" B="Deals"/>"#);
        assert_eq!(xml_escape(r#"a<b>&"c""#), "a&lt;b&gt;&amp;&quot;c&quot;");
    }

    #[test]
    fn notifications_du_jeu() {
        let Some(src) = read(format!(r"{GAME}\Base\Assets\UI\Panels\NotificationPanel.lua")) else { return };
        let out = patch_notifications(&src).unwrap();
        let joker = find(&out, "include(\"NotificationPanel_\", true);")[0];
        let bloc = find(&out, "BetterGameDLLs (src/mods/notifications.lua)")[0];
        let init = out.len() - find(&out, "Initialize();").iter().map(|&i| out.len() - i).min().unwrap();
        assert!(joker < bloc && bloc < init);
        assert!(patch_notifications(b"x = 1").is_err());
    }

    #[test]
    fn politiques_du_jeu() {
        let Some(src) = read(format!(r"{GAME}\Base\Assets\UI\Screens\GovernmentScreen.lua")) else { return };
        let out = String::from_utf8(patch_politiques(&src).unwrap()).unwrap();
        assert!(out.starts_with(POL_ENTETE));
        assert!(out.contains("BGD_ChargerRMA()") && !out.contains("cardName .. \"[NEWLINE][NEWLINE]\" .. policy.Description);"));
        let Some(xml) = read(format!(r"{GAME}\Base\Assets\UI\Screens\GovernmentScreen.xml")) else { return };
        let out = String::from_utf8(patch_xml_politiques(&xml).unwrap()).unwrap();
        let (icon, effet) = (out.find("ID=\"NewIcon\"").unwrap(), out.find("ID=\"EffectContainer\"").unwrap());
        assert!(icon < effet && effet < out.find("</Drag>").unwrap());
        assert!(patch_xml_politiques(out.as_bytes()).is_err());
    }

    #[test]
    fn bandeau_du_jeu() {
        let Some(xml) = read(format!(r"{GAME}\Base\Assets\UI\TopPanel.xml")) else { return };
        let out = String::from_utf8(patch_xml_bandeau(&xml).unwrap()).unwrap();
        assert!(out.find("BGD_TooltipType_TopPanel_Food").unwrap() < out.rfind("</Context>").unwrap());
        assert!(patch_xml_bandeau(out.as_bytes()).is_err());
    }

    #[test]
    fn drapeaux_regles() {
        let out = String::from_utf8(patch_drapeaux(b"x = 1", true, false)).unwrap();
        assert!(out.starts_with("x = 1\r\nlocal BGD_Charges, BGD_Routes = true, false;\r\n"));
    }

    #[test]
    fn mods_neutralises() {
        set_rapports(true);
        set_bandeau(true);
        set_notifications(true);
        set_charges(true);
        let w = r"C:\Program Files (x86)\Steam\steamapps\workshop\content\289070";
        let cas = [
            (format!(r"{w}\1312585482\ReportScreen.lua"), "Better Report Screen du proxy (fichier du mod neutralisé)"),
            (format!(r"{w}\1312585482\RealModifierAnalysis.lua"), "Real Modifier Analysis du mod neutralisé"),
            (format!(r"{w}\1312585482\RLL\ReportsListLoader.lua"), "ReportsList Loader neutralisé, liste du proxy"),
            (format!(r"{w}\2997927787\TPE\UI\TopPanelExtension.lua"), "TopPanel Extension Pro neutralisé"),
            (format!(r"{w}\2997927787\TPE\UI\TopPanel_TT.lua"), "infobulles de TopPanel Extension Pro neutralisé"),
            (format!(r"{w}\3063966792\NotificationPanel_QuickClear.lua"), "NotificationClear neutralisé"),
            (format!(r"{w}\2409116842\UnitFlagManager_BuilderCharges.lua"), "Better Builder Charges Tracking neutralisé"),
        ];
        for (path, quoi) in cas {
            let Some(src) = read(path.clone()) else { continue };
            let (_, w) = remplacer(&path, &src).unwrap().unwrap();
            assert_eq!(w, quoi, "{path}");
        }
        // Notre propre RMA, chargé par Civ6Proxy_Charger, n'est jamais neutralisé (il porte la marque).
        assert!(remplacer("RealModifierAnalysis.lua", EMBARQUES[2].1).is_none());
        // Fichiers du jeu sans le mod : rien à neutraliser.
        assert!(remplacer(r"C:\x\NotificationPanel_Expansion2.lua", b"x").is_none());
    }

    /// Les octets que `on_gamecore_loaded` vérifie, lus dans le GameCore d'origine.
    #[test]
    fn sites_de_commerce() {
        let path = format!(r"{GAME}\DLC\Expansion2\Binaries\Win64\GameCore_XP2_FinalRelease_orig.dll");
        let Some(d) = read(path.clone()).or_else(|| read(path.replace("_orig", ""))) else { return };
        let u32_at = |o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as usize;
        let pe = u32_at(0x3c);
        let count = u16::from_le_bytes([d[pe + 6], d[pe + 7]]) as usize;
        let first = pe + 24 + u16::from_le_bytes([d[pe + 20], d[pe + 21]]) as usize;
        let offset = |rva: usize| {
            (0..count).map(|i| first + 40 * i).find_map(|s| {
                let (va, raw, ptr) = (u32_at(s + 12), u32_at(s + 16), u32_at(s + 20));
                (va <= rva && rva < va + raw).then(|| rva - va + ptr)
            })
        };
        for s in [&TRADE_MANAGER, &FIND_ROUTE, &ROUTE_END] {
            let o = offset(s.address - crate::patch::IMAGE_BASE as usize).unwrap();
            assert_eq!(&d[o..o + s.expected.len()], &s.expected, "{:#x}", s.address);
        }
    }

    #[test]
    fn embarques_complets() {
        for (nom, text) in EMBARQUES {
            assert!(!text.is_empty(), "{nom}");
            assert!(std::str::from_utf8(text).is_ok(), "{nom}");
        }
        let lua = String::from_utf8_lossy(RAPPORTS_LUA);
        // Chaque page incluse est embarquée, et plus aucun include de page ne reste.
        for page in lua.split("BGD_Inclure(\"").skip(1).map(|s| s.split('"').next().unwrap()) {
            assert!(EMBARQUES.iter().any(|(n, _)| *n == page), "{page}");
        }
        assert!(!lua.contains("include(\"BRSPage_") && !lua.contains("GlobalParameters.BRS_"));
    }
}
