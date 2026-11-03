//! Choix des cases travaillées par une ville : réécriture de `18010f050` (City_Citizens.cpp).
//! Correctif POL : il change la simulation, il n'est appliqué que si tous les joueurs ont le proxy.
//!
//! `180110ba0` (placement des citoyens, relancé à chaque focus, croissance, annexion de case…) note
//! chaque emplacement libre (case, ou siège de spécialiste d'un district) : nourriture brute,
//! score « favori » (rendements favoris, doublés sauf l'or) et score « autre » (le reste des
//! rendements non défavorisés). Il appelle `18010f050` avec le nombre de citoyens à placer `K` et le
//! déficit de nourriture `T` (consommation moins nourriture de la ville sans ses cases). Celle-ci
//! choisit au plus `K` emplacements par programmation dynamique, en préférant une combinaison qui
//! atteint `T`, puis le score favori, puis l'autre. Mais sa comparaison n'est pas un ordre : une
//! étape peut retomber sous `T` pour un meilleur score, et l'étape suivante repasse `T` avec la
//! combinaison qu'elle a sous la main. D'où, par exemple, une case 2/1 travaillée à côté d'une 2/2
//! libre, avec le focus nourriture et production (investigation : citoyens/README.md).
//!
//! La réécriture fait le même choix, exactement : parmi les combinaisons d'au plus `K` emplacements
//! qui atteignent `T`, s'il y en a, le meilleur score favori puis autre ; sinon le meilleur score
//! tout court ; à égalité, le moins d'emplacements, puis le premier trouvé. Programmation dynamique
//! sur (emplacements pris, nourriture plafonnée à `T`). Elle ne fait que lire les candidats et
//! écrit les emplacements choisis au début du tableau, comme l'originale.
//!
//! Correctif POL : seulement si l'export `BgdPolActifs` du proxy de démarrage rend 1 (salon où tous
//! ont le proxy à la même version, ou partie locale). Sinon, l'originale. Chaque fois que le choix
//! diffère de celui de l'originale, recalculée sur une copie, on le journalise (30 premières fois).

use crate::patch::{self, Site};
use crate::{log, pol};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const SELECT: Site = Site {
    name: "choix des cases travaillées",
    address: 0x1_8010_f050,
    // mov r11,rsp / mov [r11+20h],r9d / mov [r11+10h],rdx / mov [r11+8],rcx : sans adresse relative.
    expected: &[0x4c, 0x8b, 0xdc, 0x45, 0x89, 0x4b, 0x20, 0x49, 0x89, 0x53, 0x10, 0x49, 0x89, 0x4b, 0x08],
    copy: (0, 15),
    resume: 15,
};

static TRAMP: AtomicUsize = AtomicUsize::new(0);
static HOOKED_BASE: AtomicUsize = AtomicUsize::new(0);
static DIFFS: AtomicU32 = AtomicU32::new(0);
const MAX_DIFF_LOGS: u32 = 30;

/// Un emplacement candidat, tel que le range `180110ba0` (0x18 octets).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidat {
    pub plot: usize,
    pub nourriture: i32,
    pub favori: i32,
    pub autre: i32,
    pub reste: i32,
}

type Select = unsafe extern "system" fn(*mut Candidat, *mut Candidat, u32, i32, usize) -> *mut Candidat;

/// Taille au-delà de laquelle on laisse l'originale : (emplacements) × (K + 1) × (T + 1) cases de
/// table. Une ville a au plus quelques dizaines d'emplacements, de citoyens et de déficit.
const MAX_CELLS: usize = 4_000_000;

/// Le choix corrigé : indices des emplacements retenus, croissants. None si l'entrée sort du cadre
/// prévu (nourriture négative, table trop grande) : l'appelant garde alors l'originale.
pub fn choisir(cands: &[(i32, i32, i32)], k_max: usize, seuil: i32) -> Option<Vec<usize>> {
    let n = cands.len();
    let k_max = k_max.min(n);
    let t = seuil.max(0) as usize;
    if cands.iter().any(|c| c.0 < 0) || t >= u16::MAX as usize || n * (k_max + 1) * (t + 1) > MAX_CELLS {
        return None;
    }
    let width = t + 1;
    let cells = (k_max + 1) * width;
    // meilleur[k × (T+1) + c] : meilleur (favori, autre) avec k emplacements et min(nourriture, T) = c.
    let mut prev: Vec<Option<(i64, i64)>> = vec![None; cells];
    prev[0] = Some((0, 0));
    // parent[i × cells + case] : c d'avant si l'emplacement i a été pris pour atteindre cette case.
    let mut parent = vec![u16::MAX; n * cells];
    for (i, &(f, fa, o)) in cands.iter().enumerate() {
        let mut cur = prev.clone();
        for k in 1..=k_max {
            for c0 in 0..width {
                let Some((bf, bo)) = prev[(k - 1) * width + c0] else { continue };
                let c = (c0 + f as usize).min(t);
                let cand = (bf + fa as i64, bo + o as i64);
                let at = k * width + c;
                if cur[at].map_or(true, |v| cand > v) {
                    cur[at] = Some(cand);
                    parent[i * cells + at] = c0 as u16;
                }
            }
        }
        prev = cur;
    }
    let pick = |cs: &mut dyn Iterator<Item = usize>| {
        let cs: Vec<usize> = cs.collect();
        let mut best: Option<((i64, i64), usize, usize)> = None;
        for k in 0..=k_max {
            for &c in &cs {
                if let Some(v) = prev[k * width + c] {
                    if best.map_or(true, |b| v > b.0) {
                        best = Some((v, k, c));
                    }
                }
            }
        }
        best
    };
    let (_, mut k, mut c) = pick(&mut std::iter::once(t)).or_else(|| pick(&mut (0..width)))?;
    let mut sel = Vec::with_capacity(k);
    for i in (0..n).rev() {
        if k == 0 {
            break;
        }
        let p = parent[i * cells + k * width + c];
        if p != u16::MAX {
            sel.push(i);
            c = p as usize;
            k -= 1;
        }
    }
    sel.reverse();
    Some(sel)
}

/// (nourriture plafonnée atteinte, favori, autre) d'un choix, pour le journal.
fn bilan(cands: &[Candidat], sel: impl Iterator<Item = usize>, seuil: i32) -> String {
    let (mut f, mut fa, mut o, mut cases) = (0i64, 0i64, 0i64, Vec::new());
    for i in sel {
        let c = &cands[i];
        f += c.nourriture as i64;
        fa += c.favori as i64;
        o += c.autre as i64;
        // Coordonnées de la case en +0x28 et +0x2a.
        let (x, y) = unsafe { (std::ptr::read((c.plot + 0x28) as *const i16), std::ptr::read((c.plot + 0x2a) as *const i16)) };
        cases.push(format!("({x},{y}) {}/{}/{}", c.nourriture, c.favori, c.autre));
    }
    format!("nourriture {f}/{seuil}, favori {fa}, autre {o} : {}", cases.join(" "))
}

unsafe extern "system" fn hook(begin: *mut Candidat, end: *mut Candidat, k: u32, seuil: i32, e: usize) -> *mut Candidat {
    let original: Select = std::mem::transmute(TRAMP.load(Ordering::Relaxed));
    if k == 0 || begin.is_null() || end <= begin || !pol::actifs() {
        return original(begin, end, k, seuil, e);
    }
    let n = end.offset_from(begin) as usize;
    let cands: Vec<Candidat> = std::slice::from_raw_parts(begin, n).to_vec();
    let triplets: Vec<(i32, i32, i32)> = cands.iter().map(|c| (c.nourriture, c.favori, c.autre)).collect();
    let Some(sel) = choisir(&triplets, k as usize, seuil) else {
        return original(begin, end, k, seuil, e);
    };
    if DIFFS.load(Ordering::Relaxed) < MAX_DIFF_LOGS {
        // L'originale sur une copie : elle ne fait qu'écrire son choix au début du tableau.
        let mut copie = cands.clone();
        let fin = original(copie.as_mut_ptr(), copie.as_mut_ptr().add(n), k, seuil, e);
        let m = fin.offset_from(copie.as_ptr()) as usize;
        let jeu: Vec<usize> = copie[..m].iter().filter_map(|c| cands.iter().position(|d| d == c)).collect();
        let score = |s: &[usize]| {
            let f: i64 = s.iter().map(|&i| cands[i].nourriture as i64).sum();
            (f >= seuil as i64, s.iter().map(|&i| cands[i].favori as i64).sum::<i64>(), s.iter().map(|&i| cands[i].autre as i64).sum::<i64>())
        };
        if score(&jeu) != score(&sel) {
            let d = DIFFS.fetch_add(1, Ordering::Relaxed) + 1;
            log(&format!(
                "citoyens : choix corrigé ({d}), {k} citoyens. Jeu : {}. BGD : {}",
                bilan(&cands, jeu.iter().copied(), seuil),
                bilan(&cands, sel.iter().copied(), seuil)
            ));
        }
    }
    for (j, &i) in sel.iter().enumerate() {
        *begin.add(j) = cands[i];
    }
    begin.add(sel.len())
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    if let Err(e) = patch::check(base, &[SELECT]) {
        log(&format!("citoyens : version inattendue, aucun détour (correctifs POL indisponibles) : {e}"));
        return;
    }
    match patch::install(base, &SELECT, hook as *const () as usize) {
        Ok(t) => {
            TRAMP.store(t, Ordering::Relaxed);
            HOOKED_BASE.store(base, Ordering::Relaxed);
            crate::STATUS.fetch_or(crate::ST_POL, Ordering::Relaxed);
            log(&format!("citoyens : détour posé en {:#x}, BetterGameDLLs {}", SELECT.address, crate::version::VERSION));
        }
        Err(e) => log(&format!("citoyens : échec de la pose du détour : {e}")),
    }
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet ses octets d'origine.
pub unsafe fn on_proxy_unload() {
    let base = HOOKED_BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        patch::restore(base, &SELECT);
    }
}

/// Nouvelle partie : le journal des écarts recommence.
pub fn reset() {
    DIFFS.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Générateur pseudo-aléatoire déterministe (xorshift), pour ne dépendre d'aucune crate.
    struct Alea(u64);
    impl Alea {
        fn next(&mut self, n: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % n
        }
    }

    fn cle(c: &[(i32, i32, i32)], s: &[usize], seuil: i32) -> (bool, i64, i64, i64) {
        let f: i64 = s.iter().map(|&i| c[i].0 as i64).sum();
        let fa: i64 = s.iter().map(|&i| c[i].1 as i64).sum();
        let o: i64 = s.iter().map(|&i| c[i].2 as i64).sum();
        (f >= seuil.max(0) as i64, fa, o, -(s.len() as i64))
    }

    /// Recherche exhaustive : la meilleure clé (seuil atteint, favori, autre, moins d'emplacements).
    fn exhaustif(c: &[(i32, i32, i32)], k_max: usize, seuil: i32) -> (bool, i64, i64, i64) {
        let n = c.len();
        let mut best = None;
        for mask in 0u32..(1 << n) {
            let s: Vec<usize> = (0..n).filter(|i| mask >> i & 1 == 1).collect();
            if s.len() <= k_max {
                let k = cle(c, &s, seuil);
                if best.map_or(true, |b| k > b) {
                    best = Some(k);
                }
            }
        }
        best.unwrap()
    }

    /// Une case (nourriture, production, or) avec le focus nourriture et production : comme
    /// `180110ba0`, rendements doublés sauf l'or, favoris à part.
    fn case(f: i32, p: i32, g: i32, fav_f: bool, fav_p: bool) -> (i32, i32, i32) {
        let (mut fa, mut o) = (0, g);
        if fav_f { fa += 2 * f } else { o += 2 * f }
        if fav_p { fa += 2 * p } else { o += 2 * p }
        (f, fa, o)
    }

    #[test]
    fn egal_a_l_exhaustif() {
        let mut a = Alea(0x9e37_79b9_7f4a_7c15);
        for _ in 0..20_000 {
            let n = 1 + a.next(9) as usize;
            let (ff, fp) = (a.next(10) < 6, a.next(10) < 6);
            let c: Vec<_> = (0..n).map(|_| case(a.next(6) as i32, a.next(6) as i32, a.next(4) as i32, ff, fp)).collect();
            let k = a.next(n as u64 + 2) as usize;
            let seuil = if a.next(3) == 0 { 0 } else { a.next(15) as i32 };
            let sel = choisir(&c, k, seuil).unwrap();
            assert!(sel.len() <= k && sel.windows(2).all(|w| w[0] < w[1]), "{sel:?}");
            assert_eq!(cle(&c, &sel, seuil), exhaustif(&c, k, seuil), "{c:?} k={k} seuil={seuil}");
        }
    }

    /// Le cas qui a lancé l'investigation : deux citoyens, déficit 5, focus nourriture et
    /// production. Le jeu prend 3/2 + 2/1 ; il faut 3/2 + 2/2.
    #[test]
    fn deux_deux_plutot_que_deux_un() {
        let c: Vec<_> = [(2, 2), (2, 2), (3, 2), (1, 4), (2, 1)].iter().map(|&(f, p)| case(f, p, 0, true, true)).collect();
        let sel = choisir(&c, 2, 5).unwrap();
        assert_eq!(sel, vec![0, 2]);
    }

    #[test]
    fn hors_cadre() {
        assert_eq!(choisir(&[(-1, 0, 0)], 1, 0), None, "nourriture négative");
        assert_eq!(choisir(&[(1, 1, 1)], 0, 3), Some(vec![]), "aucun citoyen");
        assert_eq!(choisir(&[], 3, 3), Some(vec![]), "aucun emplacement");
        assert_eq!(choisir(&[(0, 0, 0), (0, 0, 0)], 2, 0), Some(vec![]), "rien à gagner : personne");
    }

    #[test]
    fn disposition_des_candidats() {
        assert_eq!(std::mem::size_of::<Candidat>(), 0x18);
    }
}
