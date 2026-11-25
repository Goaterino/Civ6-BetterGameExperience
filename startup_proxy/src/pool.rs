//! Le pool de tâches de l'exe (Intel TBB, lié statiquement) : moins d'attente active
//! (analyses/chargement/pool_taches.md). Trois réglages :
//!
//! - `pool_cessions` (16 par défaut, 100 = jeu d'origine) : un ouvrier sans travail fait des pauses
//!   puis au moins 101 `SwitchToThread` avant de vérifier que l'arène est vide et de s'endormir sur
//!   son événement (le sommeil et le réveil par `SetEvent` existent déjà dans TBB). Ce compte à
//!   rebours dure environ 0,15 ms, 20 à 25 fois la latence de réveil, et se répète des centaines de
//!   fois par seconde au passage de tour. On baisse le seuil de 100 (`cmp eax,0x64`, l'octet
//!   immédiat, `140e5100b` en DX12, `140e42a5b` en DX11) : l'ouvrier prend plus tôt le chemin de
//!   sommeil de TBB.
//! - `pool_pauses` (0 par défaut = jeu d'origine) : avant sa première cession, l'ouvrier fait
//!   `2·(n+1)` tentatives séparées de pauses (`lea eax,[rsi*2+2]`, `140e50ff0`, `140e42a40` en
//!   DX11), soit 14 avec 6 threads et 26 avec 12. Le réglage remplace ce seuil par une constante
//!   (`mov eax, imm32 ; nop ; nop`, même longueur, rien ne bouge autour). Mesuré le 2026-11-17 avec
//!   6 threads et `pool_cessions=4` : les pauses ne font plus que 5 % du CPU des ouvriers pendant le
//!   passage de tour (1,0 s sur trois passages), c'est le gain maximal.
//! - `pool_attente_fin` (on par défaut) : `EXP_JobManager_Wait` (`task_arena::internal_wait`,
//!   `140e4d0a0`) finit par une boucle de `SwitchToThread` sans limite tant que des ouvriers sont
//!   dans l'arène (références `arena+0x84` >= 2), 1,1 à 1,3 s de CPU par chargement. L'appel
//!   `call [SwitchToThread]` (`140e4d3bd`) passe par `attendre_sortie` : comme l'original pendant
//!   environ 20 µs, puis `WaitOnAddress` sur les références (délai de secours 1 ms). Un ouvrier
//!   qui quitte l'arène (`lock xadd [rdi+0x84], -2`, `140e4d804`) réveille les attentes par
//!   `WakeByAddressAll` (sans attente, il reste en mode utilisateur). Les autres sorties (maîtres,
//!   emplacement 0) ne réveillent personne : le délai de 1 ms les couvre.
//!
//! Ordonnancement seulement : le résultat des tâches ne dépend déjà pas du nombre de threads, qui
//! varie d'une machine à l'autre. Le nombre d'ouvriers n'est pas changé ici.

use crate::{exe, log, motifs, patch, win};
use std::cell::Cell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

/// Compte à rebours de `receive_or_steal_task` (`140e50ff0`) ; l'octet 0x64 en +0x1b.
const CESSIONS: &str =
    "8D 04 75 02 00 00 00 3B E8 7C ?? 8B E8 FF 15 ?? ?? ?? ?? 41 8B C7 41 FF C7 83 F8 64 7C ?? 4D 85 F6 75 ?? 48 8B 4F 20 E8";
const CESSIONS_OCTET: usize = 0x1b;
/// Le seuil des pauses est le début du même motif : `lea eax,[rsi*2+2]`, 7 octets.
const PAUSES_LEA: [u8; 7] = [0x8d, 0x04, 0x75, 0x02, 0x00, 0x00, 0x00];
/// Boucle finale de `internal_wait` (`140e4d3a3`) : `call [SwitchToThread]` (`FF 15`) en +0x1a.
const ATTENTE: &str =
    "48 8B 06 8B 80 84 00 00 00 83 F8 02 73 ?? 48 8B 06 4C 39 B0 80 01 00 00 74 ?? FF 15 ?? ?? ?? ?? E9";
const ATTENTE_APPEL: usize = 0x1a;
/// Sortie d'un ouvrier de l'arène (`140e4d7f1`) : `mov r8,[rdi+0E0h] ; mov eax,-2 ;
/// mov rcx,[rdi+0D8h] ; lock xadd [rdi+84h],eax`, 27 octets, sans adressage relatif.
const SORTIE: &str = "4C 8B 87 E0 00 00 00 B8 FE FF FF FF 48 8B 8F D8 00 00 00 F0 0F C1 87 84 00 00 00";
const SORTIE_LONG: usize = 27;

/// Durée des cessions avant le sommeil, et intervalle au-delà duquel un appel ouvre un nouvel
/// épisode d'attente (en µs).
const CESSIONS_US: i64 = 20;
const EPISODE_US: i64 = 100;
const DELAI_MS: u32 = 1;

static SWITCH: AtomicUsize = AtomicUsize::new(0);
static WAIT: AtomicUsize = AtomicUsize::new(0);
static FREQ_US: AtomicI32 = AtomicI32::new(0);

thread_local! {
    /// (début de l'épisode, dernier appel), en ticks QPC.
    static EPISODE: Cell<(i64, i64)> = const { Cell::new((0, 0)) };
}

type Switch = unsafe extern "system" fn() -> i32;
type WaitOnAddress = unsafe extern "system" fn(*const c_void, *const c_void, usize, u32) -> i32;

/// Appelée à la place de `SwitchToThread` dans la boucle finale d'`internal_wait`, avec l'arène.
unsafe extern "system" fn attendre_sortie(arena: *const u8) -> i32 {
    let switch: Switch = std::mem::transmute(SWITCH.load(Ordering::Relaxed));
    let refs = arena.add(0x84) as *const i32;
    let v = std::ptr::read_volatile(refs);
    if v < 2 {
        // Plus d'ouvrier, mais l'emplacement 0 est pris par un autre maître : rien ne réveillerait
        // une attente sur les références, on cède comme le jeu.
        return switch();
    }
    let maintenant = win::qpc();
    let par_us = FREQ_US.load(Ordering::Relaxed) as i64;
    let (debut, dernier) = EPISODE.with(Cell::get);
    let debut = if maintenant - dernier > EPISODE_US * par_us { maintenant } else { debut };
    EPISODE.with(|e| e.set((debut, maintenant)));
    if maintenant - debut < CESSIONS_US * par_us {
        return switch();
    }
    let wait: WaitOnAddress = std::mem::transmute(WAIT.load(Ordering::Relaxed));
    wait(refs as *const c_void, &v as *const i32 as *const c_void, 4, DELAI_MS);
    1
}

/// Le réglage `pool_cessions` : 1 à 100 (100 = jeu d'origine) ; 16 par défaut.
pub fn cessions(cfg: &str) -> u8 {
    cfg.lines()
        .filter_map(|l| l.trim().strip_prefix("pool_cessions="))
        .last()
        .and_then(|v| v.trim().parse::<u8>().ok())
        .filter(|n| (1..=100).contains(n))
        .unwrap_or(16)
}

/// Le réglage `pool_pauses` : 0 = jeu d'origine (`2·(n+1)`, par défaut), sinon 1 à 64.
pub fn pauses(cfg: &str) -> u8 {
    cfg.lines()
        .filter_map(|l| l.trim().strip_prefix("pool_pauses="))
        .last()
        .and_then(|v| v.trim().parse::<u8>().ok())
        .filter(|n| *n <= 64)
        .unwrap_or(0)
}

/// `mov eax, n ; nop ; nop`, à la place du `lea` du seuil.
fn seuil_fixe(n: u8) -> [u8; 7] {
    [0xb8, n, 0, 0, 0, 0x90, 0x90]
}

/// Le réglage `pool_attente_fin` (dernière occurrence ; `on` par défaut).
pub fn attente_fin(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("pool_attente_fin=")).last().map(str::trim) != Some("off")
}

unsafe fn ecrire(at: usize, octets: &[u8]) -> Result<(), String> {
    let mut old = 0u32;
    if win::VirtualProtect(at as *mut c_void, octets.len(), win::PAGE_EXECUTE_READWRITE, &mut old) == 0 {
        return Err("VirtualProtect a échoué".into());
    }
    std::ptr::copy_nonoverlapping(octets.as_ptr(), at as *mut u8, octets.len());
    win::VirtualProtect(at as *mut c_void, octets.len(), old, &mut old);
    win::FlushInstructionCache(win::GetCurrentProcess(), at as *const c_void, octets.len());
    Ok(())
}

unsafe fn unique(exe_base: usize, motif: &str, quoi: &str) -> Result<usize, String> {
    let t = motifs::chercher(exe_base, &motifs::motif(motif));
    match t[..] {
        [a] => Ok(a),
        _ => Err(format!("{} {quoi} sites", t.len())),
    }
}

unsafe fn fonction(dll: &str, nom: &[u8]) -> Result<usize, String> {
    let m = win::GetModuleHandleW(win::wide(dll).as_ptr());
    let f = if m.is_null() { std::ptr::null_mut() } else { win::GetProcAddress(m, nom.as_ptr()) };
    if f.is_null() {
        return Err(format!("{} introuvable dans {dll}", String::from_utf8_lossy(&nom[..nom.len() - 1])));
    }
    Ok(f as usize)
}

// Les deux réglages modifient le même motif (le seuil des pauses à son début, l'octet des
// cessions plus loin) : on le cherche une seule fois, avant de modifier l'un ou l'autre.
unsafe fn poser_cessions(site: &Result<usize, String>, n: u8) -> Result<(), String> {
    let at = site.clone()? + CESSIONS_OCTET;
    ecrire(at, &[n])
}

unsafe fn poser_pauses(site: &Result<usize, String>, n: u8) -> Result<(), String> {
    let at = site.clone()?;
    if exe::rd::<[u8; 7]>(at) != PAUSES_LEA {
        return Err("unexpected instruction".into());
    }
    ecrire(at, &seuil_fixe(n))
}

/// La caverne de la sortie d'un ouvrier : les quatre instructions d'origine, puis
/// `WakeByAddressAll(rdi+0x84)` en gardant tous les registres volatils, les drapeaux et xmm0-5
/// (on est au milieu d'une fonction), puis retour après les 27 octets.
fn caverne_sortie(origine: &[u8; SORTIE_LONG], reveil: u64, retour: u64) -> Vec<u8> {
    let mut c = origine.to_vec();
    c.push(0x9c); // pushfq
    c.extend_from_slice(&[0x50, 0x51, 0x52, 0x41, 0x50, 0x41, 0x51, 0x41, 0x52, 0x41, 0x53]); // push rax rcx rdx r8 r9 r10 r11
    c.extend_from_slice(&[0x48, 0x81, 0xec, 0x80, 0x00, 0x00, 0x00]); // sub rsp,80h (0x20 d'ombre + 6 xmm)
    for i in 0..6u8 {
        c.extend_from_slice(&[0xf3, 0x0f, 0x7f, 0x44 + (i << 3), 0x24, 0x20 + i * 0x10]); // movdqu [rsp+20h+16i], xmmi
    }
    c.extend_from_slice(&[0x48, 0x8d, 0x8f, 0x84, 0x00, 0x00, 0x00]); // lea rcx,[rdi+84h]
    c.extend_from_slice(&[0x48, 0xb8]); // mov rax, imm64
    c.extend_from_slice(&reveil.to_le_bytes());
    c.extend_from_slice(&[0xff, 0xd0]); // call rax
    for i in 0..6u8 {
        c.extend_from_slice(&[0xf3, 0x0f, 0x6f, 0x44 + (i << 3), 0x24, 0x20 + i * 0x10]); // movdqu xmmi, [rsp+20h+16i]
    }
    c.extend_from_slice(&[0x48, 0x81, 0xc4, 0x80, 0x00, 0x00, 0x00]); // add rsp,80h
    c.extend_from_slice(&[0x41, 0x5b, 0x41, 0x5a, 0x41, 0x59, 0x41, 0x58, 0x5a, 0x59, 0x58]); // pop r11 r10 r9 r8 rdx rcx rax
    c.push(0x9d); // popfq
    c.extend_from_slice(&[0xff, 0x25, 0, 0, 0, 0]); // jmp [rip]
    c.extend_from_slice(&retour.to_le_bytes());
    c
}

/// Le relais de l'appel d'attente : `mov rcx,[rsi]` (l'arène), puis saut vers `attendre_sortie`.
fn relais_attente(crochet: u64) -> Vec<u8> {
    let mut c = vec![0x48, 0x8b, 0x0e, 0xff, 0x25, 0, 0, 0, 0];
    c.extend_from_slice(&crochet.to_le_bytes());
    c
}

unsafe fn poser_attente(exe_base: usize) -> Result<(), String> {
    let appel = unique(exe_base, ATTENTE, "arena wait")? + ATTENTE_APPEL;
    let sortie = unique(exe_base, SORTIE, "worker leave")?;
    SWITCH.store(fonction("kernel32.dll", b"SwitchToThread\0")?, Ordering::Relaxed);
    WAIT.store(fonction("kernelbase.dll", b"WaitOnAddress\0")?, Ordering::Relaxed);
    let reveil = fonction("kernelbase.dll", b"WakeByAddressAll\0")?;
    let mut f = 0i64;
    win::QueryPerformanceFrequency(&mut f);
    FREQ_US.store((f / 1_000_000).max(1) as i32, Ordering::Relaxed);

    // Réveil d'abord : sans attente, il ne fait que coûter quelques dizaines de ns.
    let page = patch::alloc_near(sortie).ok_or("pas de page proche")?;
    let origine: [u8; SORTIE_LONG] = exe::rd(sortie);
    let cav = caverne_sortie(&origine, reveil as u64, (sortie + SORTIE_LONG) as u64);
    std::ptr::copy_nonoverlapping(cav.as_ptr(), page as *mut u8, cav.len());
    // Seule la première instruction (7 octets) est remplacée par `jmp rel32 ; nop ; nop` : un saut
    // du jeu vers les instructions suivantes, restées en place, reste valable.
    let rel = patch::rel32(sortie, page).ok_or("page trop loin")?;
    let mut saut = [0x90u8; 7];
    saut[0] = 0xe9;
    saut[1..5].copy_from_slice(&rel.to_le_bytes());
    ecrire(sortie, &saut)?;

    let page = patch::alloc_near(appel).ok_or("pas de page proche")?;
    let rel_ = relais_attente(attendre_sortie as *const () as u64);
    std::ptr::copy_nonoverlapping(rel_.as_ptr(), page as *mut u8, rel_.len());
    let rel = patch::rel32(appel, page).ok_or("page trop loin")?;
    let mut call = [0x90u8; 6];
    call[0] = 0xe8;
    call[1..5].copy_from_slice(&rel.to_le_bytes());
    ecrire(appel, &call)
}

pub fn install(exe_base: usize) {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("pool: not installed (actif=off)");
        return;
    }
    let site = unsafe { unique(exe_base, CESSIONS, "yield countdown") };
    let n = cessions(&cfg);
    if n == 100 {
        log("pool: yield countdown of the game (pool_cessions=100)");
    } else {
        match unsafe { poser_cessions(&site, n) } {
            Ok(()) => log(&format!("pool: idle workers go to sleep after {n} yields instead of 100")),
            Err(e) => log(&format!("pool: yield countdown not changed ({e})")),
        }
    }
    let p = pauses(&cfg);
    if p == 0 {
        log("pool: spins before the first yield as in the game (pool_pauses=0)");
    } else {
        match unsafe { poser_pauses(&site, p) } {
            Ok(()) => log(&format!("pool: idle workers yield after {p} spin rounds instead of 2*(threads+1)")),
            Err(e) => log(&format!("pool: spin threshold not changed ({e})")),
        }
    }
    if !attente_fin(&cfg) {
        log("pool: end of job waits spins as in the game (pool_attente_fin=off)");
        return;
    }
    match unsafe { poser_attente(exe_base) } {
        Ok(()) => log("pool: end of job waits sleeps on the arena after 20 us of yields"),
        Err(e) => log(&format!("pool: end of job wait not changed ({e})")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglages() {
        assert_eq!(cessions(""), 16);
        assert_eq!(cessions("pool_cessions=8\n"), 8);
        assert_eq!(cessions("pool_cessions=100\n"), 100);
        assert_eq!(cessions("pool_cessions=0\n"), 16, "hors bornes");
        assert_eq!(cessions("pool_cessions=abc\n"), 16);
        assert_eq!(pauses(""), 0);
        assert_eq!(pauses("pool_pauses=2\n"), 2);
        assert_eq!(pauses("pool_pauses=65\n"), 0, "hors bornes");
        assert_eq!(seuil_fixe(4).len(), PAUSES_LEA.len());
        assert!(attente_fin(""));
        assert!(!attente_fin("pool_attente_fin=off\n"));
    }

    #[test]
    fn caverne() {
        let origine: [u8; SORTIE_LONG] = std::array::from_fn(|i| i as u8);
        let c = caverne_sortie(&origine, 0x1122334455667788, 0x99aabbccddeeff00);
        assert_eq!(&c[..SORTIE_LONG], &origine);
        assert!(c.ends_with(&0x99aabbccddeeff00u64.to_le_bytes()));
        // Autant de push que de pop, 16 octets d'écart de pile au total.
        let movdqu_save: Vec<_> = c.windows(3).filter(|w| w == &[0xf3, 0x0f, 0x7f]).collect();
        let movdqu_load: Vec<_> = c.windows(3).filter(|w| w == &[0xf3, 0x0f, 0x6f]).collect();
        assert_eq!((movdqu_save.len(), movdqu_load.len()), (6, 6));
        assert!(c.len() < 0x1000);
    }

    #[test]
    fn motifs_uniques_dans_les_exes() {
        for nom in ["CivilizationVI_DX12.exe", "CivilizationVI.exe"] {
            let Ok(exe) = std::fs::read(format!(
                r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam\{nom}"
            )) else {
                return;
            };
            for m in [CESSIONS, ATTENTE, SORTIE] {
                assert_eq!(motifs::trouver(&exe, &motifs::motif(m)).len(), 1, "{nom} : {m}");
            }
            let c = motifs::trouver(&exe, &motifs::motif(CESSIONS))[0];
            assert_eq!(exe[c..c + 7], PAUSES_LEA, "{nom} : le seuil des pauses");
        }
    }
}
