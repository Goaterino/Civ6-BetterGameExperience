//! Départ et fin de tour simultanés (option « No First/Last moves (BGD) ») sous macOS : le pendant
//! de depart.rs (voir son en-tête et depart_tour/README.md). Correctif POL : rien n'est retenu si
//! `pol::actifs()` est faux (le Lua demande `Civ6RolloverProxy_DepartDispo`, qui le lit).
//!
//! Sous macOS les deux dylibs vivent dans le processus du jeu : ce proxy de GameCore détourne aussi
//! deux fonctions de l'exe (`Civ6_Exe_Child`, l'image `MH_EXECUTE`, adresses du Mach-O + slide).
//! Relevé sur Civ6 Aspyr 1.4.6 (rapports A et B du portage) :
//!
//! - **Envoi des ordres** (GameCore) : `void Send(Message*)`, arm64 `0x8481ac`, x86_64 `0x826570`
//!   (chaîne `GameCore SEND: %s`, 19 appelants). Message : type u32 `+0x08`, nom `+0x10`,
//!   `PlayerOperation` joueur `+0x18`, opération `+0x20`, comme Windows. Le détour rend sans appeler
//!   l'original pour un ordre retenu.
//! - **Minuteur** (exe) : pause et reprise des chronos du tour, arm64 `0x10137cf24`, x86_64
//!   `0x1014225e0` ; chronos en `+0x18/+0x20/+0x28` et `+0x30/+0x38/+0x40` comme Windows. Le
//!   compteur est `Platform::QueryPlatformCounter` du jeu (`mach_absolute_time` brut), lu dans le
//!   corps de la fonction : les mêmes unités que celles qu'elle écrit.
//! - **Publication des événements GameCore** (exe) : arm64 `0x1011ed270`, x86_64 `0x101286e00`,
//!   refaite comme `hook_publier` de Windows. Différence : la distribution d'un enregistrement est
//!   inlinée dans l'original (pas de `distribuer`), le détour fait donc lui-même ses quatre pas
//!   (`P+8 = rec[8]`, `P+0x10 = rec+0x10`, foncteur libc++ `vtable+0x30` si la file a un
//!   observateur, entrée `+0x20` de la table des types, puis `P+0x10 = 0`).
//! - **Fin de partie / nouvelle partie** : `game_destroyed` (appelé par forward.rs sous Windows)
//!   passe par le relais `DllDestroyGameContext` de macos.rs : sa case `DESTROY` reçoit un relais
//!   nu qui garde tous les registres d'argument, appelle `game_destroyed` (qui fait `reset`) puis
//!   saute à la valeur précédente de la case (l'original, ou le relais d'un autre correctif : ils
//!   s'enchaînent). Aucun octet de code n'est écrit. `DllCreateGameContext` n'est pas touché (son
//!   corps est déjà détourné par drop_slot_mac.rs, et `base_original` lit la case `CREATE`) :
//!   `reset` y est appelé depuis le détour de drop_slot_mac.rs (`hook_nouvelle_partie`).
//!
//! **Quand écrire dans l'exe.** La table des noms d'événements et le publieur sont remplis par des
//! initialiseurs statiques de l'exe, qui tournent après le constructeur d'une dylib insérée mais
//! avant `main` : au chargement de GameCore (un `dlopen` de l'exe), ils le sont. La publication
//! (appelant unique, `Civ6App_PublishEvents`) et la pause du minuteur (gestionnaires d'événements,
//! distribués par la publication) ne tournent que sur le fil principal. Les détours de l'exe ne
//! sont donc posés que depuis le fil principal (`pthread_main_np`) : aucun autre fil ne peut être
//! dans leurs premiers octets pendant l'écriture. Le trampoline et toutes les adresses dont le
//! détour a besoin sont écrits avant le saut.
//! - arm64 : `patch::write_code` pose une copie complète de la page (avec le saut) par
//!   `mach_vm_remap` : un seul échange de page, la page reste exécutable et identique hors des 16
//!   octets du saut. Si GameCore est ouvert hors du fil principal, la pose est remise au premier
//!   appel de `BgdDepartDispo` (une native Lua, donc le fil principal).
//! - x86_64 (Rosetta, Mac Intel) : `write_code` passerait la page en RW (non exécutable) le temps
//!   d'écrire, alors que d'autres fils du jeu exécutent l'exe : on demande RWX par
//!   `mach_vm_protect` (`VM_PROT_COPY`), sinon rien n'est posé. Pas de pose différée : Rosetta a
//!   déjà traduit ce code. Le minuteur n'a que 13 octets de prologue avant un `call rel32` : le
//!   trampoline recopie les 13 octets, refait l'appel en absolu (`call [rip+2]`), puis reprend à
//!   `+18`. Aucun branchement ne vise `+1…+17` (vérifié par les tests).

#![allow(clippy::missing_safety_doc)]

use super::journal::log;
use super::{patch, pol};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

// ---------------------------------------------------------------------------------------------
// Règles (pures, testées).

/// Types de message (`+0x08`) que le joueur ne peut pas envoyer pendant l'attente (relevés dans les
/// constructeurs, les mêmes valeurs que Windows). `CivicsCommand`, `PlayerTurnComplete`,
/// `GameTurnComplete`, `GameUpdate` passent.
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
const M_TYPE: usize = 0x08;
const M_NOM: usize = 0x10;
const PO_JOUEUR: usize = 0x18;
const PO_OPERATION: usize = 0x20;

/// Un message de ce type (et, pour `PlayerOperation`, de ce joueur et cette opération) est-il un
/// ordre du joueur, retenu pendant l'attente ? `EXECUTE_SCRIPT` (scripts d'interface) passe.
fn bloque_regle(genre: u32, joueur_msg: i32, operation: u32, joueur: i32, execute_script: u32) -> bool {
    BLOQUES.contains(&genre) || (genre == PLAYER_OPERATION && joueur_msg == joueur && operation != execute_script)
}

/// Types d'événement (fixés à la compilation, table des noms à partir de 1) : début du tour local,
/// tour activé (joueur en `+4` des données).
const TYPE_DEBUT_LOCAL: u32 = 8;
const TYPE_ACTIVE: u32 = 15;
const NOMS_TYPES: [(&str, u32); 3] = [
    ("GameCoreEvent::EndTurnDirty", 1),
    ("GameCoreEvent::LocalPlayerTurnBegin", TYPE_DEBUT_LOCAL),
    ("GameCoreEvent::PlayerTurnActivated", TYPE_ACTIVE),
];

/// L'événement est-il l'activation du tour local ?
fn activation_regle(genre: u32, joueur_evt: i32, joueur: i32) -> bool {
    genre == TYPE_DEBUT_LOCAL || (genre == TYPE_ACTIVE && joueur_evt == joueur)
}

/// `call [rip+2] ; jmp +8 ; .quad appel ; jmp [rip+0] ; .quad reprise`, après `prologue` : le
/// trampoline x86_64 du minuteur (le `call rel32` de l'original refait en absolu).
#[cfg_attr(target_arch = "aarch64", allow(dead_code))]
fn trampoline_appel_x86(prologue: &[u8], appel: usize, reprise: usize) -> Vec<u8> {
    let mut t = prologue.to_vec();
    t.extend_from_slice(&[0xff, 0x15, 0x02, 0, 0, 0, 0xeb, 0x08]);
    t.extend_from_slice(&(appel as u64).to_le_bytes());
    t.extend_from_slice(&[0xff, 0x25, 0, 0, 0, 0]);
    t.extend_from_slice(&(reprise as u64).to_le_bytes());
    t
}

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

// ---------------------------------------------------------------------------------------------
// Adresses (Civ6 Aspyr 1.4.6). GameCore : relatives à l'image. Exe : adresses du Mach-O
// (image à 0x1_0000_0000), plus le slide.

struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    empreinte: u64,
}

#[cfg(target_arch = "aarch64")]
mod adresses {
    use super::Fonction;
    pub const ENVOI: Fonction = Fonction { nom: "envoi des messages", adresse: 0x8481ac, taille: 0x2d0, empreinte: 0x00f1_3526_e1f7_8977 };
    /// `stp x24,x23,[sp,#-0x40]! ; stp x22,x21 ; stp x20,x19 ; stp x29,x30`.
    pub const ENVOI_PROLOGUE: usize = 16;
    pub const MINUTEUR: Fonction = Fonction { nom: "pause du minuteur", adresse: 0x1_0137_cf24, taille: 0x10c, empreinte: 0x1098_f180_9e8c_1520 };
    /// `stp x22,x21,[sp,#-0x30]! ; stp x20,x19 ; stp x29,x30 ; add x29,sp,#0x20`.
    pub const MINUTEUR_PROLOGUE: usize = 16;
    /// `bl Platform::QueryPlatformCounter` dans le minuteur.
    pub const APPEL_COMPTEUR: usize = 0x5c;
    pub const PUBLIER: Fonction = Fonction { nom: "publication des événements", adresse: 0x1_011e_d270, taille: 0x2dc, empreinte: 0x8ecf_c8de_92a9_3d10 };
    pub const TASSER: Fonction = Fonction { nom: "tassement de la file", adresse: 0x1_011f_35f0, taille: 0x114, empreinte: 0x26b0_24fc_b327_fee6 };
    pub const TRANSFERER: Fonction = Fonction { nom: "transfert de la file", adresse: 0x1_011f_3704, taille: 0x1c0, empreinte: 0x8f38_2b77_3242_31da };
    pub const DONNEES: Fonction = Fonction { nom: "données d'un tampon", adresse: 0x1_00b0_5af0, taille: 0xc4, empreinte: 0xdece_16fc_c104_2886 };
    pub const OPERATEUR: Fonction = Fonction { nom: "operator() du foncteur", adresse: 0x1_011f_4048, taille: 0x1c, empreinte: 0x8dfd_0b08_96d1_f554 };
    pub const DETRUIRE: Fonction = Fonction { nom: "destroy() du foncteur", adresse: 0x1_011f_4040, taille: 4, empreinte: 0x2fe4_2bc0_3800_fb45 };
    pub const FONCTEUR_VTABLE: usize = 0x1_01b1_b060;
    pub const NOMS: usize = 0x1_0f09_9268;
}

#[cfg(target_arch = "x86_64")]
mod adresses {
    use super::Fonction;
    pub const ENVOI: Fonction = Fonction { nom: "envoi des messages", adresse: 0x826570, taille: 0x330, empreinte: 0x3f44_a28a_db83_ffe5 };
    /// `push rbp ; mov rbp,rsp ; push r15…rbx ; sub rsp,0x688`.
    pub const ENVOI_PROLOGUE: usize = 20;
    pub const MINUTEUR: Fonction = Fonction { nom: "pause du minuteur", adresse: 0x1_0142_25e0, taille: 0xe9, empreinte: 0xaf8a_bd2f_b0e6_379b };
    /// `push rbp ; mov rbp,rsp ; push r15 ; push r14 ; push rbx ; push rax ; mov rbx,rdi`, puis
    /// `call rel32` (5 octets) : reprise en +18.
    pub const MINUTEUR_PROLOGUE: usize = 13;
    pub const MINUTEUR_REPRISE: usize = 18;
    pub const APPEL_COMPTEUR: usize = 0x50;
    pub const PUBLIER: Fonction = Fonction { nom: "publication des événements", adresse: 0x1_0128_6e00, taille: 0x330, empreinte: 0xb304_0d13_032f_5a80 };
    pub const TASSER: Fonction = Fonction { nom: "tassement de la file", adresse: 0x1_0128_dc80, taille: 0x120, empreinte: 0xb998_8f41_1c2a_3697 };
    pub const TRANSFERER: Fonction = Fonction { nom: "transfert de la file", adresse: 0x1_0128_dda0, taille: 0x1f0, empreinte: 0x9a0e_d9e9_b38b_a582 };
    pub const DONNEES: Fonction = Fonction { nom: "données d'un tampon", adresse: 0x1_00e2_0080, taille: 0xc0, empreinte: 0xcf03_afc9_263e_15bf };
    pub const OPERATEUR: Fonction = Fonction { nom: "operator() du foncteur", adresse: 0x1_0128_e670, taille: 0x20, empreinte: 0xf843_b151_d010_81e8 };
    pub const DETRUIRE: Fonction = Fonction { nom: "destroy() du foncteur", adresse: 0x1_0128_e650, taille: 0x10, empreinte: 0x83df_ea89_27c0_8670 };
    pub const FONCTEUR_VTABLE: usize = 0x1_01c5_5f88;
    pub const NOMS: usize = 0x1_0298_2268;
}

use adresses::*;

/// Entrées de la table du foncteur (libc++ `__func`) : `destroy()`, `operator()(this, &u32, &u32)`.
const VT_DETRUIRE: usize = 0x20;
const VT_OPERATEUR: usize = 0x30;

/// Chronos : (départ compteur u64, cumul en secondes f64, en marche u8).
const CHRONOS: [(usize, usize, usize); 2] = [(0x18, 0x20, 0x28), (0x30, 0x38, 0x40)];

/// File `q = P + 0x20 + jeu * 0x90` : deux tampons de 0x18 (taille u32 en `+0x10`), bascule
/// `+0x30` (tampon lu `(~bascule) & 1`), taille max `+0x34`, décalage distribué `+0x3c`,
/// observateur `+0x40`.
const F_BASCULE: usize = 0x30;
const F_MAX: usize = 0x34;
const F_LU: usize = 0x3c;
const F_OBSERVATEUR: usize = 0x40;
/// Publieur : jeu courant `+4`, enregistrement en cours `+8` (u32) et `+0x10` (adresse), garde de
/// réentrée `+0x48c0`, table des types `+0xb0 + jeu * 0x4800` (0x30 par type, distribution `+0x20`).
const P_JEU: usize = 4;
const P_REC8: usize = 8;
const P_REC10: usize = 0x10;
const P_GARDE: usize = 0x48c0;
/// Enregistrement : taille u32 `+0`, type u32 `+4`, `+8`/`+0xc` (observateur), données `+0x50`.
const E_TYPE: usize = 4;
const E_DONNEES: usize = 0x50;
const ACTIVE_JOUEUR: usize = 4;

const SECURITE_S: f64 = 120.0;
const FORCE_S: f64 = 1.0e6;
const MAX_LOGS: u32 = 30;
const MAX_DIAG: u32 = 40;

// ---------------------------------------------------------------------------------------------
// État.

static ENVOI_TRAMP: AtomicUsize = AtomicUsize::new(0);
static MINUTEUR_TRAMP: AtomicUsize = AtomicUsize::new(0);
static MINUTEUR_THIS: AtomicUsize = AtomicUsize::new(0);
static COMPTEUR: AtomicUsize = AtomicUsize::new(0);
static PUBLIER_SITE: AtomicUsize = AtomicUsize::new(0);
static TASSER_F: AtomicUsize = AtomicUsize::new(0);
static TRANSFERER_F: AtomicUsize = AtomicUsize::new(0);
static DONNEES_F: AtomicUsize = AtomicUsize::new(0);
static VTABLE: AtomicUsize = AtomicUsize::new(0);
/// Détours de l'exe : 0 à tenter, 1 tentés, 2 remis au fil principal (arm64).
static EXE: AtomicU8 = AtomicU8::new(0);
static POSE: AtomicBool = AtomicBool::new(false);
static PREMIER_PUBLIER: AtomicBool = AtomicBool::new(false);
static PREMIER_MINUTEUR: AtomicBool = AtomicBool::new(false);

static VARIANTE_VUE: AtomicBool = AtomicBool::new(false);
static DIAG: AtomicU32 = AtomicU32::new(0);
static ARME: AtomicBool = AtomicBool::new(false);
static RETENU: AtomicBool = AtomicBool::new(false);
static REPRISE: AtomicBool = AtomicBool::new(false);
static ATTENTE: AtomicBool = AtomicBool::new(false);
/// Début de l'attente, en nanosecondes depuis `T0`.
static DEPUIS: AtomicU64 = AtomicU64::new(0);
static JOUEUR: AtomicI32 = AtomicI32::new(-1);
static EXECUTE_SCRIPT: AtomicU32 = AtomicU32::new(0);
static REJETS: AtomicU32 = AtomicU32::new(0);
static LOGS: AtomicU32 = AtomicU32::new(0);
static FORCE: AtomicBool = AtomicBool::new(false);

static T0: OnceLock<Instant> = OnceLock::new();

fn maintenant_ns() -> u64 {
    T0.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

fn attente_s() -> f64 {
    maintenant_ns().saturating_sub(DEPUIS.load(Ordering::Relaxed)) as f64 / 1e9
}

extern "C" {
    fn mach_absolute_time() -> u64;
    fn pthread_main_np() -> c_int;
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dladdr(addr: *const c_void, info: *mut DlInfo) -> c_int;
}

#[repr(C)]
struct DlInfo {
    fname: *const c_char,
    fbase: *mut c_void,
    sname: *const c_char,
    saddr: *mut c_void,
}

/// Le compteur du jeu (`Platform::QueryPlatformCounter`, lu dans le minuteur), sinon
/// `mach_absolute_time`, qu'il rend tel quel.
unsafe fn compteur() -> u64 {
    match COMPTEUR.load(Ordering::Relaxed) {
        0 => mach_absolute_time(),
        f => std::mem::transmute::<usize, unsafe extern "C" fn() -> u64>(f)(),
    }
}

unsafe fn rd<T: Copy>(a: usize) -> T {
    std::ptr::read_unaligned(a as *const T)
}

unsafe fn wr<T>(a: usize, v: T) {
    std::ptr::write_unaligned(a as *mut T, v)
}

// ---------------------------------------------------------------------------------------------
// Détours.

unsafe fn bloque(msg: usize) -> bool {
    bloque_regle(
        rd(msg + M_TYPE),
        rd(msg + PO_JOUEUR),
        rd(msg + PO_OPERATION),
        JOUEUR.load(Ordering::Relaxed),
        EXECUTE_SCRIPT.load(Ordering::Relaxed),
    )
}

unsafe fn nom(msg: usize) -> String {
    let p: usize = rd(msg + M_NOM);
    if p == 0 { "?".into() } else { CStr::from_ptr(p as *const c_char).to_string_lossy().into_owned() }
}

type Envoi = unsafe extern "C" fn(usize, usize, usize, usize, usize, usize);

/// `Send(Message*)` : seul `x0`/`rdi` compte, les autres registres d'argument sont relayés tels
/// quels.
unsafe extern "C" fn hook_envoi(a: usize, b: usize, c: usize, d: usize, e: usize, f: usize) {
    if a != 0 && ATTENTE.load(Ordering::Relaxed) && bloque(a) {
        if attente_s() > SECURITE_S {
            partir("délai de sécurité dépassé");
        } else {
            let n = REJETS.fetch_add(1, Ordering::Relaxed) + 1;
            if LOGS.fetch_add(1, Ordering::Relaxed) < MAX_LOGS {
                log(&format!("depart : {} retenu, tour pas encore lancé ({n})", nom(a)));
            }
            return;
        }
    }
    let real: Envoi = std::mem::transmute(ENVOI_TRAMP.load(Ordering::Relaxed));
    real(a, b, c, d, e, f)
}

unsafe fn chronos_a_zero(this: usize) {
    let now = compteur();
    for (depart, cumul, marche) in CHRONOS {
        wr(this + depart, now);
        wr(this + cumul, 0.0f64);
        wr(this + marche, 0u8);
    }
}

unsafe fn minuteur_original(this: usize) {
    let real: unsafe extern "C" fn(usize) = std::mem::transmute(MINUTEUR_TRAMP.load(Ordering::Relaxed));
    real(this)
}

unsafe extern "C" fn hook_minuteur(this: usize) {
    if this != 0 {
        if !PREMIER_MINUTEUR.swap(true, Ordering::Relaxed) {
            log("depart : pause du minuteur détournée, premier appel");
        }
        MINUTEUR_THIS.store(this, Ordering::Relaxed);
        if ATTENTE.load(Ordering::Relaxed) {
            chronos_a_zero(this);
            return;
        }
    }
    minuteur_original(this)
}

unsafe fn activation_locale(rec: usize) -> bool {
    activation_regle(rd(rec + E_TYPE), rd(rec + E_DONNEES + ACTIVE_JOUEUR), JOUEUR.load(Ordering::Relaxed))
}

/// La publication des événements GameCore, refaite (tasser, transférer, distribuer jusqu'à
/// épuisement) un enregistrement à la fois, comme `hook_publier` de Windows, avec la distribution
/// inlinée de l'original macOS.
unsafe extern "C" fn hook_publier(p: usize) {
    if !PREMIER_PUBLIER.swap(true, Ordering::Relaxed) {
        log("depart : publication des événements détournée, premier appel");
    }
    let garde = &*((p + P_GARDE) as *const AtomicU32);
    if garde.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        return;
    }
    if RETENU.load(Ordering::Relaxed) && attente_s() > SECURITE_S {
        partir("délai de sécurité dépassé, interface retenue");
    }
    if RETENU.load(Ordering::Relaxed) {
        garde.store(0, Ordering::Release);
        return;
    }
    let tasser: unsafe extern "C" fn(usize) = std::mem::transmute(TASSER_F.load(Ordering::Relaxed));
    let transferer: unsafe extern "C" fn(usize) = std::mem::transmute(TRANSFERER_F.load(Ordering::Relaxed));
    let donnees: unsafe extern "C" fn(usize) -> usize = std::mem::transmute(DONNEES_F.load(Ordering::Relaxed));
    let vtable = VTABLE.load(Ordering::Relaxed);
    let operateur: unsafe extern "C" fn(*const usize, *const u32, *const u32) = std::mem::transmute(rd::<usize>(vtable + VT_OPERATEUR));
    let detruire: unsafe extern "C" fn(*const usize) = std::mem::transmute(rd::<usize>(vtable + VT_DETRUIRE));
    // Reprise : les enregistrements retenus sont encore dans la file lue, après le décalage
    // distribué ; un tassement les effacerait.
    let mut reprise = REPRISE.swap(false, Ordering::Relaxed);
    loop {
        let jeu = rd::<u32>(p + P_JEU) as usize;
        let q = p + 0x20 + jeu * 0x90;
        if !reprise {
            tasser(q);
            transferer(q);
        }
        reprise = false;
        let tampon = q + (!rd::<u32>(q + F_BASCULE) & 1) as usize * 0x18;
        let taille = rd::<u32>(tampon + 0x10);
        if taille > rd::<u32>(q + F_MAX) {
            wr(q + F_MAX, taille);
        }
        let base = donnees(tampon);
        let table = p + 0xb0 + jeu * 0x4800;
        let observe = rd::<usize>(q + F_OBSERVATEUR) != 0;
        // Le foncteur de `GetPreDispatchFunction()` : {table, file}, sur la pile comme l'original.
        let foncteur = [vtable, q];
        if !VARIANTE_VUE.swap(true, Ordering::Relaxed) {
            log(&format!("depart : publication {} observateur de file", if observe { "avec" } else { "sans" }));
        }
        let mut cur = base + rd::<u32>(q + F_LU) as usize;
        let mut distribue = false;
        let mut retenu = false;
        if taille != 0 {
            let fin = donnees(tampon) + rd::<u32>(tampon + 0x10) as usize;
            while cur < fin {
                let genre = rd::<u32>(cur + E_TYPE);
                if (genre == TYPE_DEBUT_LOCAL || genre == TYPE_ACTIVE) && DIAG.fetch_add(1, Ordering::Relaxed) < MAX_DIAG {
                    let joueur = if genre == TYPE_ACTIVE { format!(" (player {})", rd::<i32>(cur + E_DONNEES + ACTIVE_JOUEUR)) } else { String::new() };
                    log(&format!("depart: event type {genre}{joueur} published, {}", if ARME.load(Ordering::Relaxed) { "armed" } else { "not armed" }));
                }
                if ARME.load(Ordering::Relaxed) && activation_locale(cur) {
                    wr(q + F_LU, (cur - base) as u32);
                    ARME.store(false, Ordering::Relaxed);
                    RETENU.store(true, Ordering::Relaxed);
                    attendre(JOUEUR.load(Ordering::Relaxed), EXECUTE_SCRIPT.load(Ordering::Relaxed));
                    log(&format!("depart : interface retenue avant le début du tour (type {genre})"));
                    retenu = true;
                    break;
                }
                let longueur = rd::<u32>(cur) as usize;
                if longueur == 0 {
                    break;
                }
                // Les quatre pas de l'original pour un enregistrement.
                let (a, b) = (rd::<u32>(cur + 8), rd::<u32>(cur + 0xc));
                wr(p + P_REC8, a);
                wr(p + P_REC10, cur + 0x10);
                if observe {
                    operateur(foncteur.as_ptr(), &a, &b);
                }
                let entree = table + genre as usize * 0x30;
                let distribuer: unsafe extern "C" fn(usize, usize) = std::mem::transmute(rd::<usize>(entree + 0x20));
                distribuer(cur + E_DONNEES, entree);
                wr(p + P_REC10, 0usize);
                cur += longueur;
                distribue = true;
            }
        }
        if observe {
            detruire(foncteur.as_ptr());
        }
        if retenu {
            garde.store(0, Ordering::Release);
            return;
        }
        // Comme l'original : file et tampon relus (le jeu courant ne change pas en route).
        let q = p + 0x20 + rd::<u32>(p + P_JEU) as usize * 0x90;
        let tampon = q + (!rd::<u32>(q + F_BASCULE) & 1) as usize * 0x18;
        wr(q + F_LU, rd::<u32>(tampon + 0x10));
        if !distribue {
            break;
        }
    }
    garde.store(0, Ordering::Release);
}

// ---------------------------------------------------------------------------------------------
// Attente, départ, fin forcée : comme Windows.

fn armer(joueur: i32, execute_script: u32) {
    if PUBLIER_SITE.load(Ordering::Relaxed) == 0 {
        return;
    }
    JOUEUR.store(joueur, Ordering::Relaxed);
    EXECUTE_SCRIPT.store(execute_script, Ordering::Relaxed);
    if !ARME.swap(true, Ordering::Relaxed) {
        log(&format!("depart : armé pour le prochain tour (joueur {joueur})"));
    }
}

unsafe fn attendre(joueur: i32, execute_script: u32) {
    JOUEUR.store(joueur, Ordering::Relaxed);
    EXECUTE_SCRIPT.store(execute_script, Ordering::Relaxed);
    REJETS.store(0, Ordering::Relaxed);
    DEPUIS.store(maintenant_ns(), Ordering::Relaxed);
    ATTENTE.store(true, Ordering::Relaxed);
    let this = MINUTEUR_THIS.load(Ordering::Relaxed);
    if this != 0 && MINUTEUR_TRAMP.load(Ordering::Relaxed) != 0 {
        chronos_a_zero(this);
    }
    log(&format!("depart : attente (joueur {joueur})"));
}

unsafe fn fin_forcer() -> bool {
    let this = MINUTEUR_THIS.load(Ordering::Relaxed);
    if this == 0 || MINUTEUR_TRAMP.load(Ordering::Relaxed) == 0 {
        return false;
    }
    if ATTENTE.swap(false, Ordering::Relaxed) {
        log("depart : attente levée par la fin du tour");
    }
    lacher();
    for (_, cumul, _) in CHRONOS {
        let v: f64 = rd(this + cumul);
        if v < FORCE_S {
            wr(this + cumul, v + FORCE_S);
        }
    }
    FORCE.store(true, Ordering::Relaxed);
    log("depart : fin du tour forcée (minuteur de l'hôte écoulé)");
    true
}

unsafe fn fin_lever() {
    if !FORCE.swap(false, Ordering::Relaxed) {
        return;
    }
    let this = MINUTEUR_THIS.load(Ordering::Relaxed);
    if this == 0 || MINUTEUR_TRAMP.load(Ordering::Relaxed) == 0 {
        return;
    }
    chronos_a_zero(this);
    if !ATTENTE.load(Ordering::Relaxed) {
        minuteur_original(this);
    }
    log("depart : fin forcée levée, minuteur remis à zéro");
}

fn lacher() -> bool {
    let retenu = RETENU.swap(false, Ordering::Relaxed);
    if retenu {
        REPRISE.store(true, Ordering::Relaxed);
    }
    retenu
}

fn partir(pourquoi: &str) -> bool {
    let retenu = lacher();
    if !ATTENTE.swap(false, Ordering::Relaxed) {
        return retenu;
    }
    unsafe {
        let this = MINUTEUR_THIS.load(Ordering::Relaxed);
        if this != 0 && MINUTEUR_TRAMP.load(Ordering::Relaxed) != 0 {
            chronos_a_zero(this);
            minuteur_original(this);
        }
    }
    log(&format!("depart : parti ({pourquoi}, {:.1} s d'attente, {} ordres retenus)", attente_s(), REJETS.load(Ordering::Relaxed)));
    true
}

/// Nouvelle partie : aucune attente ne survit à la précédente, et le gestionnaire du tour vu
/// (`this` du minuteur) n'est plus valable.
pub fn reset() {
    ATTENTE.store(false, Ordering::Relaxed);
    ARME.store(false, Ordering::Relaxed);
    RETENU.store(false, Ordering::Relaxed);
    REPRISE.store(false, Ordering::Relaxed);
    DIAG.store(0, Ordering::Relaxed);
    VARIANTE_VUE.store(false, Ordering::Relaxed);
    FORCE.store(false, Ordering::Relaxed);
    MINUTEUR_THIS.store(0, Ordering::Relaxed);
    REJETS.store(0, Ordering::Relaxed);
    LOGS.store(0, Ordering::Relaxed);
}

/// La partie est détruite : une retenue en cours tombe, sans toucher au minuteur, qui part avec.
pub fn game_destroyed() {
    let held = ATTENTE.load(Ordering::Relaxed) || RETENU.load(Ordering::Relaxed);
    reset();
    if held {
        log("depart: game destroyed, turn-start hold dropped");
    }
}

// ---------------------------------------------------------------------------------------------
// Nouvelle partie et fin de partie, par les cases des relais de macos.rs.

static SUITE_DESTRUCTION: AtomicUsize = AtomicUsize::new(0);

extern "C" fn avant_destruction() {
    game_destroyed();
}

/// Relais nu : garde les registres d'argument (entiers, flottants, `x8`/`rax`), appelle `$avant`,
/// les rend, puis saute à l'adresse rangée dans `$suite`. La pile de l'appelant n'est pas touchée :
/// la signature de la fonction relayée importe peu.
macro_rules! relais_avant {
    ($name:ident, $avant:ident, $suite:ident) => {
        #[unsafe(naked)]
        unsafe extern "C" fn $name() {
            #[cfg(target_arch = "aarch64")]
            core::arch::naked_asm!(
                "stp x29, x30, [sp, #-0xe0]!",
                "mov x29, sp",
                "stp x0, x1, [sp, #0x10]",
                "stp x2, x3, [sp, #0x20]",
                "stp x4, x5, [sp, #0x30]",
                "stp x6, x7, [sp, #0x40]",
                "str x8, [sp, #0x50]",
                "stp q0, q1, [sp, #0x60]",
                "stp q2, q3, [sp, #0x80]",
                "stp q4, q5, [sp, #0xa0]",
                "stp q6, q7, [sp, #0xc0]",
                "bl {avant}",
                "ldp q6, q7, [sp, #0xc0]",
                "ldp q4, q5, [sp, #0xa0]",
                "ldp q2, q3, [sp, #0x80]",
                "ldp q0, q1, [sp, #0x60]",
                "ldr x8, [sp, #0x50]",
                "ldp x6, x7, [sp, #0x40]",
                "ldp x4, x5, [sp, #0x30]",
                "ldp x2, x3, [sp, #0x20]",
                "ldp x0, x1, [sp, #0x10]",
                "ldp x29, x30, [sp], #0xe0",
                "adrp x16, {suite}@PAGE",
                "ldr x16, [x16, {suite}@PAGEOFF]",
                "br x16",
                avant = sym $avant,
                suite = sym $suite,
            );
            #[cfg(target_arch = "x86_64")]
            core::arch::naked_asm!(
                "push rbp",
                "mov rbp, rsp",
                "sub rsp, 0xc0",
                "mov [rsp], rdi",
                "mov [rsp + 0x8], rsi",
                "mov [rsp + 0x10], rdx",
                "mov [rsp + 0x18], rcx",
                "mov [rsp + 0x20], r8",
                "mov [rsp + 0x28], r9",
                "mov [rsp + 0x30], rax",
                "movdqu [rsp + 0x40], xmm0",
                "movdqu [rsp + 0x50], xmm1",
                "movdqu [rsp + 0x60], xmm2",
                "movdqu [rsp + 0x70], xmm3",
                "movdqu [rsp + 0x80], xmm4",
                "movdqu [rsp + 0x90], xmm5",
                "movdqu [rsp + 0xa0], xmm6",
                "movdqu [rsp + 0xb0], xmm7",
                "call {avant}",
                "movdqu xmm7, [rsp + 0xb0]",
                "movdqu xmm6, [rsp + 0xa0]",
                "movdqu xmm5, [rsp + 0x90]",
                "movdqu xmm4, [rsp + 0x80]",
                "movdqu xmm3, [rsp + 0x70]",
                "movdqu xmm2, [rsp + 0x60]",
                "movdqu xmm1, [rsp + 0x50]",
                "movdqu xmm0, [rsp + 0x40]",
                "mov rax, [rsp + 0x30]",
                "mov r9, [rsp + 0x28]",
                "mov r8, [rsp + 0x20]",
                "mov rcx, [rsp + 0x18]",
                "mov rdx, [rsp + 0x10]",
                "mov rsi, [rsp + 0x8]",
                "mov rdi, [rsp]",
                "add rsp, 0xc0",
                "pop rbp",
                "jmp qword ptr [rip + {suite}]",
                avant = sym $avant,
                suite = sym $suite,
            );
        }
    };
}
relais_avant!(relais_destruction, avant_destruction, SUITE_DESTRUCTION);

/// La case `DESTROY` de macos.rs (lue par son relais à chaque appel) reçoit notre relais ; son
/// ancienne valeur devient la suite. Au constructeur : l'exe n'a pas encore les exports.
unsafe fn poser_relais() -> bool {
    let destroy = std::ptr::addr_of_mut!(super::DESTROY);
    if destroy.read() == 0 || SUITE_DESTRUCTION.load(Ordering::Relaxed) != 0 {
        return false;
    }
    SUITE_DESTRUCTION.store(destroy.read(), Ordering::Release);
    destroy.write(relais_destruction as *const () as usize);
    true
}

// ---------------------------------------------------------------------------------------------
// Pose.

/// Le proxy ne sera jamais déchargé : l'exe et GameCore sautent dans son code.
unsafe fn garder_charge() {
    const RTLD_NOW: c_int = 0x2;
    const RTLD_NOLOAD: c_int = 0x10;
    const RTLD_NODELETE: c_int = 0x80;
    let mut info: DlInfo = std::mem::zeroed();
    if dladdr(garder_charge as *const c_void, &mut info) == 0 || info.fname.is_null() {
        log("depart : chemin du proxy introuvable (dladdr), RTLD_NODELETE non posé");
        return;
    }
    if dlopen(info.fname, RTLD_NOW | RTLD_NOLOAD | RTLD_NODELETE).is_null() {
        log("depart : RTLD_NODELETE refusé");
    }
}

/// Correctif de la liste de macos.rs : envoi des ordres (GameCore), relais de nouvelle partie,
/// puis les détours de l'exe (minuteur, publication) si le fil le permet.
pub unsafe fn installer(base: usize) -> Result<String, String> {
    if POSE.load(Ordering::Relaxed) {
        return Ok("déjà posé".into());
    }
    let envoi = base + ENVOI.adresse;
    if fnv(std::slice::from_raw_parts(envoi as *const u8, ENVOI.taille)) != ENVOI.empreinte {
        return Err(format!("version inattendue ({} en {:#x}), départ simultané indisponible", ENVOI.nom, ENVOI.adresse));
    }
    garder_charge();
    if !poser_relais() {
        return Err("relais de DllDestroyGameContext indisponible, départ simultané non posé".into());
    }
    let methode = patch::hook_with_trampoline_publie(envoi, ENVOI_PROLOGUE, hook_envoi as *const () as usize, &ENVOI_TRAMP)?;
    POSE.store(true, Ordering::Relaxed);
    let exe = poser_exe("au chargement de GameCore");
    Ok(format!("envoi des ordres détourné ({methode}), fin de partie suivie ; {exe}"))
}

/// Les détours de l'exe, une fois (ou remis à plus tard en arm64, hors du fil principal).
unsafe fn poser_exe(quand: &str) -> String {
    if pthread_main_np() == 0 {
        if cfg!(target_arch = "aarch64") {
            EXE.store(2, Ordering::Relaxed);
            return "minuteur et publication remis au fil principal (première native Lua)".into();
        }
        EXE.store(1, Ordering::Relaxed);
        return "minuteur et publication laissés tels quels : GameCore chargé hors du fil principal (x86_64, pas de pose tardive)".into();
    }
    EXE.store(1, Ordering::Relaxed);
    let Some((mh, slide)) = super::macho::executable() else {
        return "exécutable principal introuvable, minuteur et publication laissés tels quels".into();
    };
    let Some(text) = super::macho::text_section(mh, slide) else {
        return "section __text de l'exe introuvable, minuteur et publication laissés tels quels".into();
    };
    let s = slide as usize;
    let minuteur = poser_minuteur(s, text).map_or_else(|e| format!("minuteur laissé tel quel : {e}"), |m| format!("minuteur détourné ({m})"));
    let publication = poser_publication(s, text).map_or_else(
        |e| format!("publication laissée telle quelle, attente après le début du tour : {e}"),
        |m| format!("publication détournée, début de tour retenu avant l'interface ({m})"),
    );
    format!("exe ({quand}, slide {s:#x}) : {minuteur} ; {publication}")
}

/// La fonction `f` de l'exe est-elle celle relevée ? Rend son adresse en mémoire.
unsafe fn verifier(f: &Fonction, slide: usize, text: (usize, usize)) -> Result<usize, String> {
    let at = f.adresse.wrapping_add(slide);
    if at < text.0 || at + f.taille > text.0 + text.1 {
        return Err(format!("{} hors de __text", f.nom));
    }
    if fnv(std::slice::from_raw_parts(at as *const u8, f.taille)) != f.empreinte {
        return Err(format!("version inattendue ({} en {:#x})", f.nom, f.adresse));
    }
    Ok(at)
}

unsafe fn poser_minuteur(slide: usize, text: (usize, usize)) -> Result<String, String> {
    let at = verifier(&MINUTEUR, slide, text)?;
    let appel = at + APPEL_COMPTEUR;
    #[cfg(target_arch = "aarch64")]
    let cible = patch::cible_branche(appel, rd::<u32>(appel));
    #[cfg(target_arch = "x86_64")]
    let cible = patch::cible_rel32_x86(appel, std::slice::from_raw_parts(appel as *const u8, 5));
    COMPTEUR.store(cible.ok_or("appel du compteur attendu")?, Ordering::Relaxed);
    #[cfg(target_arch = "aarch64")]
    {
        patch::hook_with_trampoline_publie(at, MINUTEUR_PROLOGUE, hook_minuteur as *const () as usize, &MINUTEUR_TRAMP)
            .map(|m| format!("exe+{:#x}, {m}", MINUTEUR.adresse - 0x1_0000_0000))
    }
    #[cfg(target_arch = "x86_64")]
    {
        let call = at + MINUTEUR_PROLOGUE;
        let f1 = patch::cible_rel32_x86(call, std::slice::from_raw_parts(call as *const u8, 5)).ok_or("call rel32 attendu après le prologue")?;
        let prologue = std::slice::from_raw_parts(at as *const u8, MINUTEUR_PROLOGUE);
        let tramp = patch::executable_near(&trampoline_appel_x86(prologue, f1, at + MINUTEUR_REPRISE), at, 1 << 40)?;
        MINUTEUR_TRAMP.store(tramp, Ordering::Release);
        match ecrire_rwx(at, &patch::jump(hook_minuteur as *const () as usize)) {
            Ok(m) => Ok(format!("exe+{:#x}, appel déplacé dans le trampoline, {m}", MINUTEUR.adresse - 0x1_0000_0000)),
            Err(e) => {
                MINUTEUR_TRAMP.store(0, Ordering::Relaxed);
                Err(e)
            }
        }
    }
}

unsafe fn poser_publication(slide: usize, text: (usize, usize)) -> Result<String, String> {
    let at = verifier(&PUBLIER, slide, text)?;
    let tasser = verifier(&TASSER, slide, text)?;
    let transferer = verifier(&TRANSFERER, slide, text)?;
    let donnees = verifier(&DONNEES, slide, text)?;
    let operateur = verifier(&OPERATEUR, slide, text)?;
    let detruire = verifier(&DETRUIRE, slide, text)?;
    // Même version de l'exe : la table du foncteur et celle des noms sont à leur place.
    let vtable = FONCTEUR_VTABLE.wrapping_add(slide);
    if rd::<usize>(vtable + VT_OPERATEUR) != operateur || rd::<usize>(vtable + VT_DETRUIRE) != detruire {
        return Err("table du foncteur inattendue".into());
    }
    verifier_types(NOMS.wrapping_add(slide))?;
    TASSER_F.store(tasser, Ordering::Relaxed);
    TRANSFERER_F.store(transferer, Ordering::Relaxed);
    DONNEES_F.store(donnees, Ordering::Relaxed);
    VTABLE.store(vtable, Ordering::Release);
    let saut = patch::jump(hook_publier as *const () as usize);
    // L'original n'est jamais rappelé : pas de trampoline, le saut couvre son début (16 octets en
    // arm64, 14 des 17 du prologue en x86_64).
    #[cfg(target_arch = "aarch64")]
    let m = patch::write_code(at, &saut)?;
    #[cfg(target_arch = "x86_64")]
    let m = ecrire_rwx(at, &saut)?;
    PUBLIER_SITE.store(at, Ordering::Release);
    Ok(format!("exe+{:#x}, {m}", PUBLIER.adresse - 0x1_0000_0000))
}

/// Les types 8 et 15 sont-ils ceux qu'on croit ? La table des noms (un pointeur par type à partir
/// de 1, remplie par un initialiseur statique de l'exe) est lue maintenant, pas au constructeur.
unsafe fn verifier_types(table: usize) -> Result<(), String> {
    for (nom, t) in NOMS_TYPES {
        let p: usize = rd(table + (t as usize - 1) * 8);
        if p == 0 {
            return Err("table des noms d'événements encore vide".into());
        }
        if CStr::from_ptr(p as *const c_char).to_bytes() != nom.as_bytes() {
            return Err(format!("type {t} : {nom} attendu"));
        }
    }
    Ok(())
}

/// x86_64 : écrit dans une page de l'exe sans la rendre non exécutable (d'autres fils exécutent
/// l'exe) : copie privée RWX, puis RX. Refusé : rien n'est écrit.
#[cfg(target_arch = "x86_64")]
unsafe fn ecrire_rwx(addr: usize, bytes: &[u8]) -> Result<&'static str, String> {
    extern "C" {
        static mach_task_self_: u32;
        fn mach_vm_protect(task: u32, addr: u64, size: u64, set_maximum: c_int, prot: c_int) -> c_int;
        fn getpagesize() -> c_int;
        fn sys_icache_invalidate(start: *mut c_void, len: usize);
    }
    const R: c_int = 1;
    const W: c_int = 2;
    const X: c_int = 4;
    const COPY: c_int = 0x10;
    let ps = getpagesize() as usize;
    let debut = addr & !(ps - 1);
    let len = ((addr + bytes.len() + ps - 1) & !(ps - 1)) - debut;
    let kr = mach_vm_protect(mach_task_self_, debut as u64, len as u64, 0, R | W | X | COPY);
    if kr != 0 {
        return Err(format!("page de code non inscriptible en restant exécutable (mach_vm_protect RWX : {kr})"));
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), addr as *mut u8, bytes.len());
    let kr = mach_vm_protect(mach_task_self_, debut as u64, len as u64, 0, R | X);
    if kr != 0 {
        log(&format!("depart : mach_vm_protect RX : erreur {kr} (page laissée RWX)"));
    }
    sys_icache_invalidate(addr as *mut c_void, bytes.len());
    Ok("copie RWX par mach_vm_protect")
}

/// Pose tardive (arm64) : au premier appel d'une native, sur le fil principal.
unsafe fn pose_tardive() {
    if EXE.load(Ordering::Relaxed) == 2 && pthread_main_np() != 0 {
        let m = poser_exe("à la première native Lua");
        log(&format!("depart : {m}"));
    }
}

// ---------------------------------------------------------------------------------------------
// Exports, lus par le chargeur (startup_proxy/src/macos_depart.rs) par `dlsym` : les natives Lua
// vivent dans le chargeur.

/// Bit 0 : ordres retenables (envoi détourné) et correctifs POL actifs ; bit 1 : minuteur
/// détourné ; bit 2 : publication détournée (début du tour retenu avant l'interface).
#[no_mangle]
pub unsafe extern "C" fn BgdDepartDispo() -> u32 {
    pose_tardive();
    let dispo = ENVOI_TRAMP.load(Ordering::Relaxed) != 0 && pol::actifs();
    dispo as u32 | ((MINUTEUR_TRAMP.load(Ordering::Relaxed) != 0) as u32) << 1 | ((PUBLIER_SITE.load(Ordering::Relaxed) != 0) as u32) << 2
}

#[no_mangle]
pub extern "C" fn BgdDepartArmer(joueur: i32, execute_script: u32) {
    armer(joueur, execute_script);
}

#[no_mangle]
pub unsafe extern "C" fn BgdDepartAttendre(joueur: i32, execute_script: u32) {
    attendre(joueur, execute_script);
}

/// 1 si l'on attendait.
#[no_mangle]
pub unsafe extern "C" fn BgdDepartPartir(raison: *const c_char) -> u32 {
    let raison = if raison.is_null() { "départ".into() } else { CStr::from_ptr(raison).to_string_lossy().into_owned() };
    partir(&raison) as u32
}

/// `sortie[0..4]` : en attente (1|0), ordres retenus, secondes d'attente, activation retenue (1|0).
#[no_mangle]
pub unsafe extern "C" fn BgdDepartEtat(sortie: *mut f64) {
    if sortie.is_null() {
        return;
    }
    let on = ATTENTE.load(Ordering::Relaxed);
    let v = [on as u8 as f64, REJETS.load(Ordering::Relaxed) as f64, if on { attente_s() } else { 0.0 }, RETENU.load(Ordering::Relaxed) as u8 as f64];
    std::ptr::copy_nonoverlapping(v.as_ptr(), sortie, 4);
}

/// 1 si la fin du tour local est forcée, 0 si le minuteur n'a pas encore été vu.
#[no_mangle]
pub unsafe extern "C" fn BgdDepartFinForcer() -> u32 {
    fin_forcer() as u32
}

#[no_mangle]
pub unsafe extern "C" fn BgdDepartFinLever() {
    fin_lever();
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOSSIER: &str = "Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS";
    const CPU_ARM64: u32 = 0x0100_000c;
    const CPU_X86_64: u32 = 0x0100_0007;

    fn fichier(nom: &str) -> Option<Vec<u8>> {
        let p = std::path::Path::new(&std::env::var_os("HOME")?).join(DOSSIER).join(nom);
        let r = std::fs::read(&p).ok();
        if r.is_none() {
            eprintln!("absent, test sauté : {}", p.display());
        }
        r
    }

    /// Tranche `cpu` d'un binaire universel.
    fn tranche(b: &[u8], cpu: u32) -> Option<&[u8]> {
        let be = |o: usize| b.get(o..o + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap()));
        if be(0)? != 0xcafe_babe {
            return (u32::from_le_bytes(b.get(4..8)?.try_into().ok()?) == cpu).then_some(b);
        }
        (0..be(4)? as usize).find_map(|i| {
            let e = 8 + 20 * i;
            (be(e)? == cpu).then(|| b.get(be(e + 8)? as usize..(be(e + 8)? + be(e + 12)?) as usize)).flatten()
        })
    }

    /// Segments d'une tranche : (nom, vmaddr, vmsize, fileoff, filesize), et ses sections.
    struct Image<'a> {
        t: &'a [u8],
        segs: Vec<(String, u64, u64, u64, u64)>,
        sects: Vec<(String, String, u64, u64)>,
    }

    impl<'a> Image<'a> {
        fn new(t: &'a [u8]) -> Self {
            let u32_ = |o: usize| u32::from_le_bytes(t[o..o + 4].try_into().unwrap());
            let u64_ = |o: usize| u64::from_le_bytes(t[o..o + 8].try_into().unwrap());
            let nom = |o: usize| String::from_utf8_lossy(&t[o..o + 16]).trim_end_matches('\0').to_string();
            let (mut segs, mut sects, mut off) = (Vec::new(), Vec::new(), 32);
            for _ in 0..u32_(16) {
                let (cmd, size) = (u32_(off), u32_(off + 4) as usize);
                if cmd == 0x19 {
                    segs.push((nom(off + 8), u64_(off + 24), u64_(off + 32), u64_(off + 40), u64_(off + 48)));
                    for k in 0..u32_(off + 64) as usize {
                        let s = off + 72 + 80 * k;
                        sects.push((nom(s + 16), nom(s), u64_(s + 32), u64_(s + 40)));
                    }
                }
                off += size;
            }
            Image { t, segs, sects }
        }
        fn off(&self, a: usize) -> usize {
            let s = self.segs.iter().find(|s| (s.1..s.1 + s.4).contains(&(a as u64))).expect("adresse hors du fichier");
            (a as u64 - s.1 + s.3) as usize
        }
        fn octets(&self, a: usize, n: usize) -> &[u8] {
            let o = self.off(a);
            &self.t[o..o + n]
        }
        fn u32(&self, a: usize) -> u32 {
            u32::from_le_bytes(self.octets(a, 4).try_into().unwrap())
        }
        fn u64(&self, a: usize) -> u64 {
            u64::from_le_bytes(self.octets(a, 8).try_into().unwrap())
        }
        fn section(&self, a: usize) -> Option<(&str, &str)> {
            self.sects.iter().find(|s| (s.2..s.2 + s.3).contains(&(a as u64))).map(|s| (s.0.as_str(), s.1.as_str()))
        }
        fn texte(&self) -> (usize, &[u8]) {
            let s = self.sects.iter().find(|s| s.0 == "__TEXT" && s.1 == "__text").unwrap();
            (s.2 as usize, self.octets(s.2 as usize, s.3 as usize))
        }
    }

    fn empreinte(img: &Image, f: &Fonction) {
        assert_eq!(fnv(img.octets(f.adresse, f.taille)), f.empreinte, "{} : empreinte", f.nom);
    }

    /// Chaîne C entière (précédée et suivie d'un NUL) dans le fichier.
    fn chaine_presente(t: &[u8], s: &str) -> bool {
        let mut motif = vec![0u8];
        motif.extend_from_slice(s.as_bytes());
        motif.push(0);
        t.windows(motif.len()).any(|w| w == motif.as_slice())
    }

    #[test]
    fn regles() {
        let exec = 0xc665_ad08;
        for t in BLOQUES {
            assert!(bloque_regle(t, 99, 0, 1, exec));
        }
        assert!(!bloque_regle(0xe578_82af, 1, 0, 1, exec), "CivicsCommand passe");
        assert!(!bloque_regle(0x1bdb_4f5c, 1, 0, 1, exec), "GameTurnComplete passe");
        assert!(bloque_regle(PLAYER_OPERATION, 1, 0x1234, 1, exec));
        assert!(!bloque_regle(PLAYER_OPERATION, 1, exec, 1, exec), "EXECUTE_SCRIPT passe");
        assert!(!bloque_regle(PLAYER_OPERATION, 2, 0x1234, 1, exec), "opération d'un autre joueur");
        assert!(activation_regle(TYPE_DEBUT_LOCAL, -1, 3));
        assert!(activation_regle(TYPE_ACTIVE, 3, 3));
        assert!(!activation_regle(TYPE_ACTIVE, 4, 3));
        assert!(!activation_regle(1, 3, 3));
    }

    #[test]
    fn trampoline_x86() {
        let t = trampoline_appel_x86(&[0x55; 13], 0x1_0134_2da0, 0x1_0142_25f2);
        assert_eq!(t.len(), 13 + 8 + 8 + 6 + 8);
        // call [rip+2] : le disp32 vise l'adresse rangée après `jmp +8`.
        assert_eq!(&t[13..19], &[0xff, 0x15, 2, 0, 0, 0]);
        assert_eq!(&t[19..21], &[0xeb, 0x08]);
        assert_eq!(u64::from_le_bytes(t[13 + 6 + 2..13 + 16].try_into().unwrap()), 0x1_0134_2da0);
        // Le retour de l'appel tombe sur `jmp +8`, qui saute l'adresse jusqu'au `jmp [rip+0]`.
        assert_eq!(&t[29..35], &[0xff, 0x25, 0, 0, 0, 0]);
        assert_eq!(u64::from_le_bytes(t[35..43].try_into().unwrap()), 0x1_0142_25f2);
    }

    #[test]
    fn gamecore_arm64() {
        let Some(b) = fichier("GameCore_XP2.dll") else { return };
        let t = tranche(&b, CPU_ARM64).expect("tranche arm64");
        let f = Fonction { nom: "envoi", adresse: 0x8481ac, taille: 0x2d0, empreinte: 0x00f1_3526_e1f7_8977 };
        assert_eq!(fnv(&t[f.adresse..f.adresse + f.taille]), f.empreinte);
        assert_eq!(&t[f.adresse..f.adresse + 16], &[0xf8, 0x5f, 0xbc, 0xa9, 0xf6, 0x57, 0x01, 0xa9, 0xf4, 0x4f, 0x02, 0xa9, 0xfd, 0x7b, 0x03, 0xa9]);
        // Les 19 appelants directs (bl).
        let img = Image::new(t);
        let (debut, text) = img.texte();
        let n = (0..text.len() / 4)
            .filter(|i| {
                let mot = u32::from_le_bytes(text[i * 4..i * 4 + 4].try_into().unwrap());
                mot >> 26 == 0b100101 && patch::cible_branche(debut + i * 4, mot) == Some(f.adresse)
            })
            .count();
        assert_eq!(n, 19, "appelants de Send");
        // La chaîne du journal de Send.
        assert!(chaine_presente(t, "GameCore SEND: %s\r\n"));
    }

    #[test]
    fn gamecore_x86_64() {
        let Some(b) = fichier("GameCore_XP2.dll") else { return };
        let t = tranche(&b, CPU_X86_64).expect("tranche x86_64");
        assert_eq!(fnv(&t[0x826570..0x826570 + 0x330]), 0x3f44_a28a_db83_ffe5);
        assert_eq!(&t[0x826570..0x826570 + 20], &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec, 0x88, 0x06, 0x00, 0x00]);
        let img = Image::new(t);
        let (debut, text) = img.texte();
        let n = (0..text.len().saturating_sub(5))
            .filter(|&i| text[i] == 0xe8 && patch::cible_rel32_x86(debut + i, &text[i..i + 5]) == Some(0x826570))
            .count();
        assert!(n >= 19, "appelants de Send : {n}");
    }

    /// Branchement arm64 `b`/`bl` en `a` : sa cible.
    fn bl(img: &Image, a: usize) -> Option<usize> {
        patch::cible_branche(a, img.u32(a))
    }

    /// `adrp` + `add` arm64 en `a` : l'adresse formée.
    fn adrp_add(img: &Image, a: usize) -> usize {
        let (m1, m2) = (img.u32(a), img.u32(a + 4));
        assert_eq!(m1 & 0x9f00_0000, 0x9000_0000, "adrp en {a:#x}");
        assert_eq!(m2 & 0xffc0_0000, 0x9100_0000, "add en {:#x}", a + 4);
        let imm = (((m1 >> 5) & 0x7_ffff) << 2 | (m1 >> 29) & 3) as i64;
        let imm = (imm << 43) >> 43;
        let page = (a as i64 & !0xfff) + (imm << 12);
        (page + ((m2 >> 10) & 0xfff) as i64) as usize
    }

    #[test]
    fn exe_arm64() {
        let Some(b) = fichier("Civ6_Exe_Child") else { return };
        let img = Image::new(tranche(&b, CPU_ARM64).expect("tranche arm64"));
        let a = |f: &Fonction| (f.adresse, f.taille, f.empreinte);
        let fs = [
            Fonction { nom: "minuteur", adresse: 0x1_0137_cf24, taille: 0x10c, empreinte: 0x1098_f180_9e8c_1520 },
            Fonction { nom: "publication", adresse: 0x1_011e_d270, taille: 0x2dc, empreinte: 0x8ecf_c8de_92a9_3d10 },
            Fonction { nom: "tasser", adresse: 0x1_011f_35f0, taille: 0x114, empreinte: 0x26b0_24fc_b327_fee6 },
            Fonction { nom: "transférer", adresse: 0x1_011f_3704, taille: 0x1c0, empreinte: 0x8f38_2b77_3242_31da },
            Fonction { nom: "données", adresse: 0x1_00b0_5af0, taille: 0xc4, empreinte: 0xdece_16fc_c104_2886 },
            Fonction { nom: "operator()", adresse: 0x1_011f_4048, taille: 0x1c, empreinte: 0x8dfd_0b08_96d1_f554 },
            Fonction { nom: "destroy()", adresse: 0x1_011f_4040, taille: 4, empreinte: 0x2fe4_2bc0_3800_fb45 },
        ];
        for f in &fs {
            empreinte(&img, f);
        }
        #[cfg(target_arch = "aarch64")]
        for (f, g) in [&MINUTEUR, &PUBLIER, &TASSER, &TRANSFERER, &DONNEES, &OPERATEUR, &DETRUIRE].into_iter().zip(&fs) {
            assert_eq!(a(f), a(g), "{}", f.nom);
        }
        let _ = a;
        let (pause, publier) = (0x1_0137_cf24, 0x1_011e_d270);
        // Minuteur : prologue sans adresse relative, `bl` du jeu en +0x14, compteur en +0x5c.
        assert_eq!(img.octets(pause, 16), &[0xf6, 0x57, 0xbd, 0xa9, 0xf4, 0x4f, 0x01, 0xa9, 0xfd, 0x7b, 0x02, 0xa9, 0xfd, 0x83, 0x00, 0x91]);
        assert_eq!(bl(&img, pause + 0x14), Some(0x1_012a_cd1c));
        assert_eq!(bl(&img, pause + 0x5c), Some(0x1_008c_ccc8), "Platform::QueryPlatformCounter");
        assert!(chaine_presente(img.t, "__ZN8Platform20QueryPlatformCounterEv"));
        // Publication : appels relevés et table du foncteur.
        assert_eq!(bl(&img, publier + 0x90), Some(0x1_011f_35f0));
        assert_eq!(bl(&img, publier + 0x98), Some(0x1_011f_3704));
        for o in [0xd8, 0x124, 0x1ac] {
            assert_eq!(bl(&img, publier + o), Some(0x1_00b0_5af0), "données en +{o:#x}");
        }
        assert_eq!(adrp_add(&img, publier + 0xf8), 0x1_01b1_b060);
        assert_eq!(img.u64(0x1_01b1_b060 + VT_DETRUIRE) as usize, 0x1_011f_4040);
        assert_eq!(img.u64(0x1_01b1_b060 + VT_OPERATEUR) as usize, 0x1_011f_4048);
        // Pas d'adresse relative dans les 16 premiers octets (sub sp + 3 stp).
        assert_eq!(img.octets(publier, 4), &[0xff, 0x83, 0x02, 0xd1]);
        // Table des noms : remplie par l'initialiseur statique (adrp/add puis str), dans __bss.
        assert_eq!(adrp_add(&img, 0x1_0120_198c), 0x1_0f09_9268);
        assert!(matches!(img.section(0x1_0f09_9268), Some(("__DATA", "__bss" | "__common"))));
        for (nom, _) in NOMS_TYPES {
            assert!(chaine_presente(img.t, nom), "{nom}");
        }
    }

    #[test]
    fn exe_x86_64() {
        let Some(b) = fichier("Civ6_Exe_Child") else { return };
        let img = Image::new(tranche(&b, CPU_X86_64).expect("tranche x86_64"));
        let fs = [
            Fonction { nom: "minuteur", adresse: 0x1_0142_25e0, taille: 0xe9, empreinte: 0xaf8a_bd2f_b0e6_379b },
            Fonction { nom: "publication", adresse: 0x1_0128_6e00, taille: 0x330, empreinte: 0xb304_0d13_032f_5a80 },
            Fonction { nom: "tasser", adresse: 0x1_0128_dc80, taille: 0x120, empreinte: 0xb998_8f41_1c2a_3697 },
            Fonction { nom: "transférer", adresse: 0x1_0128_dda0, taille: 0x1f0, empreinte: 0x9a0e_d9e9_b38b_a582 },
            Fonction { nom: "données", adresse: 0x1_00e2_0080, taille: 0xc0, empreinte: 0xcf03_afc9_263e_15bf },
            Fonction { nom: "operator()", adresse: 0x1_0128_e670, taille: 0x20, empreinte: 0xf843_b151_d010_81e8 },
            Fonction { nom: "destroy()", adresse: 0x1_0128_e650, taille: 0x10, empreinte: 0x83df_ea89_27c0_8670 },
        ];
        for f in &fs {
            empreinte(&img, f);
        }
        #[cfg(target_arch = "x86_64")]
        for (f, g) in [&MINUTEUR, &PUBLIER, &TASSER, &TRANSFERER, &DONNEES, &OPERATEUR, &DETRUIRE].into_iter().zip(&fs) {
            assert_eq!((f.adresse, f.taille, f.empreinte), (g.adresse, g.taille, g.empreinte), "{}", f.nom);
        }
        let (pause, publier) = (0x1_0142_25e0usize, 0x1_0128_6e00usize);
        let rel = |a: usize| patch::cible_rel32_x86(a, img.octets(a, 5));
        // Minuteur : 13 octets de prologue, puis `call rel32` (reprise en +18) ; compteur en +0x50.
        assert_eq!(img.octets(pause, 13), &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x53, 0x50, 0x48, 0x89, 0xfb]);
        assert_eq!(img.octets(pause + 13, 1), &[0xe8]);
        assert_eq!(rel(pause + 13), Some(0x1_0134_2da0));
        assert_eq!(rel(pause + 0x50), Some(0x1_0096_edf0), "Platform::QueryPlatformCounter");
        // Aucun call/jmp rel32 de l'exe ne vise l'intérieur des 18 octets remplacés (balayage de
        // tous les octets : prudent, des faux positifs feraient échouer le test).
        let (debut, text) = img.texte();
        let dedans = (0..text.len().saturating_sub(5))
            .filter(|&i| matches!(text[i], 0xe8 | 0xe9))
            .filter_map(|i| patch::cible_rel32_x86(debut + i, &text[i..i + 5]).map(|c| (debut + i, c)))
            .filter(|&(_, c)| c > pause && c < pause + 18)
            .collect::<Vec<_>>();
        assert!(dedans.is_empty(), "branchements vers le prologue du minuteur : {dedans:x?}");
        // Branchements courts internes (jcc/jmp rel8 et jcc rel32) : tous après le prologue.
        for i in 0..0xe9 - 1 {
            let a = pause + i;
            let o = img.octets(a, 6);
            let cible = match o[0] {
                0x70..=0x7f | 0xeb => Some((a as i64 + 2 + o[1] as i8 as i64) as usize),
                0x0f if (0x80..=0x8f).contains(&o[1]) => Some((a as i64 + 6 + i32::from_le_bytes(o[2..6].try_into().unwrap()) as i64) as usize),
                _ => None,
            };
            if let Some(c) = cible {
                assert!(!(c > pause && c < pause + 18), "branchement en {a:#x} vers {c:#x}");
            }
        }
        // Publication : 17 octets de prologue sans adresse relative (le saut en prend 14).
        assert_eq!(img.octets(publier, 17), &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x83, 0xec, 0x48]);
        assert_eq!(rel(publier + 0x7b), Some(0x1_0128_dc80));
        assert_eq!(rel(publier + 0x83), Some(0x1_0128_dda0));
        for o in [0xd2, 0x146, 0x1f7] {
            assert_eq!(rel(publier + o), Some(0x1_00e2_0080), "données en +{o:#x}");
        }
        // lea rcx,[rip+…] de la table du foncteur.
        let lea = publier + 0x100;
        assert_eq!(img.octets(lea, 3), &[0x48, 0x8d, 0x0d]);
        assert_eq!((lea as i64 + 7 + i32::from_le_bytes(img.octets(lea + 3, 4).try_into().unwrap()) as i64) as usize, 0x1_01c5_5f88);
        assert_eq!(img.u64(0x1_01c5_5f88 + VT_DETRUIRE) as usize, 0x1_0128_e650);
        assert_eq!(img.u64(0x1_01c5_5f88 + VT_OPERATEUR) as usize, 0x1_0128_e670);
        // Table des noms : `mov [rip+…], r12` de l'initialiseur pour les types 1, 8 et 15.
        for (site, t) in [(0x1_0129_d085usize, 1usize), (0x1_0129_d539, 8), (0x1_0129_d9ed, 15)] {
            assert_eq!(img.octets(site, 3), &[0x4c, 0x89, 0x25], "store en {site:#x}");
            let cible = (site as i64 + 7 + i32::from_le_bytes(img.octets(site + 3, 4).try_into().unwrap()) as i64) as usize;
            assert_eq!(cible, 0x1_0298_2268 + (t - 1) * 8, "type {t}");
        }
        assert!(matches!(img.section(0x1_0298_2268), Some(("__DATA", "__bss" | "__common"))));
        for (nom, _) in NOMS_TYPES {
            assert!(chaine_presente(img.t, nom), "{nom}");
        }
    }
    /// x86_64 (Rosetta ici) : le trampoline du minuteur exécuté pour de vrai, sur une fonction
    /// factice au même prologue (13 octets, `call rel32`, suite en +18).
    #[cfg(target_arch = "x86_64")]
    #[test]
    fn trampoline_x86_execute() {
        extern "C" fn appel(this: usize) -> usize {
            // Pile alignée sur 16 à l'entrée d'une fonction (après l'adresse de retour : 8).
            let marque = 0u64;
            let sp = &marque as *const u64 as usize;
            assert_eq!(sp % 8, 0);
            this * 3 + 1
        }
        let mut f = vec![0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x53, 0x50, 0x48, 0x89, 0xfb];
        f.extend_from_slice(&[0xe8, 0xcc, 0xcc, 0xcc, 0xcc]); // jamais exécuté
        // add rsp,8 ; pop rbx ; pop r14 ; pop r15 ; pop rbp ; ret
        f.extend_from_slice(&[0x48, 0x83, 0xc4, 0x08, 0x5b, 0x41, 0x5e, 0x41, 0x5f, 0x5d, 0xc3]);
        unsafe {
            let fausse = patch::executable_near(&f, appel as *const () as usize, 1 << 40).unwrap();
            let t = trampoline_appel_x86(&f[..13], appel as *const () as usize, fausse + 18);
            let tramp = patch::executable_near(&t, fausse, 1 << 40).unwrap();
            let g: extern "C" fn(usize) -> usize = std::mem::transmute(tramp);
            assert_eq!(g(14), 43);
        }
    }

    // --- Simulation en mémoire : publication, retenue, reprise, minuteur, relais nus. ---------

    use std::sync::Mutex;

    static VUS: Mutex<Vec<(u32, i32)>> = Mutex::new(Vec::new());
    static OBSERVES: Mutex<Vec<(u32, u32, usize)>> = Mutex::new(Vec::new());
    static TASSEMENTS: AtomicU32 = AtomicU32::new(0);
    static DETRUITS: AtomicU32 = AtomicU32::new(0);
    static ORIGINAL_MINUTEUR: AtomicU32 = AtomicU32::new(0);

    extern "C" fn faux_tasser(_q: usize) {
        TASSEMENTS.fetch_add(1, Ordering::Relaxed);
    }
    extern "C" fn faux_transferer(_q: usize) {}
    unsafe extern "C" fn faux_donnees(tampon: usize) -> usize {
        rd(tampon)
    }
    unsafe extern "C" fn fausse_distribution(donnees: usize, _entree: usize) {
        let genre: u32 = rd(donnees - E_DONNEES + E_TYPE);
        VUS.lock().unwrap().push((genre, rd(donnees + ACTIVE_JOUEUR)));
    }
    unsafe extern "C" fn faux_operateur(this: *const usize, a: *const u32, b: *const u32) {
        OBSERVES.lock().unwrap().push((*a, *b, *this.add(1)));
    }
    extern "C" fn faux_detruire(_this: *const usize) {
        DETRUITS.fetch_add(1, Ordering::Relaxed);
    }
    extern "C" fn faux_minuteur(this: usize) {
        ORIGINAL_MINUTEUR.fetch_add(1, Ordering::Relaxed);
        // Comme l'original à la reprise : chrono 1 relancé.
        unsafe { wr(this + 0x28, 1u8) };
    }
    extern "C" fn suite_test(a: u64, b: u64, c: u64, d: u64, e: u64, f: u64, g: u64, h: u64, x: f64, y: f64) -> u64 {
        a + 2 * b + 3 * c + 4 * d + 5 * e + 6 * f + 7 * g + 8 * h + (x * 100.0) as u64 + (y * 1000.0) as u64
    }

    #[test]
    fn simulation() {
        let journal = std::env::temp_dir().join("bge_depart_test");
        std::env::set_var("BGE_DYLIBS", journal.join("dylibs"));
        // Publieur, jeu 0 : file en +0x20, table des types en +0xb0, garde en +0x48c0.
        let mut p_mem = vec![0u64; 0x5000 / 8];
        let p = p_mem.as_mut_ptr() as usize;
        let q = p + 0x20;
        let table = p + 0xb0;
        let mut fvt = vec![0usize; 8];
        fvt[VT_DETRUIRE / 8] = faux_detruire as *const () as usize;
        fvt[VT_OPERATEUR / 8] = faux_operateur as *const () as usize;
        // Quatre enregistrements de 0x60 octets : type 1, 15 (joueur 2), 8, 15 (joueur 3).
        let mut recs = vec![0u64; 4 * 0x60 / 8];
        let base = recs.as_mut_ptr() as usize;
        unsafe {
            for (i, (genre, joueur)) in [(1u32, 0i32), (15, 2), (8, 0), (15, 3)].into_iter().enumerate() {
                let r = base + i * 0x60;
                wr(r, 0x60u32);
                wr(r + E_TYPE, genre);
                wr(r + 8, 100 + i as u32);
                wr(r + 0xc, 200 + i as u32);
                wr(r + E_DONNEES + ACTIVE_JOUEUR, joueur);
                wr(table + genre as usize * 0x30 + 0x20, fausse_distribution as *const () as usize);
            }
            // Bascule 1 : tampon lu 0, en q+0 (adresse, capacité, taille).
            wr(q + F_BASCULE, 1u32);
            wr(q, base);
            wr(q + 0x10, 4 * 0x60u32);
            wr(q + F_OBSERVATEUR, 1usize);
            TASSER_F.store(faux_tasser as *const () as usize, Ordering::Relaxed);
            TRANSFERER_F.store(faux_transferer as *const () as usize, Ordering::Relaxed);
            DONNEES_F.store(faux_donnees as *const () as usize, Ordering::Relaxed);
            VTABLE.store(fvt.as_ptr() as usize, Ordering::Relaxed);
            PUBLIER_SITE.store(1, Ordering::Relaxed);

            // Armée pour le joueur 3 : la publication s'arrête devant le type 8.
            reset();
            armer(3, 0xc665_ad08);
            hook_publier(p);
            assert_eq!(*VUS.lock().unwrap(), vec![(1, 0), (15, 2)]);
            assert!(RETENU.load(Ordering::Relaxed) && ATTENTE.load(Ordering::Relaxed) && !ARME.load(Ordering::Relaxed));
            assert_eq!(rd::<u32>(q + F_LU), 2 * 0x60, "décalage de l'enregistrement retenu");
            assert_eq!(rd::<u32>(p + P_GARDE), 0, "garde rendue");
            assert_eq!(rd::<u32>(q + F_MAX), 4 * 0x60);
            assert_eq!(rd::<usize>(p + P_REC10), 0);
            assert_eq!(rd::<u32>(p + P_REC8), 101, "dernier enregistrement distribué");
            assert_eq!(*OBSERVES.lock().unwrap(), vec![(100, 200, q), (101, 201, q)], "observateur avant chaque distribution");
            assert_eq!(DETRUITS.load(Ordering::Relaxed), 1);
            // Retenue : rien ne part.
            hook_publier(p);
            assert_eq!(VUS.lock().unwrap().len(), 2);
            // Ordre retenu pendant l'attente, CivicsCommand non.
            let mut msg = vec![0u64; 8];
            let m = msg.as_mut_ptr() as usize;
            wr(m + M_TYPE, UNIT_COMMAND);
            assert!(bloque(m));
            wr(m + M_TYPE, 0xe578_82afu32);
            assert!(!bloque(m));
            // Départ : la publication reprend aux retenus, sans tasser d'abord.
            let tassements = TASSEMENTS.load(Ordering::Relaxed);
            assert!(partir("test"));
            assert!(!ATTENTE.load(Ordering::Relaxed) && REPRISE.load(Ordering::Relaxed));
            hook_publier(p);
            assert_eq!(*VUS.lock().unwrap(), vec![(1, 0), (15, 2), (8, 0), (15, 3)]);
            assert_eq!(TASSEMENTS.load(Ordering::Relaxed), tassements + 1, "pas de tassement à la reprise, un à la passe suivante");
            assert_eq!(rd::<u32>(q + F_LU), 4 * 0x60, "tout distribué");
            assert!(!RETENU.load(Ordering::Relaxed) && !REPRISE.load(Ordering::Relaxed));

            // Garde déjà tenue (réentrée) : rien.
            wr(p + P_GARDE, 1u32);
            wr(q + F_LU, 0u32);
            hook_publier(p);
            assert_eq!(VUS.lock().unwrap().len(), 4);
            wr(p + P_GARDE, 0u32);

            // Minuteur : gelé à zéro pendant l'attente, relancé au départ ; fin forcée.
            let mut gestionnaire = vec![0u64; 0x50 / 8];
            let g = gestionnaire.as_mut_ptr() as usize;
            MINUTEUR_TRAMP.store(faux_minuteur as *const () as usize, Ordering::Relaxed);
            wr(g + 0x20, 12.5f64);
            wr(g + 0x28, 1u8);
            attendre(3, 0);
            hook_minuteur(g);
            assert_eq!((rd::<f64>(g + 0x20), rd::<u8>(g + 0x28), ORIGINAL_MINUTEUR.load(Ordering::Relaxed)), (0.0, 0, 0));
            assert!(partir("test"));
            assert_eq!((rd::<u8>(g + 0x28), ORIGINAL_MINUTEUR.load(Ordering::Relaxed)), (1, 1));
            assert!(fin_forcer());
            assert!(rd::<f64>(g + 0x20) >= FORCE_S && rd::<f64>(g + 0x38) >= FORCE_S);
            fin_lever();
            assert_eq!((rd::<f64>(g + 0x20), ORIGINAL_MINUTEUR.load(Ordering::Relaxed)), (0.0, 2));
            // Nouvelle partie : le gestionnaire vu n'est plus valable.
            reset();
            assert_eq!(MINUTEUR_THIS.load(Ordering::Relaxed), 0);
            MINUTEUR_TRAMP.store(0, Ordering::Relaxed);
            PUBLIER_SITE.store(0, Ordering::Relaxed);

            // Relais nu de DllDestroyGameContext : registres d'argument rendus, reset appelé.
            SUITE_DESTRUCTION.store(suite_test as *const () as usize, Ordering::Relaxed);
            ATTENTE.store(true, Ordering::Relaxed);
            MINUTEUR_THIS.store(g, Ordering::Relaxed);
            let relais: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64, f64, f64) -> u64 =
                std::mem::transmute(relais_destruction as *const ());
            assert_eq!(relais(1, 2, 3, 4, 5, 6, 7, 8, 0.25, 0.5), suite_test(1, 2, 3, 4, 5, 6, 7, 8, 0.25, 0.5));
            assert!(!ATTENTE.load(Ordering::Relaxed) && MINUTEUR_THIS.load(Ordering::Relaxed) == 0, "game_destroyed appelé avant la suite");
            SUITE_DESTRUCTION.store(0, Ordering::Relaxed);
        }
        drop((p_mem, recs, fvt));
    }
}
