//! Chargement des assets : test d'existence des fichiers SHARED_DATA servi par un listage des
//! dossiers (`cache_assets=on`, par défaut).
//!
//! Ce que fait le jeu (`analyses/chargement/README.md`, « 2. Les assets » ; exe DX12 du 2026-11-05,
//! projet `civ6exe1105`) : pour chaque donnée partagée demandée, `BLP_SharedDataCache` crée une tâche
//! (`SharedDataLoad` `1407786d0`, `SharedDataUpdateOwnership` `14077a630`, et une troisième variante
//! `140777b10`). Chaque tâche parcourt **toutes** les racines SHARED_DATA (environ 25 dans
//! l'installation, 30 dans le Workshop), compose `racine\nom`, et teste son existence par
//! `PathFileExistsW` (`140996ae0`, enveloppe qui rend un booléen). Le fichier n'existe que dans une
//! ou deux racines : presque tous les tests portent sur des fichiers absents. Dans la capture du
//! 2026-11-16, 8 177 des 8 705 échantillons de la tâche de propriété sont dans `PathFileExistsW`.
//!
//! Le détour : les trois `call` de l'enveloppe dans ces tâches (et seulement eux) appellent
//! `existe`, qui répond depuis le listage du dossier (`FindFirstFileExW`, grands lots, une fois par
//! dossier, valable `VALIDITE`). Même réponse que `PathFileExistsW` : vrai pour un fichier ou un
//! dossier, comparaison sans casse. Les chemins non ASCII, avec joker, ou dont le dossier ne se
//! liste pas passent par l'enveloppe d'origine. Les attributs (`GetFileAttributesExW`) ne sont
//! demandés que pour les fichiers présents : inchangés.
//!
//! Un fichier ajouté dans une racine pendant la validité du listage ne serait vu qu'au listage
//! suivant : les racines SHARED_DATA ne changent pas pendant une session (sauf rechargement à chaud
//! des outils de développement, hors jeu normal). Purement local, sans effet sur la synchro.

use crate::{log, motifs, win};
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Durée de validité d'un listage : couvre un chargement, relisté au suivant.
const VALIDITE: Duration = Duration::from_secs(20);

/// Tâche de propriété : `lea rcx,[rbp-70h] ; call compose ; mov rcx,[rbp-70h] ; call existe ;
/// test al,al ; je ; mov rcx,[rbp-70h] ; lea rdx,[rsp+68h] ; call attributs`. Appel en +13.
const SITE_PROPRIETE: &str = "48 8D 4D 90 E8 ?? ?? ?? ?? 48 8B 4D 90 E8 ?? ?? ?? ?? 84 C0 0F 84 ?? ?? ?? ?? 48 8B 4D 90 48 8D 54 24 68 E8";
/// Tâches de chargement (deux variantes) : la même chose sur `[rbp+0B0h]`. Appel en +19.
const SITE_CHARGEMENT: &str = "48 8D 8D B0 00 00 00 E8 ?? ?? ?? ?? 48 8B 8D B0 00 00 00 E8 ?? ?? ?? ?? 84 C0 0F 84 ?? ?? ?? ?? 48 8B 8D B0 00 00 00 48 8D 54 24 70 E8";
/// L'enveloppe : `sub rsp,28h ; call [PathFileExistsW] ; test eax,eax ; setne al ; add rsp,28h ; ret`.
const ENVELOPPE: &str = "48 83 EC 28 FF 15 ?? ?? ?? ?? 85 C0 0F 95 C0 48 83 C4 28 C3";

static ORIGINE: AtomicUsize = AtomicUsize::new(0);
static SERVIS: AtomicU64 = AtomicU64::new(0);
static RENVOYES: AtomicU64 = AtomicU64::new(0);
static LISTAGES: AtomicU64 = AtomicU64::new(0);

#[repr(C)]
struct FindData {
    attributes: u32,
    creation: [u32; 2],
    access: [u32; 2],
    write: [u32; 2],
    size_high: u32,
    size_low: u32,
    reserved0: u32,
    reserved1: u32,
    name: [u16; 260],
    alt: [u16; 14],
    // Champs obsolètes que Windows peut écrire selon la version : marge.
    _marge: [u32; 4],
}

#[link(name = "kernel32")]
extern "system" {
    fn FindFirstFileExW(name: *const u16, level: u32, data: *mut FindData, op: u32, filter: *mut c_void, flags: u32) -> isize;
    fn FindNextFileW(h: isize, data: *mut FindData) -> i32;
    fn FindClose(h: isize) -> i32;
}

const FIND_EX_INFO_BASIC: u32 = 1;
const FIND_FIRST_EX_LARGE_FETCH: u32 = 2;
const INVALID_HANDLE: isize = -1;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_PATH_NOT_FOUND: u32 = 3;
const ERROR_NO_MORE_FILES: u32 = 18;

struct Dossier {
    liste_le: Instant,
    /// `None` : le dossier ne se liste pas (droits, erreur inattendue), on renvoie au système.
    noms: Option<HashSet<Vec<u16>>>,
}

static DOSSIERS: RwLock<Option<HashMap<Vec<u16>, Arc<Dossier>>>> = RwLock::new(None);

fn minuscule(c: u16) -> u16 {
    if (b'A' as u16..=b'Z' as u16).contains(&c) {
        c + 32
    } else {
        c
    }
}

/// Liste le dossier (chemin sans séparateur final), noms en minuscules ASCII.
unsafe fn lister(dossier: &[u16]) -> Option<HashSet<Vec<u16>>> {
    let mut motif: Vec<u16> = dossier.to_vec();
    motif.extend_from_slice(&[b'\\' as u16, b'*' as u16, 0]);
    let mut d: FindData = std::mem::zeroed();
    let h = FindFirstFileExW(motif.as_ptr(), FIND_EX_INFO_BASIC, &mut d, 0, std::ptr::null_mut(), FIND_FIRST_EX_LARGE_FETCH);
    LISTAGES.fetch_add(1, Ordering::Relaxed);
    if h == INVALID_HANDLE {
        return match win::GetLastError() {
            // Dossier absent : rien n'y existe, comme le dirait PathFileExistsW.
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Some(HashSet::new()),
            _ => None,
        };
    }
    let mut noms = HashSet::new();
    loop {
        let n = d.name.iter().position(|&c| c == 0).unwrap_or(d.name.len());
        let nom = &d.name[..n];
        if nom != [b'.' as u16] && nom != [b'.' as u16, b'.' as u16] {
            if nom.iter().any(|&c| c >= 0x80) {
                // Un nom non ASCII : la comparaison sans casse n'est pas la nôtre, on renonce.
                FindClose(h);
                return None;
            }
            noms.insert(nom.iter().map(|&c| minuscule(c)).collect());
        }
        if FindNextFileW(h, &mut d) == 0 {
            let e = win::GetLastError();
            FindClose(h);
            return (e == ERROR_NO_MORE_FILES).then_some(noms);
        }
    }
}

unsafe fn dossier(cle: &[u16]) -> Arc<Dossier> {
    if let Some(m) = DOSSIERS.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        if let Some(d) = m.get(cle) {
            if d.liste_le.elapsed() < VALIDITE {
                return d.clone();
            }
        }
    }
    let mut g = DOSSIERS.write().unwrap_or_else(|e| e.into_inner());
    let m = g.get_or_insert_with(HashMap::new);
    if let Some(d) = m.get(cle) {
        if d.liste_le.elapsed() < VALIDITE {
            return d.clone();
        }
    }
    bilan();
    let d = Arc::new(Dossier { liste_le: Instant::now(), noms: lister(cle) });
    m.insert(cle.to_vec(), d.clone());
    d
}

/// Réponse depuis le listage, ou `None` si le chemin doit passer par le système.
unsafe fn depuis_listage(chemin: *const u16) -> Option<bool> {
    if chemin.is_null() {
        return None;
    }
    let mut n = 0;
    while *chemin.add(n) != 0 {
        n += 1;
        if n > 1024 {
            return None;
        }
    }
    let p = std::slice::from_raw_parts(chemin, n);
    if p.iter().any(|&c| c >= 0x80 || c == b'*' as u16 || c == b'?' as u16) {
        return None;
    }
    let sep = p.iter().rposition(|&c| c == b'\\' as u16 || c == b'/' as u16)?;
    let (dir, nom) = (&p[..sep], &p[sep + 1..]);
    if dir.is_empty() || nom.is_empty() || nom.iter().all(|&c| c == b'.' as u16) {
        return None;
    }
    // Un nom finissant par un point ou une espace : Windows les retire, pas nous.
    if matches!(nom.last(), Some(&c) if c == b'.' as u16 || c == b' ' as u16) {
        return None;
    }
    let cle: Vec<u16> = dir.iter().map(|&c| minuscule(c)).collect();
    let d = dossier(&cle);
    let noms = d.noms.as_ref()?;
    let nom: Vec<u16> = nom.iter().map(|&c| minuscule(c)).collect();
    Some(noms.contains(&nom))
}

unsafe extern "C" fn existe(chemin: *const u16) -> bool {
    if let Some(r) = depuis_listage(chemin) {
        SERVIS.fetch_add(1, Ordering::Relaxed);
        return r;
    }
    RENVOYES.fetch_add(1, Ordering::Relaxed);
    let f: unsafe extern "C" fn(*const u16) -> bool = std::mem::transmute(ORIGINE.load(Ordering::Relaxed));
    f(chemin)
}

static DERNIER_BILAN: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);

/// Au premier listage d'une nouvelle série (un chargement), le bilan cumulé depuis le lancement.
fn bilan() {
    let mut g = DERNIER_BILAN.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_some_and(|t| t.elapsed() < VALIDITE) {
        return;
    }
    *g = Some(Instant::now());
    let (s, r, l) = (SERVIS.load(Ordering::Relaxed), RENVOYES.load(Ordering::Relaxed), LISTAGES.load(Ordering::Relaxed));
    if s + r > 0 {
        log(&format!("assets: since launch, {s} existence tests served from listings, {r} passed to the system, {l} listings"));
    }
}

/// Le réglage `cache_assets` (dernière occurrence ; `on` par défaut).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("cache_assets=")).last().map(str::trim) != Some("off")
}

pub fn install(exe: usize) {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("assets: not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log("assets: SHARED_DATA listing cache off (cache_assets=off)");
        return;
    }
    unsafe {
        let mut sites = Vec::new();
        for at in motifs::chercher(exe, &motifs::motif(SITE_PROPRIETE)) {
            sites.push(at + 13);
        }
        for at in motifs::chercher(exe, &motifs::motif(SITE_CHARGEMENT)) {
            sites.push(at + 19);
        }
        if sites.is_empty() || sites.len() > 3 {
            log(&format!("assets: not installed, {} call sites found instead of 1 to 3", sites.len()));
            return;
        }
        let cible = motifs::cible_rel32(sites[0]);
        if sites.iter().any(|&s| motifs::cible_rel32(s) != cible) || !motifs::correspond(cible, &motifs::motif(ENVELOPPE)) {
            log("assets: not installed, the call sites do not call the PathFileExistsW wrapper");
            return;
        }
        ORIGINE.store(cible, Ordering::Relaxed);
        for &s in &sites {
            if let Err(e) = motifs::rediriger_appel(s, existe as *const () as usize) {
                log(&format!("assets: call site {s:#x} not redirected: {e}"));
            }
        }
        log(&format!("assets: SHARED_DATA existence tests served from directory listings ({} call sites)", sites.len()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglage() {
        assert!(enabled(""));
        assert!(!enabled("cache_assets=off\n"));
        assert!(enabled("cache_assets=off\ncache_assets=on\n"));
    }

    #[test]
    fn listage_et_existence() {
        let dir = std::env::temp_dir().join(format!("bge_assets_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Sous")).unwrap();
        std::fs::write(dir.join("Leader_Foo.BLP"), b"x").unwrap();
        let w = |p: std::path::PathBuf| {
            let mut v: Vec<u16> = p.to_string_lossy().encode_utf16().collect();
            v.push(0);
            v
        };
        unsafe {
            assert_eq!(depuis_listage(w(dir.join("leader_foo.blp")).as_ptr()), Some(true));
            assert_eq!(depuis_listage(w(dir.join("LEADER_FOO.BLP")).as_ptr()), Some(true));
            assert_eq!(depuis_listage(w(dir.join("sous")).as_ptr()), Some(true));
            assert_eq!(depuis_listage(w(dir.join("absent.blp")).as_ptr()), Some(false));
            assert_eq!(depuis_listage(w(dir.join("nul_part").join("x.blp")).as_ptr()), Some(false));
            assert_eq!(depuis_listage(w(dir.join("é.blp")).as_ptr()), None);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
