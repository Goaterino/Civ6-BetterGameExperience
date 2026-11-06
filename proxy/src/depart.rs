//! Départ et fin de tour simultanés (option de partie « No First/Last moves (BGD) », voir
//! depart_tour/README.md). Entre l'activation locale du tour et le départ donné par l'hôte, les
//! ordres du joueur ne partent pas et son minuteur de tour est gelé. Rien ne touche la simulation :
//! un ordre retenu n'existe nulle part, un minuteur est local.
//!
//! - **Envoi des ordres** (GameCore) : tout message sortant passe par `1800366b0`
//!   (`Messages_Manager.cpp`, journal `GameCore SEND: %s`), qui le sérialise puis le remet à l'exe.
//!   L'ordre local n'est exécuté qu'à son retour du réseau. Le détour ne rappelle pas l'original
//!   pour les ordres du joueur pendant l'attente (`bloque`).
//! - **Minuteur** (exe) : `1400f8590` (DX12), retrouvée par signature dans les deux exe, met en
//!   pause ou relance les deux chronos du gestionnaire de tour (départ, cumul, en marche, en
//!   `+0x18/+0x20/+0x28` et `+0x30/+0x38/+0x40`). Le temps écoulé (`1400f4df0`) qu'affichent
//!   l'interface et que lit la fin de tour automatique en dépend. Pendant l'attente, le détour les
//!   tient arrêtés à zéro ; au départ, il les remet à zéro puis laisse l'original les relancer.
//!
//! Le protocole (signaux du chat caché `.bgd_pret` / `.bgd_depart`) est en Lua : `ui_depart.lua`,
//! ajouté à `InGame.lua`, et `ui_depart_chat.lua`, ajouté à `ChatPanel.lua`, qui cache ces signaux
//! sauf avec `depart_signaux=on`.

use crate::exe::{is_file, rd, text_section};
use crate::patch::{self, Site};
use crate::{log, pol, win};
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

const ENVOI: Site = Site {
    name: "envoi des messages de GameCore",
    address: 0x1_8003_66b0,
    // push rbp / push rbx / push rdi / push r14 / push r15 / lea rbp,[rsp-5b0h]
    expected: &[0x40, 0x55, 0x53, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8d, 0xac, 0x24, 0x50, 0xfa, 0xff, 0xff],
    copy: (0, 16),
    resume: 16,
};

/// Pause et reprise des chronos du tour, `1400f8590` en DX12 : prologue (15 octets sans
/// instruction relative), appels à `140100ce0`, `140102d80`, `140101ce0` (rel32 libres), test de
/// `TURNTIMER_NONE` (`0xa5196dab`), puis lecture de `+0x28` et `+0x18`. Unique dans les deux exe.
const MINUTEUR_SIG: &[Option<u8>] = &sig(&[
    "48", "89", "5c", "24", "08", "48", "89", "74", "24", "10", "57", "48", "83", "ec", "20", "48", "8b", "f1", "e8", "?", "?", "?",
    "?", "48", "8b", "c8", "48", "8b", "d8", "e8", "?", "?", "?", "?", "48", "8b", "cb", "0f", "b6", "f8", "e8", "?", "?", "?", "?",
    "3d", "ab", "6d", "19", "a5", "0f", "95", "c0", "40", "84", "ff", "75", "5c", "84", "c0", "74", "58", "40", "38", "7e", "28",
    "75", "1c", "48", "8b", "5e", "18", "e8",
]);
const MINUTEUR_PROLOGUE: usize = 15;
/// Chronos : (départ QPC, cumul en secondes f64, en marche u8).
const CHRONOS: [(usize, usize, usize); 2] = [(0x18, 0x20, 0x28), (0x30, 0x38, 0x40)];

const fn hex(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("chiffre hexadécimal attendu"),
    }
}

const fn sig<const N: usize>(s: &[&str; N]) -> [Option<u8>; N] {
    let mut out = [None; N];
    let mut i = 0;
    while i < N {
        let b = s[i].as_bytes();
        if b[0] != b'?' {
            out[i] = Some(hex(b[0]) * 16 + hex(b[1]));
        }
        i += 1;
    }
    out
}

pub fn find_sig(hay: &[u8], pat: &[Option<u8>]) -> Vec<usize> {
    hay.windows(pat.len())
        .enumerate()
        .filter(|(_, w)| w.iter().zip(pat).all(|(a, b)| b.is_none_or(|b| *a == b)))
        .map(|(i, _)| i)
        .collect()
}

// Types de message (`+0x08`, u32) que le joueur ne peut pas envoyer pendant l'attente. Relevés
// dans leurs constructeurs (depart_tour/README.md). `CivicsCommand` (choix de recherche ou de
// dogme), `PlayerTurnComplete`, `GameTurnComplete`, `GameUpdate` passent.
const UNIT_COMMAND: u32 = 0x4938_30af;
const UNIT_OPERATION: u32 = 0x8c0f_3f36;
const CITY_COMMAND: u32 = 0x346b_0a13;
const CITY_OPERATION: u32 = 0x7ce9_adc2;
const DIPLOMACY_ACTION: u32 = 0xa797_ad1c;
const DIPLOMACY_SESSION: u32 = 0x08a6_cf5f;
const DIPLOMACY_STATEMENT: u32 = 0xa417_2391;
const PLAYER_OPERATION: u32 = 0xc45d_e12f;
const BLOQUES: [u32; 7] =
    [UNIT_COMMAND, UNIT_OPERATION, CITY_COMMAND, CITY_OPERATION, DIPLOMACY_ACTION, DIPLOMACY_SESSION, DIPLOMACY_STATEMENT];
/// `PlayerOperation` : joueur en `+0x18`, opération en `+0x20`.
const PO_JOUEUR: usize = 0x18;
const PO_OPERATION: usize = 0x20;
/// Nom du message (`char*`), pour le journal.
const M_NOM: usize = 0x10;

/// Garde-fou : sans départ reçu, l'attente se lève d'elle-même au premier ordre tenté après ce
/// délai (hôte parti, migration ratée, signal perdu).
const SECURITE_S: f64 = 120.0;
const MAX_LOGS: u32 = 30;

static GC_BASE: AtomicUsize = AtomicUsize::new(0);
static ENVOI_TRAMP: AtomicUsize = AtomicUsize::new(0);
static MINUTEUR_SITE: AtomicUsize = AtomicUsize::new(0);
static MINUTEUR_TRAMP: AtomicUsize = AtomicUsize::new(0);
/// Gestionnaire du tour, vu au dernier appel de la pause (`this`).
static MINUTEUR_THIS: AtomicUsize = AtomicUsize::new(0);

static ATTENTE: AtomicBool = AtomicBool::new(false);
static DEPUIS: AtomicI64 = AtomicI64::new(0);
static JOUEUR: AtomicI32 = AtomicI32::new(-1);
static EXECUTE_SCRIPT: AtomicU32 = AtomicU32::new(0);
static REJETS: AtomicU32 = AtomicU32::new(0);
static LOGS: AtomicU32 = AtomicU32::new(0);
static SIGNAUX: AtomicBool = AtomicBool::new(false);

/// `depart_signaux=on|off` (off) : les signaux `.bgd_` restent visibles dans le chat.
pub fn set_signaux(on: bool) {
    SIGNAUX.store(on, Ordering::Relaxed);
}

fn qpc_freq() -> f64 {
    let mut f = 0i64;
    unsafe { win::QueryPerformanceFrequency(&mut f) };
    f.max(1) as f64
}

fn attente_s() -> f64 {
    (win::qpc() - DEPUIS.load(Ordering::Relaxed)) as f64 / qpc_freq()
}

/// Le message est-il un ordre du joueur, retenu pendant l'attente ?
unsafe fn bloque(msg: usize) -> bool {
    let kind = rd::<u32>(msg + 8);
    if BLOQUES.contains(&kind) {
        return true;
    }
    kind == PLAYER_OPERATION
        && rd::<i32>(msg + PO_JOUEUR) == JOUEUR.load(Ordering::Relaxed)
        && rd::<u32>(msg + PO_OPERATION) != EXECUTE_SCRIPT.load(Ordering::Relaxed)
}

unsafe fn nom(msg: usize) -> String {
    let p = rd::<usize>(msg + M_NOM);
    if p == 0 { "?".into() } else { CStr::from_ptr(p as *const c_char).to_string_lossy().into_owned() }
}

type Fn8 = unsafe extern "system" fn(usize, usize, usize, usize, usize, usize, usize, usize) -> usize;

unsafe extern "system" fn hook_envoi(a: usize, b: usize, c: usize, d: usize, e: usize, f: usize, g: usize, h: usize) -> usize {
    if a != 0 && ATTENTE.load(Ordering::Relaxed) && bloque(a) {
        if attente_s() > SECURITE_S {
            partir("délai de sécurité dépassé");
        } else {
            let n = REJETS.fetch_add(1, Ordering::Relaxed) + 1;
            if LOGS.fetch_add(1, Ordering::Relaxed) < MAX_LOGS {
                log(&format!("depart : {} retenu, tour pas encore lancé ({n})", nom(a)));
            }
            return 0;
        }
    }
    let real: Fn8 = std::mem::transmute(ENVOI_TRAMP.load(Ordering::Relaxed));
    real(a, b, c, d, e, f, g, h)
}

/// Arrête les deux chronos à zéro : le temps écoulé reste nul, le minuteur affiche tout son temps.
unsafe fn chronos_a_zero(this: usize) {
    let now = win::qpc();
    for (depart, cumul, marche) in CHRONOS {
        *((this + depart) as *mut i64) = now;
        *((this + cumul) as *mut f64) = 0.0;
        *((this + marche) as *mut u8) = 0;
    }
}

unsafe extern "system" fn hook_minuteur(this: usize) {
    if this != 0 {
        MINUTEUR_THIS.store(this, Ordering::Relaxed);
        if ATTENTE.load(Ordering::Relaxed) {
            chronos_a_zero(this);
            return;
        }
    }
    let real: unsafe extern "system" fn(usize) = std::mem::transmute(MINUTEUR_TRAMP.load(Ordering::Relaxed));
    real(this)
}

/// Entre en attente : ordres du joueur `joueur` retenus, minuteur gelé à zéro.
unsafe fn attendre(joueur: i32, execute_script: u32) {
    JOUEUR.store(joueur, Ordering::Relaxed);
    EXECUTE_SCRIPT.store(execute_script, Ordering::Relaxed);
    REJETS.store(0, Ordering::Relaxed);
    DEPUIS.store(win::qpc(), Ordering::Relaxed);
    ATTENTE.store(true, Ordering::Relaxed);
    let this = MINUTEUR_THIS.load(Ordering::Relaxed);
    if this != 0 && MINUTEUR_TRAMP.load(Ordering::Relaxed) != 0 {
        chronos_a_zero(this);
    }
    log(&format!("depart : attente (joueur {joueur})"));
}

/// Fin de tour forcée par l'hôte (option « No First/Last moves (BGD) ») : ce temps est ajouté au cumul
/// des deux chronos. Le temps écoulé dépasse la limite, l'exe envoie lui-même le `TurnComplete` du
/// joueur local à l'image suivante (`1400f8170`) et refuse d'annuler la fin de tour (`1401c9c10`).
const FORCE_S: f64 = 1.0e6;
static FORCE: AtomicBool = AtomicBool::new(false);

/// Force la fin du tour local. Rend faux si le minuteur n'a pas encore été vu.
unsafe fn fin_forcer() -> bool {
    let this = MINUTEUR_THIS.load(Ordering::Relaxed);
    if this == 0 || MINUTEUR_TRAMP.load(Ordering::Relaxed) == 0 {
        return false;
    }
    if ATTENTE.swap(false, Ordering::Relaxed) {
        log("depart : attente levée par la fin du tour");
    }
    for (_, cumul, _) in CHRONOS {
        let p = (this + cumul) as *mut f64;
        if *p < FORCE_S {
            *p += FORCE_S;
        }
    }
    FORCE.store(true, Ordering::Relaxed);
    log("depart : fin du tour forcée (minuteur de l'hôte écoulé)");
    true
}

/// Début du tour suivant : on retire ce qu'on a ajouté, sur les chronos que le jeu n'a pas remis à
/// zéro.
unsafe fn fin_lever() {
    if !FORCE.swap(false, Ordering::Relaxed) {
        return;
    }
    let this = MINUTEUR_THIS.load(Ordering::Relaxed);
    if this == 0 {
        return;
    }
    for (_, cumul, _) in CHRONOS {
        let p = (this + cumul) as *mut f64;
        if *p >= FORCE_S {
            *p -= FORCE_S;
        }
    }
}

/// Lève l'attente : ordres libres, minuteur relancé de zéro (sauf partie en pause, que l'original
/// respecte). Rend vrai si l'on attendait.
fn partir(pourquoi: &str) -> bool {
    if !ATTENTE.swap(false, Ordering::Relaxed) {
        return false;
    }
    unsafe {
        let this = MINUTEUR_THIS.load(Ordering::Relaxed);
        let tramp = MINUTEUR_TRAMP.load(Ordering::Relaxed);
        if this != 0 && tramp != 0 {
            chronos_a_zero(this);
            let real: unsafe extern "system" fn(usize) = std::mem::transmute(tramp);
            real(this);
        }
    }
    log(&format!("depart : parti ({pourquoi}, {:.1} s d'attente, {} ordres retenus)", attente_s(), REJETS.load(Ordering::Relaxed)));
    true
}

// ---------------------------------------------------------------------------------------------
// Fonctions natives, posées dans les contextes de `ui_depart.lua` et `ui_depart_chat.lua`.

type LuaCFn = unsafe extern "C-unwind" fn(*mut c_void) -> i32;

struct Hks {
    push_closure: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, i32, *const c_char, i32),
    set_field: unsafe extern "C-unwind" fn(*mut c_void, i32, *const c_char),
    cpcall: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, *mut c_void) -> i32,
    set_top: unsafe extern "C-unwind" fn(*mut c_void, i32),
    to_number: unsafe extern "C-unwind" fn(*mut c_void, i32) -> f64,
    to_lstring: unsafe extern "C-unwind" fn(*mut c_void, i32, *mut usize) -> *const c_char,
    push_number: unsafe extern "C-unwind" fn(*mut c_void, f64),
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
            to_number: std::mem::transmute(get(b"?lua_tonumber@@YANPEAUlua_State@@H@Z\0")?),
            to_lstring: std::mem::transmute(get(b"?lua_tolstring@@YAPEBDPEAUlua_State@@HPEA_K@Z\0")?),
            push_number: std::mem::transmute(get(b"?lua_pushnumber@@YAXPEAUlua_State@@N@Z\0")?),
        })
    })
    .as_ref()
}

const LUA_GLOBALSINDEX: i32 = -10002;

/// `Civ6RolloverProxy_DepartDispo()` → 1 si cette machine peut retenir les ordres (détour posé)
/// et que les correctifs POL sont actifs (tous les humains ont BetterGameDLLs à la même version),
/// sinon 0 ; puis 1 si le minuteur est gelé avec, sinon 0.
unsafe extern "C-unwind" fn lua_dispo(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let dispo = ENVOI_TRAMP.load(Ordering::Relaxed) != 0 && pol::actifs();
    (h.push_number)(l, dispo as u8 as f64);
    (h.push_number)(l, (MINUTEUR_TRAMP.load(Ordering::Relaxed) != 0) as u8 as f64);
    2
}

/// `Civ6RolloverProxy_DepartAttendre(joueur local, PlayerOperations.EXECUTE_SCRIPT)`.
unsafe extern "C-unwind" fn lua_attendre(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let joueur = (h.to_number)(l, 1) as i32;
    // Les hachages du jeu dépassent 2^31 : lus en nombre, rendus en u32.
    let exec = (h.to_number)(l, 2) as i64 as u32;
    attendre(joueur, exec);
    0
}

/// `Civ6RolloverProxy_DepartPartir(raison)` → 1 si l'on attendait.
unsafe extern "C-unwind" fn lua_partir(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let p = (h.to_lstring)(l, 1, std::ptr::null_mut());
    let raison = if p.is_null() { "départ".into() } else { CStr::from_ptr(p).to_string_lossy().into_owned() };
    (h.push_number)(l, partir(&raison) as u8 as f64);
    1
}

/// `Civ6RolloverProxy_DepartEtat()` → en attente (1|0), ordres retenus, secondes d'attente.
unsafe extern "C-unwind" fn lua_etat(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let on = ATTENTE.load(Ordering::Relaxed);
    (h.push_number)(l, on as u8 as f64);
    (h.push_number)(l, REJETS.load(Ordering::Relaxed) as f64);
    (h.push_number)(l, if on { attente_s() } else { 0.0 });
    3
}

/// `Civ6RolloverProxy_DepartSignaux()` → 1 si les signaux `.bgd_` restent visibles dans le chat.
unsafe extern "C-unwind" fn lua_signaux(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    (h.push_number)(l, SIGNAUX.load(Ordering::Relaxed) as u8 as f64);
    1
}

/// `Civ6RolloverProxy_FinForcer()` → 1 si la fin du tour local est forcée, 0 si le minuteur n'a
/// pas encore été vu.
unsafe extern "C-unwind" fn lua_fin_forcer(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    (h.push_number)(l, fin_forcer() as u8 as f64);
    1
}

/// `Civ6RolloverProxy_FinLever()` : au début du tour local, minuteur rendu à son état sans la fin
/// forcée.
unsafe extern "C-unwind" fn lua_fin_lever(_l: *mut c_void) -> i32 {
    fin_lever();
    0
}

/// `Civ6RolloverProxy_DepartJournal(texte)` : une ligne dans le journal du proxy.
unsafe extern "C-unwind" fn lua_journal(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let p = (h.to_lstring)(l, 1, std::ptr::null_mut());
    if !p.is_null() {
        log(&format!("depart : {}", CStr::from_ptr(p).to_string_lossy()));
    }
    0
}

const NATIVES: [(&[u8], LuaCFn); 8] = [
    (b"Civ6RolloverProxy_DepartDispo\0", lua_dispo),
    (b"Civ6RolloverProxy_DepartAttendre\0", lua_attendre),
    (b"Civ6RolloverProxy_DepartPartir\0", lua_partir),
    (b"Civ6RolloverProxy_DepartEtat\0", lua_etat),
    (b"Civ6RolloverProxy_DepartSignaux\0", lua_signaux),
    (b"Civ6RolloverProxy_DepartJournal\0", lua_journal),
    (b"Civ6RolloverProxy_FinForcer\0", lua_fin_forcer),
    (b"Civ6RolloverProxy_FinLever\0", lua_fin_lever),
];

unsafe extern "C-unwind" fn installer(l: *mut c_void) -> i32 {
    if let Some(h) = hks() {
        for (name, f) in NATIVES {
            (h.push_closure)(l, f, 0, name.as_ptr() as *const c_char, 0);
            (h.set_field)(l, LUA_GLOBALSINDEX, name.as_ptr() as *const c_char);
        }
    }
    0
}

unsafe fn publier(l: *mut c_void) -> bool {
    let Some(h) = hks() else { return false };
    let r = (h.cpcall)(l, installer, std::ptr::null_mut());
    if r != 0 {
        (h.set_top)(l, -2);
    }
    r == 0
}

// ---------------------------------------------------------------------------------------------
// Scripts.

const BLOC_PARTIE: &str = include_str!("ui_depart.lua");
const BLOC_CHAT: &str = include_str!("ui_depart_chat.lua");

/// Le script suivi de son bloc, si c'est `InGame.lua` (ou son remplaçant du même nom, celui de
/// MPH) ou `ChatPanel.lua` (que MPH inclut). Le bloc d'InGame coupe aussi la synchro de tour de
/// MPH (`CPL_SYNCTURN`), qui ne synchronise rien : il l'est donc toujours, détours posés ou non.
pub fn patch(chunk: &str, src: &[u8]) -> Option<(Vec<u8>, &'static str)> {
    let (bloc, what) = if is_file(chunk, "InGame.lua") {
        (BLOC_PARTIE, "départ simultané et synchro de tour de MPH coupée")
    } else if is_file(chunk, "ChatPanel.lua") {
        (BLOC_CHAT, "signaux du départ simultané cachés")
    } else {
        return None;
    };
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(bloc.as_bytes());
    Some((out, what))
}

/// L'accroche Lua de `ui` est toujours nécessaire : le bloc d'InGame coupe la synchro de tour de
/// MPH même sans départ simultané.
pub fn interface_requise() -> bool {
    true
}

/// `patch`, après avoir posé les fonctions natives dans le contexte qui compile le script.
pub unsafe fn patch_lua(l: *mut c_void, chunk: &str, src: &[u8]) -> Option<(Vec<u8>, &'static str)> {
    let r = patch(chunk, src)?;
    if !publier(l) {
        log(&format!("depart : {chunk} : fonctions natives non posées, départ simultané indisponible"));
    }
    Some(r)
}

// ---------------------------------------------------------------------------------------------
// Pose.

pub unsafe fn on_gamecore_loaded(base: usize) {
    match patch::check(base, &[ENVOI]).and_then(|()| patch::install(base, &ENVOI, hook_envoi as *const () as usize)) {
        Ok(t) => {
            GC_BASE.store(base, Ordering::Relaxed);
            ENVOI_TRAMP.store(t, Ordering::Relaxed);
        }
        Err(e) => {
            log(&format!("depart : envoi des ordres non détourné, départ simultané indisponible : {e}"));
            return;
        }
    }
    let exe = win::GetModuleHandleW(std::ptr::null()) as usize;
    let minuteur = (|| {
        let (start, size) = text_section(exe).ok_or("section .text de l'exe introuvable")?;
        let text = std::slice::from_raw_parts(start as *const u8, size);
        let at = match &find_sig(text, MINUTEUR_SIG)[..] {
            [at] => start + at,
            v => return Err(format!("pause du minuteur trouvée {} fois au lieu d'une", v.len())),
        };
        let prologue = std::slice::from_raw_parts(at as *const u8, MINUTEUR_PROLOGUE).to_vec();
        let t = patch::install_at("pause du minuteur", at, &prologue, at + MINUTEUR_PROLOGUE, hook_minuteur as *const () as usize)?;
        MINUTEUR_SITE.store(at, Ordering::Relaxed);
        MINUTEUR_TRAMP.store(t, Ordering::Relaxed);
        Ok::<usize, String>(at - exe)
    })();
    match minuteur {
        Ok(rva) => log(&format!("depart : détours posés (envoi des ordres, minuteur en exe+{rva:#x})")),
        Err(e) => log(&format!("depart : envoi des ordres détourné, minuteur laissé tel quel : {e}")),
    }
}

/// Nouvelle partie : aucune attente ne survit à la précédente.
pub fn reset() {
    ATTENTE.store(false, Ordering::Relaxed);
    FORCE.store(false, Ordering::Relaxed);
    MINUTEUR_THIS.store(0, Ordering::Relaxed);
    REJETS.store(0, Ordering::Relaxed);
    LOGS.store(0, Ordering::Relaxed);
}

/// Le proxy est déchargé alors que l'exe et l'originale restent en mémoire : on remet leurs octets.
pub unsafe fn on_proxy_unload() {
    ATTENTE.store(false, Ordering::Relaxed);
    fin_lever();
    let at = MINUTEUR_SITE.swap(0, Ordering::Relaxed);
    let tramp = MINUTEUR_TRAMP.load(Ordering::Relaxed);
    if at != 0 && tramp != 0 {
        // Le trampoline commence par le prologue d'origine.
        patch::restore_at(at, std::slice::from_raw_parts(tramp as *const u8, MINUTEUR_PROLOGUE));
    }
    let base = GC_BASE.swap(0, Ordering::Relaxed);
    if base != 0 && ENVOI_TRAMP.load(Ordering::Relaxed) != 0 {
        patch::restore(base, &ENVOI);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXES: [&str; 2] = [
        r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam\CivilizationVI_DX12.exe",
        r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam\CivilizationVI.exe",
    ];

    #[test]
    fn signature_unique_dans_les_deux_exe() {
        for path in EXES {
            let Ok(data) = std::fs::read(path) else {
                eprintln!("absent, test sauté : {path}");
                continue;
            };
            assert_eq!(find_sig(&data, MINUTEUR_SIG).len(), 1, "{path}");
        }
    }

    #[test]
    fn signature_avec_jokers() {
        assert_eq!(MINUTEUR_SIG[0], Some(0x48));
        assert_eq!(MINUTEUR_SIG[19], None);
        let pat = sig(&["aa", "?", "cc"]);
        assert_eq!(find_sig(&[0, 0xaa, 7, 0xcc, 0xaa, 8, 0xcd], &pat), vec![1]);
    }

    #[test]
    fn blocs_ajoutes() {
        let (out, _) = patch(r"C:\x\Base\Assets\UI\InGame.lua", b"Initialize();").unwrap();
        assert!(out.starts_with(b"Initialize();\r\n-- "));
        assert!(patch("ui/ingame.lua", b"x").is_some(), "le remplaçant de MPH");
        assert!(patch(r"C:\x\Popups\ChatPanel.lua", b"x").is_some());
        assert!(patch(r"C:\x\chatpanel_MPH.lua", b"x").is_none(), "MPH inclut ChatPanel.lua, qui reçoit le bloc");
        assert!(patch(r"C:\x\InGameTopOptionsMenu.lua", b"x").is_none());
        for (name, _) in NATIVES {
            let name = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
            assert!(BLOC_PARTIE.contains(name) || BLOC_CHAT.contains(name), "{name} posée mais pas utilisée");
        }
    }
}
