//! Mémoire de `18037ec60` (Rules_Yield.cpp) : rendement d'une case pour un joueur.
//!
//! La fonction lit la case, ses 6 voisines et des tables de règles, en entiers ; elle n'écrit rien.
//! L'IA l'appelle des milliers de fois avec les mêmes arguments pendant ses évaluations parallèles.
//!
//! Ces évaluations tournent en tâches lancées par `1804a1100`, que le thread GameCore attend dans
//! `1804a16e0` en exécutant lui-même des tâches. Pendant cette attente, personne ne modifie l'état
//! de jeu : les tâches le lisent en parallèle, et le moteur l'interdit par ses assertions
//! (`IsProcessingAi`). On ne mémorise donc qu'à l'intérieur d'une attente, et chaque attente ouvre
//! une génération neuve : rien ne survit d'une attente à l'autre.
//!
//! On ne mémorise que les vraies cases de la carte : l'appelant peut passer une autre forme de case
//! (une copie modifiée, par exemple), dont l'adresse ne dit rien du contenu. La case doit donc être
//! celle que la carte rend pour ses coordonnées.
//!
//! `1804a16e0` est remplacée entièrement (son prologue contient un `lea` relatif à RIP) : le détour
//! refait ses quatre étapes, entourées de l'ouverture et de la fermeture de la génération.
//!
//! Réglage `rendements=off|count|verify|fast|quiet` dans rollover_proxy.cfg (quiet par défaut) :
//! - `count` : détours posés, originale appelée ; on compte les appels, dans et hors attente ;
//! - `verify` : on mémorise, on rend toujours l'originale et on compte les écarts avec la mémoire ;
//! - `fast` : la mémoire répond quand elle peut, comptée et chronométrée ;
//! - `quiet` : comme `fast`, sans compteur ni chronomètre ni journal.

use crate::patch::{self, Site, IMAGE_BASE};
use crate::{log, win};
use std::sync::atomic::{fence, AtomicI64, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};

const YIELD: Site = Site {
    name: "rendement d'une case",
    address: 0x1_8037_ec60,
    // mov [rsp+18h],r8d / push rbp / push rbx / push rdi / push r12 / push r13 / push r14 / push r15
    expected: &[0x44, 0x89, 0x44, 0x24, 0x18, 0x55, 0x53, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57],
    copy: (0, 16),
    resume: 16,
};

const AI_WAIT: Site = Site {
    name: "attente des tâches de l'IA",
    address: 0x1_804a_16e0,
    // push rbx / sub rsp,30h / lea rax,[1809c5fc0] / mov rbx,rcx : remplacée entièrement, aucun
    // trampoline utilisable.
    expected: &[0x40, 0x53, 0x48, 0x83, 0xec, 0x30, 0x48, 0x8d, 0x05, 0xd3, 0x48, 0x52, 0x00, 0x48, 0x8b, 0xd9],
    copy: (0, 0),
    resume: 0,
};

// Fonctions et données de l'originale, adresses Ghidra.
const MAP: u64 = 0x1_8005_26c0; // () -> carte
const JOB_WAIT: u64 = 0x1_8099_22c0; // (groupe de tâches, &{nom, 0}) : EXP_JobManager_Wait
const NULL_JOB_NAME: u64 = 0x1_809c_5fc0; // "<Null Job Name>"
const ENGINE: u64 = 0x1_80b8_aa80; // pointeur global vers l'objet moteur

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Mode {
    Off = 0,
    Count = 1,
    Verify = 2,
    Fast = 3,
    Quiet = 4,
}

static MODE: AtomicU8 = AtomicU8::new(Mode::Quiet as u8);
static BASE: AtomicUsize = AtomicUsize::new(0);
static TRAMP: AtomicUsize = AtomicUsize::new(0);
static HOOKED_BASE: AtomicUsize = AtomicUsize::new(0);
static WAIT_HOOKED: AtomicU8 = AtomicU8::new(0);

fn mode() -> Mode {
    match MODE.load(Ordering::Relaxed) {
        1 => Mode::Count,
        2 => Mode::Verify,
        3 => Mode::Fast,
        4 => Mode::Quiet,
        _ => Mode::Off,
    }
}

pub fn set_mode(m: Mode) {
    MODE.store(m as u8, Ordering::Relaxed);
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read(addr as *const T)
}

fn gc(addr: u64) -> usize {
    BASE.load(Ordering::Relaxed) + (addr - IMAGE_BASE) as usize
}

/// Attentes de l'IA en cours (0 ou 1 en pratique) et génération de la mémoire. La génération change
/// à l'entrée et à la sortie de chaque attente : une valeur n'est lue que dans l'attente où elle a
/// été calculée.
static IN_WAIT: AtomicU32 = AtomicU32::new(0);
static GENERATION: AtomicU32 = AtomicU32::new(1);

unsafe extern "system" fn hook_wait(this: usize) {
    GENERATION.fetch_add(1, Ordering::AcqRel);
    IN_WAIT.fetch_add(1, Ordering::AcqRel);
    let job: [usize; 2] = [gc(NULL_JOB_NAME), 0];
    let wait: unsafe extern "system" fn(usize, *const [usize; 2]) = std::mem::transmute(gc(JOB_WAIT));
    wait(this + 0x400, &job);
    IN_WAIT.fetch_sub(1, Ordering::AcqRel);
    GENERATION.fetch_add(1, Ordering::AcqRel);
    // La suite de l'originale : drapeau « IA en cours » baissé, notification au moteur si demandée.
    let notify = rd::<u8>(this + 0x38ad);
    *((this + 0x38ac) as *mut u8) = 0;
    if notify != 0 {
        let engine = rd::<usize>(gc(ENGINE));
        let f: unsafe extern "system" fn(usize) = std::mem::transmute(rd::<usize>(rd::<usize>(engine) + 0x40));
        f(engine);
    }
    *((this + 0x38ad) as *mut u8) = 0;
}

// Mémoire : table à adressage direct, une case par empreinte, protégée par un compteur de version
// (pair : stable, impair : écriture en cours). Une écriture concurrente sur la même case est
// abandonnée : ce n'est qu'une mémoire.
const SLOTS: usize = 1 << 16;

struct Slot {
    version: AtomicU64,
    key: AtomicU64,
    /// Résultat (32 bits bas) et génération (32 bits hauts).
    value: AtomicU64,
}

static TABLE: [Slot; SLOTS] =
    [const { Slot { version: AtomicU64::new(0), key: AtomicU64::new(0), value: AtomicU64::new(0) } }; SLOTS];

/// Clé complète sur 64 bits, ou None si un argument sort des bornes prévues.
fn key(plot: usize, yield_type: usize, player: usize, flag: usize) -> Option<u64> {
    let (yield_type, player) = (yield_type as u32, player as u32);
    let player = if player == u32::MAX { 0x7f } else { player };
    if plot & 7 != 0 || plot >> 47 != 0 || yield_type >= 16 || player >= 0x7f {
        return None;
    }
    Some(((plot as u64 >> 3) << 12 | (yield_type as u64) << 8 | (player as u64) << 1 | ((flag as u8 != 0) as u64)) + 1)
}

fn slot(key: u64) -> &'static Slot {
    &TABLE[(key.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - SLOTS.trailing_zeros())) as usize]
}

fn lookup(key: u64, generation: u32) -> Option<u32> {
    let s = slot(key);
    let v1 = s.version.load(Ordering::Acquire);
    if v1 & 1 != 0 {
        return None;
    }
    let k = s.key.load(Ordering::Relaxed);
    let value = s.value.load(Ordering::Relaxed);
    fence(Ordering::Acquire);
    if s.version.load(Ordering::Relaxed) != v1 || k != key || (value >> 32) as u32 != generation {
        return None;
    }
    Some(value as u32)
}

fn store(key: u64, generation: u32, result: u32) {
    let s = slot(key);
    let v = s.version.load(Ordering::Relaxed);
    if v & 1 != 0 || s.version.compare_exchange(v, v + 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        return;
    }
    fence(Ordering::Release);
    s.key.store(key, Ordering::Relaxed);
    s.value.store((generation as u64) << 32 | result as u64, Ordering::Relaxed);
    s.version.store(v + 2, Ordering::Release);
}

/// La case est-elle celle que la carte rend pour ses coordonnées ?
unsafe fn is_map_plot(plot: usize) -> bool {
    let map: unsafe extern "system" fn() -> usize = std::mem::transmute(gc(MAP));
    let map = map();
    if map == 0 {
        return false;
    }
    let xy: [i32; 2] = [rd::<i16>(plot + 0x28) as i32, rd::<i16>(plot + 0x2a) as i32];
    let get: unsafe extern "system" fn(usize, *const [i32; 2]) -> usize = std::mem::transmute(rd::<usize>(rd::<usize>(map) + 0x50));
    get(map, &xy) == plot
}

type YieldFn = unsafe extern "system" fn(usize, usize, usize, usize, usize) -> usize;

unsafe fn original(this: usize, plot: usize, yield_type: usize, player: usize, flag: usize) -> usize {
    let f: YieldFn = std::mem::transmute(TRAMP.load(Ordering::Relaxed));
    f(this, plot, yield_type, player, flag)
}

// Statistiques par rafale d'appels, comme pour les améliorations.
static CALLS: AtomicU64 = AtomicU64::new(0);
static IN_WAIT_CALLS: AtomicU64 = AtomicU64::new(0);
static KEYED: AtomicU64 = AtomicU64::new(0);
static HITS: AtomicU64 = AtomicU64::new(0);
static MISMATCHES: AtomicU64 = AtomicU64::new(0);
static TICKS: AtomicU64 = AtomicU64::new(0);
static LAST: AtomicI64 = AtomicI64::new(0);

fn flush() {
    let calls = CALLS.swap(0, Ordering::Relaxed);
    let in_wait = IN_WAIT_CALLS.swap(0, Ordering::Relaxed);
    let keyed = KEYED.swap(0, Ordering::Relaxed);
    let hits = HITS.swap(0, Ordering::Relaxed);
    let mismatches = MISMATCHES.swap(0, Ordering::Relaxed);
    let ticks = TICKS.swap(0, Ordering::Relaxed);
    if calls > 0 {
        log(&format!(
            "rendements mode={:?} appels={calls} dans_attente={in_wait} cases_carte={keyed} trouves={hits} \
             ecarts={mismatches} ms={:.1}",
            mode(),
            win::qpc_ms(ticks as i64),
        ));
    }
}

unsafe extern "system" fn hook(this: usize, plot: usize, yield_type: usize, player: usize, flag: usize) -> usize {
    let m = mode();
    let quiet = m == Mode::Quiet;
    let t0 = if quiet { 0 } else { win::qpc() };
    if !quiet {
        let last = LAST.swap(t0, Ordering::Relaxed);
        if last != 0 && win::qpc_ms(t0 - last) > 1000.0 {
            flush();
        }
        CALLS.fetch_add(1, Ordering::Relaxed);
    }
    let generation = GENERATION.load(Ordering::Acquire);
    let k = if IN_WAIT.load(Ordering::Acquire) == 0 {
        None
    } else {
        if !quiet {
            IN_WAIT_CALLS.fetch_add(1, Ordering::Relaxed);
        }
        key(plot, yield_type, player, flag).filter(|_| is_map_plot(plot))
    };
    let r = match (m, k) {
        (_, None) => original(this, plot, yield_type, player, flag),
        (Mode::Count, Some(_)) => {
            KEYED.fetch_add(1, Ordering::Relaxed);
            original(this, plot, yield_type, player, flag)
        }
        (Mode::Verify, Some(k)) => {
            KEYED.fetch_add(1, Ordering::Relaxed);
            let o = original(this, plot, yield_type, player, flag);
            match lookup(k, generation) {
                Some(v) => {
                    HITS.fetch_add(1, Ordering::Relaxed);
                    if v as usize != o {
                        MISMATCHES.fetch_add(1, Ordering::Relaxed);
                    }
                }
                None => store(k, generation, o as u32),
            }
            o
        }
        (_, Some(k)) => {
            if !quiet {
                KEYED.fetch_add(1, Ordering::Relaxed);
            }
            match lookup(k, generation) {
                Some(v) => {
                    if !quiet {
                        HITS.fetch_add(1, Ordering::Relaxed);
                    }
                    v as usize
                }
                None => {
                    let o = original(this, plot, yield_type, player, flag);
                    store(k, generation, o as u32);
                    o
                }
            }
        }
    };
    if !quiet {
        TICKS.fetch_add((win::qpc() - t0) as u64, Ordering::Relaxed);
    }
    r
}

/// Appelé quand l'originale vient d'être chargée, avant le premier appel relayé.
pub unsafe fn on_gamecore_loaded(base: usize) {
    if mode() == Mode::Off {
        return;
    }
    if let Err(e) = patch::check(base, &[YIELD, AI_WAIT]) {
        log(&format!("rendements : version inattendue, aucun détour : {e}"));
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    // L'attente d'abord : sans elle, la mémoire ne s'ouvrirait jamais.
    let r = patch::install(base, &AI_WAIT, hook_wait as *const () as usize).and_then(|_| {
        WAIT_HOOKED.store(1, Ordering::Relaxed);
        patch::install(base, &YIELD, hook as *const () as usize)
    });
    HOOKED_BASE.store(base, Ordering::Relaxed);
    match r {
        Ok(t) => {
            TRAMP.store(t, Ordering::Relaxed);
            log(&format!("rendements : détours posés, mode {:?}", mode()));
        }
        Err(e) => log(&format!("rendements : échec de la pose des détours : {e}")),
    }
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet ses octets d'origine.
pub unsafe fn on_proxy_unload() {
    let base = HOOKED_BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        if TRAMP.load(Ordering::Relaxed) != 0 {
            patch::restore(base, &YIELD);
        }
        if WAIT_HOOKED.load(Ordering::Relaxed) != 0 {
            patch::restore(base, &AI_WAIT);
        }
    }
}
