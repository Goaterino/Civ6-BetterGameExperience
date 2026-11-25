//! Rechargement de tous et barrière après un chargement, sous macOS : le pendant de `rejoin.rs` et de
//! `reload_all.rs` (voir leurs en-têtes : ce qu'ils corrigent et pourquoi), avec leurs règles
//! communes (`rejoin_regle.rs`, `reload_all_regle.rs`) et les mêmes lignes de journal (`rejoin:`,
//! `reload:`), pour qu'un Mac compté comme équipé fasse exactement ce que fait un PC :
//!
//! - **client** : une demande de copie que l'hôte envoie pour chaque joueur (`NetSnapshotRequest`)
//!   marque le rechargement de tous ; la vérification de la partie chargée est gardée (l'hôte
//!   l'attend), un « tour de synchro terminé » laissé par une manche d'avant le chargement ne termine
//!   pas la barrière de ce chargement, et les ordres de jeu d'un joueur qui recharge sont ignorés ;
//!   après un chargement seul, la vérification que l'hôte n'attend pas est sautée (drapeaux du
//!   `NetSnapshot`, lus par l'accroche de son `Process`, ce que fait `snapshots.rs` sous Windows) ;
//! - **hôte** : `partie.lua` lance le rechargement de tous (une demande par joueur, l'hôte en
//!   dernier), et les réponses, vérifications et demandes d'un joueur qui recharge, d'avant sa fin de
//!   chargement, sont écartées.
//!
//! Pas de `sync_notes` sous macOS : `Civ6Proxy_SyncNotes` rend `""` (le bloc de `partie.lua` ne
//! tourne qu'en sa présence). Les notes ne servent qu'au message de chat qui dit pourquoi l'hôte
//! recharge tout ; la décision vient de la demande de copie du client, inchangée. Pas de
//! `resynchro_volume` non plus (hôte seul, le bloc teste sa présence).
//!
//! Relevé dans Civ6_Exe_Child (Civ6 Aspyr 1.4.6), adresses de l'exe (base `0x100000000`). Les
//! tables virtuelles suivent l'ABI Itanium (deux destructeurs en tête) : `Process` des messages en
//! + 0x28 (Windows + 0x20), les méthodes de session à + 8 de Windows ; les champs des messages et des
//! joueurs réseau sont à + 4 de Windows (sauf le drapeau de test d'une vérification, + 1) :
//!
//! | rôle | arm64 | x86_64 |
//! |---|---|---|
//! | mise à jour de la barrière (« start check » une fois) | `0x10137d8f0` | `0x101422ff0` |
//! | « fin de tour » du contrôleur (entre dans la barrière) | `0x10137e2b0` | `0x101423a10` |
//! | « tour de synchro terminé » du contrôleur | `0x10137ea60` | `0x1014241e0` |
//! | `FGameSyncManager` « start check » (vtable + 0x18) | `0x100bd6d64` | `0x100c826e0` |
//! | session d'un ID (gestionnaire réseau, ID) | `0x100be118c` | `0x100c8c850` |
//! | accusé à réception (`[sync + 0x38]`) | `0x100bdb514` | `0x100c87c90` |
//! | « attend la partie » (joueur, attend, raison) | `0x100bff550` | `0x100cad980` |
//! | globales : synchro, gestionnaire réseau, tâches | `0x10d40b4d8`, `0x10d40b4e0`, `0x10f09b8b0` | `0x10e229698`, `0x10e2296a0`, `0x1029848f0` |
//! | `NetSnapshotRequest` vtable, `Process`, constructeur | `0x101b14ca0`, `0x100c259ec`, `0x100c2584c` | `0x101c4f9f0`, `0x100cd7ef0`, `0x100cd7d70` |
//! | `NetGameSyncProcessed` vtable, `Process` | `0x101b14da0`, `0x100c2654c` | `0x101c4faf0`, `0x100cd8c30` |
//! | `NetGameSyncCheck` vtable, `Process` | `0x101b14d60`, `0x100c26300` | `0x101c4fab0`, `0x100cd8920` |
//! | `NetSnapshot` vtable, `Process` | `0x101b14ce0`, `0x100c25e70` | `0x101c4fa30`, `0x100cd8500` |
//! | réception : test « le joueur local attend sa copie » (en ligne) | `0x100c11038` (0x6c octets) | `0x100cc0f85` (0x5e octets) |
//!
//! Le test de la réception est en ligne sous macOS (une fonction sous Windows) : sa suite
//! d'instructions est remplacée, sur place, par un appel à `on_receive` (session, message) puis un
//! saut vers la suite normale ou vers « (ignored, %d) ». Tout est vérifié au démarrage (empreinte
//! FNV-1a de chaque fonction entière, cases des vtables), avant que le jeu n'exécute ce code.
//!
//! Réglages (`rollover_proxy.cfg`) comme sous Windows : `actif=off`, `rejoin_barrier=off`,
//! `stale_round_guard=off`, `reload_all=off` (ne lance pas de rechargement comme hôte),
//! `reload_orders=off`.

use super::journal::log;
use super::options::{argument_n, lire, rendre, LuaCFn};
use super::salon_texte::actif_off;
use super::{macho, patch};
use std::ffi::{c_int, c_void};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

#[path = "rejoin_regle.rs"]
mod rj;
#[path = "reload_all_regle.rs"]
mod ra;

/// Base des adresses de l'exe.
const BASE_EXE: usize = 0x1_0000_0000;

struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    empreinte: u64,
}

const fn f(nom: &'static str, adresse: usize, taille: usize, empreinte: u64) -> Fonction {
    Fonction { nom, adresse, taille, empreinte }
}

struct Exe {
    update: Fonction,
    turn_end: Fonction,
    round_complete: Fonction,
    /// Le début de « start check » : il lit le gestionnaire réseau et appelle `lookup`.
    start_check: Fonction,
    lookup: Fonction,
    ack_on_receipt: Fonction,
    set_waiting: Fonction,
    snapshot_process: Fonction,
    request_process: Fonction,
    request_ctor: Fonction,
    ack_process: Fonction,
    check_process: Fonction,
    /// Le test en ligne de la réception, remplacé ; la suite normale et celle d'un message ignoré.
    recv: Fonction,
    recv_normal: usize,
    sync: usize,
    net_manager: usize,
    jobs: usize,
    snapshot_vtable: usize,
    request_vtable: usize,
    ack_vtable: usize,
    check_vtable: usize,
    /// Octets de « fin de tour » recopiés dans le trampoline (sans instruction relative).
    turn_end_prologue: usize,
}

const ARM: Exe = Exe {
    update: f("mise à jour de la barrière", 0x1_0137_d8f0, 0x4c, 0x8b0e_16d3_e0cd_1b7c),
    turn_end: f("fin de tour", 0x1_0137_e2b0, 0xa0, 0x56b1_9501_d69c_baf3),
    round_complete: f("tour de synchro terminé", 0x1_0137_ea60, 0x18, 0x64f8_a9d8_2185_3d0f),
    start_check: f("start check", 0x1_00bd_6d64, 0x40, 0xad89_d72e_e175_64d5),
    lookup: f("session d'un ID", 0x1_00be_118c, 0x4c, 0x9d09_9cff_90c5_ac42),
    ack_on_receipt: f("accusé à réception", 0x1_00bd_b514, 0xb8, 0x160c_7dd4_9fca_804f),
    set_waiting: f("attend la partie", 0x1_00bf_f550, 0x1c, 0x53cb_9e84_ea32_2988),
    snapshot_process: f("NetSnapshot::Process", 0x1_00c2_5e70, 0x8c, 0x4fef_313e_58f3_fb89),
    request_process: f("NetSnapshotRequest::Process", 0x1_00c2_59ec, 0x78, 0x83aa_7f5f_cc8b_5d6a),
    request_ctor: f("NetSnapshotRequest()", 0x1_00c2_584c, 0x44, 0x5ce5_7f11_0bbb_a994),
    ack_process: f("NetGameSyncProcessed::Process", 0x1_00c2_654c, 0x58, 0x82d0_da88_0eda_f373),
    check_process: f("NetGameSyncCheck::Process", 0x1_00c2_6300, 0x12c, 0x773e_1e5e_6ae2_04be),
    recv: f("test d'attente de la réception", 0x1_00c1_1038, 0x6c, 0xa383_5171_5787_b951),
    recv_normal: 0x1_00c1_12a0,
    sync: 0x1_0d40_b4d8,
    net_manager: 0x1_0d40_b4e0,
    jobs: 0x1_0f09_b8b0,
    snapshot_vtable: 0x1_01b1_4ce0,
    request_vtable: 0x1_01b1_4ca0,
    ack_vtable: 0x1_01b1_4da0,
    check_vtable: 0x1_01b1_4d60,
    // stp ; stp ; stp ; add x29, sp, #0x20
    turn_end_prologue: 16,
};

const X86: Exe = Exe {
    update: f("mise à jour de la barrière", 0x1_0142_2ff0, 0x40, 0xe1b1_706d_90a4_c08b),
    turn_end: f("fin de tour", 0x1_0142_3a10, 0xb0, 0x785d_3be9_6313_302f),
    round_complete: f("tour de synchro terminé", 0x1_0142_41e0, 0x20, 0x99e8_5f82_ea63_0106),
    start_check: f("start check", 0x1_00c8_26e0, 0x40, 0x9a60_0355_a9a0_30e9),
    lookup: f("session d'un ID", 0x1_00c8_c850, 0x50, 0x2ea5_1695_3069_c68a),
    ack_on_receipt: f("accusé à réception", 0x1_00c8_7c90, 0xa0, 0x0d9b_f178_80e0_945e),
    set_waiting: f("attend la partie", 0x1_00ca_d980, 0x20, 0x51d2_fdf3_ed75_b5cf),
    snapshot_process: f("NetSnapshot::Process", 0x1_00cd_8500, 0x70, 0xca49_238b_4728_3816),
    request_process: f("NetSnapshotRequest::Process", 0x1_00cd_7ef0, 0x70, 0x9474_e7ea_0100_45f2),
    request_ctor: f("NetSnapshotRequest()", 0x1_00cd_7d70, 0x40, 0x056e_fae0_ab63_0963),
    ack_process: f("NetGameSyncProcessed::Process", 0x1_00cd_8c30, 0x40, 0x5678_b1cf_8849_d6db),
    check_process: f("NetGameSyncCheck::Process", 0x1_00cd_8920, 0x140, 0x9ffe_eef1_bc90_290f),
    recv: f("test d'attente de la réception", 0x1_00cc_0f85, 0x5e, 0x3c1b_9040_1b07_d132),
    recv_normal: 0x1_00cc_1234,
    sync: 0x1_0e22_9698,
    net_manager: 0x1_0e22_96a0,
    jobs: 0x1_0298_48f0,
    snapshot_vtable: 0x1_01c4_fa30,
    request_vtable: 0x1_01c4_f9f0,
    ack_vtable: 0x1_01c4_faf0,
    check_vtable: 0x1_01c4_fab0,
    // push rbp ; mov rbp,rsp ; push r15 ; push r14 ; push r12 ; push rbx ; mov byte [rdi+8],1
    turn_end_prologue: 15,
};

const EST_X86: bool = cfg!(target_arch = "x86_64");
const EXE: Exe = if EST_X86 { X86 } else { ARM };

/// L'entrée `Process` des vtables de messages (deux destructeurs, texte, écriture, lecture, process).
const PROCESS: usize = 0x28;
// Message : drapeaux, type, émetteur. Demande : joueur, raison. Réponse : joueur. Vérification :
// test. NetSnapshot : drapeaux, taille des données.
const MSG_HANDLED: usize = 8;
const MSG_FLAGS: usize = 0xc;
const MSG_FLAG_LOCAL: u8 = 4;
const MSG_TYPE: usize = 0x1c;
const MSG_SENDER: usize = 0x28;
const MSG_PLAYER: usize = 0x3c;
const MSG_REASON: usize = 0x40;
const MSG_TEST: usize = 0x39;
const SNAPSHOT_FLAGS: usize = 0x3c;
const SNAPSHOT_SIZE: usize = 0x58;
/// Un `NetSnapshotRequest` fait 0x48 octets ; de la marge.
const MSG_WORDS: usize = 16;
// Session : diffusion, joueur local, hôte, joueur réseau d'un ID, « est l'hôte ».
const SESSION_BROADCAST: usize = 0xc0;
const SESSION_LOCAL: usize = 0x150;
const SESSION_HOST: usize = 0x158;
const SESSION_PLAYER: usize = 0x160;
const SESSION_IS_HOST: usize = 0x1c8;
// Joueur réseau : attend la partie, la partie lui a été envoyée, a répondu à la manche, raison.
const PLAYER_WAITING: usize = 0x1e;
const PLAYER_SENT: usize = 0x1f;
const PLAYER_ANSWERED: usize = 0x23;
const PLAYER_REASON: usize = 0x2c;
// Gestionnaire de synchro : ID de session, « start check », « une réponse est arrivée », accusé dû,
// manche en cours.
const SYNC_SESSION: usize = 8;
const SYNC_START_CHECK: usize = 0x18;
const SYNC_ON_ANSWER: usize = 0x30;
const SYNC_ACK_OWED: usize = 0x38;
const SYNC_ROUND: usize = 0x3c;

static SLIDE: AtomicUsize = AtomicUsize::new(0);
/// Les accroches de la barrière sont posées (`rejoin::installed`).
static BARRIERE: AtomicBool = AtomicBool::new(false);
/// Celles du rechargement de tous aussi (`reload_all::installed`).
static RECHARGE: AtomicBool = AtomicBool::new(false);
static NEVER_SKIP: AtomicBool = AtomicBool::new(false);
static TURN_END_ORIG: AtomicUsize = AtomicUsize::new(0);
static SNAPSHOT_ORIG: AtomicUsize = AtomicUsize::new(0);
static REQUEST_ORIG: AtomicUsize = AtomicUsize::new(0);
static ACK_ORIG: AtomicUsize = AtomicUsize::new(0);
static CHECK_ORIG: AtomicUsize = AtomicUsize::new(0);
/// Le dernier snapshot remis au jeu (`rj::pack`), jusqu'à ce qu'une barrière d'après chargement le lise.
static LAST_SNAPSHOT: AtomicU64 = AtomicU64::new(0);

type Process = unsafe extern "C" fn(usize, usize);

fn image(a: usize) -> usize {
    (a as isize + SLIDE.load(Ordering::Relaxed) as isize) as usize
}

unsafe fn rd<T: Copy>(a: usize) -> T {
    std::ptr::read_unaligned(a as *const T)
}

unsafe fn method<F: Copy>(object: usize, offset: usize) -> F {
    std::mem::transmute_copy(&rd::<usize>(rd::<usize>(object) + offset))
}

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

// ---- session et gestionnaire de synchro (rejoin.rs)

unsafe fn sync_manager() -> usize {
    rd(image(EXE.sync))
}

/// La session à laquelle appartient le gestionnaire de synchro.
unsafe fn session() -> Option<usize> {
    let sync = sync_manager();
    let manager: usize = rd(image(EXE.net_manager));
    if sync == 0 || manager == 0 {
        return None;
    }
    let lookup: unsafe extern "C" fn(usize, u32) -> usize = std::mem::transmute(image(EXE.lookup.adresse));
    let s = lookup(manager, rd(sync + SYNC_SESSION));
    (s != 0).then_some(s)
}

unsafe fn is_host() -> Option<bool> {
    let s = session()?;
    Some(method::<unsafe extern "C" fn(usize) -> u8>(s, SESSION_IS_HOST)(s) != 0)
}

/// Session, gestionnaire de synchro, et si cette machine est l'hôte (`rejoin::network`).
unsafe fn network() -> Option<(usize, usize, bool)> {
    if !BARRIERE.load(Ordering::Relaxed) {
        return None;
    }
    let s = session()?;
    Some((s, sync_manager(), is_host()?))
}

unsafe fn local_player(s: usize) -> i32 {
    method::<unsafe extern "C" fn(usize) -> i32>(s, SESSION_LOCAL)(s)
}

unsafe fn host_player(s: usize) -> i32 {
    method::<unsafe extern "C" fn(usize) -> i32>(s, SESSION_HOST)(s)
}

unsafe fn net_player(s: usize, player: i32) -> usize {
    method::<unsafe extern "C" fn(usize, i32) -> usize>(s, SESSION_PLAYER)(s, player)
}

/// Raison de la dernière demande de copie du joueur local.
unsafe fn local_reason() -> Option<i32> {
    let s = session()?;
    let p = net_player(s, local_player(s));
    (p != 0).then(|| rd::<i32>(p + PLAYER_REASON))
}

// ---- barrière (rejoin.rs)

/// Remplace « tour de synchro terminé » du contrôleur.
unsafe extern "C" fn round_complete(ctl: *mut u8) {
    let after_turn_end = rj::AFTER_TURN_END.load(Ordering::Relaxed);
    if !rj::complete_round(ctl, after_turn_end) {
        log("rejoin: a sync round that ended before this load did not end its turn barrier");
        return;
    }
    if let Some((_, _, host)) = network() {
        ra::round_done(host, after_turn_end);
    }
}

/// « Fin de tour » accrochée : la barrière qui suit est une vraie, et tout snapshot est du passé.
unsafe extern "C" fn turn_end(ctl: *mut u8) {
    rj::AFTER_TURN_END.store(true, Ordering::Relaxed);
    LAST_SNAPSHOT.store(0, Ordering::Relaxed);
    let orig: unsafe extern "C" fn(*mut u8) = std::mem::transmute(TURN_END_ORIG.load(Ordering::Acquire));
    orig(ctl)
}

/// Remplace la mise à jour de la barrière : le code du jeu, plus la sortie sans vérification d'une
/// barrière d'après chargement, sur un client, quand l'hôte n'attend pas sa vérification.
unsafe extern "C" fn update(ctl: *mut u8) {
    if *ctl.add(rj::CTL_CHECK_STARTED) != 0 {
        return;
    }
    let after_turn_end = rj::AFTER_TURN_END.load(Ordering::Relaxed);
    let host = is_host();
    if !after_turn_end && host == Some(false) {
        let packed = LAST_SNAPSHOT.swap(0, Ordering::Relaxed);
        if packed != 0 {
            let load = local_reason().and_then(|reason| rj::unpack(packed, reason));
            // Les deux sont lues : chacune ne vaut que pour ce chargement.
            let asked = rj::take_keep();
            if ra::take_keep() || asked {
                log(&rj::describe("sync check after loading kept: the host reloaded everyone", load));
            } else if !NEVER_SKIP.load(Ordering::Relaxed) && rj::skip(after_turn_end, host, load) {
                *ctl.add(rj::CTL_IN_BARRIER) = 0;
                log(&rj::describe("turn barrier after loading left without a sync check", load));
                return;
            } else {
                log(&rj::describe("sync check after loading kept, the host waits for it", load));
            }
        }
    }
    if *ctl.add(rj::CTL_JOBS) != 0 {
        let jobs: usize = rd(image(EXE.jobs));
        if jobs != 0 && rd::<u32>(jobs + rj::JOBS_PENDING) != 0 {
            return;
        }
    }
    *ctl.add(rj::CTL_CHECK_STARTED) = 1;
    rj::AFTER_TURN_END.store(false, Ordering::Relaxed);
    let sync = sync_manager();
    method::<unsafe extern "C" fn(usize)>(sync, SYNC_START_CHECK)(sync)
}

/// Remplace `NetSnapshot::Process` : note les drapeaux du snapshot avant que le jeu n'envoie un
/// accusé dû (`rejoin::snapshot_arrives`, appelé par `snapshots.rs` sous Windows).
unsafe extern "C" fn on_snapshot(msg: usize, s: usize) {
    if msg != 0 && s != 0 && rd::<i32>(msg + SNAPSHOT_SIZE) > 0 {
        let sync = sync_manager();
        let ack = sync != 0 && rd::<u8>(sync + SYNC_ACK_OWED) != 0;
        LAST_SNAPSHOT.store(rj::pack(rd(msg + SNAPSHOT_FLAGS), ack), Ordering::Relaxed);
    }
    let orig: Process = std::mem::transmute(SNAPSHOT_ORIG.load(Ordering::Acquire));
    orig(msg, s)
}

/// L'interface de la partie se charge (InGame.lua) : `rejoin::new_game`.
pub(crate) fn new_game() {
    rj::new_game();
}

// ---- rechargement de tous (reload_all.rs)

unsafe fn reloading(s: usize, player: i32) -> bool {
    if ra::RELOADING.load(Ordering::Relaxed) & ra::bit(player) == 0 {
        return false;
    }
    let p = net_player(s, player);
    ra::still_reloading(player, (p != 0).then(|| rd::<u8>(p + PLAYER_WAITING) != 0))
}

fn host_ready(cfg: &str) -> bool {
    !actif_off(cfg) && ra::enabled(cfg) && RECHARGE.load(Ordering::Relaxed)
}

unsafe fn state(turn: i32, player: i32) -> i32 {
    if !RECHARGE.load(Ordering::Relaxed) {
        return ra::NOT_POSSIBLE;
    }
    let Some((s, sync, true)) = network() else { return ra::NOT_POSSIBLE };
    if rd::<u8>(sync + SYNC_ROUND) == 0 {
        return ra::NOT_POSSIBLE;
    }
    if super::pol::BgdPolActifs() == 0 {
        return ra::MIXED_VERSIONS;
    }
    if reloading(s, player) {
        return ra::RUNNING;
    }
    if (0..ra::MAX_PLAYERS).any(|p| reloading(s, p)) {
        return ra::OTHERS_RELOADING;
    }
    if ra::turn_allows(turn) {
        ra::CAN_START
    } else {
        ra::GAVE_UP
    }
}

unsafe fn start(turn: i32, player: i32) -> i32 {
    let st = state(turn, player);
    if st != ra::CAN_START {
        return st;
    }
    let Some((s, _, true)) = network() else { return ra::NOT_POSSIBLE };
    let local = local_player(s);
    let ctor: unsafe extern "C" fn(*mut u64, i32, i32) = std::mem::transmute(image(EXE.request_ctor.adresse));
    let broadcast: unsafe extern "C" fn(usize, *mut u64, u32, u32) = method(s, SESSION_BROADCAST);
    let (mut asked, mut alone) = (Vec::new(), Vec::new());
    for player in (0..ra::MAX_PLAYERS).filter(|p| *p != local).chain([local]) {
        let p = net_player(s, player);
        if p == 0 {
            continue;
        }
        if player != local && rd::<u8>(p + PLAYER_WAITING) != 0 && rd::<u8>(p + PLAYER_SENT) != 0 {
            alone.push(player.to_string());
            continue;
        }
        ra::RELOADING.fetch_or(ra::bit(player), Ordering::Relaxed);
        ra::MARKED.fetch_and(!ra::bit(player), Ordering::Relaxed);
        *((p + PLAYER_ANSWERED) as *mut u8) = 0;
        let mut message = [0u64; MSG_WORDS];
        ctor(message.as_mut_ptr(), player, ra::REASON_RESYNC);
        broadcast(s, message.as_mut_ptr(), 1, 0);
        asked.push(player.to_string());
    }
    let n = ra::note_started(turn);
    let alone = if alone.is_empty() { String::new() } else { format!("; already loading alone: {}", alone.join(", ")) };
    log(&format!("reload: turn {turn}, reload {n} of everyone started for player(s) {}{alone}", asked.join(", ")));
    ra::CAN_START
}

unsafe fn recheck() {
    if !RECHARGE.load(Ordering::Relaxed) {
        return;
    }
    let Some((_, sync, true)) = network() else { return };
    if rd::<u8>(sync + SYNC_ROUND) != 0 {
        method::<unsafe extern "C" fn(usize)>(sync, SYNC_ON_ANSWER)(sync);
    }
}

/// Remplace `NetSnapshotRequest::Process`.
unsafe extern "C" fn on_request(msg: usize, s: usize) {
    let original: Process = std::mem::transmute(REQUEST_ORIG.load(Ordering::Acquire));
    if s == 0 {
        return original(msg, s);
    }
    let (sender, player, reason) = (rd::<i32>(msg + MSG_SENDER), rd::<i32>(msg + MSG_PLAYER), rd::<i32>(msg + MSG_REASON));
    let (local, host_id) = (local_player(s), host_player(s));
    match ra::classify(local == host_id, local, host_id, sender, player, reason, reloading(s, player)) {
        ra::Request::Plain => original(msg, s),
        ra::Request::Drop => log(&format!("reload: player {player}'s own request dropped, its copy is on its way")),
        ra::Request::Reload { keep } => {
            // Sur un client, l'hôte recharge aussi : sa propre demande suit.
            ra::RELOADING.fetch_or(ra::bit(player) | ra::bit(host_id), Ordering::Relaxed);
            original(msg, s);
            ra::MARKED.fetch_or(ra::bit(player), Ordering::Relaxed);
            if keep {
                ra::KEEP.store(true, Ordering::Relaxed);
                log("reload: the host reloads everyone; the check after this game's load is kept");
            }
        }
    }
}

/// Remplace `NetGameSyncProcessed::Process` : chez l'hôte, pas d'un joueur qui recharge encore.
unsafe extern "C" fn on_ack(msg: usize, s: usize) {
    let player = rd::<i32>(msg + MSG_PLAYER);
    if s != 0 && local_player(s) == host_player(s) && player != local_player(s) && reloading(s, player) {
        log(&format!("reload: player {player}'s answer from before its load dropped"));
        return;
    }
    let original: Process = std::mem::transmute(ACK_ORIG.load(Ordering::Acquire));
    original(msg, s)
}

/// Remplace `NetGameSyncCheck::Process` : chez l'hôte, pas d'un joueur qui recharge encore.
unsafe extern "C" fn on_check(msg: usize, s: usize) {
    let sender = rd::<i32>(msg + MSG_SENDER);
    if s != 0 && rd::<u8>(msg + MSG_TEST) == 0 && local_player(s) == host_player(s) && sender != local_player(s) && reloading(s, sender) {
        log(&format!("reload: player {sender}'s check from before its load dropped"));
        return;
    }
    let original: Process = std::mem::transmute(CHECK_ORIG.load(Ordering::Acquire));
    original(msg, s)
}

/// Appelée à la place du test en ligne de la réception : 1 si le message est ignoré. Le test du jeu
/// (le joueur local attend sa copie et ne l'a pas reçue, message ni traité ni local), où « attend »
/// vaut aussi pour un ordre de jeu d'un joueur qui recharge et dont la demande a été vue ici.
unsafe extern "C" fn on_receive(s: usize, msg: usize) -> u32 {
    let p = net_player(s, local_player(s));
    let mut waiting = p != 0 && rd::<u8>(p + PLAYER_WAITING) != 0 && rd::<u8>(p + PLAYER_SENT) == 0;
    if !waiting && rd::<u32>(msg + MSG_TYPE) == ra::TYPE_GAME_ORDER {
        let sender = rd::<i32>(msg + MSG_SENDER);
        if ra::MARKED.load(Ordering::Relaxed) & ra::bit(sender) != 0 && reloading(s, sender) {
            log(&format!("reload: game order from player {sender} ignored, sent before its load"));
            waiting = true;
        }
    }
    (waiting && rd::<u8>(msg + MSG_HANDLED) == 0 && rd::<u8>(msg + MSG_FLAGS) & MSG_FLAG_LOCAL == 0) as u32
}

/// Le code posé sur le test de la réception, en `at` (adresses de l'image), qui appelle `hook`.
#[cfg(target_arch = "aarch64")]
fn code_reception(at: usize, normal: usize, ignore: usize, hook: usize) -> Option<Vec<u8>> {
    let imm19 = |de: usize, vers: usize| {
        let d = (vers as i64 - de as i64) / 4;
        ((-(1 << 18))..(1 << 18)).contains(&d).then_some(((d as u32) & 0x7_ffff) << 5)
    };
    let mut mots = vec![
        0xaa14_03e0, // mov x0, x20 (la session)
        0xaa13_03e1, // mov x1, x19 (le message)
        0x5800_0090, // ldr x16, #0x10 (l'adresse, en + 0x18)
        0xd63f_0200, // blr x16
        0x3400_0000 | imm19(at + 0x10, normal)?, // cbz w0, suite normale
        u32::from_le_bytes(patch::branche(at + 0x14, ignore)?),
    ];
    mots.extend([hook as u32, (hook as u64 >> 32) as u32]);
    Some(mots.iter().flat_map(|m| m.to_le_bytes()).collect())
}

#[cfg(target_arch = "x86_64")]
fn code_reception(at: usize, normal: usize, ignore: usize, hook: usize) -> Option<Vec<u8>> {
    // mov rdi,r14 (la session) ; mov rsi,rbx (le message) ; call [rip+13] ; test eax,eax ;
    // je normal ; jmp ignoré ; l'adresse.
    let mut v = vec![0x4c, 0x89, 0xf7, 0x48, 0x89, 0xde, 0xff, 0x15, 13, 0, 0, 0, 0x85, 0xc0, 0x0f, 0x84];
    let je = i32::try_from(normal as i64 - (at as i64 + 20)).ok()?;
    v.extend(je.to_le_bytes());
    v.extend(patch::rel32_x86(0xe9, at + 20, ignore)?);
    v.extend((hook as u64).to_le_bytes());
    Some(v)
}

/// Remplit le reste de la zone remplacée (jamais exécuté) : `brk` / `int3`.
fn remplir(mut code: Vec<u8>, taille: usize) -> Vec<u8> {
    let bouche: &[u8] = if EST_X86 { &[0xcc] } else { &[0x00, 0x00, 0x20, 0xd4] };
    while code.len() < taille {
        code.extend_from_slice(bouche);
    }
    code.truncate(taille);
    code
}

extern "C" {
    static mach_task_self_: u32;
    fn getpagesize() -> c_int;
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
    fn mach_vm_protect(task: u32, addr: u64, size: u64, set_maximum: c_int, prot: c_int) -> c_int;
}

/// Pose `valeur` dans une case de vtable (des données : `__DATA_CONST` en arm64, `__DATA` en x86_64).
unsafe fn poser_case(case: usize, valeur: usize) -> bool {
    const PROT_READ: c_int = 1;
    const PROT_WRITE: c_int = 2;
    const VM_PROT_COPY: c_int = 0x10;
    let ps = getpagesize() as usize;
    let page = case & !(ps - 1);
    let ok = mprotect(page as *mut c_void, ps, PROT_READ | PROT_WRITE) == 0
        || mach_vm_protect(mach_task_self_, page as u64, ps as u64, 0, PROT_READ | PROT_WRITE | VM_PROT_COPY) == 0;
    if ok {
        std::ptr::write_volatile(case as *mut usize, valeur);
    }
    ok
}

// ---- natives de partie.lua (une chaîne chacune, macos_options.rs dit pourquoi)

fn args(l: *mut c_void) -> Option<(i32, i32)> {
    unsafe { Some((ra::int_text(&argument_n(l, 1))?, ra::int_text(&argument_n(l, 2))?)) }
}

/// `Civ6Proxy_SyncNotes()` → `""` : pas de notes de synchro sous macOS.
unsafe extern "C-unwind" fn sync_notes(l: *mut c_void) -> c_int {
    rendre(l, "")
}

/// `Civ6Proxy_MacReloadAllReady()` → `1` si cette machine, comme hôte, recharge tout le monde.
unsafe extern "C-unwind" fn mac_ready(l: *mut c_void) -> c_int {
    let cfg = lire().unwrap_or_default();
    rendre(l, if host_ready(&cfg) { "1" } else { "0" })
}

/// `Civ6Proxy_MacReloadAllState(tour, joueur)` → l'état, en chiffre (reload_all.rs, `lua_state`).
unsafe extern "C-unwind" fn mac_state(l: *mut c_void) -> c_int {
    let st = args(l).map_or(ra::NOT_POSSIBLE, |(turn, player)| state(turn, player));
    rendre(l, &st.to_string())
}

/// `Civ6Proxy_MacReloadAllStart(tour, joueur)` → mêmes valeurs ; 1 quand le rechargement est lancé.
unsafe extern "C-unwind" fn mac_start(l: *mut c_void) -> c_int {
    let st = args(l).map_or(ra::NOT_POSSIBLE, |(turn, player)| start(turn, player));
    rendre(l, &st.to_string())
}

/// `Civ6Proxy_ReloadAllRecheck()` : un joueur est parti.
unsafe extern "C-unwind" fn lua_recheck(_l: *mut c_void) -> c_int {
    recheck();
    0
}

/// `Civ6Proxy_RejoinKeepCheck()` : l'hôte recharge tout le monde ; le prochain chargement garde sa
/// vérification.
unsafe extern "C-unwind" fn lua_keep_check(_l: *mut c_void) -> c_int {
    rj::keep_next_check();
    log("rejoin: the host reloads everyone; the check after the next load is kept");
    0
}

/// Natives de `partie.lua`, à poser avec celles de macos_salon.rs (`NATIVES_PARTIE`).
pub(crate) const NATIVES_PARTIE: [(LuaCFn, &str); 6] = [
    (sync_notes, "Civ6Proxy_SyncNotes\0"),
    (mac_ready, "Civ6Proxy_MacReloadAllReady\0"),
    (mac_state, "Civ6Proxy_MacReloadAllState\0"),
    (mac_start, "Civ6Proxy_MacReloadAllStart\0"),
    (lua_recheck, "Civ6Proxy_ReloadAllRecheck\0"),
    (lua_keep_check, "Civ6Proxy_RejoinKeepCheck\0"),
];

/// Cale placée devant le bloc de `partie.lua` : les fonctions à nombres qu'il attend.
pub(crate) const CALE_PARTIE: &str = r#"
-- macOS : natives à une seule chaîne (startup_proxy/src/macos_reload.rs).
if Civ6Proxy_MacReloadAllReady ~= nil then
	local function BGD_RN(v) return string.format("%.0f", tonumber(v) or -1); end
	Civ6Proxy_ReloadAllReady = function() return tonumber(Civ6Proxy_MacReloadAllReady()) or 0; end
	Civ6Proxy_ReloadAllState = function(t, p) return tonumber(Civ6Proxy_MacReloadAllState(BGD_RN(t), BGD_RN(p))) or 0; end
	Civ6Proxy_ReloadAllStart = function(t, p) return tonumber(Civ6Proxy_MacReloadAllStart(BGD_RN(t), BGD_RN(p))) or 0; end
end
"#;

// ---- pose

unsafe fn verifier(fs: &[&Fonction]) -> Result<(), String> {
    for f in fs {
        let e = fnv(std::slice::from_raw_parts(image(f.adresse) as *const u8, f.taille));
        if e != f.empreinte {
            return Err(format!("{} en {:#x}", f.nom, f.adresse));
        }
    }
    Ok(())
}

unsafe fn case_process(vtable: usize) -> usize {
    image(vtable + PROCESS)
}

/// Les cases `Process` des vtables tiennent bien leur fonction.
unsafe fn verifier_cases(cases: &[(usize, &Fonction)]) -> Result<(), String> {
    for (vtable, f) in cases {
        if rd::<usize>(case_process(*vtable)) != image(f.adresse) {
            return Err(format!("la vtable en {vtable:#x} ne tient pas {}", f.nom));
        }
    }
    Ok(())
}

/// Pose la barrière, puis le rechargement de tous. Appelée au démarrage, avant que le jeu n'exécute
/// ce code. `Err` : rien n'est posé.
pub unsafe fn installer(_mh: *const macho::MachHeader64, slide: isize) -> Result<(), String> {
    let cfg = lire().unwrap_or_default();
    if actif_off(&cfg) {
        return Err("non posé (actif=off)".into());
    }
    if !rj::enabled(&cfg) {
        log("rejoin: turned off (rejoin_barrier=off)");
        log("reload: not installed (needs the rejoin fix)");
        return Ok(());
    }
    SLIDE.store(slide as usize, Ordering::Relaxed);
    let e = &EXE;
    verifier(&[&e.update, &e.turn_end, &e.round_complete, &e.start_check, &e.lookup, &e.ack_on_receipt, &e.set_waiting])
        .map_err(|m| format!("rejoin: unexpected exe ({m}), fix not installed"))?;
    // La fin de tour d'abord : sans elle, la mise à jour prendrait chaque barrière pour une d'après
    // chargement.
    let at = image(e.turn_end.adresse);
    patch::hook_with_trampoline_publie(at, e.turn_end_prologue, turn_end as *const () as usize, &TURN_END_ORIG)
        .map_err(|m| format!("rejoin: rejoin turn end: {m}, fix not installed"))?;
    BARRIERE.store(true, Ordering::Relaxed);
    patch::write_code(image(e.update.adresse), &patch::jump(update as *const () as usize))
        .map_err(|m| format!("rejoin: rejoin barrier update: {m}, fix not installed"))?;
    log(&format!("rejoin: no turn barrier after loading on clients (macOS {:#x})", e.update.adresse - BASE_EXE));
    if !rj::guard_enabled(&cfg) {
        log("rejoin: stale round guard turned off (stale_round_guard=off)");
    } else {
        match patch::write_code(image(e.round_complete.adresse), &patch::jump(round_complete as *const () as usize)) {
            Ok(_) => log("rejoin: a sync round that ended before a load no longer ends that load's barrier (macOS)"),
            Err(m) => log(&format!("rejoin: rejoin round complete: {m}, stale round guard not installed")),
        }
    }
    // Les drapeaux du snapshot (ce que fait snapshots.rs sous Windows) : sans eux, rien n'est sauté.
    let snap = verifier(&[&e.snapshot_process]).and_then(|_| verifier_cases(&[(e.snapshot_vtable, &e.snapshot_process)]));
    match snap {
        Ok(()) => {
            SNAPSHOT_ORIG.store(image(e.snapshot_process.adresse), Ordering::Release);
            if poser_case(case_process(e.snapshot_vtable), on_snapshot as Process as usize) {
                log("rejoin: snapshot flags read on arrival (NetSnapshot::Process)");
            } else {
                log("rejoin: NetSnapshot vtable not writable: no check after a load is skipped");
            }
        }
        Err(m) => log(&format!("rejoin: unexpected exe ({m}): no check after a load is skipped")),
    }
    installer_recharge(&cfg);
    Ok(())
}

unsafe fn installer_recharge(cfg: &str) {
    let e = &EXE;
    let jamais = |m: String| {
        NEVER_SKIP.store(true, Ordering::Relaxed);
        log(&m);
    };
    let fs = [&e.request_process, &e.request_ctor, &e.ack_process, &e.check_process, &e.recv];
    let cases = [(e.request_vtable, &e.request_process), (e.ack_vtable, &e.ack_process), (e.check_vtable, &e.check_process)];
    if let Err(m) = verifier(&fs).and_then(|_| verifier_cases(&cases)) {
        return jamais(format!("reload: unknown exe ({m}), not installed; every check after a load is kept"));
    }
    // Le code de la réception d'abord construit : rien n'est changé s'il ne peut pas l'être.
    let orders = ra::orders_enabled(cfg);
    let recv_at = image(e.recv.adresse);
    let code = if orders {
        let ignore = recv_at + e.recv.taille;
        match code_reception(recv_at, image(e.recv_normal), ignore, on_receive as *const () as usize) {
            Some(c) if c.len() <= e.recv.taille => Some(remplir(c, e.recv.taille)),
            _ => return jamais("reload: receive relay out of range, not installed; every check after a load is kept".into()),
        }
    } else {
        None
    };
    // Chaque original est rangé avant que sa case ne soit remplacée : la case sert aussitôt.
    for (nom, vtable, fonction, hook, original) in [
        ("snapshot request", e.request_vtable, &e.request_process, on_request as Process, &REQUEST_ORIG),
        ("sync answer", e.ack_vtable, &e.ack_process, on_ack as Process, &ACK_ORIG),
        ("sync check", e.check_vtable, &e.check_process, on_check as Process, &CHECK_ORIG),
    ] {
        original.store(image(fonction.adresse), Ordering::Release);
        if !poser_case(case_process(vtable), hook as usize) {
            return jamais(format!("reload: vtable not writable for the {nom}, not installed; every check after a load is kept"));
        }
    }
    if let Some(code) = code {
        if let Err(m) = patch::write_code(recv_at, &code) {
            return jamais(format!("reload: receive relay: {m}, not installed; every check after a load is kept"));
        }
    }
    RECHARGE.store(true, Ordering::Relaxed);
    let host = if ra::enabled(cfg) { "" } else { "; reload_all=off, not started by this machine" };
    log(&format!(
        "reload: the host starts the reload of everyone and waits for every loaded game (macOS {}){}{host}",
        std::env::consts::ARCH,
        if orders { "" } else { "; reload_orders=off" }
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exe() -> Option<Vec<u8>> {
        let home = std::env::var_os("HOME")?;
        let p = std::path::Path::new(&home).join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/Civ6_Exe_Child");
        std::fs::read(p).ok()
    }

    /// Tranche `cpu` d'un binaire universel.
    fn tranche(b: &[u8], cpu: u32) -> Option<&[u8]> {
        let be = |o: usize| b.get(o..o + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap()));
        if be(0)? != 0xcafe_babe {
            return None;
        }
        (0..be(4)? as usize).find_map(|i| {
            let e = 8 + 20 * i;
            (be(e)? == cpu).then(|| b.get(be(e + 8)? as usize..(be(e + 8)? + be(e + 12)?) as usize)).flatten()
        })
    }

    /// Décalage dans le fichier d'une adresse, par les segments du Mach-O.
    fn decalage(t: &[u8], a: usize) -> usize {
        let u32_ = |o: usize| u32::from_le_bytes(t[o..o + 4].try_into().unwrap()) as usize;
        let u64_ = |o: usize| u64::from_le_bytes(t[o..o + 8].try_into().unwrap()) as usize;
        let mut o = 32;
        for _ in 0..u32_(16) {
            if u32_(o) == 0x19 {
                let (vm, fo, fs) = (u64_(o + 24), u64_(o + 40), u64_(o + 48));
                if (vm..vm + fs).contains(&a) {
                    return fo + a - vm;
                }
            }
            o += u32_(o + 4);
        }
        panic!("{a:#x} hors des segments");
    }

    fn octets(t: &[u8], a: usize, n: usize) -> &[u8] {
        &t[decalage(t, a)..decalage(t, a) + n]
    }

    /// Une case de pointeur du fichier (pointeur brut, ou chaîné : 36 bits bas).
    fn case(t: &[u8], a: usize) -> usize {
        (u64::from_le_bytes(octets(t, a, 8).try_into().unwrap()) & 0xf_ffff_ffff) as usize
    }

    fn verifier_fichier(t: &[u8], e: &Exe) {
        for f in [
            &e.update, &e.turn_end, &e.round_complete, &e.start_check, &e.lookup, &e.ack_on_receipt, &e.set_waiting, &e.snapshot_process,
            &e.request_process, &e.request_ctor, &e.ack_process, &e.check_process, &e.recv,
        ] {
            assert_eq!(fnv(octets(t, f.adresse, f.taille)), f.empreinte, "{} en {:#x}", f.nom, f.adresse);
        }
        for (vt, f) in [(e.snapshot_vtable, &e.snapshot_process), (e.request_vtable, &e.request_process), (e.ack_vtable, &e.ack_process), (e.check_vtable, &e.check_process)] {
            assert_eq!(case(t, vt + PROCESS), f.adresse, "vtable {vt:#x} : {}", f.nom);
        }
    }

    #[test]
    fn exe_arm64() {
        let Some(b) = exe() else { return };
        let t = tranche(&b, 0x0100_000c).expect("tranche arm64");
        verifier_fichier(t, &ARM);
        let mot = |a: usize| u32::from_le_bytes(octets(t, a, 4).try_into().unwrap());
        // Le test remplacé : cbz x0 (pas de joueur) vers la suite normale, en + 0x48.
        let m = mot(ARM.recv.adresse + 0x48);
        assert_eq!(m & 0xff00_001f, 0xb400_0000, "cbz x0");
        assert_eq!(ARM.recv.adresse + 0x48 + ((((m >> 5) & 0x7_ffff) << 13) as i32 >> 11) as usize, ARM.recv_normal);
        // « (ignored, %d) » suit la zone remplacée : adrp/add de l'indicateur de journal en + 0x6c.
        assert_eq!(mot(ARM.recv.adresse + ARM.recv.taille) & 0x9f00_0000, 0x9000_0000, "adrp après la zone");
        // La mise à jour se termine dans sa taille ; le gestionnaire « tour terminé » aussi.
        assert_eq!(mot(ARM.update.adresse + 0x48), 0xd61f_0020, "br x1 en fin de mise à jour");
        assert_eq!(mot(ARM.round_complete.adresse + 0x14), 0xd65f_03c0, "ret");
    }

    #[test]
    fn exe_x86_64() {
        let Some(b) = exe() else { return };
        let t = tranche(&b, 0x0100_0007).expect("tranche x86_64");
        verifier_fichier(t, &X86);
        // Le test remplacé : je (pas de joueur) vers la suite normale, en + 0x37.
        let i = octets(t, X86.recv.adresse + 0x37, 6);
        assert_eq!(&i[..2], &[0x0f, 0x84]);
        let rel = i32::from_le_bytes(i[2..6].try_into().unwrap());
        assert_eq!((X86.recv.adresse as i64 + 0x37 + 6 + rel as i64) as usize, X86.recv_normal);
        // Le prologue recopié de « fin de tour » : sans instruction relative.
        assert_eq!(octets(t, X86.turn_end.adresse, X86.turn_end_prologue), [0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x54, 0x53, 0xc6, 0x47, 0x08, 0x01]);
    }

    #[test]
    fn le_code_de_reception_tient_et_vise_juste() {
        let e = &EXE;
        let (at, normal, ignore) = (e.recv.adresse, e.recv_normal, e.recv.adresse + e.recv.taille);
        let c = code_reception(at, normal, ignore, 0x1122_3344_5566_7788).unwrap();
        assert!(c.len() <= e.recv.taille, "{} octets", c.len());
        let r = remplir(c.clone(), e.recv.taille);
        assert_eq!((r.len(), &r[..c.len()]), (e.recv.taille, &c[..]));
        if EST_X86 {
            assert_eq!(patch::cible_rel32_x86(at + 20, &c[20..25]), Some(ignore));
            let je = i32::from_le_bytes(c[16..20].try_into().unwrap());
            assert_eq!((at as i64 + 20 + je as i64) as usize, normal);
            assert_eq!(u64::from_le_bytes(c[12 + 13..12 + 21].try_into().unwrap()), 0x1122_3344_5566_7788, "call [rip+13]");
        } else {
            let mot = |i: usize| u32::from_le_bytes(c[4 * i..4 * i + 4].try_into().unwrap());
            assert_eq!(patch::cible_branche(at + 0x14, mot(5)), Some(ignore));
            let cbz = mot(4);
            assert_eq!(at + 0x10 + ((((cbz >> 5) & 0x7_ffff) << 13) as i32 >> 11) as usize, normal);
            assert_eq!((at + 0x18) % 8, 0, "adresse alignée");
            assert_eq!(u64::from_le_bytes(c[0x18..0x20].try_into().unwrap()), 0x1122_3344_5566_7788);
        }
        assert!(e.turn_end_prologue >= patch::jump(0).len());
        assert!(e.update.taille >= patch::jump(0).len() && e.round_complete.taille >= patch::jump(0).len());
    }

    #[test]
    fn natives_attendues_par_le_bloc() {
        use super::super::salon_texte::PARTIE_BLOCK;
        for nom in ["Civ6Proxy_SyncNotes", "Civ6Proxy_ReloadAllRecheck", "Civ6Proxy_RejoinKeepCheck"] {
            assert!(PARTIE_BLOCK.contains(nom) && NATIVES_PARTIE.iter().any(|(_, n)| n.trim_end_matches('\0') == nom), "{nom}");
        }
        for nom in ["Civ6Proxy_ReloadAllReady", "Civ6Proxy_ReloadAllState", "Civ6Proxy_ReloadAllStart"] {
            assert!(PARTIE_BLOCK.contains(nom) && CALE_PARTIE.contains(&format!("{nom} = function")), "{nom}");
            let mac = nom.replace("Civ6Proxy_", "Civ6Proxy_Mac");
            assert!(CALE_PARTIE.contains(&mac) && NATIVES_PARTIE.iter().any(|(_, n)| n.trim_end_matches('\0') == mac), "{mac}");
        }
    }

    #[test]
    fn rien_sans_accroches() {
        unsafe {
            assert_eq!(state(10, 1), ra::NOT_POSSIBLE);
            assert_eq!(start(10, 1), ra::NOT_POSSIBLE);
            recheck();
            assert!(network().is_none());
        }
        assert!(!host_ready(""), "rechargement non posé");
        new_game();
        assert!(!NEVER_SKIP.load(Ordering::Relaxed));
    }
}
