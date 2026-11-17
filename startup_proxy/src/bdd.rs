//! Base de gameplay gardée d'un chargement à l'autre quand elle n'a pas à changer
//! (`bdd_gardee=on`, par défaut). GameCore est toujours déchargé et rechargé (correctif de yodha,
//! `fresh_gamecore`) : seule la reconstruction de la base est sautée.
//!
//! Ce que fait le jeu (`analyses/chargement/README.md`, « Reconstruction à la demande » ; exe DX12
//! du 2026-11-05, projet `civ6exe1105`). À chaque chargement, `UpdateContent` (`14024d340`) :
//! 1. décharge et recharge la DLL de GameCore ;
//! 2. remet la localisation à `Origin`, puis la base de gameplay à zéro (`1400c0280` : fermeture
//!    de la base en mémoire et reconstruction depuis `Base/Assets/Gameplay/Data`, sauf si son
//!    compteur de modifications n'a pas bougé depuis sa création) ;
//! 3. y écrit les données embarquées de GameCore (`14024e340` → GameCore vtable +0x50, des
//!    `INSERT` de types d'effets décrits par des registres statiques, sans autre effet) ;
//! 4. applique tous les composants des mods (`140b3d8f0`) : UpdateDatabase seul écrit dans la base
//!    de gameplay (1,4 s), UpdateText dans la localisation, les autres inscrivent des fichiers
//!    (couleurs, icônes, scripts, interface, art) ;
//! 5. initialise GameCore, qui relit toute la base, puis copie la base dans
//!    `Cache/DebugGameplay.sqlite` (`1400bfc10`).
//!
//! Le salon (configuration) remet la localisation à `Origin` à chaque passage, ce qui fait
//! toujours prendre ce chemin complet ; mais rien ne touche à la base de gameplay entre deux
//! chargements. GameCore n'y fait que des `SELECT`.
//!
//! Le détour, sur les seuls appels faits depuis `UpdateContent` :
//! - à la fin d'un `UpdateContent` réussi, on relève la connexion de la base de gameplay, son
//!   compteur de modifications (vtable +0x30, celui dont le jeu se sert lui-même) et la liste des
//!   actions en jeu appliquées (`+0x208`, identifiants d'actions des mods) ;
//! - au chargement suivant, si la connexion est la même, que son compteur n'a pas bougé et que la
//!   liste d'actions cible est identique, la remise à zéro (2), les données embarquées (3), le
//!   gestionnaire UpdateDatabase (4) et la copie de débogage (5) sont sautés. Tout le reste est
//!   fait comme d'habitude, dont le rechargement de GameCore et UpdateText.
//!
//! Le contenu de la base est donc celui que la même liste d'actions vient de produire : pas de
//! désynchro possible, même face à un joueur sans BGE. En cas d'échec d'`UpdateContent`, le jeu
//! fait lui-même son retour complet à vanilla (`14024cea0`), qui n'est pas détourné, et l'état
//! relevé est oublié.

use crate::{exe, localisation, log, motifs, patch};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

/// Prologue d'`UpdateContent` : `mov [rsp+18h],r8 ; mov [rsp+10h],rdx ; mov [rsp+8],rcx ;
/// push rbp ; push rbx ; push rsi ; push rdi ; push r12 ; push r14 ; push r15 ; mov rbp,rsp ;
/// sub rsp,80h ; lea r15,[rcx+30h]`. Les 15 premiers octets (les trois `mov`) sont recopiés.
const UPDATE_CONTENT: &str =
    "4C 89 44 24 18 48 89 54 24 10 48 89 4C 24 08 55 53 56 57 41 54 41 56 41 57 48 8B EC 48 81 EC 80 00 00 00 4C 8D 79 30";
const UPDATE_CONTENT_COPIE: usize = 15;
const UPDATE_CONTENT_TAILLE: usize = 0xa00;
/// Remise à zéro de la base de gameplay (`1400c0280`) : test du compteur contre `+0x1c`.
const REMISE: &str = "40 53 48 83 EC 20 48 8B D9 48 8B 49 08 48 85 C9 74 13 48 8B 01 FF 50 30 39 43 1C 75 08 32 C0";
/// Remise de la localisation à `Origin` (`1400c1f60`) : test du compteur contre `+0x10`.
const REMISE_LOC: &str = "40 53 48 83 EC 20 48 8B D9 48 8B 49 08 48 85 C9 0F 84 ?? ?? ?? ?? 48 8B 01 FF 50 30 3B 43 10 0F 84";
/// Données embarquées de GameCore (`14024e340`).
const EMBARQUEES: &str = "40 57 48 83 EC 20 48 8D 79 30 48 8B CF 48 8D 15 ?? ?? ?? ?? E8 ?? ?? ?? ?? 48 8D 4C 24 30 E8";
/// Application des composants (`140b3d8f0`).
const COMPOSANTS: &str =
    "48 89 54 24 10 55 56 41 55 41 56 41 57 48 8D AC 24 30 FC FF FF 48 81 EC D0 04 00 00 4C 8B F1 48 8B F2 48 8B 49 38";
/// Copie de débogage (`1400bfc10`), commune à trois fonctions sœurs : seul l'appel depuis
/// `UpdateContent` la désigne.
const COPIE_DEBOGAGE: &str = "48 8B C4 55 53 41 57 48 8D A8 88 FE FF FF 48 81 EC 60 02 00 00 48 83 79 08 00 4C 8B F9 0F 84";

/// Objet de `UpdateContent` : liste des actions en jeu appliquées (vecteur d'u32 : début, fin).
const ACTIONS: usize = 0x208;
/// Cadre de mods : liste des gestionnaires d'actions (`std::list`, nœud : suivant, précédent,
/// type u32 en +0x10, gestionnaire en +0x18).
const GESTIONNAIRES: usize = 0xc8;
/// Objet de `UpdateContent` : cadre de mods (premier argument de l'application des composants).
const CADRE: usize = 0x30;
/// Base de gameplay : connexion.
const CONNEXION: usize = 8;
/// Connexion : compteur de modifications.
const VT_COMPTEUR: usize = 0x30;

static ORIG_UPDATE: AtomicUsize = AtomicUsize::new(0);
static ORIG_REMISE: AtomicUsize = AtomicUsize::new(0);
static ORIG_EMBARQUEES: AtomicUsize = AtomicUsize::new(0);
static ORIG_COMPOSANTS: AtomicUsize = AtomicUsize::new(0);
static ORIG_COPIE: AtomicUsize = AtomicUsize::new(0);
static ORIG_REMISE_LOC: AtomicUsize = AtomicUsize::new(0);

/// Dans un `UpdateContent` en cours.
static DANS_UPDATE: AtomicBool = AtomicBool::new(false);
/// La base est gardée pour l'`UpdateContent` en cours.
static GARDEE: AtomicBool = AtomicBool::new(false);
/// Objet base de gameplay vu à la remise de l'`UpdateContent` en cours.
static BASE: AtomicUsize = AtomicUsize::new(0);
/// Objet base de localisation vu à sa remise dans l'`UpdateContent` en cours.
static LOC: AtomicUsize = AtomicUsize::new(0);
/// Les textes des mods ont été remis depuis le delta : UpdateText est neutralisé.
static LOC_GARDEE: AtomicBool = AtomicBool::new(false);
/// Rejeu des textes des mods depuis la mémoire (`textes_gardes`, `on` par défaut).
static LOC_ACTIF: AtomicBool = AtomicBool::new(false);

/// Capture en cours (chargement complet) : empreinte du schéma avant UpdateText.
static CAPTURE: Mutex<Option<localisation::Capture>> = Mutex::new(None);
/// Delta lu à la fin des composants de l'`UpdateContent` en cours, gardé s'il réussit.
static DELTA_LU: Mutex<Option<localisation::Delta>> = Mutex::new(None);

struct DeltaReleve {
    connexion: usize,
    actions: Vec<u32>,
    delta: localisation::Delta,
}

static DELTA: Mutex<Option<DeltaReleve>> = Mutex::new(None);

#[derive(Clone, PartialEq, Debug)]
struct Releve {
    connexion: usize,
    compteur: i32,
    /// `total_changes()` lu en SQL : ne dépend pas du sens exact du compteur de la vtable.
    modifications: Option<std::ffi::CString>,
    actions: Vec<u32>,
}

static RELEVE: Mutex<Option<Releve>> = Mutex::new(None);

unsafe fn compteur(connexion: usize) -> i32 {
    let vt = *(connexion as *const usize);
    let f: unsafe extern "C" fn(usize) -> i32 = std::mem::transmute(*((vt + VT_COMPTEUR) as *const usize));
    f(connexion)
}

unsafe fn modifications(connexion: usize) -> Option<std::ffi::CString> {
    localisation::Connexion(connexion).valeur("SELECT total_changes()")
}

unsafe fn actions(contenu: usize) -> Vec<u32> {
    let (debut, fin) = (*((contenu + ACTIONS) as *const *const u32), *((contenu + ACTIONS + 8) as *const *const u32));
    if debut.is_null() || fin < debut {
        return Vec::new();
    }
    std::slice::from_raw_parts(debut, fin.offset_from(debut) as usize).to_vec()
}

/// Résultat d'`UpdateContent` : octet de drapeaux (bit 0 : réussi), puis code d'erreur (0 : aucun).
unsafe fn reussi(resultat: *const u8) -> bool {
    !resultat.is_null() && *resultat & 1 != 0 && *(resultat.add(4) as *const i32) == 0
}

unsafe extern "C" fn update_content(contenu: usize, resultat: *mut u8, rappel: *mut c_void) -> *mut u8 {
    let orig: unsafe extern "C" fn(usize, *mut u8, *mut c_void) -> *mut u8 = std::mem::transmute(ORIG_UPDATE.load(Ordering::Relaxed));
    DANS_UPDATE.store(true, Ordering::Relaxed);
    GARDEE.store(false, Ordering::Relaxed);
    BASE.store(0, Ordering::Relaxed);
    LOC.store(0, Ordering::Relaxed);
    LOC_GARDEE.store(false, Ordering::Relaxed);
    let r = orig(contenu, resultat, rappel);
    DANS_UPDATE.store(false, Ordering::Relaxed);
    fin_localisation(contenu, reussi(resultat));
    let gardee = GARDEE.swap(false, Ordering::Relaxed);
    let base = BASE.swap(0, Ordering::Relaxed);
    let mut releve = RELEVE.lock().unwrap_or_else(|e| e.into_inner());
    if base == 0 {
        // Chemin court du jeu (configuration identique) ou échec avant la remise : la base n'a
        // pas été touchée par cet appel, le relevé reste valable s'il l'était.
        if !reussi(resultat) {
            *releve = None;
        }
        return r;
    }
    let connexion = *((base + CONNEXION) as *const usize);
    if reussi(resultat) && connexion != 0 {
        let n = Releve { connexion, compteur: compteur(connexion), modifications: modifications(connexion), actions: actions(contenu) };
        if gardee {
            log(&format!("database: gameplay database kept ({} actions)", n.actions.len()));
        } else {
            log(&format!("database: gameplay database built ({} actions), kept for the next load if unchanged", n.actions.len()));
        }
        *releve = Some(n);
    } else {
        if gardee {
            log("database: content update failed with the kept database, it will be rebuilt");
        }
        *releve = None;
    }
    r
}

/// Fin d'`UpdateContent` : garde le delta lu si tout a réussi, l'oublie en cas d'échec.
unsafe fn fin_localisation(contenu: usize, ok: bool) {
    let loc = LOC.swap(0, Ordering::Relaxed);
    let gardee = LOC_GARDEE.swap(false, Ordering::Relaxed);
    // Capture commencée mais jamais lue (composants non atteints) : rien à retirer.
    CAPTURE.lock().unwrap_or_else(|e| e.into_inner()).take();
    let lu = DELTA_LU.lock().unwrap_or_else(|e| e.into_inner()).take();
    let mut delta = DELTA.lock().unwrap_or_else(|e| e.into_inner());
    if !ok {
        *delta = None;
        return;
    }
    if loc == 0 {
        return;
    }
    if gardee {
        if let Some(d) = delta.as_ref() {
            log(&format!("localization: mod texts replayed from memory ({} rows, {} deletions)", d.delta.taille(), d.delta.suppressions()));
        }
        return;
    }
    *delta = lu.map(|d| {
        log(&format!("localization: mod texts captured ({} rows, {} deletions), replayed next time if the actions are the same", d.taille(), d.suppressions()));
        DeltaReleve { connexion: *((loc + CONNEXION) as *const usize), actions: actions(contenu), delta: d }
    });
}

/// Remise de la localisation à `Origin` depuis `UpdateContent` : ensuite, soit le delta est remis
/// (UpdateText sera neutralisé), soit la capture est posée.
unsafe extern "C" fn remise_loc(loc: usize) -> u8 {
    let orig: unsafe extern "C" fn(usize) -> u8 = std::mem::transmute(ORIG_REMISE_LOC.load(Ordering::Relaxed));
    let r = orig(loc);
    if !DANS_UPDATE.load(Ordering::Relaxed) || LOC.load(Ordering::Relaxed) != 0 || !LOC_ACTIF.load(Ordering::Relaxed) {
        return r;
    }
    LOC.store(loc, Ordering::Relaxed);
    let connexion = *((loc + CONNEXION) as *const usize);
    let contenu = CONTENU.load(Ordering::Relaxed);
    if connexion == 0 || contenu == 0 || gestionnaire(contenu + CADRE, b"UpdateText") == 0 {
        return r;
    }
    let c = localisation::Connexion(connexion);
    {
        let delta = DELTA.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(d) = delta.as_ref() {
            if d.connexion == connexion && d.actions == actions(contenu) {
                if localisation::rejouer(&c, &d.delta) {
                    LOC_GARDEE.store(true, Ordering::Relaxed);
                    // « Reverted » : le jeu remettra en cache les textes, qui ont changé.
                    return 1;
                }
                return r;
            }
            let why = if d.connexion != connexion { "new connection" } else { "different mod actions" };
            log(&format!("localization: mod texts rebuilt ({why})"));
        }
    }
    *CAPTURE.lock().unwrap_or_else(|e| e.into_inner()) = localisation::commencer_capture(&c);
    r
}

unsafe extern "C" fn remise(base: usize) -> u8 {
    let orig: unsafe extern "C" fn(usize) -> u8 = std::mem::transmute(ORIG_REMISE.load(Ordering::Relaxed));
    if !DANS_UPDATE.load(Ordering::Relaxed) || BASE.load(Ordering::Relaxed) != 0 {
        return orig(base);
    }
    BASE.store(base, Ordering::Relaxed);
    let connexion = *((base + CONNEXION) as *const usize);
    let contenu = CONTENU.load(Ordering::Relaxed);
    // Sans gestionnaire UpdateDatabase identifiable, on ne saurait pas le neutraliser : la base
    // est reconstruite.
    let garder = connexion != 0 && contenu != 0 && gestionnaire(contenu + CADRE, b"UpdateDatabase") != 0 && {
        let releve = RELEVE.lock().unwrap_or_else(|e| e.into_inner());
        match releve.as_ref() {
            Some(r) => {
                let actuel = Releve { connexion, compteur: compteur(connexion), modifications: modifications(connexion), actions: actions(contenu) };
                let ok = *r == actuel && r.modifications.is_some();
                if !ok {
                    let why = if r.connexion != connexion {
                        "new connection"
                    } else if r.compteur != actuel.compteur || r.modifications != actuel.modifications {
                        "database modified since"
                    } else {
                        "different mod actions"
                    };
                    log(&format!("database: rebuilt ({why})"));
                }
                ok
            }
            None => false,
        }
    };
    if garder {
        GARDEE.store(true, Ordering::Relaxed);
        // « Gameplay database unchanged, no rollback needed. »
        return 0;
    }
    orig(base)
}

unsafe extern "C" fn embarquees(contenu: usize) -> u8 {
    if GARDEE.load(Ordering::Relaxed) {
        return 1;
    }
    let orig: unsafe extern "C" fn(usize) -> u8 = std::mem::transmute(ORIG_EMBARQUEES.load(Ordering::Relaxed));
    orig(contenu)
}

unsafe extern "C" fn copie_debogage(base: usize) {
    if GARDEE.load(Ordering::Relaxed) {
        return;
    }
    let orig: unsafe extern "C" fn(usize) = std::mem::transmute(ORIG_COPIE.load(Ordering::Relaxed));
    orig(base)
}

// Gestionnaire neutre : BeginApplyComponents, ApplyComponent et EndApplyComponents réussissent
// sans rien faire.
unsafe extern "C" fn neutre_detruit(_this: *mut c_void, _flags: u32) {}
unsafe extern "C" fn neutre_oui(_this: *mut c_void, _arg: *mut c_void) -> u8 {
    1
}
#[repr(C)]
struct Neutre {
    vtable: *const usize,
}
unsafe impl Sync for Neutre {}
struct Vt([usize; 4]);
static NEUTRE_VT: std::sync::OnceLock<Vt> = std::sync::OnceLock::new();
static NEUTRE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

fn neutre() -> usize {
    *NEUTRE.get_or_init(|| {
        let vt = NEUTRE_VT.get_or_init(|| {
            Vt([neutre_detruit as *const () as usize, neutre_oui as *const () as usize, neutre_oui as *const () as usize, neutre_oui as *const () as usize])
        });
        Box::into_raw(Box::new(Neutre { vtable: vt.0.as_ptr() })) as usize
    })
}

/// CRC-32 (polynôme 0xEDB88320) du nom de type : le jeu indexe ses gestionnaires ainsi
/// (`140b3cee0`). Les deux variantes (avec et sans inversion finale) sont admises.
fn crc32(s: &[u8]) -> (u32, u32) {
    let mut c = 0xffff_ffffu32;
    for &b in s {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
        }
    }
    (!c, c)
}

/// Le gestionnaire du type d'action `nom` dans le cadre de mods, 0 s'il n'est pas trouvé.
unsafe fn gestionnaire(cadre: usize, nom: &[u8]) -> usize {
    let (a, b) = crc32(nom);
    let tete = *((cadre + GESTIONNAIRES) as *const usize);
    if tete == 0 {
        return 0;
    }
    let mut n = *(tete as *const usize);
    let mut garde = 0;
    while n != tete && n != 0 && garde < 4096 {
        let ty = *((n + 0x10) as *const u32);
        if ty == a || ty == b {
            return *((n + 0x18) as *const usize);
        }
        n = *(n as *const usize);
        garde += 1;
    }
    0
}

/// Remplace par le neutre, dans tous les nœuds qui le désignent, le gestionnaire du type `nom`.
/// Rend les emplacements modifiés et le gestionnaire d'origine.
unsafe fn neutraliser(cadre: usize, nom: &[u8]) -> (Vec<*mut usize>, usize) {
    let cible = gestionnaire(cadre, nom);
    let tete = *((cadre + GESTIONNAIRES) as *const usize);
    let mut remplaces = Vec::new();
    if cible == 0 || tete == 0 {
        return (remplaces, cible);
    }
    let mut n = *(tete as *const usize);
    while n != tete && n != 0 && remplaces.len() < 64 {
        let slot = (n + 0x18) as *mut usize;
        if *slot == cible {
            *slot = neutre();
            remplaces.push(slot);
        }
        n = *(n as *const usize);
    }
    (remplaces, cible)
}

unsafe extern "C" fn composants(cadre: usize, liste: *mut c_void) -> u8 {
    let orig: unsafe extern "C" fn(usize, *mut c_void) -> u8 = std::mem::transmute(ORIG_COMPOSANTS.load(Ordering::Relaxed));
    let mut restaurer = Vec::new();
    if GARDEE.load(Ordering::Relaxed) {
        let (v, c) = neutraliser(cadre, b"UpdateDatabase");
        if v.is_empty() {
            log("database: UpdateDatabase handler vanished before the components, the kept database may be incomplete");
        }
        restaurer.extend(v.into_iter().map(|s| (s, c)));
    }
    if LOC_GARDEE.load(Ordering::Relaxed) {
        let (v, c) = neutraliser(cadre, b"UpdateText");
        restaurer.extend(v.into_iter().map(|s| (s, c)));
    }
    let r = orig(cadre, liste);
    for (slot, c) in restaurer {
        *slot = c;
    }
    // Capture : lecture du delta après les composants.
    if let Some(avant) = CAPTURE.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let loc = LOC.load(Ordering::Relaxed);
        if loc != 0 {
            let c = localisation::Connexion(*((loc + CONNEXION) as *const usize));
            if r != 0 {
                *DELTA_LU.lock().unwrap_or_else(|e| e.into_inner()) = localisation::lire_capture(&c, avant);
            }
        }
    }
    r
}

static CONTENU: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn update_content_entree(contenu: usize, resultat: *mut u8, rappel: *mut c_void) -> *mut u8 {
    CONTENU.store(contenu, Ordering::Relaxed);
    update_content(contenu, resultat, rappel)
}

/// Le réglage `textes_gardes` (dernière occurrence ; `on` par défaut).
pub fn textes_enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("textes_gardes=")).last().map(str::trim) != Some("off")
}

/// Le réglage `bdd_gardee` (dernière occurrence ; `on` par défaut).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("bdd_gardee=")).last().map(str::trim) != Some("off")
}

pub fn install(exe_base: usize) {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("database: not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log("database: rebuilt at every load (bdd_gardee=off)");
        return;
    }
    LOC_ACTIF.store(textes_enabled(&cfg), Ordering::Relaxed);
    if !textes_enabled(&cfg) {
        log("localization: mod texts rebuilt at every load (textes_gardes=off)");
    }
    unsafe {
        if let Err(e) = installer(exe_base) {
            log(&format!("database: not installed: {e}"));
        }
    }
}

unsafe fn unique(exe_base: usize, nom: &str, m: &str) -> Result<Vec<usize>, String> {
    let v = motifs::chercher(exe_base, &motifs::motif(m));
    if v.is_empty() {
        return Err(format!("{nom} not found"));
    }
    Ok(v)
}

unsafe fn installer(exe_base: usize) -> Result<(), String> {
    let uc = unique(exe_base, "UpdateContent", UPDATE_CONTENT)?;
    let [uc] = uc[..] else { return Err(format!("UpdateContent found {} times", uc.len())) };
    let cibles: [(&str, Vec<usize>); 5] = [
        ("reset", unique(exe_base, "gameplay database reset", REMISE)?),
        ("embedded", unique(exe_base, "embedded data", EMBARQUEES)?),
        ("components", unique(exe_base, "component application", COMPOSANTS)?),
        ("debug copy", unique(exe_base, "debug copy", COPIE_DEBOGAGE)?),
        ("localization reset", unique(exe_base, "localization reset", REMISE_LOC)?),
    ];
    // Les appels de ces fonctions depuis UpdateContent : exactement un chacun.
    let corps = std::slice::from_raw_parts(uc as *const u8, UPDATE_CONTENT_TAILLE);
    let mut sites = [0usize; 5];
    for (i, (nom, adresses)) in cibles.iter().enumerate() {
        let trouves: Vec<usize> = (0..corps.len() - 5)
            .filter(|&k| corps[k] == 0xe8 && adresses.contains(&motifs::cible_rel32(uc + k)))
            .map(|k| uc + k)
            .collect();
        let [s] = trouves[..] else { return Err(format!("{} calls to {nom} in UpdateContent", trouves.len())) };
        sites[i] = s;
    }
    let hooks = [
        remise as *const () as usize,
        embarquees as *const () as usize,
        composants as *const () as usize,
        copie_debogage as *const () as usize,
        remise_loc as *const () as usize,
    ];
    let origs = [&ORIG_REMISE, &ORIG_EMBARQUEES, &ORIG_COMPOSANTS, &ORIG_COPIE, &ORIG_REMISE_LOC];
    // Les originales d'abord, avant de poser quoi que ce soit.
    for (s, o) in sites.iter().zip(origs) {
        o.store(motifs::cible_rel32(*s), Ordering::Relaxed);
    }
    let prologue: [u8; UPDATE_CONTENT_COPIE] = exe::rd(uc);
    let tramp = patch::install_at("UpdateContent", uc, &prologue, uc + UPDATE_CONTENT_COPIE, update_content_entree as *const () as usize)?;
    ORIG_UPDATE.store(tramp, Ordering::Relaxed);
    for (s, h) in sites.iter().zip(hooks) {
        motifs::rediriger_appel(*s, h)?;
    }
    log("database: gameplay database and mod texts kept between loads when the mod actions are the same");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglage() {
        assert!(enabled(""));
        assert!(!enabled("bdd_gardee=off"));
    }

    #[test]
    fn crc() {
        // Valeur connue du CRC-32 standard.
        assert_eq!(crc32(b"123456789").0, 0xcbf4_3926);
    }
}
