//! Garde des messages réseau : bornes sur les comptes lus par le désérialiseur de `Data_Variant.cpp`.
//!
//! Le gel du 9 novembre 2026 : à la déconnexion d'un joueur, pendant le vidage de la file des
//! messages entrants, un message
//! compressé est arrivé abîmé (zlib le refuse). Le jeu ignore l'erreur de décompression et
//! désérialise la suite : `TypedVariantMap::Deserialize` lit alors un compte d'entrées sur 4 octets
//! (`0x61746143`, « Cata » lu comme un entier) et boucle dessus sans aucune borne ni contrôle de
//! lecture, en allouant à chaque tour. Le jeu gèle, puis manque de mémoire dix minutes plus tard.
//!
//! Sept lectures de compte ou de longueur ne sont pas bornées, six dans ce désérialiseur et une
//! dans la lecture de `NetGameConfig` :
//! - `DeserializeTypedVariantMap` (`1407a1e20`) : compte des paires clé/valeur, compte des entrées
//!   (la boucle du gel) ;
//! - `DeserializeNamedVariantMap` (`1407adf50`, la racine des messages `NetPlayerInfo` et
//!   `NetGameConfig`) : compte, ou mot de drapeaux négatif suivi du compte ;
//! - `DeserializeVariantList` (`1407ad890`) : compte ;
//! - `DeserializeDataVariant` (`1407a3450`) : longueur, allouée élément × longueur pour un tampon
//!   brut (bit 24 des drapeaux) ;
//! - `140266140`, lecture des clés changées de `NetGameConfig` : compte, avec une recherche linéaire
//!   à chaque tour.
//!
//! Toutes passent par la lecture de champ du sérialiseur, `ReadSerializerField` (`14099f9d0`,
//! vtable +0x30 du lecteur). On la détourne : après la lecture d'origine, si l'adresse de retour est
//! l'un des sept sites et que la valeur lue est impossible (plus de 100 000 entrées, ou un tampon de
//! plus de 16 Mo), elle est remplacée par 0. La boucle ne tourne pas, le message est consommé vide,
//! la partie continue. Une valeur plausible n'est jamais touchée. En jeu, le joueur le dit à tous
//! dans le chat (`partie.lua`, `Civ6Proxy_GardeMessages()`).
//!
//! Vider ne suffit pas pour `NetGameConfig` et `NetPlayerInfo` : leur traitement efface la
//! configuration (de la partie, d'un joueur) avant d'y recopier la carte reçue, qui l'effacerait
//! donc. Leur vtable est détournée : un message dont la lecture a demandé une correction n'est pas
//! appliqué, la configuration reste celle d'avant. Le traitement de `NetPlayerInfo` ne borne pas
//! l'identifiant du joueur (seulement -1) : un identifiant hors des emplacements n'est pas appliqué
//! non plus (voir `MESSAGES`).
//!
//! A failed read leaves its destination untouched, so at the seven sites the destination is set
//! before the read: no leftover count. A message whose root map count cannot be read is damaged. A
//! `NetGameConfig` with no changed key and an empty map would replace the whole game configuration
//! with nothing (every player's civ, leader and level): it is never applied. Such messages come from
//! the game's queue flush after a load, when a chunked snapshot is still arriving.
//!
//! Names: Ghidra project `civ6-netcode`, bookmarks "Gel 2026-11-09" (DX12 addresses above). The
//! same rules, installed by an external hook during development, ran over two whole ten-player
//! games (162 turns, 108,000 messages received) without a false alarm.
//! Addresses of both exes of 2026-11-05 (`EXES`): `CivilizationVI_DX12.exe` and
//! `CivilizationVI.exe` (DX11). DX11 is the same code at other addresses: same functions, same
//! size, byte for byte except relative displacements (calls, rip-relative addressing). At startup
//! the loaded exe is checked byte by byte against each table; on another exe (another version),
//! nothing is installed and the log says so. Without the DX11 table, a player on DX11 had no guard:
//! that player froze, then crashed, on 2026-11-10 (20:09), while the DX12 player next to them was
//! protected.
//! Réglages : `actif=off` (l'interrupteur général) ne pose rien, comme pour le reste du mod ;
//! c'est un correctif de bug, sans réglage en release. Dans les builds de diagnostic (feature
//! `diag`) seulement, `garde_messages` dans `rollover_proxy.cfg` (onglet, section Diagnostic) :
//! `off` désactive la garde seule ; `test` injecte une fois le compte du gel à la première lecture
//! « carte : entrées », pour voir la garde agir dans une partie jouée seul (le journal montre
//! l'injection, puis la correction). Le mode `test` abîme un vrai message : jamais en release.

use crate::{log, patch, win};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// Base des adresses Ghidra de l'exe.
const BASE: u64 = 0x1_4000_0000;

/// Prologue of `ReadSerializerField(reader, descriptor, destination, bytes)` (`Exe::lecture`):
/// `mov [rsp+8],rbx ; push rdi ; sub rsp,70h ; mov rax,r8 ; mov rbx,rdx`, 16 bytes without
/// relative addressing, copied into the trampoline. The same in both exes.
const PROLOGUE: [u8; 16] = [0x48, 0x89, 0x5c, 0x24, 0x08, 0x57, 0x48, 0x83, 0xec, 0x70, 0x49, 0x8b, 0xc0, 0x48, 0x8b, 0xda];

/// Plus d'entrées qu'aucune carte ou liste d'un message n'en a (quelques dizaines en pratique).
pub const MAX_ENTREES: u32 = 100_000;
/// Tampon brut d'un variant (chaîne, tableau d'octets) : un message fait quelques Ko.
pub const MAX_TAMPON: u64 = 16 << 20;
/// Scalaire d'un variant : élément × longueur, 8 octets en pratique.
pub const MAX_SCALAIRE: u64 = 4096;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Regle {
    /// Un nombre d'entrées.
    Compte,
    /// Un nombre d'entrées, ou, négatif, le mot de drapeaux du format nommé (laissé tel quel).
    CompteOuDrapeaux,
    /// La longueur d'un variant. Ses drapeaux sont lus avant elle, et rangés juste après elle en
    /// mémoire : à destination + 4 (`DeserializeDataVariant`, drapeaux en variant + 0xC, longueur
    /// en variant + 8 ; vérifié dans Ghidra le 2026-11-10).
    Longueur,
}

/// The seven reads, in the order of `Exe::retours`.
const SITES: [(&str, Regle); 7] = [
    ("carte : paires", Regle::Compte),
    ("carte : entrées", Regle::Compte),
    ("carte nommée : compte", Regle::CompteOuDrapeaux),
    ("carte nommée : compte après drapeaux", Regle::Compte),
    ("liste : compte", Regle::Compte),
    ("variant : longueur", Regle::Longueur),
    ("configuration : clés changées", Regle::Compte),
];
/// The read where test mode injects the freeze count: "carte : entrées".
const SITE_INJECTION: usize = 1;
/// The root map count of `NetGameConfig` and `NetPlayerInfo`: "carte nommée : compte".
const SITE_RACINE: usize = 2;
/// Written before that read; still there after it, the read failed. Not a possible count.
const NON_LU: u32 = 0x7fff_fffe;
/// `call qword ptr [rax+30h]`, juste avant chaque adresse de retour.
const APPEL: [u8; 3] = [0xff, 0x50, 0x30];

/// The functions that contain these reads, in the order of `Exe::fonctions`, and their first bytes,
/// the same in both exes.
const FONCTIONS: [(&str, [u8; 12]); 5] = [
    ("DeserializeTypedVariantMap", [0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x48]),
    ("DeserializeDataVariant", [0x4c, 0x8b, 0xdc, 0x49, 0x89, 0x5b, 0x10, 0x49, 0x89, 0x73, 0x18, 0x57]),
    ("DeserializeNamedVariantMap", [0x48, 0x8b, 0xc4, 0x55, 0x48, 0x8d, 0xa8, 0x38, 0xfe, 0xff, 0xff, 0x48]),
    ("DeserializeVariantList", [0x48, 0x89, 0x5c, 0x24, 0x18, 0x48, 0x89, 0x6c, 0x24, 0x20, 0x56, 0x41]),
    ("NetGameConfig : clés changées", [0x48, 0x89, 0x5c, 0x24, 0x18, 0x48, 0x89, 0x74, 0x24, 0x20, 0x55, 0x57]),
];

/// The messages whose handler clears a configuration before copying the received map into it (map
/// entry `+0x178`, `1407a6660`, fourth argument 0: cleared through `+0x160`), in the order of
/// `Exe::vtables`.
/// - `NetGameConfig`, handler `140266e00`: `1401033f0(configuration, map, changed keys)`. With no
///   changed key, the whole game configuration is cleared, then rebuilt.
/// - `NetPlayerInfo`, handler `140267330`: `140116630(slots + id × 0x40, map, null)`, the player's
///   configuration cleared, then copied. Player ID at message + 0x38.
const MESSAGES: [&str; 2] = ["NetGameConfig", "NetPlayerInfo"];
/// Vtable entries: destructor, text, write, read, process.
const LIRE: usize = 3;
const TRAITER: usize = 4;
/// The configuration's player slots, 0x40 bytes each, run from `[+0x2e0]` to `[+0x2e8]`.
const EMPLACEMENTS: usize = 0x2e0;
/// `NetGameConfig`: number of changed keys (0: the whole configuration is replaced), received map.
const CONFIG_CHANGED_KEYS: usize = 0x458;
const CONFIG_MAP: usize = 0x468;
/// Map: end marker, first entry (equal when the map is empty).
const MAP_END: usize = 0x30;
const MAP_FIRST: usize = 0x38;

/// Everything that depends on the exe: Ghidra addresses (base `0x140000000`) and bytes that contain
/// a relative displacement.
struct Exe {
    nom: &'static str,
    /// `ReadSerializerField`, the hooked function (prologue `PROLOGUE`).
    lecture: u64,
    /// Return address of `call qword ptr [rax+30h]` for each read of `SITES`.
    retours: [u64; 7],
    /// Start of each function of `FONCTIONS`.
    fonctions: [u64; 5],
    /// For each message of `MESSAGES`: its vtable (`Civ6NetMessage.cpp`), set by its constructors,
    /// and its five entries.
    vtables: [(u64, [u64; 5]); 2],
    /// The configuration singleton.
    configuration: u64,
    /// The function that reads it: `sub rsp,38h ; mov rax,[rip+…]`, the displacement leading to
    /// `configuration`.
    singleton: (u64, [u8; 11]),
    /// Start of the `NetPlayerInfo` handler, up to the `slots + id × 0x40` computation (`cmp
    /// [rcx+38h],-1` … `call singleton` … `add rcx,[rax+2e0h]`).
    traitement_joueur: (u64, [u8; 0x35]),
}

/// Both exes of 2026-11-05. DX11 was found function by function: the same bytes as DX12 except
/// relative displacements, the same size (`.pdata` table), and the same context around each read.
const EXES: [Exe; 2] = [
    Exe {
        nom: "DX12",
        lecture: 0x1_4099_f9d0,
        retours: [0x1_407a_1f44, 0x1_407a_209f, 0x1_407a_dfe3, 0x1_407a_e03a, 0x1_407a_d901, 0x1_407a_34f4, 0x1_4026_619b],
        fonctions: [0x1_407a_1e20, 0x1_407a_3450, 0x1_407a_df50, 0x1_407a_d890, 0x1_4026_6140],
        vtables: [
            (0x1_40fe_0020, [0x1_4026_69c0, 0x1_4026_6ca0, 0x1_4026_7680, 0x1_4026_79d0, 0x1_4026_6e00]),
            (0x1_40fe_0048, [0x1_4026_6aa0, 0x1_4026_6da0, 0x1_4026_7810, 0x1_4026_7c40, 0x1_4026_7330]),
        ],
        configuration: 0x1_4231_00a0,
        singleton: (0x1_4010_08b0, [0x48, 0x83, 0xec, 0x38, 0x48, 0x8b, 0x05, 0xe5, 0xf7, 0x20, 0x02]),
        traitement_joueur: (
            0x1_4026_7340,
            [
                0x83, 0x79, 0x38, 0xff, 0x48, 0x8b, 0xf2, 0x48, 0x8b, 0xd9, 0x0f, 0x84, 0x62, 0x02, 0x00, 0x00, 0x48, 0x89,
                0xbc, 0x24, 0x90, 0x20, 0x00, 0x00, 0xe8, 0x53, 0x95, 0xe9, 0xff, 0x8b, 0x4b, 0x38, 0x48, 0x8d, 0x53, 0x40,
                0x48, 0xc1, 0xe1, 0x06, 0x45, 0x33, 0xc0, 0x48, 0x8b, 0xf8, 0x48, 0x03, 0x88, 0xe0, 0x02, 0x00, 0x00,
            ],
        ),
    },
    Exe {
        nom: "DX11",
        lecture: 0x1_4099_2530,
        retours: [0x1_4079_fc34, 0x1_4079_fd8f, 0x1_407a_be73, 0x1_407a_beca, 0x1_407a_b791, 0x1_407a_12b4, 0x1_4026_466b],
        fonctions: [0x1_4079_fb10, 0x1_407a_1210, 0x1_407a_bde0, 0x1_407a_b720, 0x1_4026_4610],
        vtables: [
            (0x1_40fd_0cc0, [0x1_4026_4e90, 0x1_4026_5170, 0x1_4026_5b40, 0x1_4026_5e90, 0x1_4026_52d0]),
            (0x1_40fd_0ce8, [0x1_4026_4f70, 0x1_4026_5270, 0x1_4026_5cd0, 0x1_4026_6100, 0x1_4026_5800]),
        ],
        configuration: 0x1_422d_8a20,
        singleton: (0x1_400f_fa30, [0x48, 0x83, 0xec, 0x38, 0x48, 0x8b, 0x05, 0xe5, 0x8f, 0x1d, 0x02]),
        traitement_joueur: (
            0x1_4026_5810,
            [
                0x83, 0x79, 0x38, 0xff, 0x48, 0x8b, 0xf2, 0x48, 0x8b, 0xd9, 0x0f, 0x84, 0x62, 0x02, 0x00, 0x00, 0x48, 0x89,
                0xbc, 0x24, 0x90, 0x20, 0x00, 0x00, 0xe8, 0x03, 0xa2, 0xe9, 0xff, 0x8b, 0x4b, 0x38, 0x48, 0x8d, 0x53, 0x40,
                0x48, 0xc1, 0xe1, 0x06, 0x45, 0x33, 0xc0, 0x48, 0x8b, 0xf8, 0x48, 0x03, 0x88, 0xe0, 0x02, 0x00, 0x00,
            ],
        ),
    },
];

/// Messages lus avec une correction et pas encore traités (adresses). Borné : un message détruit
/// sans traitement ne reste pas.
static ABIMES: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());
const MAX_ABIMES: usize = 64;
/// Lecture et traitement d'origine de chaque message de `MESSAGES`.
static ORIGINES: [[AtomicUsize; 2]; 2] = [[AtomicUsize::new(0), AtomicUsize::new(0)], [AtomicUsize::new(0), AtomicUsize::new(0)]];

/// Return addresses of the seven reads in the loaded exe (0 until the guard is installed).
static RETOURS: [AtomicUsize; 7] = [
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
];
/// Address of the configuration singleton in the loaded exe.
static CONFIGURATION: AtomicUsize = AtomicUsize::new(0);
static TRAMP: AtomicUsize = AtomicUsize::new(0);
static CORRECTIONS: AtomicU32 = AtomicU32::new(0);
/// Corrections pas encore annoncées dans le chat (`a_annoncer`).
static A_ANNONCER: AtomicU32 = AtomicU32::new(0);
/// Mode test : injection du compte du gel encore à faire.
static INJECTION: AtomicBool = AtomicBool::new(false);
/// The exe is not one of the builds BGE knows: none of the network fixes is installed.
static UNKNOWN_EXE: AtomicBool = AtomicBool::new(false);

pub fn unknown_exe() -> bool {
    UNKNOWN_EXE.load(Ordering::Relaxed)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off,
    On,
    /// La garde, plus une injection du compte du gel à la première lecture « carte : entrées ».
    Test,
}

/// Le réglage `garde_messages` de `rollover_proxy.cfg` (dernière occurrence ; `on` par défaut).
/// Lu seulement avec la feature `diag`.
#[cfg_attr(not(feature = "diag"), allow(dead_code))]
pub fn mode(cfg: &str) -> Mode {
    match cfg.lines().filter_map(|l| l.trim().strip_prefix("garde_messages=")).last().map(str::trim) {
        Some("off") => Mode::Off,
        Some("test") => Mode::Test,
        _ => Mode::On,
    }
}

/// Pourquoi la valeur lue est impossible, ou `None` si elle est plausible. `drapeaux` ne sert qu'à
/// la règle `Longueur`.
pub fn absurde(regle: Regle, valeur: u32, drapeaux: u32) -> Option<&'static str> {
    match regle {
        Regle::CompteOuDrapeaux if valeur & 0x8000_0000 != 0 => None,
        Regle::Compte | Regle::CompteOuDrapeaux => (valeur > MAX_ENTREES).then_some("compte"),
        Regle::Longueur => {
            // Carte ou liste (bits 25 à 28, 30) : la longueur ne sert pas.
            if drapeaux & 0x5e00_0000 != 0 {
                return None;
            }
            let octets = u64::from(drapeaux & 0xff_ffff) * u64::from(valeur);
            let limite = if drapeaux & 0x0100_0000 != 0 { MAX_TAMPON } else { MAX_SCALAIRE };
            (octets > limite).then_some("longueur")
        }
    }
}

/// L'identifiant d'un `NetPlayerInfo` est-il applicable ? -1 (aucun joueur) : le jeu ne fait rien ;
/// sinon, un emplacement existant.
pub fn joueur_valide(id: i32, emplacements: usize) -> bool {
    id == -1 || usize::try_from(id).is_ok_and(|i| i < emplacements)
}

/// Would this `NetGameConfig` wipe the game configuration? No changed key (whole replacement) and an
/// empty map (first entry = end marker).
pub fn wipes_configuration(changed_keys: u32, first: usize, end: usize) -> bool {
    changed_keys == 0 && first == end
}

unsafe fn config_wipes(msg: *mut u8) -> bool {
    let changed_keys = std::ptr::read_unaligned(msg.add(CONFIG_CHANGED_KEYS) as *const u32);
    let map = msg as usize + CONFIG_MAP;
    wipes_configuration(changed_keys, std::ptr::read_unaligned((map + MAP_FIRST) as *const usize), map + MAP_END)
}

/// Une correction de plus (journal, chat) ; le total.
fn compter() -> u32 {
    A_ANNONCER.fetch_add(1, Ordering::Relaxed);
    CORRECTIONS.fetch_add(1, Ordering::Relaxed) + 1
}

/// Les corrections faites depuis le dernier appel, remises à 0 : `Civ6Proxy_GardeMessages()`, que
/// `partie.lua` interroge en jeu pour le dire dans le chat.
pub fn a_annoncer() -> u32 {
    A_ANNONCER.swap(0, Ordering::Relaxed)
}

// `C-unwind` : si la lecture d'origine levait une exception C++, elle traverserait ces cadres au
// lieu d'arrêter le processus. Aucune levée n'a été trouvée dans le désérialiseur (Ghidra,
// 2026-11-10), mais la lecture virtuelle du lecteur n'a pas été suivie.
type Lecture = unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, *mut u8, usize) -> usize;

/// La lecture détournée. Le bout d'assembleur `bgd_messages_lecture_stub` l'appelle avec, en
/// cinquième argument, l'adresse de retour de l'appelant de la lecture : le site dans le
/// désérialiseur. Après la lecture d'origine, une valeur impossible à l'un des sept sites devient 0.
unsafe extern "C-unwind" fn lecture(lecteur: *mut c_void, desc: *mut c_void, dest: *mut u8, n: usize, retour: usize) -> usize {
    let real: Lecture = std::mem::transmute(TRAMP.load(Ordering::Relaxed));
    let site = if n == 4 && !dest.is_null() { RETOURS.iter().position(|r| r.load(Ordering::Relaxed) == retour) } else { None };
    let Some(i) = site else {
        return real(lecteur, desc, dest, n);
    };
    // A failed read leaves the destination as it was: start from a known value, not from a leftover
    // one. A message whose root map count cannot be read is damaged (not applied); elsewhere 0, as
    // the game itself does for the changed keys.
    let racine = i == SITE_RACINE;
    std::ptr::write_unaligned(dest as *mut u32, if racine { NON_LU } else { 0 });
    let r = real(lecteur, desc, dest, n);
    let (nom, regle) = SITES[i];
    if racine && std::ptr::read_unaligned(dest as *const u32) == NON_LU {
        std::ptr::write_unaligned(dest as *mut u32, 0);
        let total = compter();
        if total <= 20 || total % 100 == 0 {
            log(&format!("messages: {nom}: read failed, count set to 0 ({total} corrections)"));
        }
        return r;
    }
    if i == SITE_INJECTION && INJECTION.swap(false, Ordering::Relaxed) {
        std::ptr::write_unaligned(dest as *mut u32, 0x6174_6143);
        log("messages : test : compte du gel (0x61746143) injecté à la lecture « carte : entrées », la garde doit le remplacer");
    }
    let valeur = std::ptr::read_unaligned(dest as *const u32);
    let drapeaux = if regle == Regle::Longueur { std::ptr::read_unaligned(dest.add(4) as *const u32) } else { 0 };
    if let Some(motif) = absurde(regle, valeur, drapeaux) {
        std::ptr::write_unaligned(dest as *mut u32, 0);
        let total = compter();
        if total <= 20 || total % 100 == 0 {
            log(&format!("messages : {nom} : {motif} impossible ({valeur:#x}, drapeaux {drapeaux:#x}) remplacé par 0 ({total} corrections)"));
        }
    }
    r
}

// Le détour reçoit les quatre arguments de la lecture et, en [rsp], l'adresse de retour de son
// appelant : on la passe en cinquième argument (pile, après les 32 octets de garde). À l'entrée,
// rsp ≡ 8 (mod 16) ; moins 0x28, il est aligné pour l'appel.
std::arch::global_asm!(
    ".text",
    ".globl bgd_messages_lecture_stub",
    ".seh_proc bgd_messages_lecture_stub",
    "bgd_messages_lecture_stub:",
    "    mov r10, qword ptr [rsp]",
    "    sub rsp, 0x28",
    "    .seh_stackalloc 0x28",
    "    .seh_endprologue",
    "    mov qword ptr [rsp + 0x20], r10",
    "    call {lecture}",
    "    add rsp, 0x28",
    "    ret",
    ".seh_endproc",
    lecture = sym lecture,
);
extern "C-unwind" {
    fn bgd_messages_lecture_stub();
}

type Fonction = unsafe extern "C-unwind" fn(*mut u8, *mut c_void);

/// Lecture d'un message de `MESSAGES` : s'il a fallu corriger une lecture pendant la sienne, il est
/// noté abîmé. Le compteur est global : une correction faite au même instant sur un autre thread
/// fait au pire écarter ce message aussi (la garde n'agit presque jamais).
unsafe fn lire(i: usize, msg: *mut u8, lecteur: *mut c_void) {
    let origine: Fonction = std::mem::transmute(ORIGINES[i][0].load(Ordering::Relaxed));
    let avant = CORRECTIONS.load(Ordering::Relaxed);
    origine(msg, lecteur);
    let abime = CORRECTIONS.load(Ordering::Relaxed) != avant;
    let mut liste = ABIMES.lock().unwrap_or_else(|e| e.into_inner());
    // Une adresse réemployée par un nouveau message repart propre.
    liste.retain(|&m| m != msg as usize);
    if abime {
        if liste.len() >= MAX_ABIMES {
            liste.remove(0);
        }
        liste.push(msg as usize);
        drop(liste);
        log(&format!("messages: {} damaged when read, it will not be applied", MESSAGES[i]));
    }
}

/// Les emplacements de joueurs de la configuration, si elle existe déjà.
unsafe fn emplacements() -> Option<usize> {
    let configuration = *(CONFIGURATION.load(Ordering::Relaxed) as *const usize);
    if configuration == 0 {
        return None;
    }
    let debut = *((configuration + EMPLACEMENTS) as *const usize);
    let fin = *((configuration + EMPLACEMENTS + 8) as *const usize);
    Some(fin.saturating_sub(debut) / 0x40)
}

/// Traitement d'un message de `MESSAGES` : pas appliqué s'il est abîmé, ni pour un `NetPlayerInfo`
/// dont l'identifiant n'est pas un emplacement.
unsafe fn traiter(i: usize, msg: *mut u8, contexte: *mut c_void) {
    let abime = {
        let mut liste = ABIMES.lock().unwrap_or_else(|e| e.into_inner());
        let position = liste.iter().position(|&m| m == msg as usize);
        position.map(|p| liste.remove(p)).is_some()
    };
    if abime {
        log(&format!("messages: damaged {} not applied, the configuration is kept", MESSAGES[i]));
        return;
    }
    if i == 0 && config_wipes(msg) {
        let total = compter();
        log(&format!("messages: NetGameConfig replacing the whole configuration with nothing, not applied ({total} corrections)"));
        return;
    }
    if i == 1 {
        let id = std::ptr::read_unaligned(msg.add(0x38) as *const i32);
        if let Some(n) = emplacements() {
            if !joueur_valide(id, n) {
                let total = compter();
                log(&format!("messages : NetPlayerInfo : joueur {id} hors des {n} emplacements, non appliqué ({total} corrections)"));
                return;
            }
        }
    }
    let origine: Fonction = std::mem::transmute(ORIGINES[i][1].load(Ordering::Relaxed));
    origine(msg, contexte);
}

unsafe extern "C-unwind" fn lire_config(msg: *mut u8, lecteur: *mut c_void) {
    lire(0, msg, lecteur)
}
unsafe extern "C-unwind" fn traiter_config(msg: *mut u8, contexte: *mut c_void) {
    traiter(0, msg, contexte)
}
unsafe extern "C-unwind" fn lire_joueur(msg: *mut u8, lecteur: *mut c_void) {
    lire(1, msg, lecteur)
}
unsafe extern "C-unwind" fn traiter_joueur(msg: *mut u8, contexte: *mut c_void) {
    traiter(1, msg, contexte)
}

/// Détourne lecture et traitement des messages de `MESSAGES` en remplaçant leurs entrées de vtable.
/// Les vtables sont relogées par le chargeur avant DllMain : `verifier` les a comparées à l'exe chargé.
unsafe fn installer_messages(exe: usize, table: &Exe) -> Result<(), String> {
    let detours: [[Fonction; 2]; 2] = [[lire_config, traiter_config], [lire_joueur, traiter_joueur]];
    for (i, (adresse, _)) in table.vtables.iter().enumerate() {
        let vtable = (exe + (adresse - BASE) as usize) as *mut usize;
        let mut old = 0u32;
        if win::VirtualProtect(vtable.add(LIRE) as *mut c_void, 16, win::PAGE_EXECUTE_READWRITE, &mut old) == 0 {
            return Err(format!("{}: VirtualProtect failed", MESSAGES[i]));
        }
        ORIGINES[i][0].store(*vtable.add(LIRE), Ordering::Relaxed);
        ORIGINES[i][1].store(*vtable.add(TRAITER), Ordering::Relaxed);
        *vtable.add(LIRE) = detours[i][0] as usize;
        *vtable.add(TRAITER) = detours[i][1] as usize;
        win::VirtualProtect(vtable.add(LIRE) as *mut c_void, 16, old, &mut old);
    }
    Ok(())
}

/// Are all the table's bytes there, in the image read by `lire(address, n)`? Otherwise, the first
/// difference. `entree(address)` reads a vtable entry and brings it back to base `0x140000000`.
/// Shared by the loaded exe (`verifier`) and the files (tests).
fn empreinte<'a>(table: &Exe, lire: impl Fn(u64, usize) -> &'a [u8], entree: impl Fn(u64) -> u64) -> Result<(), String> {
    if lire(table.lecture, PROLOGUE.len()) != PROLOGUE {
        return Err(format!("ReadSerializerField: unexpected bytes at {:#x}", table.lecture));
    }
    for ((nom, debut), adresse) in FONCTIONS.iter().zip(table.fonctions) {
        if lire(adresse, debut.len()) != debut {
            return Err(format!("{nom} : octets inattendus en {adresse:#x}"));
        }
    }
    for ((nom, _), retour) in SITES.iter().zip(table.retours) {
        if lire(retour - APPEL.len() as u64, APPEL.len()) != APPEL {
            return Err(format!("{nom}: no read call before {retour:#x}"));
        }
    }
    let (singleton, octets_singleton) = &table.singleton;
    let (traitement, octets_traitement) = &table.traitement_joueur;
    let empreintes: [(&str, u64, &[u8]); 2] = [
        ("configuration singleton", *singleton, octets_singleton),
        ("NetPlayerInfo handler", *traitement, octets_traitement),
    ];
    for (nom, adresse, octets) in empreintes {
        if lire(adresse, octets.len()) != octets {
            return Err(format!("{nom} : octets inattendus en {adresse:#x}"));
        }
    }
    // 7-byte `mov rax,[rip+d]` at offset 4: the singleton is at function + 11 + d.
    let d = i32::from_le_bytes(octets_singleton[7..11].try_into().unwrap());
    if singleton.wrapping_add(11).wrapping_add_signed(i64::from(d)) != table.configuration {
        return Err(format!("configuration singleton: does not lead to {:#x}", table.configuration));
    }
    for (nom, (vtable, entrees)) in MESSAGES.iter().zip(&table.vtables) {
        for (k, attendue) in entrees.iter().enumerate() {
            if entree(vtable + 8 * k as u64) != *attendue {
                return Err(format!("{nom}: unexpected vtable at {vtable:#x}"));
            }
        }
    }
    Ok(())
}

/// Does the table match the loaded exe? Its vtables are relocated there by the loader.
unsafe fn verifier(exe: usize, table: &Exe) -> Result<(), String> {
    let image = |adresse: u64| exe + (adresse - BASE) as usize;
    empreinte(
        table,
        |adresse, n| std::slice::from_raw_parts(image(adresse) as *const u8, n),
        |adresse| (*(image(adresse) as *const usize)).wrapping_sub(exe) as u64 + BASE,
    )
}

/// Pose le détour, sauf `garde_messages=off`. Appelée par DllMain : aucun code de l'exe n'a encore
/// tourné, la lecture n'est donc exécutée par aucun thread pendant qu'on l'écrase.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    // L'interrupteur général : avec `actif=off`, le jeu doit se comporter comme sans BGE.
    if crate::salon::actif_off(&cfg) {
        log("messages : garde non posée (actif=off)");
        return;
    }
    // Correctif de bug : pas de réglage en release, la garde est toujours posée.
    #[cfg(feature = "diag")]
    let m = mode(&cfg);
    #[cfg(not(feature = "diag"))]
    let m = Mode::On;
    match m {
        Mode::Off => {
            log("messages : garde désactivée (garde_messages=off)");
            return;
        }
        Mode::Test => INJECTION.store(true, Ordering::Relaxed),
        Mode::On => {}
    }
    let exe = win::GetModuleHandleW(std::ptr::null()) as usize;
    let mut raisons = Vec::new();
    let Some(table) = EXES.iter().find(|t| verifier(exe, t).map_err(|e| raisons.push(format!("{}: {e}", t.nom))).is_ok()) else {
        log(&format!(
            "messages: unknown exe ({}), guard not installed (DX12 or DX11 exe of 2026-11-05 expected)",
            raisons.join("; ")
        ));
        UNKNOWN_EXE.store(true, Ordering::Relaxed);
        return;
    };
    let image = |adresse: u64| exe + (adresse - BASE) as usize;
    for (r, retour) in RETOURS.iter().zip(table.retours) {
        r.store(image(retour), Ordering::Relaxed);
    }
    CONFIGURATION.store(image(table.configuration), Ordering::Relaxed);
    let at = image(table.lecture);
    match patch::install_at("lecture de champ", at, &PROLOGUE, at + PROLOGUE.len(), bgd_messages_lecture_stub as *const () as usize) {
        Ok(t) => {
            TRAMP.store(t, Ordering::Relaxed);
            log(&format!("messages: guard installed ({}, field read at exe+{:#x}, {} sites)", table.nom, at - exe, SITES.len()));
            match installer_messages(exe, table) {
                Ok(()) => log("messages : NetGameConfig et NetPlayerInfo abîmés écartés"),
                Err(e) => log(&format!("messages : {e}, messages de configuration non surveillés")),
            }
        }
        Err(e) => log(&format!("messages : {e}, garde absente")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";

    #[test]
    fn reglage() {
        assert_eq!(mode(""), Mode::On);
        assert_eq!(mode("mode=coalesce\r\n"), Mode::On);
        assert_eq!(mode("garde_messages=off\r\n"), Mode::Off);
        assert_eq!(mode("garde_messages=test\r\n"), Mode::Test);
        assert_eq!(mode("garde_messages=off\ngarde_messages=on\n"), Mode::On, "la dernière occurrence compte");
    }

    #[test]
    fn annonce_remise_a_zero() {
        A_ANNONCER.store(3, Ordering::Relaxed);
        assert_eq!(a_annoncer(), 3);
        assert_eq!(a_annoncer(), 0, "une correction n'est annoncée qu'une fois");
    }

    #[test]
    fn empty_whole_replacement_wipes() {
        let end = 0x1000_0030;
        assert!(wipes_configuration(0, end, end), "no changed key, empty map");
        assert!(!wipes_configuration(0, 0x2000_0000, end), "whole replacement with entries");
        assert!(!wipes_configuration(3, end, end), "only the changed keys are touched");
    }

    #[test]
    fn identifiant_du_joueur() {
        assert!(joueur_valide(-1, 0), "aucun joueur : le jeu ne fait rien");
        assert!(joueur_valide(0, 12));
        assert!(joueur_valide(11, 12));
        assert!(!joueur_valide(12, 12), "hors des emplacements");
        assert!(!joueur_valide(0x6174_6143, 64), "identifiant abîmé");
        assert!(!joueur_valide(-2, 64));
    }

    #[test]
    fn regles() {
        assert_eq!(absurde(Regle::Compte, 50, 0), None);
        assert_eq!(absurde(Regle::Compte, MAX_ENTREES, 0), None);
        assert_eq!(absurde(Regle::Compte, 0x6174_6143, 0), Some("compte"), "le compte du gel");
        assert_eq!(absurde(Regle::CompteOuDrapeaux, 0x8000_0001, 0), None, "mot de drapeaux");
        assert_eq!(absurde(Regle::CompteOuDrapeaux, 0x7fff_ffff, 0), Some("compte"));
        // Chaîne de 1 000 octets : plausible ; 20 Mo : non.
        assert_eq!(absurde(Regle::Longueur, 1000, 0x0100_0001), None);
        assert_eq!(absurde(Regle::Longueur, 20 << 20, 0x0100_0001), Some("longueur"));
        // Carte imbriquée : la longueur ne sert pas, quelle qu'elle soit.
        assert_eq!(absurde(Regle::Longueur, 0xffff_ffff, 0x0200_0000), None);
        // Scalaire de 4 octets : 1 élément oui, 2 000 non.
        assert_eq!(absurde(Regle::Longueur, 1, 4), None);
        assert_eq!(absurde(Regle::Longueur, 2000, 4), Some("longueur"));
    }

    /// Décalage dans le fichier d'une adresse virtuelle, par les en-têtes de section du PE.
    fn offset(file: &[u8], adresse: u64) -> Option<usize> {
        let rd32 = |o: usize| u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
        let rd16 = |o: usize| u16::from_le_bytes(file[o..o + 2].try_into().unwrap()) as usize;
        let nt = rd32(0x3c);
        let sections = nt + 24 + rd16(nt + 20);
        let rva = (adresse - BASE) as usize;
        (0..rd16(nt + 6)).map(|i| sections + 40 * i).find_map(|s| {
            let (va, taille, brut) = (rd32(s + 12), rd32(s + 16), rd32(s + 20));
            (rva >= va && rva < va + taille).then(|| rva - va + brut)
        })
    }

    /// The file of each exe, in the order of `EXES`.
    const FICHIERS: [&str; 2] = ["CivilizationVI_DX12.exe", "CivilizationVI.exe"];

    /// The table applied to an exe file. In the file, the vtables hold addresses at the preferred
    /// base (0x140000000), without relocation.
    fn empreinte_fichier(table: &Exe, fichier: &[u8]) -> Result<(), String> {
        let lire = |adresse: u64, n: usize| offset(fichier, adresse).map_or(&[][..], |o| &fichier[o..][..n]);
        empreinte(table, lire, |adresse| u64::from_le_bytes(lire(adresse, 8).try_into().unwrap_or_default()))
    }

    /// Each table matches its own installed exe, and only that one.
    #[test]
    fn octets_dans_les_exes() {
        for (i, nom) in FICHIERS.iter().enumerate() {
            let Ok(fichier) = std::fs::read(format!(r"{EXE_DIR}\{nom}")) else {
                eprintln!("{nom} missing, test skipped");
                continue;
            };
            for (j, table) in EXES.iter().enumerate() {
                let resultat = empreinte_fichier(table, &fichier);
                if i == j {
                    assert_eq!(resultat, Ok(()), "table {} on {nom}", table.nom);
                } else {
                    assert!(resultat.is_err(), "the {} table must not match {nom}", table.nom);
                }
            }
        }
    }
}
