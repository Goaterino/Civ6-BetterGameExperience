//! Re-liaison des paires relayées (« 0 ms ») : deux joueurs dont le lien direct a échoué à
//! l'arrivée de l'un d'eux passent par l'hôte pour toute la partie. BGE refait le lien direct
//! pendant le départ de tour simultané, sans que personne ne quitte la partie.
//!
//! Ce que fait Civ6 (projet Ghidra `civ6-netcode`, exe DX12 du 5 novembre 2026) :
//! - un joueur qui arrive reçoit de l'hôte un `NetPeerInfoSteam(joueur, steamID)` par joueur déjà
//!   là, et ouvre vers chacun une connexion en attente (file de la session, un à la fois). L'autre
//!   accepte toute connexion qui porte la clé de session, puis chacun envoie son
//!   `NetIntroductionSteam` sur le nouveau lien : la file retire la connexion en attente, et
//!   `CreateNetPlayer` (`140812f90`) lie le joueur à la connexion ;
//! - sans réponse pendant 20 s, la connexion échoue : `NetPeerFailure` à l'hôte, qui diffuse
//!   `NetRelay(nouveau, existant)` à tous. Chaque machine note la paire dans le gestionnaire de
//!   relais (session + 0x60, table joueur → partenaires) ; les messages entre les deux passent par
//!   l'hôte (« 0 ms » dans la liste des joueurs). Rien ne retente le lien. Quand l'hôte part,
//!   chaque machine déconnecte tous les joueurs de sa table (`14084f5d0`) ;
//! - le chemin de Civ6 ne sait pas relier une paire déjà relayée : à l'introduction d'un joueur
//!   relayé, il saute la file et `CreateNetPlayer` remplace le joueur existant (« Disconnecting the
//!   old connection »), ce qui publie `DelayedPlayerLeft` : le jeu croirait que le joueur est parti
//!   et revenu (pause, gestion de déconnexion de MPH, désynchro).
//!
//! Ce que fait BGE (les deux joueurs de la paire ont BGE : le départ simultané l'exige) :
//! - `reparer(partenaire, steamID)` ouvre une connexion en attente vers le partenaire, comme à
//!   l'arrivée (on donne au traitement de `NetPeerInfoSteam` un message fabriqué) ;
//! - le traitement de `NetIntroductionSteam` est détourné : une introduction d'un joueur relayé
//!   avec nous, arrivée sur une connexion qu'aucun joueur n'a encore, est la re-liaison. On lie le
//!   joueur existant à cette connexion sur place (sa connexion en + 0x10, la table connexion →
//!   joueur), on retire notre connexion en attente vers lui, et on retire la paire de la table des
//!   relais : Civ6 lui envoie désormais ses messages en direct. Aucun départ n'est publié ;
//! - le traitement de `NetRelay` est détourné : un `NetRelay` d'une paire déjà relayée est ignoré.
//!   C'est ce que l'hôte renvoie quand notre tentative échoue ; Civ6 le traiterait en renvoyant les
//!   introductions par le relais, et le partenaire remplacerait le joueur (départ publié) ;
//! - une connexion en attente échouée reste en tête de file (l'hôte, qui relaie, ne répond pas à
//!   `NetPeerFailure`) : on la retire nous-mêmes, sinon elle bloque la tentative suivante ;
//! - `retirer(a, b)` enlève la paire de la table locale, chez tous les joueurs équipés, quand
//!   l'initiateur annonce la réussite (`partie.lua`) : sinon, au départ de l'hôte, ils
//!   déconnecteraient les deux joueurs.
//!
//! Le Lua (`partie.lua`) choisit le moment : le départ de tour simultané (ordres retenus, personne
//! ne joue, rien en vol entre les deux), dit dans le chat qu'il a trouvé un lien à 0 ms et qu'il le
//! répare, puis le résultat, et retient le signal « prêt » tant que la tentative n'est pas finie.
//! Adresses des deux exe du 5 novembre 2026 (`EXES`), vérifiées octet par octet au démarrage.

use crate::{log, win};
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

/// Base des adresses Ghidra de l'exe.
const BASE: u64 = 0x1_4000_0000;

/// Entrée « traitement » des vtables de messages (destructeur, texte, écriture, lecture, traitement).
const TRAITER: usize = 4;

/// Tout ce qui dépend de l'exe (adresses Ghidra, base `0x140000000`).
struct Exe {
    nom: &'static str,
    /// Vtables de `NetIntroductionSteam`, `NetRelay` et `NetPeerInfoSteam`, et leurs cinq entrées.
    intro: (u64, [u64; 5]),
    relay: (u64, [u64; 5]),
    peer_info: (u64, [u64; 5]),
    /// Vtables des messages qu'un joueur envoie sur chaque nouveau lien, dans l'ordre de `REPETES`.
    repetes: [(u64, [u64; 5]); 3],
    /// Le gestionnaire réseau (un pointeur global) : connexions en + 0x88, sessions en + 0x10.
    reseau: u64,
    /// Décalage, dans le traitement de `NetIntroductionSteam`, du `mov rcx,[rip+…]` qui lit `reseau`.
    reseau_lu_a: u64,
    file_tete: u64,
    file_retirer: u64,
    connexion: u64,
    connexion_steam: u64,
    joueur_trouver: u64,
    carte_case: u64,
    relais_effacer: u64,
    relais_contient: u64,
}

/// Premiers octets des fonctions appelées, les mêmes dans les deux exe, dans l'ordre : file_tete,
/// file_retirer, connexion, connexion_steam, joueur_trouver, carte_case, relais_effacer,
/// relais_contient.
const PROLOGUES: [(&str, [u8; 12]); 8] = [
    ("tête de la file", [0x48, 0x8b, 0x41, 0x18, 0x48, 0x2b, 0x41, 0x10, 0x48, 0xa9, 0xf8, 0xff]),
    ("retrait de la file", [0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xec, 0x20, 0x48, 0x8b]),
    ("connexion", [0x4c, 0x8b, 0x01, 0x49, 0x8b, 0xc8, 0x49, 0x8b, 0x40, 0x08, 0x80, 0x78]),
    ("steamID de la connexion", [0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48]),
    ("joueur", [0x40, 0x53, 0x48, 0x83, 0xec, 0x10, 0x44, 0x8b, 0x59, 0x28, 0x44, 0x8b]),
    ("case de la table des connexions", [0x48, 0x83, 0xec, 0x58, 0x44, 0x8b, 0x02, 0x33, 0xd2, 0x44, 0x8b, 0x49]),
    ("effacement d'un relayé", [0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x6c, 0x24, 0x10, 0x48, 0x89]),
    ("paire relayée ?", [0x48, 0x83, 0xec, 0x18, 0x44, 0x8b, 0x51, 0x10, 0x44, 0x8b, 0xca, 0x4c]),
];

/// Le DX11 a été trouvé fonction par fonction comme pour la garde des messages (mêmes octets hors
/// déplacements relatifs, même taille dans `.pdata`) ; outil : `Tools\civ6-dx11-map\relais_map.py`.
const EXES: [Exe; 2] = [
    Exe {
        nom: "DX12",
        intro: (0x1_4105_7460, [0x1_4085_1500, 0x1_4085_16d0, 0x1_4085_1ba0, 0x1_4085_1c00, 0x1_4085_1730]),
        relay: (0x1_4105_0b90, [0x1_4080_a9a0, 0x1_4080_f610, 0x1_4080_af90, 0x1_4080_afe0, 0x1_4081_0c60]),
        peer_info: (0x1_4105_7438, [0x1_4085_1500, 0x1_4085_1700, 0x1_4085_1ba0, 0x1_4085_1c00, 0x1_4085_1aa0]),
        repetes: [
            (0x1_4105_0be0, [0x1_4080_e4c0, 0x1_4080_f5b0, 0x1_4081_14c0, 0x1_4081_1e80, 0x1_4081_0930]),
            (0x1_4105_0c08, [0x1_4080_a9a0, 0x1_4080_f180, 0x1_4081_15a0, 0x1_4081_1760, 0x1_4080_f850]),
            (0x1_4105_0c80, [0x1_4080_a9a0, 0x1_4080_f6e0, 0x1_4081_15a0, 0x1_4081_1760, 0x1_4081_1020]),
        ],
        reseau: 0x1_4239_2ef8,
        reseau_lu_a: 0x52,
        file_tete: 0x1_4084_d580,
        file_retirer: 0x1_4084_d5d0,
        connexion: 0x1_4083_7ed0,
        connexion_steam: 0x1_4084_ecb0,
        joueur_trouver: 0x1_4081_35d0,
        carte_case: 0x1_4081_28f0,
        relais_effacer: 0x1_4084_f9d0,
        relais_contient: 0x1_4084_f3e0,
    },
    Exe {
        nom: "DX11",
        intro: (0x1_4104_7fd0, [0x1_4084_f860, 0x1_4084_fa30, 0x1_4084_ff00, 0x1_4084_ff60, 0x1_4084_fa90]),
        relay: (0x1_4104_1720, [0x1_4080_9350, 0x1_4080_d8a0, 0x1_4080_f780, 0x1_4081_0080, 0x1_4080_eef0]),
        peer_info: (0x1_4104_7fa8, [0x1_4084_f860, 0x1_4084_fa60, 0x1_4084_ff00, 0x1_4084_ff60, 0x1_4084_fe00]),
        repetes: [
            (0x1_4104_1770, [0x1_4080_c750, 0x1_4080_d840, 0x1_4080_f750, 0x1_4081_0050, 0x1_4080_ebc0]),
            (0x1_4104_1798, [0x1_4080_9350, 0x1_4080_d410, 0x1_4080_f470, 0x1_4081_02c0, 0x1_4080_dae0]),
            (0x1_4104_1810, [0x1_4080_9350, 0x1_4080_d970, 0x1_4080_f470, 0x1_4081_02c0, 0x1_4080_f2b0]),
        ],
        reseau: 0x1_4235_b878,
        reseau_lu_a: 0x52,
        file_tete: 0x1_4084_b990,
        file_retirer: 0x1_4084_b9e0,
        connexion: 0x1_4083_64e0,
        connexion_steam: 0x1_4084_d030,
        joueur_trouver: 0x1_4081_1b80,
        carte_case: 0x1_4081_0bd0,
        relais_effacer: 0x1_4084_dd50,
        relais_contient: 0x1_4084_d760,
    },
];

// Disposition des objets réseau, la même dans les deux exe (lue dans les accesseurs de la session :
// `mov rax,[rcx+48h] ; ret` etc., vérifiés à chaque emploi de la session).
const SESSION_JOUEURS: usize = 0x40;
const SESSION_FILE: usize = 0x48;
const SESSION_RELAIS: usize = 0x60;
/// Accesseurs de la session (entrées de sa vtable) et leurs octets : file, joueurs, relais.
const ACCESSEURS: [(usize, [u8; 5]); 3] =
    [(0x268, [0x48, 0x8b, 0x41, 0x48, 0xc3]), (0x270, [0x48, 0x8b, 0x41, 0x40, 0xc3]), (0x288, [0x48, 0x8b, 0x41, 0x60, 0xc3])];
/// Gestionnaire des joueurs : notre identifiant, table connexion → joueur (objet table).
const JOUEURS_LOCAL: usize = 0x08;
const JOUEURS_CONNEXIONS: usize = 0x40;
/// Joueur réseau : connexion (0 quand il est relayé).
const JOUEUR_CONNEXION: usize = 0x10;
/// Connexion en attente : joueur visé, état (0xb : échouée).
const ATTENTE_JOUEUR: usize = 0x50;
const ATTENTE_ETAT: usize = 0x5c;
const ETAT_ECHEC: i32 = 0xb;
/// Message réseau : connexion d'arrivée ; `NetIntroductionSteam` et `NetPeerInfoSteam` : joueur,
/// steamID ; `NetRelay` : nouveau, existant.
const MSG_CONNEXION: usize = 0x10;
const MSG_JOUEUR: usize = 0x38;
const MSG_STEAM: usize = 0x3c;
const MSG_RELAY_EXISTANT: usize = 0x3c;
/// Taille du message fabriqué pour `NetPeerInfoSteam` (il ne lit que + 0x38 à + 0x44).
const MSG_TAILLE: usize = 0x80;

/// Une tentative abandonnée après ce délai, réponse ou non (le transport abandonne à 20 s).
pub const TENTATIVE_MAX_MS: u64 = 45_000;

/// Ce que chaque joueur envoie sur un lien qui s'ouvre, juste après son introduction (vu dans le
/// journal du 10 novembre, 20:33:06 : introduction, `NetRegister`, `NetAppRegister`,
/// `NetSnapshotProcessed` infoOnly). Sur une re-liaison, le partenaire est déjà enregistré chez
/// nous (le même joueur réseau, ses drapeaux gardés) : ces messages ne lui apprennent rien, et
/// `NetRegister` rappellerait l'application (« joueur enregistré ») pour un joueur qui n'est jamais
/// parti. Ils sont jetés quand ils arrivent par la connexion re-liée, pendant `REPETES_MS`.
const REPETES: [&str; 3] = ["NetRegister", "NetAppRegister", "NetSnapshotProcessed"];
const REPETES_MS: u64 = 30_000;

type Traitement = unsafe extern "C-unwind" fn(*mut u8, *mut c_void);

static EXE: AtomicUsize = AtomicUsize::new(0);
static TABLE: AtomicUsize = AtomicUsize::new(usize::MAX);
static SESSION: AtomicUsize = AtomicUsize::new(0);
static ORIG_INTRO: AtomicUsize = AtomicUsize::new(0);
static ORIG_RELAY: AtomicUsize = AtomicUsize::new(0);
static ORIG_REPETES: [AtomicUsize; 3] = [AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0)];
/// Connexions re-liées récemment, et jusqu'à quand leurs messages répétés sont jetés.
static RECENTES: Mutex<Vec<(u32, u64)>> = Mutex::new(Vec::new());

/// La tentative en cours : partenaire, début (ms), et partenaires reliés depuis.
struct Tentative {
    partenaire: i32,
    debut_ms: u64,
}
static TENTATIVE: Mutex<Option<Tentative>> = Mutex::new(None);
/// Bit n : le joueur n a été relié en direct par BGE depuis la dernière tentative (`etat`).
static RELIES: AtomicU64 = AtomicU64::new(0);

fn table() -> Option<&'static Exe> {
    EXES.get(TABLE.load(Ordering::Relaxed))
}

fn maintenant_ms() -> u64 {
    win::qpc_ms(win::qpc()) as u64
}

unsafe fn image(adresse: u64) -> usize {
    EXE.load(Ordering::Relaxed) + (adresse - BASE) as usize
}

unsafe fn lire<T: Copy>(adresse: usize) -> T {
    std::ptr::read_unaligned(adresse as *const T)
}

/// Une session de Civ6 valide (ses accesseurs ont les octets attendus) ?
unsafe fn session_valide(s: usize) -> bool {
    if s == 0 {
        return false;
    }
    let vt: usize = lire(s);
    if vt == 0 {
        return false;
    }
    ACCESSEURS.iter().all(|(entree, octets)| {
        let f: usize = lire(vt + entree);
        f != 0 && std::slice::from_raw_parts(f as *const u8, octets.len()) == octets
    })
}

/// La session de la partie : celle vue au dernier message traité, sinon la première du
/// gestionnaire réseau (table session → objet en + 0x10, nombre de cases en + 0x18).
unsafe fn session() -> Option<usize> {
    let s = SESSION.load(Ordering::Relaxed);
    if session_valide(s) {
        return Some(s);
    }
    let t = table()?;
    let reseau: usize = lire(image(t.reseau));
    if reseau == 0 {
        return None;
    }
    let cases: usize = lire(reseau + 0x10);
    let n: u32 = lire(reseau + 0x18);
    if cases == 0 || n == 0 || n > 4096 {
        return None;
    }
    for i in 0..n as usize {
        let mut e: usize = lire(cases + 8 * i);
        let mut garde = 0;
        while e != 0 && garde < 64 {
            let s: usize = lire(e + 8);
            if session_valide(s) {
                SESSION.store(s, Ordering::Relaxed);
                return Some(s);
            }
            e = lire(e + 0x10);
            garde += 1;
        }
    }
    None
}

unsafe fn objet(s: usize, decalage: usize) -> Option<usize> {
    let o: usize = lire(s + decalage);
    (o != 0).then_some(o)
}

unsafe fn local(s: usize) -> Option<i32> {
    Some(lire(objet(s, SESSION_JOUEURS)? + JOUEURS_LOCAL))
}

unsafe fn relayes(s: usize, a: i32, b: i32) -> bool {
    let (Some(t), Some(r)) = (table(), objet(s, SESSION_RELAIS)) else { return false };
    let f: unsafe extern "C-unwind" fn(usize, u32, i32) -> u8 = std::mem::transmute(image(t.relais_contient));
    f(r, a as u32, b) != 0
}

/// Une table de Civ6 entier → valeur (cases en + 8, nombre de cases en + 0x10 ; entrée : clé + 0,
/// valeur + 8, suivante + 0x10) contient-elle la clé ?
unsafe fn table_contient(t: usize, cle: u32) -> bool {
    let cases: usize = lire(t + 8);
    let n: u32 = lire(t + 0x10);
    if cases == 0 || n == 0 {
        return false;
    }
    let fin: usize = lire(cases + 8 * n as usize);
    let mut e: usize = lire(cases + 8 * (cle % n) as usize);
    let mut garde = 0;
    while e != 0 && e != fin && garde < 4096 {
        if lire::<u32>(e) == cle {
            return true;
        }
        e = lire(e + 0x10);
        garde += 1;
    }
    false
}

/// Enlève `v` d'une liste de partenaires (vecteur d'entiers) ; le nombre restant.
pub fn enlever(liste: &mut Vec<i32>, v: i32) -> usize {
    liste.retain(|&x| x != v);
    liste.len()
}

/// Enlève `b` des partenaires de `a` dans la table des relais (entrée : clé + 0, vecteur + 8 / + 0x10,
/// suivante + 0x20 ; cases en + 8, nombre en + 0x10) ; efface l'entrée de `a` si elle est vide.
unsafe fn retirer_partenaire(r: usize, a: i32, b: i32) {
    let Some(t) = table() else { return };
    let cases: usize = lire(r + 8);
    let n: u32 = lire(r + 0x10);
    if cases == 0 || n == 0 {
        return;
    }
    let mut e: usize = lire(cases + 8 * (a as u32 % n) as usize);
    let mut garde = 0;
    while e != 0 && garde < 4096 {
        if lire::<i32>(e) == a {
            let debut: usize = lire(e + 8);
            let fin: usize = lire(e + 0x10);
            if debut != 0 && fin >= debut {
                let mut liste: Vec<i32> = std::slice::from_raw_parts(debut as *const i32, (fin - debut) / 4).to_vec();
                let reste = enlever(&mut liste, b);
                if reste < (fin - debut) / 4 {
                    std::ptr::copy_nonoverlapping(liste.as_ptr(), debut as *mut i32, reste);
                    *((e + 0x10) as *mut usize) = debut + 4 * reste;
                }
                if reste == 0 {
                    let effacer: unsafe extern "C-unwind" fn(usize, *const u32) -> i32 = std::mem::transmute(image(t.relais_effacer));
                    let cle = a as u32;
                    effacer(r, &cle);
                }
            }
            return;
        }
        e = lire(e + 0x20);
        garde += 1;
    }
}

/// Enlève la paire des deux côtés.
unsafe fn retirer_paire(s: usize, a: i32, b: i32) -> bool {
    let Some(r) = objet(s, SESSION_RELAIS) else { return false };
    let avant = relayes(s, a, b) || relayes(s, b, a);
    retirer_partenaire(r, a, b);
    retirer_partenaire(r, b, a);
    avant
}

/// Les paires de la table des relais, chacune une fois (a < b).
unsafe fn paires(s: usize) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    let Some(r) = objet(s, SESSION_RELAIS) else { return out };
    let cases: usize = lire(r + 8);
    let n: u32 = lire(r + 0x10);
    if cases == 0 || n == 0 || n > 4096 {
        return out;
    }
    let fin: usize = lire(cases + 8 * n as usize);
    for i in 0..n as usize {
        let mut e: usize = lire(cases + 8 * i);
        let mut garde = 0;
        while e != 0 && e != fin && garde < 4096 {
            let a: i32 = lire(e);
            let debut: usize = lire(e + 8);
            let fin_v: usize = lire(e + 0x10);
            if debut != 0 && fin_v >= debut && fin_v - debut < 4 * 64 {
                for &b in std::slice::from_raw_parts(debut as *const i32, (fin_v - debut) / 4) {
                    let p = (a.min(b), a.max(b));
                    if !out.contains(&p) {
                        out.push(p);
                    }
                }
            }
            e = lire(e + 0x20);
            garde += 1;
        }
    }
    out.sort();
    out
}

/// Texte des paires pour le Lua : « a-b,c-d ».
pub fn paires_texte(p: &[(i32, i32)]) -> String {
    p.iter().map(|(a, b)| format!("{a}-{b}")).collect::<Vec<_>>().join(",")
}

/// La connexion en attente en tête de file : (adresse, joueur, état).
unsafe fn tete(s: usize) -> Option<(usize, i32, i32)> {
    let t = table()?;
    let q = objet(s, SESSION_FILE)?;
    let f: unsafe extern "C-unwind" fn(usize) -> usize = std::mem::transmute(image(t.file_tete));
    let p = f(q);
    (p != 0).then(|| (p, lire(p + ATTENTE_JOUEUR), lire(p + ATTENTE_ETAT)))
}

unsafe fn retirer_tete(s: usize, reussie: bool) {
    let (Some(t), Some(q)) = (table(), objet(s, SESSION_FILE)) else { return };
    let f: unsafe extern "C-unwind" fn(usize, u8) = std::mem::transmute(image(t.file_retirer));
    f(q, reussie as u8);
}

/// Introduction détournée. Re-liaison : un joueur relayé avec nous s'introduit sur une connexion
/// qu'aucun joueur n'a, et son joueur réseau n'a pas de connexion. Sinon, le traitement d'origine.
unsafe extern "C-unwind" fn intro(msg: *mut u8, s: *mut c_void) {
    let s = s as usize;
    if session_valide(s) {
        SESSION.store(s, Ordering::Relaxed);
        if !msg.is_null() && relier(msg as usize, s) {
            return;
        }
    }
    let orig: Traitement = std::mem::transmute(ORIG_INTRO.load(Ordering::Relaxed));
    orig(msg, s as *mut c_void)
}

unsafe fn relier(msg: usize, s: usize) -> bool {
    let Some(t) = table() else { return false };
    let Some(moi) = local(s) else { return false };
    let joueur: i32 = lire(msg + MSG_JOUEUR);
    let conn: u32 = lire(msg + MSG_CONNEXION);
    if joueur < 0 || joueur == moi || conn == 0 || !relayes(s, moi, joueur) {
        return false;
    }
    let Some(jm) = objet(s, SESSION_JOUEURS) else { return false };
    if table_contient(jm + JOUEURS_CONNEXIONS, conn) {
        return false; // arrivée par le relais (la connexion de l'hôte)
    }
    let trouver: unsafe extern "C-unwind" fn(usize, u32) -> usize = std::mem::transmute(image(t.joueur_trouver));
    let jr = trouver(jm, joueur as u32);
    if jr == 0 || lire::<u32>(jr + JOUEUR_CONNEXION) != 0 {
        return false;
    }
    // Comme le traitement d'origine : steamID sur la connexion, et notre connexion en attente vers
    // lui retirée (réussie).
    let steam: u64 = lire(msg + MSG_STEAM);
    let reseau: usize = lire(image(t.reseau));
    if reseau != 0 {
        let connexion: unsafe extern "C-unwind" fn(usize, u32) -> usize = std::mem::transmute(image(t.connexion));
        let c = connexion(reseau + 0x88, conn);
        if c != 0 {
            let poser: unsafe extern "C-unwind" fn(usize, u64, u8) = std::mem::transmute(image(t.connexion_steam));
            poser(c, steam, 0);
        }
    }
    if let Some((_, cible, _)) = tete(s) {
        if cible == joueur {
            retirer_tete(s, true);
        }
    }
    // Le joueur réseau existant prend la connexion : plus de remplacement, plus de départ publié.
    *((jr + JOUEUR_CONNEXION) as *mut u32) = conn;
    let case: unsafe extern "C-unwind" fn(usize, *const u32) -> *mut usize = std::mem::transmute(image(t.carte_case));
    let v = case(jm + JOUEURS_CONNEXIONS, &conn);
    if !v.is_null() {
        *v = jr;
    }
    retirer_paire(s, moi, joueur);
    if (0..64).contains(&joueur) {
        RELIES.fetch_or(1u64 << joueur, Ordering::Relaxed);
    }
    {
        let mut recentes = RECENTES.lock().unwrap_or_else(|e| e.into_inner());
        let maintenant = maintenant_ms();
        recentes.retain(|&(c, fin)| c != conn && fin > maintenant);
        recentes.push((conn, maintenant + REPETES_MS));
    }
    log(&format!("reliaison : joueur {joueur} relié en direct (connexion {conn}), paire {moi}-{joueur} retirée du relais"));
    true
}

/// `NetRelay` détourné : une paire déjà relayée est ignorée (renvoyé par l'hôte quand une
/// tentative échoue ; le traiter renverrait les introductions et publierait un départ).
unsafe extern "C-unwind" fn relay(msg: *mut u8, s: *mut c_void) {
    let s = s as usize;
    if session_valide(s) && !msg.is_null() {
        SESSION.store(s, Ordering::Relaxed);
        let a: i32 = lire(msg as usize + MSG_JOUEUR);
        let b: i32 = lire(msg as usize + MSG_RELAY_EXISTANT);
        if relayes(s, a, b) {
            log(&format!("reliaison : NetRelay {a}-{b} déjà connu, ignoré"));
            return;
        }
        log(&format!("reliaison : NetRelay {a}-{b} (lien direct échoué, relais par l'hôte)"));
    }
    let orig: Traitement = std::mem::transmute(ORIG_RELAY.load(Ordering::Relaxed));
    orig(msg, s as *mut c_void)
}

/// Message répété (`REPETES[i]`) arrivé par une connexion re-liée depuis moins de `REPETES_MS` ?
unsafe fn repete(msg: *mut u8) -> bool {
    if msg.is_null() {
        return false;
    }
    let conn: u32 = lire(msg as usize + MSG_CONNEXION);
    let maintenant = maintenant_ms();
    RECENTES.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|&(c, fin)| c == conn && maintenant < fin)
}

unsafe fn traiter_repete(i: usize, msg: *mut u8, s: *mut c_void) {
    if repete(msg) {
        log(&format!("reliaison : {} répété sur la connexion re-liée, jeté", REPETES[i]));
        return;
    }
    let orig: Traitement = std::mem::transmute(ORIG_REPETES[i].load(Ordering::Relaxed));
    orig(msg, s)
}
unsafe extern "C-unwind" fn repete_register(msg: *mut u8, s: *mut c_void) {
    traiter_repete(0, msg, s)
}
unsafe extern "C-unwind" fn repete_app_register(msg: *mut u8, s: *mut c_void) {
    traiter_repete(1, msg, s)
}
unsafe extern "C-unwind" fn repete_snapshot(msg: *mut u8, s: *mut c_void) {
    traiter_repete(2, msg, s)
}

/// Où en est une tentative, vue de la tête de file et de la table des relais.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Etat {
    EnCours,
    Reliee,
    Echec,
}

/// `tete` : joueur et état de la connexion en attente en tête de file ; `relaye` : la paire est
/// encore dans la table ; `relie` : l'introduction a été traitée par `relier`.
pub fn etat(partenaire: i32, tete: Option<(i32, i32)>, relaye: bool, relie: bool, ecoule_ms: u64) -> Etat {
    if relie {
        return Etat::Reliee;
    }
    if !relaye {
        // Sortie de la table sans notre re-liaison : le partenaire est parti.
        return Etat::Echec;
    }
    match tete {
        Some((j, e)) if j == partenaire && e == ETAT_ECHEC => Etat::Echec,
        Some((j, _)) if j == partenaire && ecoule_ms < TENTATIVE_MAX_MS => Etat::EnCours,
        Some((j, _)) if j == partenaire => Etat::Echec,
        _ if ecoule_ms < 2_000 => Etat::EnCours,
        _ => Etat::Echec,
    }
}

/// Ouvre une connexion en attente vers `partenaire`, comme à l'arrivée d'un joueur.
unsafe fn reparer(partenaire: i32, steam: u64) -> Result<(), String> {
    let t = table().ok_or("exe non reconnu")?;
    let s = session().ok_or("pas de session")?;
    let moi = local(s).ok_or("pas de joueur local")?;
    if partenaire < 0 || partenaire == moi {
        return Err(format!("partenaire {partenaire} invalide"));
    }
    if !relayes(s, moi, partenaire) {
        return Err(format!("{moi}-{partenaire} n'est pas relayé"));
    }
    if steam == 0 {
        return Err("steamID inconnu".into());
    }
    let mut tentative = TENTATIVE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(en_cours) = tentative.as_ref() {
        if maintenant_ms().saturating_sub(en_cours.debut_ms) < TENTATIVE_MAX_MS {
            return Err(format!("tentative vers {} en cours", en_cours.partenaire));
        }
    }
    // Une connexion échouée laissée en tête (à l'arrivée du joueur, ou notre tentative d'avant)
    // bloquerait la file : on la retire, comme la réponse que l'hôte n'envoie pas.
    if let Some((_, j, e)) = tete(s) {
        if e == ETAT_ECHEC {
            retirer_tete(s, false);
            log(&format!("reliaison : connexion échouée vers {j} retirée de la tête de file"));
        } else {
            return Err(format!("file occupée (joueur {j}, état {e})"));
        }
    }
    let mut msg = [0u8; MSG_TAILLE];
    std::ptr::write_unaligned(msg.as_mut_ptr().add(MSG_JOUEUR) as *mut i32, partenaire);
    std::ptr::write_unaligned(msg.as_mut_ptr().add(MSG_STEAM) as *mut u64, steam);
    RELIES.fetch_and(!(1u64 << (partenaire & 63)), Ordering::Relaxed);
    *tentative = Some(Tentative { partenaire, debut_ms: maintenant_ms() });
    drop(tentative);
    let f: Traitement = std::mem::transmute(image(t.peer_info.1[TRAITER]));
    f(msg.as_mut_ptr(), s as *mut c_void);
    log(&format!("reliaison : tentative de lien direct {moi}-{partenaire} (steamID {steam})"));
    Ok(())
}

/// L'état de la tentative vers `partenaire` ; une connexion échouée en tête est retirée.
unsafe fn suivre(partenaire: i32) -> Etat {
    let Some(s) = session() else { return Etat::Echec };
    let Some(moi) = local(s) else { return Etat::Echec };
    let debut = TENTATIVE.lock().unwrap_or_else(|e| e.into_inner()).as_ref().filter(|t| t.partenaire == partenaire).map(|t| t.debut_ms);
    let Some(debut) = debut else { return Etat::Echec };
    let tete_file = tete(s);
    let relie = RELIES.load(Ordering::Relaxed) & (1u64 << (partenaire & 63)) != 0;
    let e = etat(partenaire, tete_file.map(|(_, j, e)| (j, e)), relayes(s, moi, partenaire), relie, maintenant_ms().saturating_sub(debut));
    if e != Etat::EnCours {
        if e == Etat::Echec {
            if let Some((_, j, _)) = tete_file {
                if j == partenaire {
                    retirer_tete(s, false);
                }
            }
        }
        *TENTATIVE.lock().unwrap_or_else(|e| e.into_inner()) = None;
        log(&format!("reliaison : tentative {moi}-{partenaire} : {}", if e == Etat::Reliee { "lien direct" } else { "échec, la paire reste relayée" }));
    }
    e
}

/// `Civ6Proxy_Relais()` → les paires relayées de la partie, « a-b,c-d » (vide sans relais).
pub(crate) unsafe extern "C-unwind" fn lua_paires(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let texte = session().map(|s| paires_texte(&paires(s))).unwrap_or_default();
    crate::options::push_str(h, l, &texte);
    1
}

fn arg_entier(l: *mut c_void, i: i32) -> Option<i64> {
    // SAFETY: lecture d'un argument Lua par l'API du jeu.
    unsafe { crate::salon::arg_str(l, i) }.and_then(|v| v.trim().split('.').next().and_then(|x| x.parse().ok()))
}

/// `Civ6Proxy_RelaisReparer(partenaire, steamID)` → 1 si la tentative est lancée, sinon 0.
pub(crate) unsafe extern "C-unwind" fn lua_reparer(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let partenaire = arg_entier(l, 1).unwrap_or(-1) as i32;
    let steam = crate::salon::arg_str(l, 2).and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0);
    let r = reparer(partenaire, steam);
    if let Err(e) = &r {
        log(&format!("reliaison : pas de tentative vers {partenaire} : {e}"));
    }
    (h.push_integer)(l, r.is_ok() as i32);
    1
}

/// `Civ6Proxy_RelaisEtat(partenaire)` → 0 en cours, 1 relié en direct, 2 échec.
pub(crate) unsafe extern "C-unwind" fn lua_etat(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let partenaire = arg_entier(l, 1).unwrap_or(-1) as i32;
    let e = match suivre(partenaire) {
        Etat::EnCours => 0,
        Etat::Reliee => 1,
        Etat::Echec => 2,
    };
    (h.push_integer)(l, e);
    1
}

/// `Civ6Proxy_RelaisRetirer(a, b)` → 1 si la paire était dans la table locale (retirée), sinon 0.
/// Jamais une paire dont nous sommes : l'annonce peut arriver avant l'introduction du partenaire,
/// et `relier` ne reconnaîtrait plus la re-liaison (le chemin de Civ6 publierait un départ). Notre
/// paire sort de la table quand l'introduction arrive.
pub(crate) unsafe extern "C-unwind" fn lua_retirer(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let (Some(a), Some(b)) = (arg_entier(l, 1), arg_entier(l, 2)) else { return 0 };
    let tiers = |s: usize| local(s).is_some_and(|moi| moi != a as i32 && moi != b as i32);
    let retiree = session().is_some_and(|s| tiers(s) && retirer_paire(s, a as i32, b as i32));
    if retiree {
        log(&format!("reliaison : paire {a}-{b} retirée (annoncée reliée)"));
    }
    (h.push_integer)(l, retiree as i32);
    1
}

/// Les octets de la table sont-ils tous là ? `lire(adresse, n)` lit l'image, `entree(adresse)` une
/// entrée de vtable ramenée à la base `0x140000000`. Commun à l'exe chargé et aux fichiers (tests).
fn empreinte<'a>(t: &Exe, lire: impl Fn(u64, usize) -> &'a [u8], entree: impl Fn(u64) -> u64) -> Result<(), String> {
    let vtables = [
        ("NetIntroductionSteam", &t.intro),
        ("NetRelay", &t.relay),
        ("NetPeerInfoSteam", &t.peer_info),
        (REPETES[0], &t.repetes[0]),
        (REPETES[1], &t.repetes[1]),
        (REPETES[2], &t.repetes[2]),
    ];
    for (nom, (vtable, entrees)) in vtables {
        for (k, attendue) in entrees.iter().enumerate() {
            if entree(vtable + 8 * k as u64) != *attendue {
                return Err(format!("{nom} : vtable inattendue en {vtable:#x}"));
            }
        }
    }
    let fonctions = [t.file_tete, t.file_retirer, t.connexion, t.connexion_steam, t.joueur_trouver, t.carte_case, t.relais_effacer, t.relais_contient];
    for ((nom, octets), adresse) in PROLOGUES.iter().zip(fonctions) {
        if lire(adresse, octets.len()) != octets {
            return Err(format!("{nom} : octets inattendus en {adresse:#x}"));
        }
    }
    // `mov rcx,[rip+d]` dans le traitement de l'introduction : il lit le gestionnaire réseau.
    let at = t.intro.1[TRAITER] + t.reseau_lu_a;
    let m = lire(at, 7);
    if m.len() < 7 || m[..3] != [0x48, 0x8b, 0x0d] {
        return Err(format!("lecture du gestionnaire réseau absente en {at:#x}"));
    }
    let d = i32::from_le_bytes(m[3..7].try_into().unwrap());
    if (at + 7).wrapping_add_signed(i64::from(d)) != t.reseau {
        return Err(format!("gestionnaire réseau : ne mène pas en {:#x}", t.reseau));
    }
    Ok(())
}

unsafe fn verifier(exe: usize, t: &Exe) -> Result<(), String> {
    let image = |adresse: u64| exe + (adresse - BASE) as usize;
    empreinte(
        t,
        |adresse, n| std::slice::from_raw_parts(image(adresse) as *const u8, n),
        |adresse| (*(image(adresse) as *const usize)).wrapping_sub(exe) as u64 + BASE,
    )
}

unsafe fn detourner(exe: usize, vtable: u64, detour: usize, orig: &AtomicUsize) -> Result<(), String> {
    let entree = (exe + (vtable - BASE) as usize + 8 * TRAITER) as *mut usize;
    let mut ancien = 0u32;
    if win::VirtualProtect(entree as *mut c_void, 8, win::PAGE_EXECUTE_READWRITE, &mut ancien) == 0 {
        return Err("VirtualProtect a échoué".into());
    }
    orig.store(*entree, Ordering::Relaxed);
    *entree = detour;
    win::VirtualProtect(entree as *mut c_void, 8, ancien, &mut ancien);
    Ok(())
}

// Mode test (builds de diagnostic seulement) : `reliaison_test=<joueur>` dans rollover_proxy.cfg du
// joueur qui rejoint la partie fait échouer son lien vers ce joueur, par le vrai chemin d'échec de
// Civ6 (la connexion en attente passe à l'état 10 : la file appelle l'échec, `NetPeerFailure`,
// l'hôte relaie la paire). De quoi essayer la re-liaison sans attendre un vrai échec. Une fois par
// lancement ; jamais le lien vers l'hôte (son échec annulerait l'arrivée).
#[cfg(feature = "diag")]
static TEST_CIBLE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);
#[cfg(feature = "diag")]
static ORIG_PEER_INFO: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "diag")]
const ETAT_ABANDON: i32 = 10;
#[cfg(feature = "diag")]
const ATTENTE_HOTE: usize = 0x58;

/// Le réglage `reliaison_test` (dernière occurrence) : le joueur dont le lien doit échouer.
#[cfg_attr(not(feature = "diag"), allow(dead_code))]
pub fn test_cible(cfg: &str) -> Option<i32> {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("reliaison_test=")).last().and_then(|v| v.trim().parse().ok()).filter(|&j| j >= 0)
}

#[cfg(feature = "diag")]
unsafe extern "C-unwind" fn peer_info(msg: *mut u8, s: *mut c_void) {
    let orig: Traitement = std::mem::transmute(ORIG_PEER_INFO.load(Ordering::Relaxed));
    orig(msg, s);
    let cible = TEST_CIBLE.load(Ordering::Relaxed);
    let s = s as usize;
    if cible < 0 || msg.is_null() || lire::<i32>(msg as usize + MSG_JOUEUR) != cible || !session_valide(s) {
        return;
    }
    let Some(q) = objet(s, SESSION_FILE) else { return };
    let (debut, fin): (usize, usize) = (lire(q + 0x10), lire(q + 0x18));
    if fin < debut + 8 {
        return;
    }
    let attente: usize = lire(fin - 8);
    if attente == 0 || lire::<i32>(attente + ATTENTE_JOUEUR) != cible || lire::<u8>(attente + ATTENTE_HOTE) != 0 {
        return;
    }
    *((attente + ATTENTE_ETAT) as *mut i32) = ETAT_ABANDON;
    TEST_CIBLE.store(-1, Ordering::Relaxed);
    log(&format!("reliaison : test : lien vers le joueur {cible} marqué échoué, l'hôte va relayer la paire"));
}

/// Le réglage `reliaison` de `rollover_proxy.cfg` (dernière occurrence) : `off` ne pose rien.
pub fn actif(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("reliaison=")).last().map(str::trim) != Some("off")
}

/// Appelée par DllMain : aucun code de l'exe n'a encore tourné.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("reliaison : non posé (actif=off)");
        return;
    }
    if !actif(&cfg) {
        log("reliaison : désactivé (reliaison=off)");
        return;
    }
    let exe = win::GetModuleHandleW(std::ptr::null()) as usize;
    let mut raisons = Vec::new();
    let Some(i) = EXES.iter().position(|t| verifier(exe, t).map_err(|e| raisons.push(format!("{} : {e}", t.nom))).is_ok()) else {
        log(&format!("reliaison : exe non reconnu ({}), re-liaison absente", raisons.join(" ; ")));
        return;
    };
    EXE.store(exe, Ordering::Relaxed);
    TABLE.store(i, Ordering::Relaxed);
    let t = &EXES[i];
    let repetes: [Traitement; 3] = [repete_register, repete_app_register, repete_snapshot];
    let r = detourner(exe, t.relay.0, relay as Traitement as usize, &ORIG_RELAY)
        .and_then(|_| (0..3).try_for_each(|i| detourner(exe, t.repetes[i].0, repetes[i] as usize, &ORIG_REPETES[i])))
        .and_then(|_| detourner(exe, t.intro.0, intro as Traitement as usize, &ORIG_INTRO));
    match r {
        Ok(()) => log(&format!("reliaison : re-liaison des paires relayées posée ({})", t.nom)),
        Err(e) => {
            log(&format!("reliaison : {e}, re-liaison absente"));
            return;
        }
    }
    #[cfg(feature = "diag")]
    if let Some(cible) = test_cible(&cfg) {
        TEST_CIBLE.store(cible, Ordering::Relaxed);
        match detourner(exe, t.peer_info.0, peer_info as Traitement as usize, &ORIG_PEER_INFO) {
            Ok(()) => log(&format!("reliaison : test : le lien vers le joueur {cible} échouera à l'arrivée")),
            Err(e) => log(&format!("reliaison : test : {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";
    const FICHIERS: [&str; 2] = ["CivilizationVI_DX12.exe", "CivilizationVI.exe"];

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

    fn empreinte_fichier(t: &Exe, fichier: &[u8]) -> Result<(), String> {
        let lire = |adresse: u64, n: usize| offset(fichier, adresse).map_or(&[][..], |o| &fichier[o..][..n]);
        empreinte(t, lire, |adresse| u64::from_le_bytes(lire(adresse, 8).try_into().unwrap_or_default()))
    }

    /// Chaque table correspond à son exe installé, et à lui seul.
    #[test]
    fn octets_dans_les_exes() {
        for (i, nom) in FICHIERS.iter().enumerate() {
            let Ok(fichier) = std::fs::read(format!(r"{EXE_DIR}\{nom}")) else {
                eprintln!("{nom} absent, test sauté");
                continue;
            };
            for (j, t) in EXES.iter().enumerate() {
                let r = empreinte_fichier(t, &fichier);
                if i == j {
                    assert_eq!(r, Ok(()), "table {} sur {nom}", t.nom);
                } else {
                    assert!(r.is_err(), "la table {} ne doit pas passer sur {nom}", t.nom);
                }
            }
        }
    }

    #[test]
    fn reglage() {
        assert!(actif(""));
        assert!(!actif("reliaison=off\r\n"));
        assert!(actif("reliaison=off\nreliaison=on\n"), "la dernière occurrence compte");
    }

    #[test]
    fn reglage_de_test() {
        assert_eq!(test_cible(""), None);
        assert_eq!(test_cible("reliaison_test=8\r\n"), Some(8));
        assert_eq!(test_cible("reliaison_test=8\nreliaison_test=\n"), None, "vidé : plus de test");
        assert_eq!(test_cible("reliaison_test=-1\n"), None);
    }

    #[test]
    fn texte_des_paires() {
        assert_eq!(paires_texte(&[]), "");
        assert_eq!(paires_texte(&[(6, 8), (7, 8)]), "6-8,7-8");
    }

    #[test]
    fn retrait_d_un_partenaire() {
        let mut v = vec![8, 3, 8];
        assert_eq!(enlever(&mut v, 8), 1);
        assert_eq!(v, vec![3]);
        assert_eq!(enlever(&mut v, 5), 1, "absent : rien ne change");
        assert_eq!(enlever(&mut v, 3), 0);
    }

    #[test]
    fn suivi_d_une_tentative() {
        // Introduction traitée par `relier` : relié.
        assert_eq!(etat(8, Some((8, 2)), true, true, 1_000), Etat::Reliee);
        assert_eq!(etat(8, None, false, true, 1_000), Etat::Reliee);
        // Paire sortie de la table sans re-liaison (partenaire parti) : échec, pas « réparé ».
        assert_eq!(etat(8, None, false, false, 1_000), Etat::Echec);
        // En tête, en cours.
        assert_eq!(etat(8, Some((8, 2)), true, false, 5_000), Etat::EnCours);
        // En tête, échouée (transport : 20 s sans réponse).
        assert_eq!(etat(8, Some((8, ETAT_ECHEC)), true, false, 21_000), Etat::Echec);
        // Trop long.
        assert_eq!(etat(8, Some((8, 2)), true, false, TENTATIVE_MAX_MS), Etat::Echec);
        // Plus en tête et toujours relayé : retirée sans lien (réponse de l'hôte).
        assert_eq!(etat(8, None, true, false, 3_000), Etat::Echec);
        // Juste lancée : la file ne l'a peut-être pas encore prise.
        assert_eq!(etat(8, None, true, false, 500), Etat::EnCours);
    }
}
