//! Une partie chargée calcule comme la partie en cours (`visibilite_chargee=on` et
//! `ecouteurs_charges=on`, par défaut ; GameCore_XP2 1.0.12.72).
//!
//! Une machine qui charge une sauvegarde (rechargement après une désynchro, retour dans la partie)
//! doit retrouver la partie que fait tourner l'hôte. La comparaison d'une partie en cours avec une
//! partie rechargée (contrôle `testSync` juste après le rechargement) a trouvé deux écarts dus au
//! chargement lui-même :
//!
//! - **Améliorations mémorisées** (`visibilite_chargee`). Après un chargement, la première étape de
//!   la passe post-chargement (`18063def0` -> `18025cb50`) écrit chaque amélioration de la carte
//!   dans la couche mémorisée (`Visibility::Instance.m_aExplicitLayers`, `IMPROVEMENTS`) de chaque
//!   joueur qui a révélé sa case. La partie en cours ne l'y écrit que quand le joueur voit la
//!   case : un joueur rechargé se souvenait donc de villages tribaux posés après la révélation, et
//!   les archives de visibilité de quatre joueurs différaient après chaque rechargement. Pendant
//!   cette passe, l'écriture d'amélioration mémorisée (`18035de10`) laisse désormais les couches
//!   des joueurs telles que sauvegardées. L'instance partagée (1000), qu'un chargement ne restaure
//!   pas, est toujours reconstruite par la passe.
//! - **Ordre des écouteurs** (`ecouteurs_charges`). Une nouvelle partie connecte les écouteurs
//!   propres d'un joueur aux événements du jeu (`1802d6c40`) juste après avoir créé le joueur,
//!   avant que ses objets d'IA connectent les leurs ; un chargement les connecte à la fin de la
//!   lecture du joueur (`1803037b0`), après eux. Un événement appelle ses écouteurs dans l'ordre de
//!   la liste : après un chargement, l'IA évaluait une cité-État rencontrée avant que l'émissaire
//!   de la première rencontre soit donné (`AI::Diplomatic.m_diploActionEvaluator` différait).
//!   Pendant un chargement, les écouteurs que cet appel ajoute sont déplacés là où finissait la
//!   liste de chaque événement au début de la lecture de ce joueur. Ceux du gestionnaire des cités
//!   libres (`1801ad4a0`), qu'une nouvelle partie connecte juste après ceux du joueur des cités
//!   libres et un chargement après tous les joueurs, vont là où commençait la lecture du joueur
//!   suivant.
//!
//! La divergence des barbares un tour après un rechargement venait d'ailleurs : du contrôle de
//! synchro du tour chargé, que l'hôte compare à son tour suivant et auquel il répond par des deltas
//! qui écrasent sur place le nouvel arbre de comportement des barbares du client. `rejoin.rs` saute
//! ce contrôle.
//!
//! Les deux changent ce que calcule une machine qui charge : ce sont des correctifs POL, appliqués
//! seulement quand `pol::actifs()` (tous les humains ont ce proxy à la même version, ou partie
//! locale), et seulement si les cinq sites ont les octets attendus. Réglages :
//! `visibilite_chargee=on|off`, `ecouteurs_charges=on|off` (on par défaut tous les deux).

use crate::exe::rd;
use crate::patch::{self, Site, IMAGE_BASE};
use crate::{log, pol};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

// ------------------------------------------------------------------ sites (GameCore_XP2 1.0.12.72)

/// Passe post-chargement (jeu) : exécutée une fois après un chargement, pas après la mise en place
/// d'une nouvelle partie (`18063d720`).
const POST_LOAD: Site = Site {
    name: "post-load pass",
    address: 0x1_8063_def0,
    // mov [rsp+0x10],rbx ; mov [rsp+0x18],rbp ; mov [rsp+0x20],rsi
    expected: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x6C, 0x24, 0x18, 0x48, 0x89, 0x74, 0x24, 0x20],
    copy: (0, 15),
    resume: 15,
};
/// Écriture d'une amélioration mémorisée (gestionnaire de visibilité, joueur, case, amélioration)
/// -> vrai si elle a changé.
const SET_REMEMBERED: Site = Site {
    name: "remembered improvement setter",
    address: 0x1_8035_de10,
    // mov [rsp+8],rbx ; mov [rsp+0x10],rsi ; push rdi ; sub rsp,0x20
    expected: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20],
    copy: (0, 15),
    resume: 15,
};
/// Lecture d'un joueur (flux, joueur) -> flux ; la boucle du chargement (`180305f80`) l'appelle
/// pour chaque joueur.
const PLAYER_READ: Site = Site {
    name: "player read",
    address: 0x1_8030_37b0,
    // mov [rsp+0x20],rbx ; push rbp ; push rsi ; push rdi ; push r12 ; push r13 ; push r14 ; push r15
    expected: &[0x48, 0x89, 0x5C, 0x24, 0x20, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57],
    copy: (0, 16),
    resume: 16,
};
/// Connexion des écouteurs du joueur (objet de mise en place en `Player +0x790`, jeu), détournée
/// après son `test rdx,rdx ; je` : la pile y est encore comme à l'appel.
const PLAYER_CONNECT: Site = Site {
    name: "player listener connects",
    address: 0x1_802d_6c49,
    // mov rax,rsp ; push rbp ; push rsi ; push rdi ; lea rbp,[rax-0x5f] ; sub rsp,0xc0
    expected: &[0x48, 0x8B, 0xC4, 0x55, 0x56, 0x57, 0x48, 0x8D, 0x68, 0xA1, 0x48, 0x81, 0xEC, 0xC0, 0x00, 0x00, 0x00],
    copy: (0, 17),
    resume: 17,
};
/// Connexion des écouteurs du gestionnaire des cités libres (gestionnaire) ; `+0xe8` est le joueur
/// des cités libres. Appelée par sa mise en place de nouvelle partie (`1801ad430`) et par sa
/// lecture (`1801acfd0`).
const FC_CONNECT: Site = Site {
    name: "Free Cities manager listener connects",
    address: 0x1_801a_d4a0,
    // mov [rsp+0x18],rsi ; push rbp ; push rdi ; push r12 ; push r13 ; push r14 ; mov rbp,rsp
    expected: &[0x48, 0x89, 0x74, 0x24, 0x18, 0x55, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x48, 0x8B, 0xEC],
    copy: (0, 16),
    resume: 16,
};
const SITES: [Site; 5] = [POST_LOAD, SET_REMEMBERED, PLAYER_READ, PLAYER_CONNECT, FC_CONNECT];

/// Les joueurs (objet rendu par `180306b80`) : `+0x20` le tableau des `Player*` par id, `+0x28` sa
/// fin.
const PLAYERS: u64 = 0x1_80b8_e140;
/// Le jeu : `+0x280` son objet d'événements, dont les événements sont espacés de 0x18 octets. Un
/// événement porte en `+8` la sentinelle d'une liste circulaire de nœuds (`+0` suivant, `+8`
/// précédent, `+0x50` état, 1 = connecté), appelés dans l'ordre de la liste ; une connexion ajoute
/// à la fin.
const GAME: u64 = 0x1_80b8_aa68;
const GAME_EVENTS: usize = 0x280;
/// L'événement connecté le plus haut en 1.0.12.72 est en `+0x13c8`.
const EVENTS_SCAN: usize = 0x1800;
const FC_PLAYER: usize = 0xe8;
/// L'instance de visibilité partagée par tous les joueurs.
const SHARED_INSTANCE: i32 = 1000;

// ------------------------------------------------------------------ état

static VISIBILITY: AtomicBool = AtomicBool::new(true);
static LISTENERS: AtomicBool = AtomicBool::new(true);
static BASE: AtomicUsize = AtomicUsize::new(0);
static TRAMPS: [AtomicUsize; 5] = [const { AtomicUsize::new(0) }; 5];

static IN_POST_LOAD: AtomicBool = AtomicBool::new(false);
static SKIPPED: AtomicU32 = AtomicU32::new(0);
/// Pendant la lecture d'un joueur : chaque événement (sentinelle, dernier nœud au début de la
/// lecture).
static READ_MARKS: Mutex<Option<Vec<(usize, usize)>>> = Mutex::new(None);
/// Chaque lecture de joueur du chargement en cours, dans l'ordre de lecture : (id du joueur, ses
/// repères).
static PLAYER_MARKS: Mutex<Vec<(i32, Vec<(usize, usize)>)>> = Mutex::new(Vec::new());
/// Depuis la dernière passe post-chargement, joueurs puis gestionnaire des cités libres : appels
/// qui ont déplacé quelque chose, écouteurs déplacés, listes changées, listes laissées telles
/// quelles.
static STATS: Mutex<[[u32; 4]; 2]> = Mutex::new([[0; 4]; 2]);

pub fn set_visibility(on: bool) {
    VISIBILITY.store(on, Ordering::Relaxed);
}

pub fn set_listeners(on: bool) {
    LISTENERS.store(on, Ordering::Relaxed);
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn tramp(i: usize) -> usize {
    TRAMPS[i].load(Ordering::Relaxed)
}

fn global(address: u64) -> usize {
    BASE.load(Ordering::Relaxed) + (address - IMAGE_BASE) as usize
}

#[repr(C)]
#[derive(Default)]
struct MemInfo {
    base: usize,
    alloc_base: usize,
    alloc_protect: u32,
    partition: u32,
    size: usize,
    state: u32,
    protect: u32,
    kind: u32,
    pad: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn VirtualQuery(at: *const c_void, info: *mut MemInfo, size: usize) -> usize;
}

/// Vrai si `[at, at+len)` est validée (MEM_COMMIT) et lisible en ce moment. Une plage qui déborde
/// (un -1 lu comme pointeur) ne l'est pas.
unsafe fn readable(at: usize, len: usize) -> bool {
    let Some(end) = at.checked_add(len) else { return false };
    if at < 0x10000 || end > 0x8000_0000_0000 {
        return false;
    }
    let mut p = at;
    while p < end {
        let mut m = MemInfo::default();
        if VirtualQuery(p as *const c_void, &mut m, std::mem::size_of::<MemInfo>()) == 0 {
            return false;
        }
        // MEM_COMMIT, ni PAGE_NOACCESS (1) ni PAGE_GUARD (0x100).
        if m.state != 0x1000 || m.protect == 0 || m.protect & 0x101 != 0 {
            return false;
        }
        p = m.base + m.size;
    }
    true
}

/// L'id du joueur en `p`, -1 si `p` n'est pas dans le tableau des joueurs.
unsafe fn player_id(p: usize) -> i32 {
    let mgr: usize = rd(global(PLAYERS));
    if p == 0 || mgr == 0 || !readable(mgr + 0x20, 16) {
        return -1;
    }
    let (begin, end): (usize, usize) = (rd(mgr + 0x20), rd(mgr + 0x28));
    let n = if end > begin { ((end - begin) / 8).min(64) } else { 0 };
    (0..n).find(|i| readable(begin + i * 8, 8) && rd::<usize>(begin + i * 8) == p).map_or(-1, |i| i as i32)
}

// ------------------------------------------------------------------ améliorations mémorisées

type PostLoadFn = unsafe extern "system-unwind" fn(usize) -> u64;
type SetRememberedFn = unsafe extern "system-unwind" fn(usize, i32, u32, u32) -> u64;

unsafe extern "system-unwind" fn post_load_hook(game: usize) -> u64 {
    let orig: PostLoadFn = std::mem::transmute(tramp(0));
    let pol = pol::actifs();
    let vis = pol && VISIBILITY.load(Ordering::Relaxed);
    SKIPPED.store(0, Ordering::Relaxed);
    IN_POST_LOAD.store(vis, Ordering::SeqCst);
    let r = orig(game);
    IN_POST_LOAD.store(false, Ordering::SeqCst);
    if vis {
        log(&format!(
            "loaded game: post-load pass done, {} remembered-improvement write(s) of the players skipped",
            SKIPPED.load(Ordering::Relaxed)
        ));
    }
    let st = std::mem::take(&mut *lock(&STATS));
    if pol && LISTENERS.load(Ordering::Relaxed) {
        log(&format!(
            "loaded game: listeners of {} player(s) moved to where their read started ({} in {} list(s), {} list(s) left as they were); Free Cities manager: {} in {} list(s), {} left",
            st[0][0], st[0][1], st[0][2], st[0][3], st[1][1], st[1][2], st[1][3]
        ));
    }
    lock(&PLAYER_MARKS).clear();
    r
}

unsafe extern "system-unwind" fn set_remembered_hook(mgr: usize, player: i32, plot: u32, value: u32) -> u64 {
    if player != SHARED_INSTANCE && IN_POST_LOAD.load(Ordering::SeqCst) {
        SKIPPED.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    let orig: SetRememberedFn = std::mem::transmute(tramp(1));
    orig(mgr, player, plot, value)
}

// ------------------------------------------------------------------ ordre des écouteurs

type PlayerReadFn = unsafe extern "system-unwind" fn(usize, usize, usize, usize) -> usize;
type ConnectFn = unsafe extern "system-unwind" fn(usize, usize, usize, usize);

/// Chaque événement de l'objet d'événements du jeu qui se lit comme tel : (sentinelle, dernier
/// nœud).
unsafe fn event_tails() -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let game: usize = rd(global(GAME));
    if game == 0 || !readable(game + GAME_EVENTS, 8) {
        return out;
    }
    let events: usize = rd(game + GAME_EVENTS);
    for off in (0..EVENTS_SCAN).step_by(0x18) {
        let event = events + off;
        if !readable(event, 0x18) {
            break;
        }
        let sentinel: usize = rd(event + 8);
        if sentinel == 0 || !readable(sentinel, 0x58) {
            continue;
        }
        let last: usize = rd(sentinel + 8);
        if last != 0 && readable(last, 0x58) && rd::<usize>(last) == sentinel {
            out.push((sentinel, last));
        }
    }
    out
}

unsafe extern "system-unwind" fn player_read_hook(stream: usize, player: usize, r8: usize, r9: usize) -> usize {
    let orig: PlayerReadFn = std::mem::transmute(tramp(2));
    if !(LISTENERS.load(Ordering::Relaxed) && pol::actifs()) {
        return orig(stream, player, r8, r9);
    }
    let marks = event_tails();
    let id = player_id(player);
    if id >= 0 {
        lock(&PLAYER_MARKS).push((id, marks.clone()));
    }
    *lock(&READ_MARKS) = Some(marks);
    let r = orig(stream, player, r8, r9);
    *lock(&READ_MARKS) = None;
    r
}

unsafe extern "system-unwind" fn player_connect_hook(setup: usize, game: usize, r8: usize, r9: usize) {
    let orig: ConnectFn = std::mem::transmute(tramp(3));
    let Some(marks) = lock(&READ_MARKS).clone() else {
        return orig(setup, game, r8, r9);
    };
    let before: Vec<usize> = marks.iter().map(|&(s, _)| rd(s + 8)).collect();
    orig(setup, game, r8, r9);
    move_after_marks(&marks, &before, 0);
}

unsafe extern "system-unwind" fn fc_connect_hook(mgr: usize, rdx: usize, r8: usize, r9: usize) {
    let orig: ConnectFn = std::mem::transmute(tramp(4));
    let id = if readable(mgr + FC_PLAYER, 8) { player_id(rd(mgr + FC_PLAYER)) } else { -1 };
    // Les repères de la lecture qui suit celle du joueur des cités libres (aucun dans une nouvelle
    // partie, ou quand ce joueur a été lu en dernier : les écouteurs restent à la fin).
    let marks = {
        let all = lock(&PLAYER_MARKS);
        all.iter().position(|(p, _)| *p == id).and_then(|i| all.get(i + 1)).map(|(_, m)| m.clone())
    };
    let Some(marks) = marks else {
        return orig(mgr, rdx, r8, r9);
    };
    let before: Vec<usize> = marks.iter().map(|&(s, _)| rd(s + 8)).collect();
    orig(mgr, rdx, r8, r9);
    move_after_marks(&marks, &before, 1);
}

/// Pour chaque événement de `marks` (sentinelle, repère) : déplace les nœuds ajoutés depuis
/// `before` (le dernier nœud de la liste à ce moment) juste après le repère, si le repère et ce
/// nœud sont encore dans la liste (le repère d'abord) et si chaque nouveau nœud est simplement
/// connecté (aucune émission en cours ne le tient). Compte dans `STATS[group]`.
unsafe fn move_after_marks(marks: &[(usize, usize)], before: &[usize], group: usize) {
    let mut stats = lock(&STATS);
    let st = &mut stats[group];
    let mut moved_here = 0;
    for (&(sentinel, mark), &old_tail) in marks.iter().zip(before.iter()) {
        let tail: usize = rd(sentinel + 8);
        if tail == old_tail || mark == old_tail {
            continue; // rien d'ajouté, ou rien de connecté depuis le repère : déjà en place
        }
        let (mut mark_seen, mut old_seen) = (mark == sentinel, false);
        let mut node: usize = rd(sentinel);
        let mut steps = 0;
        while node != sentinel && steps < 100_000 && readable(node, 0x58) {
            if node == mark && !old_seen {
                mark_seen = true;
            }
            if node == old_tail {
                old_seen = true;
                break;
            }
            node = rd(node);
            steps += 1;
        }
        let mut plain = mark_seen && old_seen;
        let first: usize = if plain { rd(old_tail) } else { 0 };
        let mut n = 0;
        if plain {
            let mut k = first;
            n = 1;
            plain = readable(k, 0x58) && rd::<i32>(k + 0x50) == 1;
            while plain && k != tail && n < 4096 {
                k = rd(k);
                n += 1;
                plain = readable(k, 0x58) && rd::<i32>(k + 0x50) == 1;
            }
            plain &= k == tail;
        }
        if !plain {
            if st[3] < 3 {
                log(&format!(
                    "loaded game: event list at {sentinel:#x} left as it was (mark {}, last node before {}, {n} new node(s))",
                    if mark_seen { "found" } else { "gone" },
                    if old_seen { "found" } else { "gone" }
                ));
            }
            st[3] += 1;
            continue;
        }
        // Détache first..tail de la fin, puis le remet après le repère (`+0` suivant, `+8`
        // précédent).
        std::ptr::write(old_tail as *mut usize, sentinel);
        std::ptr::write((sentinel + 8) as *mut usize, old_tail);
        let after: usize = rd(mark);
        std::ptr::write(mark as *mut usize, first);
        std::ptr::write((first + 8) as *mut usize, mark);
        std::ptr::write(tail as *mut usize, after);
        std::ptr::write((after + 8) as *mut usize, tail);
        st[1] += n;
        st[2] += 1;
        moved_here += 1;
    }
    if moved_here > 0 {
        st[0] += 1;
    }
}

// ------------------------------------------------------------------ pose

pub unsafe fn on_gamecore_loaded(base: usize) {
    if !VISIBILITY.load(Ordering::Relaxed) && !LISTENERS.load(Ordering::Relaxed) {
        log("loaded game: off (visibilite_chargee=off, ecouteurs_charges=off), no hooks");
        return;
    }
    if let Err(e) = patch::check(base, &SITES) {
        log(&format!("loaded game: unexpected game version, no hooks (POL fix unavailable): {e}"));
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    let hooks: [usize; 5] = [
        post_load_hook as *const () as usize,
        set_remembered_hook as *const () as usize,
        player_read_hook as *const () as usize,
        player_connect_hook as *const () as usize,
        fc_connect_hook as *const () as usize,
    ];
    // Les trampolines d'abord, les détours ensuite : un détour peut appeler le trampoline d'un
    // autre.
    for (i, s) in SITES.iter().enumerate() {
        match patch::install(base, s, hooks[i]) {
            Ok(t) => TRAMPS[i].store(t, Ordering::Relaxed),
            Err(e) => {
                log(&format!("loaded game: failed to hook {}: {e}; hooks removed", s.name));
                for s in &SITES[..i] {
                    patch::restore(base, s);
                }
                BASE.store(0, Ordering::Relaxed);
                return;
            }
        }
    }
    *lock(&READ_MARKS) = None;
    lock(&PLAYER_MARKS).clear();
    *lock(&STATS) = [[0; 4]; 2];
    crate::STATUS.fetch_or(crate::ST_POL, Ordering::Relaxed);
    log(&format!(
        "loaded game: 5 hooks installed (visibility {}, listeners {})",
        if VISIBILITY.load(Ordering::Relaxed) { "on" } else { "off" },
        if LISTENERS.load(Ordering::Relaxed) { "on" } else { "off" }
    ));
}

pub unsafe fn on_proxy_unload() {
    let base = BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        for s in &SITES {
            patch::restore(base, s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque site a ses octets attendus dans le GameCore installé (test sauté sans lui).
    /// `CIV6_DIR` remplace le dossier du jeu.
    #[test]
    fn sites_match_installed_gamecore() {
        let dirs = [
            std::env::var("CIV6_DIR").unwrap_or_default(),
            r"D:\SteamLibrary\steamapps\common\Sid Meier's Civilization VI".to_string(),
            r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI".to_string(),
        ];
        let found = dirs.iter().filter(|d| !d.is_empty()).find_map(|d| {
            std::fs::read(format!(r"{d}\Base\Binaries\Win64Steam\GameCore_XP2_FinalRelease.dll")).ok()
        });
        let Some(file) = found else {
            eprintln!("GameCore missing, test skipped");
            return;
        };
        let rd32 = |o: usize| u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
        let rd16 = |o: usize| u16::from_le_bytes(file[o..o + 2].try_into().unwrap()) as usize;
        let nt = rd32(0x3c);
        let sections = nt + 24 + rd16(nt + 20);
        for s in &SITES {
            let rva = (s.address - IMAGE_BASE) as usize;
            let o = (0..rd16(nt + 6))
                .map(|i| sections + 40 * i)
                .find_map(|h| {
                    let (va, size, raw) = (rd32(h + 12), rd32(h + 16), rd32(h + 20));
                    (rva >= va && rva < va + size).then(|| rva - va + raw)
                })
                .unwrap_or_else(|| panic!("{}: not in the file", s.name));
            assert_eq!(&file[o..o + s.expected.len()], s.expected, "{}", s.name);
            assert!(s.expected.len() >= 14 && s.copy.1 <= s.expected.len() && s.resume >= s.copy.1, "{}", s.name);
        }
    }
}
