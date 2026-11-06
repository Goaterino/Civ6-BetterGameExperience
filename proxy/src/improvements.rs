//! Réécriture de `180260720` (Improvement_Yields.cpp) : « ce joueur peut-il construire cette
//! amélioration ? ».
//!
//! L'originale vérifie les prérequis de technologie et de dogme, puis parcourt une liste de la base
//! de règles (`1807d0a80`, entrées : nom d'amélioration en +0x20, trait en +0x28) en comparant par
//! `strcmp` le nom de l'amélioration à celui de chaque entrée. Pour chaque entrée trouvée, la
//! réponse est non si la civilisation ou le dirigeant du joueur a ce trait.
//!
//! Le parcours par `strcmp` ne dépend que de l'amélioration : on le précalcule une fois par
//! amélioration (table amélioration → traits, dans l'ordre de la liste). La civilisation et le
//! dirigeant du joueur, que l'originale recherche à chaque trait trouvé, sont mémorisés par joueur.
//! Les prérequis sont refaits à l'identique, avec les mêmes fonctions de l'originale. La fonction ne
//! fait que lire l'état de jeu.
//!
//! Les mémoires sont lues sans verrou ni compteur de références : la fonction est appelée des
//! millions de fois par passage de tour, depuis une douzaine de threads.
//!
//! Réglage `ameliorations=off|count|verify|fast|quiet` dans rollover_proxy.cfg (quiet par défaut) :
//! - `count` : détour posé, originale appelée, on compte et on chronomètre ;
//! - `verify` : on calcule les deux versions, on rend celle de l'originale et on compte les écarts ;
//! - `fast` : version précalculée seule, comptée et chronométrée ;
//! - `quiet` : version précalculée seule, sans compteur ni chronomètre ni journal (pour jouer).

use crate::patch::{self, Site, IMAGE_BASE};
use crate::{log, win};
use std::sync::atomic::{AtomicI64, AtomicPtr, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;

const CAN_BUILD: Site = Site {
    name: "amélioration constructible",
    address: 0x1_8026_0720,
    // push r14 / sub rsp,30h / mov r14,r8 / test r8,r8 / jnz +9 : le détour écarte lui-même le
    // pointeur nul, le trampoline reprend donc en 180260737, la suite du cas non nul.
    expected: &[0x41, 0x56, 0x48, 0x83, 0xec, 0x30, 0x4d, 0x8b, 0xf0, 0x4d, 0x85, 0xc0, 0x75, 0x09],
    copy: (0, 9),
    resume: 0x17,
};

// Fonctions et donnée de l'originale, adresses Ghidra.
const PLAYER: u64 = 0x1_8004_4f00; // (id joueur) -> joueur
const HAS_TECH: u64 = 0x1_8028_1280; // (techs du joueur, index) -> bool
const HAS_CIVIC: u64 = 0x1_8033_4cc0; // (dogmes du joueur, index) -> bool
const LIST: u64 = 0x1_807d_0a80; // () -> &[début, fin) des entrées (nom, trait)
const PLAYER_CONFIG: u64 = 0x1_8016_4d60; // (configuration + 0xd80, id) -> configuration du joueur
const CIV_TYPE: u64 = 0x1_8016_9dc0; // (configuration du joueur) -> type de civilisation
const LEADER_TYPE: u64 = 0x1_8016_a000; // (configuration du joueur) -> type de dirigeant
const CIVS: u64 = 0x1_807d_0640; // () -> table des civilisations
const LEADERS: u64 = 0x1_807d_0fc0; // () -> table des dirigeants
const FIND: u64 = 0x1_8022_09a0; // (table, type) -> ligne ou nul
const CONFIG: u64 = 0x1_80b8_aa60; // pointeur global relu à chaque entrée trouvée

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

/// Adresse dans l'originale chargée d'une fonction ou donnée repérée dans Ghidra.
fn gc(addr: u64) -> usize {
    BASE.load(Ordering::Relaxed) + (addr - IMAGE_BASE) as usize
}

macro_rules! call {
    ($addr:expr, fn($($t:ty),*) -> $r:ty, $($a:expr),*) => {{
        let f: unsafe extern "system" fn($($t),*) -> $r = std::mem::transmute(gc($addr));
        f($($a),*)
    }};
}

/// Une amélioration et ses traits, dans l'ordre de la liste. Jamais libérée : une entrée publiée peut
/// être lue sans verrou par tous les threads.
struct Entry {
    improvement: usize,
    /// Pointeur du nom de l'amélioration au moment du calcul, revérifié à chaque appel.
    name: usize,
    traits: Box<[usize]>,
}

/// Lignes de civilisation et de dirigeant d'un joueur, valables pour sa configuration `pc`.
struct PlayerRows {
    pc: usize,
    civ: usize,
    leader: usize,
}

const SLOTS: usize = 1024; // puissance de 2, bien plus que les ~110 améliorations vues en partie
const MAX_ENTRIES: usize = SLOTS / 2;
const PLAYERS: usize = 64;

/// Mémoires d'une partie, lues sans verrou. On en crée une nouvelle quand la liste des entrées
/// (nom, trait) change d'adresse ou de taille, ou à chaque nouvelle partie ; l'ancienne est
/// abandonnée sans être libérée, puisqu'un autre thread peut encore la lire.
struct Memo {
    list: (usize, usize),
    slots: [AtomicPtr<Entry>; SLOTS],
    entries: AtomicUsize,
    players: [AtomicPtr<PlayerRows>; PLAYERS],
}

static MEMO: AtomicPtr<Memo> = AtomicPtr::new(std::ptr::null_mut());
/// Sérialise les écritures (rares) ; les lectures n'en ont pas besoin.
static WRITE: Mutex<()> = Mutex::new(());

fn write_lock() -> std::sync::MutexGuard<'static, ()> {
    WRITE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Abandonne les mémoires : appelé à chaque nouvelle partie (DllCreateGameContext).
pub fn reset() {
    let _w = write_lock();
    MEMO.store(std::ptr::null_mut(), Ordering::Release);
}

fn memo(list: (usize, usize)) -> &'static Memo {
    let m = MEMO.load(Ordering::Acquire);
    if !m.is_null() && unsafe { (*m).list } == list {
        return unsafe { &*m };
    }
    let _w = write_lock();
    let m = MEMO.load(Ordering::Acquire);
    if !m.is_null() && unsafe { (*m).list } == list {
        return unsafe { &*m };
    }
    let fresh: &'static Memo = Box::leak(Box::new(Memo {
        list,
        slots: [const { AtomicPtr::new(std::ptr::null_mut()) }; SLOTS],
        entries: AtomicUsize::new(0),
        players: [const { AtomicPtr::new(std::ptr::null_mut()) }; PLAYERS],
    }));
    MEMO.store(fresh as *const Memo as *mut Memo, Ordering::Release);
    fresh
}

fn slot(improvement: usize) -> usize {
    (improvement >> 4).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - SLOTS.trailing_zeros())
}

unsafe fn same_cstr(a: usize, b: usize) -> bool {
    let (a, b) = (a as *const u8, b as *const u8);
    let mut i = 0;
    loop {
        let (x, y) = (*a.add(i), *b.add(i));
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
        i += 1;
    }
}

/// Le parcours par `strcmp` de l'originale : traits des entrées dont le nom est celui de l'amélioration.
unsafe fn scan(list: (usize, usize), name: usize) -> Vec<usize> {
    let mut traits = Vec::new();
    let mut e = list.0;
    while e != list.1 {
        let entry = rd::<usize>(e);
        if same_cstr(rd::<usize>(entry + 0x20), name) {
            traits.push(rd::<usize>(entry + 0x28));
        }
        e += 8;
    }
    traits
}

enum Traits {
    Cached(&'static [usize]),
    Fresh(Vec<usize>),
}

impl std::ops::Deref for Traits {
    type Target = [usize];
    fn deref(&self) -> &[usize] {
        match self {
            Traits::Cached(t) => t,
            Traits::Fresh(t) => t,
        }
    }
}

unsafe fn traits_of(m: &'static Memo, improvement: usize) -> Traits {
    let name = rd::<usize>(improvement + 0x30);
    let mut i = slot(improvement);
    loop {
        let e = m.slots[i].load(Ordering::Acquire);
        if e.is_null() {
            break;
        }
        let e = &*e;
        if e.improvement == improvement {
            if e.name == name {
                return Traits::Cached(&e.traits);
            }
            // Même adresse, autre nom : objet réutilisé sans changement de liste. On ne mémorise pas.
            return Traits::Fresh(scan(m.list, name));
        }
        i = (i + 1) & (SLOTS - 1);
    }
    let traits = scan(m.list, name);
    let _w = write_lock();
    if m.entries.load(Ordering::Relaxed) >= MAX_ENTRIES {
        return Traits::Fresh(traits);
    }
    let mut i = slot(improvement);
    loop {
        let e = m.slots[i].load(Ordering::Acquire);
        if e.is_null() {
            let e: &'static Entry = Box::leak(Box::new(Entry { improvement, name, traits: traits.into() }));
            m.slots[i].store(e as *const Entry as *mut Entry, Ordering::Release);
            m.entries.fetch_add(1, Ordering::Relaxed);
            return Traits::Cached(&e.traits);
        }
        if (*e).improvement == improvement {
            return Traits::Fresh(traits); // inséré entre-temps par un autre thread
        }
        i = (i + 1) & (SLOTS - 1);
    }
}

/// Lignes de civilisation et de dirigeant du joueur. La civilisation et le dirigeant ne changent pas
/// en cours de partie : on les mémorise par joueur, et on revérifie à chaque appel que la configuration
/// du joueur est le même objet. L'originale les recalcule à chaque trait trouvé.
unsafe fn civ_and_leader(m: &'static Memo, player: usize) -> (usize, usize) {
    let id = rd::<u32>(player + 0xd8);
    let config = rd::<usize>(gc(CONFIG));
    let pc = call!(PLAYER_CONFIG, fn(usize, u32) -> usize, config + 0xd80, id);
    let cached = m.players.get(id as usize);
    if let Some(p) = cached.map(|c| c.load(Ordering::Acquire)).filter(|p| !p.is_null()) {
        if (*p).pc == pc {
            return ((*p).civ, (*p).leader);
        }
    }
    let civ_type = call!(CIV_TYPE, fn(usize) -> u32, pc);
    let civ = call!(FIND, fn(usize, u32) -> usize, call!(CIVS, fn() -> usize,), civ_type);
    let leader_type = call!(LEADER_TYPE, fn(usize) -> u32, pc);
    let leader = call!(FIND, fn(usize, u32) -> usize, call!(LEADERS, fn() -> usize,), leader_type);
    if let Some(c) = cached {
        let rows: &'static PlayerRows = Box::leak(Box::new(PlayerRows { pc, civ, leader }));
        c.store(rows as *const PlayerRows as *mut PlayerRows, Ordering::Release);
    }
    (civ, leader)
}

unsafe fn contains(begin: usize, end: usize, value: usize) -> bool {
    let mut p = begin;
    while p != end {
        if rd::<usize>(p) == value {
            return true;
        }
        p += 8;
    }
    false
}

/// Même réponse que l'originale, sans le parcours par `strcmp` ni la recherche répétée de la
/// civilisation et du dirigeant.
unsafe fn fast(player_id: u32, imp: usize) -> u8 {
    let player = call!(PLAYER, fn(u32) -> usize, player_id);
    if rd::<i32>(imp + 0x1c) == 0 {
        return 0;
    }
    // Technologie requise, puis technologie qui rend obsolète.
    let t = rd::<usize>(imp + 0xf0);
    if t != 0 && call!(HAS_TECH, fn(usize, u32) -> u8, rd::<usize>(player + 0x6f0), rd::<u32>(t + 0x14)) == 0 {
        return 0;
    }
    let t = rd::<usize>(imp + 0xe0);
    if t != 0 && call!(HAS_TECH, fn(usize, u32) -> u8, rd::<usize>(player + 0x6f0), rd::<u32>(t + 0x14)) != 0 {
        return 0;
    }
    // Dogme requis, puis dogme qui rend obsolète.
    let c = rd::<usize>(imp + 0xf8);
    if c != 0 && call!(HAS_CIVIC, fn(usize, u32) -> u8, rd::<usize>(player + 0x718), rd::<u32>(c + 0x14)) == 0 {
        return 0;
    }
    let c = rd::<usize>(imp + 0xe8);
    if c != 0 && call!(HAS_CIVIC, fn(usize, u32) -> u8, rd::<usize>(player + 0x718), rd::<u32>(c + 0x14)) != 0 {
        return 0;
    }
    let list_ptr = call!(LIST, fn() -> usize,);
    let m = memo((rd::<usize>(list_ptr), rd::<usize>(list_ptr + 8)));
    let traits = traits_of(m, imp);
    if traits.is_empty() {
        return 1;
    }
    // L'originale rend 0 dès que la civilisation ou le dirigeant est introuvable, ou a l'un des traits.
    let (civ, leader) = civ_and_leader(m, player);
    if civ == 0 || leader == 0 {
        return 0;
    }
    for &tr in traits.iter() {
        if contains(rd(civ + 0x108), rd(civ + 0x110), tr) || contains(rd(leader + 0xb8), rd(leader + 0xc0), tr) {
            return 0;
        }
    }
    1
}

unsafe fn original(a1: usize, player: usize, imp: usize) -> u8 {
    let f: unsafe extern "system" fn(usize, usize, usize) -> u8 = std::mem::transmute(TRAMP.load(Ordering::Relaxed));
    f(a1, player, imp)
}

// Statistiques par rafale d'appels : une rafale se termine après une seconde sans appel, et elle est
// journalisée au premier appel de la suivante.
static CALLS: AtomicU64 = AtomicU64::new(0);
static TICKS: AtomicU64 = AtomicU64::new(0);
static MISMATCHES: AtomicU64 = AtomicU64::new(0);
static LAST: AtomicI64 = AtomicI64::new(0);

fn flush() {
    let calls = CALLS.swap(0, Ordering::Relaxed);
    let ticks = TICKS.swap(0, Ordering::Relaxed);
    let mismatches = MISMATCHES.swap(0, Ordering::Relaxed);
    if calls > 0 {
        let m = MEMO.load(Ordering::Acquire);
        let cached = if m.is_null() { 0 } else { unsafe { (*m).entries.load(Ordering::Relaxed) } };
        log(&format!(
            "ameliorations mode={:?} appels={calls} ms={:.1} ecarts={mismatches} table={cached}",
            mode(),
            win::qpc_ms(ticks as i64),
        ));
    }
}

unsafe extern "system" fn hook(a1: usize, player: usize, imp: usize) -> u8 {
    if imp == 0 {
        return 0;
    }
    let m = mode();
    if m == Mode::Quiet {
        return fast(player as u32, imp);
    }
    let t0 = win::qpc();
    let last = LAST.swap(t0, Ordering::Relaxed);
    if last != 0 && win::qpc_ms(t0 - last) > 1000.0 {
        flush();
    }
    let r = match m {
        Mode::Fast => fast(player as u32, imp),
        Mode::Verify => {
            let o = original(a1, player, imp);
            if fast(player as u32, imp) != o {
                MISMATCHES.fetch_add(1, Ordering::Relaxed);
            }
            o
        }
        _ => original(a1, player, imp),
    };
    CALLS.fetch_add(1, Ordering::Relaxed);
    TICKS.fetch_add((win::qpc() - t0) as u64, Ordering::Relaxed);
    r
}

/// Appelé quand l'originale vient d'être chargée, avant le premier appel relayé.
pub unsafe fn on_gamecore_loaded(base: usize) {
    if mode() == Mode::Off {
        return;
    }
    let sites = [CAN_BUILD];
    if let Err(e) = patch::check(base, &sites) {
        log(&format!("ameliorations : version inattendue, aucun détour : {e}"));
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    match patch::install(base, &CAN_BUILD, hook as *const () as usize) {
        Ok(t) => {
            TRAMP.store(t, Ordering::Relaxed);
            HOOKED_BASE.store(base, Ordering::Relaxed);
            log(&format!("ameliorations : détour posé, mode {:?}", mode()));
        }
        Err(e) => log(&format!("ameliorations : échec de la pose du détour : {e}")),
    }
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet ses octets d'origine.
pub unsafe fn on_proxy_unload() {
    let base = HOOKED_BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        patch::restore(base, &CAN_BUILD);
    }
}
