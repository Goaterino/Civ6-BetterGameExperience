//! Unités promises à l'IA pour ses prochains quartiers : leur table est tenue dans un ordre fixe
//! (`unites_promises=on`, par défaut).
//!
//! `Player::Units.m_aiAddOnDistrictFounding` (AutoVariable en `Player::Units + 0x2c8`, table en
//! `+0x2d8`) est une `hash_map<hash de quartier, hash_map<hash d'unité, i32 restant>>` EASTL : les
//! unités promises pour un quartier (lignes de `MajorStartingUnits` avec `OnDistrictCreated`).
//! C'est la seule variable synchronisée de GameCore_XP2 sérialisée dans l'ordre des seaux
//! (buckets) de la table. Cet ordre dépend de l'histoire de chaque machine, pas seulement du
//! contenu : un chargement garde l'ancien tableau de seaux de la table externe et réinsère chaque
//! élément en tête de sa chaîne, ce qui inverse les clés en collision. Deux conséquences :
//!
//! - Fausse désynchro. Le hash de synchro (vtable de la variable `+0x38`, `180351620`) est un
//!   FNV-1a des octets sérialisés : un même contenu donne un autre hash après un rechargement
//!   (`914d5dd7` contre `51caacc7` pour `{CITY_CENTER: {SETTLER: 0, BUILDER: 0}}`), et aucun delta
//!   ne peut le réparer.
//! - Vraie divergence. À l'achèvement d'un quartier, `180192c20` parcourt la table interne dans
//!   l'ordre des seaux et crée une unité par entrée non nulle. Le colon et le bâtisseur
//!   partagent le seau 0 d'une table à 2 seaux : après un rechargement, les deux unités sont
//!   créées dans l'ordre inverse et échangent leurs ID.
//!
//! Le correctif, sur quatre sites (adresses du GameCore_XP2 1.0.12.72, traduites par `patch`) :
//! - `180345a80` / `180345970` (écriture de la table externe / interne) : remplacées par des
//!   écritures au même format (`u32` nombre, puis `u32` clé + table interne, ou `u32` clé + `i32`
//!   valeur), triées par clé. Sauvegarde, delta et hash ne dépendent plus de l'ordre des seaux.
//!   Le jeu d'origine charge toujours ces sauvegardes : sa lecture accepte n'importe quel ordre.
//! - `1803462a0` (lecture, pour les chargements et les deltas reçus) et `18034cb20`
//!   (`table[a][b] += n`, la seule mutation) : après l'originale, chaque chaîne de la table externe
//!   et de chaque table interne est triée par clé, en reliant autrement ses nœuds (sans
//!   allocation). Une table interne part toujours d'un seau et ne grandit que par insertions,
//!   sans jamais perdre de clé : son nombre de seaux ne dépend que de sa taille et, chaînes
//!   triées, son ordre de parcours est le même sur toutes les machines. `18034cb20` ne retrie que
//!   si un nombre d'éléments a changé : dans la boucle de création, il ne fait que décrémenter des
//!   clés existantes, et relier autrement la chaîne en cours de parcours casserait ce parcours.
//!
//! Correctif POL : il change la simulation (ordre des créations) et le hash de synchro, il n'est
//! appliqué que si `pol::actifs()` (tous les humains ont ce proxy à la même version, ou partie
//! locale). Sinon, chaque détour appelle l'originale.
//!
//! Pas de réglage joueur : seuls les builds de diagnostic (feature `diag`) lisent
//! `unites_promises=on|count|off` dans rollover_proxy.cfg. `count` pose les détours, ne change rien
//! et journalise quand l'ordre d'origine diffère de l'ordre canonique ; `off` et `count` doivent être
//! les mêmes sur toutes les machines d'une partie.

use crate::patch::{self, Site};
use crate::{log, pol};
use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};

// ------------------------------------------------------------- structures (GameCore_XP2 1.0.12.72)

/// Table de hachage EASTL telle que GameCore la range (0x28 octets) : 8 octets de bases vides, le
/// tableau de seaux (`bucket_count` entrées puis une sentinelle de fin `!0`), des comptes sur 32
/// bits, la politique de redimensionnement (nombres premiers) et l'allocateur.
#[repr(C)]
pub struct Table {
    pub bases: u64,
    pub buckets: *mut usize,
    pub bucket_count: u32,
    pub count: u32,
    pub max_load: f32,
    pub growth: f32,
    pub next_resize: u32,
    pub alloc: u32,
}

/// Nœud interne : hash du type d'unité, unités encore dues, suivant (0x10 octets).
#[repr(C)]
pub struct Inner {
    pub key: u32,
    pub value: i32,
    pub next: *mut Inner,
}

/// Nœud externe : hash du type de quartier, table interne, suivant (0x38 octets).
#[repr(C)]
pub struct Outer {
    pub key: u32,
    pub pad: u32,
    pub table: Table,
    pub next: *mut Outer,
}

const _: () = assert!(std::mem::size_of::<Table>() == 0x28);
const _: () = assert!(std::mem::size_of::<Inner>() == 0x10);
const _: () = assert!(std::mem::size_of::<Outer>() == 0x38);

/// `Player::Units + 0x2c8` (AutoVariable, vérifié dans `18034cb20`) + `0x10` (sa valeur, vérifié
/// dans `18072a920`).
pub const TABLE_OFFSET: usize = 0x2c8 + 0x10;
/// Au-delà, ce n'est pas cette table (ou la mémoire n'est pas ce qu'on croit) : l'originale.
const MAX_ELEMENTS: u32 = 1 << 16;
const END_SENTINEL: usize = usize::MAX;

pub trait Node {
    fn key(&self) -> u32;
    fn next(&mut self) -> &mut *mut Self;
}
impl Node for Inner {
    fn key(&self) -> u32 {
        self.key
    }
    fn next(&mut self) -> &mut *mut Self {
        &mut self.next
    }
}
impl Node for Outer {
    fn key(&self) -> u32 {
        self.key
    }
    fn next(&mut self) -> &mut *mut Self {
        &mut self.next
    }
}

/// Les nœuds de `t` dans l'ordre de parcours d'origine (seau par seau, chaque chaîne dans
/// son ordre). None si la table ne ressemble pas à une table EASTL de cette forme.
pub unsafe fn nodes<N: Node>(t: &Table) -> Option<Vec<*mut N>> {
    if t.buckets.is_null() || t.bucket_count == 0 || t.count > MAX_ELEMENTS || t.bucket_count > 4 * MAX_ELEMENTS {
        return None;
    }
    if *t.buckets.add(t.bucket_count as usize) != END_SENTINEL {
        return None;
    }
    let mut out = Vec::with_capacity(t.count as usize);
    for i in 0..t.bucket_count as usize {
        let mut n = *t.buckets.add(i) as *mut N;
        while !n.is_null() {
            if out.len() >= t.count as usize {
                return None;
            }
            out.push(n);
            n = *(*n).next();
        }
    }
    (out.len() == t.count as usize).then_some(out)
}

/// Trie chaque chaîne de seau de `t` par clé, en reliant les nœuds sur place. Rend le nombre de
/// chaînes reliées autrement (avec `apply`) ou qui le seraient (sans) ; None si la table semble
/// fausse.
pub unsafe fn sort_chains<N: Node>(t: &mut Table, apply: bool) -> Option<u32> {
    nodes::<N>(t)?; // valide la forme avant toute écriture
    let mut changed = 0;
    for i in 0..t.bucket_count as usize {
        let slot = t.buckets.add(i);
        let mut chain: Vec<*mut N> = Vec::new();
        let mut n = *slot as *mut N;
        while !n.is_null() {
            chain.push(n);
            n = *(*n).next();
        }
        if chain.windows(2).all(|w| (*w[0]).key() <= (*w[1]).key()) {
            continue;
        }
        changed += 1;
        if apply {
            chain.sort_by_key(|&n| (*n).key());
            *slot = chain[0] as usize;
            for w in chain.windows(2) {
                *(*w[0]).next() = w[1];
            }
            *(**chain.last().unwrap()).next() = std::ptr::null_mut();
        }
    }
    Some(changed)
}

/// Forme canonique de toute la table : chaînes externes et chaînes de chaque table interne triées.
pub unsafe fn canonicalize(t: &mut Table, apply: bool) -> Option<u32> {
    let mut changed = sort_chains::<Outer>(t, apply)?;
    for o in nodes::<Outer>(t)? {
        changed += sort_chains::<Inner>(&mut (*o).table, apply)?;
    }
    Some(changed)
}

// ------------------------------------------------------------------ écriture

/// Le flux du jeu : l'entrée `+0x28` de sa vtable est `write(this, tampon, longueur)`.
#[repr(C)]
pub struct Stream {
    pub vtable: *const usize,
}
type Write = unsafe extern "system" fn(*mut Stream, *const u8, u64) -> usize;

unsafe fn put(s: *mut Stream, v: &[u8; 4]) {
    let w: Write = std::mem::transmute(*(*s).vtable.add(0x28 / 8));
    w(s, v.as_ptr(), 4);
}

unsafe fn sorted<N: Node>(t: &Table) -> Option<Vec<*mut N>> {
    let mut v = nodes::<N>(t)?;
    v.sort_by_key(|&n| (*n).key());
    Some(v)
}

/// Mêmes octets que `180345970`, entrées triées par clé.
pub unsafe fn write_inner_sorted(s: *mut Stream, t: &Table) -> Option<()> {
    let v = sorted::<Inner>(t)?;
    put(s, &t.count.to_le_bytes());
    for n in v {
        put(s, &(*n).key.to_le_bytes());
        put(s, &(*n).value.to_le_bytes());
    }
    Some(())
}

/// Mêmes octets que `180345a80`, entrées triées par clé aux deux niveaux. Les tables internes sont
/// validées avant le premier octet écrit : un retour à l'originale ne laisse jamais un flux à
/// moitié écrit.
pub unsafe fn write_outer_sorted(s: *mut Stream, t: &Table) -> Option<()> {
    let v = sorted::<Outer>(t)?;
    for &o in &v {
        nodes::<Inner>(&(*o).table)?;
    }
    put(s, &t.count.to_le_bytes());
    for o in v {
        put(s, &(*o).key.to_le_bytes());
        write_inner_sorted(s, &(*o).table)?;
    }
    Some(())
}

// ------------------------------------------------------------------ sites

const WRITE_INNER: Site = Site {
    name: "promised units: inner writer",
    address: 0x1_8034_5970,
    // mov [rsp+18h],rbx / mov [rsp+20h],rbp / push rsi / sub rsp,30h (recopiés), puis
    // mov eax,[rdx+14h] (nombre) ... call [rax+28h] (write) / mov rcx,[rbx+8] / mov eax,[rbx+10h].
    expected: &[
        0x48, 0x89, 0x5C, 0x24, 0x18, 0x48, 0x89, 0x6C, 0x24, 0x20, 0x56, 0x48, 0x83, 0xEC, 0x30, 0x8B, 0x42, 0x14,
        0x48, 0x8B, 0xDA, 0x89, 0x44, 0x24, 0x40, 0x48, 0x8D, 0x54, 0x24, 0x40, 0x48, 0x8B, 0x01, 0x41, 0xB8, 0x04,
        0x00, 0x00, 0x00, 0x48, 0x8B, 0xF1, 0xFF, 0x50, 0x28, 0x48, 0x8B, 0x4B, 0x08, 0x8B, 0x43, 0x10, 0x48, 0x89,
        0x4C, 0x24, 0x28,
    ],
    copy: (0, 15),
    resume: 15,
};

const WRITE_OUTER: Site = Site {
    name: "promised units: outer writer",
    address: 0x1_8034_5a80,
    // mov r11,rsp / push rbx / push r12 / push r15 / sub rsp,80h (recopiés), puis la même
    // écriture du nombre et les mêmes lectures du tableau de seaux que l'écriture interne.
    expected: &[
        0x4C, 0x8B, 0xDC, 0x53, 0x41, 0x54, 0x41, 0x57, 0x48, 0x81, 0xEC, 0x80, 0x00, 0x00, 0x00, 0x8B, 0x42, 0x14,
        0x48, 0x8B, 0xDA, 0x41, 0x89, 0x43, 0x08, 0x49, 0x8D, 0x53, 0x08, 0x48, 0x8B, 0x01, 0x41, 0xB8, 0x04, 0x00,
        0x00, 0x00, 0x4C, 0x8B, 0xF9, 0xFF, 0x50, 0x28, 0x48, 0x8B, 0x53, 0x08, 0x8B, 0x43, 0x10,
    ],
    copy: (0, 15),
    resume: 15,
};

const READ_OUTER: Site = Site {
    name: "promised units: reader",
    address: 0x1_8034_62a0,
    // push rbp / push r12..r15 / lea rbp,[rsp-37h] (recopiés) / sub rsp,0E0h / mov r8d,[rdx+10h] /
    // mov r14,rdx / mov rdx,[rdx+8] / mov r12,rcx / mov rcx,r14 (puis l'appel qui vide les nœuds).
    expected: &[
        0x40, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0x6C, 0x24, 0xC9, 0x48, 0x81, 0xEC,
        0xE0, 0x00, 0x00, 0x00, 0x44, 0x8B, 0x42, 0x10, 0x4C, 0x8B, 0xF2, 0x48, 0x8B, 0x52, 0x08, 0x4C, 0x8B, 0xE1,
        0x49, 0x8B, 0xCE,
    ],
    copy: (0, 15),
    resume: 15,
};

const ADD_COUNT: Site = Site {
    name: "promised units: table[a][b] += n",
    address: 0x1_8034_cb20,
    // mov [rsp+20h],rbx / mov [rsp+10h],edx / push rdi / sub rsp,40h (recopiés) / add rcx,2C8h
    // (position de l'AutoVariable) / mov edi,r9d / mov ebx,r8d (puis call 18072a920).
    expected: &[
        0x48, 0x89, 0x5C, 0x24, 0x20, 0x89, 0x54, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x48, 0x81, 0xC1, 0xC8,
        0x02, 0x00, 0x00, 0x41, 0x8B, 0xF9, 0x41, 0x8B, 0xD8,
    ],
    copy: (0, 14),
    resume: 14,
};

/// Octets, ailleurs, dont dépendent les remplacements : positions des champs des nœuds et de la
/// valeur de l'AutoVariable.
const LAYOUT: &[(u64, &[u8], &str)] = &[
    (0x1_8034_5a10, &[0x8B, 0x07], "inner key at +0"),
    (0x1_8034_5a21, &[0x8B, 0x47, 0x04], "inner value at +4"),
    (0x1_8034_5a45, &[0x48, 0x8B, 0x7F, 0x08], "inner next at +8"),
    (0x1_8034_5b40, &[0x8B, 0x06, 0x48, 0x8D, 0x56, 0x08], "outer key at +0, inner table at +8"),
    (0x1_8034_5bde, &[0x48, 0x8B, 0x76, 0x30], "outer next at +0x30"),
    (0x1_8034_5b70, &[0xE8, 0xFB, 0xFD, 0xFF, 0xFF], "outer writer calls the inner writer"),
    (0x1_8034_cb3b, &[0xE8, 0xE0, 0xDD, 0x3D, 0x00], "+= n gets the table from 18072a920"),
    (0x1_8072_a9b3, &[0x48, 0x8D, 0x43, 0x10], "18072a920 returns var + 0x10"),
    (0x1_8072_aa25, &[0x48, 0x8D, 0x41, 0x10], "18072a920 returns var + 0x10 (no archive)"),
];

pub const SITES: [Site; 4] = [WRITE_INNER, WRITE_OUTER, READ_OUTER, ADD_COUNT];

unsafe fn check_layout(base: usize) -> Result<(), String> {
    for (addr, bytes, what) in LAYOUT {
        let have = patch::as_known(*addr, std::slice::from_raw_parts((base + patch::rva(*addr)) as *const u8, bytes.len()));
        if have != *bytes {
            return Err(format!("{what}: unexpected bytes at {addr:#x} ({have:02x?})"));
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ mode et détours

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off = 0,
    Count = 1,
    On = 2,
}
static MODE: AtomicU8 = AtomicU8::new(Mode::On as u8);

pub fn set_mode(m: Mode) {
    MODE.store(m as u8, Ordering::Relaxed);
}
fn mode() -> Mode {
    match MODE.load(Ordering::Relaxed) {
        0 => Mode::Off,
        1 => Mode::Count,
        _ => Mode::On,
    }
}

static TRAMPS: [AtomicUsize; 4] = [AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0)];
static HOOKED_BASE: AtomicUsize = AtomicUsize::new(0);
static LOGS: AtomicU32 = AtomicU32::new(0);
static FALLBACKS: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 30;

fn note(msg: &str) {
    if LOGS.fetch_add(1, Ordering::Relaxed) < MAX_LOGS {
        log(&format!("promised units: {msg}"));
    }
}

fn fallback(what: &str) {
    if FALLBACKS.fetch_add(1, Ordering::Relaxed) < 5 {
        log(&format!("promised units: {what}: table layout not as expected, original used"));
    }
}

type Writer = unsafe extern "system" fn(*mut Stream, *const Table) -> *mut Stream;
type Reader = unsafe extern "system" fn(usize, *mut Table);
type AddCount = unsafe extern "system" fn(usize, u32, u32, i32) -> u64;

fn tramp(i: usize) -> usize {
    TRAMPS[i].load(Ordering::Relaxed)
}

/// Vrai quand l'écriture d'origine produirait les octets dans un autre ordre que l'écriture triée.
unsafe fn stock_order_differs(t: &Table) -> bool {
    let (Some(a), Some(b)) = (nodes::<Outer>(t), sorted::<Outer>(t)) else { return false };
    a != b || a.iter().any(|&o| matches!((nodes::<Inner>(&(*o).table), sorted::<Inner>(&(*o).table)), (Some(x), Some(y)) if x != y))
}

unsafe extern "system" fn hook_write_inner(s: *mut Stream, t: *const Table) -> *mut Stream {
    let original: Writer = std::mem::transmute(tramp(0));
    if mode() == Mode::On && pol::actifs() && !t.is_null() {
        if write_inner_sorted(s, &*t).is_some() {
            return s;
        }
        fallback("inner writer");
    }
    original(s, t)
}

unsafe extern "system" fn hook_write_outer(s: *mut Stream, t: *const Table) -> *mut Stream {
    let original: Writer = std::mem::transmute(tramp(1));
    let m = mode();
    if m != Mode::Off && pol::actifs() && !t.is_null() {
        if m == Mode::On {
            if write_outer_sorted(s, &*t).is_some() {
                return s;
            }
            fallback("outer writer");
        } else if stock_order_differs(&*t) {
            note("count: stock serialization order differs from key order (sync hash depends on history)");
        }
    }
    original(s, t)
}

unsafe extern "system" fn hook_read_outer(s: usize, t: *mut Table) {
    let original: Reader = std::mem::transmute(tramp(2));
    original(s, t);
    let m = mode();
    if m != Mode::Off && !t.is_null() && pol::actifs() {
        match canonicalize(&mut *t, m == Mode::On) {
            Some(0) => {}
            Some(n) => note(&format!("{n} chain(s) {} after a load or delta", if m == Mode::On { "sorted" } else { "out of order (count)" })),
            None => fallback("reader"),
        }
    }
}

/// (nombre d'éléments de la table externe, nombre d'éléments de la table interne de `district`
/// ou `u32::MAX`).
unsafe fn shape(t: &Table, district: u32) -> Option<(u32, u32)> {
    let inner = nodes::<Outer>(t)?.into_iter().find(|&o| (*o).key == district).map_or(u32::MAX, |o| (*o).table.count);
    Some((t.count, inner))
}

unsafe extern "system" fn hook_add_count(units: usize, district: u32, unit: u32, n: i32) -> u64 {
    let original: AddCount = std::mem::transmute(tramp(3));
    let m = mode();
    if m == Mode::Off || units == 0 || !pol::actifs() {
        return original(units, district, unit, n);
    }
    let t = (units + TABLE_OFFSET) as *mut Table;
    let before = shape(&*t, district);
    let r = original(units, district, unit, n);
    // Seulement après une insertion : les décréments de clés existantes ont lieu pendant que
    // 180192c20 parcourt cette même chaîne, qu'il ne faut pas relier autrement sous lui.
    if before.is_some() && shape(&*t, district) != before {
        match canonicalize(&mut *t, m == Mode::On) {
            Some(0) => {}
            Some(c) => note(&format!("{c} chain(s) {} after an insertion", if m == Mode::On { "sorted" } else { "out of order (count)" })),
            None => fallback("+= n"),
        }
    }
    r
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    if mode() == Mode::Off {
        log("promised units: off (unites_promises=off), no hooks");
        return;
    }
    if let Err(e) = patch::check(base, &SITES).and_then(|_| check_layout(base)) {
        log(&format!("promised units: unexpected game version, no hooks (POL fix unavailable): {e}"));
        return;
    }
    let hooks: [usize; 4] = [
        hook_write_inner as *const () as usize,
        hook_write_outer as *const () as usize,
        hook_read_outer as *const () as usize,
        hook_add_count as *const () as usize,
    ];
    for (i, s) in SITES.iter().enumerate() {
        match patch::install(base, s, hooks[i]) {
            Ok(t) => TRAMPS[i].store(t, Ordering::Relaxed),
            Err(e) => {
                log(&format!("promised units: failed to hook {}: {e}; hooks removed", s.name));
                for s in &SITES[..i] {
                    patch::restore(base, s);
                }
                return;
            }
        }
    }
    HOOKED_BASE.store(base, Ordering::Relaxed);
    crate::STATUS.fetch_or(crate::ST_POL, Ordering::Relaxed);
    log(&format!("promised units: 4 hooks installed ({:?}), BetterGameExperience {}", mode(), crate::version::VERSION));
}

pub unsafe fn on_proxy_unload() {
    let base = HOOKED_BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        for s in &SITES {
            patch::restore(base, s);
        }
    }
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    const SETTLER: u32 = 0xd064_9d28;
    const BUILDER: u32 = 0x11d5_7532;
    const TRADER_LIKE: u32 = 0x5a5a_0003;
    const SPEARMAN_LIKE: u32 = 0x7b7b_0004;
    const CITY_CENTER: u32 = 0x8a32_007d;

    /// Une table à la forme de GameCore, construite en mémoire Rust. Les chaînes sont données
    /// explicitement : les tests construisent ainsi toute forme que deux machines pourraient avoir.
    struct Owned {
        table: Box<Table>,
        _buckets: Vec<usize>,
    }

    fn table(buckets: Vec<usize>, count: u32) -> Owned {
        let mut b = buckets;
        let n = b.len() as u32;
        b.push(END_SENTINEL);
        let t = Box::new(Table { bases: 0, buckets: b.as_mut_ptr(), bucket_count: n, count, max_load: 1.0, growth: 2.0, next_resize: n, alloc: 0 });
        Owned { table: t, _buckets: b }
    }

    /// Table interne à `bucket_count` seaux, `entries` insérées dans cet ordre en tête de chaîne
    /// (comme l'insertion du jeu). Les nœuds ne sont jamais libérés : tests seulement.
    fn inner(bucket_count: u32, entries: &[(u32, i32)]) -> Owned {
        let mut b = vec![0usize; bucket_count as usize];
        for &(k, v) in entries {
            let i = (k % bucket_count) as usize;
            let node = Box::into_raw(Box::new(Inner { key: k, value: v, next: b[i] as *mut Inner }));
            b[i] = node as usize;
        }
        table(b, entries.len() as u32)
    }

    fn outer(bucket_count: u32, entries: Vec<(u32, Owned)>) -> (Owned, Vec<Owned>) {
        let mut b = vec![0usize; bucket_count as usize];
        let n = entries.len() as u32;
        let mut keep = Vec::new();
        for (k, mut o) in entries {
            let i = (k % bucket_count) as usize;
            let t = std::mem::replace(&mut *o.table, Table { bases: 0, buckets: std::ptr::null_mut(), bucket_count: 0, count: 0, max_load: 0.0, growth: 0.0, next_resize: 0, alloc: 0 });
            let node = Box::into_raw(Box::new(Outer { key: k, pad: 0, table: t, next: b[i] as *mut Outer }));
            b[i] = node as usize;
            keep.push(o); // garde en vie le tableau de seaux de la table déplacée
        }
        (table(b, n), keep)
    }

    /// Doublure du flux : l'entrée +0x28 ajoute à `buf`.
    #[repr(C)]
    struct Sink {
        vtable: *const usize,
        buf: Vec<u8>,
    }
    unsafe extern "system" fn sink_write(s: *mut Stream, p: *const u8, n: u64) -> usize {
        (*(s as *mut Sink)).buf.extend_from_slice(std::slice::from_raw_parts(p, n as usize));
        0
    }
    static VTABLE: [Option<Write>; 8] = [None, None, None, None, None, Some(sink_write), None, None];
    fn sink() -> Sink {
        Sink { vtable: VTABLE.as_ptr() as *const usize, buf: Vec::new() }
    }

    /// Les écritures d'origine, d'après 180345a80 / 180345970 : ordre des seaux.
    unsafe fn stock_inner(out: &mut Vec<u8>, t: &Table) {
        out.extend(t.count.to_le_bytes());
        for n in nodes::<Inner>(t).unwrap() {
            out.extend((*n).key.to_le_bytes());
            out.extend((*n).value.to_le_bytes());
        }
    }
    unsafe fn stock_outer(t: &Table) -> Vec<u8> {
        let mut out = t.count.to_le_bytes().to_vec();
        for o in nodes::<Outer>(t).unwrap() {
            out.extend((*o).key.to_le_bytes());
            stock_inner(&mut out, &(*o).table);
        }
        out
    }
    unsafe fn sorted_outer(t: &Table) -> Vec<u8> {
        let mut s = sink();
        write_outer_sorted(&mut s as *mut Sink as *mut Stream, t).unwrap();
        s.buf
    }
    fn fnv1a(b: &[u8]) -> u32 {
        b.iter().fold(0x811c_9dc5u32, |h, &x| (h ^ x as u32).wrapping_mul(0x0100_0193))
    }
    unsafe fn iteration(t: &Table) -> Vec<(u32, Vec<(u32, i32)>)> {
        nodes::<Outer>(t)
            .unwrap()
            .into_iter()
            .map(|o| ((*o).key, nodes::<Inner>(&(*o).table).unwrap().into_iter().map(|n| ((*n).key, (*n).value)).collect()))
            .collect()
    }

    /// Les deux valeurs du hash relevées par BGE pour un même contenu, après deux chargements d'une
    /// même sauvegarde.
    #[test]
    fn stock_model_reproduces_observed_hashes() {
        unsafe {
            let a = outer(2, vec![(CITY_CENTER, inner(2, &[(BUILDER, 0), (SETTLER, 0)]))]);
            let b = outer(2, vec![(CITY_CENTER, inner(2, &[(SETTLER, 0), (BUILDER, 0)]))]);
            assert_eq!(fnv1a(&stock_outer(&a.0.table)), 0x914d_5dd7, "chain SETTLER, BUILDER");
            assert_eq!(fnv1a(&stock_outer(&b.0.table)), 0x51ca_acc7, "chain BUILDER, SETTLER");
            // Écriture triée : un seul hash pour les deux (BUILDER < SETTLER), celui des octets
            // d'origine de la forme triée.
            assert_eq!(sorted_outer(&a.0.table), sorted_outer(&b.0.table));
            assert_eq!(fnv1a(&sorted_outer(&a.0.table)), 0x51ca_acc7);
        }
    }

    #[test]
    fn empty_table_hash() {
        unsafe {
            let e = table(vec![0], 0);
            assert_eq!(fnv1a(&sorted_outer(&e.table)), 0x4b95_f515);
            assert_eq!(sorted_outer(&e.table), stock_outer(&e.table));
        }
    }

    /// Même format : quand l'ordre de parcours d'origine est déjà celui des clés, mêmes octets.
    #[test]
    fn same_bytes_as_stock_when_already_sorted() {
        unsafe {
            // 5 seaux : les clés tombent dans des seaux d'indice croissant et aucune chaîne
            // n'a deux clés.
            let keys = [5u32, 11, 17, 23];
            let it = inner(5, &keys.iter().map(|&k| (k, k as i32 - 3)).collect::<Vec<_>>());
            let o = outer(3, vec![(9, it), (13, inner(2, &[(4, 1)]))]);
            // Ici, l'ordre des seaux est celui des clés (internes 5, 11, 17, 23 dans les
            // seaux 0..3 ; externes 9, 13 dans les seaux 0, 1) : l'écriture d'origine suit
            // déjà l'ordre des clés.
            assert_eq!(stock_outer(&o.0.table), sorted_outer(&o.0.table));
        }
    }

    /// Toute forme d'un même contenu : une seule suite d'octets et, après `canonicalize`, un seul
    /// ordre de parcours.
    #[test]
    fn layouts_converge() {
        unsafe {
            let content = [(BUILDER, 2), (TRADER_LIKE, 1), (SPEARMAN_LIKE, 1), (SETTLER, 1)];
            let mut perms = vec![content.to_vec()];
            perms.push(content.iter().rev().copied().collect());
            perms.push(vec![content[2], content[0], content[3], content[1]]);
            let mut bytes = Vec::new();
            let mut orders = Vec::new();
            for (i, p) in perms.iter().enumerate() {
                // Le nombre de seaux de la table externe dépend de l'histoire de la machine
                // (gardé d'un chargement à l'autre), pas celui des tables internes.
                let mut o = outer([2, 3, 7][i], vec![(CITY_CENTER, inner(5, p)), (0x1234_5678, inner(2, &[(1, 1), (3, 0)]))]);
                bytes.push(sorted_outer(&o.0.table));
                canonicalize(&mut o.0.table, true).unwrap();
                let it = iteration(&o.0.table);
                // Le jeu parcourt une table interne (trouvée par sa clé) : on compare l'ordre
                // interne quartier par quartier.
                let mut per: Vec<_> = it.clone();
                per.sort_by_key(|e| e.0);
                orders.push(per);
                // Chaînes canoniques : l'écriture d'origine produit elle aussi le contenu de chaque
                // table interne dans un seul ordre.
                assert_eq!(canonicalize(&mut o.0.table, true), Some(0), "idempotent");
            }
            assert!(bytes.windows(2).all(|w| w[0] == w[1]), "sorted bytes independent of layout");
            assert!(orders.windows(2).all(|w| w[0] == w[1]), "inner iteration order independent of layout");
        }
    }

    #[test]
    fn count_mode_writes_nothing() {
        unsafe {
            let mut o = outer(2, vec![(CITY_CENTER, inner(2, &[(BUILDER, 0), (SETTLER, 0)]))]);
            let before = iteration(&o.0.table);
            assert_eq!(canonicalize(&mut o.0.table, false), Some(1));
            assert_eq!(iteration(&o.0.table), before);
            assert_eq!(canonicalize(&mut o.0.table, true), Some(1));
            assert_ne!(iteration(&o.0.table), before);
        }
    }

    #[test]
    fn bad_layout_falls_back() {
        unsafe {
            let t = table(vec![0, 0], 1); // le nombre dit 1, aucun nœud
            assert!(nodes::<Inner>(&t.table).is_none());
            *t.table.buckets.add(2) = 0; // sentinelle absente
            assert!(nodes::<Inner>(&t.table).is_none());
            let mut s = sink();
            assert!(write_inner_sorted(&mut s as *mut Sink as *mut Stream, &t.table).is_none());
            assert!(s.buf.is_empty(), "nothing written before the fallback");
        }
    }

    #[test]
    fn sites_are_consistent() {
        for s in &SITES {
            assert!(s.expected.len() >= 14 && s.copy.1 <= s.expected.len() && s.resume >= s.copy.1, "{}", s.name);
        }
        assert_eq!(TABLE_OFFSET, 0x2d8);
    }

    /// Contre la DLL installée, si elle est là, quel que soit son build : chaque site et chaque
    /// fenêtre de `LAYOUT` concordent.
    #[test]
    fn matches_installed_gamecore() {
        let path = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\DLC\Expansion2\Binaries\Win64\GameCore_XP2_FinalRelease.dll";
        let Ok(data) = std::fs::read(path) else {
            eprintln!("GameCore missing, test skipped");
            return;
        };
        let file = |rva: u64| -> usize {
            // .text : position dans le fichier = rva - `VirtualAddress` + `PointerToRawData` (lus
            // dans la table des sections).
            let pe = u32::from_le_bytes(data[0x3c..0x40].try_into().unwrap()) as usize;
            let nsec = u16::from_le_bytes(data[pe + 6..pe + 8].try_into().unwrap()) as usize;
            let opt = u16::from_le_bytes(data[pe + 20..pe + 22].try_into().unwrap()) as usize;
            let sec = pe + 24 + opt;
            for i in 0..nsec {
                let h = sec + i * 40;
                let va = u32::from_le_bytes(data[h + 12..h + 16].try_into().unwrap()) as u64;
                let vs = u32::from_le_bytes(data[h + 8..h + 12].try_into().unwrap()) as u64;
                let raw = u32::from_le_bytes(data[h + 20..h + 24].try_into().unwrap()) as u64;
                if rva >= va && rva < va + vs {
                    return (rva - va + raw) as usize;
                }
            }
            panic!("rva {rva:#x} not in a section");
        };
        for s in &SITES {
            let at = file(patch::rva(s.address) as u64);
            assert_eq!(patch::as_known(s.address, &data[at..at + s.expected.len()]), s.expected, "{}", s.name);
        }
        for (addr, bytes, what) in LAYOUT {
            let at = file(patch::rva(*addr) as u64);
            assert_eq!(patch::as_known(*addr, &data[at..at + bytes.len()]), *bytes, "{what}");
        }
    }
}
