//! Unités promises à l'IA, règle commune à Windows (unites_promises.rs) et macOS
//! (unites_promises_mac.rs) : la table `hash_map<quartier, hash_map<unité, i32>>` EASTL telle que
//! GameCore la range, son ordre canonique (chaînes de seaux triées par clé) et son écriture triée.
//! Voir l'en-tête de unites_promises.rs.
//!
//! La disposition est la même sous Windows (MSVC) et sous macOS (clang, relevée en arm64) : relevée
//! dans le code qui la lit (unites_promises.rs, `LAYOUT` ; unites_promises_mac.rs, `DISPOSITION`).

// Sous macOS, l'écriture complète et le mode de comptage (Windows seulement) ne servent qu'aux tests.
#![cfg_attr(not(windows), allow(dead_code))]

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

/// Au-delà, ce n'est pas cette table (ou la mémoire n'est pas ce qu'on croit) : l'originale.
const MAX_ELEMENTS: u32 = 1 << 16;
pub const END_SENTINEL: usize = usize::MAX;

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

/// Les nœuds de `t` triés par clé.
pub unsafe fn sorted<N: Node>(t: &Table) -> Option<Vec<*mut N>> {
    let mut v = nodes::<N>(t)?;
    v.sort_by_key(|&n| (*n).key());
    Some(v)
}

/// Vrai quand l'écriture d'origine produirait les octets dans un autre ordre que l'écriture triée.
pub unsafe fn stock_order_differs(t: &Table) -> bool {
    let (Some(a), Some(b)) = (nodes::<Outer>(t), sorted::<Outer>(t)) else { return false };
    a != b || a.iter().any(|&o| matches!((nodes::<Inner>(&(*o).table), sorted::<Inner>(&(*o).table)), (Some(x), Some(y)) if x != y))
}

// ------------------------------------------------------------------ écriture

/// Table interne : `u32` nombre, puis `u32` clé et `i32` valeur, triées par clé (les octets de
/// l'écriture du jeu quand elle suit déjà l'ordre des clés).
pub unsafe fn write_inner_with(put: &mut dyn FnMut(&[u8; 4]), t: &Table) -> Option<()> {
    let v = sorted::<Inner>(t)?;
    put(&t.count.to_le_bytes());
    for n in v {
        put(&(*n).key.to_le_bytes());
        put(&(*n).value.to_le_bytes());
    }
    Some(())
}

/// Les éléments de la table externe, sans le nombre : `u32` clé puis table interne, triés par clé
/// aux deux niveaux. Les tables internes sont validées avant le premier octet écrit : un retour à
/// l'originale ne laisse jamais un flux à moitié écrit.
pub unsafe fn write_elements_with(put: &mut dyn FnMut(&[u8; 4]), t: &Table) -> Option<()> {
    let v = sorted::<Outer>(t)?;
    for &o in &v {
        nodes::<Inner>(&(*o).table)?;
    }
    for o in v {
        put(&(*o).key.to_le_bytes());
        write_inner_with(put, &(*o).table)?;
    }
    Some(())
}

/// Toute la table externe : `u32` nombre puis les éléments triés (validés avant le nombre).
pub unsafe fn write_outer_with(put: &mut dyn FnMut(&[u8; 4]), t: &Table) -> Option<()> {
    let v = sorted::<Outer>(t)?;
    for &o in &v {
        nodes::<Inner>(&(*o).table)?;
    }
    put(&t.count.to_le_bytes());
    write_elements_with(put, t)
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub const SETTLER: u32 = 0xd064_9d28;
    pub const BUILDER: u32 = 0x11d5_7532;
    const TRADER_LIKE: u32 = 0x5a5a_0003;
    const SPEARMAN_LIKE: u32 = 0x7b7b_0004;
    pub const CITY_CENTER: u32 = 0x8a32_007d;

    /// Une table à la forme de GameCore, construite en mémoire Rust. Les chaînes sont données
    /// explicitement : les tests construisent ainsi toute forme que deux machines pourraient avoir.
    pub struct Owned {
        pub table: Box<Table>,
        _buckets: Vec<usize>,
    }

    pub fn table(buckets: Vec<usize>, count: u32) -> Owned {
        let mut b = buckets;
        let n = b.len() as u32;
        b.push(END_SENTINEL);
        let t = Box::new(Table { bases: 0, buckets: b.as_mut_ptr(), bucket_count: n, count, max_load: 1.0, growth: 2.0, next_resize: n, alloc: 0 });
        Owned { table: t, _buckets: b }
    }

    /// Table interne à `bucket_count` seaux, `entries` insérées dans cet ordre en tête de chaîne
    /// (comme l'insertion du jeu). Les nœuds ne sont jamais libérés : tests seulement.
    pub fn inner(bucket_count: u32, entries: &[(u32, i32)]) -> Owned {
        let mut b = vec![0usize; bucket_count as usize];
        for &(k, v) in entries {
            let i = (k % bucket_count) as usize;
            let node = Box::into_raw(Box::new(Inner { key: k, value: v, next: b[i] as *mut Inner }));
            b[i] = node as usize;
        }
        table(b, entries.len() as u32)
    }

    pub fn outer(bucket_count: u32, entries: Vec<(u32, Owned)>) -> (Owned, Vec<Owned>) {
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

    /// Les écritures d'origine (180345a80 / 180345970 sous Windows, `0xa6b300` et ses appelants
    /// sous macOS) : ordre des seaux.
    pub unsafe fn stock_inner(out: &mut Vec<u8>, t: &Table) {
        out.extend(t.count.to_le_bytes());
        for n in nodes::<Inner>(t).unwrap() {
            out.extend((*n).key.to_le_bytes());
            out.extend((*n).value.to_le_bytes());
        }
    }
    pub unsafe fn stock_outer(t: &Table) -> Vec<u8> {
        let mut out = t.count.to_le_bytes().to_vec();
        for o in nodes::<Outer>(t).unwrap() {
            out.extend((*o).key.to_le_bytes());
            stock_inner(&mut out, &(*o).table);
        }
        out
    }
    pub unsafe fn sorted_outer(t: &Table) -> Vec<u8> {
        let mut buf = Vec::new();
        write_outer_with(&mut |b| buf.extend_from_slice(b), t).unwrap();
        buf
    }
    pub fn fnv1a(b: &[u8]) -> u32 {
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

    /// L'écriture des éléments seuls (macOS : le nombre est écrit par l'appelant) suit le nombre
    /// dans l'écriture complète.
    #[test]
    fn elements_follow_the_count() {
        unsafe {
            let o = outer(2, vec![(CITY_CENTER, inner(2, &[(BUILDER, 2), (SETTLER, 1)])), (3, inner(1, &[(7, 1)]))]);
            let mut buf = o.0.table.count.to_le_bytes().to_vec();
            write_elements_with(&mut |b| buf.extend_from_slice(b), &o.0.table).unwrap();
            assert_eq!(buf, sorted_outer(&o.0.table));
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
            assert!(stock_order_differs(&o.0.table));
            assert_eq!(canonicalize(&mut o.0.table, false), Some(1));
            assert_eq!(iteration(&o.0.table), before);
            assert_eq!(canonicalize(&mut o.0.table, true), Some(1));
            assert_ne!(iteration(&o.0.table), before);
            assert!(!stock_order_differs(&o.0.table));
        }
    }

    #[test]
    fn bad_layout_falls_back() {
        unsafe {
            let t = table(vec![0, 0], 1); // le nombre dit 1, aucun nœud
            assert!(nodes::<Inner>(&t.table).is_none());
            *t.table.buckets.add(2) = 0; // sentinelle absente
            assert!(nodes::<Inner>(&t.table).is_none());
            let mut buf = Vec::new();
            assert!(write_inner_with(&mut |b| buf.extend_from_slice(b), &t.table).is_none());
            assert!(write_elements_with(&mut |b| buf.extend_from_slice(b), &t.table).is_none());
            assert!(buf.is_empty(), "nothing written before the fallback");
        }
    }
}
