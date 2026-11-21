//! Relecture sur place : le client ne relit que les variables qui diffèrent de celles de l'hôte
//! (`relecture_fidele=on`, par défaut, le réglage de `relecture` dans le proxy GameCore). Jeux
//! 1.0.12.72 et 1.0.12.74, exes DX11 et DX12.
//!
//! Quand l'hôte envoie à un client une archive que celui-ci possède déjà, en données de création
//! (« full AutoArchive snapshot, already exists on client) Syncing... », par exemple avec
//! `reparation_synchro=create`), le gestionnaire du client (`1407f1630` en DX11 1.0.12.72) la relit
//! sur place par `140dbd170` : il la désinscrit du gestionnaire, relit son en-tête et *chaque*
//! variable, puis la réinscrit. Relire une variable qui a déjà la valeur de l'hôte n'est pas neutre :
//! ses octets restent les mêmes, mais l'IA du client décide ensuite autrement que celle de l'hôte
//! (des contrats d'IA d'un seul côté, puis un rechargement complet). Sur une petite carte avec des
//! IA, des archives d'IA renvoyées ainsi à chaque tour, alors que rien n'avait divergé, donnaient 16
//! rechargements complets en 20 tours ; 15 points de dégâts sur une unité d'IA, chez le client
//! seulement, avec `reparation_synchro=create`, en donnaient 2. Avec ce module, aucun dans les deux
//! cas, comme le jeu d'origine et `fulldelta`.
//!
//! Le chemin de delta d'origine (`140dbd3c0`) ne relit que les variables qui diffèrent et saute les
//! autres (vtable `+0x18`). Ce module fait de même sur place : l'appel du gestionnaire à `140dbd170`
//! est redirigé vers une copie de cette fonction qui, pour chaque variable, écrit sa valeur actuelle
//! (vtable `+0x20`) dans le flux de travail du gestionnaire (celui des empreintes, vtable `+0x40` du
//! gestionnaire) et la compare aux octets suivants des données reçues. Mêmes octets : la variable
//! est sautée, et le saut doit s'arrêter juste après eux (sinon retour en arrière et relecture).
//! Octets différents : relue, comme dans l'original. Désinscription, en-tête et réinscription sont
//! faits comme dans l'original, dont tous les octets sont vérifiés avant la pose (rien n'est posé
//! sinon).
//!
//! Après la réinscription, chaque variable relue lance sa DeltaDesyncCallback, comme après un delta
//! (`1407f2e10` : première entrée de la table de l'archive à l'indice de la variable, appelée
//! `f(objet, variable)`, tant que l'archive est encore inscrite sous sa clé). L'original n'en lance
//! aucune. Les types d'IA n'en ont pas ; un arbre de comportement en a une sur `m_definitions`, qui
//! refait l'édition de liens de l'arbre après sa relecture (voir `relecture`, proxy GameCore).
//!
//! Le client seulement : il n'a pas besoin que l'hôte ait ce proxy. Pas un correctif POL : il ne
//! change que la façon dont le client relit une archive que l'hôte lui renvoie. Il complète
//! `relecture` (proxy GameCore), qui rend exacte la relecture des variables qui diffèrent.

use crate::{exe, log, motifs, win};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// Gestionnaire du client, branche « already exists on client » : flux sur les données de
/// création, puis `call relecture(archive = [rdi+8], flux)`.
const SITE: &str = "33 D2 48 8D 4D ?? E8 ?? ?? ?? ?? 48 8B 4F 08 48 8D 55 ?? E8 ?? ?? ?? ?? 48 8D 4D ?? E8";
const SITE_CALL: usize = 0x13;

/// `140dbd170` (DX11 1.0.12.72), en entier : la copie en refait chaque pas, avec les mêmes champs.
/// Seul le rel32 de l'appel de l'en-tête (`+0x38`) est libre.
const RELECTURE: &str = "48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 48 83 EC 20 80 79 48 00 48 8B F2 48 8B F9 \
                         74 11 48 8B 49 50 48 8B D7 48 8B 01 FF 50 08 C6 47 48 00 48 8B D6 48 8D 4F 58 E8 ?? ?? ?? ?? \
                         48 8B 5F 08 48 3B 5F 10 74 1F 66 0F 1F 84 00 00 00 00 00 48 8B 0B 48 8B D6 48 8B 01 FF 50 08 \
                         48 83 C3 08 48 3B 5F 10 75 EA 48 8B 4F 50 48 85 C9 74 1B B8 FF FF FF FF 48 39 47 58 74 10 \
                         48 8B 01 48 8B D7 FF 10 84 C0 74 04 C6 47 48 01";
const HEADER_CALL: usize = 0x38;

// Archive : variables, DeltaDesyncCallbacks, inscription, gestionnaire, clé (début de l'en-tête),
// type.
const AR_VARS: usize = 0x08;
const AR_VARS_END: usize = 0x10;
const AR_CALLBACKS: usize = 0x20;
const AR_CALLBACKS_END: usize = 0x28;
const AR_REGISTERED: usize = 0x48;
const AR_MGR: usize = 0x50;
const AR_KEY: usize = 0x58;
const AR_TYPE: usize = 0x60;
const AR_TYPE_END: usize = 0x68;
/// Clé que l'original n'inscrit pas.
const NO_KEY: u64 = 0xffff_ffff;
// Flux reçu : tampon, position, taille.
const ST_BUF: usize = 0x08;
const ST_POS: usize = 0x10;
const ST_SIZE: usize = 0x18;
// Vtables : variable, gestionnaire, flux de travail (tampon en `+0x08`).
const VAR_READ: usize = 0x08;
const VAR_SKIP: usize = 0x18;
const VAR_WRITE: usize = 0x20;
const MGR_REGISTER: usize = 0x00;
const MGR_UNREGISTER: usize = 0x08;
const MGR_SCRATCH: usize = 0x40;
const SCRATCH_SIZE: usize = 0x10;
const SCRATCH_RESET: usize = 0x18;
const SCRATCH_BUF: usize = 0x08;
// Entrée de DeltaDesyncCallback : indice de la variable (u32), objet, fonction.
const CB_SIZE: usize = 0x18;
const CB_OBJ: usize = 0x08;
const CB_FN: usize = 0x10;
// Archives inscrites du gestionnaire : seaux, nombre de seaux (u32) ; nœud {clé, archive, suivant}.
const MGR_BUCKETS: usize = 0x18;
const MGR_BUCKET_COUNT: usize = 0x20;
const NODE_ARCHIVE: usize = 0x08;
const NODE_NEXT: usize = 0x10;

type Fn1 = unsafe extern "system-unwind" fn(usize) -> usize;
type Fn2 = unsafe extern "system-unwind" fn(usize, usize);
type Register = unsafe extern "system-unwind" fn(usize, usize) -> u8;

static HEADER: AtomicUsize = AtomicUsize::new(0);
static LOGS: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 200;

fn log_limited(msg: &str) {
    if LOGS.fetch_add(1, Ordering::Relaxed) < MAX_LOGS {
        log(msg);
    }
}

/// Le réglage `relecture_fidele` (dernière occurrence ; `on` par défaut).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("relecture_fidele=")).last().map(str::trim) != Some("off")
}

unsafe fn rd<T: Copy>(at: usize) -> T {
    exe::rd::<T>(at)
}

unsafe fn wr<T: Copy>(at: usize, v: T) {
    std::ptr::write_unaligned(at as *mut T, v)
}

/// La fonction de l'emplacement `slot` de la vtable de `obj`.
unsafe fn vfn<F: Copy>(obj: usize, slot: usize) -> F {
    std::mem::transmute_copy::<usize, F>(&rd::<usize>(rd::<usize>(obj) + slot))
}

/// Écriture complète de `var` dans le flux de travail du gestionnaire : (tampon, taille).
unsafe fn ecriture(mgr: usize, var: usize) -> Option<(usize, usize)> {
    if mgr == 0 {
        return None;
    }
    let s = vfn::<Fn1>(mgr, MGR_SCRATCH)(mgr);
    if s == 0 {
        return None;
    }
    vfn::<Fn2>(s, SCRATCH_RESET)(s, 0);
    vfn::<Fn2>(var, VAR_WRITE)(var, s);
    let n = vfn::<Fn1>(s, SCRATCH_SIZE)(s);
    let buf = rd::<usize>(s + SCRATCH_BUF);
    (buf != 0 && n != 0).then_some((buf, n))
}

/// Les octets suivants du flux reçu sont-ils `n` octets égaux à ceux de `buf` ?
unsafe fn memes_octets(stream: usize, buf: usize, n: usize) -> bool {
    let (data, pos, size) = (rd::<usize>(stream + ST_BUF), rd::<usize>(stream + ST_POS), rd::<usize>(stream + ST_SIZE));
    data != 0
        && pos <= size
        && n <= size - pos
        && std::slice::from_raw_parts(buf as *const u8, n) == std::slice::from_raw_parts((data + pos) as *const u8, n)
}

/// La copie de `140dbd170`, qui saute les variables déjà égales à celles de l'hôte. Rend les
/// indices des variables relues et le nombre de sautées.
unsafe fn relire(archive: usize, stream: usize) -> (Vec<u32>, u32) {
    if rd::<u8>(archive + AR_REGISTERED) != 0 {
        let mgr = rd::<usize>(archive + AR_MGR);
        vfn::<Fn2>(mgr, MGR_UNREGISTER)(mgr, archive);
        wr::<u8>(archive + AR_REGISTERED, 0);
    }
    let header: Fn2 = std::mem::transmute::<usize, Fn2>(HEADER.load(Ordering::Relaxed));
    header(archive + AR_KEY, stream);
    let mgr = rd::<usize>(archive + AR_MGR);
    let (mut relues, mut sautees) = (Vec::new(), 0);
    let mut p = rd::<usize>(archive + AR_VARS);
    let mut i = 0u32;
    while p != rd::<usize>(archive + AR_VARS_END) {
        let var = rd::<usize>(p);
        p += 8;
        i += 1;
        let pos = rd::<usize>(stream + ST_POS);
        if let Some((buf, n)) = ecriture(mgr, var) {
            if memes_octets(stream, buf, n) {
                vfn::<Fn2>(var, VAR_SKIP)(var, stream);
                if rd::<usize>(stream + ST_POS) == pos + n {
                    sautees += 1;
                    continue;
                }
                log_limited(&format!(
                    "in-place re-read: {}: the skip of a variable stopped {} byte(s) after it instead of {n}, read instead",
                    type_name(archive),
                    rd::<usize>(stream + ST_POS).wrapping_sub(pos)
                ));
                wr::<usize>(stream + ST_POS, pos);
            }
        }
        vfn::<Fn2>(var, VAR_READ)(var, stream);
        relues.push(i - 1);
    }
    let mgr = rd::<usize>(archive + AR_MGR);
    if mgr != 0 && rd::<u64>(archive + AR_KEY) != NO_KEY && vfn::<Register>(mgr, MGR_REGISTER)(mgr, archive) != 0 {
        wr::<u8>(archive + AR_REGISTERED, 1);
    }
    (relues, sautees)
}

/// L'archive est-elle inscrite sous `key` dans le gestionnaire `mgr` ?
unsafe fn inscrite(mgr: usize, key: u64, archive: usize) -> bool {
    let (buckets, n) = (rd::<usize>(mgr + MGR_BUCKETS), rd::<u32>(mgr + MGR_BUCKET_COUNT));
    if buckets == 0 || n == 0 {
        return false;
    }
    let mut node = rd::<usize>(buckets + ((key as u32) % n) as usize * 8);
    while node != 0 {
        if rd::<u64>(node) == key {
            return rd::<usize>(node + NODE_ARCHIVE) == archive;
        }
        node = rd::<usize>(node + NODE_NEXT);
    }
    false
}

/// La DeltaDesyncCallback de la variable `i` : (objet, fonction) de la première entrée à cet indice.
unsafe fn rappel(archive: usize, i: u32) -> Option<(usize, usize)> {
    let (b, e) = (rd::<usize>(archive + AR_CALLBACKS), rd::<usize>(archive + AR_CALLBACKS_END));
    if b == 0 || e <= b {
        return None;
    }
    (b..e).step_by(CB_SIZE).find(|&c| rd::<u32>(c) == i).map(|c| (rd::<usize>(c + CB_OBJ), rd::<usize>(c + CB_FN)))
}

/// Lance la DeltaDesyncCallback de chaque variable relue, comme le chemin de delta, tant que
/// l'archive est inscrite sous sa clé (une callback peut la retirer). Rend le nombre lancé.
unsafe fn rappeler(archive: usize, relues: &[u32]) -> u32 {
    let mut lancees = 0;
    for &i in relues {
        let Some((obj, f)) = rappel(archive, i).filter(|&(_, f)| f != 0) else { continue };
        let (mgr, key) = (rd::<usize>(archive + AR_MGR), rd::<u64>(archive + AR_KEY));
        if mgr == 0 || !inscrite(mgr, key, archive) {
            log_limited(&format!("in-place re-read: {}: not registered, its callbacks skipped", type_name(archive)));
            break;
        }
        let vars = rd::<usize>(archive + AR_VARS);
        let n = (rd::<usize>(archive + AR_VARS_END) - vars) / 8;
        if (i as usize) < n {
            std::mem::transmute::<usize, Fn2>(f)(obj, rd::<usize>(vars + i as usize * 8));
            lancees += 1;
        }
    }
    lancees
}

unsafe fn type_name(archive: usize) -> String {
    let (b, e) = (rd::<usize>(archive + AR_TYPE), rd::<usize>(archive + AR_TYPE_END));
    let t = if b == 0 || e <= b || e - b > 128 {
        String::from("?")
    } else {
        String::from_utf8_lossy(std::slice::from_raw_parts(b as *const u8, e - b)).trim_end_matches('\0').to_string()
    };
    format!("{t} {:x}", rd::<u64>(archive + AR_KEY))
}

unsafe extern "system-unwind" fn relecture_hook(archive: usize, stream: usize) {
    let (relues, sautees) = relire(archive, stream);
    if relues.is_empty() {
        return;
    }
    let lancees = rappeler(archive, &relues);
    log_limited(&format!(
        "in-place re-read: {}: {} variable(s) read again, {sautees} already the host's, {lancees} callback(s) run",
        type_name(archive),
        relues.len()
    ));
}

#[derive(Debug, PartialEq)]
pub struct Sites {
    pub call: usize,
    pub func: usize,
    pub header: usize,
}

/// Trouve l'appel du gestionnaire, vérifie la fonction appelée octet par octet et lit l'adresse de
/// la lecture de l'en-tête. `text_va` est l'adresse de `text[0]`.
pub fn locate(text: &[u8], text_va: usize) -> Result<Sites, String> {
    let hits = motifs::trouver(text, &motifs::motif(SITE));
    let [h] = hits[..] else { return Err(format!("{} in-place call sites instead of 1", hits.len())) };
    // Cible d'un `call rel32` à l'offset `o` de `text`, en offset de `text` s'il y tombe.
    let cible = |o: usize| -> Option<usize> {
        let t = o as i64 + 5 + i32::from_le_bytes(text[o + 1..o + 5].try_into().unwrap()) as i64;
        usize::try_from(t).ok().filter(|&t| t < text.len())
    };
    let call = h + SITE_CALL;
    let motif = motifs::motif(RELECTURE);
    let func = cible(call)
        .filter(|&f| f + motif.len() <= text.len() && motifs::trouver(&text[f..f + motif.len()], &motif) == [0])
        .ok_or("in-place re-read function: unexpected bytes (already redirected?)")?;
    let header = cible(func + HEADER_CALL).ok_or("header read outside .text")?;
    Ok(Sites { call: text_va + call, func: text_va + func, header: text_va + header })
}

/// Appelé par DllMain : aucun code de l'exe n'a encore tourné.
pub fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("in-place re-read: not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log("in-place re-read: every variable read again (relecture_fidele=off)");
        return;
    }
    unsafe {
        let module = win::GetModuleHandleW(std::ptr::null()) as usize;
        let Some((start, size)) = exe::text_section(module) else {
            log("in-place re-read: .text of the exe not found, not installed");
            return;
        };
        let text = std::slice::from_raw_parts(start as *const u8, size);
        let sites = match locate(text, start) {
            Ok(s) => s,
            Err(e) => {
                log(&format!("in-place re-read: unknown exe ({e}), not installed"));
                return;
            }
        };
        HEADER.store(sites.header, Ordering::Relaxed);
        if let Err(e) = motifs::rediriger_appel(sites.call, relecture_hook as Fn2 as usize) {
            log(&format!("in-place re-read: not installed ({e})"));
            return;
        }
        let ghidra = |a: usize| a - module + 0x1_4000_0000;
        log(&format!(
            "in-place re-read: only variables that differ from the host's are read again (call {:#x} -> {:#x})",
            ghidra(sites.call),
            ghidra(sites.func)
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("relecture_fidele=off\n"));
        assert!(enabled("relecture_fidele=off\nrelecture_fidele=on\n"));
    }

    // Faux objets du jeu, aux mêmes offsets. Une variable s'écrit `[n][n octets]`.

    #[repr(C)]
    struct Flux {
        vt: usize,
        buf: usize,
        pos: usize,
        size: usize,
    }

    #[repr(C)]
    struct Travail {
        vt: usize,
        buf: usize,
        data: Vec<u8>,
    }

    #[repr(C)]
    struct Gestionnaire {
        vt: usize,
        travail: *mut Travail,
        inscriptions: u32,
        desinscriptions: u32,
    }

    #[repr(C)]
    struct Variable {
        vt: usize,
        valeur: Vec<u8>,
        lectures: u32,
        /// Un saut qui s'arrête un octet trop loin.
        saut_faux: bool,
    }

    unsafe extern "system-unwind" fn lire(var: usize, s: usize) {
        let (v, f) = (&mut *(var as *mut Variable), &mut *(s as *mut Flux));
        let n = rd::<u8>(f.buf + f.pos) as usize;
        v.valeur = std::slice::from_raw_parts((f.buf + f.pos + 1) as *const u8, n).to_vec();
        v.lectures += 1;
        f.pos += 1 + n;
    }

    unsafe extern "system-unwind" fn sauter(var: usize, s: usize) {
        let (v, f) = (&*(var as *const Variable), &mut *(s as *mut Flux));
        f.pos += 1 + rd::<u8>(f.buf + f.pos) as usize + v.saut_faux as usize;
    }

    unsafe extern "system-unwind" fn ecrire(var: usize, s: usize) {
        let (v, t) = (&*(var as *const Variable), &mut *(s as *mut Travail));
        t.data.push(v.valeur.len() as u8);
        t.data.extend_from_slice(&v.valeur);
        t.buf = t.data.as_ptr() as usize;
    }

    unsafe extern "system-unwind" fn travail(mgr: usize) -> usize {
        (*(mgr as *const Gestionnaire)).travail as usize
    }

    unsafe extern "system-unwind" fn vider(s: usize, _: usize) {
        (*(s as *mut Travail)).data.clear();
    }

    unsafe extern "system-unwind" fn taille(s: usize) -> usize {
        (*(s as *const Travail)).data.len()
    }

    unsafe extern "system-unwind" fn inscrire(mgr: usize, _: usize) -> u8 {
        (*(mgr as *mut Gestionnaire)).inscriptions += 1;
        1
    }

    unsafe extern "system-unwind" fn desinscrire(mgr: usize, _: usize) {
        (*(mgr as *mut Gestionnaire)).desinscriptions += 1;
    }

    /// En-tête : la clé, 8 octets.
    unsafe extern "system-unwind" fn entete(cle: usize, s: usize) {
        let f = &mut *(s as *mut Flux);
        wr::<u64>(cle, rd::<u64>(f.buf + f.pos));
        f.pos += 8;
    }

    fn vtable(slots: &[(usize, usize)]) -> Vec<usize> {
        let mut vt = vec![0usize; 0x50 / 8];
        for &(at, f) in slots {
            vt[at / 8] = f;
        }
        vt
    }

    /// Égale : sautée. Différente : relue. Égale mais au saut faux : relue après retour en
    /// arrière. Désinscrite et réinscrite une fois, flux lu jusqu'au bout.
    #[test]
    fn only_differing_variables_are_read() {
        let vt_var = vtable(&[(VAR_READ, lire as Fn2 as usize), (VAR_SKIP, sauter as Fn2 as usize), (VAR_WRITE, ecrire as Fn2 as usize)]);
        let vt_travail = vtable(&[(SCRATCH_SIZE, taille as Fn1 as usize), (SCRATCH_RESET, vider as Fn2 as usize)]);
        let vt_mgr = vtable(&[
            (MGR_REGISTER, inscrire as Register as usize),
            (MGR_UNREGISTER, desinscrire as Fn2 as usize),
            (MGR_SCRATCH, travail as Fn1 as usize),
        ]);
        let mut trav = Travail { vt: vt_travail.as_ptr() as usize, buf: 0, data: Vec::new() };
        let mut mgr = Gestionnaire { vt: vt_mgr.as_ptr() as usize, travail: &mut trav, inscriptions: 0, desinscriptions: 0 };
        let var = |v: &[u8], saut_faux| Variable { vt: vt_var.as_ptr() as usize, valeur: v.to_vec(), lectures: 0, saut_faux };
        let mut vars = [var(b"aa", false), var(b"b", false), var(b"c", true)];
        let ptrs: Vec<usize> = vars.iter_mut().map(|v| v as *mut Variable as usize).collect();
        let type_name = b"AI::Test";
        let mut archive = [0u8; 0x80];
        let a = archive.as_mut_ptr() as usize;
        unsafe {
            wr(a + AR_VARS, ptrs.as_ptr() as usize);
            wr(a + AR_VARS_END, ptrs.as_ptr() as usize + 8 * ptrs.len());
            wr(a + AR_REGISTERED, 1u8);
            wr(a + AR_MGR, &mut mgr as *mut Gestionnaire as usize);
            wr(a + AR_TYPE, type_name.as_ptr() as usize);
            wr(a + AR_TYPE_END, type_name.as_ptr() as usize + type_name.len());
        }
        let mut donnees = 0x1234u64.to_le_bytes().to_vec();
        donnees.extend_from_slice(b"\x02aa\x03bbb\x01c");
        let mut flux = Flux { vt: 0, buf: donnees.as_ptr() as usize, pos: 0, size: donnees.len() };
        HEADER.store(entete as Fn2 as usize, Ordering::Relaxed);
        let (relues, sautees) = unsafe { relire(a, &mut flux as *mut Flux as usize) };
        assert_eq!((relues, sautees), (vec![1, 2], 1));
        assert_eq!(flux.pos, donnees.len());
        assert_eq!(vars[0].lectures, 0);
        assert_eq!((vars[1].lectures, vars[1].valeur.as_slice()), (1, &b"bbb"[..]));
        assert_eq!((vars[2].lectures, vars[2].valeur.as_slice()), (1, &b"c"[..]));
        assert_eq!((mgr.desinscriptions, mgr.inscriptions), (1, 1));
        assert_eq!(unsafe { (rd::<u8>(a + AR_REGISTERED), rd::<u64>(a + AR_KEY)) }, (1, 0x1234));
    }

    unsafe extern "system-unwind" fn noter(obj: usize, var: usize) {
        (*(obj as *mut Vec<usize>)).push(var);
    }

    /// Seules les variables relues lancent leur callback, `f(objet, variable)`, et seulement tant que
    /// l'archive est inscrite sous sa clé (deuxième nœud de son seau).
    #[test]
    fn callbacks_of_read_variables() {
        let mut vus: Vec<usize> = Vec::new();
        let obj = &mut vus as *mut Vec<usize> as usize;
        let vars = [0x100usize, 0x200, 0x300];
        let f = noter as Fn2 as usize;
        // Indice, objet, fonction : les variables 0 et 2, rien pour la 1.
        let table: [usize; 6] = [0, obj, f, 2, obj, f];
        let mut archive = [0u8; 0x80];
        let a = archive.as_mut_ptr() as usize;
        let key = 0x1235u64;
        let autre: [usize; 3] = [0x1231, 0x9999, 0];
        let mut noeud: [usize; 3] = [key as usize, a, 0];
        let mut seaux = [0usize; 4];
        let sx = seaux.as_mut_ptr() as usize;
        let mut mgr = [0u8; 0x30];
        let m = mgr.as_mut_ptr() as usize;
        unsafe {
            wr(a + AR_VARS, vars.as_ptr() as usize);
            wr(a + AR_VARS_END, vars.as_ptr() as usize + 8 * vars.len());
            wr(a + AR_CALLBACKS, table.as_ptr() as usize);
            wr(a + AR_CALLBACKS_END, table.as_ptr() as usize + 8 * table.len());
            wr(a + AR_MGR, m);
            wr(a + AR_KEY, key);
            noeud[1] = a;
            let mut autre = autre;
            autre[2] = noeud.as_mut_ptr() as usize;
            wr(sx + 8, autre.as_ptr() as usize);
            wr(m + MGR_BUCKETS, sx);
            wr(m + MGR_BUCKET_COUNT, 4u32);
            assert_eq!(rappeler(a, &[1, 2]), 1);
            assert_eq!(vus, [0x300]);
            wr(sx + 8, 0usize);
            assert_eq!(rappeler(a, &[0, 2]), 0);
            assert_eq!(vus, [0x300]);
        }
    }

    /// Une archive dont la clé est `NO_KEY` n'est pas réinscrite, comme dans l'original.
    #[test]
    fn no_key_not_registered() {
        let vt_mgr = vtable(&[(MGR_REGISTER, inscrire as Register as usize), (MGR_UNREGISTER, desinscrire as Fn2 as usize)]);
        let mut mgr = Gestionnaire { vt: vt_mgr.as_ptr() as usize, travail: std::ptr::null_mut(), inscriptions: 0, desinscriptions: 0 };
        let mut archive = [0u8; 0x80];
        let a = archive.as_mut_ptr() as usize;
        unsafe {
            wr(a + AR_MGR, &mut mgr as *mut Gestionnaire as usize);
            wr(a + AR_VARS, a);
            wr(a + AR_VARS_END, a);
        }
        let donnees = NO_KEY.to_le_bytes();
        let mut flux = Flux { vt: 0, buf: donnees.as_ptr() as usize, pos: 0, size: 8 };
        HEADER.store(entete as Fn2 as usize, Ordering::Relaxed);
        assert_eq!(unsafe { relire(a, &mut flux as *mut Flux as usize) }, (vec![], 0));
        assert_eq!((mgr.desinscriptions, mgr.inscriptions), (0, 0));
        assert_eq!(unsafe { rd::<u8>(a + AR_REGISTERED) }, 0);
    }

    /// `.text` d'un fichier PE : (octets, adresse Ghidra de son premier octet).
    fn text_of(file: &[u8]) -> (&[u8], usize) {
        let rd32 = |o: usize| u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
        let rd16 = |o: usize| u16::from_le_bytes(file[o..o + 2].try_into().unwrap()) as usize;
        let nt = rd32(0x3c);
        let sections = nt + 24 + rd16(nt + 20);
        let s = (0..rd16(nt + 6)).map(|i| sections + 40 * i).find(|&s| &file[s..s + 5] == b".text").unwrap();
        let (va, raw, rawsize) = (rd32(s + 12), rd32(s + 20), rd32(s + 16));
        (&file[raw..raw + rawsize], 0x1_4000_0000 + va)
    }

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";
    /// Appel, fonction et lecture de l'en-tête attendus, dans l'ordre de `crate::test_exe`
    /// (1.0.12.72 puis 1.0.12.74, DX12 puis DX11).
    const EXPECTED: [(&str, Sites); 4] = [
        ("CivilizationVI_DX12.exe", Sites { call: 0x1_407f_4b8b, func: 0x1_40dc_b510, header: 0x1_40dc_b5b0 }),
        ("CivilizationVI.exe", Sites { call: 0x1_407f_2c9b, func: 0x1_40db_d170, header: 0x1_40db_d210 }),
        ("CivilizationVI_DX12.exe", Sites { call: 0x1_407f_4bbb, func: 0x1_40dc_b310, header: 0x1_40dc_b3b0 }),
        ("CivilizationVI.exe", Sites { call: 0x1_407f_39ab, func: 0x1_40db_d1d0, header: 0x1_40db_d270 }),
    ];

    #[test]
    fn sites_in_installed_exes() {
        let mut seen = 0;
        for (i, (name, sites)) in EXPECTED.iter().enumerate() {
            let path = crate::test_exe(EXE_DIR, i, name);
            let Ok(file) = std::fs::read(&path) else {
                eprintln!("{path} missing, skipped");
                continue;
            };
            seen += 1;
            let (text, va) = text_of(&file);
            assert_eq!(&locate(text, va).unwrap_or_else(|e| panic!("{path}: {e}")), sites, "{path}");
        }
        if seen == 0 {
            eprintln!("no game exe found, test skipped");
        }
    }
}
