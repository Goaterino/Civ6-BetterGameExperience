//! Boutons d'attaque des bannières (voir ui_strike/README.md) : patch de l'interface au
//! moment où l'exe lit ses fichiers, sans mod. Rien ne part sur le réseau, chaque joueur qui a le
//! proxy change son propre affichage.
//!
//! - **Lua.** L'exe charge chaque script par `140dc30e0`, seul appelant de `luaL_loadbuffer`
//!   (HavokScript_FinalRelease.dll). On remplace l'entrée de cet import dans la table d'imports
//!   de l'exe : pour `CityBannerManager.lua`, quel que soit le mod qui le fournit, on compile un
//!   texte modifié qui appartient au proxy (luaL_loadbuffer ne garde pas le tampon).
//! - **XML.** `140a2d280` (DX12 ; `140a1f3f0` en DX11, retrouvée par la même signature) lit un
//!   fichier XML entier dans un tampon du jeu `{ptr, len, cap, allocateur}`, NUL final compris.
//!   Pour `CityBannerInstances.xml`, on réécrit ce tampon après la lecture, en l'agrandissant par
//!   l'allocateur du jeu, qui le libérera.
//!
//! Réglages dans rollover_proxy.cfg :
//! - `ui_boutons=on|off` (on) : boutons au-dessus des drapeaux d'unités (`ui_boutons.lua` et
//!   `ui_boutons.xml`), et `ConsumeMouse` sur les boutons nucléaires ;
//! - `ui_recentrage=on|off` (off) : off retire le recentrage de la caméra au clic sur ces boutons,
//!   en passant `false` en second argument de `UI.SelectCity` / `UI.SelectDistrict` ;
//! - `ui_journal=on|off` (off) : note le nom de chaque script et de chaque XML chargés ;
//! - `ui_mesure_epingles=on|off` (off) : ajoute `ui_epingles.lua` à la fin des scripts que
//!   recharge chaque `PlayerInfoChanged` (épingles, liste des épingles, bandeau des dirigeants, DMT).
//!   Les durées vont dans Lua.log (voir map_pins/README.md) ;
//! - `ui_epingles=on|off` (on) : ajoute `ui_epingles_opti.lua` à MapPinManager.lua et
//!   MapPinListPanel.lua. Plus de reconstruction quand aucune épingle n'a changé, une seule lecture
//!   de GetMapPins par joueur et par reconstruction, liste fermée reconstruite à son ouverture ;
//! - `ui_merveilles=on|off` (on) : ajoute `ui_merveilles.lua` à ProductionHelper.lua. Une merveille
//!   retirée de la file par un échange (wonder_queue/README.md) est remise en tête par des commandes
//!   du jeu, rejouées à l'identique par tous les joueurs, proxy ou non ;
//! - `ui_diag_accords=on|off` (off) : ajoute `ui_diag_accords.lua` à DiplomacyDealView.lua. Écrit
//!   dans Lua.log les événements d'accord et de session, et l'état de l'écran d'accord (diagnostic
//!   des accords croisés, docs/netcode/README.md) ;
//! - `ui_rendements=on|off` (on, lu par `rendements_empire`) : ajoute `ui_rendements.lua` à
//!   TopPanel.lua. Le bandeau redessine ses rendements après un gouverneur ou un personnage
//!   illustre, dont `rendements_empire` refait les instantanés (empire_yields/README.md).
//!
//! Les mods d'interface repris (`mods_ui`) passent par les mêmes accroches : leur remplacement du
//! texte entier avant les correctifs de ce module, leurs blocs après, une seule compilation.
//!
//! Dès que l'accroche Lua est posée, trois défauts du jeu sont aussi corrigés (`FIXES`) : le bouton
//! d'attaque d'une ville non sélectionnée demandait deux clics (`patch_city_panel`), un clic sur le
//! drapeau d'une unité ne tirait pas en visée d'une ville ou d'un quartier (`patch_unit_flags`), et
//! la production en cours du panneau de production ne se montrait pas en passant d'une ville sans
//! production à une ville qui en a une (`patch_production_panel`).

use crate::exe::{self, is_file, iat_slot, replace_buffer, write_slot, GameBuf, ReadXml, XmlHook};
use crate::{log, win};
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

const LUA_BLOCK: &str = include_str!("ui_boutons.lua");
const XML_BLOCK: &str = include_str!("ui_boutons.xml");
const MESURE_BLOCK: &str = include_str!("ui_epingles.lua");
const OPTI_BLOCK: &str = include_str!("ui_epingles_opti.lua");
const MERVEILLES_BLOCK: &str = include_str!("ui_merveilles.lua");
const RENDEMENTS_BLOCK: &str = include_str!("ui_rendements.lua");
const CONGRES_BLOCK: &str = include_str!("ui_congres.lua");
const CIBLES_BLOCK: &str = include_str!("ui_congres_cibles.lua");
const DIAG_ACCORDS_BLOCK: &str = include_str!("ui_diag_accords.lua");

static BOUTONS: AtomicBool = AtomicBool::new(true);
static RECENTRAGE: AtomicBool = AtomicBool::new(false);
static JOURNAL: AtomicBool = AtomicBool::new(false);
static MESURE: AtomicBool = AtomicBool::new(false);
static OPTI: AtomicBool = AtomicBool::new(true);
static MERVEILLES: AtomicBool = AtomicBool::new(true);
static DIAG_ACCORDS: AtomicBool = AtomicBool::new(false);
/// Les calques sont définis dans le dernier CityBannerInstances.xml lu : le bloc Lua, qui s'en
/// sert, n'est ajouté qu'à cette condition.
static XML_PATCHED: AtomicBool = AtomicBool::new(false);

static IAT_SLOT: AtomicUsize = AtomicUsize::new(0);
static REAL_LOADBUFFER: AtomicUsize = AtomicUsize::new(0);
static XML_HOOK: Mutex<Option<XmlHook>> = Mutex::new(None);
static XML_TRAMP: AtomicUsize = AtomicUsize::new(0);

pub fn set_boutons(on: bool) {
    BOUTONS.store(on, Ordering::Relaxed);
}

pub fn set_recentrage(on: bool) {
    RECENTRAGE.store(on, Ordering::Relaxed);
}

pub fn set_journal(on: bool) {
    JOURNAL.store(on, Ordering::Relaxed);
}

pub fn set_mesure_epingles(on: bool) {
    MESURE.store(on, Ordering::Relaxed);
}

pub fn set_epingles(on: bool) {
    OPTI.store(on, Ordering::Relaxed);
}

pub fn set_merveilles(on: bool) {
    MERVEILLES.store(on, Ordering::Relaxed);
}

pub fn set_diag_accords(on: bool) {
    DIAG_ACCORDS.store(on, Ordering::Relaxed);
}

fn active() -> bool {
    BOUTONS.load(Ordering::Relaxed)
        || !RECENTRAGE.load(Ordering::Relaxed)
        || JOURNAL.load(Ordering::Relaxed)
        || MESURE.load(Ordering::Relaxed)
        || OPTI.load(Ordering::Relaxed)
        || MERVEILLES.load(Ordering::Relaxed)
        || DIAG_ACCORDS.load(Ordering::Relaxed)
        || crate::rendements_empire::enabled()
        || crate::mods_ui::any()
        || crate::spectateurs::poses()
        || crate::depart::interface_requise()
}

// ---------------------------------------------------------------------------------------------
// Remplacements de texte, sans état : testés sur les vrais fichiers (voir les tests en bas).

/// Les fonctions de clic et l'appel qui sélectionne, donc recentre.
const CLICKS: [(&str, &str); 3] = [
    ("OnCityStrikeButtonClick", "UI.SelectCity("),
    ("OnDistrictRangeStrikeButtonClick", "UI.SelectDistrict("),
    ("OnICBMStrikeButtonClick", "UI.SelectCity("),
];

/// Ajoute `false` en second argument du premier `call` du corps de la fonction `func`.
fn no_recenter(text: &mut String, func: &str, call: &str) -> Result<(), String> {
    let header = format!("function {func}");
    let start = text
        .match_indices(&header)
        .map(|(i, _)| i)
        .find(|&i| text[i + header.len()..].trim_start().starts_with('('))
        .ok_or_else(|| format!("{func} introuvable"))?;
    let body_end = text[start + header.len()..].find("\nfunction ").map_or(text.len(), |i| start + header.len() + i);
    let at = text[start..body_end].find(call).ok_or_else(|| format!("{func} : {call}…) introuvable"))? + start + call.len();
    let close = text[at..body_end].find(')').ok_or_else(|| format!("{func} : parenthèse fermante introuvable"))? + at;
    let args = text[at..close].trim();
    if args.is_empty() || args.contains(',') {
        return Err(format!("{func} : arguments inattendus ({args})"));
    }
    let new = format!(" {args}, false ");
    text.replace_range(at..close, &new);
    Ok(())
}

/// Texte modifié de CityBannerManager.lua, et ce qui a été fait ou pas, pour le journal.
pub fn patch_lua(src: &[u8], boutons: bool, recentrage: bool) -> (Option<Vec<u8>>, Vec<String>) {
    let mut notes = Vec::new();
    let Ok(text) = std::str::from_utf8(src) else {
        return (None, vec!["texte non UTF-8, laissé tel quel".into()]);
    };
    let mut text = text.to_owned();
    let mut changed = false;
    if !recentrage {
        for (func, call) in CLICKS {
            match no_recenter(&mut text, func, call) {
                Ok(()) => changed = true,
                Err(e) => notes.push(format!("recentrage gardé : {e}")),
            }
        }
    }
    if boutons {
        text.push_str("\r\n");
        text.push_str(LUA_BLOCK);
        changed = true;
    }
    (changed.then(|| text.into_bytes()), notes)
}

/// Corrige la faute de `CityPanel.lua` (OnCitySelectionChanged) qui coûte un second clic au bouton
/// d'attaque d'une ville non sélectionnée. Le clic sélectionne la ville puis passe en
/// CITY_RANGE_ATTACK ; l'événement CitySelectionChanged arrive ensuite et doit garder ce mode si la
/// ville peut tirer, mais teste `CityManager.GetCity(owningPlayerID, cityID)`, alors que le
/// paramètre s'appelle `ownerPlayerID`. Avec un joueur nil, le test échoue toujours et l'interface
/// repasse en SELECTION.
pub fn patch_city_panel(src: &[u8]) -> Result<Vec<u8>, String> {
    const HEADER: &str = "function OnCitySelectionChanged( ownerPlayerID";
    const WRONG: &str = "CityManager.GetCity(owningPlayerID, cityID)";
    let text = std::str::from_utf8(src).map_err(|_| "texte non UTF-8".to_string())?;
    let start = text.find(HEADER).ok_or("OnCitySelectionChanged( ownerPlayerID… introuvable")?;
    let end = text[start + HEADER.len()..].find("\nfunction ").map_or(text.len(), |i| start + HEADER.len() + i);
    let body = &text[start..end];
    if body.contains("local owningPlayerID") {
        return Err("owningPlayerID est défini dans la fonction".into());
    }
    let at = body.find(WRONG).ok_or("appel fautif introuvable (déjà corrigé ?)")? + start;
    let mut out = text.to_owned();
    out.replace_range(at..at + WRONG.len(), "CityManager.GetCity(ownerPlayerID, cityID)");
    Ok(out.into_bytes())
}

const FLAG_BLOCK: &str = include_str!("ui_drapeaux.lua");

/// En visée d'une ville ou d'un quartier, un clic gauche sur le drapeau d'une unité étrangère ne
/// tirait pas : `OnUnitFlagClick` ne sait faire attaquer que l'unité sélectionnée, et le drapeau
/// prend le clic avant WorldInput. Les deux clics gauches des drapeaux passent par
/// `Civ6RolloverProxy_OnUnitFlagLClick` (ui_drapeaux.lua), inséré avant `OnUnitFlagClick` : il
/// tire sur la case de l'unité, sinon il appelle `OnUnitFlagClick`. Le clic droit ne change pas.
///
/// Travaille sur les octets : le fichier du jeu n'est pas en UTF-8 (celui de BSM l'est).
pub fn patch_unit_flags(src: &[u8]) -> Result<Vec<u8>, String> {
    const CLICK: &[u8] = b"RegisterCallback( Mouse.eLClick, OnUnitFlagClick )";
    const NEW_CLICK: &[u8] = b"RegisterCallback( Mouse.eLClick, Civ6RolloverProxy_OnUnitFlagLClick )";
    const HEADER: &[u8] = b"\nfunction OnUnitFlagClick(";
    let find_from = |from: usize, pat: &[u8]| src[from..].windows(pat.len()).position(|w| w == pat).map(|i| from + i);
    let mut clicks = Vec::new();
    while let Some(i) = find_from(clicks.last().map_or(0, |&i: &usize| i + CLICK.len()), CLICK) {
        clicks.push(i);
    }
    if clicks.len() != 2 {
        return Err(format!("{} clics gauches branchés sur OnUnitFlagClick au lieu de 2", clicks.len()));
    }
    let header = find_from(0, HEADER).ok_or("OnUnitFlagClick introuvable")? + 1;
    // (position, texte inséré, octets remplacés), dans l'ordre du fichier.
    let mut edits = vec![(header, FLAG_BLOCK.as_bytes(), 0)];
    edits.extend(clicks.iter().map(|&c| (c, NEW_CLICK, CLICK.len())));
    edits.sort_by_key(|e| e.0);
    let mut out = Vec::with_capacity(src.len() + FLAG_BLOCK.len() + 64);
    let mut pos = 0;
    for (at, insert, skip) in edits {
        out.extend_from_slice(&src[pos..at]);
        out.extend_from_slice(insert);
        pos = at + skip;
    }
    out.extend_from_slice(&src[pos..]);
    Ok(out)
}

const PRODUCTION_BLOCK: &str = include_str!("ui_production.lua");

/// Les variables locales de ProductionPanel.lua que lit `ui_production.lua`, et la fonction qu'il
/// enveloppe.
const PRODUCTION_NAMES: [&str; 5] =
    ["local LISTMODE", "local m_isQueueOpen", "local m_hasProductionToShow", "local m_CurrentListMode", "\nfunction View("];

/// En changeant de ville, la production en cours du panneau gardait la visibilité de la ville
/// précédente : cachée après une ville sans production. `ui_production.lua`, ajouté à la fin du
/// fichier pour lire ses variables locales, enveloppe `View` et applique la règle des onglets.
/// Octets laissés tels quels, comme pour les drapeaux.
pub fn patch_production_panel(src: &[u8]) -> Result<Vec<u8>, String> {
    let has = |pat: &str| src.windows(pat.len()).any(|w| w == pat.as_bytes());
    if let Some(missing) = PRODUCTION_NAMES.iter().find(|n| !has(n)) {
        return Err(format!("{} introuvable", missing.trim_start()));
    }
    if has("Civ6RolloverProxy (src/ui_production.lua)") {
        return Err("déjà corrigé".into());
    }
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(PRODUCTION_BLOCK.as_bytes());
    Ok(out)
}

/// Nom du fichier d'un script, en minuscules.
fn script_name(chunk: &str) -> String {
    chunk.rsplit(['/', '\\']).next().unwrap_or("").trim_start_matches('@').to_ascii_lowercase()
}

/// Scripts que `ui_epingles_opti.lua` allège : les drapeaux des épingles et leur liste.
const OPTIMIZED: [&str; 4] = ["mappinmanager.lua", "mappinmanager_dmt.lua", "mappinmanager_cqui.lua", "mappinlistpanel.lua"];

/// Nom du script, en minuscules, s'il fait partie de ceux que mesure `ui_epingles.lua` : ceux qui
/// reconstruisent quelque chose à chaque `PlayerInfoChanged`, et le calcul des rendements de DMT.
fn measured(chunk: &str) -> Option<String> {
    let name = script_name(chunk);
    // DiplomacyRibbonSupport.lua est une bibliothèque incluse ailleurs (LeaderIcon.lua), pas le bandeau.
    let ribbon = name == "diplomacyribbon.lua" || (name.starts_with("diplomacyribbon_") && name.ends_with(".lua"));
    (OPTIMIZED.contains(&name.as_str()) || name == "dmt_yieldcalculator.lua" || ribbon).then_some(name)
}

/// HavokScript : `os.clock` n'y avance que par secondes entières. Le bloc de mesure lit donc
/// `Civ6RolloverProxy_Clock()` (secondes, résolution QueryPerformanceCounter), posée dans la table
/// globale (`LUA_GLOBALSINDEX`, -10002, celle où l'exe pose `PlayerConfiguration`) avant chaque
/// script mesuré. La pose passe par `lua_cpcall` : si la table refuse l'écriture, l'erreur reste
/// dans Lua (d'où `C-unwind`) et le bloc se rabat sur `os.clock`.
const CLOCK_NAME: &[u8] = b"Civ6RolloverProxy_Clock\0";
const LUA_GLOBALSINDEX: i32 = -10002;

type LuaCFn = unsafe extern "C-unwind" fn(*mut c_void) -> i32;

struct Hks {
    push_number: unsafe extern "C-unwind" fn(*mut c_void, f64),
    push_closure: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, i32, *const c_char, i32),
    set_field: unsafe extern "C-unwind" fn(*mut c_void, i32, *const c_char),
    cpcall: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, *mut c_void) -> i32,
    set_top: unsafe extern "C-unwind" fn(*mut c_void, i32),
}

static HKS: std::sync::OnceLock<Option<Hks>> = std::sync::OnceLock::new();
static CLOCK_LOGGED: AtomicBool = AtomicBool::new(false);

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
            push_number: std::mem::transmute(get(b"?lua_pushnumber@@YAXPEAUlua_State@@N@Z\0")?),
            push_closure: std::mem::transmute(get(b"?hks_pushnamedcclosure@@YAXPEAUlua_State@@P6AH0@ZHPEBDH@Z\0")?),
            set_field: std::mem::transmute(get(b"?lua_setfield@@YAXPEAUlua_State@@HPEBD@Z\0")?),
            cpcall: std::mem::transmute(get(b"?lua_cpcall@@YAHPEAUlua_State@@P6AH0@ZPEAX@Z\0")?),
            set_top: std::mem::transmute(get(b"?lua_settop@@YAXPEAUlua_State@@H@Z\0")?),
        })
    })
    .as_ref()
}

unsafe extern "C-unwind" fn lua_clock(l: *mut c_void) -> i32 {
    if let Some(h) = hks() {
        (h.push_number)(l, win::qpc_ms(win::qpc()) / 1000.0);
        return 1;
    }
    0
}

unsafe extern "C-unwind" fn install_clock(l: *mut c_void) -> i32 {
    if let Some(h) = hks() {
        (h.push_closure)(l, lua_clock, 0, CLOCK_NAME.as_ptr() as *const c_char, 0);
        (h.set_field)(l, LUA_GLOBALSINDEX, CLOCK_NAME.as_ptr() as *const c_char);
    }
    0
}

unsafe fn publish_clock(l: *mut c_void) {
    let Some(h) = hks() else {
        if !CLOCK_LOGGED.swap(true, Ordering::Relaxed) {
            log("ui : fonctions HavokScript introuvables, mesure des épingles avec os.clock (à la seconde)");
        }
        return;
    };
    let r = (h.cpcall)(l, install_clock, std::ptr::null_mut());
    if r != 0 {
        // lua_cpcall laisse l'erreur sur la pile.
        (h.set_top)(l, -2);
    }
    if !CLOCK_LOGGED.swap(true, Ordering::Relaxed) {
        log(&if r == 0 { "ui : horloge Civ6RolloverProxy_Clock posée".to_string() } else { format!("ui : horloge non posée ({r}), mesure avec os.clock") });
    }
}

/// `Civ6RolloverProxy_Pol()` → 1 si les scripts du congrès doivent écarter les spectateurs, sinon 0
/// (`spectateurs::lua_actif`). Relu à chaque appel : l'état POL n'est fixé qu'au chargement.
const POL_NAME: &[u8] = b"Civ6RolloverProxy_Pol\0";
static POL_LOGGED: AtomicBool = AtomicBool::new(false);

unsafe extern "C-unwind" fn lua_pol(l: *mut c_void) -> i32 {
    if let Some(h) = hks() {
        (h.push_number)(l, if crate::spectateurs::lua_actif() { 1.0 } else { 0.0 });
        return 1;
    }
    0
}

unsafe extern "C-unwind" fn install_pol(l: *mut c_void) -> i32 {
    if let Some(h) = hks() {
        (h.push_closure)(l, lua_pol, 0, POL_NAME.as_ptr() as *const c_char, 0);
        (h.set_field)(l, LUA_GLOBALSINDEX, POL_NAME.as_ptr() as *const c_char);
    }
    0
}

/// Pose `Civ6RolloverProxy_Pol` dans la table globale de l'état qui compile le script. Absente, les
/// blocs du congrès ne font rien.
unsafe fn publish_pol(l: *mut c_void) {
    let r = match hks() {
        Some(h) => {
            let r = (h.cpcall)(l, install_pol, std::ptr::null_mut());
            if r != 0 {
                (h.set_top)(l, -2);
            }
            r
        }
        None => -1,
    };
    if r != 0 || !POL_LOGGED.swap(true, Ordering::Relaxed) {
        log(&if r == 0 { "ui : Civ6RolloverProxy_Pol posée".to_string() } else { format!("ui : Civ6RolloverProxy_Pol non posée ({r}), congrès inchangé") });
    }
}

/// Le script suivi du bloc d'optimisation (`opti`), puis du bloc de mesure (`mesure`), chacun
/// précédé de son contexte (`EpinglesOpti_Ctx`, `MesureEpingles_Ctx`). La mesure vient en dernier :
/// elle enveloppe les fonctions déjà optimisées. Octets laissés tels quels : les fichiers du jeu ne
/// sont pas tous en UTF-8.
pub fn patch_epingles(src: &[u8], ctx: &str, opti: bool, mesure: bool) -> Vec<u8> {
    let mut out = src.to_vec();
    if opti {
        out.extend_from_slice(format!("\r\nlocal EpinglesOpti_Ctx = \"{ctx}\";\r\n").as_bytes());
        out.extend_from_slice(OPTI_BLOCK.as_bytes());
    }
    if mesure {
        out.extend_from_slice(format!("\r\nlocal MesureEpingles_Ctx = \"{ctx}\";\r\n").as_bytes());
        out.extend_from_slice(MESURE_BLOCK.as_bytes());
    }
    out
}

/// DiplomacyDealView.lua suivi de `ui_diag_accords.lua`, qui ne fait qu'écrire dans Lua.log.
pub fn patch_diag_accords(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(DIAG_ACCORDS_BLOCK.as_bytes());
    out
}

/// ProductionHelper.lua suivi de `ui_merveilles.lua`.
pub fn patch_merveilles(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(MERVEILLES_BLOCK.as_bytes());
    out
}

/// TopPanel.lua suivi de `ui_rendements.lua`, qui appelle `OnRefreshYields` : le fichier doit la
/// définir. BSM (TopPanel_Spectator.lua) et les extensions incluent ce fichier, ils en profitent.
pub fn patch_top_panel(src: &[u8]) -> Result<Vec<u8>, String> {
    let has = |pat: &str| src.windows(pat.len()).any(|w| w == pat.as_bytes());
    if !has("function OnRefreshYields(") {
        return Err("OnRefreshYields introuvable".into());
    }
    if has("Civ6RolloverProxy (src/ui_rendements.lua)") {
        return Err("déjà corrigé".into());
    }
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(RENDEMENTS_BLOCK.as_bytes());
    Ok(out)
}

/// WorldCongressPopup.lua suivi de `ui_congres.lua` : la fenêtre du congrès en lecture seule pour
/// un spectateur. Le bloc lit les locales du fichier : on vérifie qu'elles y sont.
pub fn patch_congres(src: &[u8]) -> Result<Vec<u8>, String> {
    let has = |pat: &str| src.windows(pat.len()).any(|w| w == pat.as_bytes());
    if has("Civ6RolloverProxy (src/ui_congres.lua)") {
        return Err("déjà corrigé".into());
    }
    for pat in [
        "local m_IsInSession",
        "local m_CurrentPhase",
        "local PHASE_STEP_MAX",
        "local WORLD_CONGRESS_STAGE_1",
        "local m_kResolutionVotes",
        "function SetPhase(",
        "function OnLocalPlayerTurnEnd(",
        "function OnResumeCongress(",
    ] {
        if !has(pat) {
            return Err(format!("{pat} introuvable"));
        }
    }
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(CONGRES_BLOCK.as_bytes());
    Ok(out)
}

/// WorldCongress.lua (script de gameplay) précédé de `ui_congres_cibles.lua` : les cibles
/// « joueur » des résolutions tirées de `BGD_MajorsCibles()`, sans les spectateurs.
pub fn patch_congres_cibles(src: &[u8]) -> Result<Vec<u8>, String> {
    const APPEL: &str = "PlayerManager.GetAliveMajors()";
    let text = String::from_utf8_lossy(src);
    if text.contains("BGD_MajorsCibles") {
        return Err("déjà corrigé".into());
    }
    let n = text.matches(APPEL).count();
    if n == 0 {
        return Err(format!("{APPEL} introuvable"));
    }
    // Octets hors appel laissés tels quels : les fichiers du jeu ne sont pas tous en UTF-8.
    let mut out = CIBLES_BLOCK.as_bytes().to_vec();
    out.extend_from_slice(b"\r\n");
    let mut i = 0;
    while i < src.len() {
        if src[i..].starts_with(APPEL.as_bytes()) {
            out.extend_from_slice(b"BGD_MajorsCibles()");
            i += APPEL.len();
        } else {
            out.push(src[i]);
            i += 1;
        }
    }
    Ok(out)
}

type Fix = fn(&[u8]) -> Result<Vec<u8>, String>;

/// Corrections appliquées dès que l'accroche Lua est posée : fichier, correction, journal.
const FIXES: [(&str, Fix, &str); 3] = [
    ("CityPanel.lua", patch_city_panel, "owningPlayerID corrigé"),
    ("UnitFlagManager.lua", patch_unit_flags, "clic sur un drapeau en visée d'une ville ou d'un quartier"),
    ("ProductionPanel.lua", patch_production_panel, "production en cours affichée au changement de ville"),
];

/// Les cadres que les calques de ui_boutons.xml reproduisent : sans eux, les boutons déplacés ne
/// seraient plus à leur place, et on ne patche rien.
const FRAMES: [&str; 6] = [
    r#"<ZoomAnchor ID="Anchor" ZoomOffsetNear="0,0,0" ZoomOffsetFar="0,0,-20">"#,
    r#"<Container ID="Container" Anchor="C,C" Offset="0,-20" Size="200,41">"#,
    r#"<Container ID="EncampmentBannerContainer" Anchor="C,C" Size="106,34">"#,
    r#"<Container ID="WMDBannerContainer" Anchor="C,C" Size="88,34">"#,
    r#"<Container Anchor="L,C" Offset="25,0">"#,
    r#"<Container Anchor="R,C" Offset="25,0">"#,
];

/// Texte modifié de CityBannerInstances.xml : calques ajoutés, `ConsumeMouse` sur les boutons
/// nucléaires (le bouton d'attaque l'a déjà).
pub fn patch_xml(src: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(src).map_err(|_| "texte non UTF-8".to_string())?;
    if let Some(missing) = FRAMES.iter().find(|f| !text.contains(*f)) {
        return Err(format!("cadre introuvable : {missing}"));
    }
    let end = text.rfind("</Include>").ok_or("balise </Include> introuvable")?;
    let mut out = String::with_capacity(text.len() + XML_BLOCK.len() + 64);
    out.push_str(&text[..end]);
    out.push_str(XML_BLOCK);
    out.push_str(&text[end..]);
    for id in ["NukeBombButton", "ThermoNukeBombButton"] {
        let open = format!(r#"<Button ID="{id}""#);
        let at = out.find(&open).ok_or_else(|| format!("{id} introuvable"))?;
        let tag_end = out[at..].find('>').ok_or_else(|| format!("{id} : balise non fermée"))? + at;
        if !out[at..tag_end].contains("ConsumeMouse") {
            out.insert_str(at + open.len(), r#" ConsumeMouse="1""#);
        }
    }
    Ok(out.into_bytes())
}

// ---------------------------------------------------------------------------------------------
// Accroches.

pub(crate) type LoadBuffer = unsafe extern "C" fn(*mut c_void, *const u8, usize, *const c_char) -> i32;

/// Le vrai `luaL_loadbuffer`, une fois l'accroche posée : `mods_ui` y compile ses fichiers embarqués.
pub(crate) unsafe fn real_loadbuffer() -> Option<LoadBuffer> {
    let p = REAL_LOADBUFFER.load(Ordering::Relaxed);
    (p != 0).then(|| std::mem::transmute(p))
}

/// Correctifs de `ui` (sans les reprises de mods) : le texte modifié, ou None. Ce qui est fait va
/// dans `what`, ce qui ne l'est pas va au journal.
unsafe fn patch_ui(l: *mut c_void, chunk: &str, src: &[u8], what: &mut Vec<String>) -> Option<Vec<u8>> {
    let ctx = script_name(chunk);
    let opti = OPTI.load(Ordering::Relaxed) && OPTIMIZED.contains(&ctx.as_str());
    let mesure = MESURE.load(Ordering::Relaxed) && measured(chunk).is_some();
    if opti || mesure {
        if mesure {
            publish_clock(l);
        }
        what.push(
            match (opti, mesure) {
                (true, true) => "épingles allégées et mesurées",
                (true, false) => "épingles allégées",
                _ => "mesure des épingles ajoutée",
            }
            .into(),
        );
        return Some(patch_epingles(src, &ctx, opti, mesure));
    }
    if MERVEILLES.load(Ordering::Relaxed) && ctx == "productionhelper.lua" {
        what.push("rattrapage des merveilles ajouté".into());
        return Some(patch_merveilles(src));
    }
    let congres: [(&str, Fix, &str); 2] = [
        ("WorldCongressPopup.lua", patch_congres, "fenêtre du congrès en lecture seule pour les spectateurs"),
        ("WorldCongress.lua", patch_congres_cibles, "spectateurs retirés des cibles des résolutions"),
    ];
    if crate::spectateurs::poses() {
        for (file, patch, done) in congres {
            if !is_file(chunk, file) {
                continue;
            }
            publish_pol(l);
            return match patch(src) {
                Ok(text) => {
                    what.push(done.into());
                    Some(text)
                }
                Err(e) => {
                    log(&format!("ui : {chunk} laissé tel quel : {e}"));
                    None
                }
            };
        }
    }
    if crate::rendements_empire::enabled() && is_file(chunk, "TopPanel.lua") {
        return match patch_top_panel(src) {
            Ok(text) => {
                what.push("rendements redessinés après gouverneur ou personnage illustre".into());
                Some(text)
            }
            Err(e) => {
                log(&format!("ui : {chunk} laissé tel quel : {e}"));
                None
            }
        };
    }
    if DIAG_ACCORDS.load(Ordering::Relaxed) && is_file(chunk, "DiplomacyDealView.lua") {
        what.push("diagnostic des accords ajouté".into());
        return Some(patch_diag_accords(src));
    }
    if let Some((text, done)) = crate::depart::patch_lua(l, chunk, src) {
        what.push(done.into());
        return Some(text);
    }
    for (file, patch, done) in FIXES {
        if !is_file(chunk, file) {
            continue;
        }
        return match patch(src) {
            Ok(text) => {
                what.push(done.into());
                Some(text)
            }
            Err(e) => {
                log(&format!("ui : {chunk} laissé tel quel : {e}"));
                None
            }
        };
    }
    if !is_file(chunk, "CityBannerManager.lua") {
        return None;
    }
    let boutons = BOUTONS.load(Ordering::Relaxed) && XML_PATCHED.load(Ordering::Relaxed);
    if BOUTONS.load(Ordering::Relaxed) && !boutons {
        log("ui : calques absents de CityBannerInstances.xml, boutons laissés dans les bannières");
    }
    let (new, notes) = patch_lua(src, boutons, RECENTRAGE.load(Ordering::Relaxed));
    for n in &notes {
        log(&format!("ui : {chunk} : {n}"));
    }
    if new.is_some() {
        what.push(format!(
            "modifié (boutons {}, recentrage {})",
            if boutons { "au-dessus" } else { "inchangés" },
            if RECENTRAGE.load(Ordering::Relaxed) { "gardé" } else { "retiré" }
        ));
    }
    new
}

/// Trois étapes : le texte entier remplacé par une reprise de mod (`mods_ui::remplacer`), les
/// correctifs de `ui`, puis les blocs des reprises (`mods_ui::ajouter`). Une seule compilation.
unsafe extern "C" fn hook_loadbuffer(l: *mut c_void, buf: *const u8, size: usize, name: *const c_char) -> i32 {
    let real: LoadBuffer = std::mem::transmute(REAL_LOADBUFFER.load(Ordering::Relaxed));
    if name.is_null() || buf.is_null() {
        return real(l, buf, size, name);
    }
    let chunk = CStr::from_ptr(name).to_string_lossy();
    if JOURNAL.load(Ordering::Relaxed) {
        log(&format!("ui : script {chunk} ({size} octets)"));
    }
    let src = std::slice::from_raw_parts(buf, size);
    let entree_bandeau = crate::mods_ui::prendre_entree_bandeau();
    let mut what: Vec<String> = Vec::new();
    let mut mods = false;
    let mut text: Option<Vec<u8>> = None;
    match crate::mods_ui::remplacer(&chunk, src) {
        Some(Ok((t, w))) => {
            text = Some(t);
            what.push(w);
            mods = true;
        }
        Some(Err(e)) => log(&format!("ui : {chunk} : reprise de mod impossible : {e}")),
        None => {}
    }
    if let Some(t) = patch_ui(l, &chunk, text.as_deref().unwrap_or(src), &mut what) {
        text = Some(t);
    }
    match crate::mods_ui::ajouter(&chunk, text.as_deref().unwrap_or(src), entree_bandeau) {
        Some(Ok((t, w))) => {
            text = Some(t);
            what.push(w);
            mods = true;
        }
        Some(Err(e)) => log(&format!("ui : {chunk} : reprise de mod laissée de côté : {e}")),
        None => {}
    }
    let Some(text) = text else { return real(l, buf, size, name) };
    if mods {
        crate::mods_ui::publier(l);
    }
    let r = real(l, text.as_ptr(), text.len(), name);
    log(&format!(
        "ui : {chunk} : {}, compilation {}",
        what.join(" ; "),
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
    if JOURNAL.load(Ordering::Relaxed) {
        log(&format!("ui : XML {name}"));
    }
    crate::mods_ui::xml_lu(&name);
    if let Some(res) = crate::mods_ui::xml(&name, (*buf).text().unwrap_or(&[])) {
        match res {
            Ok((text, what)) if replace_buffer(&mut *buf, &text) => log(&format!("ui : {name} : {what}")),
            Ok(_) => log(&format!("ui : {name} : allocation impossible, laissé tel quel")),
            Err(e) => log(&format!("ui : {name} laissé tel quel : {e}")),
        }
        return r;
    }
    if !BOUTONS.load(Ordering::Relaxed) || !is_file(&name, "CityBannerInstances.xml") {
        return r;
    }
    let b = &mut *buf;
    XML_PATCHED.store(false, Ordering::Relaxed);
    let Some(src) = b.text() else { return r };
    match patch_xml(src) {
        Ok(text) if replace_buffer(b, &text) => {
            XML_PATCHED.store(true, Ordering::Relaxed);
            log(&format!("ui : {name} : calques ajoutés"));
        }
        Ok(_) => log(&format!("ui : {name} : allocation impossible, laissé tel quel")),
        Err(e) => log(&format!("ui : {name} laissé tel quel : {e}")),
    }
    r
}

pub unsafe fn on_gamecore_loaded() {
    if !active() || IAT_SLOT.load(Ordering::Relaxed) != 0 {
        return;
    }
    let exe = win::GetModuleHandleW(std::ptr::null()) as usize;

    // XML d'abord : le bloc Lua dépend des calques.
    if BOUTONS.load(Ordering::Relaxed) || JOURNAL.load(Ordering::Relaxed) || crate::mods_ui::xml_needed() {
        // Si le proxy de démarrage est installé, la lecture est déjà détournée : on s'enchaîne.
        match exe::hook_read_xml(exe, hook_read_xml as *const () as usize) {
            Ok(h) => {
                XML_TRAMP.store(h.tramp, Ordering::Relaxed);
                if h.chained {
                    log("ui : lecture XML déjà détournée (proxy de démarrage), détour enchaîné");
                }
                *XML_HOOK.lock().unwrap() = Some(h);
            }
            Err(e) => log(&format!("ui : {e}, pas de calques")),
        }
    }

    let Some(slot) = iat_slot(exe, "HavokScript_FinalRelease.dll", "luaL_loadbuffer") else {
        log("ui : import luaL_loadbuffer introuvable dans l'exe, interface inchangée");
        return;
    };
    REAL_LOADBUFFER.store(*(slot as *const usize), Ordering::Relaxed);
    if !write_slot(slot, hook_loadbuffer as *const () as usize) {
        log("ui : VirtualProtect a échoué sur la table d'imports");
        return;
    }
    IAT_SLOT.store(slot, Ordering::Relaxed);
    log(&format!(
        "ui : accroches posées (lecture XML {}), boutons {}, recentrage {}, journal {}, mesure des épingles {}, épingles allégées {}, merveilles {}, rendements {}",
        if XML_TRAMP.load(Ordering::Relaxed) != 0 { "oui" } else { "non" },
        if BOUTONS.load(Ordering::Relaxed) { "on" } else { "off" },
        if RECENTRAGE.load(Ordering::Relaxed) { "on" } else { "off" },
        if JOURNAL.load(Ordering::Relaxed) { "on" } else { "off" },
        if MESURE.load(Ordering::Relaxed) { "on" } else { "off" },
        if OPTI.load(Ordering::Relaxed) { "on" } else { "off" },
        if MERVEILLES.load(Ordering::Relaxed) { "on" } else { "off" },
        if crate::rendements_empire::enabled() { "on" } else { "off" },
    ));
    if crate::mods_ui::any() {
        log(&format!("ui : mods repris : {}", crate::mods_ui::journal_reglages()));
    }
}

/// Le proxy est déchargé alors que l'exe reste en mémoire : on rend l'import et la lecture XML.
pub unsafe fn on_proxy_unload() {
    let slot = IAT_SLOT.swap(0, Ordering::Relaxed);
    if slot != 0 && *(slot as *const usize) == hook_loadbuffer as *const () as usize {
        write_slot(slot, REAL_LOADBUFFER.load(Ordering::Relaxed));
    }
    if let Some(h) = XML_HOOK.lock().ok().and_then(|mut g| g.take()) {
        exe::unhook_read_xml(&h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exe::{find_sig, is_import, rd};

    const GAME: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI";
    const BSM: &str = r"C:\Program Files (x86)\Steam\steamapps\workshop\content\289070\1916397407";

    fn read(path: String) -> Option<Vec<u8>> {
        let r = std::fs::read(&path).ok();
        if r.is_none() {
            eprintln!("absent, test sauté : {path}");
        }
        r
    }

    #[test]
    fn noms_de_fichiers() {
        assert!(is_file("C:/x/UI/Replacements/CityBannerManager.lua", "CityBannerManager.lua"));
        assert!(is_file(r"..\..\DLC\citybannermanager.lua", "CityBannerManager.lua"));
        assert!(!is_file("C:/x/CityBannerManager_Expansion2.lua", "CityBannerManager.lua"));
        assert!(!is_file("C:/x/MyCityBannerManager.lua", "CityBannerManager.lua"));
    }

    #[test]
    fn diagnostic_des_accords_ajoute_a_l_ecran_d_accord() {
        let out = String::from_utf8(patch_diag_accords(b"Initialize();")).unwrap();
        assert!(out.starts_with("Initialize();\r\n-- ====="));
        assert!(out.ends_with(DIAG_ACCORDS_BLOCK));
        assert!(is_file(r"C:\x\DLC\Expansion2\UI\Replacements\DiplomacyDealView.lua", "DiplomacyDealView.lua"));
        assert!(!is_file(r"C:\x\ui\Replacements\diplomacydealview_MPH.lua", "DiplomacyDealView.lua"));
    }

    #[test]
    fn merveilles_ajoutees_a_production_helper() {
        assert_eq!(script_name(r"C:\x\Base\Assets\UI\ProductionHelper.lua"), "productionhelper.lua");
        let out = String::from_utf8(patch_merveilles(b"x = 1")).unwrap();
        assert!(out.starts_with("x = 1\r\n-- ====="));
        assert!(out.ends_with(MERVEILLES_BLOCK));
        let Some(src) = read(format!(r"{GAME}\Base\Assets\UI\ProductionHelper.lua")) else { return };
        assert!(String::from_utf8_lossy(&src).contains("function SwapQueueItem(sourceIndex, destIndex)"));
    }

    #[test]
    fn scripts_mesures() {
        assert_eq!(measured(r"C:\x\Base\Assets\UI\MapPinManager.lua").as_deref(), Some("mappinmanager.lua"));
        assert_eq!(measured("ui/mappinmanager_dmt.lua").as_deref(), Some("mappinmanager_dmt.lua"));
        assert_eq!(measured(r"..\DLC\Expansion2\UI\Replacements\DiplomacyRibbon_Expansion2.lua").as_deref(), Some("diplomacyribbon_expansion2.lua"));
        assert_eq!(measured("@MapPinListPanel.lua").as_deref(), Some("mappinlistpanel.lua"));
        assert!(measured("C:/x/MapPinPopup.lua").is_none());
        assert!(measured("C:/x/DiplomacyRibbon.xml").is_none());
        assert!(measured(r"C:\x\Base\Assets\UI\DiplomacyRibbonSupport.lua").is_none());
        assert_eq!(measured(r"C:\x\diplomacyribbon_spec.lua").as_deref(), Some("diplomacyribbon_spec.lua"));
        let out = patch_epingles(b"x = 1", "mappinmanager.lua", false, true);
        assert!(out.starts_with(b"x = 1\r\nlocal MesureEpingles_Ctx = \"mappinmanager.lua\";\r\n"));
        assert!(out.ends_with(MESURE_BLOCK.as_bytes()));
        // Optimisation d'abord, mesure ensuite : la mesure enveloppe les fonctions optimisées.
        let both = String::from_utf8(patch_epingles(b"x = 1", "mappinlistpanel.lua", true, true)).unwrap();
        let (o, m) = (both.find("local EpinglesOpti_Ctx = \"mappinlistpanel.lua\";").unwrap(), both.find("local MesureEpingles_Ctx").unwrap());
        assert!(o < m && both.ends_with(MESURE_BLOCK));
        assert_eq!(patch_epingles(b"x = 1", "a.lua", false, false), b"x = 1");
        assert!(!OPTIMIZED.contains(&script_name(r"C:\x\DiplomacyRibbon.lua").as_str()));
    }

    #[test]
    fn recentrage() {
        let src = "function OnICBMStrikeButtonClick( iPlotID, eWMD )\n\tUI.SelectCity(pCity);\nend\nfunction X()\n\tUI.SelectCity(a);\nend\n";
        let mut t = src.to_string();
        no_recenter(&mut t, "OnICBMStrikeButtonClick", "UI.SelectCity(").unwrap();
        assert!(t.contains("UI.SelectCity( pCity, false );"));
        assert!(t.contains("UI.SelectCity(a);"), "seule la fonction visée change");
        let mut t2 = t.clone();
        assert!(no_recenter(&mut t2, "OnICBMStrikeButtonClick", "UI.SelectCity(").is_err(), "pas deux fois");
        assert!(no_recenter(&mut t2, "Absente", "UI.SelectCity(").is_err());
    }

    fn check_lua(path: String) {
        let Some(src) = read(path) else { return };
        let (new, notes) = patch_lua(&src, true, false);
        assert!(notes.is_empty(), "{notes:?}");
        let new = String::from_utf8(new.unwrap()).unwrap();
        assert!(new.contains("UI.SelectCity( pCity, false )"));
        assert!(new.contains("UI.SelectDistrict( pDistrict, false )"));
        // Le fichier en contient déjà une, en commentaire : « Don't auto center ».
        let before = String::from_utf8_lossy(&src).matches(", false )").count();
        assert_eq!(new.matches(", false )").count(), before + 3);
        assert!(new.ends_with(LUA_BLOCK));
        // Les noms dont le bloc a besoin existent dans ce fichier.
        for name in ["local m_CityBannerIM", "local m_EncampmentBannerIM", "local m_WMDBannerIM", "local YOFFSET_2DVIEW",
            "local ZOFFSET_3DVIEW", "function CityBanner:UpdatePosition", "function CityBanner:UpdateVisibility",
            "function CityBanner:UpdateRangeStrike", "function CityBanner:UpdateWMDBanner", "function CityBanner:Uninitialize",
            "function OnShutdown"] {
            assert!(new.contains(name), "{name} absent");
        }
    }

    #[test]
    fn lua_du_jeu() {
        check_lua(format!(r"{GAME}\DLC\Expansion2\UI\CityBanners\CityBannerManager.lua"));
    }

    #[test]
    fn lua_de_bsm() {
        check_lua(format!(r"{BSM}\UI\Replacements\CityBannerManager.lua"));
    }

    #[test]
    fn xml_du_jeu() {
        let Some(src) = read(format!(r"{GAME}\DLC\Expansion2\UI\CityBanners\CityBannerInstances.xml")) else { return };
        let new = String::from_utf8(patch_xml(&src).unwrap()).unwrap();
        assert!(new.contains(r#"<Button ID="NukeBombButton" ConsumeMouse="1" Anchor="C,C""#));
        assert!(new.contains(r#"<Button ID="ThermoNukeBombButton" ConsumeMouse="1" Anchor="C,C""#));
        assert!(new.contains(r#"<Instance Name="ProxyStrikeWMD">"#));
        assert!(new.trim_end().ends_with("</Include>"));
        // Trois instances ajoutées, chacune fermée.
        let old = String::from_utf8_lossy(&src);
        for (open, close) in [("<Instance ", "</Instance>"), ("<ZoomAnchor", "</ZoomAnchor>")] {
            assert_eq!(new.matches(open).count(), old.matches(open).count() + 3, "{open}");
            assert_eq!(new.matches(close).count(), old.matches(close).count() + 3, "{close}");
        }
        assert!(patch_xml(b"<Include></Include>").is_err());
    }

    /// La table d'imports du binaire de test : son entrée pour GetModuleHandleW pointe sur la
    /// fonction que rend GetProcAddress.
    #[test]
    fn table_d_imports() {
        unsafe {
            let exe = win::GetModuleHandleW(std::ptr::null()) as usize;
            let slot = iat_slot(exe, "kernel32.dll", "GetModuleHandleW").expect("import introuvable");
            let k32 = win::GetModuleHandleW(win::wide("kernel32.dll").as_ptr());
            assert_eq!(*(slot as *const usize), win::GetProcAddress(k32, b"GetModuleHandleW\0".as_ptr()) as usize);
            assert!(iat_slot(exe, "kernel32.dll", "PasUneFonction").is_none());
            assert!(iat_slot(exe, "HavokScript_FinalRelease.dll", "luaL_loadbuffer").is_none());
        }
    }

    #[test]
    fn city_panel_du_jeu() {
        let Some(src) = read(format!(r"{GAME}\Base\Assets\UI\Panels\CityPanel.lua")) else { return };
        let new = String::from_utf8(patch_city_panel(&src).unwrap()).unwrap();
        assert!(!new.contains("owningPlayerID"));
        assert_eq!(new.len(), src.len() - 1, "owning → owner, rien d'autre");
        assert!(patch_city_panel(new.as_bytes()).is_err(), "pas deux fois");
    }

    fn check_flags(path: String) {
        let Some(src) = read(path) else { return };
        let bytes = patch_unit_flags(&src).unwrap();
        // Rien d'autre ne change : le bloc, plus deux noms de fonction plus longs.
        let longer = "Civ6RolloverProxy_OnUnitFlagLClick".len() - "OnUnitFlagClick".len();
        assert_eq!(bytes.len(), src.len() + FLAG_BLOCK.len() + 2 * longer);
        let new = String::from_utf8_lossy(&bytes);
        assert_eq!(new.matches("RegisterCallback( Mouse.eLClick, Civ6RolloverProxy_OnUnitFlagLClick )").count(), 2);
        assert_eq!(new.matches("RegisterCallback( Mouse.eRClick, OnUnitFlagClick )").count(), 2, "clic droit inchangé");
        // Le gestionnaire est défini avant OnUnitFlagClick, donc avant tout branchement.
        assert!(new.find("function Civ6RolloverProxy_OnUnitFlagLClick").unwrap() < new.find("\nfunction OnUnitFlagClick(").unwrap());
        assert!(patch_unit_flags(&bytes).is_err(), "pas deux fois");
    }

    #[test]
    fn drapeaux_du_jeu() {
        check_flags(format!(r"{GAME}\Base\Assets\UI\UnitFlagManager.lua"));
    }

    #[test]
    fn top_panel_du_jeu() {
        let Some(src) = read(format!(r"{GAME}\DLC\Expansion2\UI\Replacements\TopPanel.lua")) else { return };
        let new = patch_top_panel(&src).unwrap();
        assert!(new.starts_with(&src) && new.ends_with(RENDEMENTS_BLOCK.as_bytes()));
        assert!(patch_top_panel(&new).is_err(), "pas deux fois");
        assert!(patch_top_panel(b"function RefreshYields() end").is_err());
        assert!(is_file(r"C:\x\DLC\Expansion2\UI\Replacements\TopPanel.lua", "TopPanel.lua"));
        assert!(!is_file("ui/topPanel_spectator.lua", "TopPanel.lua"));
    }

    #[test]
    fn congres_du_jeu() {
        let Some(src) = read(format!(r"{GAME}\DLC\Expansion2\UI\Additions\WorldCongressPopup.lua")) else { return };
        let new = patch_congres(&src).unwrap();
        assert!(new.starts_with(&src) && new.ends_with(CONGRES_BLOCK.as_bytes()));
        assert!(patch_congres(&new).is_err(), "pas deux fois");
        assert!(patch_congres(b"function SetPhase() end").is_err());
        assert!(is_file(r"C:\x\DLC\Expansion2\UI\Additions\WorldCongressPopup.lua", "WorldCongressPopup.lua"));
        assert!(!is_file(r"C:\x\DLC\Expansion2\UI\Additions\WorldCongressPopup.lua", "WorldCongress.lua"));
    }

    #[test]
    fn cibles_du_congres_du_jeu() {
        let Some(src) = read(format!(r"{GAME}\DLC\Expansion2\Scripts\WorldCongress.lua")) else { return };
        let new = patch_congres_cibles(&src).unwrap();
        let text = String::from_utf8_lossy(&new);
        assert!(text.starts_with(CIBLES_BLOCK));
        // Les cinq validateurs à cible joueur ; le reste vient du bloc lui-même.
        let corps = &text[CIBLES_BLOCK.len()..];
        assert_eq!(corps.matches("BGD_MajorsCibles()").count(), 5);
        assert_eq!(corps.matches("PlayerManager.GetAliveMajors()").count(), 0);
        assert_eq!(new.len() + 5 * "PlayerManager.GetAliveMajors()".len(), src.len() + CIBLES_BLOCK.len() + 2 + 5 * "BGD_MajorsCibles()".len());
        assert!(patch_congres_cibles(&new).is_err(), "pas deux fois");
        assert!(patch_congres_cibles(b"local x = 1").is_err());
    }

    #[test]
    fn production_panel_du_jeu() {
        let Some(src) = read(format!(r"{GAME}\Base\Assets\UI\Panels\ProductionPanel.lua")) else { return };
        let new = patch_production_panel(&src).unwrap();
        assert!(new.starts_with(&src) && new.ends_with(PRODUCTION_BLOCK.as_bytes()));
        assert!(patch_production_panel(&new).is_err(), "pas deux fois");
        assert!(patch_production_panel(b"function View(data) end").is_err());
        assert!(is_file(r"C:\x\Base\Assets\UI\Panels\ProductionPanel.lua", "ProductionPanel.lua"));
        assert!(!is_file("ui/replacements/productionpanel_bbg.lua", "ProductionPanel.lua"));
    }

    #[test]
    fn drapeaux_de_bsm() {
        check_flags(format!(r"{BSM}\UI\Replacements\unitflagmanager.lua"));
    }

    #[test]
    fn noms_decores() {
        let mangled = b"?luaL_loadbuffer@@YAHPEAUlua_State@@PEBD_K1@Z";
        assert!(is_import(mangled, "luaL_loadbuffer"));
        assert!(is_import(b"luaL_loadbuffer", "luaL_loadbuffer"));
        assert!(!is_import(b"?luaL_loadbufferx@@YAHPEAUlua_State@@PEBD_K1@Z", "luaL_loadbuffer"));
        assert!(!is_import(b"?luaL_loadstring@@YAHPEAUlua_State@@PEBD@Z", "luaL_loadbuffer"));
    }

    /// L'entrée luaL_loadbuffer des vrais exe, projetés comme images sans être exécutés : celle que
    /// Ghidra montre en DX12 (`140fa54e8`).
    #[test]
    fn import_havokscript_des_exe() {
        #[link(name = "kernel32")]
        extern "system" {
            fn LoadLibraryExW(name: *const u16, file: *mut c_void, flags: u32) -> win::HMODULE;
            fn FreeLibrary(module: win::HMODULE) -> i32;
        }
        const LOAD_LIBRARY_AS_IMAGE_RESOURCE: u32 = 0x20;
        for exe in ["CivilizationVI_DX12.exe", "CivilizationVI.exe"] {
            let path = format!(r"{GAME}\Base\Binaries\Win64Steam\{exe}");
            if !std::path::Path::new(&path).exists() {
                eprintln!("absent, test sauté : {path}");
                continue;
            }
            unsafe {
                let h = LoadLibraryExW(win::wide(&path).as_ptr(), std::ptr::null_mut(), LOAD_LIBRARY_AS_IMAGE_RESOURCE);
                assert!(!h.is_null(), "{exe}");
                // Les bits bas du handle signalent une image projetée en ressource.
                let base = h as usize & !3;
                let slot = iat_slot(base, "HavokScript_FinalRelease.dll", "luaL_loadbuffer").unwrap_or_else(|| panic!("{exe}"));
                if exe == "CivilizationVI_DX12.exe" {
                    assert_eq!(slot - base, 0xfa54e8);
                }
                FreeLibrary(h);
            }
        }
    }

    #[test]
    fn signature_xml() {
        for (exe, expected) in [("CivilizationVI_DX12.exe", 0xa2d280usize), ("CivilizationVI.exe", 0xa1f3f0)] {
            let Some(bytes) = read(format!(r"{GAME}\Base\Binaries\Win64Steam\{exe}")) else { continue };
            // Adresse virtuelle relative de chaque occurrence, par les en-têtes de sections.
            let base = bytes.as_ptr() as usize;
            let (nt, hits) = unsafe { (base + rd::<u32>(base + 0x3c) as usize, find_sig(&bytes, false)) };
            assert_eq!(hits.len(), 1, "{exe}");
            let rva = unsafe {
                let count = rd::<u16>(nt + 6) as usize;
                let first = nt + 24 + rd::<u16>(nt + 20) as usize;
                (0..count)
                    .map(|i| first + 40 * i)
                    .find_map(|s| {
                        let (va, raw, rawsize) = (rd::<u32>(s + 12) as usize, rd::<u32>(s + 20) as usize, rd::<u32>(s + 16) as usize);
                        (raw..raw + rawsize).contains(&hits[0]).then(|| hits[0] - raw + va)
                    })
                    .unwrap()
            };
            assert_eq!(rva, expected, "{exe}");
        }
    }
}
