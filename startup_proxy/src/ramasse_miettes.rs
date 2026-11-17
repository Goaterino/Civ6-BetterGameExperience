//! Destruction de l'interface : un seul ramasse-miettes Lua complet par arbre de contextes détruit,
//! au lieu d'un par fonction Lua libérée (`gc_en_lot=on`, par défaut).
//!
//! Ce que fait le jeu (`analyses/chargement/README.md`, « 5. Les 3 s avant le remap » ; exe DX12
//! du 2026-11-05) : chaque poignée de fonction Lua libérée (`1404d91b0` → `140982730` → entrée
//! +0x18 du gestionnaire, `140980470`) fait
//! ```c
//! lua_cpcall(L, nettoyage, poignée);
//! lua_gc(L, LUA_GCCOLLECT, 0);   // complet, sur tout l'état Lua partagé
//! ```
//! Quand un contexte racine de l'interface est arrêté (`140080990`, `14008ad40` : sortie de partie,
//! remap, retour au menu), son arrêt (`1408f0710`, entrée +0x1a8 des contextes) arrête et détruit
//! récursivement tous ses enfants, et chacun libère une douzaine de poignées : autant de
//! ramasse-miettes complets. Avant le remap, 2 340 échantillons du thread principal en viennent.
//!
//! Le détour :
//! - `1408f0710` est remplacée (elle ne fait qu'appeler `1408f0120` puis sauter à
//!   `ContextBase::Shutdown`) par la même chose, encadrée d'un compteur de profondeur ;
//! - l'appel de `lua_gc` dans `140980470` passe par `gc` : pendant un arrêt de contexte (profondeur
//!   > 0, sur ce thread), une collecte complète est notée au lieu d'être faite ;
//! - à la sortie de l'arrêt le plus externe, une seule collecte complète par état noté.
//!
//! L'ordre des finaliseurs est celui d'une collecte complète, seulement plus tard dans le même
//! arrêt. Interface seulement, sans effet sur la synchro.
//!
//! Seulement pour l'arrêt de l'interface d'une partie (sortie, remap), sur le thread principal :
//! entre la fin d'un chargement (`LoadGameViewState::End`, vsync.rs) et le premier arrêt d'au
//! moins 20 collectes. Au lancement d'une partie depuis le salon, l'arrêt des 30 contextes du salon
//! avec la collecte reportée laissait le `OnTick` de `StagingRoom.lua` (MPH) tourner sur un
//! contexte déjà nettoyé (« function expected instead of nil », 46 à 208 fois par capture depuis
//! G1), et une fois écrire dans de la mémoire libérée de HavokScript (crash du 2026-11-17,
//! `20261117-145241-vsync-levier`). Cet arrêt-là reste celui du jeu ; le mécanisme exact n'est pas
//! établi (l'arrêt tourne bien sur le thread principal, `20261117-154532-leviers-4`). Sans les
//! détours de vsync.rs, G1 ne reporte plus rien.

use crate::{log, motifs};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// Dans `140980470` : `lea rdx,[nettoyage] ; mov r8,rdi ; call lua_cpcall ; mov rcx,[rbx+10h] ;
/// xor r8d,r8d ; lea edx,[r8+2] ; call lua_gc ; call … ; cmp qword [rbx+38h],0`. `lua_gc` en +0x1a.
const SITE_GC: &str = "48 8D 15 ?? ?? ?? ?? 4C 8B C7 E8 ?? ?? ?? ?? 48 8B 4B 10 45 33 C0 41 8D 50 02 E8 ?? ?? ?? ?? E8 ?? ?? ?? ?? 48 83 7B 38 00";
const SITE_GC_APPEL: usize = 0x1a;
/// Le jeu fait chaque collecte sous le verrou de l'état Lua partagé : `call verrou` juste avant le
/// motif (`call … ; mov rcx,[rbx+10h]`, en -9), `call deverrou` juste après `lua_gc` (+0x1f). La
/// collecte différée les reprend : sans eux, un autre thread qui exécute du Lua au même moment
/// corrompt le tas (crash du 2026-11-17 dans HavokScript au chargement d'une sauvegarde).
const SITE_VERROU: usize = 9;
const SITE_DEVERROU: usize = 0x1f;
/// Début de `1408f0120` (arrêt d'un contexte) : `mov [rsp+20h],rbx ; push r14 ; sub rsp,30h ;
/// test dword [rcx+70h],8000000h ; mov rbx,rcx ; mov [rsp+50h],rdi ; je ; add rcx,0BB0h`.
const ARRET_INTERNE: &str = "48 89 5C 24 20 41 56 48 83 EC 30 F7 41 70 00 00 00 08 48 8B D9 48 89 7C 24 50 74 ?? 48 81 C1 B0 0B 00 00";
/// `1408f0710` : `push rbx ; sub rsp,20h ; mov rbx,rcx ; call 1408f0120 ; mov rcx,rbx ;
/// add rsp,20h ; pop rbx ; jmp ContextBase::Shutdown`. Plusieurs fonctions ont cette forme : on
/// garde celle qui appelle `ARRET_INTERNE`.
const ARRET: &str = "40 53 48 83 EC 20 48 8B D9 E8 ?? ?? ?? ?? 48 8B CB 48 83 C4 20 5B E9 ?? ?? ?? ??";
const LUA_GCCOLLECT: i32 = 2;

static LUA_GC: AtomicUsize = AtomicUsize::new(0);
static VERROU: AtomicUsize = AtomicUsize::new(0);
static DEVERROU: AtomicUsize = AtomicUsize::new(0);
static INTERNE: AtomicUsize = AtomicUsize::new(0);
static SHUTDOWN: AtomicUsize = AtomicUsize::new(0);
static EVITES: AtomicU64 = AtomicU64::new(0);
/// Le thread principal : celui de DllMain, qui charge le proxy au lancement de l'exe.
static PRINCIPAL: AtomicU32 = AtomicU32::new(0);
/// Une partie est affichée : vrai à la fin d'un chargement (`LoadGameViewState::End`, vsync.rs),
/// faux après l'arrêt de l'interface de la partie (au moins `SORTIE_DE_PARTIE` collectes).
static EN_PARTIE: AtomicBool = AtomicBool::new(false);
const SORTIE_DE_PARTIE: u64 = 20;

/// Fin d'un chargement : l'interface de la partie est en place.
pub fn fin_de_chargement() {
    EN_PARTIE.store(true, Ordering::Release);
}

thread_local! {
    static PROFONDEUR: Cell<u32> = const { Cell::new(0) };
    static EN_ATTENTE: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

type LuaGc = unsafe extern "C" fn(*mut c_void, i32, i32) -> i32;

unsafe extern "C" fn gc(l: *mut c_void, what: i32, data: i32) -> i32 {
    let real: LuaGc = std::mem::transmute(LUA_GC.load(Ordering::Relaxed));
    if what == LUA_GCCOLLECT && PROFONDEUR.with(Cell::get) > 0 {
        EN_ATTENTE.with(|v| {
            let mut v = v.borrow_mut();
            if !v.contains(&(l as usize)) {
                v.push(l as usize);
            }
        });
        EVITES.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    real(l, what, data)
}

unsafe extern "C" fn arret(ctx: *mut c_void) {
    let interne: unsafe extern "C" fn(*mut c_void) = std::mem::transmute(INTERNE.load(Ordering::Relaxed));
    let shutdown: unsafe extern "C" fn(*mut c_void) = std::mem::transmute(SHUTDOWN.load(Ordering::Relaxed));
    let racine = PROFONDEUR.with(Cell::get) == 0;
    if racine && (crate::win::GetCurrentThreadId() != PRINCIPAL.load(Ordering::Relaxed) || !EN_PARTIE.load(Ordering::Acquire)) {
        interne(ctx);
        shutdown(ctx);
        return;
    }
    PROFONDEUR.with(|p| p.set(p.get() + 1));
    interne(ctx);
    shutdown(ctx);
    let externe = PROFONDEUR.with(|p| {
        p.set(p.get() - 1);
        p.get() == 0
    });
    if externe {
        let etats = EN_ATTENTE.with(|v| std::mem::take(&mut *v.borrow_mut()));
        let real: LuaGc = std::mem::transmute(LUA_GC.load(Ordering::Relaxed));
        let evites = EVITES.swap(0, Ordering::Relaxed);
        if evites >= SORTIE_DE_PARTIE {
            // L'interface de la partie est fermée : jusqu'à la fin du prochain chargement, les
            // arrêts restent ceux du jeu (le salon).
            EN_PARTIE.store(false, Ordering::Release);
        }
        if evites > 0 {
            log(&format!("lua gc: context tree stopped, {evites} full collections replaced by {}", etats.len()));
        }
        if !etats.is_empty() {
            let verrou: unsafe extern "C" fn() = std::mem::transmute(VERROU.load(Ordering::Relaxed));
            let deverrou: unsafe extern "C" fn() = std::mem::transmute(DEVERROU.load(Ordering::Relaxed));
            verrou();
            for l in etats {
                real(l as *mut c_void, LUA_GCCOLLECT, 0);
            }
            deverrou();
        }
    }
}

/// Le réglage `gc_en_lot` (dernière occurrence ; `on` par défaut).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("gc_en_lot=")).last().map(str::trim) != Some("off")
}

pub fn install(exe: usize) {
    PRINCIPAL.store(unsafe { crate::win::GetCurrentThreadId() }, Ordering::Relaxed);
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("lua gc: not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log("lua gc: one full collection per release (gc_en_lot=off)");
        return;
    }
    unsafe {
        let sites = motifs::chercher(exe, &motifs::motif(SITE_GC));
        let interne = motifs::chercher(exe, &motifs::motif(ARRET_INTERNE));
        let ([site], [interne]) = (&sites[..], &interne[..]) else {
            log(&format!("lua gc: not installed ({} lua_gc sites, {} context stops found)", sites.len(), interne.len()));
            return;
        };
        let arrets: Vec<usize> =
            motifs::chercher(exe, &motifs::motif(ARRET)).into_iter().filter(|&a| motifs::cible_rel32(a + 9) == *interne).collect();
        let [arret_at] = arrets[..] else {
            log(&format!("lua gc: not installed ({} context stop wrappers found)", arrets.len()));
            return;
        };
        let (v, d) = (site - SITE_VERROU, site + SITE_DEVERROU);
        if crate::exe::rd::<u8>(v) != 0xe8 || crate::exe::rd::<[u8; 4]>(v + 5) != [0x48, 0x8b, 0x4b, 0x10] || crate::exe::rd::<u8>(d) != 0xe8 {
            log("lua gc: not installed (Lua state lock calls not found around lua_gc)");
            return;
        }
        VERROU.store(motifs::cible_rel32(v), Ordering::Relaxed);
        DEVERROU.store(motifs::cible_rel32(d), Ordering::Relaxed);
        SHUTDOWN.store(motifs::cible_rel32(arret_at + 22), Ordering::Relaxed);
        INTERNE.store(*interne, Ordering::Relaxed);
        let appel = site + SITE_GC_APPEL;
        match motifs::rediriger_appel(appel, gc as *const () as usize) {
            Ok(ancienne) => LUA_GC.store(ancienne, Ordering::Relaxed),
            Err(e) => {
                log(&format!("lua gc: not installed: {e}"));
                return;
            }
        }
        // La fonction entière est remplacée : les 14 octets du saut couvrent jusqu'à l'appel
        // compris, et `arret` refait elle-même les deux appels.
        let bytes: [u8; 27] = crate::exe::rd(arret_at);
        if let Err(e) = crate::patch::install_at("arrêt de contexte", arret_at, &[], arret_at + bytes.len(), arret as *const () as usize) {
            // L'appel de lua_gc est déjà redirigé : sans compteur de profondeur, `gc` laisse tout
            // passer, le comportement reste celui du jeu.
            log(&format!("lua gc: context stop not hooked: {e}"));
            return;
        }
        log("lua gc: one full collection per context tree stop");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglage() {
        assert!(enabled(""));
        assert!(!enabled("gc_en_lot=off"));
    }
}
