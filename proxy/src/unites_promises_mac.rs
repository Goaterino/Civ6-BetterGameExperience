//! Unités promises à l'IA pour ses prochains quartiers, sous macOS : le pendant de
//! unites_promises.rs (voir son en-tête). Correctif POL. Table, ordre canonique et écriture triée
//! communs avec Windows (unites_promises_regle.rs).
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image, depuis la
//! chaîne `m_aiAddOnDistrictFounding` (constructeur de `Player::Units` `0xa5ec98`) :
//! - l'AutoVariable est en `Player::Units + 0x2d0` (Windows `+0x2c8`), vtable `0x12f44d8`, table en
//!   `+0x2e0` (`+0x10`). Même disposition EASTL que Windows : seaux `+0x8`, nombre de seaux `+0x10`
//!   et d'éléments `+0x14` (`u32`), sentinelle `!0` après le dernier seau (relue par toutes les
//!   boucles, posée par la lecture `0xa72628`) ; nœud externe : clé `+0`, table interne `+8`,
//!   suivant `+0x30` ; nœud interne : clé `+0`, valeur `+4`, suivant `+8`. `Player::Units` est en
//!   `Player + 0x6d0`.
//! - Écriture. clang a mis en ligne l'écriture du nombre dans chaque appelant ; les éléments sont
//!   écrits par `0xa6b300(nœud, seau, nœud de fin, seau de fin, flux, table)` (clé, nombre, puis
//!   clé et valeur de chaque entrée interne, recopiée avant d'être écrite), appelée par la
//!   sauvegarde de `Player::Units` (`0xa5b134`), l'écriture de la variable (vtable +0x28,
//!   `0xa715f8`), son hash de synchro (+0x40, `0xa716d8`) et sa comparaison de diagnostic (+0x48,
//!   `0xa71838` et `0xa718b0`, la seconde sur une copie lue). Elle est remplacée par l'écriture
//!   triée des éléments quand la plage est la table entière (le cas de ses cinq appelants) ;
//!   sinon, l'originale.
//! - Lecture `0xa72480(flux, table)` (vtable +0x10 via `0xa714c0`, chargements et deltas ; +0x20
//!   pour une lecture jetée) : après l'originale, chaque chaîne est triée par clé, comme sous
//!   Windows (`1803462a0`).
//! - `table[a][b] += n` : `0xa68684(unités, quartier, unité, n)` (Windows `18034cb20`), la seule
//!   mutation : appelée par la mise en place des unités de départ (`0x9b3758`, deux appels) et par
//!   la création des unités promises à l'achèvement d'un quartier (`0x4e371c`, qui parcourt la
//!   table interne rendue par `0xa684d4` dans l'ordre des seaux et décrémente des clés existantes).
//!   Le suivi de FAutoArchive y est en ligne. Après l'originale, retri seulement si un nombre
//!   d'éléments a changé, comme sous Windows.
//!
//! Le flux du jeu a la même vtable que sous Windows : `+0x28` est `write(flux, tampon, longueur)`.
//!
//! x86_64 : pas encore relevé (`installer` le dit) ; le bit `ST_POL` reste donc baissé en x86_64.

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]

use super::journal::log;
use super::unites_promises_regle::{canonicalize, nodes, write_elements_with, Outer, Table};
use super::{patch, pol};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    /// FNV-1a 64 du corps entier : offsets et appels de cette version du jeu.
    empreinte: u64,
}

const ELEMENTS: Fonction = Fonction { nom: "écriture des éléments", adresse: 0xa6b300, taille: 0x1ec, empreinte: 0xc6f6_5e1b_1b0f_a8eb };
const LECTURE: Fonction = Fonction { nom: "lecture", adresse: 0xa72480, taille: 0x388, empreinte: 0x2d51_3c0f_920b_ffb1 };
const AJOUT: Fonction = Fonction { nom: "table[a][b] += n", adresse: 0xa68684, taille: 0x148, empreinte: 0xe9c3_21f6_f622_7fde };
/// Prologue recopié dans les trampolines : quatre instructions de pile, sans adresse relative.
const PROLOGUE: usize = 16;

/// `Player::Units + 0x2d0` (AutoVariable, `0xa686b8` et `0xa5f630`) + `0x10` (sa valeur,
/// `0xa68744` et `0xa5b0cc`).
pub const TABLE_OFFSET: usize = 0x2d0 + 0x10;

/// Mots relus ailleurs, dont dépendent les détours : (adresse, mot arm64, ce qu'il montre).
const DISPOSITION: &[(usize, u32, &str)] = &[
    (0xa5f630, 0x910b_4274, "add x20, x19, #0x2d0 : AutoVariable en Units+0x2d0"),
    (0xa68744, 0x910b_82a0, "add x0, x21, #0x2e0 : table en Units+0x2e0"),
    (0xa715b4, 0x9100_4016, "add x22, x0, #0x10 : la table est la valeur de la variable (+0x10)"),
    (0xa715b8, 0x2944_2809, "ldp w9, w10, [x0, #0x20] : nombre de seaux, nombre d'éléments"),
    (0xa6b358, 0xf940_1af7, "ldr x23, [x23, #0x30] : suivant du nœud externe en +0x30"),
    (0xa6b36c, 0xb840_8428, "ldr w8, [x1], #0x8 : clé en +0, table interne en +8"),
    (0xa6b3dc, 0x2940_2788, "ldp w8, w9, [x28] : clé et valeur interne en +0 et +4"),
    (0xa6b414, 0xf940_079c, "ldr x28, [x28, #0x8] : suivant du nœud interne en +8"),
    (0xa715f8, 0x97ff_e742, "bl 0xa6b300 : l'écriture de la variable passe par les éléments"),
    (0xa716d8, 0x97ff_e70a, "bl 0xa6b300 : le hash de synchro aussi"),
    (0xa5b134, 0x9400_4073, "bl 0xa6b300 : la sauvegarde de Player::Units aussi"),
    (0xa714cc, 0x1400_03ed, "b 0xa72480 : la lecture de la variable"),
];

static BASE: AtomicUsize = AtomicUsize::new(0);
static T_ELEMENTS: AtomicUsize = AtomicUsize::new(0);
static T_LECTURE: AtomicUsize = AtomicUsize::new(0);
static T_AJOUT: AtomicUsize = AtomicUsize::new(0);
static LOGS: AtomicU32 = AtomicU32::new(0);
static REPLIS: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 30;

fn note(msg: &str) {
    if LOGS.fetch_add(1, Ordering::Relaxed) < MAX_LOGS {
        log(&format!("unités promises : {msg}"));
    }
}

fn repli(quoi: &str) {
    if REPLIS.fetch_add(1, Ordering::Relaxed) < 5 {
        log(&format!("unités promises : {quoi} : table inattendue, originale employée"));
    }
}

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

/// Le flux du jeu : l'entrée `+0x28` de sa vtable est `write(flux, tampon, longueur)`.
#[repr(C)]
struct Flux {
    vtable: *const usize,
}
type Ecrire = unsafe extern "C" fn(*mut Flux, *const u8, u64) -> usize;

unsafe fn put(s: *mut Flux, v: &[u8; 4]) {
    let w: Ecrire = std::mem::transmute(*(*s).vtable.add(0x28 / 8));
    w(s, v.as_ptr(), 4);
}

/// Ce que rend `0xa6b300` (dans x0 et x1) : le flux et la table.
#[repr(C)]
pub struct Paire {
    a: usize,
    b: usize,
}

/// Vrai si `[noeud, fin)` (itérateurs EASTL : nœud et case de seau) couvre toute la table `t`.
unsafe fn plage_entiere(t: &Table, noeud: usize, seau: *const usize, fin: usize, seau_fin: *const usize) -> bool {
    let Some(v) = nodes::<Outer>(t) else { return false };
    let dernier = t.buckets.add(t.bucket_count as usize) as *const usize;
    if seau_fin != dernier || fin != *dernier {
        return false;
    }
    match v.first() {
        None => noeud == fin,
        Some(&premier) => noeud == premier as usize && *seau == noeud,
    }
}

unsafe extern "C" fn hook_elements(noeud: usize, seau: *const usize, fin: usize, seau_fin: *const usize, flux: *mut Flux, table: *const Table) -> Paire {
    let original: unsafe extern "C" fn(usize, *const usize, usize, *const usize, *mut Flux, *const Table) -> Paire =
        std::mem::transmute(T_ELEMENTS.load(Ordering::Relaxed));
    if !table.is_null() && !flux.is_null() && pol::actifs() {
        if plage_entiere(&*table, noeud, seau, fin, seau_fin) {
            if write_elements_with(&mut |v| put(flux, v), &*table).is_some() {
                return Paire { a: flux as usize, b: table as usize };
            }
        }
        repli("écriture");
    }
    original(noeud, seau, fin, seau_fin, flux, table)
}

unsafe extern "C" fn hook_lecture(flux: usize, table: *mut Table) {
    let original: unsafe extern "C" fn(usize, *mut Table) = std::mem::transmute(T_LECTURE.load(Ordering::Relaxed));
    original(flux, table);
    if !table.is_null() && pol::actifs() {
        match canonicalize(&mut *table, true) {
            Some(0) => {}
            Some(n) => note(&format!("{n} chaîne(s) triée(s) après un chargement ou un delta")),
            None => repli("lecture"),
        }
    }
}

/// (nombre d'éléments de la table externe, nombre d'éléments de la table interne de `quartier`
/// ou `u32::MAX`).
unsafe fn forme(t: &Table, quartier: u32) -> Option<(u32, u32)> {
    let interne = nodes::<Outer>(t)?.into_iter().find(|&o| (*o).key == quartier).map_or(u32::MAX, |o| (*o).table.count);
    Some((t.count, interne))
}

unsafe extern "C" fn hook_ajout(unites: usize, quartier: u32, unite: u32, n: i32) {
    let original: unsafe extern "C" fn(usize, u32, u32, i32) = std::mem::transmute(T_AJOUT.load(Ordering::Relaxed));
    if unites == 0 || !pol::actifs() {
        return original(unites, quartier, unite, n);
    }
    let t = (unites + TABLE_OFFSET) as *mut Table;
    let avant = forme(&*t, quartier);
    original(unites, quartier, unite, n);
    // Seulement après une insertion : les décréments de clés existantes ont lieu pendant que
    // 0x4e371c parcourt cette même chaîne, qu'il ne faut pas relier autrement sous lui.
    if avant.is_some() && forme(&*t, quartier) != avant {
        match canonicalize(&mut *t, true) {
            Some(0) => {}
            Some(c) => note(&format!("{c} chaîne(s) triée(s) après une insertion")),
            None => repli("+= n"),
        }
    }
}

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    let fonctions = [
        (ELEMENTS, &T_ELEMENTS, hook_elements as *const () as usize),
        (LECTURE, &T_LECTURE, hook_lecture as *const () as usize),
        (AJOUT, &T_AJOUT, hook_ajout as *const () as usize),
    ];
    for (f, _, _) in &fonctions {
        if fnv(std::slice::from_raw_parts((base + f.adresse) as *const u8, f.taille)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    for &(a, mot, quoi) in DISPOSITION {
        if std::ptr::read_unaligned((base + a) as *const u32) != mot {
            return Err(format!("version inattendue ({quoi}, en {a:#x}), aucun détour"));
        }
    }
    BASE.store(base, Ordering::Relaxed);
    let mut methode = "";
    for (f, tramp, accroche) in fonctions {
        // Le trampoline est publié avant que le saut ne soit écrit.
        methode = patch::hook_with_trampoline_publie(base + f.adresse, PROLOGUE, accroche, tramp)?;
    }
    Ok(format!("table des unités promises tenue triée (écriture, lecture, insertion ; {methode})"))
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(_base: usize) -> Result<String, String> {
    Err("pas encore relevé en x86_64".into())
}

#[cfg(test)]
mod tests {
    use super::super::unites_promises_regle::tests::{fnv1a, inner, outer, sorted_outer, table, BUILDER, CITY_CENTER, SETTLER};
    use super::*;

    /// Doublure du flux : l'entrée +0x28 ajoute à `buf`.
    #[repr(C)]
    struct Puits {
        vtable: *const usize,
        buf: Vec<u8>,
    }
    unsafe extern "C" fn puits_ecrire(s: *mut Flux, p: *const u8, n: u64) -> usize {
        (*(s as *mut Puits)).buf.extend_from_slice(std::slice::from_raw_parts(p, n as usize));
        0
    }
    static VTABLE: [Option<Ecrire>; 8] = [None, None, None, None, None, Some(puits_ecrire), None, None];

    /// Les itérateurs de début et de fin que les appelants de `0xa6b300` calculent.
    unsafe fn bornes(t: &Table) -> (usize, *const usize, usize, *const usize) {
        let mut s = t.buckets as *const usize;
        while *s == 0 {
            s = s.add(1);
        }
        let fin = t.buckets.add(t.bucket_count as usize) as *const usize;
        (*s, s, *fin, fin)
    }

    #[test]
    fn plage_et_octets() {
        unsafe {
            let a = outer(2, vec![(CITY_CENTER, inner(2, &[(BUILDER, 0), (SETTLER, 0)]))]);
            let t = &*a.0.table;
            let (n, s, f, sf) = bornes(t);
            assert!(plage_entiere(t, n, s, f, sf));
            assert!(!plage_entiere(t, f, sf, f, sf), "plage vide d'une table non vide");
            // L'appelant écrit le nombre, les éléments suivent : le hash relevé sous Windows.
            let mut p = Puits { vtable: VTABLE.as_ptr() as *const usize, buf: t.count.to_le_bytes().to_vec() };
            write_elements_with(&mut |v| put(&mut p as *mut Puits as *mut Flux, v), t).unwrap();
            assert_eq!(p.buf, sorted_outer(t));
            assert_eq!(fnv1a(&p.buf), 0x51ca_acc7);
            // Table vide : début = fin = sentinelle.
            let e = table(vec![0], 0);
            let (n, s, f, sf) = bornes(&e.table);
            assert_eq!(n, f);
            assert!(plage_entiere(&e.table, n, s, f, sf));
        }
    }

    /// `__TEXT,__text` de la tranche arm64.
    const TEXTE: (usize, usize) = (0xcb8, 0xfc93a8);

    #[test]
    fn fonctions_du_jeu() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::super::macho::tranche_arm64(&b).expect("tranche arm64");
        let mot = |a: usize| u32::from_le_bytes(t[a..a + 4].try_into().unwrap());
        for f in [ELEMENTS, LECTURE, AJOUT] {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
            // Prologue sans adresse relative : sub sp / stp (pas d'adrp, adr, b, bl, ldr littéral).
            for k in 0..PROLOGUE / 4 {
                let m = mot(f.adresse + 4 * k);
                let relatif = m & 0x9f00_0000 == 0x9000_0000 || m & 0x9f00_0000 == 0x1000_0000 || m & 0x7c00_0000 == 0x1400_0000 || m & 0x3b00_0000 == 0x1800_0000;
                assert!(!relatif, "{} : instruction relative en +{}", f.nom, 4 * k);
            }
        }
        for &(a, m, quoi) in DISPOSITION {
            assert_eq!(mot(a), m, "{quoi}");
        }
        // Les cinq appels de l'écriture des éléments, et seulement eux.
        let code = || (TEXTE.0 / 4..TEXTE.1 / 4).map(|i| 4 * i);
        let appels: Vec<usize> = code()
            .filter(|&a| mot(a) & 0xfc00_0000 == 0x9400_0000 && patch::cible_branche(a, mot(a)) == Some(ELEMENTS.adresse))
            .collect();
        assert_eq!(appels, vec![0xa5b134, 0xa715f8, 0xa716d8, 0xa71838, 0xa718b0]);
        // `+= n` : appelée par la mise en place (deux fois) et la création des unités promises.
        let ajouts: Vec<usize> = code()
            .filter(|&a| mot(a) & 0xfc00_0000 == 0x9400_0000 && patch::cible_branche(a, mot(a)) == Some(AJOUT.adresse))
            .collect();
        assert_eq!(ajouts, vec![0x4e3bd0, 0x9b3a98, 0x9b3f58]);
    }
}
