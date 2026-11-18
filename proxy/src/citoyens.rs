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

use crate::citoyens_regle::{choisir, Candidat};
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


type Select = unsafe extern "system" fn(*mut Candidat, *mut Candidat, u32, i32, usize) -> *mut Candidat;


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
            log(&format!("citoyens : détour posé en {:#x}, BetterGameExperience {}", SELECT.address, crate::version::VERSION));
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
