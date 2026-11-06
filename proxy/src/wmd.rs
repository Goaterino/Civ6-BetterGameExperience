//! Accélération de l'affichage pendant la visualisation d'une frappe WMD (voir nuke/README.md).
//!
//! Un nuke se visualise comme un combat, de `CombatVisBegin` à `CombatVisEnd`, pendant 11 à 12 s
//! qui retiennent toutes les autres animations. Pendant cette seule visualisation, on multiplie
//! l'échelle de temps de l'affichage, celle de la commande de console « timescale » (App+0x559c
//! dans l'exe DX12, appliquée au pas de chaque frame par `140076a40`). GameCore et le minuteur de tour ne la voient
//! pas : ils comptent en QueryPerformanceCounter. Rien ne part sur le réseau, chaque joueur
//! accélère son propre affichage.
//!
//! Une visualisation est celle d'une frappe quand :
//! - GameCore a calculé une frappe peu avant, sur son thread, pendant la vidange de sa file
//!   (`1800372f0`) : `180158af0` (ICBM d'un silo) ou `180156ae0`, qui sert aussi aux combats
//!   ordinaires ;
//! - elle n'a qu'un membre, le tireur : un combat ordinaire en a deux.
//!
//! Et seulement si le joueur a activé le combat rapide (Options → Jeu), lu comme le fait
//! `UserConfiguration.IsQuickCombat` : sans lui, le nuke garde sa vitesse d'origine.
//!
//! Côté exe, on détourne les fonctions de diffusion des deux événements, `1400f0400` et `1400f0460`
//! dans DX12. Leur prologue lit un pointeur relatif à RIP, que le trampoline relit en adressage
//! absolu. Les exe DX12 et DX11 sont compilés des mêmes sources : mêmes fonctions, à d'autres
//! adresses (`EXES`). On prend l'agencement dont tous les octets vérifiés correspondent.
//!
//! Réglage `wmd_accel=<facteur>` dans rollover_proxy.cfg, 20 par défaut ; 1 ou `off` : aucun détour.

use crate::patch::{self, Site};
use crate::{log, win};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

const DRAIN: Site = Site {
    name: "vidange de la file de messages",
    address: 0x1_8003_72f0,
    // mov [rsp+8],rbx / mov [rsp+10h],rsi / push rdi / sub rsp,60h
    expected: &[0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xec, 0x60],
    copy: (0, 15),
    resume: 15,
};

const ICBM: Site = Site {
    name: "résultats d'une frappe ICBM",
    address: 0x1_8015_8af0,
    // mov [rsp+18h],r8b / mov [rsp+10h],dl / push rbx / push rbp / push rsi / push rdi / push r13 /
    // push r14 / push r15
    expected: &[
        0x44, 0x88, 0x44, 0x24, 0x18, 0x88, 0x54, 0x24, 0x10, 0x53, 0x55, 0x56, 0x57, 0x41, 0x55, 0x41, 0x56,
        0x41, 0x57,
    ],
    copy: (0, 19),
    resume: 19,
};

const UNIT_WMD: Site = Site {
    name: "résultats d'une attaque d'unité",
    address: 0x1_8015_6ae0,
    // push rbx / push rsi / push rdi / sub rsp,60h / movzx ebx,r8b / movzx edi,dl
    expected: &[0x40, 0x53, 0x56, 0x57, 0x48, 0x83, 0xec, 0x60, 0x41, 0x0f, 0xb6, 0xd8, 0x0f, 0xb6, 0xfa],
    copy: (0, 15),
    resume: 15,
};

/// Fonction de diffusion d'un événement de l'exe : mov [rsp+8],rbx / push rdi / sub rsp,40h /
/// mov rax,[rip+disp]. Les 10 premiers octets se recopient, le `mov` est refait en absolu.
struct ExeSite {
    name: &'static str,
    /// Adresse dans l'exe, base 0x140000000.
    address: u64,
    expected: [u8; 17],
}

/// Ce dont le module a besoin dans un exe. Adresses Ghidra : projets civ6exe (DX12) et civ6exe11
/// (DX11), retrouvés dans le second par les chaînes « SerialEvent::CombatVis* » (fonctions
/// d'enregistrement), « IsQuickCombat » (sa closure Lua) et « timescale » (commande de console).
/// Celles du build 26407122 (exe du 2026-11-02) sont retrouvées par le même chemin, plus le
/// constructeur de l'application (celui qui pose 1.0 à l'échelle de temps) pour son pointeur global.
struct Exe {
    name: &'static str,
    vis_begin: ExeSite,
    vis_end: ExeSite,
    /// () -> configuration de l'utilisateur, créée au besoin.
    user_config: u64,
    /// (configuration) -> bool, valeur « QuickCombat ».
    is_quick_combat: u64,
    /// Pointeur vers l'objet application.
    app: u64,
    /// Décalage de l'échelle de temps (f32, 1.0 au repos) dans l'objet application.
    timescale: usize,
}

const fn dispatch_prologue(disp: [u8; 4]) -> [u8; 17] {
    [0x48, 0x89, 0x5c, 0x24, 0x08, 0x57, 0x48, 0x83, 0xec, 0x40, 0x48, 0x8b, 0x05, disp[0], disp[1], disp[2], disp[3]]
}

/// Un agencement par exe connu : le premier dont tous les octets vérifiés correspondent est pris.
const EXES: [Exe; 4] = [
    Exe {
        name: "DX12, build 26407122",
        vis_begin: ExeSite { name: "CombatVisBegin", address: 0x1_400f_0390, expected: dispatch_prologue([0xe7, 0xdd, 0x21, 0x02]) },
        vis_end: ExeSite { name: "CombatVisEnd", address: 0x1_400f_03f0, expected: dispatch_prologue([0x97, 0xdd, 0x21, 0x02]) },
        user_config: 0x1_4011_8a50,
        is_quick_combat: 0x1_4011_9630,
        app: 0x1_412f_b2c8,
        timescale: 0x559c,
    },
    Exe {
        name: "DX11, build 26407122",
        vis_begin: ExeSite { name: "CombatVisBegin", address: 0x1_400e_f640, expected: dispatch_prologue([0xb7, 0x74, 0x1e, 0x02]) },
        vis_end: ExeSite { name: "CombatVisEnd", address: 0x1_400e_f6a0, expected: dispatch_prologue([0x67, 0x74, 0x1e, 0x02]) },
        user_config: 0x1_4011_7b60,
        is_quick_combat: 0x1_4011_8740,
        app: 0x1_412c_3c88,
        timescale: 0x557c,
    },
    Exe {
        name: "DX12, build 15296837",
        vis_begin: ExeSite { name: "CombatVisBegin", address: 0x1_400f_0400, expected: dispatch_prologue([0x77, 0xcd, 0x21, 0x02]) },
        vis_end: ExeSite { name: "CombatVisEnd", address: 0x1_400f_0460, expected: dispatch_prologue([0x27, 0xcd, 0x21, 0x02]) },
        user_config: 0x1_4011_8ca0,
        is_quick_combat: 0x1_4011_9880,
        app: 0x1_412f_a2c8,
        timescale: 0x559c,
    },
    Exe {
        name: "DX11, build 15296837",
        vis_begin: ExeSite { name: "CombatVisBegin", address: 0x1_400e_ff70, expected: dispatch_prologue([0x87, 0x5b, 0x1e, 0x02]) },
        vis_end: ExeSite { name: "CombatVisEnd", address: 0x1_400e_ffd0, expected: dispatch_prologue([0x37, 0x5b, 0x1e, 0x02]) },
        user_config: 0x1_4011_82e0,
        is_quick_combat: 0x1_4011_8ec0,
        app: 0x1_412c_2c88,
        timescale: 0x557c,
    },
];

const EXE_IMAGE_BASE: u64 = 0x1_4000_0000;
const EXE_COPY: usize = 10;
/// Début des fonctions de configuration, le même dans les deux exe, vérifié avant de les appeler.
const USER_CONFIG_BYTES: [u8; 7] = [0x48, 0x83, 0xec, 0x38, 0x48, 0x8b, 0x05];
const IS_QUICK_COMBAT_BYTES: [u8; 10] = [0x48, 0x89, 0x5c, 0x24, 0x08, 0x57, 0x48, 0x83, 0xec, 0x20];

/// Une frappe calculée reste en attente de sa visualisation pendant ce délai, compté depuis la
/// dernière frappe ou la fin de la dernière visualisation accélérée (plusieurs frappes dans la
/// file se visualisent l'une après l'autre).
const PENDING_MS: f64 = 3000.0;
/// Garde-fou : une visualisation accélérée sans CombatVisEnd (partie quittée en pleine animation)
/// rend l'échelle d'origine au bout de ce délai. Un nuke ×20 dure 0,6 s.
const MAX_ACCEL_MS: f64 = 5000.0;

static FACTOR: AtomicU32 = AtomicU32::new(0x41a0_0000); // 20.0f32
static GC_BASE: AtomicUsize = AtomicUsize::new(0);
static EXE_BASE: AtomicUsize = AtomicUsize::new(0);
/// Indice dans `EXES` de l'exe reconnu, valable quand EXE_BASE n'est pas nul.
static EXE_INDEX: AtomicUsize = AtomicUsize::new(0);
static DRAIN_TRAMP: AtomicUsize = AtomicUsize::new(0);
static ICBM_TRAMP: AtomicUsize = AtomicUsize::new(0);
static UNIT_TRAMP: AtomicUsize = AtomicUsize::new(0);
static BEGIN_TRAMP: AtomicUsize = AtomicUsize::new(0);
static END_TRAMP: AtomicUsize = AtomicUsize::new(0);
/// Thread qui vide la file de GameCore en ce moment, 0 hors vidange.
static DRAIN_TID: AtomicU32 = AtomicU32::new(0);

struct State {
    /// Frappes calculées dont la visualisation n'est pas encore venue.
    pending: u32,
    /// Dernière frappe, ou fin de la dernière visualisation accélérée (QPC).
    last: i64,
    /// Échelle d'avant l'accélération en cours, et son début (QPC).
    accel: Option<(f32, i64)>,
}

static STATE: Mutex<State> = Mutex::new(State { pending: 0, last: 0, accel: None });

pub fn set_factor(f: f32) {
    FACTOR.store(f.to_bits(), Ordering::Relaxed);
}

fn factor() -> f32 {
    f32::from_bits(FACTOR.load(Ordering::Relaxed))
}

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn layout() -> &'static Exe {
    &EXES[EXE_INDEX.load(Ordering::Relaxed)]
}

fn exe(addr: u64) -> usize {
    EXE_BASE.load(Ordering::Relaxed) + (addr - EXE_IMAGE_BASE) as usize
}

/// Combat rapide activé par le joueur ? Thread principal seulement, comme l'interface.
unsafe fn quick_combat() -> bool {
    let get: unsafe extern "system" fn() -> usize = std::mem::transmute(exe(layout().user_config));
    let is_quick: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(exe(layout().is_quick_combat));
    let cfg = get();
    cfg != 0 && is_quick(cfg) as u8 != 0
}

unsafe fn timescale() -> Option<*mut f32> {
    let app = *(exe(layout().app) as *const usize);
    (app != 0).then(|| (app + layout().timescale) as *mut f32)
}

/// Rend l'échelle d'origine si une accélération est en cours.
unsafe fn restore_timescale(s: &mut State, why: &str) {
    let Some((from, since)) = s.accel.take() else { return };
    let now = win::qpc();
    s.last = now;
    if let Some(p) = timescale() {
        *p = from;
    }
    log(&format!("wmd : échelle rendue à {from} ({why}, {:.0} ms)", win::qpc_ms(now - since)));
}

unsafe fn expire(s: &mut State) {
    if let Some((_, since)) = s.accel {
        if win::qpc_ms(win::qpc() - since) > MAX_ACCEL_MS {
            restore_timescale(s, "délai dépassé");
        }
    }
}

type Fn8 = unsafe extern "system" fn(usize, usize, usize, usize, usize, usize, usize, usize) -> usize;

// Détours de GameCore. Relais à 8 `usize`, comme forward.rs : les arguments sont entiers.

unsafe extern "system" fn hook_drain(a: usize, b: usize, c: usize, d: usize, e: usize, f: usize, g: usize, h: usize) -> usize {
    let tid = win::GetCurrentThreadId();
    let outer = DRAIN_TID.load(Ordering::Relaxed) != tid;
    if outer {
        DRAIN_TID.store(tid, Ordering::Relaxed);
        expire(&mut state());
    }
    let real: Fn8 = std::mem::transmute(DRAIN_TRAMP.load(Ordering::Relaxed));
    let r = real(a, b, c, d, e, f, g, h);
    if outer {
        DRAIN_TID.store(0, Ordering::Relaxed);
    }
    r
}

/// Une frappe calculée par le thread GameCore pendant sa vidange : sa visualisation va suivre.
fn on_strike() {
    if DRAIN_TID.load(Ordering::Relaxed) == unsafe { win::GetCurrentThreadId() } {
        let mut s = state();
        s.pending += 1;
        s.last = win::qpc();
    }
}

unsafe extern "system" fn hook_icbm(a: usize, b: usize, c: usize, d: usize, e: usize, f: usize, g: usize, h: usize) -> usize {
    on_strike();
    let real: Fn8 = std::mem::transmute(ICBM_TRAMP.load(Ordering::Relaxed));
    real(a, b, c, d, e, f, g, h)
}

unsafe extern "system" fn hook_unit(a: usize, b: usize, c: usize, d: usize, e: usize, f: usize, g: usize, h: usize) -> usize {
    on_strike();
    let real: Fn8 = std::mem::transmute(UNIT_TRAMP.load(Ordering::Relaxed));
    real(a, b, c, d, e, f, g, h)
}

// Détours de l'exe, sur le thread principal. Données construites par 1403c72d0 : 4 membres
// { i16 joueur, u16 type, i32 id } en 0..31, joueur -1 pour un membre vide.

unsafe fn members(data: usize) -> usize {
    (0..4).filter(|i| *((data + i * 8) as *const i16) != -1).count()
}

unsafe extern "system" fn hook_vis_begin(data: usize, b: usize) {
    {
        let mut s = state();
        expire(&mut s);
        let fresh = s.pending > 0 && win::qpc_ms(win::qpc() - s.last) < PENDING_MS;
        if !fresh {
            s.pending = 0;
        }
        if fresh && s.accel.is_none() && data != 0 && members(data) == 1 {
            s.pending -= 1;
            match timescale().filter(|_| quick_combat()) {
                // Garde-fou : une valeur inattendue veut dire que l'adresse est fausse.
                Some(p) if *p > 0.05 && *p < 20.0 => {
                    let from = *p;
                    *p = from * factor();
                    s.accel = Some((from, win::qpc()));
                    log(&format!("wmd : visualisation accélérée, échelle {from} → {}", *p));
                }
                Some(p) => log(&format!("wmd : échelle de temps inattendue ({}), pas d'accélération", *p)),
                None => log("wmd : combat rapide désactivé, visualisation à vitesse normale"),
            }
        }
    }
    let real: unsafe extern "system" fn(usize, usize) = std::mem::transmute(BEGIN_TRAMP.load(Ordering::Relaxed));
    real(data, b)
}

unsafe extern "system" fn hook_vis_end(data: usize, b: usize) {
    restore_timescale(&mut state(), "fin de visualisation");
    let real: unsafe extern "system" fn(usize, usize) = std::mem::transmute(END_TRAMP.load(Ordering::Relaxed));
    real(data, b)
}

unsafe fn exe_at(base: usize, site: &ExeSite) -> usize {
    base + (site.address - EXE_IMAGE_BASE) as usize
}

unsafe fn install_exe(base: usize, site: &ExeSite, hook: usize) -> Result<usize, String> {
    let at = exe_at(base, site);
    let next = at + site.expected.len();
    let disp = i32::from_le_bytes(site.expected[13..17].try_into().unwrap());
    // movabs rax,[adresse] : 48 A1 imm64.
    let mut prologue = site.expected[..EXE_COPY].to_vec();
    prologue.extend_from_slice(&[0x48, 0xa1]);
    prologue.extend_from_slice(&((next as i64 + disp as i64) as u64).to_le_bytes());
    patch::install_at(site.name, at, &prologue, next, hook)
}

/// Tous les octets vérifiés de cet agencement sont-ils dans l'exe chargé en `exe` ?
unsafe fn matches(exe: usize, e: &Exe) -> bool {
    // Taille de l'image (SizeOfImage de l'en-tête PE) : un autre exe peut être plus petit.
    let size = *((exe + *((exe + 0x3c) as *const u32) as usize + 0x50) as *const u32) as u64;
    let sites: [(u64, &[u8]); 4] = [
        (e.vis_begin.address, &e.vis_begin.expected),
        (e.vis_end.address, &e.vis_end.expected),
        (e.user_config, &USER_CONFIG_BYTES),
        (e.is_quick_combat, &IS_QUICK_COMBAT_BYTES),
    ];
    sites.iter().all(|(address, expected)| {
        address - EXE_IMAGE_BASE + expected.len() as u64 <= size
            && std::slice::from_raw_parts((exe + (address - EXE_IMAGE_BASE) as usize) as *const u8, expected.len()) == *expected
    })
}

/// Appelé quand l'originale vient d'être chargée, avant le premier appel relayé.
pub unsafe fn on_gamecore_loaded(base: usize) {
    if factor() <= 1.0 {
        return;
    }
    let exe = win::GetModuleHandleW(std::ptr::null()) as usize;
    let Some(index) = EXES.iter().position(|e| matches(exe, e)) else {
        log("wmd : exe inattendu (aucun agencement connu de DX12 ni de DX11), aucun détour");
        return;
    };
    if let Err(e) = patch::check(base, &[DRAIN, ICBM, UNIT_WMD]) {
        log(&format!("wmd : version inattendue, aucun détour : {e}"));
        return;
    }
    EXE_INDEX.store(index, Ordering::Relaxed);
    EXE_BASE.store(exe, Ordering::Relaxed);
    GC_BASE.store(base, Ordering::Relaxed);
    // L'exe d'abord : la fin de visualisation doit pouvoir rendre l'échelle avant qu'une frappe
    // ne puisse l'accélérer.
    let r = (|| {
        END_TRAMP.store(install_exe(exe, &EXES[index].vis_end, hook_vis_end as *const () as usize)?, Ordering::Relaxed);
        BEGIN_TRAMP.store(install_exe(exe, &EXES[index].vis_begin, hook_vis_begin as *const () as usize)?, Ordering::Relaxed);
        DRAIN_TRAMP.store(patch::install(base, &DRAIN, hook_drain as *const () as usize)?, Ordering::Relaxed);
        ICBM_TRAMP.store(patch::install(base, &ICBM, hook_icbm as *const () as usize)?, Ordering::Relaxed);
        UNIT_TRAMP.store(patch::install(base, &UNIT_WMD, hook_unit as *const () as usize)?, Ordering::Relaxed);
        Ok::<(), String>(())
    })();
    match r {
        Ok(()) => log(&format!("wmd : détours posés ({}), accélération ×{}", EXES[index].name, factor())),
        Err(e) => log(&format!("wmd : échec de la pose des détours : {e}")),
    }
}

/// Nouvelle partie : rien de la précédente ne doit rester accéléré.
pub fn reset() {
    let mut s = state();
    s.pending = 0;
    unsafe { restore_timescale(&mut s, "nouvelle partie") };
}

/// Le proxy est déchargé alors que l'exe et l'originale restent en mémoire : on remet leurs octets.
pub unsafe fn on_proxy_unload() {
    reset();
    let exe = EXE_BASE.swap(0, Ordering::Relaxed);
    let base = GC_BASE.swap(0, Ordering::Relaxed);
    if exe != 0 {
        let e = layout();
        if END_TRAMP.load(Ordering::Relaxed) != 0 {
            patch::restore_at(exe_at(exe, &e.vis_end), &e.vis_end.expected);
        }
        if BEGIN_TRAMP.load(Ordering::Relaxed) != 0 {
            patch::restore_at(exe_at(exe, &e.vis_begin), &e.vis_begin.expected);
        }
    }
    if base != 0 {
        for (site, tramp) in [(&DRAIN, &DRAIN_TRAMP), (&ICBM, &ICBM_TRAMP), (&UNIT_WMD, &UNIT_TRAMP)] {
            if tramp.load(Ordering::Relaxed) != 0 {
                patch::restore(base, site);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI";

    /// Chaque exe installé, projeté comme image sans être exécuté, a exactement un agencement connu,
    /// de son propre type (DX12 ou DX11). Échoue après une mise à jour du jeu qui déplace le code.
    #[test]
    fn agencement_des_exe_installes() {
        #[link(name = "kernel32")]
        extern "system" {
            fn LoadLibraryExW(name: *const u16, file: *mut std::ffi::c_void, flags: u32) -> win::HMODULE;
            fn FreeLibrary(module: win::HMODULE) -> i32;
        }
        const LOAD_LIBRARY_AS_IMAGE_RESOURCE: u32 = 0x20;
        for (exe, kind) in [("CivilizationVI_DX12.exe", "DX12"), ("CivilizationVI.exe", "DX11")] {
            let path = format!(r"{GAME}\Base\Binaries\Win64Steam\{exe}");
            if !std::path::Path::new(&path).exists() {
                eprintln!("absent, test sauté : {path}");
                continue;
            }
            unsafe {
                let h = LoadLibraryExW(win::wide(&path).as_ptr(), std::ptr::null_mut(), LOAD_LIBRARY_AS_IMAGE_RESOURCE);
                assert!(!h.is_null(), "{exe}");
                let base = h as usize & !3;
                let found: Vec<&str> = EXES.iter().filter(|e| matches(base, e)).map(|e| e.name).collect();
                FreeLibrary(h);
                assert_eq!(found.len(), 1, "{exe} : {found:?}");
                assert!(found[0].starts_with(kind), "{exe} : {found:?}");
            }
        }
    }
}
