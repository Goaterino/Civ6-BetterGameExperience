//! Production en réserve d'une ville (débordement), montrée par l'interface (BET-21).
//!
//! GameCore garde dans la file de production (`ville + 0x10d0`) la production qui reste à verser
//! dans le prochain élément : `file + 0x1f4`, en virgule fixe 24.8 (`m_xProductionOverflow`, nom
//! tiré de la sérialisation `180106510`). Elle vient de deux chemins de `1800f5080` (ajout de la
//! production du tour) :
//! - un élément terminé : min(production du tour, production de base) − ce qui manquait à
//!   l'élément, au moins 0 ;
//! - une file vide : la production du tour s'y ajoute, puis `180266120` remet à zéro les villes
//!   à file vide sur un événement de tour, d'où un plafond d'un tour.
//! Au tour suivant, `1800f5080` ajoute la réserve à la production du tour pour l'élément en tête,
//! puis la remet à zéro (ou au nouveau débordement). Relevé en jeu le 2026-11-10 : des villes à 2 à
//! 4 éléments en file gardaient 13, 33 et 77 points ; la réserve n'est donc pas versée au choix d'un
//! élément.
//!
//! La valeur n'est pas dans le cache de l'interface (`Cache_City_BuildQueue`). On la relève quand
//! le cache est reconstruit : dans `18006b860`, le `call 1800fd160` en `18006b8d1` (rcx = la file)
//! est redirigé vers un relais qui note la réserve de la ville puis appelle l'original. Lecture
//! seule, dans le fil qui construit le cache : rien ne change dans la partie. `tours` redirige un
//! autre appel de la même fonction (`18006b8dc`), sans conflit.
//!
//! Le Lua lit le relevé par `BGD_Debordement(joueur, ville)` → points de production, ou rien si
//! la ville n'a pas encore été vue (`ui_debordement.lua`, posé par `ui`).
//!
//! Réglage `ui_debordement=on|off` dans rollover_proxy.cfg (on par défaut).
//!
//! Diagnostic `diag_production=on|off` (off par défaut ; rapports BGE-E6DF54 et BGE-36E0D7) :
//! - à chaque relevé d'une ville du joueur local (celui qui appelle `BGD_Debordement`), quand
//!   l'objet ou la valeur change : adresse de la file, adresse de la ville lue en `file + 0x1e8`,
//!   identifiant, valeur brute de `file + 0x1f4`, taille de la file, et si `ville + 0x10d0` est
//!   bien la file (lignes `debordement diag : relevé …`) ;
//! - pour tout joueur, une file dont la ville ne la désigne pas (`… INCOHÉRENT …`, 50 au plus) ;
//! - à chaque lecture par le Lua, quand la valeur ou le tour changent : joueur, ville, tour (passé
//!   par le Lua), valeur rendue et relevé dont elle vient (`debordement diag : lu …`).
//!
//! Le même réglage pose `BGD_DIAG_PRODUCTION` pour le badge des quartiers (`ui_quartiers.lua`,
//! lignes `BGD diag quartiers` dans Lua.log).

use crate::log;
use crate::patch::{alloc_near, rel32, write_rel32};
use crate::win;
use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

static ENABLED: AtomicBool = AtomicBool::new(true);
/// Diagnostic `diag_production`.
static DIAG: AtomicBool = AtomicBool::new(false);
/// Joueur dont le Lua lit la réserve (le joueur local), -1 avant la première lecture.
static DIAG_JOUEUR: AtomicI32 = AtomicI32::new(-1);
/// Dernier tour passé par le Lua.
static DIAG_TOUR: AtomicI32 = AtomicI32::new(-1);
static DIAG_INCOHERENTS: AtomicUsize = AtomicUsize::new(0);
/// Dernier relevé par (joueur, ville) : file, ville, valeur brute, taille de la file.
static DIAG_RELEVES: Mutex<Option<HashMap<(i32, i32), (usize, usize, i32, i32)>>> = Mutex::new(None);
/// Dernière lecture journalisée par (joueur, ville) : valeur rendue (24.8, ou -1), tour.
static DIAG_LUS: Mutex<Option<HashMap<(i32, i32), (i32, i32)>>> = Mutex::new(None);
static BASE: AtomicUsize = AtomicUsize::new(0);
static PATCHED_BASE: AtomicUsize = AtomicUsize::new(0);

/// Réserve relevée par (joueur, ville), en virgule fixe 24.8.
static RESERVES: Mutex<Option<HashMap<(i32, i32), i32>>> = Mutex::new(None);
/// File de production de chaque ville relevée, pour relire la réserve en direct : le cache n'est
/// reconstruit que quand la file de la ville change, pas quand le tour verse la réserve
/// (BGE-E6DF54 : « +8 » resté sur la capitale au tour suivant, jusqu'à un changement de la file).
static FILES: Mutex<Option<HashMap<(i32, i32), usize>>> = Mutex::new(None);
static TRACES: AtomicUsize = AtomicUsize::new(0);
const MAX_TRACES: usize = 20;

/// Site du `call` redirigé, et la fonction qu'il appelle.
const SITE: u64 = 0x1_8006_b8d1;
const CALLEE: u64 = 0x1_800f_d160;

// Champs de GameCore.
const FILE_VILLE: usize = 0x1e8; // file -> ville
const FILE_TAILLE: usize = 0x1e0; // nombre d'éléments
const FILE_RESERVE: usize = 0x1f4; // m_xProductionOverflow, 24.8
const VILLE_ID: usize = 0xa8; // identifiant de la ville (celui de pCity:GetID())
const VILLE_JOUEUR: usize = 0xd8; // joueur propriétaire
const VILLE_FILE: usize = 0x10d0; // ville -> sa file (vérifié par le diagnostic)

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// Réglage `diag_production`.
pub fn set_diag(on: bool) {
    DIAG.store(on, Ordering::Relaxed);
}

pub fn diag() -> bool {
    DIAG.load(Ordering::Relaxed)
}

/// Diagnostic d'un relevé (fil du cache).
fn diag_releve(queue: usize, city: usize, joueur: i32, id: i32, reserve: i32, taille: i32) {
    let coherent = city.wrapping_add(VILLE_FILE) == queue;
    if !coherent && DIAG_INCOHERENTS.fetch_add(1, Ordering::Relaxed) < 50 {
        log(&format!(
            "debordement diag : INCOHÉRENT file {queue:#x}, ville {city:#x} (ville + 0x10d0 = {:#x}), joueur {joueur}, id {id}, +0x1f4 = {reserve}",
            city.wrapping_add(VILLE_FILE)
        ));
    }
    if joueur != DIAG_JOUEUR.load(Ordering::Relaxed) {
        return;
    }
    let nouveau = (queue, city, reserve, taille);
    let avant = {
        let mut g = DIAG_RELEVES.lock().unwrap_or_else(|e| e.into_inner());
        g.get_or_insert_with(HashMap::new).insert((joueur, id), nouveau)
    };
    if avant != Some(nouveau) {
        log(&format!(
            "debordement diag : relevé joueur {joueur}, ville {id} : file {queue:#x}, ville {city:#x}, ville + 0x10d0 = file : {}, +0x1f4 = {reserve} ({:.2}), file de {taille} éléments, dernier tour vu par le Lua {}",
            if coherent { "oui" } else { "NON" },
            reserve as f64 / 256.0,
            DIAG_TOUR.load(Ordering::Relaxed)
        ));
    }
}

/// Diagnostic d'une lecture par le Lua.
fn diag_lecture(joueur: i32, ville: i32, tour: i32) {
    DIAG_JOUEUR.store(joueur, Ordering::Relaxed);
    DIAG_TOUR.store(tour, Ordering::Relaxed);
    let valeur = {
        let g = RESERVES.lock().unwrap_or_else(|e| e.into_inner());
        g.as_ref().and_then(|m| m.get(&(joueur, ville)).copied())
    };
    let cle = (valeur.unwrap_or(-1), tour);
    let avant = {
        let mut g = DIAG_LUS.lock().unwrap_or_else(|e| e.into_inner());
        g.get_or_insert_with(HashMap::new).insert((joueur, ville), cle)
    };
    if avant == Some(cle) {
        return;
    }
    let releve = {
        let g = DIAG_RELEVES.lock().unwrap_or_else(|e| e.into_inner());
        g.as_ref().and_then(|m| m.get(&(joueur, ville)).copied())
    };
    let releve = match releve {
        Some((q, c, r, t)) => format!("file {q:#x}, ville {c:#x}, +0x1f4 = {r}, file de {t} éléments"),
        None => "aucun relevé diag pour cette clé".to_string(),
    };
    log(&format!(
        "debordement diag : lu joueur {joueur}, ville {ville}, tour {tour} : cache {} ({releve})",
        match valeur {
            Some(v) => format!("{:.2}", v as f64 / 256.0),
            None => "absent".to_string(),
        }
    ));
}

/// Le relevé est posé : `BGD_Debordement` rend des valeurs.
pub fn pose() -> bool {
    PATCHED_BASE.load(Ordering::Relaxed) != 0
}

fn gc(addr: u64) -> usize {
    BASE.load(Ordering::Relaxed) + crate::patch::rva(addr)
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read(addr as *const T)
}

/// Relais : note la réserve de la ville de la file, puis appelle `1800fd160` (rcx = file).
unsafe extern "system" fn relais(queue: usize) -> usize {
    if queue != 0 {
        let city = rd::<usize>(queue + FILE_VILLE);
        if city != 0 {
            let (joueur, id, reserve) = (rd::<i32>(city + VILLE_JOUEUR), rd::<i32>(city + VILLE_ID), rd::<i32>(queue + FILE_RESERVE));
            if DIAG.load(Ordering::Relaxed) {
                diag_releve(queue, city, joueur, id, reserve, rd::<i32>(queue + FILE_TAILLE));
            }
            if (0..64).contains(&joueur) {
                let avant = {
                    let mut g = RESERVES.lock().unwrap_or_else(|e| e.into_inner());
                    g.get_or_insert_with(HashMap::new).insert((joueur, id), reserve)
                };
                FILES.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert((joueur, id), queue);
                if avant != Some(reserve) && TRACES.fetch_add(1, Ordering::Relaxed) < MAX_TRACES {
                    log(&format!(
                        "debordement : joueur {joueur}, ville {id} : {:.2} (file de {} éléments)",
                        reserve as f64 / 256.0,
                        rd::<i32>(queue + FILE_TAILLE)
                    ));
                }
            }
        }
    }
    let f: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(gc(CALLEE));
    f(queue)
}

/// Réserve relevée, en points de production.
fn reserve(joueur: i32, ville: i32) -> Option<f64> {
    if let Some(v) = unsafe { reserve_directe(joueur, ville) } {
        return Some(v as f64 / 256.0);
    }
    let g = RESERVES.lock().unwrap_or_else(|e| e.into_inner());
    g.as_ref()?.get(&(joueur, ville)).map(|&v| v as f64 / 256.0)
}

#[repr(C)]
struct MemoryBasicInformation {
    base_address: usize,
    allocation_base: usize,
    allocation_protect: u32,
    partition_id: u16,
    region_size: usize,
    state: u32,
    protect: u32,
    kind: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn VirtualQuery(address: *const c_void, buffer: *mut MemoryBasicInformation, length: usize) -> usize;
}

/// `taille` octets lisibles en `at` (mémoire engagée, ni gardée ni interdite).
unsafe fn lisible(at: usize, taille: usize) -> bool {
    let mut fin_ok = at;
    while fin_ok < at + taille {
        let mut mbi: MemoryBasicInformation = std::mem::zeroed();
        if VirtualQuery(fin_ok as *const c_void, &mut mbi, std::mem::size_of::<MemoryBasicInformation>()) == 0 {
            return false;
        }
        const LECTURE: u32 = 0x02 | 0x04 | 0x08 | 0x20 | 0x40 | 0x80; // READONLY … EXECUTE_WRITECOPY
        if mbi.state != win::MEM_COMMIT || mbi.protect & LECTURE == 0 || mbi.protect & 0x100 != 0 {
            return false;
        }
        fin_ok = mbi.base_address + mbi.region_size;
    }
    true
}

/// Réserve relue dans la file de la ville, si la file relevée désigne toujours cette ville.
unsafe fn reserve_directe(joueur: i32, ville: i32) -> Option<i32> {
    let queue = *FILES.lock().unwrap_or_else(|e| e.into_inner()).as_ref()?.get(&(joueur, ville))?;
    if !lisible(queue + FILE_VILLE, FILE_RESERVE + 4 - FILE_VILLE) {
        return None;
    }
    let city = rd::<usize>(queue + FILE_VILLE);
    if city == 0 || !lisible(city + VILLE_ID, VILLE_JOUEUR + 4 - VILLE_ID) {
        return None;
    }
    if rd::<i32>(city + VILLE_JOUEUR) != joueur || rd::<i32>(city + VILLE_ID) != ville {
        return None;
    }
    Some(rd::<i32>(queue + FILE_RESERVE))
}

// ---------------------------------------------------------------------------------------------
// Coût des quartiers, lu en direct (badge des quartiers en réduction, ui_quartiers.lua).
//
// Le cache de la file d'une ville (`18006b860`) recopie le coût de chaque type de quartier
// (`1800fd620`) et n'est reconstruit que quand la file de cette ville change. Poser un campus
// recalcule la table du joueur (`1801029c0` → `1802c4f10`), mais une autre ville déjà vue garde
// l'ancien coût : BGE-36E0D7, campus à 42 (réduit) affiché à Akhetaton alors que la table disait 65.
// `1800fd620` : joueur (`180044f00`) + 0x6e8 → gestionnaire de quartiers, + 0x120 → coût par type
// (`1802c43e0`). Lecture seule.

const JOUEUR: u64 = 0x1_8004_4f00; // (id joueur) -> joueur
const COUT_TYPE: u64 = 0x1_802c_43e0; // (gestionnaire, type) -> coût enregistré
const COUT_VILLE: u64 = 0x1_800f_d620; // remplit le cache ; lit joueur + 0x6e8
const JOUEUR_QUARTIERS: usize = 0x6e8;
const QUARTIERS_COUTS: usize = 0x120;
/// mov rax,[rcx+120h] / movsxd rdx,edx / mov eax,[rax+rdx*4] / ret
const COUT_TYPE_OCTETS: [u8; 14] = [0x48, 0x8b, 0x81, 0x20, 0x01, 0x00, 0x00, 0x48, 0x63, 0xd2, 0x8b, 0x04, 0x90, 0xc3];
/// mov edx,ecx / mov rcx,[rip+…] (le reste est relatif)
const JOUEUR_OCTETS: [u8; 5] = [0x8b, 0xd1, 0x48, 0x8b, 0x0d];

static COUT_BASE: AtomicUsize = AtomicUsize::new(0);

/// Vérifie les fonctions lues ; rend vrai si `BGD_CoutQuartier` peut être posée.
unsafe fn verifier_cout(base: usize) -> bool {
    let at = |a: u64| base + crate::patch::rva(a);
    let octets = |a: usize, n: usize| std::slice::from_raw_parts(a as *const u8, n).to_vec();
    if octets(at(COUT_TYPE), COUT_TYPE_OCTETS.len()) != COUT_TYPE_OCTETS || octets(at(JOUEUR), JOUEUR_OCTETS.len()) != JOUEUR_OCTETS {
        return false;
    }
    // `mov …,[…+6E8h]` dans 1800fd620 : le gestionnaire de quartiers est bien en + 0x6e8.
    octets(at(COUT_VILLE), 0xe0).windows(4).any(|w| w == [0xe8, 0x06, 0x00, 0x00])
}

/// Coût enregistré du type de quartier `index` pour `joueur`, ou None.
unsafe fn cout_quartier(joueur: i32, index: i32) -> Option<i32> {
    let base = COUT_BASE.load(Ordering::Relaxed);
    if base == 0 || !(0..64).contains(&joueur) || !(0..256).contains(&index) {
        return None;
    }
    let f: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + crate::patch::rva(JOUEUR));
    let player = f(joueur as u32);
    if player == 0 {
        return None;
    }
    let gestionnaire = rd::<usize>(player + JOUEUR_QUARTIERS);
    if gestionnaire == 0 {
        return None;
    }
    let table = rd::<usize>(gestionnaire + QUARTIERS_COUTS);
    if table == 0 || !lisible(table + index as usize * 4, 4) {
        return None;
    }
    Some(rd::<i32>(table + index as usize * 4))
}

// ---------------------------------------------------------------------------------------------
// Lua.

pub(crate) type LuaCFn = unsafe extern "C-unwind" fn(*mut c_void) -> i32;

pub(crate) struct Hks {
    pub(crate) push_closure: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, i32, *const c_char, i32),
    pub(crate) set_field: unsafe extern "C-unwind" fn(*mut c_void, i32, *const c_char),
    pub(crate) cpcall: unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, *mut c_void) -> i32,
    pub(crate) set_top: unsafe extern "C-unwind" fn(*mut c_void, i32),
    pub(crate) to_integer: unsafe extern "C-unwind" fn(*mut c_void, i32) -> i32,
    pub(crate) push_number: unsafe extern "C-unwind" fn(*mut c_void, f64),
}

static HKS: OnceLock<Option<Hks>> = OnceLock::new();

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
            push_closure: std::mem::transmute(get(b"?hks_pushnamedcclosure@@YAXPEAUlua_State@@P6AH0@ZHPEBDH@Z\0")?),
            set_field: std::mem::transmute(get(b"?lua_setfield@@YAXPEAUlua_State@@HPEBD@Z\0")?),
            cpcall: std::mem::transmute(get(b"?lua_cpcall@@YAHPEAUlua_State@@P6AH0@ZPEAX@Z\0")?),
            set_top: std::mem::transmute(get(b"?lua_settop@@YAXPEAUlua_State@@H@Z\0")?),
            to_integer: std::mem::transmute(get(b"?lua_tointeger@@YAHPEAUlua_State@@H@Z\0")?),
            push_number: std::mem::transmute(get(b"?lua_pushnumber@@YAXPEAUlua_State@@N@Z\0")?),
        })
    })
    .as_ref()
}

pub(crate) const LUA_GLOBALSINDEX: i32 = -10002;
const NOM: &[u8] = b"BGD_Debordement\0";

unsafe extern "C-unwind" fn lua_debordement(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let (joueur, ville) = ((h.to_integer)(l, 1), (h.to_integer)(l, 2));
    if DIAG.load(Ordering::Relaxed) {
        // Troisième argument : le tour (Game.GetCurrentGameTurn), 0 s'il manque.
        diag_lecture(joueur, ville, (h.to_integer)(l, 3));
    }
    match reserve(joueur, ville) {
        Some(v) => {
            (h.push_number)(l, v);
            1
        }
        None => 0,
    }
}

const NOM_COUT: &[u8] = b"BGD_CoutQuartier\0";

/// `BGD_CoutQuartier(joueur, index du type)` → coût enregistré du type, ou rien.
unsafe extern "C-unwind" fn lua_cout_quartier(l: *mut c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    match cout_quartier((h.to_integer)(l, 1), (h.to_integer)(l, 2)) {
        Some(c) => {
            (h.push_number)(l, c as f64);
            1
        }
        None => 0,
    }
}

unsafe extern "C-unwind" fn installer_cout(l: *mut c_void) -> i32 {
    if let Some(h) = hks() {
        (h.push_closure)(l, lua_cout_quartier, 0, NOM_COUT.as_ptr() as *const c_char, 0);
        (h.set_field)(l, LUA_GLOBALSINDEX, NOM_COUT.as_ptr() as *const c_char);
    }
    0
}

/// Pose `BGD_CoutQuartier` dans l'état qui compile ProductionPanel, si les fonctions lues sont
/// celles attendues. Sans elle, le badge garde le coût du cache de la ville.
pub unsafe fn publier_cout(l: *mut c_void) -> bool {
    if COUT_BASE.load(Ordering::Relaxed) == 0 {
        return false;
    }
    let Some(h) = hks() else { return false };
    let r = (h.cpcall)(l, installer_cout, std::ptr::null_mut());
    if r != 0 {
        (h.set_top)(l, -2);
    }
    r == 0
}

unsafe extern "C-unwind" fn installer(l: *mut c_void) -> i32 {
    if let Some(h) = hks() {
        (h.push_closure)(l, lua_debordement, 0, NOM.as_ptr() as *const c_char, 0);
        (h.set_field)(l, LUA_GLOBALSINDEX, NOM.as_ptr() as *const c_char);
    }
    0
}

static PUBLIE_LOG: AtomicBool = AtomicBool::new(false);

/// Pose `BGD_Debordement` dans la table globale de l'état qui compile le script. Rend vrai si
/// elle y est : sans elle, le bloc Lua ne fait rien.
pub unsafe fn publier(l: *mut c_void) -> bool {
    let r = match hks() {
        Some(h) => {
            let r = (h.cpcall)(l, installer, std::ptr::null_mut());
            if r != 0 {
                (h.set_top)(l, -2);
            }
            r
        }
        None => -1,
    };
    if r != 0 || !PUBLIE_LOG.swap(true, Ordering::Relaxed) {
        log(&if r == 0 { "debordement : BGD_Debordement posée".to_string() } else { format!("debordement : BGD_Debordement non posée ({r})") });
    }
    r == 0
}

// ---------------------------------------------------------------------------------------------
// Pose.

/// Appelé quand l'originale vient d'être chargée.
pub unsafe fn on_gamecore_loaded(base: usize) {
    if verifier_cout(base) {
        COUT_BASE.store(base, Ordering::Relaxed);
    } else {
        COUT_BASE.store(0, Ordering::Relaxed);
        log("debordement : version inattendue, coût des quartiers lu dans le cache de la ville");
    }
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    let at = gc(SITE);
    if rd::<u8>(at) != 0xe8 || rel32(at, gc(CALLEE)) != Some(rd::<i32>(at + 1)) {
        log(&format!("debordement : version inattendue en {SITE:#x}, réserve de production non relevée"));
        return;
    }
    let Some(page) = alloc_near(base) else {
        log("debordement : pas de mémoire à moins de 2 Go de GameCore, réserve non relevée");
        return;
    };
    // Relais : jmp [rip+0] puis l'adresse de `relais`.
    std::ptr::copy_nonoverlapping([0xffu8, 0x25, 0, 0, 0, 0].as_ptr(), page as *mut u8, 6);
    std::ptr::write_unaligned((page + 6) as *mut u64, relais as *const () as usize as u64);
    match rel32(at, page) {
        Some(rel) => write_rel32(at, rel),
        None => {
            log("debordement : relais hors de portée, réserve non relevée");
            return;
        }
    }
    PATCHED_BASE.store(base, Ordering::Relaxed);
    log("debordement : réserve de production relevée à chaque mise à jour du cache de la file (18006b8d1)");
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet l'appel d'origine.
pub unsafe fn on_proxy_unload() {
    let base = PATCHED_BASE.swap(0, Ordering::Relaxed);
    if base == 0 {
        return;
    }
    let at = base + crate::patch::rva(SITE);
    if let Some(rel) = rel32(at, base + crate::patch::rva(CALLEE)) {
        if rd::<u8>(at) == 0xe8 && rd::<i32>(at + 1) != rel {
            write_rel32(at, rel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releve_en_points() {
        RESERVES.lock().unwrap().get_or_insert_with(HashMap::new).insert((0, 65536), 13 * 256 + 128);
        assert_eq!(reserve(0, 65536), Some(13.5));
        assert_eq!(reserve(0, 1), None);
    }
}
