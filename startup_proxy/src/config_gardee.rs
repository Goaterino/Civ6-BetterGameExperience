//! Étape de configuration gardée quand on recharge une partie depuis le jeu et que rien n'a changé
//! (`config_gardee=on`, par défaut). Complète `bdd`, qui fait de même pour la base de gameplay à
//! l'étape suivante.
//!
//! Dans les jeux 1.0.12.72 et 1.0.12.74 (exes DX11 et DX12), chaque chargement lance l'une après
//! l'autre les deux étapes de configuration du cadre de mods, depuis l'automate de chargement
//! (`140083c80` en DX11 1.0.12.72) :
//!
//! - l'étape de configuration (`140246e60` en DX11) reconstruit la base de configuration, remet la
//!   base de localisation aux textes du FrontEnd (menus, salon) et importe les fichiers implicites
//!   du FrontEnd (1,35 s d'un rechargement de 13 s sur une petite carte à deux joueurs) ;
//! - l'étape de gameplay (`UpdateContent`, détournée par `bdd`) recharge la DLL de GameCore, remet
//!   à leur origine les bases de localisation et de gameplay, puis réapplique les composants des
//!   mods.
//!
//! Chaque étape a un chemin « matches target configuration », pris quand les actions des mods sont
//! les mêmes que la fois précédente et que la base de localisation n'a pas changé depuis le dernier
//! passage de l'étape (compteur de modifications gardé dans l'objet du cadre de mods, en `+0x238`
//! pour l'étape de configuration et en `+0x23c` pour l'étape de gameplay). L'étape de gameplay
//! écrit -1 en `+0x238` en finissant : un chargement depuis une partie reconstruit donc toujours la
//! base de configuration, même quand rien n'a changé.
//!
//! Quand la dernière étape de gameplay a réussi et que ni la base de localisation ni celle de
//! gameplay n'ont été écrites depuis sa fin, le module met en `+0x238` le compteur de la
//! localisation avant l'étape de configuration, qui prend alors son propre chemin « matches ». Il
//! écrit ensuite -1 en `+0x23c` : l'étape de gameplay prend son chemin complet, comme avant (DLL de
//! GameCore déchargée et rechargée, le GameCore neuf de `chargeur` ; localisation remise et textes
//! des mods rejoués, par `bdd` et `localisation` ; base de gameplay gardée ou reconstruite selon la
//! règle de `bdd`). Après elle, `+0x238` vaut de nouveau -1, comme le laisse le jeu : le FrontEnd
//! reconstruit son contenu quand le joueur quitte la partie.
//!
//! Ce qui change : entre les deux étapes, la base de localisation contient les textes de la partie
//! au lieu de ceux du FrontEnd, et la base de configuration est celle que la dernière configuration
//! a construite pour les mêmes actions des mods. Rien ne les lit à ce moment-là. Comme rien n'a
//! remis la base de localisation à son origine, `bdd` la laisse ensuite intacte elle aussi quand
//! elle est exactement telle que l'a laissée la dernière étape de gameplay (même connexion, même
//! compteur de modifications, mêmes actions des mods) : ni `ROLLBACK TO Origin`, ni rejeu des
//! textes des mods, ni UpdateText, et le jeu saute la mise en cache des textes puisque la base n'a
//! pas changé.

use crate::{exe, log, patch, win};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Automate de chargement : `test al,al ; jz` après l'appel au cadre de mods, puis
/// `lea rdx,[rbp+..] ; mov rcx,rbx ; call` vers l'étape de configuration.
const SIG_CONFIG: &str = "45 33 C0 48 8B CB 48 8B D6 E8 ?? ?? ?? ?? 84 C0 0F 84 ?? ?? ?? ?? 48 8D 95 ?? ?? ?? ?? 48 8B CB E8 ?? ?? ?? ??";
const SIG_CONFIG_CALL: usize = 32;
/// Automate de chargement : `mov rcx,rbx ; lea rdx,[rbp+..] ; mov [rbp+..],rax ; call` vers l'étape
/// de gameplay, puis `test byte [rbp+..],1`.
const SIG_GAMEPLAY: &str = "48 8B CB 48 8D 95 ?? ?? ?? ?? 48 89 85 ?? ?? ?? ?? E8 ?? ?? ?? ?? F6 85 ?? ?? ?? ?? 01";
const SIG_GAMEPLAY_CALL: usize = 17;
/// Étape de gameplay : `mov dword [r14+238h], -1` (compteur de l'étape de configuration invalidé).
const MARK_238: &[u8] = &[0x41, 0xc7, 0x86, 0x38, 0x02, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff];
/// Étape de gameplay : `cmp [r14+23Ch], eax` et `mov [r14+23Ch], eax` (son propre compteur).
const MARK_23C: [&[u8]; 2] = [&[0x41, 0x39, 0x86, 0x3c, 0x02, 0x00, 0x00], &[0x41, 0x89, 0x86, 0x3c, 0x02, 0x00, 0x00]];
/// Étape de configuration : `mov rcx,[rip+app] ; call base_loc ; mov [rbp+..],rax ; test rax,rax ;
/// jz ; mov rcx,rax ; call connexion ; mov rcx,rax ; mov rdx,[rax] ; call [rdx+30h] ;
/// cmp [rbx+238h],eax` (`base_loc` et `connexion` : accesseurs de la base de localisation et de sa
/// connexion). C'est ainsi que l'étape lit le compteur de modifications de la base de localisation.
const SIG_LOC_COUNTER: &str = "48 8B 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? 48 89 45 ?? 48 85 C0 74 ?? 48 8B C8 E8 ?? ?? ?? ?? 48 8B C8 48 8B 10 FF 52 30 39 83 38 02 00 00";
/// Ligne de journal de l'étape de gameplay, suivie des appels [journal, accesseur de la base de
/// gameplay, ...].
const T_REBUILD: &[u8] = b"Rebuilding gameplay database.\0";

/// Champs de l'objet du cadre de mods.
const FW_CONFIG_COUNTER: usize = 0x238;
const FW_GAMEPLAY_COUNTER: usize = 0x23c;
const INVALID: u32 = 0xffff_ffff;

type ConfigFn = unsafe extern "system-unwind" fn(usize, usize) -> usize;
type GameplayFn = unsafe extern "system-unwind" fn(usize, usize, usize) -> usize;
type GetterFn = unsafe extern "system" fn(usize) -> usize;

static ORIG_CONFIG: AtomicUsize = AtomicUsize::new(0);
static ORIG_GAMEPLAY: AtomicUsize = AtomicUsize::new(0);
static APP: AtomicUsize = AtomicUsize::new(0);
static LOC_DB_FN: AtomicUsize = AtomicUsize::new(0);
static GAMEPLAY_DB_FN: AtomicUsize = AtomicUsize::new(0);
static CONN_FN: AtomicUsize = AtomicUsize::new(0);
/// Ce chargement a prérempli le compteur de l'étape de configuration.
static PRESET: AtomicBool = AtomicBool::new(false);
/// Compteur de la localisation au début de l'étape de configuration de ce chargement.
static LOC_BEFORE: Mutex<Option<u32>> = Mutex::new(None);
/// Compteurs de la localisation et de la base de gameplay juste après la dernière étape de gameplay
/// réussie.
static END: Mutex<Option<(u32, u32)>> = Mutex::new(None);
static STARTED: Mutex<Option<Instant>> = Mutex::new(None);
/// Module posé : `bdd` peut alors laisser telle quelle une base de localisation inchangée.
static ACTIF: AtomicBool = AtomicBool::new(false);

/// L'étape de configuration peut-elle être gardée aux rechargements (module posé) ?
pub fn actif() -> bool {
    ACTIF.load(Ordering::Relaxed)
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Le réglage `config_gardee` (dernière occurrence ; `on` par défaut).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("config_gardee=")).last().map(str::trim) != Some("off")
}

/// Le compteur de modifications d'une base, lu comme le lisent les étapes de configuration.
unsafe fn counter(db_getter: &AtomicUsize) -> Option<u32> {
    let (app_at, getter, conn_getter) = (APP.load(Ordering::Relaxed), db_getter.load(Ordering::Relaxed), CONN_FN.load(Ordering::Relaxed));
    if app_at == 0 || getter == 0 || conn_getter == 0 {
        return None;
    }
    let app: usize = exe::rd(app_at);
    if app == 0 {
        return None;
    }
    let db = std::mem::transmute::<usize, GetterFn>(getter)(app);
    if db == 0 {
        return None;
    }
    let conn = std::mem::transmute::<usize, GetterFn>(conn_getter)(db);
    if conn == 0 {
        return None;
    }
    let vtbl: usize = exe::rd(conn);
    let f: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(exe::rd::<usize>(vtbl + 0x30));
    Some(f(conn))
}

unsafe fn loc_counter() -> Option<u32> {
    counter(&LOC_DB_FN)
}

unsafe fn gameplay_counter() -> Option<u32> {
    counter(&GAMEPLAY_DB_FN)
}

fn hex(v: Option<u32>) -> String {
    v.map_or("?".into(), |v| format!("{v:#x}"))
}

/// Ce chargement peut-il garder l'étape de configuration ? Oui si la dernière étape de gameplay a
/// réussi (son compteur est valide) et qu'aucune des deux bases n'a été écrite depuis sa fin.
pub fn keep(gameplay_step_counter: u32, now: (Option<u32>, Option<u32>), end: Option<(u32, u32)>) -> bool {
    match (now, end) {
        ((Some(l), Some(g)), Some(e)) => gameplay_step_counter != INVALID && (l, g) == e,
        _ => false,
    }
}

unsafe extern "system-unwind" fn config_hook(fw: usize, out: usize) -> usize {
    *lock(&STARTED) = Some(Instant::now());
    PRESET.store(false, Ordering::Relaxed);
    if fw != 0 {
        let now = (loc_counter(), gameplay_counter());
        let end = *lock(&END);
        if keep(exe::rd(fw + FW_GAMEPLAY_COUNTER), now, end) {
            std::ptr::write_unaligned((fw + FW_CONFIG_COUNTER) as *mut u32, now.0.unwrap_or(INVALID));
            *lock(&LOC_BEFORE) = now.0;
            PRESET.store(true, Ordering::Relaxed);
        } else if end.is_some() {
            log(&format!(
                "configuration: rebuilt (localization {}, gameplay {}; after the last game configure: {})",
                hex(now.0),
                hex(now.1),
                end.map_or("none".into(), |(l, g)| format!("localization {l:#x}, gameplay {g:#x}"))
            ));
        }
    }
    let orig: ConfigFn = std::mem::transmute(ORIG_CONFIG.load(Ordering::Relaxed));
    orig(fw, out)
}

unsafe extern "system-unwind" fn gameplay_hook(fw: usize, out: usize, functor: usize) -> usize {
    let config_secs = lock(&STARTED).take().map(|t| t.elapsed().as_secs_f64());
    let preset = PRESET.load(Ordering::Relaxed);
    let mut kept = false;
    if fw != 0 && preset {
        if loc_counter().is_some() && loc_counter() == *lock(&LOC_BEFORE) {
            // L'étape de configuration a pris son chemin « matches ». On force le chemin complet de
            // l'étape de gameplay (nouvelle DLL de GameCore) ; bdd décide de ses bases comme
            // d'habitude.
            std::ptr::write_unaligned((fw + FW_GAMEPLAY_COUNTER) as *mut u32, INVALID);
            kept = true;
        } else {
            log("configuration: preset, but the configuration step rebuilt anyway");
        }
    }
    let orig: GameplayFn = std::mem::transmute(ORIG_GAMEPLAY.load(Ordering::Relaxed));
    let r = orig(fw, out, functor);
    if fw != 0 {
        // L'étape range son compteur à la fin de ses deux chemins ; -1 : elle n'a pas chargé de
        // contenu de partie.
        let ok = exe::rd::<u32>(fw + FW_GAMEPLAY_COUNTER) != INVALID;
        *lock(&END) = match (ok, loc_counter(), gameplay_counter()) {
            (true, Some(l), Some(g)) => Some((l, g)),
            _ => None,
        };
    }
    if PRESET.swap(false, Ordering::Relaxed) && fw != 0 {
        // Comme le laisse l'étape de gameplay d'origine : le FrontEnd reconstruit son contenu.
        std::ptr::write_unaligned((fw + FW_CONFIG_COUNTER) as *mut u32, INVALID);
        if kept {
            log(&format!(
                "configuration: kept (configuration step {:.2} s)",
                config_secs.unwrap_or(0.0)
            ));
        }
    }
    r
}

fn parse_sig(s: &str) -> Vec<Option<u8>> {
    s.split_whitespace().map(|t| if t == "??" { None } else { u8::from_str_radix(t, 16).ok() }).collect()
}

fn find_all(hay: &[u8], pat: &[Option<u8>]) -> Vec<usize> {
    (0..hay.len().saturating_sub(pat.len() - 1))
        .filter(|&i| pat.iter().enumerate().all(|(k, p)| p.map_or(true, |b| hay[i + k] == b)))
        .collect()
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Cible d'un `call rel32` de 5 octets en `at` (`va` : son adresse). Pour un `lea r,[rip+rel32]` de
/// 7 octets en `i`, passer `i + 2` et son adresse + 2.
fn call_target(code: &[u8], at: usize, va: usize) -> usize {
    let rel = i32::from_le_bytes(code[at + 1..at + 5].try_into().unwrap());
    (va as i64 + 5 + rel as i64) as usize
}

#[derive(Debug, PartialEq)]
pub struct Sites {
    pub config_call: usize,
    pub config_fn: usize,
    pub gameplay_call: usize,
    pub gameplay_fn: usize,
    /// Adresse de la globale de l'application, accesseurs des bases de localisation et de gameplay,
    /// accesseur de la connexion.
    pub app: usize,
    pub loc_db_fn: usize,
    pub gameplay_db_fn: usize,
    pub conn_fn: usize,
}

/// `image` : le module projeté depuis sa base (pour les chaînes) ; `text` : sa section .text, en
/// `text_va`.
pub fn locate(image: &[u8], base: usize, text: &[u8], text_va: usize) -> Result<Sites, String> {
    let unique = |name: &str, sig: &str, k: usize| -> Result<(usize, usize), String> {
        let hits = find_all(text, &parse_sig(sig));
        let [h] = hits[..] else { return Err(format!("{name}: {} matches instead of 1", hits.len())) };
        Ok((text_va + h + k, call_target(text, h + k, text_va + h + k)))
    };
    let (config_call, config_fn) = unique("configuration step call", SIG_CONFIG, SIG_CONFIG_CALL)?;
    let (gameplay_call, gameplay_fn) = unique("gameplay step call", SIG_GAMEPLAY, SIG_GAMEPLAY_CALL)?;
    let body = |va: usize, n: usize| -> &[u8] {
        let o = va.wrapping_sub(text_va);
        if o < text.len() { &text[o..(o + n).min(text.len())] } else { &[] }
    };
    let in_text = |t: usize| t >= text_va && t < text_va + text.len() && t % 16 == 0;
    let gp = body(gameplay_fn, 0xc00);
    if !contains(gp, MARK_238) || MARK_23C.iter().any(|m| !contains(gp, m)) {
        return Err(format!("gameplay step {gameplay_fn:#x}: counter field marks missing"));
    }
    let cf = body(config_fn, 0x600);
    let hits = find_all(cf, &parse_sig(SIG_LOC_COUNTER));
    let [h] = hits[..] else { return Err(format!("localization counter read: {} matches in the configuration step", hits.len())) };
    let rel = i32::from_le_bytes(cf[h + 3..h + 7].try_into().unwrap());
    let app = (config_fn as i64 + h as i64 + 7 + rel as i64) as usize;
    let loc_db_fn = call_target(cf, h + 7, config_fn + h + 7);
    let conn_fn = call_target(cf, h + 24, config_fn + h + 24);

    // « Rebuilding gameplay database. » [journal, accesseur de la base de gameplay, ...] : les deux
    // appels directs qui suivent.
    let at = (1..image.len().saturating_sub(T_REBUILD.len()))
        .find(|&i| image[i - 1] == 0 && &image[i..i + T_REBUILD.len()] == T_REBUILD)
        .ok_or("text \"Rebuilding gameplay database.\" not found")?;
    let lea = (0..gp.len().saturating_sub(7))
        .find(|&i| gp[i..i + 3] == [0x48, 0x8d, 0x15] && call_target(gp, i + 2, gameplay_fn + i + 2) == base + at)
        .ok_or("\"Rebuilding gameplay database.\" not used in the gameplay step")?;
    let mut calls = Vec::new();
    let mut i = lea + 7;
    while calls.len() < 2 && i + 5 <= gp.len() {
        if gp[i] == 0xe8 && in_text(call_target(gp, i, gameplay_fn + i)) {
            calls.push(call_target(gp, i, gameplay_fn + i));
            i += 5;
        } else {
            i += 1;
        }
    }
    let [_, gameplay_db_fn] = calls[..] else { return Err("gameplay database getter not found".into()) };
    if gameplay_db_fn == loc_db_fn {
        return Err("gameplay step: unexpected call layout after \"Rebuilding gameplay database.\"".into());
    }
    Ok(Sites { config_call, config_fn, gameplay_call, gameplay_fn, app, loc_db_fn, gameplay_db_fn, conn_fn })
}

/// SizeOfImage, d'après l'en-tête PE d'un module projeté.
unsafe fn image_size(module: usize) -> usize {
    let nt = module + exe::rd::<u32>(module + 0x3c) as usize;
    exe::rd::<u32>(nt + 24 + 56) as usize
}

pub fn install(exe_base: usize) {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("configuration: not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log("configuration: rebuilt at every load (config_gardee=off)");
        return;
    }
    unsafe {
        let module = if exe_base != 0 { exe_base } else { win::GetModuleHandleW(std::ptr::null()) as usize };
        let Some((start, size)) = exe::text_section(module) else {
            log("configuration: .text of the exe not found, not installed");
            return;
        };
        let text = std::slice::from_raw_parts(start as *const u8, size);
        let image = std::slice::from_raw_parts(module as *const u8, image_size(module));
        let sites = match locate(image, module, text, start) {
            Ok(s) => s,
            Err(e) => {
                log(&format!("configuration: unknown exe ({e}), not installed"));
                return;
            }
        };
        let Some(page) = patch::alloc_near(module) else {
            log("configuration: no memory within 2 GB of the exe, not installed");
            return;
        };
        let hooks = [(sites.config_call, config_hook as ConfigFn as usize), (sites.gameplay_call, gameplay_hook as GameplayFn as usize)];
        let mut rels = Vec::new();
        for (k, (site, hook)) in hooks.iter().enumerate() {
            let relay = page + 16 * k;
            let mut code = vec![0xff, 0x25, 0, 0, 0, 0];
            code.extend_from_slice(&(*hook as u64).to_le_bytes());
            std::ptr::copy_nonoverlapping(code.as_ptr(), relay as *mut u8, code.len());
            let Some(r) = patch::rel32(*site, relay) else {
                log("configuration: relay out of range, not installed");
                return;
            };
            rels.push(r);
        }
        ORIG_CONFIG.store(sites.config_fn, Ordering::Relaxed);
        ORIG_GAMEPLAY.store(sites.gameplay_fn, Ordering::Relaxed);
        APP.store(sites.app, Ordering::Relaxed);
        LOC_DB_FN.store(sites.loc_db_fn, Ordering::Relaxed);
        GAMEPLAY_DB_FN.store(sites.gameplay_db_fn, Ordering::Relaxed);
        CONN_FN.store(sites.conn_fn, Ordering::Relaxed);
        for ((site, _), r) in hooks.iter().zip(rels) {
            patch::write_rel32(*site, r);
        }
        ACTIF.store(true, Ordering::Relaxed);
        let ghidra = |a: usize| a - module + 0x1_4000_0000;
        log(&format!(
            "configuration: kept on reloads when unchanged (configuration step {:#x}, gameplay step {:#x})",
            ghidra(sites.config_fn),
            ghidra(sites.gameplay_fn)
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("config_gardee=off\n"));
        assert!(enabled("config_gardee=off\nconfig_gardee=on\n"));
    }

    #[test]
    fn keeps() {
        let end = Some((5, 9));
        assert!(keep(3, (Some(5), Some(9)), end));
        assert!(!keep(INVALID, (Some(5), Some(9)), end));
        assert!(!keep(3, (Some(6), Some(9)), end));
        assert!(!keep(3, (Some(5), Some(10)), end));
        assert!(!keep(3, (None, Some(9)), end));
        assert!(!keep(3, (Some(5), Some(9)), None));
    }

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";

    /// Les sites dans chaque exe connu qui est là (DX12 et DX11 de chaque version, dans l'ordre de
    /// `crate::test_exe`).
    #[test]
    fn locate_in_installed_exes() {
        let mut found = 0;
        for (i, exe_name) in ["CivilizationVI_DX12.exe", "CivilizationVI.exe"].iter().cycle().take(6).enumerate() {
            let path = crate::test_exe(EXE_DIR, i, exe_name);
            let Ok(file) = std::fs::read(&path) else { continue };
            let (image, base, text_rva, text_len) = map(&file);
            let text = &image[text_rva..text_rva + text_len];
            let sites = locate(&image, base, text, base + text_rva).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_ne!(sites.config_fn, sites.gameplay_fn);
            found += 1;
        }
        if found == 0 {
            eprintln!("no Civilization VI exe found, test skipped");
        }
    }

    /// Projette les sections d'un fichier PE à leurs RVA : (image, base préférée, RVA de .text,
    /// taille de .text).
    fn map(file: &[u8]) -> (Vec<u8>, usize, usize, usize) {
        let rd32 = |o: usize| u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
        let rd16 = |o: usize| u16::from_le_bytes(file[o..o + 2].try_into().unwrap()) as usize;
        let pe = rd32(0x3c);
        let base = u64::from_le_bytes(file[pe + 24 + 24..pe + 24 + 32].try_into().unwrap()) as usize;
        let size = rd32(pe + 24 + 56);
        let mut image = vec![0u8; size];
        let headers = rd32(pe + 24 + 60);
        image[..headers].copy_from_slice(&file[..headers]);
        let first = pe + 24 + rd16(pe + 20);
        let mut text = (0, 0);
        for i in 0..rd16(pe + 6) {
            let s = first + 40 * i;
            let (vsize, rva, raw_size, raw) = (rd32(s + 8), rd32(s + 12), rd32(s + 16), rd32(s + 20));
            let n = raw_size.min(vsize.max(raw_size)).min(file.len() - raw);
            image[rva..rva + n].copy_from_slice(&file[raw..raw + n]);
            if &file[s..s + 5] == b".text" {
                text = (rva, vsize);
            }
        }
        (image, base, text.0, text.1)
    }
}
