//! Triangulation de Delaunay de `TerrainBuilder.AnalyzeChokepoints` : réécriture de la « fermeture
//! éclair » `1805c9bb0` (réglage `delaunay_rapide=off|on|verifier`).
//!
//! La triangulation se fait par division et fusion (`1805cbc60` → `1805cb9c0` → `1805c3570`). Pour
//! fusionner deux moitiés, `1805c3570` trouve l'arête de base (tangente basse) puis appelle
//! `1805c9bb0` tant qu'elle rend vrai : chaque appel ajoute une arête à la couture et un triangle.
//! C'est 82 % du temps de la génération des goulets (analyses/chargement/README.md, section 3).
//!
//! # Ce que fait l'originale (désassemblée, pas seulement décompilée)
//!
//! Objets (vecteurs MSVC : début, fin, capacité, puis un mot d'allocateur) :
//! - moitié (`0xa8` octets) : `+0x08` index des points (`u32`), `+0x28` arêtes (paires d'index
//!   locaux), `+0x48` triangles (4 `u32` : 3 index d'arêtes, et un 4e jamais touché ici) ;
//! - fusion (même type) : `+0x00` pointeur vers le vecteur des points (`[i32; 2]`), `+0x68` couture
//!   (paires gauche, droite), `+0x88` nouveaux triangles (20 octets, voir [`Triangle`]) ;
//! - 4e argument : petit vecteur de candidats exclus (16 places en ligne).
//!
//! Avec `(bl, br)` la dernière arête de la couture, `d0, d4` = point `bl` − point `br` :
//! 1. côté droit, puis côté gauche, exclus vidés avant chacun : pour chaque index `i` croissant de la
//!    moitié, différent de la base et non exclu, s'il existe une arête `(i, base)` ou `(base, i)` et
//!    que l'orientation entière (`i32` modulaire) est > 0, on calcule en `f32`
//!    `((d4·0,5 + d0)·3·a + d4·2,25·fy) / sqrtf(a·3·a + fy·2,25·fy)` avec `a = fy·0,5 + fx` ; on garde
//!    les deux meilleurs (`>` strict, le premier gagne les égalités). S'il y a un second et que
//!    `1805c8d80` (dans le cercle) le dit, le meilleur est exclu et la première arête
//!    `{meilleur, base}` est supprimée, puis on recommence ce côté ;
//! 2. supprimer l'arête `k` d'une moitié : on retire ses triangles qui citent `k`, les nouveaux
//!    triangles dont la 1re arête est `k` et marquée de ce côté, on décrémente les index `>= k` (non
//!    signés), puis on retire l'arête du vecteur ;
//! 3. aucun candidat : rend faux. Sinon on choisit le côté (s'il y en a deux, `1805c8d80` tranche),
//!    on ajoute l'arête à la couture et un triangle dont la 1re arête est la dernière arête
//!    `{base, candidat}` de ce côté, et rend vrai.
//!
//! # La réécriture
//!
//! L'originale cherche les voisins de la base en parcourant toutes les arêtes pour chaque point :
//! O(points × arêtes) à chaque tour. Ici, un seul parcours des arêtes donne la liste des voisins,
//! triée : même ordre (index croissant), mêmes candidats. Les suppressions sont faites en une passe
//! (même résultat que les décalages un par un). Les calculs en `f32` sont les mêmes, dans le même
//! ordre, avec les mêmes constantes (vérifiées au chargement) ; la racine est la `sqrtf` importée
//! par l'originale (son entrée d'IAT) et le test du cercle est la fonction de l'originale. Les
//! ajouts passent par les fonctions d'insertion de l'originale quand le vecteur est plein : mêmes
//! allocations. La fonction est vérifiée par empreinte (FNV-1a) : une autre version du jeu ne
//! pose rien.
//!
//! Vérification hors du jeu (tests de ce module, sur la vraie DLL de l'installation Steam) : des
//! centaines de milliers de fusions tirées au hasard (doublons, boucles, coordonnées qui débordent
//! en `i32`, vecteurs pleins) et des triangulations entières de cartes au hasard donnent des
//! vecteurs identiques à ceux de l'originale. Sur une carte de 3 600 terres, la triangulation passe
//! d'environ 1,9 s à 15 ms.
//!
//! Réglage `delaunay_rapide` :
//! - `off` : rien n'est posé ;
//! - `on` (par défaut) : la réécriture seule ;
//! - `verifier` : l'originale tourne sur les vraies données, la réécriture sur une copie, et les
//!   vecteurs sont comparés (arêtes et triangles des deux moitiés, couture, nouveaux triangles,
//!   exclus, valeur rendue). Le résultat gardé est celui de l'originale.
//!
//! Chaque triangulation (`1805cbc60`) journalise une ligne : durée, appels, écarts.

use crate::patch::{self, Site, IMAGE_BASE};
use crate::{log, win};
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};

const FERMETURE: Site = Site {
    name: "fermeture éclair de Delaunay",
    address: 0x1_805c_9bb0,
    // mov rax,rsp / mov [rax+20h],r9 / mov [rax+18h],r8 / mov [rax+10h],rdx / mov [rax+8],rcx / push rbp
    expected: &[
        0x48, 0x8b, 0xc4, 0x4c, 0x89, 0x48, 0x20, 0x4c, 0x89, 0x40, 0x18, 0x48, 0x89, 0x50, 0x10, 0x48, 0x89, 0x48,
        0x08, 0x55,
    ],
    copy: (0, 15),
    resume: 15,
};

const TRIANGULATION: Site = Site {
    name: "triangulation de Delaunay",
    address: 0x1_805c_bc60,
    // mov [rsp+8],rbx / mov [rsp+10h],rsi / push rdi / sub rsp,40h
    expected: &[0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xec, 0x40],
    copy: (0, 15),
    resume: 15,
};

/// Corps entier de la fermeture (`1805c9bb0`..`1805cab56`) et du test du cercle
/// (`1805c8d80`..`1805c8fdf`) : empreintes FNV-1a 64 des octets du fichier. Ni l'un ni l'autre n'a
/// de relocation.
const EMPREINTES: [(u64, u64, u64); 2] = [
    (0x1_805c_9bb0, 0x1_805c_ab56, 0x7322_a5b0_b92b_66b6),
    (0x1_805c_8d80, 0x1_805c_8fdf, 0xaf42_4267_f769_5c5b),
];

// Fonctions et données de l'originale, adresses Ghidra.
const DANS_CERCLE: u64 = 0x1_805c_8d80; // (a, b, c, d : *[i32; 2]) -> bool
const INSERER_U32: u64 = 0x1_805d_9cc0; // insertion dans le petit vecteur des exclus
const INSERER_PAIRE: u64 = 0x1_802e_3460; // insertion dans un vecteur de paires (couture)
const INSERER_TRIANGLE: u64 = 0x1_8013_2210; // insertion dans le vecteur des nouveaux triangles
/// `jmp [rip+66ech]` vers l'entrée d'IAT de `sqrtf` (api-ms-win-crt-math).
const SQRTF_THUNK: (u64, [u8; 6]) = (0x1_809b_0fd6, [0xff, 0x25, 0xec, 0x66, 0x00, 0x00]);
const SQRTF_IAT: u64 = 0x1_809b_76c8;
/// Constantes lues par la fermeture (`DAT_1809d18b0`, `DAT_1809d62dc`, `DAT_180a09038`).
const DEMI: f32 = 0.5;
const TROIS: f32 = 3.0;
const K: f32 = 2.25;
const CONSTANTES: [(u64, f32); 3] = [(0x1_809d_18b0, DEMI), (0x1_809d_62dc, TROIS), (0x1_80a0_9038, K)];

// Décalages dans les objets.
const IDX: usize = 0x08;
const ARETES: usize = 0x28;
const TRIANGLES: usize = 0x48;
const COUTURE: usize = 0x68;
const NOUVEAUX: usize = 0x88;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Mode {
    Off = 0,
    On = 1,
    Verifier = 2,
}

static MODE: AtomicU8 = AtomicU8::new(Mode::On as u8);
static BASE: AtomicUsize = AtomicUsize::new(0);
static TRAMP_FERMETURE: AtomicUsize = AtomicUsize::new(0);
static TRAMP_TRIANGULATION: AtomicUsize = AtomicUsize::new(0);
static HOOKED_BASE: AtomicUsize = AtomicUsize::new(0);

pub fn mode() -> Mode {
    match MODE.load(Ordering::Relaxed) {
        1 => Mode::On,
        2 => Mode::Verifier,
        _ => Mode::Off,
    }
}

pub fn set_mode(m: Mode) {
    MODE.store(m as u8, Ordering::Relaxed);
}

/// Journal du proxy ; dans les tests, la sortie d'erreur (pas le journal du joueur).
fn journal(msg: &str) {
    #[cfg(test)]
    eprintln!("{msg}");
    #[cfg(not(test))]
    log(msg);
}

/// Compteurs de couverture des branches, pour les tests.
#[cfg(test)]
static COUVERTURE: [AtomicU64; 6] = [const { AtomicU64::new(0) }; 6];
macro_rules! couvre {
    ($i:expr) => {
        #[cfg(test)]
        COUVERTURE[$i].fetch_add(1, Ordering::Relaxed);
    };
}

fn gc(addr: u64) -> usize {
    BASE.load(Ordering::Relaxed) + (addr - IMAGE_BASE) as usize
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

type FnCercle = unsafe extern "system" fn(*const [i32; 2], *const [i32; 2], *const [i32; 2], *const [i32; 2]) -> u8;
type FnRacine = unsafe extern "C" fn(f32) -> f32;
type FnFermeture = unsafe extern "system" fn(usize, usize, usize, usize) -> u8;
type FnTriangulation = unsafe extern "system" fn(usize);

/// Les deux fonctions de l'originale dont la réécriture reprend le résultat.
#[derive(Clone, Copy)]
pub(crate) struct Fonctions {
    pub dans_cercle: FnCercle,
    pub racine: FnRacine,
}

unsafe fn fonctions() -> Fonctions {
    Fonctions {
        dans_cercle: std::mem::transmute::<usize, FnCercle>(gc(DANS_CERCLE)),
        racine: std::mem::transmute::<usize, FnRacine>(rd::<usize>(gc(SQRTF_IAT))),
    }
}

// ---------------------------------------------------------------------------------------------
// La réécriture, sur des tranches : celles du jeu (mode on) ou des copies (mode verifier, tests).

/// Nouveau triangle de la fusion (20 octets) : trois index d'arêtes, puis pour chacune un drapeau
/// « arête de la moitié gauche » (`+0xc..+0xe`, `drapeaux[0..3]`) et « arête de la moitié droite »
/// (`+0xf..+0x11`, `drapeaux[3..6]`) ; ni l'un ni l'autre : arête de la couture. `+0x12` et `+0x13`
/// restent à zéro.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Triangle {
    pub aretes: [u32; 3],
    pub drapeaux: [u8; 8],
}
const _: () = assert!(std::mem::size_of::<Triangle>() == 20);

pub(crate) struct Moitie<'a> {
    pub idx: &'a [u32],
    pub aretes: &'a mut [[u32; 2]],
    pub n_aretes: usize,
    pub triangles: &'a mut [[u32; 4]],
    pub n_triangles: usize,
}

pub(crate) struct Fusion<'a> {
    pub points: &'a [[i32; 2]],
    pub gauche: Moitie<'a>,
    pub droite: Moitie<'a>,
    pub nouveaux: &'a mut [Triangle],
    pub n_nouveaux: usize,
    /// Dernière arête de la couture : index local à gauche, index local à droite.
    pub base: [u32; 2],
    /// Longueur de la couture avant l'appel.
    pub n_couture: usize,
}

/// Ce que l'appel ajoute : exclus de chaque côté (dans l'ordre), arête de couture et triangle.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Issue {
    pub exclus_droite: Vec<u32>,
    pub exclus_gauche: Vec<u32>,
    pub ajout: Option<([u32; 2], Triangle)>,
}

const AUCUN: u32 = u32::MAX;

/// Orientation de `p` par rapport à la base, en `i32` modulaire comme l'originale (même formule des
/// deux côtés).
#[inline]
fn orientation(bl: [i32; 2], br: [i32; 2], p: [i32; 2]) -> i32 {
    let a = br[0].wrapping_sub(bl[0]).wrapping_mul(2).wrapping_sub(bl[1]).wrapping_add(br[1]);
    let a = a.wrapping_mul(p[1].wrapping_sub(br[1]));
    let b = p[0].wrapping_sub(br[0]).wrapping_mul(2).wrapping_sub(br[1]).wrapping_add(p[1]);
    let b = b.wrapping_mul(br[1].wrapping_sub(bl[1]));
    a.wrapping_sub(b)
}

/// La mesure en `f32` de l'originale, opération par opération (`1805c9dbb`..`1805c9e36`).
#[inline]
unsafe fn mesure(d0: i32, d4: i32, dx: i32, dy: i32, racine: FnRacine) -> f32 {
    let (fx, fy) = (dx as f32, dy as f32);
    let (d0, d4) = (d0 as f32, d4 as f32);
    let a = fy * DEMI + fx;
    let num = (d4 * DEMI + d0) * TROIS * a + d4 * K * fy;
    num / racine(a * TROIS * a + fy * K * fy)
}

/// Supprime l'arête `k` d'une moitié, comme `1805c9f86`..`1805ca123` (droite) et
/// `1805ca47b`..`1805ca623` (gauche).
fn supprimer(h: &mut Moitie, nouveaux: &mut [Triangle], n_nouveaux: &mut usize, droite: bool, k: u32) {
    let mut w = 0;
    for r in 0..h.n_triangles {
        let t = h.triangles[r];
        if t[0] != k && t[1] != k && t[2] != k {
            h.triangles[w] = t;
            w += 1;
        }
    }
    h.n_triangles = w;
    let o = if droite { 3 } else { 0 };
    let mut w = 0;
    for r in 0..*n_nouveaux {
        let t = nouveaux[r];
        if t.drapeaux[o] == 0 || t.aretes[0] != k {
            nouveaux[w] = t;
            w += 1;
        }
    }
    *n_nouveaux = w;
    for t in &mut h.triangles[..h.n_triangles] {
        for v in &mut t[..3] {
            if *v >= k {
                *v = v.wrapping_sub(1);
            }
        }
    }
    for t in &mut nouveaux[..*n_nouveaux] {
        for j in 0..3 {
            if t.drapeaux[o + j] != 0 && t.aretes[j] >= k {
                t.aretes[j] = t.aretes[j].wrapping_sub(1);
            }
        }
    }
    let n = h.n_aretes;
    h.aretes.copy_within(k as usize + 1..n, k as usize);
    h.n_aretes = n - 1;
}

/// Contexte commun aux deux côtés.
struct Base {
    p_bl: [i32; 2],
    p_br: [i32; 2],
    d0: i32,
    d4: i32,
}

/// Candidat d'un côté (`AUCUN` s'il n'y en a pas), avec les exclusions et suppressions faites en
/// chemin.
#[allow(clippy::too_many_arguments)]
unsafe fn candidat(
    points: &[[i32; 2]],
    h: &mut Moitie,
    nouveaux: &mut [Triangle],
    n_nouveaux: &mut usize,
    droite: bool,
    base: u32,
    b: &Base,
    exclus: &mut Vec<u32>,
    f: &Fonctions,
) -> u32 {
    // L'originale borne la boucle par le nombre de points en `i32` (lu ici < 2^31).
    let n = h.idx.len();
    if n == 0 {
        return AUCUN;
    }
    let mut voisins: Vec<u32> = Vec::new();
    loop {
        voisins.clear();
        for &[a, c] in &h.aretes[..h.n_aretes] {
            let j = if a == base {
                c
            } else if c == base {
                a
            } else {
                continue;
            };
            if j != base && (j as usize) < n {
                voisins.push(j);
            }
        }
        voisins.sort_unstable();
        voisins.dedup();
        let (mut meilleur, mut second, mut fa, mut fb) = (AUCUN, AUCUN, 0f32, 0f32);
        for &i in &voisins {
            if exclus.contains(&i) {
                continue;
            }
            let p = points[h.idx[i as usize] as usize];
            if orientation(b.p_bl, b.p_br, p) <= 0 {
                continue;
            }
            let (dx, dy) = if droite {
                (p[0].wrapping_sub(b.p_br[0]), p[1].wrapping_sub(b.p_br[1]))
            } else {
                (b.p_bl[0].wrapping_sub(p[0]), b.p_bl[1].wrapping_sub(p[1]))
            };
            let v = mesure(b.d0, b.d4, dx, dy, f.racine);
            // `comiss v, fa / ja` puis `comiss v, fb / jbe` : `>` strict, faux si NaN.
            if meilleur == AUCUN || v > fa {
                second = meilleur;
                fb = fa;
                meilleur = i;
                fa = v;
            } else if second == AUCUN || v > fb {
                second = i;
                fb = v;
            }
        }
        if meilleur == AUCUN || second == AUCUN {
            return meilleur;
        }
        let pm = &points[h.idx[meilleur as usize] as usize];
        let ps = &points[h.idx[second as usize] as usize];
        if (f.dans_cercle)(&b.p_bl, &b.p_br, pm, ps) == 0 {
            return meilleur;
        }
        couvre!(0);
        exclus.push(meilleur);
        let k = (0..h.n_aretes).find(|&k| {
            let [a, c] = h.aretes[k];
            (a == meilleur || c == meilleur) && (a == base || c == base)
        });
        if let Some(k) = k {
            couvre!(1);
            supprimer(h, nouveaux, n_nouveaux, droite, k as u32);
        }
    }
}

/// Dernière arête dont les deux bouts sont dans `{a, b}` (`AUCUN` sinon), comme `1805ca730`.
fn derniere(h: &Moitie, a: u32, b: u32) -> u32 {
    let mut r = AUCUN;
    for (k, &[e0, e1]) in h.aretes[..h.n_aretes].iter().enumerate() {
        if (e0 == a || e0 == b) && (e1 == a || e1 == b) {
            r = k as u32;
        }
    }
    r
}

/// Un appel de la fermeture. Les suppressions sont faites dans les tranches (longueurs mises à
/// jour dans `fu`) ; les ajouts sont rendus, à faire par l'appelant.
pub(crate) unsafe fn fermeture(fu: &mut Fusion, f: &Fonctions) -> Issue {
    let [bl, br] = fu.base;
    let points = fu.points;
    let p_bl = points[fu.gauche.idx[bl as usize] as usize];
    let p_br = points[fu.droite.idx[br as usize] as usize];
    let b = Base { p_bl, p_br, d0: p_bl[0].wrapping_sub(p_br[0]), d4: p_bl[1].wrapping_sub(p_br[1]) };
    let mut exclus_droite = Vec::new();
    let rc = candidat(points, &mut fu.droite, fu.nouveaux, &mut fu.n_nouveaux, true, br, &b, &mut exclus_droite, f);
    let mut exclus_gauche = Vec::new();
    let lc = candidat(points, &mut fu.gauche, fu.nouveaux, &mut fu.n_nouveaux, false, bl, &b, &mut exclus_gauche, f);
    // Longueur de la couture après l'ajout, en 32 bits comme l'originale.
    let n = (fu.n_couture + 1) as u32;
    let ajout = if lc == AUCUN && rc == AUCUN {
        couvre!(2);
        None
    } else {
        let a_gauche = if lc == AUCUN {
            false
        } else if rc == AUCUN {
            true
        } else {
            let pr = &points[fu.droite.idx[rc as usize] as usize];
            let pl = &points[fu.gauche.idx[lc as usize] as usize];
            couvre!(3);
            (f.dans_cercle)(&p_bl, &p_br, pr, pl) != 0
        };
        couvre!(if a_gauche { 4 } else { 5 });
        Some(if a_gauche {
            let k = derniere(&fu.gauche, bl, lc);
            ([lc, br], Triangle { aretes: [k, n.wrapping_sub(2), n.wrapping_sub(1)], drapeaux: [1, 0, 0, 0, 0, 0, 0, 0] })
        } else {
            let k = derniere(&fu.droite, br, rc);
            ([bl, rc], Triangle { aretes: [k, n.wrapping_sub(1), n.wrapping_sub(2)], drapeaux: [0, 0, 0, 1, 0, 0, 0, 0] })
        })
    };
    Issue { exclus_droite, exclus_gauche, ajout }
}

// ---------------------------------------------------------------------------------------------
// Lecture et écriture des objets du jeu.

/// Vecteur MSVC : début, fin, fin de la capacité.
#[repr(C)]
struct VecJeu {
    debut: usize,
    fin: usize,
    cap: usize,
}

/// Longueur d'un vecteur d'éléments de `taille` octets, s'il est cohérent et de moins de 2^31
/// éléments (l'originale compte en `i32`).
unsafe fn longueur(v: usize, taille: usize) -> Option<usize> {
    let v = &*(v as *const VecJeu);
    if v.fin < v.debut || v.cap < v.fin || (v.fin - v.debut) % taille != 0 || v.debut % 4 != 0 {
        return None;
    }
    let n = (v.fin - v.debut) / taille;
    (n < 1 << 31 && (n == 0 || v.debut != 0)).then_some(n)
}

unsafe fn tranche<'a, T>(v: usize, n: usize) -> &'a mut [T] {
    if n == 0 {
        &mut []
    } else {
        std::slice::from_raw_parts_mut((*(v as *const VecJeu)).debut as *mut T, n)
    }
}

/// Adresses et longueurs lues dans les objets, validées : les index lus par la fermeture
/// (index de points, bases) sont dans les bornes.
struct Brut {
    points: (usize, usize),
    idx: [(usize, usize); 2],
    aretes: [(usize, usize); 2],
    triangles: [(usize, usize); 2],
    nouveaux: (usize, usize),
    couture: (usize, usize),
    base: [u32; 2],
}

unsafe fn lire(m: usize, g: usize, d: usize) -> Option<Brut> {
    let pv = rd::<usize>(m);
    if pv == 0 {
        return None;
    }
    let np = longueur(pv, 8)?;
    let couture = (m + COUTURE, longueur(m + COUTURE, 8)?);
    if couture.1 == 0 {
        return None;
    }
    let mut brut = Brut {
        points: (pv, np),
        idx: [(g + IDX, longueur(g + IDX, 4)?), (d + IDX, longueur(d + IDX, 4)?)],
        aretes: [(g + ARETES, longueur(g + ARETES, 8)?), (d + ARETES, longueur(d + ARETES, 8)?)],
        triangles: [(g + TRIANGLES, longueur(g + TRIANGLES, 16)?), (d + TRIANGLES, longueur(d + TRIANGLES, 16)?)],
        nouveaux: (m + NOUVEAUX, longueur(m + NOUVEAUX, 20)?),
        couture,
        base: [0, 0],
    };
    brut.base = rd::<[u32; 2]>((*((m + COUTURE) as *const VecJeu)).fin - 8);
    for (cote, &(v, n)) in brut.idx.iter().enumerate() {
        if brut.base[cote] as usize >= n || tranche::<u32>(v, n).iter().any(|&p| p as usize >= np) {
            return None;
        }
    }
    Some(brut)
}

impl Brut {
    /// La fusion sur les objets du jeu eux-mêmes.
    unsafe fn en_place<'a>(&self) -> Fusion<'a> {
        let moitie = |c: usize| Moitie {
            idx: tranche(self.idx[c].0, self.idx[c].1),
            aretes: tranche(self.aretes[c].0, self.aretes[c].1),
            n_aretes: self.aretes[c].1,
            triangles: tranche(self.triangles[c].0, self.triangles[c].1),
            n_triangles: self.triangles[c].1,
        };
        Fusion {
            points: tranche(self.points.0, self.points.1),
            gauche: moitie(0),
            droite: moitie(1),
            nouveaux: tranche(self.nouveaux.0, self.nouveaux.1),
            n_nouveaux: self.nouveaux.1,
            base: self.base,
            n_couture: self.couture.1,
        }
    }
}

/// Copie des vecteurs que la fermeture modifie.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Copie {
    aretes: [Vec<[u32; 2]>; 2],
    triangles: [Vec<[u32; 4]>; 2],
    nouveaux: Vec<Triangle>,
}

impl Copie {
    unsafe fn de(brut: &Brut) -> Copie {
        let v = |(a, n): (usize, usize)| tranche::<[u32; 2]>(a, n).to_vec();
        let t = |(a, n): (usize, usize)| tranche::<[u32; 4]>(a, n).to_vec();
        Copie {
            aretes: [v(brut.aretes[0]), v(brut.aretes[1])],
            triangles: [t(brut.triangles[0]), t(brut.triangles[1])],
            nouveaux: tranche::<Triangle>(brut.nouveaux.0, brut.nouveaux.1).to_vec(),
        }
    }

    unsafe fn fusion<'a>(&'a mut self, brut: &Brut) -> Fusion<'a> {
        let [ag, ad] = &mut self.aretes;
        let [tg, td] = &mut self.triangles;
        Fusion {
            points: tranche(brut.points.0, brut.points.1),
            gauche: Moitie {
                idx: tranche(brut.idx[0].0, brut.idx[0].1),
                n_aretes: ag.len(),
                aretes: ag,
                n_triangles: tg.len(),
                triangles: tg,
            },
            droite: Moitie {
                idx: tranche(brut.idx[1].0, brut.idx[1].1),
                n_aretes: ad.len(),
                aretes: ad,
                n_triangles: td.len(),
                triangles: td,
            },
            n_nouveaux: self.nouveaux.len(),
            nouveaux: &mut self.nouveaux,
            base: brut.base,
            n_couture: brut.couture.1,
        }
    }
}

/// Ajoute `x` en fin de vecteur, comme l'originale : en place s'il reste de la place, sinon par
/// sa fonction d'insertion (même croissance, même allocateur).
unsafe fn pousser<T: Copy>(v: usize, x: T, inserer: u64) {
    let vj = v as *mut VecJeu;
    let fin = (*vj).fin;
    if fin < (*vj).cap {
        (*vj).fin = fin + std::mem::size_of::<T>();
        if fin != 0 {
            std::ptr::write_unaligned(fin as *mut T, x);
        }
    } else {
        let f: unsafe extern "system" fn(usize, usize, *const T) -> usize = std::mem::transmute(gc(inserer));
        f(v, fin, &x);
    }
}

unsafe fn fixer_longueur(v: usize, n: usize, taille: usize) {
    let vj = v as *mut VecJeu;
    (*vj).fin = (*vj).debut + n * taille;
}

/// Mode on : la réécriture sur les objets du jeu, puis les ajouts.
unsafe fn rapide(brut: &Brut, exclus: usize) -> u8 {
    let mut fu = brut.en_place();
    let issue = fermeture(&mut fu, &fonctions());
    let (ng, nd, nt) = (
        (fu.gauche.n_aretes, fu.gauche.n_triangles),
        (fu.droite.n_aretes, fu.droite.n_triangles),
        fu.n_nouveaux,
    );
    drop(fu);
    fixer_longueur(brut.aretes[0].0, ng.0, 8);
    fixer_longueur(brut.triangles[0].0, ng.1, 16);
    fixer_longueur(brut.aretes[1].0, nd.0, 8);
    fixer_longueur(brut.triangles[1].0, nd.1, 16);
    fixer_longueur(brut.nouveaux.0, nt, 20);
    // Les exclus : vidés, remplis à droite, vidés, remplis à gauche, comme l'originale.
    for liste in [&issue.exclus_droite, &issue.exclus_gauche] {
        fixer_longueur(exclus, 0, 4);
        for &x in liste {
            pousser(exclus, x, INSERER_U32);
        }
    }
    match issue.ajout {
        Some((arete, triangle)) => {
            pousser(brut.couture.0, arete, INSERER_PAIRE);
            pousser(brut.nouveaux.0, triangle, INSERER_TRIANGLE);
            1
        }
        None => 0,
    }
}

unsafe fn original(m: usize, g: usize, d: usize, exclus: usize) -> u8 {
    let f: FnFermeture = std::mem::transmute(TRAMP_FERMETURE.load(Ordering::Relaxed));
    f(m, g, d, exclus)
}

/// Mode verifier : la réécriture sur une copie, l'originale sur les vraies données, puis la
/// comparaison. Rend la valeur de l'originale.
unsafe fn verifier(brut: &Brut, m: usize, g: usize, d: usize, exclus: usize) -> u8 {
    let mut copie = Copie::de(brut);
    let mut fu = copie.fusion(brut);
    let issue = fermeture(&mut fu, &fonctions());
    let lens = ([fu.gauche.n_aretes, fu.droite.n_aretes], [fu.gauche.n_triangles, fu.droite.n_triangles], fu.n_nouveaux);
    drop(fu);
    for c in 0..2 {
        copie.aretes[c].truncate(lens.0[c]);
        copie.triangles[c].truncate(lens.1[c]);
    }
    copie.nouveaux.truncate(lens.2);
    if let Some((_, t)) = issue.ajout {
        copie.nouveaux.push(t);
    }

    let r = original(m, g, d, exclus);

    let ecart = (|| {
        let (ag, ad) = (longueur(g + ARETES, 8)?, longueur(d + ARETES, 8)?);
        let (tg, td) = (longueur(g + TRIANGLES, 16)?, longueur(d + TRIANGLES, 16)?);
        let reel = Copie {
            aretes: [tranche::<[u32; 2]>(g + ARETES, ag).to_vec(), tranche::<[u32; 2]>(d + ARETES, ad).to_vec()],
            triangles: [tranche::<[u32; 4]>(g + TRIANGLES, tg).to_vec(), tranche::<[u32; 4]>(d + TRIANGLES, td).to_vec()],
            nouveaux: tranche::<Triangle>(m + NOUVEAUX, longueur(m + NOUVEAUX, 20)?).to_vec(),
        };
        let mut quoi = Vec::new();
        if (r != 0) != issue.ajout.is_some() {
            quoi.push(format!("valeur rendue {r} au lieu de {}", issue.ajout.is_some() as u8));
        }
        for (c, nom) in [(0, "gauche"), (1, "droite")] {
            if reel.aretes[c] != copie.aretes[c] {
                quoi.push(format!("arêtes {nom} ({} au lieu de {})", reel.aretes[c].len(), copie.aretes[c].len()));
            }
            if reel.triangles[c] != copie.triangles[c] {
                quoi.push(format!("triangles {nom} ({} au lieu de {})", reel.triangles[c].len(), copie.triangles[c].len()));
            }
        }
        if reel.nouveaux != copie.nouveaux {
            quoi.push(format!("nouveaux triangles ({} au lieu de {})", reel.nouveaux.len(), copie.nouveaux.len()));
        }
        let couture = tranche::<[u32; 2]>(m + COUTURE, longueur(m + COUTURE, 8)?);
        let attendue = brut.couture.1 + issue.ajout.is_some() as usize;
        if couture.len() != attendue || issue.ajout.is_some_and(|(a, _)| couture.last() != Some(&a)) {
            quoi.push(format!("couture ({} au lieu de {attendue})", couture.len()));
        }
        if tranche::<u32>(exclus, longueur(exclus, 4)?) != &issue.exclus_gauche[..] {
            quoi.push("exclus".to_string());
        }
        Some(quoi)
    })()
    .unwrap_or_else(|| vec!["vecteurs illisibles après l'originale".to_string()]);
    if !ecart.is_empty() {
        let n = ECARTS.fetch_add(1, Ordering::Relaxed);
        if n < 5 {
            journal(&format!("delaunay : écart, base ({}, {}) : {}", brut.base[0], brut.base[1], ecart.join(", ")));
        }
    }
    r
}

// Statistiques d'une triangulation, journalisées à sa fin.
static APPELS: AtomicU64 = AtomicU64::new(0);
static TICKS: AtomicU64 = AtomicU64::new(0);
static ECARTS: AtomicU64 = AtomicU64::new(0);
static IGNORES: AtomicU64 = AtomicU64::new(0);

unsafe extern "system" fn hook_fermeture(m: usize, g: usize, d: usize, exclus: usize) -> u8 {
    let t0 = win::qpc();
    let mode = mode();
    let brut = if mode == Mode::Off { None } else { lire(m, g, d) };
    let r = match (mode, brut) {
        (Mode::On, Some(b)) => rapide(&b, exclus),
        (Mode::Verifier, Some(b)) => verifier(&b, m, g, d, exclus),
        (Mode::Off, _) => original(m, g, d, exclus),
        (_, None) => {
            IGNORES.fetch_add(1, Ordering::Relaxed);
            original(m, g, d, exclus)
        }
    };
    APPELS.fetch_add(1, Ordering::Relaxed);
    TICKS.fetch_add((win::qpc() - t0) as u64, Ordering::Relaxed);
    r
}

unsafe extern "system" fn hook_triangulation(constructeur: usize) {
    let t0 = win::qpc();
    let f: FnTriangulation = std::mem::transmute(TRAMP_TRIANGULATION.load(Ordering::Relaxed));
    f(constructeur);
    let ms = win::qpc_ms(win::qpc() - t0);
    let points = longueur(constructeur + 8, 8).unwrap_or(0);
    log(&format!(
        "delaunay : triangulation de {points} points en {ms:.0} ms, mode {:?}, fermetures {} ({:.0} ms), écarts {}, \
         ignorées {}",
        mode(),
        APPELS.swap(0, Ordering::Relaxed),
        win::qpc_ms(TICKS.swap(0, Ordering::Relaxed) as i64),
        ECARTS.swap(0, Ordering::Relaxed),
        IGNORES.swap(0, Ordering::Relaxed),
    ));
}

fn fnv1a(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

/// Tout ce dont la réécriture dépend, vérifié avant de poser quoi que ce soit.
unsafe fn verifier_version(base: usize) -> Result<(), String> {
    patch::check(base, &[FERMETURE, TRIANGULATION])?;
    let a = |x: u64| base + (x - IMAGE_BASE) as usize;
    for (debut, fin, attendue) in EMPREINTES {
        let h = fnv1a(std::slice::from_raw_parts(a(debut) as *const u8, (fin - debut) as usize));
        if h != attendue {
            return Err(format!("empreinte de {debut:#x} : {h:#x} au lieu de {attendue:#x}"));
        }
    }
    for (adr, valeur) in CONSTANTES {
        let lu = rd::<u32>(a(adr));
        if lu != valeur.to_bits() {
            return Err(format!("constante en {adr:#x} : {lu:#x} au lieu de {:#x}", valeur.to_bits()));
        }
    }
    if rd::<[u8; 6]>(a(SQRTF_THUNK.0)) != SQRTF_THUNK.1 || rd::<usize>(a(SQRTF_IAT)) == 0 {
        return Err("saut vers sqrtf inattendu".to_string());
    }
    Ok(())
}

/// Appelé quand l'originale vient d'être chargée, avant le premier appel relayé.
pub unsafe fn on_gamecore_loaded(base: usize) {
    if mode() == Mode::Off {
        log("delaunay : réglage off, fermeture d'origine");
        return;
    }
    if let Err(e) = verifier_version(base) {
        log(&format!("delaunay : version inattendue, aucun détour : {e}"));
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    let t = match patch::install(base, &TRIANGULATION, hook_triangulation as *const () as usize) {
        Ok(t) => t,
        Err(e) => {
            log(&format!("delaunay : échec de la pose du détour : {e}"));
            return;
        }
    };
    TRAMP_TRIANGULATION.store(t, Ordering::Relaxed);
    match patch::install(base, &FERMETURE, hook_fermeture as *const () as usize) {
        Ok(t) => {
            TRAMP_FERMETURE.store(t, Ordering::Relaxed);
            HOOKED_BASE.store(base, Ordering::Relaxed);
            log(&format!("delaunay : détours posés, mode {:?}", mode()));
        }
        Err(e) => {
            patch::restore(base, &TRIANGULATION);
            log(&format!("delaunay : échec de la pose du détour : {e}"));
        }
    }
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet ses octets d'origine.
pub unsafe fn on_proxy_unload() {
    let base = HOOKED_BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        patch::restore(base, &FERMETURE);
        patch::restore(base, &TRIANGULATION);
    }
}

/// Banc d'essai hors du jeu : l'originale est chargée depuis l'installation Steam (sans DllMain,
/// qui échoue hors du jeu ; imports système et CRT résolus à la main), la fermeture d'origine est
/// comparée à la réécriture sur des fusions tirées au hasard, puis sur des triangulations entières
/// (`1805c9960` puis `1805cb9c0` jusqu'à une seule moitié, comme `1805cbc60` sans son journal).
/// Sauté si le jeu n'est pas installé. `BGE_DELAUNAY_CAS=<n>` multiplie le nombre de cas.
#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryExW(name: *const u16, file: *mut c_void, flags: u32) -> win::HMODULE;
        fn LoadLibraryA(name: *const u8) -> win::HMODULE;
    }
    const DONT_RESOLVE_DLL_REFERENCES: u32 = 1;
    const PAGE_READWRITE: u32 = 4;
    const DLL: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\DLC\Expansion2\Binaries\Win64\GameCore_XP2_FinalRelease.dll";

    type Malloc = unsafe extern "C" fn(usize, usize) -> usize;
    type Free = unsafe extern "C" fn(usize);

    struct Jeu {
        malloc: Malloc,
        free: Free,
    }

    fn cstr(p: usize) -> String {
        unsafe { std::ffi::CStr::from_ptr(p as *const std::ffi::c_char) }.to_string_lossy().into_owned()
    }

    /// L'originale chargée sans DllMain, imports système et CRT résolus ; les autres DLL du jeu
    /// (base de données, Havok…) ne servent pas aux fonctions appelées ici.
    unsafe fn charger() -> Option<(usize, Jeu)> {
        if !std::path::Path::new(DLL).exists() {
            return None;
        }
        let h = LoadLibraryExW(win::wide(DLL).as_ptr(), std::ptr::null_mut(), DONT_RESOLVE_DLL_REFERENCES) as usize;
        assert_ne!(h, 0, "chargement de l'originale");
        let pe = h + rd::<u32>(h + 0x3c) as usize;
        let mut desc = h + rd::<u32>(pe + 0x18 + 0x78) as usize;
        while rd::<u32>(desc + 12) != 0 {
            let nom = cstr(h + rd::<u32>(desc + 12) as usize).to_ascii_lowercase();
            let systeme = ["api-ms-win-crt", "kernel32", "vcruntime", "msvcp", "ole32"].iter().any(|p| nom.starts_with(p));
            let module = if systeme { LoadLibraryA(format!("{nom}\0").as_ptr()) as usize } else { 0 };
            let (oft, ft) = (rd::<u32>(desc) as usize, rd::<u32>(desc + 16) as usize);
            let mut j = 0;
            while module != 0 {
                let lookup = rd::<u64>(h + if oft != 0 { oft } else { ft } + 8 * j);
                if lookup == 0 {
                    break;
                }
                let p = if lookup >> 63 != 0 {
                    win::GetProcAddress(module as win::HMODULE, (lookup & 0xffff) as usize as *const u8)
                } else {
                    let n = format!("{}\0", cstr(h + (lookup & 0x7fff_ffff) as usize + 2));
                    win::GetProcAddress(module as win::HMODULE, n.as_ptr())
                } as usize;
                if p != 0 {
                    let slot = h + ft + 8 * j;
                    let mut old = 0;
                    win::VirtualProtect(slot as *mut c_void, 8, PAGE_READWRITE, &mut old);
                    std::ptr::write(slot as *mut usize, p);
                    win::VirtualProtect(slot as *mut c_void, 8, old, &mut old);
                }
                j += 1;
            }
            desc += 20;
        }
        let ucrt = LoadLibraryA(b"ucrtbase.dll\0".as_ptr());
        let malloc = win::GetProcAddress(ucrt, b"_aligned_malloc\0".as_ptr()) as usize;
        let free = win::GetProcAddress(ucrt, b"_aligned_free\0".as_ptr()) as usize;
        Some((h, Jeu { malloc: std::mem::transmute::<usize, Malloc>(malloc), free: std::mem::transmute::<usize, Free>(free) }))
    }

    /// Générateur pseudo-aléatoire (xorshift64*), reproductible.
    struct Hasard(u64);
    impl Hasard {
        fn suivant(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }
        fn sous(&mut self, n: u64) -> u64 {
            if n == 0 {
                0
            } else {
                self.suivant() % n
            }
        }
        fn entre(&mut self, a: i64, b: i64) -> i64 {
            a + self.sous((b - a + 1) as u64) as i64
        }
        fn piece(&mut self, sur: u64) -> bool {
            self.sous(sur) == 0
        }
    }

    impl Jeu {
        /// Remplit le vecteur MSVC en `at` avec `data` et `marge` places libres, dans le tas de la CRT.
        unsafe fn vecteur<T: Copy>(&self, at: usize, data: &[T], marge: usize) {
            let t = std::mem::size_of::<T>();
            let cap = data.len() + marge;
            let p = if cap == 0 { 0 } else { (self.malloc)((cap * t + 15) & !15, 16) };
            if !data.is_empty() {
                std::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, p as *mut u8, data.len() * t);
            }
            let v = at as *mut VecJeu;
            (*v).debut = p;
            (*v).fin = p + data.len() * t;
            (*v).cap = p + cap * t;
        }

        unsafe fn liberer(&self, at: usize) {
            let v = at as *mut VecJeu;
            if (*v).debut != 0 {
                (self.free)((*v).debut);
            }
            (*v).debut = 0;
            (*v).fin = 0;
            (*v).cap = 0;
        }
    }

    unsafe fn contenu<T: Copy>(at: usize) -> Vec<T> {
        let v = &*(at as *const VecJeu);
        let n = (v.fin - v.debut) / std::mem::size_of::<T>();
        tranche::<T>(at, n).to_vec()
    }

    /// Une fusion tirée au hasard, en données Rust.
    #[derive(Clone)]
    struct Cas {
        points: Vec<[i32; 2]>,
        idx: [Vec<u32>; 2],
        aretes: [Vec<[u32; 2]>; 2],
        triangles: [Vec<[u32; 4]>; 2],
        couture: Vec<[u32; 2]>,
        nouveaux: Vec<Triangle>,
        marge: usize,
    }

    fn tirer(r: &mut Hasard) -> Cas {
        let echelle = [2i64, 6, 40, 1000, 100_000, 1 << 30][r.sous(6) as usize];
        let np = r.entre(3, 120) as usize;
        let points = (0..np).map(|_| [r.entre(-echelle, echelle) as i32, r.entre(-echelle, echelle) as i32]).collect();
        let mut idx: [Vec<u32>; 2] = Default::default();
        let mut aretes: [Vec<[u32; 2]>; 2] = Default::default();
        let mut triangles: [Vec<[u32; 4]>; 2] = Default::default();
        let mut bases = [0u32; 2];
        for c in 0..2 {
            let n = r.entre(1, 30) as u32;
            idx[c] = (0..n).map(|_| r.sous(np as u64) as u32).collect();
            bases[c] = r.sous(n as u64) as u32;
            let na = r.sous(3 * n as u64 + 2);
            for _ in 0..na {
                let mut a = r.sous(n as u64 + 2) as u32;
                let mut b = r.sous(n as u64 + 2) as u32;
                if r.piece(2) {
                    // Arêtes de la base, dans un sens ou dans l'autre (parfois en double).
                    if r.piece(2) {
                        a = bases[c];
                    } else {
                        b = bases[c];
                    }
                }
                aretes[c].push([a, b]);
            }
            let nt = r.sous(2 * n as u64 + 2);
            for _ in 0..nt {
                let m = na + 2;
                triangles[c].push([r.sous(m) as u32, r.sous(m) as u32, r.sous(m) as u32, r.suivant() as u32]);
            }
        }
        let mut couture: Vec<[u32; 2]> = (0..r.sous(3)).map(|_| [r.suivant() as u32, r.suivant() as u32]).collect();
        couture.push(bases);
        let m = aretes[0].len().max(aretes[1].len()) as u64 + 2;
        let nouveaux = (0..r.sous(10))
            .map(|_| {
                let mut d = [0u8; 8];
                for x in &mut d[..6] {
                    *x = r.piece(2) as u8;
                }
                d[6] = r.sous(3) as u8;
                Triangle { aretes: [r.sous(m) as u32, r.sous(m) as u32, r.sous(m) as u32], drapeaux: d }
            })
            .collect();
        Cas { points, idx, aretes, triangles, couture, nouveaux, marge: [0, 1, 8][r.sous(3) as usize] }
    }

    /// Les objets du jeu pour un cas : fusion, deux moitiés, vecteur des points, petit vecteur des
    /// exclus (16 places en ligne, comme dans `1805c3570`).
    struct Objets {
        m: Box<[usize; 21]>,
        g: Box<[usize; 21]>,
        d: Box<[usize; 21]>,
        pts: Box<[usize; 4]>,
        exclus: Box<[usize; 13]>,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Etat {
        rendu: u8,
        idx: [Vec<u32>; 2],
        aretes: [Vec<[u32; 2]>; 2],
        triangles: [Vec<[u32; 4]>; 2],
        couture: Vec<[u32; 2]>,
        nouveaux: Vec<Triangle>,
        exclus: Vec<u32>,
    }

    impl Objets {
        unsafe fn new(j: &Jeu, c: &Cas) -> Objets {
            let mut o = Objets {
                m: Box::new([0; 21]),
                g: Box::new([0; 21]),
                d: Box::new([0; 21]),
                pts: Box::new([0; 4]),
                exclus: Box::new([0; 13]),
            };
            let pts = o.pts.as_mut_ptr() as usize;
            j.vecteur(pts, &c.points, 0);
            o.m[0] = pts;
            for (h, k) in [(o.g.as_mut_ptr() as usize, 0), (o.d.as_mut_ptr() as usize, 1)] {
                *(h as *mut usize) = pts;
                j.vecteur(h + IDX, &c.idx[k], 0);
                j.vecteur(h + ARETES, &c.aretes[k], 0);
                j.vecteur(h + TRIANGLES, &c.triangles[k], 0);
            }
            let m = o.m.as_mut_ptr() as usize;
            j.vecteur(m + COUTURE, &c.couture, c.marge);
            j.vecteur(m + NOUVEAUX, &c.nouveaux, c.marge);
            let ex = o.exclus.as_mut_ptr() as usize;
            o.exclus[0] = ex + 40;
            o.exclus[1] = ex + 40;
            o.exclus[2] = ex + 40 + 64;
            o.exclus[4] = ex + 40;
            o
        }

        fn adresses(&mut self) -> (usize, usize, usize, usize) {
            (
                self.m.as_mut_ptr() as usize,
                self.g.as_mut_ptr() as usize,
                self.d.as_mut_ptr() as usize,
                self.exclus.as_mut_ptr() as usize,
            )
        }

        unsafe fn etat(&mut self, rendu: u8) -> Etat {
            let (m, g, d, ex) = self.adresses();
            Etat {
                rendu,
                idx: [contenu(g + IDX), contenu(d + IDX)],
                aretes: [contenu(g + ARETES), contenu(d + ARETES)],
                triangles: [contenu(g + TRIANGLES), contenu(d + TRIANGLES)],
                couture: contenu(m + COUTURE),
                nouveaux: contenu(m + NOUVEAUX),
                exclus: contenu(ex),
            }
        }

        unsafe fn liberer(mut self, j: &Jeu) {
            let (m, g, d, ex) = self.adresses();
            for h in [g, d] {
                for o in [IDX, ARETES, TRIANGLES] {
                    j.liberer(h + o);
                }
            }
            j.liberer(m + COUTURE);
            j.liberer(m + NOUVEAUX);
            j.liberer(self.pts.as_mut_ptr() as usize);
            if self.exclus[0] != ex + 40 {
                (j.free)(self.exclus[0]);
            }
        }
    }

    fn cas_demandes(defaut: usize) -> usize {
        let f = std::env::var("BGE_DELAUNAY_CAS").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(1);
        defaut * f.max(1)
    }

    /// Fusions tirées au hasard : originale, réécriture (mode on) et mode verifier, chacun sur ses
    /// propres objets ; tous les vecteurs et la valeur rendue doivent être identiques.
    unsafe fn fusions_au_hasard(j: &Jeu, n: usize) {
        let mut r = Hasard(0x9e37_79b9_7f4a_7c15);
        for cas in 0..n {
            let c = tirer(&mut r);
            let mut a = Objets::new(j, &c);
            let (m, g, d, ex) = a.adresses();
            let ra = original(m, g, d, ex) & 1;
            let ea = a.etat(ra);
            let mut b = Objets::new(j, &c);
            set_mode(Mode::On);
            let (m, g, d, ex) = b.adresses();
            let rb = hook_fermeture(m, g, d, ex);
            let eb = b.etat(rb);
            assert_eq!(ea, eb, "cas {cas} : la réécriture diffère de l'originale");
            let mut v = Objets::new(j, &c);
            set_mode(Mode::Verifier);
            let (m, g, d, ex) = v.adresses();
            let rv = hook_fermeture(m, g, d, ex) & 1;
            assert_eq!(ECARTS.load(Ordering::Relaxed), 0, "cas {cas} : écart signalé par le mode verifier");
            assert_eq!(ea, v.etat(rv), "cas {cas} : le mode verifier a changé le résultat");
            for o in [a, b, v] {
                o.liberer(j);
            }
        }
        assert_eq!(IGNORES.load(Ordering::Relaxed), 0, "cas valides refusés");
    }

    /// Une carte hexagonale au hasard (continents en disques, un peu de bruit), points des terres
    /// comme `1805c95b0` : triés (x, puis rangées paires d'abord, puis y), puis x -= y/2.
    fn carte(r: &mut Hasard) -> Vec<[i32; 2]> {
        let (w, h) = if r.piece(4) { (r.entre(60, 106), r.entre(40, 66)) } else { (r.entre(6, 60), r.entre(5, 40)) };
        let blocs: Vec<(i64, i64, i64)> =
            (0..r.entre(1, 8)).map(|_| (r.entre(0, w - 1), r.entre(0, h - 1), r.entre(2, (w.min(h) / 2).max(3)))).collect();
        let mut p = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let terre = blocs.iter().any(|&(cx, cy, ra)| (x - cx).pow(2) + (y - cy).pow(2) <= ra * ra);
                if terre != r.piece(12) {
                    p.push([x as i32, y as i32]);
                }
            }
        }
        if p.len() < 3 {
            p.extend([[0, 0], [1, 0], [0, 1]]);
            p.sort();
            p.dedup();
        }
        p.sort_by(|a, b| (a[0], a[1] & 1, a[1]).cmp(&(b[0], b[1] & 1, b[1])));
        for q in &mut p {
            q[0] -= q[1].div_euclid(2);
        }
        p
    }

    type Triangulation = (Vec<u32>, Vec<[u32; 2]>, Vec<[u32; 4]>);

    /// La triangulation entière ; rend la moitié finale (index, arêtes, triangles).
    unsafe fn trianguler(j: &Jeu, points: &[[i32; 2]]) -> Triangulation {
        let mut constructeur = Box::new([0usize; 16]);
        let c = constructeur.as_mut_ptr() as usize;
        j.vecteur(c + 8, points, 0);
        let initiales: unsafe extern "system" fn(usize) = std::mem::transmute(gc(0x1_805c_9960));
        let etape: unsafe extern "system" fn(usize, usize) = std::mem::transmute(gc(0x1_805c_b9c0));
        let detruire: unsafe extern "system" fn(usize) = std::mem::transmute(gc(0x1_805c_41b0));
        initiales(c);
        let mut tmp = Box::new([0usize; 4]);
        let t = tmp.as_mut_ptr() as usize;
        let moities = |v: usize| {
            let v = &*(v as *const VecJeu);
            (v.debut, (v.fin - v.debut) / 0xa8)
        };
        while moities(c + 0x28).1 > 1 {
            etape(c, t);
        }
        let (debut, n) = moities(t);
        for i in 0..n {
            detruire(debut + i * 0xa8);
        }
        j.liberer(t);
        let (debut, n) = moities(c + 0x28);
        assert_eq!(n, 1);
        let r = (contenu(debut + IDX), contenu(debut + ARETES), contenu(debut + TRIANGLES));
        detruire(debut);
        j.liberer(c + 0x28);
        j.liberer(c + 8);
        r
    }

    unsafe fn triangulations(j: &Jeu, n: usize) {
        let mut r = Hasard(0x2545_f491_4f6c_dd1d);
        let mut points_total = 0;
        let (mut ms_original, mut ms_rapide) = (0.0, 0.0);
        for cas in 0..n {
            let p = carte(&mut r);
            points_total += p.len();
            set_mode(Mode::Off);
            let t0 = win::qpc();
            let a = trianguler(j, &p);
            let t1 = win::qpc();
            set_mode(Mode::On);
            let b = trianguler(j, &p);
            let t2 = win::qpc();
            ms_original += win::qpc_ms(t1 - t0);
            ms_rapide += win::qpc_ms(t2 - t1);
            assert_eq!(a, b, "carte {cas} ({} points) : la réécriture diffère de l'originale", p.len());
            set_mode(Mode::Verifier);
            let v = trianguler(j, &p);
            assert_eq!(ECARTS.load(Ordering::Relaxed), 0, "carte {cas} : écart signalé par le mode verifier");
            assert_eq!(a, v, "carte {cas} : le mode verifier a changé le résultat");
            if p.len() > 3000 {
                eprintln!(
                    "carte {cas} : {} points, {} arêtes, originale {:.0} ms, réécriture {:.0} ms",
                    p.len(),
                    a.1.len(),
                    win::qpc_ms(t1 - t0),
                    win::qpc_ms(t2 - t1)
                );
            }
        }
        eprintln!("{n} triangulations, {points_total} points, originale {ms_original:.0} ms, réécriture {ms_rapide:.0} ms");
        assert_eq!(IGNORES.load(Ordering::Relaxed), 0, "cas valides refusés");
    }

    #[test]
    fn identique_a_l_originale() {
        unsafe {
            let Some((h, jeu)) = charger() else {
                eprintln!("absent, test sauté : {DLL}");
                return;
            };
            verifier_version(h).expect("version de l'originale");
            BASE.store(h, Ordering::Relaxed);
            let t = patch::install(h, &FERMETURE, hook_fermeture as *const () as usize).expect("détour");
            TRAMP_FERMETURE.store(t, Ordering::Relaxed);

            fusions_au_hasard(&jeu, cas_demandes(20_000));
            eprintln!(
                "fusions au hasard, couverture (exclusions, suppressions, aucun, deux côtés, gauche, droite) : {:?}",
                COUVERTURE.iter().map(|c| c.swap(0, Ordering::Relaxed)).collect::<Vec<_>>()
            );
            triangulations(&jeu, cas_demandes(40));
            eprintln!(
                "triangulations, couverture : {:?}",
                COUVERTURE.iter().map(|c| c.load(Ordering::Relaxed)).collect::<Vec<_>>()
            );
            patch::restore(h, &FERMETURE);
        }
    }

    #[test]
    fn orientation_et_mesure() {
        assert_eq!(orientation([0, 0], [1, 0], [0, 1]), 2);
        // Débordement en i32 modulaire, comme l'originale (imul 32 bits) : pas de panique.
        let _ = orientation([i32::MAX, i32::MIN], [i32::MIN, i32::MAX], [i32::MAX, i32::MAX]);
        unsafe extern "C" fn racine(x: f32) -> f32 {
            x.sqrt()
        }
        let v = unsafe { mesure(1, 0, 0, 1, racine) };
        let a = 1.0f32 * 0.5;
        assert_eq!(v, (0.5f32 * 0.0 + 1.0) * 3.0 * a / (a * 3.0 * a + 2.25f32).sqrt());
    }
}
