//! Cache des hachages d'archives de l'hôte au contrôle de synchro de fin de tour
//! (`cache_synchro=on`, par défaut). Voir `analyses/passage_de_tour_reseau/README.md`, piste 1.
//!
//! **Hôte seulement.** Pour chaque client, le gestionnaire de l'hôte (`1407f0b70` en DX12 .58, celui
//! de `sync_notes` et `reparation_synchro`) recalcule le hachage de chacune de ses archives
//! (`140dc5390` : le hachage de chaque variable, `FAutoVariable::calculateDataHash`, une méthode
//! `const`, puis FNV-1a) et le compare à celui que le client annonce. Avec dix clients, l'hôte refait
//! dix fois le même calcul sur des dizaines de milliers d'archives.
//!
//! **Ce qui est redirigé.** Le seul appel au hachage fait par ce gestionnaire, par un relais qui
//! passe aussi le hachage annoncé par le client (`[r14+8]`) et la clé de l'archive (`[rbx]`). Les
//! autres appels du hachage (le client qui construit son propre contrôle) ne sont pas touchés.
//!
//! **Quand une valeur gardée sert.** Elle est rendue seulement si :
//! - le proxy de GameCore compte les messages qu'il exécute (`executions`, exports
//!   `BgdMessagesGameCore` et `BgdSuiviMessages`) ;
//! - rien n'a changé depuis son calcul : même génération (voir plus bas), même archive (pointeur)
//!   sous la même clé ;
//! - elle est égale au hachage annoncé par ce client. Un écart est toujours recalculé : la
//!   réparation part d'une valeur fraîche, comme dans le jeu d'origine.
//!
//! **La génération** change, et le cache est vidé, à chaque :
//! - début d'un contrôle (`StartGameSyncCheck`) ;
//! - inscription ou désinscription d'une archive (création, destruction, chargement d'objets) ;
//! - message exécuté par GameCore, avant et après son exécution : un ordre appliqué pendant la
//!   barrière périme le cache, qui reprend ensuite depuis des valeurs fraîches ;
//! - création ou destruction d'un contexte de partie, pose ou retrait du proxy de GameCore.
//!
//! Ces trois fonctions du gestionnaire sont trouvées dans sa table virtuelle (`141049800` en DX12
//! .58 : +0x28, +0x30, +0x40), chacune par une chaîne qu'elle seule écrit. Leurs entrées pointent
//! vers un relais de trois instructions qui augmente un compteur, puis saute à la fonction du jeu,
//! sans toucher aux arguments.
//!
//! **Modes** (`rollover_proxy.cfg`) : `cache_synchro=on|verify|off`. `verify` recalcule même quand
//! une valeur gardée servirait, rend la valeur fraîche (comme le jeu d'origine) et note tout écart :
//! c'est la preuve, en partie réelle, que le cache rend les mêmes résultats. Une ligne de journal
//! par génération utilisée : valeurs réutilisées, calculées, écarts en `verify`, et ce qui l'a
//! close. `actif=off` ne pose rien.

use crate::pe::{find_pattern, pattern, Image};
use crate::{log, patch, win};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

/// `mov rcx,[rbx+8]` (l'archive de l'hôte) ; `call hachage` ; `mov esi,eax` ;
/// `cmp [r14+8],eax` (hachage annoncé par le client) ; `jnz` ; `test eax,eax` ; `jnz`.
const SIG_HASH: &str = "48 8B 4B 08 E8 ?? ?? ?? ?? 8B F0 41 39 46 08 75 ?? 85 C0 0F 85";
const SIG_HASH_CALL: usize = 4;
/// Le hachage d'archive : `call [rax+38h]` (hachage d'une variable) dans ses 0x80 premiers octets.
const HASH_MARK: [u8; 3] = [0xff, 0x50, 0x38];
/// Chaînes écrites par les trois fonctions du gestionnaire.
const REGISTER: &[u8] = b"Attempted to register an autoarchive without an ID.  Make sure you call SetID() with a unique ID before registering. @assign bolson\r\n";
const UNREGISTER: &[u8] = b"Attempted to unregister an autoarchive without an ID. @assign bolson";
const START: &[u8] = b"StartGameSyncCheck()\r\n";
/// Places dans la table virtuelle, depuis l'inscription.
const SLOT_UNREGISTER: u32 = 0x08;
const SLOT_START: u32 = 0x18;

/// Ce que la recherche a trouvé, en adresses relatives.
#[derive(Debug, PartialEq)]
pub struct Sites {
    pub hash_call: u32,
    pub hash: u32,
    pub handler: u32,
    /// Entrées de la table virtuelle : inscription, désinscription, début du contrôle.
    pub slots: [u32; 3],
}

pub fn locate(img: &Image) -> Result<Sites, String> {
    let text = img.section(".text").ok_or(".text introuvable")?;
    let code = img.data(&text);
    let hits = find_pattern(code, &pattern(SIG_HASH));
    let [h] = hits[..] else { return Err(format!("appel du hachage : {} motifs au lieu de 1", hits.len())) };
    let hash_call = text.rva + (h + SIG_HASH_CALL) as u32;
    let hash = img.rel32_target(hash_call).ok_or("appel du hachage illisible")?;
    // Le gestionnaire de l'hôte, reconnu comme le fait `reparation_synchro`.
    let va = img.base as usize + text.rva as usize;
    let handler = crate::reparation_synchro::locate(code, va)?.handler - img.base as usize;
    if !(handler..handler + 0x2000).contains(&(hash_call as usize)) {
        return Err(format!("appel du hachage {hash_call:#x} hors du gestionnaire {handler:#x}"));
    }
    let calls = img.rel32_sites(hash).iter().filter(|s| s.1 == 0xe8).count();
    if calls != 2 {
        return Err(format!("hachage {hash:#x} : {calls} appels au lieu de 2"));
    }
    if !img.at(hash, 0x80).is_some_and(|b| b.windows(3).any(|w| w == HASH_MARK)) {
        return Err(format!("hachage {hash:#x} : octets inattendus"));
    }
    let register = img.function_of_string(REGISTER)?;
    let unregister = img.function_of_string(UNREGISTER)?;
    let start = img.function_of_string(START)?;
    let tables: Vec<u32> = img
        .pointers_to(".rdata", register)
        .into_iter()
        .filter(|&v| img.pointer_at(v + SLOT_UNREGISTER) == Some(unregister) && img.pointer_at(v + SLOT_START) == Some(start))
        .collect();
    let [table] = tables[..] else { return Err(format!("table du gestionnaire : {} au lieu de 1", tables.len())) };
    Ok(Sites { hash_call, hash, handler: handler as u32, slots: [table, table + SLOT_UNREGISTER, table + SLOT_START] })
}

// ---------------------------------------------------------------------------------------------
// Générations

/// Débuts de contrôle, inscriptions et désinscriptions d'archives, messages exécutés par GameCore
/// (augmenté par le proxy de GameCore).
static ROUNDS: AtomicU64 = AtomicU64::new(0);
static ARCHIVES: AtomicU64 = AtomicU64::new(0);
static MESSAGES: AtomicU64 = AtomicU64::new(0);
/// Le proxy de GameCore compte les messages.
static TRACKED: AtomicBool = AtomicBool::new(false);

/// Export lu par le proxy de GameCore (`executions`) : le compteur des messages exécutés.
pub extern "system" fn messages_counter() -> *const AtomicU64 {
    &MESSAGES
}

/// Export appelé par le proxy de GameCore : 1 une fois ses détours posés, 0 avant son déchargement.
pub extern "system" fn messages_tracked(on: u32) {
    TRACKED.store(on != 0, Ordering::SeqCst);
    MESSAGES.fetch_add(1, Ordering::SeqCst);
}

type Generation = (u64, u64, u64);

fn generation() -> Generation {
    (ROUNDS.load(Ordering::SeqCst), ARCHIVES.load(Ordering::SeqCst), MESSAGES.load(Ordering::SeqCst))
}

// ---------------------------------------------------------------------------------------------
// Cache

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off,
    On,
    Verify,
}

/// `cache_synchro=on|verify|off`, la dernière ligne l'emporte ; `on` par défaut.
pub fn mode(cfg: &str) -> Mode {
    match cfg.lines().filter_map(|l| l.trim().strip_prefix("cache_synchro=")).last().map(str::trim) {
        Some("off") => Mode::Off,
        Some("verify") => Mode::Verify,
        _ => Mode::On,
    }
}

#[derive(Clone, Copy)]
struct Entry {
    archive: usize,
    hash: u32,
}

#[derive(Default, Debug, PartialEq)]
pub struct Stats {
    pub reused: u64,
    pub computed: u64,
    /// `verify` : valeurs gardées qui auraient servi, et celles qui différaient du recalcul.
    pub checked: u64,
    pub differed: u64,
}

#[derive(Default)]
pub struct Cache {
    generation: Generation,
    entries: HashMap<u64, Entry>,
    pub stats: Stats,
}

/// Ce qui a clos une génération, pour le journal.
pub fn closed_by(old: Generation, new: Generation) -> String {
    let mut why = Vec::new();
    if new.0 != old.0 {
        why.push("new sync check".to_string());
    }
    if new.1 != old.1 {
        why.push(format!("{} archive (un)registration(s)", new.1.wrapping_sub(old.1)));
    }
    if new.2 != old.2 {
        why.push(format!("{} GameCore message mark(s)", new.2.wrapping_sub(old.2)));
    }
    why.join(", ")
}

impl Cache {
    /// Passe à la génération `now` : vide le cache, et rend le bilan de la précédente si elle a
    /// servi.
    pub fn roll(&mut self, now: Generation) -> Option<(Stats, String)> {
        if self.generation == now {
            return None;
        }
        let old = std::mem::replace(&mut self.generation, now);
        self.entries.clear();
        let stats = std::mem::take(&mut self.stats);
        (stats.reused + stats.computed + stats.checked > 0).then(|| (stats, closed_by(old, now)))
    }

    /// La valeur gardée pour cette archive, si elle vaut le hachage annoncé par le client.
    pub fn get(&self, key: u64, archive: usize, client: u32) -> Option<u32> {
        self.entries.get(&key).filter(|e| e.archive == archive && e.hash == client).map(|e| e.hash)
    }

    /// Range un hachage calculé pendant la génération `during`, si elle est toujours en cours.
    pub fn put(&mut self, during: Generation, key: u64, archive: usize, hash: u32) {
        if self.generation == during {
            self.entries.insert(key, Entry { archive, hash });
        }
    }
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);
static HASH: AtomicUsize = AtomicUsize::new(0);
static MODE: AtomicU32 = AtomicU32::new(0);
static LOGS: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 2000;

fn log_limited(msg: &str) {
    let n = LOGS.fetch_add(1, Ordering::Relaxed);
    if n < MAX_LOGS {
        log(msg);
    } else if n == MAX_LOGS {
        log("sync hash cache: log limit reached");
    }
}

fn with_cache<R>(f: impl FnOnce(&mut Cache) -> R) -> R {
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Cache::default))
}

fn report(r: Option<(Stats, String)>) {
    if let Some((s, why)) = r {
        let verify = if MODE.load(Ordering::Relaxed) == Mode::Verify as u32 {
            format!(", verify: {} checked, {} differed", s.checked, s.differed)
        } else {
            String::new()
        };
        log_limited(&format!("sync hash cache: {} reused, {} computed{verify}; closed by {why}", s.reused, s.computed));
    }
}

type HashFn = unsafe extern "system-unwind" fn(usize) -> u32;

/// Remplace l'appel du gestionnaire de l'hôte au hachage d'une archive. `client` : le hachage
/// annoncé par le client ; `key` : la clé de l'archive.
unsafe extern "system-unwind" fn hash_hook(archive: usize, client: u32, key: u64) -> u32 {
    let original: HashFn = std::mem::transmute(HASH.load(Ordering::Relaxed));
    if !TRACKED.load(Ordering::SeqCst) || archive == 0 {
        return original(archive);
    }
    let verify = MODE.load(Ordering::Relaxed) == Mode::Verify as u32;
    // Lue avant le calcul : une exécution de message pendant celui-ci change la génération, et la
    // valeur n'est pas gardée.
    let now = generation();
    let (kept, closed) = with_cache(|c| {
        let closed = c.roll(now);
        let kept = c.get(key, archive, client);
        if kept.is_some() {
            if verify {
                c.stats.checked += 1;
            } else {
                c.stats.reused += 1;
            }
        }
        (kept, closed)
    });
    report(closed);
    if let (Some(h), false) = (kept, verify) {
        return h;
    }
    let fresh = original(archive);
    match kept {
        // Un message exécuté pendant le recalcul change légitimement le hachage : pas un écart.
        Some(h) if h != fresh && generation() == now => {
            with_cache(|c| c.stats.differed += 1);
            log_limited(&format!("sync hash cache: verify: archive {key:#x} kept {h:#x}, computed {fresh:#x}"));
        }
        Some(_) => {}
        None => with_cache(|c| {
            c.stats.computed += 1;
            c.put(now, key, archive, fresh);
        }),
    }
    fresh
}

// ---------------------------------------------------------------------------------------------
// Installation

/// Relais d'une entrée de la table virtuelle : `mov r11,compteur ; lock inc qword [r11] ;
/// jmp [rip+0] ; fonction`. r11 n'est pas un argument et l'appelé peut le changer.
fn counting_relay(counter: &'static AtomicU64, target: usize) -> Vec<u8> {
    let mut code = vec![0x49, 0xbb];
    code.extend_from_slice(&(counter as *const AtomicU64 as u64).to_le_bytes());
    code.extend_from_slice(&[0xf0, 0x49, 0xff, 0x03, 0xff, 0x25, 0, 0, 0, 0]);
    code.extend_from_slice(&(target as u64).to_le_bytes());
    code
}

/// Relais de l'appel du hachage : `mov edx,[r14+8] ; mov r8,[rbx] ; jmp [rip+0] ; hash_hook`.
fn hash_relay(hook: usize) -> Vec<u8> {
    let mut code = vec![0x41, 0x8b, 0x56, 0x08, 0x4c, 0x8b, 0x03, 0xff, 0x25, 0, 0, 0, 0];
    code.extend_from_slice(&(hook as u64).to_le_bytes());
    code
}

/// Appelé par DllMain, avant `sync_notes` et `reparation_synchro`.
pub unsafe fn install() {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("sync hash cache: not installed (actif=off)");
        return;
    }
    let m = mode(&cfg);
    if m == Mode::Off {
        log("sync hash cache: turned off (cache_synchro=off)");
        return;
    }
    let module = win::GetModuleHandleW(std::ptr::null()) as usize;
    let Some(img) = Image::mapped(module) else { return };
    let sites = match locate(&img) {
        Ok(s) => s,
        Err(e) => {
            log(&format!("sync hash cache: unknown exe ({e}), not installed"));
            return;
        }
    };
    let Some(page) = patch::alloc_near(module) else {
        log("sync hash cache: no memory within 2 GB of the exe, not installed");
        return;
    };
    let call = module + sites.hash_call as usize;
    let Some(rel) = patch::rel32(call, page) else {
        log("sync hash cache: relay out of range, not installed");
        return;
    };
    // Les entrées telles qu'elles sont maintenant : le relais y saute, un échec les remet.
    let slots = sites.slots.map(|r| module + r as usize);
    let targets = slots.map(|s| crate::exe::rd::<usize>(s));
    let counters: [&'static AtomicU64; 3] = [&ARCHIVES, &ARCHIVES, &ROUNDS];
    let code = hash_relay(hash_hook as *const () as usize);
    std::ptr::copy_nonoverlapping(code.as_ptr(), page as *mut u8, code.len());
    for k in 0..3 {
        let code = counting_relay(counters[k], targets[k]);
        std::ptr::copy_nonoverlapping(code.as_ptr(), (page + 0x20 * (k + 1)) as *mut u8, code.len());
    }
    MODE.store(m as u32, Ordering::Relaxed);
    HASH.store(module + sites.hash as usize, Ordering::Relaxed);
    for k in 0..3 {
        if !crate::exe::write_slot(slots[k], page + 0x20 * (k + 1)) {
            for j in 0..k {
                crate::exe::write_slot(slots[j], targets[j]);
            }
            log("sync hash cache: manager table not writable, not installed");
            return;
        }
    }
    patch::write_rel32(call, rel);
    let ghidra = |r: u32| r as u64 + 0x1_4000_0000;
    log(&format!(
        "sync hash cache: installed, mode {m:?} (hash call {:#x} -> {:#x}, handler {:#x}, manager register slot {:#x})",
        ghidra(sites.hash_call),
        ghidra(sites.hash),
        ghidra(sites.handler),
        ghidra(sites.slots[0])
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings() {
        assert_eq!(mode(""), Mode::On);
        assert_eq!(mode("cache_synchro=off\r\n"), Mode::Off);
        assert_eq!(mode("cache_synchro=verify"), Mode::Verify);
        assert_eq!(mode("cache_synchro=off\ncache_synchro=on"), Mode::On, "la dernière ligne l'emporte");
    }

    #[test]
    fn kept_only_for_the_same_generation_archive_and_client_hash() {
        let mut c = Cache::default();
        let g = (1, 0, 0);
        assert!(c.roll(g).is_none(), "rien n'a servi");
        c.put(g, 7, 0x1000, 0xabc);
        assert_eq!(c.get(7, 0x1000, 0xabc), Some(0xabc));
        assert_eq!(c.get(7, 0x1000, 0xabd), None, "autre hachage annoncé : recalculé");
        assert_eq!(c.get(7, 0x2000, 0xabc), None, "autre archive sous la même clé");
        c.put((1, 0, 1), 8, 0x1000, 0x1);
        assert_eq!(c.get(8, 0x1000, 0x1), None, "calculé pendant une autre génération : pas gardé");
        c.stats.reused = 1;
        let (s, why) = c.roll((1, 0, 2)).unwrap();
        assert_eq!(s.reused, 1);
        assert_eq!(why, "2 GameCore message mark(s)");
        assert_eq!(c.get(7, 0x1000, 0xabc), None, "génération nouvelle : vidé");
    }

    #[test]
    fn what_closed_a_generation() {
        assert_eq!(closed_by((1, 5, 9), (2, 7, 9)), "new sync check, 2 archive (un)registration(s)");
    }

    #[test]
    fn relays() {
        let r = counting_relay(&ROUNDS, 0x1122_3344_5566_7788);
        assert_eq!(r.len(), 28);
        assert_eq!(&r[10..20], &[0xf0, 0x49, 0xff, 0x03, 0xff, 0x25, 0, 0, 0, 0]);
        assert_eq!(&r[20..], &0x1122_3344_5566_7788u64.to_le_bytes());
        assert_eq!(hash_relay(0).len(), 21);
    }

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";
    /// (appel du hachage, hachage, table du gestionnaire) attendus, dans l'ordre de `crate::test_exe`
    /// ; `None` : pas encore relevés (1.0.12.72 et 1.0.12.74, à lire dans le journal du test).
    const EXPECTED: [(&str, Option<(u32, u32, u32)>); 6] = [
        ("CivilizationVI_DX12.exe", None),
        ("CivilizationVI.exe", None),
        ("CivilizationVI_DX12.exe", None),
        ("CivilizationVI.exe", None),
        ("CivilizationVI_DX12.exe", Some((0x7f_1262, 0xdc_5390, 0x104_9800))),
        ("CivilizationVI.exe", Some((0x7f_0d22, 0xdb_6e30, 0x103_92d0))),
    ];

    #[test]
    fn sites_in_known_exes() {
        let mut seen = 0;
        for (i, &(name, expected)) in EXPECTED.iter().enumerate() {
            let path = crate::test_exe(EXE_DIR, i, name);
            let Ok(file) = std::fs::read(&path) else {
                eprintln!("{path} missing, skipped");
                continue;
            };
            seen += 1;
            let img = Image::file(&file).unwrap();
            let s = locate(&img).unwrap_or_else(|e| panic!("{path}: {e}"));
            eprintln!("{path}: {s:x?}");
            if let Some((call, hash, table)) = expected {
                assert_eq!((s.hash_call, s.hash), (call, hash), "{path}");
                if table != 0 {
                    assert_eq!(s.slots[0], table + 0x28, "{path}");
                }
            }
        }
        if seen == 0 {
            eprintln!("no game exe found, test skipped");
        }
    }
}
