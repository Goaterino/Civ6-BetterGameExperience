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
//! Noms : projet Ghidra `civ6-netcode`, signets « Gel 2026-11-09 ». Les mêmes règles, posées par
//! un hook externe pendant la mise au point, ont tourné sur deux parties entières de dix joueurs
//! (162 tours, 108 000 messages reçus) sans fausse alerte.
//! Adresses de `CivilizationVI_DX12.exe` du 5 novembre 2026, vérifiées octet par octet : sur un
//! autre exe (DX11, autre version), rien n'est posé et le journal le dit.
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

/// `ReadSerializerField(lecteur, descripteur, destination, octets)`. Prologue : `mov [rsp+8],rbx ;
/// push rdi ; sub rsp,70h ; mov rax,r8 ; mov rbx,rdx`, 16 octets sans adressage relatif, recopiés
/// dans le trampoline.
const LECTURE: u64 = 0x1_4099_f9d0;
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

struct Site {
    nom: &'static str,
    /// Adresse de retour de `call qword ptr [rax+30h]` dans la fonction.
    retour: u64,
    regle: Regle,
}

const SITES: [Site; 7] = [
    Site { nom: "carte : paires", retour: 0x1_407a_1f44, regle: Regle::Compte },
    Site { nom: "carte : entrées", retour: 0x1_407a_209f, regle: Regle::Compte },
    Site { nom: "carte nommée : compte", retour: 0x1_407a_dfe3, regle: Regle::CompteOuDrapeaux },
    Site { nom: "carte nommée : compte après drapeaux", retour: 0x1_407a_e03a, regle: Regle::Compte },
    Site { nom: "liste : compte", retour: 0x1_407a_d901, regle: Regle::Compte },
    Site { nom: "variant : longueur", retour: 0x1_407a_34f4, regle: Regle::Longueur },
    Site { nom: "configuration : clés changées", retour: 0x1_4026_619b, regle: Regle::Compte },
];
/// `call qword ptr [rax+30h]`, juste avant chaque adresse de retour.
const APPEL: [u8; 3] = [0xff, 0x50, 0x30];

/// Débuts des fonctions qui contiennent ces lectures : avec les sites et le prologue de la lecture,
/// l'empreinte de l'exe analysé.
const FONCTIONS: &[(&str, u64, [u8; 12])] = &[
    ("DeserializeTypedVariantMap", 0x1_407a_1e20, [0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x48]),
    ("DeserializeDataVariant", 0x1_407a_3450, [0x4c, 0x8b, 0xdc, 0x49, 0x89, 0x5b, 0x10, 0x49, 0x89, 0x73, 0x18, 0x57]),
    ("DeserializeNamedVariantMap", 0x1_407a_df50, [0x48, 0x8b, 0xc4, 0x55, 0x48, 0x8d, 0xa8, 0x38, 0xfe, 0xff, 0xff, 0x48]),
    ("DeserializeVariantList", 0x1_407a_d890, [0x48, 0x89, 0x5c, 0x24, 0x18, 0x48, 0x89, 0x6c, 0x24, 0x20, 0x56, 0x41]),
    ("NetGameConfig : clés changées", 0x1_4026_6140, [0x48, 0x89, 0x5c, 0x24, 0x18, 0x48, 0x89, 0x74, 0x24, 0x20, 0x55, 0x57]),
];

/// Un message dont le traitement efface une configuration avant d'y recopier la carte reçue
/// (entrée `+0x178` de la carte, `1407a6660`, quatrième argument 0 : effacement par `+0x160`).
struct Message {
    nom: &'static str,
    /// Sa vtable (`Civ6NetMessage.cpp`), posée par ses constructeurs.
    vtable: u64,
    /// Destructeur, texte, écriture, lecture, traitement.
    entrees: [u64; 5],
}
const LIRE: usize = 3;
const TRAITER: usize = 4;
const MESSAGES: [Message; 2] = [
    // Traitement `140266e00` : `1401033f0(configuration, carte, clés changées)`. Sans clé changée,
    // toute la configuration de la partie est effacée puis reconstruite.
    Message {
        nom: "NetGameConfig",
        vtable: 0x1_40fe_0020,
        entrees: [0x1_4026_69c0, 0x1_4026_6ca0, 0x1_4026_7680, 0x1_4026_79d0, 0x1_4026_6e00],
    },
    // Traitement `140267330` : `140116630(emplacements + id × 0x40, carte, null)`, la configuration
    // du joueur effacée puis recopiée. Identifiant en message + 0x38.
    Message {
        nom: "NetPlayerInfo",
        vtable: 0x1_40fe_0048,
        entrees: [0x1_4026_6aa0, 0x1_4026_6da0, 0x1_4026_7810, 0x1_4026_7c40, 0x1_4026_7330],
    },
];
/// Le singleton de la configuration (lu par `1401008b0`) ; ses emplacements de joueurs, de 0x40
/// octets, vont de `[+0x2e0]` à `[+0x2e8]`.
const CONFIGURATION: u64 = 0x1_4231_00a0;
const EMPLACEMENTS: usize = 0x2e0;
/// `sub rsp,38h ; mov rax,[rip+0220f7e5]` : le singleton lu en `1423100a0`.
const SINGLETON: (u64, [u8; 11]) = (0x1_4010_08b0, [0x48, 0x83, 0xec, 0x38, 0x48, 0x8b, 0x05, 0xe5, 0xf7, 0x20, 0x02]);
/// Début du traitement de `NetPlayerInfo`, jusqu'au calcul `emplacements + id × 0x40` (`cmp
/// [rcx+38h],-1` … `add rcx,[rax+2e0h]`).
const TRAITEMENT_JOUEUR: (u64, [u8; 0x35]) = (
    0x1_4026_7340,
    [
        0x83, 0x79, 0x38, 0xff, 0x48, 0x8b, 0xf2, 0x48, 0x8b, 0xd9, 0x0f, 0x84, 0x62, 0x02, 0x00, 0x00, 0x48, 0x89, 0xbc, 0x24,
        0x90, 0x20, 0x00, 0x00, 0xe8, 0x53, 0x95, 0xe9, 0xff, 0x8b, 0x4b, 0x38, 0x48, 0x8d, 0x53, 0x40, 0x48, 0xc1, 0xe1, 0x06,
        0x45, 0x33, 0xc0, 0x48, 0x8b, 0xf8, 0x48, 0x03, 0x88, 0xe0, 0x02, 0x00, 0x00,
    ],
);
/// Messages lus avec une correction et pas encore traités (adresses). Borné : un message détruit
/// sans traitement ne reste pas.
static ABIMES: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());
const MAX_ABIMES: usize = 64;
/// Lecture et traitement d'origine de chaque message de `MESSAGES`.
static ORIGINES: [[AtomicUsize; 2]; 2] = [[AtomicUsize::new(0), AtomicUsize::new(0)], [AtomicUsize::new(0), AtomicUsize::new(0)]];

static EXE: AtomicUsize = AtomicUsize::new(0);
static TRAMP: AtomicUsize = AtomicUsize::new(0);
static CORRECTIONS: AtomicU32 = AtomicU32::new(0);
/// Corrections pas encore annoncées dans le chat (`a_annoncer`).
static A_ANNONCER: AtomicU32 = AtomicU32::new(0);
/// Mode test : injection du compte du gel encore à faire.
static INJECTION: AtomicBool = AtomicBool::new(false);

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
    let r = real(lecteur, desc, dest, n);
    if n == 4 && !dest.is_null() {
        let exe = EXE.load(Ordering::Relaxed);
        if let Some(site) = SITES.iter().find(|s| exe + (s.retour - BASE) as usize == retour) {
            if site.retour == 0x1_407a_209f && INJECTION.swap(false, Ordering::Relaxed) {
                std::ptr::write_unaligned(dest as *mut u32, 0x6174_6143);
                log("messages : test : compte du gel (0x61746143) injecté à la lecture « carte : entrées », la garde doit le remplacer");
            }
            let valeur = std::ptr::read_unaligned(dest as *const u32);
            let drapeaux = if site.regle == Regle::Longueur { std::ptr::read_unaligned(dest.add(4) as *const u32) } else { 0 };
            if let Some(motif) = absurde(site.regle, valeur, drapeaux) {
                std::ptr::write_unaligned(dest as *mut u32, 0);
                let total = compter();
                if total <= 20 || total % 100 == 0 {
                    log(&format!(
                        "messages : {} : {motif} impossible ({valeur:#x}, drapeaux {drapeaux:#x}) remplacé par 0 ({total} corrections)",
                        site.nom
                    ));
                }
            }
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
        log(&format!("messages : {} abîmé à la lecture, il ne sera pas appliqué", MESSAGES[i].nom));
    }
}

/// Les emplacements de joueurs de la configuration, si elle existe déjà.
unsafe fn emplacements() -> Option<usize> {
    let exe = EXE.load(Ordering::Relaxed);
    let configuration = *((exe + (CONFIGURATION - BASE) as usize) as *const usize);
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
        log(&format!("messages : {} abîmé non appliqué, la configuration est gardée", MESSAGES[i].nom));
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
unsafe fn installer_messages(exe: usize) -> Result<(), String> {
    let detours: [[Fonction; 2]; 2] = [[lire_config, traiter_config], [lire_joueur, traiter_joueur]];
    for (i, m) in MESSAGES.iter().enumerate() {
        let vtable = (exe + (m.vtable - BASE) as usize) as *mut usize;
        let mut old = 0u32;
        if win::VirtualProtect(vtable.add(LIRE) as *mut c_void, 16, win::PAGE_EXECUTE_READWRITE, &mut old) == 0 {
            return Err(format!("{} : VirtualProtect a échoué", m.nom));
        }
        ORIGINES[i][0].store(*vtable.add(LIRE), Ordering::Relaxed);
        ORIGINES[i][1].store(*vtable.add(TRAITER), Ordering::Relaxed);
        *vtable.add(LIRE) = detours[i][0] as usize;
        *vtable.add(TRAITER) = detours[i][1] as usize;
        win::VirtualProtect(vtable.add(LIRE) as *mut c_void, 16, old, &mut old);
    }
    Ok(())
}

/// Les octets attendus sont-ils tous là ? Sinon, la première différence.
unsafe fn verifier(exe: usize) -> Result<(), String> {
    let lire = |adresse: u64, n: usize| std::slice::from_raw_parts((exe + (adresse - BASE) as usize) as *const u8, n);
    if lire(LECTURE, PROLOGUE.len()) != &PROLOGUE[..] {
        return Err(format!("ReadSerializerField : octets inattendus en {LECTURE:#x}"));
    }
    for (nom, adresse, debut) in FONCTIONS {
        if lire(*adresse, debut.len()) != &debut[..] {
            return Err(format!("{nom} : octets inattendus en {adresse:#x}"));
        }
    }
    for s in &SITES {
        if lire(s.retour - APPEL.len() as u64, APPEL.len()) != &APPEL[..] {
            return Err(format!("{} : pas d'appel de lecture avant {:#x}", s.nom, s.retour));
        }
    }
    let empreintes: [(&str, u64, &[u8]); 2] = [
        ("singleton de la configuration", SINGLETON.0, &SINGLETON.1),
        ("traitement de NetPlayerInfo", TRAITEMENT_JOUEUR.0, &TRAITEMENT_JOUEUR.1),
    ];
    for (nom, adresse, octets) in empreintes {
        if lire(adresse, octets.len()) != octets {
            return Err(format!("{nom} : octets inattendus en {adresse:#x}"));
        }
    }
    for m in &MESSAGES {
        let vtable = (exe + (m.vtable - BASE) as usize) as *const usize;
        for (k, entree) in m.entrees.iter().enumerate() {
            if *vtable.add(k) != exe + (entree - BASE) as usize {
                return Err(format!("{} : vtable inattendue en {:#x}", m.nom, m.vtable));
            }
        }
    }
    Ok(())
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
    if let Err(e) = verifier(exe) {
        log(&format!("messages : {e}, garde absente (exe DX12 du 5 novembre 2026 attendu)"));
        return;
    }
    EXE.store(exe, Ordering::Relaxed);
    let at = exe + (LECTURE - BASE) as usize;
    match patch::install_at("lecture de champ", at, &PROLOGUE, at + PROLOGUE.len(), bgd_messages_lecture_stub as *const () as usize) {
        Ok(t) => {
            TRAMP.store(t, Ordering::Relaxed);
            log(&format!("messages : garde posée (lecture de champ en exe+{:#x}, {} sites)", at - exe, SITES.len()));
            match installer_messages(exe) {
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

    /// L'empreinte correspond à l'exe DX12 installé, et pas à l'exe DX11.
    #[test]
    fn octets_dans_les_exes() {
        let Ok(dx12) = std::fs::read(format!(r"{EXE_DIR}\CivilizationVI_DX12.exe")) else {
            eprintln!("exe absent, test sauté");
            return;
        };
        let lire = |adresse: u64, n: usize| &dx12[offset(&dx12, adresse).unwrap()..][..n];
        assert_eq!(lire(LECTURE, PROLOGUE.len()), &PROLOGUE[..]);
        for (nom, adresse, debut) in FONCTIONS {
            assert_eq!(lire(*adresse, debut.len()), &debut[..], "{nom}");
        }
        for s in &SITES {
            assert_eq!(lire(s.retour - APPEL.len() as u64, APPEL.len()), &APPEL[..], "{}", s.nom);
        }
        assert_eq!(lire(SINGLETON.0, SINGLETON.1.len()), &SINGLETON.1[..], "singleton");
        assert_eq!(lire(TRAITEMENT_JOUEUR.0, TRAITEMENT_JOUEUR.1.len()), &TRAITEMENT_JOUEUR.1[..], "traitement de NetPlayerInfo");
        // Dans le fichier, les vtables portent les adresses à la base préférée (0x140000000).
        for m in &MESSAGES {
            for (k, entree) in m.entrees.iter().enumerate() {
                let q = u64::from_le_bytes(lire(m.vtable + 8 * k as u64, 8).try_into().unwrap());
                assert_eq!(q, *entree, "{} : entrée {k}", m.nom);
            }
        }
        if let Ok(dx11) = std::fs::read(format!(r"{EXE_DIR}\CivilizationVI.exe")) {
            let same = offset(&dx11, LECTURE).is_some_and(|o| dx11[o..o + PROLOGUE.len()] == PROLOGUE);
            assert!(!same, "l'exe DX11 ne doit pas passer l'empreinte avec les adresses DX12");
        }
    }
}
