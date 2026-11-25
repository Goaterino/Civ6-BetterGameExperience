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

// Table, nœuds, ordre canonique et écriture triée : communs avec macOS (unites_promises_regle.rs).
#[path = "unites_promises_regle.rs"]
mod regle;
pub use regle::*;

/// `Player::Units + 0x2c8` (AutoVariable, vérifié dans `18034cb20`) + `0x10` (sa valeur, vérifié
/// dans `18072a920`).
pub const TABLE_OFFSET: usize = 0x2c8 + 0x10;

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

/// Mêmes octets que `180345970`, entrées triées par clé.
pub unsafe fn write_inner_sorted(s: *mut Stream, t: &Table) -> Option<()> {
    write_inner_with(&mut |v| put(s, v), t)
}

/// Mêmes octets que `180345a80`, entrées triées par clé aux deux niveaux. Les tables internes sont
/// validées avant le premier octet écrit : un retour à l'originale ne laisse jamais un flux à
/// moitié écrit.
pub unsafe fn write_outer_sorted(s: *mut Stream, t: &Table) -> Option<()> {
    write_outer_with(&mut |v| put(s, v), t)
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
    use super::regle::tests::{fnv1a, inner, outer, sorted_outer, table, BUILDER, CITY_CENTER, SETTLER};
    use super::*;

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

    /// Les écritures par le flux du jeu donnent les octets de la règle commune (hash relevé).
    #[test]
    fn stream_writers_match_shared_rule() {
        unsafe {
            let a = outer(2, vec![(CITY_CENTER, inner(2, &[(BUILDER, 0), (SETTLER, 0)]))]);
            let mut s = sink();
            write_outer_sorted(&mut s as *mut Sink as *mut Stream, &a.0.table).unwrap();
            assert_eq!(s.buf, sorted_outer(&a.0.table));
            assert_eq!(fnv1a(&s.buf), 0x51ca_acc7);
            let bad = table(vec![0, 0], 1);
            let mut s = sink();
            assert!(write_inner_sorted(&mut s as *mut Sink as *mut Stream, &bad.table).is_none());
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
