//! Regroupement des instantanés de l'interface : villes d'un joueur et diplomatie d'un joueur.
//!
//! Voir rollover/cascade_guerre.md :
//! - `180072f80(événement, joueur)` refait l'instantané de toutes les villes d'un joueur ;
//! - `1800b7630(événement, joueur)` refait l'instantané diplomatique d'un joueur envers tous les
//!   autres, infobulles de `1801d02d0` comprises.
//!
//! Les deux ne font que lire l'état de jeu et écrire dans des réserves de cache que seul le code
//! `Cache_*` lit. Pendant une portée, on note les appels au lieu de les exécuter, puis on rejoue
//! chaque triplet (instantané, événement, joueur) une seule fois, dans l'ordre de première
//! apparition, à la fermeture de la portée la plus externe. Deux portées :
//! - `1802a1f90` (DeclareWar), récursive à travers le signal « guerre déclarée » ;
//! - avec `portee=message`, aussi `18004afc0` (Game_Processor), une passe de traitement d'un
//!   message `GameUpdate`. Sa fermeture précède `18007bbe0`, qui expédie les caches à l'interface.
//!
//! Le détour de la passe est aussi posé pour `rendements_empire` (`ui_rendements=on`), même en
//! mode Off ou avec `portee=cascade` : il appelle alors seulement `rendements_empire::after_step`
//! à la fin de la passe la plus externe.
//!
//! L'état de portée est un statique protégé par un verrou, rattaché au thread qui l'a ouverte :
//! pas de `thread_local!`, dont les destructeurs survivraient au déchargement du proxy.

use crate::patch::{self, Site};
use crate::{log, win};
use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;

const SNAPSHOT: Site = Site {
    name: "instantané des villes d'un joueur",
    address: 0x1_8007_2f80,
    // mov [rsp+10h],rbx / push rdi / sub rsp,30h / movsxd rbx,edx / mov edi,ecx
    expected: &[0x48, 0x89, 0x5c, 0x24, 0x10, 0x57, 0x48, 0x83, 0xec, 0x30, 0x48, 0x63, 0xda, 0x8b, 0xf9],
    copy: (0, 15),
    resume: 15,
};

const DIPLO: Site = Site {
    name: "instantané diplomatique d'un joueur",
    address: 0x1_800b_7630,
    // mov [rsp+10h],rbx / push rsi / sub rsp,20h / movsxd rbx,edx / mov esi,ecx
    expected: &[0x48, 0x89, 0x5c, 0x24, 0x10, 0x56, 0x48, 0x83, 0xec, 0x20, 0x48, 0x63, 0xda, 0x8b, 0xf1],
    copy: (0, 15),
    resume: 15,
};

const DECLARE_WAR: Site = Site {
    name: "DeclareWar",
    address: 0x1_802a_1f90,
    // cmp edx,-1 / jz <ret> : refait par le détour, car le saut relatif ne se recopie pas.
    // Puis mov rax,rsp / mov [rax+8],rcx / push rbx / push rbp / push rdi / push r12 / sub rsp,68h
    expected: &[
        0x83, 0xfa, 0xff, 0x0f, 0x84, 0x0a, 0x02, 0x00, 0x00, 0x48, 0x8b, 0xc4, 0x48, 0x89, 0x48, 0x08,
        0x53, 0x55, 0x57, 0x41, 0x54, 0x48, 0x83, 0xec, 0x68,
    ],
    copy: (9, 25),
    resume: 25,
};

const STEP: Site = Site {
    name: "passe de Game_Processor",
    address: 0x1_8004_afc0,
    // mov [rsp+18h],rbx / push rbp / push rsi / push rdi / push r12 / push r13 / push r14 / push r15
    expected: &[0x48, 0x89, 0x5c, 0x24, 0x18, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57],
    copy: (0, 16),
    resume: 16,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Mode {
    /// Aucun détour.
    Off = 0,
    /// Détours posés, comportement inchangé : on mesure et on compte seulement.
    Count = 1,
    /// Les instantanés sont regroupés.
    Coalesce = 2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Scope {
    /// Seulement pendant une cascade de DeclareWar.
    Cascade = 0,
    /// Aussi pendant chaque passe de Game_Processor.
    Message = 1,
}

static MODE: AtomicU8 = AtomicU8::new(Mode::Coalesce as u8);
static SCOPE: AtomicU8 = AtomicU8::new(Scope::Message as u8);
static TRAMP_SNAPSHOT: AtomicUsize = AtomicUsize::new(0);
static TRAMP_DIPLO: AtomicUsize = AtomicUsize::new(0);
static TRAMP_DECLARE: AtomicUsize = AtomicUsize::new(0);
static TRAMP_STEP: AtomicUsize = AtomicUsize::new(0);
/// Base de l'originale une fois les détours posés, pour les retirer au déchargement.
static HOOKED_BASE: AtomicUsize = AtomicUsize::new(0);
/// Le détour de la passe n'est posé qu'avec `portee=message`.
static STEP_HOOKED: AtomicU8 = AtomicU8::new(0);

fn mode() -> Mode {
    match MODE.load(Ordering::Relaxed) {
        0 => Mode::Off,
        1 => Mode::Count,
        _ => Mode::Coalesce,
    }
}

pub fn set_mode(m: Mode) {
    MODE.store(m as u8, Ordering::Relaxed);
}

fn scope() -> Scope {
    if SCOPE.load(Ordering::Relaxed) == 1 { Scope::Message } else { Scope::Cascade }
}

pub fn set_scope(s: Scope) {
    SCOPE.store(s as u8, Ordering::Relaxed);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
enum Kind {
    City = 0,
    Diplo = 1,
}

#[derive(Default)]
struct Pending {
    depth: u32,
    max_depth: u32,
    start: i64,
    declare_calls: u32,
    calls: [u32; 2],
    /// Triplets (instantané, événement, joueur) dans l'ordre de première apparition.
    pending: Vec<(u8, usize, usize)>,
}

impl Pending {
    fn unique(&self, kind: Kind) -> u32 {
        self.pending.iter().filter(|p| p.0 == kind as u8).count() as u32
    }
}

/// Thread qui a ouvert la portée en cours (0 : aucune). Les autres threads passent sans s'arrêter.
static OWNER: AtomicU32 = AtomicU32::new(0);
static PENDING: Mutex<Pending> =
    Mutex::new(Pending { depth: 0, max_depth: 0, start: 0, declare_calls: 0, calls: [0; 2], pending: Vec::new() });

fn pending() -> std::sync::MutexGuard<'static, Pending> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

type SnapshotFn = unsafe extern "system" fn(usize, usize);
type DeclareWarFn = unsafe extern "system" fn(usize, usize, usize, usize, usize, usize);
type StepFn = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;

unsafe fn call_snapshot(kind: u8, event: usize, player: usize) {
    let tramp = if kind == Kind::City as u8 { &TRAMP_SNAPSHOT } else { &TRAMP_DIPLO };
    let f: SnapshotFn = std::mem::transmute(tramp.load(Ordering::Relaxed));
    f(event, player)
}

unsafe fn request(kind: Kind, event: usize, player: usize) {
    let mut defer = false;
    if OWNER.load(Ordering::Relaxed) == win::GetCurrentThreadId() {
        let mut p = pending();
        p.calls[kind as usize] += 1;
        // Seuls les 32 bits bas comptent : les deux fonctions lisent ECX et EDX.
        let key = (kind as u8, event & 0xffff_ffff, player & 0xffff_ffff);
        if !p.pending.contains(&key) {
            p.pending.push(key);
        }
        defer = mode() == Mode::Coalesce;
    }
    if !defer {
        call_snapshot(kind as u8, event, player);
    }
}

unsafe extern "system" fn hook_snapshot(event: usize, player: usize) {
    request(Kind::City, event, player)
}

unsafe extern "system" fn hook_diplo(event: usize, player: usize) {
    request(Kind::Diplo, event, player)
}

/// Ouvre une portée sur ce thread, ou rend faux si une autre portée est ouverte ailleurs.
unsafe fn enter(declare: bool) -> bool {
    let tid = win::GetCurrentThreadId();
    let owner = OWNER.load(Ordering::Relaxed);
    if owner != 0 && owner != tid {
        return false;
    }
    let mut p = pending();
    if p.depth == 0 {
        *p = Pending { start: win::qpc(), ..Pending::default() };
        OWNER.store(tid, Ordering::Relaxed);
    }
    p.depth += 1;
    p.max_depth = p.max_depth.max(p.depth);
    p.declare_calls += declare as u32;
    true
}

unsafe fn leave() {
    let done = {
        let mut p = pending();
        p.depth -= 1;
        (p.depth == 0).then(|| std::mem::take(&mut *p))
    };
    if let Some(p) = done {
        OWNER.store(0, Ordering::Relaxed);
        finish(p);
    }
}

unsafe extern "system" fn hook_declare_war(this: usize, target: usize, reason: usize, force: usize, a5: usize, a6: usize) {
    if target as u32 == u32::MAX {
        return;
    }
    let f: DeclareWarFn = std::mem::transmute(TRAMP_DECLARE.load(Ordering::Relaxed));
    if !enter(true) {
        // Portée ouverte sur un autre thread : on ne s'en mêle pas.
        return f(this, target, reason, force, a5, a6);
    }
    f(this, target, reason, force, a5, a6);
    leave();
}

/// Passes de `Game_Processor` en cours : `rendements_empire` n'agit qu'à la fin de la plus externe.
static STEP_DEPTH: AtomicU32 = AtomicU32::new(0);

/// Vrai pendant une passe de `Game_Processor` (détour posé).
pub fn in_step() -> bool {
    STEP_DEPTH.load(Ordering::Relaxed) != 0
}

unsafe extern "system" fn hook_step(a: usize, b: usize, c: usize, d: usize) -> usize {
    let f: StepFn = std::mem::transmute(TRAMP_STEP.load(Ordering::Relaxed));
    // Le détour sert aussi à `rendements_empire` : la portée n'est ouverte qu'avec `portee=message`.
    let scoped = mode() != Mode::Off && scope() == Scope::Message && enter(false);
    STEP_DEPTH.fetch_add(1, Ordering::Relaxed);
    let r = f(a, b, c, d);
    if scoped {
        leave();
    }
    if STEP_DEPTH.fetch_sub(1, Ordering::Relaxed) == 1 {
        crate::rendements_empire::after_step();
    }
    r
}

// Statistiques des passes, par rafale : une rafale se termine après une seconde sans passe, et elle
// est journalisée à la fin de la passe suivante.
static BURST: Mutex<[u64; 7]> = Mutex::new([0; 7]); // passes, villes (demandés, uniques), diplo (demandés, uniques), ticks de portée, ticks de rejeu
static LAST: AtomicI64 = AtomicI64::new(0);

fn flush_burst() {
    let b = std::mem::take(&mut *BURST.lock().unwrap_or_else(|e| e.into_inner()));
    if b[0] > 0 && (b[1] > 0 || b[3] > 0) {
        log(&format!(
            "passes mode={:?} portee={:?} passes={} villes_demandees={} villes_uniques={} diplo_demandees={} \
             diplo_uniques={} portee_ms={:.1} rejeu_ms={:.1}",
            mode(),
            scope(),
            b[0],
            b[1],
            b[2],
            b[3],
            b[4],
            win::qpc_ms(b[5] as i64),
            win::qpc_ms(b[6] as i64),
        ));
    }
}

unsafe fn finish(p: Pending) {
    let scope_end = win::qpc();
    if mode() == Mode::Coalesce {
        for &(kind, event, player) in &p.pending {
            call_snapshot(kind, event, player);
        }
    }
    let flush_end = win::qpc();
    if p.declare_calls > 1 || (p.declare_calls == 1 && p.calls[Kind::City as usize] + p.calls[Kind::Diplo as usize] > 0) {
        // Une cascade de déclarations : une ligne à elle, comme avant.
        let players: Vec<String> = p
            .pending
            .iter()
            .map(|&(k, e, pl)| format!("{}{}:{:#x}", if k == Kind::City as u8 { "v" } else { "d" }, pl as i32, e))
            .collect();
        log(&format!(
            "cascade mode={:?} declarewar={} profondeur_max={} instantanes_demandes={} uniques={} \
             diplo_demandes={} diplo_uniques={} cascade_ms={:.1} rejeu_ms={:.1} joueurs=[{}]",
            mode(),
            p.declare_calls,
            p.max_depth,
            p.calls[Kind::City as usize],
            p.unique(Kind::City),
            p.calls[Kind::Diplo as usize],
            p.unique(Kind::Diplo),
            win::qpc_ms(scope_end - p.start),
            win::qpc_ms(flush_end - scope_end),
            players.join(" "),
        ));
        return;
    }
    if p.declare_calls > 0 {
        return; // déclaration isolée, sans instantané
    }
    let last = LAST.swap(flush_end, Ordering::Relaxed);
    if last != 0 && win::qpc_ms(p.start - last) > 1000.0 {
        flush_burst();
    }
    let mut b = BURST.lock().unwrap_or_else(|e| e.into_inner());
    b[0] += 1;
    b[1] += p.calls[Kind::City as usize] as u64;
    b[2] += p.unique(Kind::City) as u64;
    b[3] += p.calls[Kind::Diplo as usize] as u64;
    b[4] += p.unique(Kind::Diplo) as u64;
    b[5] += (scope_end - p.start) as u64;
    b[6] += (flush_end - scope_end) as u64;
}

/// Appelé quand l'originale vient d'être chargée, avant le premier appel relayé.
pub unsafe fn on_gamecore_loaded(base: usize) {
    let refresh = crate::rendements_empire::enabled();
    if mode() == Mode::Off {
        if !refresh {
            log("originale chargée, mode Off : aucun détour");
            return;
        }
        // Seule la passe est détournée, pour `rendements_empire`.
        if let Err(e) = patch::check(base, &[STEP]) {
            log(&format!("originale chargée en {base:#x}, mode Off, passe inattendue, aucun détour : {e}"));
            return;
        }
        match patch::install(base, &STEP, hook_step as *const () as usize) {
            Ok(t) => {
                TRAMP_STEP.store(t, Ordering::Relaxed);
                STEP_HOOKED.store(1, Ordering::Relaxed);
                HOOKED_BASE.store(base, Ordering::Relaxed);
                log(&format!("originale chargée en {base:#x}, mode Off : seule la passe est détournée (rendements d'empire)"));
            }
            Err(e) => log(&format!("échec de la pose du détour de la passe : {e}")),
        }
        return;
    }
    let with_step = scope() == Scope::Message || refresh;
    let sites = [SNAPSHOT, DIPLO, DECLARE_WAR, STEP];
    if let Err(e) = patch::check(base, &sites) {
        log(&format!("originale chargée en {base:#x}, version inattendue, aucun détour : {e}"));
        return;
    }
    let r = (|| {
        TRAMP_SNAPSHOT.store(patch::install(base, &SNAPSHOT, hook_snapshot as *const () as usize)?, Ordering::Relaxed);
        TRAMP_DIPLO.store(patch::install(base, &DIPLO, hook_diplo as *const () as usize)?, Ordering::Relaxed);
        TRAMP_DECLARE.store(patch::install(base, &DECLARE_WAR, hook_declare_war as *const () as usize)?, Ordering::Relaxed);
        if with_step {
            TRAMP_STEP.store(patch::install(base, &STEP, hook_step as *const () as usize)?, Ordering::Relaxed);
            STEP_HOOKED.store(1, Ordering::Relaxed);
        }
        Ok::<(), String>(())
    })();
    // Même en cas d'échec partiel, on retient la base pour retirer au déchargement ce qui a été posé.
    HOOKED_BASE.store(base, Ordering::Relaxed);
    match r {
        Ok(()) => {
            crate::STATUS.fetch_or(crate::ST_HOOKED, Ordering::Relaxed);
            log(&format!("originale chargée en {base:#x}, détours posés, mode {:?}, portée {:?}", mode(), scope()));
        }
        Err(e) => log(&format!("échec de la pose des détours : {e}")),
    }
}

/// Le proxy est déchargé alors que l'originale reste en mémoire : on remet ses octets d'origine.
/// Aucune écriture de fichier ici (on est sous le verrou du chargeur).
pub unsafe fn on_proxy_unload() {
    let base = HOOKED_BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        if STEP_HOOKED.load(Ordering::Relaxed) != 0 {
            patch::restore(base, &STEP);
        }
        for (site, tramp) in [(&DECLARE_WAR, &TRAMP_DECLARE), (&DIPLO, &TRAMP_DIPLO), (&SNAPSHOT, &TRAMP_SNAPSHOT)] {
            if tramp.load(Ordering::Relaxed) != 0 {
                patch::restore(base, site);
            }
        }
    }
}
