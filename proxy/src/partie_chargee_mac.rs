//! Une partie chargée calcule comme la partie en cours, sous macOS : le pendant de
//! partie_chargee.rs (voir son en-tête : `visibilite_chargee`, `ecouteurs_charges`,
//! `preferences_chargees`). Correctif POL, les trois sous-correctifs toujours actifs (pas de
//! réglage, comme la release Windows).
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image :
//! - **Améliorations mémorisées.** Setter `0xa84b5c(gestionnaire, joueur, case, amélioration)` ->
//!   vrai si changée (Windows `18035de10` ; couche `+0x18`, instance partagée 1000 en `+0x50`) et
//!   getter `0xa84a78(gestionnaire, joueur, case)` -> `short` signé, -1 sans couche (Windows
//!   `18035b4a0`). Le setter n'a que deux appels, dans `0x737e8c(?, case)` : `0xe5b20b82`, setter
//!   pour chaque joueur qui a révélé la case, puis pour 1000, puis getter du joueur local et
//!   `0xf391d7b3` s'ils sont égaux — l'étape de Windows (`18025cb50`), mise à part par clang et
//!   appelée par l'amélioration posée (`0x737968`) et par l'étape de la passe (`0x73c74c`, toutes
//!   les cases). Passe post-chargement à quatre étapes : `0x82eae8` (Windows `18063def0`), seuls
//!   appelants `0x42cc2c` / `0x42cf54` (entrées de vtable).
//! - **Ordre des écouteurs.** Lecture d'un joueur `0x9e5840(flux, joueur)` -> flux (boucle du
//!   chargement `0x9e84fc`, Windows `1803037b0`), qui finit par `0x9bec6c(joueur)` : connexion des
//!   écouteurs du joueur `0x97e8e8(objet de mise en place [joueur+0x798], jeu [0x1390218])`
//!   (Windows `1802d6c40`, `Player+0x790`). Gestionnaire des cités libres : connexion
//!   `0x5619b8(gestionnaire)` (Windows `1801ad4a0`), appelée par sa lecture `0x5617d8` et sa mise
//!   en place `0x562638` ; joueur des cités libres en `+0xf0` (Windows `+0xe8`). Les événements du
//!   jeu sont en `[jeu+0x288]` (Windows `+0x280`), objet de `0x19e0` octets (`0x42c438`), espacés de
//!   0x18 : vtable, anneau `+8`, compteur `+0x10`. L'anneau est celui de Windows (Simple::Signal) :
//!   sentinelle créée par `0x55384` (suivant = précédent = elle-même), nœud de 0x38 octets ajouté à
//!   la fin par `0x55504` : suivant `+0`, précédent `+8`, `std::function` libc++ `+0x10`,
//!   compte de références `+0x30` (Windows `+0x50`), 1 pour un écouteur simplement connecté.
//!   Joueur par id : `0x3bbf18(id)` (0 hors bornes), id en `Player+0xe0`.
//! - **Préférences d'IA.** Lecture du grand stratège `0x714194(grand stratège, flux)` (seul
//!   appelant `0x714168`, depuis la lecture du joueur, Windows `1804b5560`). Elle finit par la mise
//!   en place `0x717e0c` (Windows `1804b39a0`), qui écrit le profil d'IA dans le tableau de
//!   préférences en `+0x430` ; l'initialisation d'une nouvelle partie `0x7178a0` (Windows
//!   `1804b44a0`) le remet à zéro juste après (`0x717bfc`). IndexList : données `+0x430`, nombre
//!   `+0x440` (`u32`, `+0x10` dans `0x2bdaf0`), valeur par défaut `+0x448` ; joueur en `+0x98`.
//!   Tout est Windows + 8.
//!
//! En x86_64 (Mac Intel, Rosetta), compilation à part, même découpage sauf l'écriture de la
//! mémoire d'une case, coupée autrement par clang. Relevé depuis les mêmes constantes et appelants :
//! - passe post-chargement `0x80a9a0` (appelants `0x4251c8` / `0x425498`), étape des améliorations
//!   `0x749b20` (appelée en `0x80a9cb`) -> `0x744b40(?, case)` (`0xe5b20b82`, setter des joueurs,
//!   1000, getter du joueur local, puis `0xe5b20b82 + 0xddfcc31` = `0xf391d7b3`), aussi appelée par
//!   l'amélioration posée (`0x7447ff`) ; setter `0xa89270` (deux appels seulement, `0x744bf6` et
//!   `0x744c16` ; rend `al`), getter `0xa891a0` ;
//! - lecture d'un joueur `0x9de0d0(flux, joueur)` (appelée en `0x9e13e3`), qui finit par
//!   `0x9b5ce0(joueur)` (`0x9e0914`) : `0x9711a0([joueur+0x798], [0x13dc0b0])` ;
//! - cités libres : connexion `0x55a810`, appelée par la lecture (`0x55a7eb`) et la mise en place
//!   (`0x55b561`), joueur en `+0xf0`, id en `Player+0xe0` ;
//! - grand stratège : `0x71e490` (appelée par la lecture du joueur en `0x9dfe67`) -> lecture
//!   `0x71e4b0(grand stratège, flux)`, qui finit par la mise en place `0x722650` (`0x71eb16`) ;
//!   l'initialisation `0x722070` la suit (`0x72208e`) de la remise à zéro (`+0x448` en `0x72236c`) ;
//!   IndexList::assign `0x2a4a40` (nombre `+0x10`) ;
//! - jeu `[0x13dc0b0]`, événements `[jeu+0x288]` (objet de `0x19e0` octets, `0x424976`), pas 0x18.
//!   **Nœud d'écouteur de 0x50 octets** (`0x433ed0` crée la sentinelle, `0x55e730` ajoute à la fin) :
//!   suivant `+0`, précédent `+8`, `std::function` `+0x10`, compte de références **`+0x40`**.
//!   Joueur par id `0x3ae880(id)`.
//! Le saut posé (`jmp [rip+0]`) prend 14 octets : chaque trampoline recopie des instructions entières
//! sans adresse relative (`push`, `sub rsp`, `mov` entre registres ; 15 ou 20 octets).

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]
// Les relevés x86_64 ne servent, en arm64, qu'aux tests qui lisent le jeu installé.
#![cfg_attr(target_arch = "aarch64", allow(dead_code))]

use super::journal::log;
use super::{patch, pol};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    /// FNV-1a 64 du corps entier.
    empreinte: u64,
}

const POST_LOAD: Fonction = Fonction { nom: "passe post-chargement", adresse: 0x82eae8, taille: 0x5c, empreinte: 0x3adb_2604_c57e_6c2d };
const SET_REMEMBERED: Fonction = Fonction { nom: "écriture d'amélioration mémorisée", adresse: 0xa84b5c, taille: 0x8c, empreinte: 0x5937_0cc7_f668_8feb };
const PLAYER_READ: Fonction = Fonction { nom: "lecture d'un joueur", adresse: 0x9e5840, taille: 0x28f4, empreinte: 0xc4a8_0981_5992_7f4f };
const PLAYER_CONNECT: Fonction = Fonction { nom: "écouteurs du joueur", adresse: 0x97e8e8, taille: 0x1b48, empreinte: 0x7f15_a062_4735_5c29 };
const FC_CONNECT: Fonction = Fonction { nom: "écouteurs des cités libres", adresse: 0x5619b8, taille: 0x59c, empreinte: 0x0196_5da5_e800_4241 };
const GS_READ: Fonction = Fonction { nom: "lecture du grand stratège", adresse: 0x714194, taille: 0x734, empreinte: 0x7b6e_7f72_4fd9_dd9f };
/// Détournées, dans cet ordre (indices des trampolines).
const DETOURS: [Fonction; 6] = [POST_LOAD, SET_REMEMBERED, PLAYER_READ, PLAYER_CONNECT, FC_CONNECT, GS_READ];

const GET_REMEMBERED: Fonction = Fonction { nom: "lecture d'amélioration mémorisée", adresse: 0xa84a78, taille: 0x94, empreinte: 0xfca6_0575_cb80_adc3 };
const PLAYER_BY_ID: Fonction = Fonction { nom: "joueur par id", adresse: 0x3bbf18, taille: 0x3c, empreinte: 0xc130_b171_60ee_f6fa };
/// Appelées par le proxy, ou dont le comportement fonde la règle : vérifiées, pas détournées.
const VERIFIEES: [Fonction; 8] = [
    GET_REMEMBERED,
    PLAYER_BY_ID,
    Fonction { nom: "mémoire d'une case (setters, puis getter du joueur local)", adresse: 0x737e8c, taille: 0x190, empreinte: 0x7fe6_0a18_b861_0d57 },
    Fonction { nom: "étape des améliorations", adresse: 0x73c74c, taille: 0x60, empreinte: 0x1078_01ac_5b8a_9386 },
    Fonction { nom: "connexion du joueur ([joueur+0x798], [0x1390218])", adresse: 0x9bec6c, taille: 0x14, empreinte: 0x9714_3f58_4033_0cc3 },
    Fonction { nom: "anneau d'un événement", adresse: 0x55384, taille: 0x180, empreinte: 0x25e3_ddf1_2846_2f0b },
    Fonction { nom: "ajout d'un écouteur", adresse: 0x55504, taille: 0xe0, empreinte: 0xb26c_e1c1_2831_cdbe },
    Fonction { nom: "IndexList::assign", adresse: 0x2bdaf0, taille: 0x110, empreinte: 0xa806_ec5a_b7a4_e011 },
];
/// Mots relus ailleurs : (adresse, mot arm64, ce qu'il montre).
const DISPOSITION: &[(usize, u32, &str)] = &[
    (0x42c438, 0x5283_3c00, "mov w0, #0x19e0 : taille de l'objet des événements"),
    (0x562664, 0xb940_e101, "ldr w1, [x8, #0xe0] : id du joueur en +0xe0"),
    (0x82eb10, 0x97fc_370f, "bl 0x73c74c : l'étape des améliorations dans la passe"),
    (0x9e80b4, 0x97ff_5aee, "bl 0x9bec6c : connexion à la fin de la lecture du joueur"),
    (0x561998, 0x9400_0008, "bl 0x5619b8 : connexion à la fin de la lecture des cités libres"),
    (0x562678, 0x17ff_fcd0, "b 0x5619b8 : connexion à la mise en place des cités libres"),
    (0x714180, 0x9400_0005, "bl 0x714194 : lecture du grand stratège"),
];
/// Prologue recopié dans les trampolines : quatre instructions de pile, sans adresse relative.
const PROLOGUE: usize = 16;

/// x86_64 : les mêmes fonctions (voir l'en-tête), dans l'ordre de `DETOURS` et de `VERIFIEES`.
const X86_DETOURS: [Fonction; 6] = [
    Fonction { nom: "passe post-chargement", adresse: 0x80a9a0, taille: 0x70, empreinte: 0x3c73_f20f_9eb8_c108 },
    Fonction { nom: "écriture d'amélioration mémorisée", adresse: 0xa89270, taille: 0x80, empreinte: 0x597c_f9eb_c24e_25af },
    Fonction { nom: "lecture d'un joueur", adresse: 0x9de0d0, taille: 0x2e50, empreinte: 0x8c59_6ebe_2231_3aea },
    Fonction { nom: "écouteurs du joueur", adresse: 0x9711a0, taille: 0x2010, empreinte: 0x6545_7328_83e0_cba7 },
    Fonction { nom: "écouteurs des cités libres", adresse: 0x55a810, taille: 0x640, empreinte: 0x7e52_80fd_70e2_15e5 },
    Fonction { nom: "lecture du grand stratège", adresse: 0x71e4b0, taille: 0x8d0, empreinte: 0x3c97_199d_f1ca_bfb6 },
];
const X86_GET_REMEMBERED: Fonction = Fonction { nom: "lecture d'amélioration mémorisée", adresse: 0xa891a0, taille: 0x80, empreinte: 0x02ef_0125_c179_c023 };
const X86_PLAYER_BY_ID: Fonction = Fonction { nom: "joueur par id", adresse: 0x3ae880, taille: 0x40, empreinte: 0x38df_57d2_aa82_0224 };
const X86_VERIFIEES: [Fonction; 10] = [
    X86_GET_REMEMBERED,
    X86_PLAYER_BY_ID,
    Fonction { nom: "mémoire d'une case (setters, puis getter du joueur local)", adresse: 0x744b40, taille: 0x180, empreinte: 0xac02_8784_e764_cda5 },
    Fonction { nom: "étape des améliorations", adresse: 0x749b20, taille: 0x50, empreinte: 0xc60d_9a86_801b_e874 },
    Fonction { nom: "connexion du joueur ([joueur+0x798], [0x13dc0b0])", adresse: 0x9b5ce0, taille: 0x20, empreinte: 0x596a_db08_e515_bef1 },
    Fonction { nom: "anneau d'un événement", adresse: 0x433ed0, taille: 0x150, empreinte: 0xfe97_1658_206f_d4cd },
    Fonction { nom: "ajout d'un écouteur", adresse: 0x55e730, taille: 0xe0, empreinte: 0x71ae_4c24_9321_39e6 },
    Fonction { nom: "IndexList::assign", adresse: 0x2a4a40, taille: 0xf0, empreinte: 0x1a06_9bd8_8df9_4c75 },
    Fonction { nom: "lecture du grand stratège (appelante)", adresse: 0x71e490, taille: 0x20, empreinte: 0x7f11_a756_b8d2_01a3 },
    Fonction { nom: "mise en place du grand stratège", adresse: 0x722650, taille: 0x660, empreinte: 0x6aba_3162_8eb9_727b },
];
/// Prologues recopiés (instructions entières, sans adresse relative, au moins 14 octets).
const X86_PROLOGUES: [&[u8]; 6] = [
    // push rbp ; mov rbp, rsp ; push r15 ; push r14 ; push rbx ; push rax ; mov rbx, rdi ; mov rdi, [rdi+0x288]
    &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x53, 0x50, 0x48, 0x89, 0xfb, 0x48, 0x8b, 0xbf, 0x88, 0x02, 0x00, 0x00],
    // push rbp ; mov rbp, rsp ; push r15 ; push r14 ; push rbx ; push rax ; mov ebx, ecx ; mov r14d, edx
    &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x53, 0x50, 0x89, 0xcb, 0x41, 0x89, 0xd6],
    // push rbp ; mov rbp, rsp ; push r15..r12 ; push rbx ; sub rsp, imm32
    &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec, 0x28, 0x03, 0x00, 0x00],
    &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec, 0xb8, 0x06, 0x00, 0x00],
    &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec, 0x68, 0x01, 0x00, 0x00],
    &[0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec, 0x88, 0x01, 0x00, 0x00],
];
/// Octets relus ailleurs : (adresse, octets x86_64, ce qu'ils montrent).
const X86_DISPOSITION: &[(usize, &[u8], &str)] = &[
    (0x424976, &[0xbf, 0xe0, 0x19, 0x00, 0x00], "mov edi, 0x19e0 : taille de l'objet des événements"),
    (0x4249a2, &[0x4c, 0x89, 0xa3, 0x88, 0x02, 0x00, 0x00], "mov [rbx+0x288], r12 : événements en jeu+0x288"),
    (0x80a9ad, &[0x48, 0x8b, 0xbf, 0x88, 0x02, 0x00, 0x00], "mov rdi, [rdi+0x288] : la passe lit les événements du jeu"),
    (0x9b5cee, &[0x48, 0x8b, 0xbf, 0x98, 0x07, 0x00, 0x00], "mov rdi, [rdi+0x798] : objet de mise en place du joueur"),
    (0x55b539, &[0x4c, 0x89, 0xb3, 0xf0, 0x00, 0x00, 0x00], "mov [rbx+0xf0], r14 : joueur des cités libres en +0xf0"),
    (0x55b54c, &[0x8b, 0xb1, 0xe0, 0x00, 0x00, 0x00], "mov esi, [rcx+0xe0] : id du joueur en +0xe0"),
    (0x55a8c5, &[0x4d, 0x8d, 0xa5, 0xc0, 0x03, 0x00, 0x00], "lea r12, [r13+0x3c0] : un événement (pas 0x18)"),
    (0x55a8fa, &[0x41, 0xff, 0x85, 0xd0, 0x03, 0x00, 0x00], "inc [r13+0x3d0] : compteur de l'événement en +0x10"),
    (0x55a909, &[0x49, 0x8b, 0xbd, 0xc8, 0x03, 0x00, 0x00], "mov rdi, [r13+0x3c8] : anneau de l'événement en +8"),
    (0x433f0f, &[0xbf, 0x50, 0x00, 0x00, 0x00], "mov edi, 0x50 : sentinelle de 0x50 octets"),
    (0x433f6c, &[0x41, 0xc7, 0x47, 0x40, 0x01, 0x00, 0x00, 0x00], "mov [r15+0x40], 1 : compte de références de la sentinelle"),
    (0x55e756, &[0xbf, 0x50, 0x00, 0x00, 0x00], "mov edi, 0x50 : nœud de 0x50 octets"),
    (0x55e7ad, &[0x41, 0xc7, 0x46, 0x40, 0x01, 0x00, 0x00, 0x00], "mov [r14+0x40], 1 : compte 1 d'un écouteur connecté"),
    (0x55e7b5, &[0x48, 0x8b, 0x43, 0x08, 0x49, 0x89, 0x46, 0x08, 0x49, 0x89, 0x1e], "précédent +8, suivant +0 : ajout en fin d'anneau"),
    (0x7229fb, &[0x4c, 0x8d, 0xb3, 0x30, 0x04, 0x00, 0x00], "lea r14, [rbx+0x430] : la mise en place écrit les préférences"),
    (0x722869, &[0x48, 0x8b, 0xbb, 0x98, 0x00, 0x00, 0x00], "mov rdi, [rbx+0x98] : joueur du grand stratège"),
    (0x72236c, &[0xc7, 0x83, 0x48, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], "mov [rbx+0x448], 0 : remise à zéro de l'initialisation"),
    (0x2a4ace, &[0x41, 0x8b, 0x47, 0x10], "mov eax, [r15+0x10] : nombre de l'IndexList en +0x10"),
];
/// `call`/`jmp rel32` relus : (site, cible).
const X86_APPELS: &[(usize, usize)] = &[
    (0x4251c8, 0x80a9a0),
    (0x425498, 0x80a9a0),
    (0x80a9cb, 0x749b20),
    (0x749b4f, 0x744b40),
    (0x744bf6, 0xa89270),
    (0x744c16, 0xa89270),
    (0x744c3e, 0xa891a0),
    (0x9e13e3, 0x9de0d0),
    (0x9e0914, 0x9b5ce0),
    (0x9b5cf6, 0x9711a0),
    (0x55a7eb, 0x55a810),
    (0x55b561, 0x55a810),
    (0x55a904, 0x433ed0),
    (0x55a917, 0x55e730),
    (0x9dfe67, 0x71e490),
    (0x71e49f, 0x71e4b0),
    (0x71eb16, 0x722650),
    (0x72208e, 0x722650),
];
/// Le jeu en x86_64 : `lea rax, [rip + …]` de `0x9b5ce4` (disp32 en `0x9b5ce7`) et `mov rax,
/// [rip + …]` de `0x3ae884` (disp32 en `0x3ae887`).
const X86_GAME: usize = 0x13dc0b0;
const X86_GAME_DISP: [usize; 2] = [0x9b5ce7, 0x3ae887];

/// Le jeu (`[0x1390218]`, lu par `0x9bec6c`) : `+0x288` son objet d'événements.
const GAME: usize = if cfg!(target_arch = "x86_64") { X86_GAME } else { 0x1390218 };
const GAME_EVENTS: usize = 0x288;
const EVENTS_SIZE: usize = 0x19e0;
/// Nœud d'écouteur : 0x38 octets et compte en +0x30 (arm64), 0x50 et +0x40 (x86_64).
const NODE_SIZE: usize = if cfg!(target_arch = "x86_64") { 0x50 } else { 0x38 };
const NODE_REFS: usize = if cfg!(target_arch = "x86_64") { 0x40 } else { 0x30 };
const PLAYER_BY_ID_AT: usize = if cfg!(target_arch = "x86_64") { X86_PLAYER_BY_ID.adresse } else { PLAYER_BY_ID.adresse };
const GET_REMEMBERED_AT: usize = if cfg!(target_arch = "x86_64") { X86_GET_REMEMBERED.adresse } else { GET_REMEMBERED.adresse };
const PLAYER_ID: usize = 0xe0;
const FC_PLAYER: usize = 0xf0;
const SHARED_INSTANCE: i32 = 1000;
const GS_PLAYER: usize = 0x98;
const GS_PREFS: usize = 0x430;
const GS_PREFS_COUNT: usize = 0x440;
const GS_PREFS_DEFAULT: usize = 0x448;

// ------------------------------------------------------------------ état

static BASE: AtomicUsize = AtomicUsize::new(0);
static TRAMPS: [AtomicUsize; 6] = [const { AtomicUsize::new(0) }; 6];
static IN_POST_LOAD: AtomicBool = AtomicBool::new(false);
static UNDONE: AtomicU32 = AtomicU32::new(0);
/// Écritures d'un joueur faites pendant la passe et pas encore défaites : (gestionnaire, joueur,
/// case, ancienne valeur). Toutes portent sur la même case.
static PENDING: Mutex<Vec<(usize, i32, u32, i32)>> = Mutex::new(Vec::new());
/// Pendant la lecture d'un joueur : chaque événement (sentinelle, dernier nœud au début).
static READ_MARKS: Mutex<Option<Vec<(usize, usize)>>> = Mutex::new(None);
/// Chaque lecture de joueur du chargement en cours, dans l'ordre : (id, repères).
static PLAYER_MARKS: Mutex<Vec<(i32, Vec<(usize, usize)>)>> = Mutex::new(Vec::new());
/// Joueurs puis cités libres : appels qui ont déplacé, écouteurs déplacés, listes changées,
/// listes laissées telles quelles.
static STATS: Mutex<[[u32; 4]; 2]> = Mutex::new([[0; 4]; 2]);
static PREFS_CLEARED: Mutex<(u32, Vec<(i32, u32)>)> = Mutex::new((0, Vec::new()));

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn tramp(i: usize) -> usize {
    TRAMPS[i].load(Ordering::Relaxed)
}

fn gc(addr: usize) -> usize {
    BASE.load(Ordering::Relaxed) + addr
}

unsafe fn rd<T: Copy>(at: usize) -> T {
    std::ptr::read_unaligned(at as *const T)
}

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

extern "C" {
    static mach_task_self_: u32;
    fn mach_vm_region(task: u32, address: *mut u64, size: *mut u64, flavor: i32, info: *mut i32, count: *mut u32, object: *mut u32) -> i32;
}
const VM_REGION_BASIC_INFO_64: i32 = 9;
const VM_PROT_READ: i32 = 1;

/// Vrai si `[at, at+len)` est projetée et lisible en ce moment (le pendant du `VirtualQuery` de
/// Windows). Une plage qui déborde (un -1 lu comme pointeur) ne l'est pas.
unsafe fn readable(at: usize, len: usize) -> bool {
    let Some(end) = at.checked_add(len) else { return false };
    if at < 0x10000 || end > 0x8000_0000_0000 {
        return false;
    }
    let mut p = at as u64;
    while p < end as u64 {
        let (mut addr, mut size) = (p, 0u64);
        let mut info = [0i32; 16];
        let mut count: u32 = 9;
        let mut object: u32 = 0;
        if mach_vm_region(mach_task_self_, &mut addr, &mut size, VM_REGION_BASIC_INFO_64, info.as_mut_ptr(), &mut count, &mut object) != 0 {
            return false;
        }
        // La région rendue est celle qui contient `p`, ou la suivante.
        if addr > p || info[0] & VM_PROT_READ == 0 || size == 0 {
            return false;
        }
        p = addr + size;
    }
    true
}

/// L'id du joueur en `p`, -1 si `0x3bbf18` ne le rend pas pour son id.
unsafe fn player_id(p: usize) -> i32 {
    if p == 0 || !readable(p + PLAYER_ID, 4) {
        return -1;
    }
    let id: i32 = rd(p + PLAYER_ID);
    if !(0..=64).contains(&id) {
        return -1;
    }
    let par_id: unsafe extern "C" fn(i32) -> usize = std::mem::transmute(gc(PLAYER_BY_ID_AT));
    if par_id(id) == p { id } else { -1 }
}

// ------------------------------------------------------------------ améliorations mémorisées

type SetRememberedFn = unsafe extern "C" fn(usize, i32, u32, u32) -> u64;

unsafe extern "C" fn post_load_hook(events: usize) -> u64 {
    let orig: unsafe extern "C" fn(usize) -> u64 = std::mem::transmute(tramp(0));
    let pol = pol::actifs();
    UNDONE.store(0, Ordering::Relaxed);
    lock(&PENDING).clear();
    IN_POST_LOAD.store(pol, Ordering::SeqCst);
    let r = orig(events);
    IN_POST_LOAD.store(false, Ordering::SeqCst);
    undo_pending(None);
    let st = std::mem::take(&mut *lock(&STATS));
    let (cleared, profiles) = std::mem::take(&mut *lock(&PREFS_CLEARED));
    if pol {
        log(&format!(
            "partie chargée : passe post-chargement faite, {} écriture(s) d'amélioration mémorisée des joueurs défaite(s) après le test d'affichage",
            UNDONE.load(Ordering::Relaxed)
        ));
        log(&format!(
            "partie chargée : écouteurs de {} joueur(s) remis où commençait leur lecture ({} dans {} liste(s), {} liste(s) laissée(s)) ; cités libres : {} dans {} liste(s), {} laissée(s)",
            st[0][0], st[0][1], st[0][2], st[0][3], st[1][1], st[1][2], st[1][3]
        ));
        log(&format!(
            "partie chargée : préférences d'IA de {cleared} grand(s) stratège(s) remises à zéro après leur lecture ; valeurs du profil retirées (joueur, nombre) : {profiles:?}"
        ));
    }
    lock(&PLAYER_MARKS).clear();
    r
}

/// Remet l'ancienne valeur de chaque écriture en attente, sauf si elle porte sur `keep`.
unsafe fn undo_pending(keep: Option<u32>) {
    let mut pending = lock(&PENDING);
    if pending.is_empty() || pending.first().map(|p| p.2) == keep {
        return;
    }
    let orig: SetRememberedFn = std::mem::transmute(tramp(1));
    for (mgr, player, plot, old) in pending.drain(..) {
        orig(mgr, player, plot, old as u32);
        UNDONE.fetch_add(1, Ordering::Relaxed);
    }
}

/// Comme sous Windows : l'écriture d'un joueur a lieu (le getter du joueur local la relit pour
/// l'affichage), puis elle est défaite à la première écriture d'une autre case, ou à la fin.
unsafe extern "C" fn set_remembered_hook(mgr: usize, player: i32, plot: u32, value: u32) -> u64 {
    let orig: SetRememberedFn = std::mem::transmute(tramp(1));
    if !IN_POST_LOAD.load(Ordering::SeqCst) {
        return orig(mgr, player, plot, value);
    }
    undo_pending(Some(plot));
    if player == SHARED_INSTANCE {
        return orig(mgr, player, plot, value);
    }
    let get: unsafe extern "C" fn(usize, i32, u32) -> i32 = std::mem::transmute(gc(GET_REMEMBERED_AT));
    let old = get(mgr, player, plot);
    let r = orig(mgr, player, plot, value);
    if r & 0xff != 0 {
        lock(&PENDING).push((mgr, player, plot, old));
    }
    r
}

// ------------------------------------------------------------------ ordre des écouteurs

type ConnectFn = unsafe extern "C" fn(usize, usize, usize, usize);

/// Chaque événement de l'objet d'événements du jeu qui se lit comme tel : (sentinelle, dernier
/// nœud).
unsafe fn event_tails() -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let game: usize = rd(gc(GAME));
    if game == 0 || !readable(game + GAME_EVENTS, 8) {
        return out;
    }
    let events: usize = rd(game + GAME_EVENTS);
    if !readable(events, EVENTS_SIZE) {
        return out;
    }
    for off in (0..EVENTS_SIZE).step_by(0x18) {
        let sentinel: usize = rd(events + off + 8);
        if sentinel == 0 || !readable(sentinel, NODE_SIZE) {
            continue;
        }
        let last: usize = rd(sentinel + 8);
        if last != 0 && readable(last, NODE_SIZE) && rd::<usize>(last) == sentinel {
            out.push((sentinel, last));
        }
    }
    out
}

unsafe extern "C" fn player_read_hook(stream: usize, player: usize, x2: usize, x3: usize) -> usize {
    let orig: unsafe extern "C" fn(usize, usize, usize, usize) -> usize = std::mem::transmute(tramp(2));
    if !pol::actifs() {
        return orig(stream, player, x2, x3);
    }
    let marks = event_tails();
    let id = player_id(player);
    if id >= 0 {
        lock(&PLAYER_MARKS).push((id, marks.clone()));
    }
    *lock(&READ_MARKS) = Some(marks);
    let r = orig(stream, player, x2, x3);
    *lock(&READ_MARKS) = None;
    r
}

unsafe extern "C" fn player_connect_hook(setup: usize, game: usize, x2: usize, x3: usize) {
    let orig: ConnectFn = std::mem::transmute(tramp(3));
    let Some(marks) = lock(&READ_MARKS).clone() else {
        return orig(setup, game, x2, x3);
    };
    let before: Vec<usize> = marks.iter().map(|&(s, _)| rd(s + 8)).collect();
    orig(setup, game, x2, x3);
    move_after_marks(&marks, &before, 0);
}

unsafe extern "C" fn fc_connect_hook(mgr: usize, x1: usize, x2: usize, x3: usize) {
    let orig: ConnectFn = std::mem::transmute(tramp(4));
    let id = if readable(mgr + FC_PLAYER, 8) { player_id(rd(mgr + FC_PLAYER)) } else { -1 };
    // Les repères de la lecture qui suit celle du joueur des cités libres (aucun dans une nouvelle
    // partie, ou quand ce joueur a été lu en dernier : les écouteurs restent à la fin).
    let marks = {
        let all = lock(&PLAYER_MARKS);
        all.iter().position(|(p, _)| *p == id).and_then(|i| all.get(i + 1)).map(|(_, m)| m.clone())
    };
    let Some(marks) = marks else {
        return orig(mgr, x1, x2, x3);
    };
    let before: Vec<usize> = marks.iter().map(|&(s, _)| rd(s + 8)).collect();
    orig(mgr, x1, x2, x3);
    move_after_marks(&marks, &before, 1);
}

/// La règle de Windows (`move_after_marks` de partie_chargee.rs), nœuds de `NODE_SIZE` octets et
/// compte de références en `NODE_REFS` : déplace les nœuds ajoutés depuis `before` juste après le repère, si
/// le repère et ce nœud sont encore dans la liste (le repère d'abord) et si chaque nouveau nœud
/// est simplement connecté (compte 1 : aucune émission en cours ne le tient).
unsafe fn move_after_marks(marks: &[(usize, usize)], before: &[usize], group: usize) {
    let mut stats = lock(&STATS);
    let st = &mut stats[group];
    let mut moved_here = 0;
    for (&(sentinel, mark), &old_tail) in marks.iter().zip(before.iter()) {
        let tail: usize = rd(sentinel + 8);
        if tail == old_tail || mark == old_tail {
            continue;
        }
        let (mut mark_seen, mut old_seen) = (mark == sentinel, false);
        let mut node: usize = rd(sentinel);
        let mut steps = 0;
        while node != sentinel && steps < 100_000 && readable(node, NODE_SIZE) {
            if node == mark && !old_seen {
                mark_seen = true;
            }
            if node == old_tail {
                old_seen = true;
                break;
            }
            node = rd(node);
            steps += 1;
        }
        let mut plain = mark_seen && old_seen;
        let first: usize = if plain { rd(old_tail) } else { 0 };
        let mut n = 0;
        if plain {
            let mut k = first;
            n = 1;
            plain = readable(k, NODE_SIZE) && rd::<i32>(k + NODE_REFS) == 1;
            while plain && k != tail && n < 4096 {
                k = rd(k);
                n += 1;
                plain = readable(k, NODE_SIZE) && rd::<i32>(k + NODE_REFS) == 1;
            }
            plain &= k == tail;
        }
        if !plain {
            if st[3] < 3 {
                log(&format!(
                    "partie chargée : liste d'événement en {sentinel:#x} laissée telle quelle (repère {}, dernier nœud d'avant {}, {n} nouveau(x) nœud(s))",
                    if mark_seen { "trouvé" } else { "disparu" },
                    if old_seen { "trouvé" } else { "disparu" }
                ));
            }
            st[3] += 1;
            continue;
        }
        std::ptr::write(old_tail as *mut usize, sentinel);
        std::ptr::write((sentinel + 8) as *mut usize, old_tail);
        let after: usize = rd(mark);
        std::ptr::write(mark as *mut usize, first);
        std::ptr::write((first + 8) as *mut usize, mark);
        std::ptr::write(tail as *mut usize, after);
        std::ptr::write((after + 8) as *mut usize, tail);
        st[1] += n;
        st[2] += 1;
        moved_here += 1;
    }
    if moved_here > 0 {
        st[0] += 1;
    }
}

// ------------------------------------------------------------------ préférences d'IA

unsafe extern "C" fn gs_read_hook(gs: usize, stream: usize) {
    let orig: unsafe extern "C" fn(usize, usize) = std::mem::transmute(tramp(5));
    orig(gs, stream);
    if !pol::actifs() || !readable(gs + GS_PREFS, 0x20) {
        return;
    }
    // Ce que fait l'initialisation après la mise en place, dans une nouvelle partie (0x717bfc).
    let (data, n): (usize, u32) = (rd(gs + GS_PREFS), rd(gs + GS_PREFS_COUNT));
    let n = n as usize;
    let mut dropped = 0;
    if data != 0 && n > 0 && n < 4096 && readable(data, n * 4) {
        for i in 0..n {
            let at = (data + i * 4) as *mut i32;
            dropped += (*at != 0) as u32;
            *at = 0;
        }
    }
    *((gs + GS_PREFS_DEFAULT) as *mut i32) = 0;
    let mut st = lock(&PREFS_CLEARED);
    st.0 += 1;
    if dropped > 0 {
        let id = if readable(gs + GS_PLAYER, 8) { player_id(rd(gs + GS_PLAYER)) } else { -1 };
        st.1.push((id, dropped));
    }
}

// ------------------------------------------------------------------ pose

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    let octets = |f: &Fonction| std::slice::from_raw_parts((base + f.adresse) as *const u8, f.taille);
    for f in DETOURS.iter().chain(VERIFIEES.iter()) {
        if fnv(octets(f)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    for &(a, mot, quoi) in DISPOSITION {
        if rd::<u32>(base + a) != mot {
            return Err(format!("version inattendue ({quoi}, en {a:#x}), aucun détour"));
        }
    }
    poser(base, &DETOURS, &[PROLOGUE; 6])
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    let octets = |a: usize, n: usize| std::slice::from_raw_parts((base + a) as *const u8, n);
    for f in X86_DETOURS.iter().chain(X86_VERIFIEES.iter()) {
        if fnv(octets(f.adresse, f.taille)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    for (f, p) in X86_DETOURS.iter().zip(X86_PROLOGUES) {
        if octets(f.adresse, p.len()) != p {
            return Err(format!("prologue inattendu ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    for &(a, attendu, quoi) in X86_DISPOSITION {
        if octets(a, attendu.len()) != attendu {
            return Err(format!("version inattendue ({quoi}, en {a:#x}), aucun détour"));
        }
    }
    for &(a, cible) in X86_APPELS {
        if patch::cible_rel32_x86(a, octets(a, 5)) != Some(cible) {
            return Err(format!("version inattendue (appel de {cible:#x} en {a:#x}), aucun détour"));
        }
    }
    for d in X86_GAME_DISP {
        if (d as i64 + 4 + rd::<i32>(base + d) as i64) as usize != X86_GAME {
            return Err(format!("version inattendue (le jeu n'est pas en {X86_GAME:#x}, en {d:#x}), aucun détour"));
        }
    }
    poser(base, &X86_DETOURS, &X86_PROLOGUES.map(|p| p.len()))
}

/// Pose les six détours, une fois tout vérifié.
unsafe fn poser(base: usize, detours: &[Fonction; 6], prologues: &[usize; 6]) -> Result<String, String> {
    BASE.store(base, Ordering::Relaxed);
    *lock(&READ_MARKS) = None;
    lock(&PLAYER_MARKS).clear();
    *lock(&STATS) = [[0; 4]; 2];
    *lock(&PREFS_CLEARED) = (0, Vec::new());
    let hooks: [usize; 6] = [
        post_load_hook as *const () as usize,
        set_remembered_hook as *const () as usize,
        player_read_hook as *const () as usize,
        player_connect_hook as *const () as usize,
        fc_connect_hook as *const () as usize,
        gs_read_hook as *const () as usize,
    ];
    // Chaque trampoline est publié avant que son saut ne soit écrit ; tout est posé au démarrage,
    // avant la première partie, donc avant tout appel.
    let mut methode = "";
    for (i, f) in detours.iter().enumerate() {
        methode = patch::hook_with_trampoline_publie(base + f.adresse, prologues[i], hooks[i], &TRAMPS[i])?;
    }
    Ok(format!("6 détours posés (améliorations mémorisées, ordre des écouteurs, préférences d'IA ; {methode})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lisible() {
        let v = vec![1u8; 64];
        unsafe {
            assert!(readable(v.as_ptr() as usize, 64));
            assert!(!readable(0, 8));
            assert!(!readable(usize::MAX - 4, 8));
            assert!(!readable(0x10000, 8) || readable(0x10000, 8)); // ne plante pas
        }
    }

    /// Liste circulaire de nœuds de `NODE_SIZE` octets (suivant, précédent, compte en `NODE_REFS`),
    /// comme `0x55384` / `0x55504` (`0x433ed0` / `0x55e730` en x86_64) : le déplacement de Windows
    /// avec la disposition macOS.
    #[test]
    fn deplacement_apres_repere() {
        unsafe {
            let mut mem = vec![[0usize; 10]; 6]; // sentinelle + 5 nœuds (0x50 octets suffisent aux deux)
            let a: Vec<usize> = mem.iter_mut().map(|n| n.as_mut_ptr() as usize).collect();
            let link = |order: &[usize]| {
                for (i, &n) in order.iter().enumerate() {
                    let next = order[(i + 1) % order.len()];
                    let prev = order[(i + order.len() - 1) % order.len()];
                    std::ptr::write(n as *mut usize, next);
                    std::ptr::write((n + 8) as *mut usize, prev);
                    std::ptr::write((n + NODE_REFS) as *mut i32, 1);
                }
            };
            // Au début de la lecture : S, n1, n2 (repère n2). Puis l'IA ajoute n3, n4 ; à la fin la
            // connexion du joueur ajoute n5.
            link(&[a[0], a[1], a[2], a[3], a[4], a[5]]);
            *((a[0] + NODE_REFS) as *mut i32) = 2;
            move_after_marks(&[(a[0], a[2])], &[a[4]], 0);
            let mut order = vec![];
            let mut k: usize = rd(a[0]);
            while k != a[0] {
                order.push(k);
                k = rd(k);
            }
            assert_eq!(order, vec![a[1], a[2], a[5], a[3], a[4]]);
            // Précédents cohérents.
            for w in order.windows(2) {
                assert_eq!(rd::<usize>(w[1] + 8), w[0]);
            }
            assert_eq!(rd::<usize>(a[0] + 8), a[4]);
            // Un nœud tenu par une émission (compte 2) : liste laissée telle quelle.
            link(&[a[0], a[1], a[2], a[3], a[4], a[5]]);
            *((a[5] + NODE_REFS) as *mut i32) = 2;
            move_after_marks(&[(a[0], a[2])], &[a[4]], 0);
            assert_eq!(rd::<usize>(a[4]), a[5]);
        }
    }

    #[test]
    fn fonctions_du_jeu() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::super::macho::tranche_arm64(&b).expect("tranche arm64");
        let mot = |a: usize| u32::from_le_bytes(t[a..a + 4].try_into().unwrap());
        for f in DETOURS.iter().chain(VERIFIEES.iter()) {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
        }
        for f in &DETOURS {
            for k in 0..PROLOGUE / 4 {
                let m = mot(f.adresse + 4 * k);
                let relatif = m & 0x9f00_0000 == 0x9000_0000 || m & 0x9f00_0000 == 0x1000_0000 || m & 0x7c00_0000 == 0x1400_0000 || m & 0x3b00_0000 == 0x1800_0000;
                assert!(!relatif, "{} : instruction relative en +{}", f.nom, 4 * k);
            }
        }
        for &(a, m, quoi) in DISPOSITION {
            assert_eq!(mot(a), m, "{quoi}");
        }
        // Le setter n'est appelé que par l'écriture de la mémoire d'une case (deux fois).
        let appels: Vec<usize> = (0xcb8 / 4..0xfc93a8 / 4)
            .map(|i| 4 * i)
            .filter(|&a| mot(a) & 0xfc00_0000 == 0x9400_0000 && patch::cible_branche(a, mot(a)) == Some(SET_REMEMBERED.adresse))
            .collect();
        assert_eq!(appels, vec![0x737f44, 0x737f64]);
    }

    #[test]
    fn fonctions_du_jeu_x86_64() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::super::spectateurs::tranche_x86_64(&b).expect("tranche x86_64");
        for f in X86_DETOURS.iter().chain(X86_VERIFIEES.iter()) {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} x86_64 : empreinte {e:#x}", f.nom);
        }
        for (f, p) in X86_DETOURS.iter().zip(X86_PROLOGUES) {
            assert_eq!(&t[f.adresse..f.adresse + p.len()], p, "{} : prologue", f.nom);
            // `jmp [rip+0]` et son adresse : 14 octets (patch_mac.rs, `jump` en x86_64).
            assert!(p.len() >= 14, "{} : prologue plus court que le saut", f.nom);
        }
        for &(a, o, quoi) in X86_DISPOSITION {
            assert_eq!(&t[a..a + o.len()], o, "{quoi}");
        }
        let appel = |a: usize| patch::cible_rel32_x86(a, &t[a..a + 5]);
        for &(a, cible) in X86_APPELS {
            assert_eq!(appel(a), Some(cible), "appel en {a:#x}");
        }
        for d in X86_GAME_DISP {
            let v = i32::from_le_bytes(t[d..d + 4].try_into().unwrap());
            assert_eq!((d as i64 + 4 + v as i64) as usize, X86_GAME, "jeu relu en {d:#x}");
        }
        // Le setter n'est appelé que par l'écriture de la mémoire d'une case (deux fois), et chaque
        // fonction détournée par les seuls appelants relevés.
        const TEXTE_X86: (usize, usize) = (0x1020, 0x1019ead);
        let vers = |cible: usize| -> Vec<usize> {
            (TEXTE_X86.0..TEXTE_X86.1 - 5).filter(|&a| matches!(t[a], 0xe8 | 0xe9) && appel(a) == Some(cible)).collect()
        };
        assert_eq!(vers(0xa89270), vec![0x744bf6, 0x744c16]);
        assert_eq!(vers(0x80a9a0), vec![0x4251c8, 0x425498]);
        assert_eq!(vers(0x9de0d0), vec![0x9e13e3]);
        assert_eq!(vers(0x9711a0), vec![0x9b5cf6]);
        assert_eq!(vers(0x55a810), vec![0x55a7eb, 0x55b561]);
        assert_eq!(vers(0x71e4b0), vec![0x71e49f]);
    }
}
