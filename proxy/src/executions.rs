//! Compte les messages que GameCore exécute, pour le cache des hachages de l'hôte au contrôle de
//! synchro (`cache_synchro` du proxy de démarrage). Ne change rien au jeu : un compteur est
//! augmenté avant et après chaque exécution.
//!
//! **Le site.** GameCore n'exécute un message que dans sa fonction de réception
//! (`Messages::Manager::ReceiveMessage`, `Messages_Manager.cpp`), celle qui écrit
//! « GameCore RECV (%i): %s » puis appelle l'`Execute` du message en levant l'octet
//! « message en cours ». Elle a deux appelants, dans GameCore 2020 comme en 1.0.12.58 :
//! - la boucle qui vide la file des messages (`ProcessMessages`), par un `call` ;
//! - la méthode du contexte appelée par l'exe pour un message à exécuter tout de suite, par un
//!   `jmp` après `mov ecx,edx ; mov rdx,r8`.
//!
//! `BroadcastMessage` (« GameCore SEND ») n'exécute rien : il remet le message à l'exe, qui le
//! renvoie par l'un de ces deux chemins, à l'hôte compris.
//!
//! **Recherche.** Sans table d'adresses : la chaîne, unique dans `.rdata`, la fonction qui la
//! désigne (table des exceptions), puis exactement un `call` et un `jmp rel32` vers elle, le `jmp`
//! précédé de son brassage d'arguments. Le moindre écart, et rien n'est posé. Les deux appels sont
//! redirigés vers `reception`, qui augmente le compteur, appelle la fonction du jeu et l'augmente de
//! nouveau.
//!
//! **Le compteur** est celui du proxy de démarrage (export `BgdMessagesGameCore`). Sans lui, rien
//! n'est posé. Le proxy de démarrage ne sert son cache que pendant que ce module le dit posé
//! (`BgdSuiviMessages`) : une installation ratée, ou le déchargement de ce proxy, coupe le cache.

use crate::patch::{alloc_near, rel32, write_rel32};
use crate::pe::Image;
use crate::{log, win};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

const RECV_FORMAT: &[u8] = b"GameCore RECV (%i): %s\r\n";
/// `mov ecx,edx ; mov rdx,r8` : la méthode du contexte avant son `jmp`.
const THUNK_BEFORE: [u8; 5] = [0x8b, 0xca, 0x49, 0x8b, 0xd0];

/// Ce que la recherche a trouvé, en adresses relatives.
#[derive(Debug, PartialEq)]
pub struct Sites {
    pub receive: u32,
    pub call: u32,
    pub jump: u32,
}

pub fn locate(img: &Image) -> Result<Sites, String> {
    let receive = img.function_of_string(RECV_FORMAT)?;
    let sites = img.rel32_sites(receive);
    let calls: Vec<u32> = sites.iter().filter(|s| s.1 == 0xe8).map(|s| s.0).collect();
    let jumps: Vec<u32> = sites.iter().filter(|s| s.1 == 0xe9).map(|s| s.0).collect();
    let ([call], [jump]) = (&calls[..], &jumps[..]) else {
        return Err(format!("réception {receive:#x} : {} call et {} jmp au lieu de 1 et 1", calls.len(), jumps.len()));
    };
    if img.at(jump - THUNK_BEFORE.len() as u32, THUNK_BEFORE.len()) != Some(&THUNK_BEFORE[..]) {
        return Err(format!("méthode du contexte {jump:#x} : octets inattendus"));
    }
    Ok(Sites { receive, call: *call, jump: *jump })
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);
static RECEIVE: AtomicUsize = AtomicUsize::new(0);
/// Sites redirigés (adresses), pour le déchargement.
static CALL_SITE: AtomicUsize = AtomicUsize::new(0);
static JUMP_SITE: AtomicUsize = AtomicUsize::new(0);
static TRACKED: AtomicUsize = AtomicUsize::new(0);

type Receive = unsafe extern "system-unwind" fn(i32, usize);
type Counter = unsafe extern "system" fn() -> *const AtomicU64;
type Tracked = unsafe extern "system" fn(u32);

/// Une génération nouvelle : aussi à la création et à la destruction d'un contexte de partie.
pub fn bump() {
    let c = COUNTER.load(Ordering::Acquire);
    if c != 0 {
        unsafe { &*(c as *const AtomicU64) }.fetch_add(1, Ordering::SeqCst);
    }
}

/// Remplace les deux appels à la réception de GameCore.
unsafe extern "system-unwind" fn reception(sender: i32, stream: usize) {
    let original: Receive = std::mem::transmute(RECEIVE.load(Ordering::Relaxed));
    bump();
    original(sender, stream);
    bump();
}

unsafe fn set_tracked(on: bool) {
    let f = TRACKED.load(Ordering::Relaxed);
    if f != 0 {
        let f: Tracked = std::mem::transmute(f);
        f(on as u32);
    }
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    let m = crate::pol::startup_module();
    let (counter, tracked) = if m.is_null() {
        (std::ptr::null_mut(), std::ptr::null_mut())
    } else {
        (win::GetProcAddress(m, b"BgdMessagesGameCore\0".as_ptr()), win::GetProcAddress(m, b"BgdSuiviMessages\0".as_ptr()))
    };
    if counter.is_null() || tracked.is_null() {
        log("executions : proxy de démarrage sans cache des contrôles de synchro, rien n'est posé");
        return;
    }
    let Some(img) = Image::mapped(base) else { return };
    let sites = match locate(&img) {
        Ok(s) => s,
        Err(e) => {
            log(&format!("executions : GameCore inattendu ({e}), rien n'est posé, le cache des contrôles de synchro reste coupé"));
            return;
        }
    };
    let Some(page) = alloc_near(base) else {
        log("executions : pas de mémoire à moins de 2 Go de GameCore, rien n'est posé");
        return;
    };
    // Relais : jmp [rip+0] ; reception.
    let mut code = vec![0xff, 0x25, 0, 0, 0, 0];
    code.extend_from_slice(&(reception as Receive as usize as u64).to_le_bytes());
    std::ptr::copy_nonoverlapping(code.as_ptr(), page as *mut u8, code.len());
    let (call, jump) = (base + sites.call as usize, base + sites.jump as usize);
    let (Some(r_call), Some(r_jump)) = (rel32(call, page), rel32(jump, page)) else {
        log("executions : relais hors de portée, rien n'est posé");
        return;
    };
    let counter: Counter = std::mem::transmute(counter);
    COUNTER.store(counter() as usize, Ordering::Release);
    TRACKED.store(tracked as usize, Ordering::Relaxed);
    RECEIVE.store(base + sites.receive as usize, Ordering::Relaxed);
    write_rel32(call, r_call);
    write_rel32(jump, r_jump);
    CALL_SITE.store(call, Ordering::Relaxed);
    JUMP_SITE.store(jump, Ordering::Relaxed);
    // Posé : le cache peut servir, à partir d'une génération nouvelle.
    bump();
    set_tracked(true);
    log(&format!("executions : messages exécutés par GameCore comptés (réception {:#x})", sites.receive as u64 + crate::patch::IMAGE_BASE));
}

/// Le proxy est déchargé, l'originale reste : le cache est coupé, puis les appels remis.
pub unsafe fn on_proxy_unload() {
    let receive = RECEIVE.load(Ordering::Relaxed);
    if receive == 0 {
        return;
    }
    set_tracked(false);
    bump();
    for site in [CALL_SITE.swap(0, Ordering::Relaxed), JUMP_SITE.swap(0, Ordering::Relaxed)] {
        if let (true, Some(rel)) = (site != 0, rel32(site, receive)) {
            write_rel32(site, rel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GameCore 1.0.12.58 (`CIV6_EXE_58_DIR`, et ceux du 2023-11-11 et du 2024-06-26 dans
    /// `binaires_jeu`), celui de l'installation (1.0.12.74), et celui de 2020 (`binaires/`, Linux) :
    /// la recherche y trouve la réception et ses deux appelants.
    #[test]
    fn sites_in_known_gamecores() {
        let installed = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\DLC\Expansion2\Binaries\Win64";
        let mut dirs: Vec<String> = vec![installed.to_string(), concat!(env!("CARGO_MANIFEST_DIR"), "/../binaires").to_string()];
        dirs.extend(std::env::var("CIV6_EXE_58_DIR"));
        dirs.push(concat!(env!("CARGO_MANIFEST_DIR"), "/../binaires_jeu/1.0.12.58").to_string());
    dirs.push(concat!(env!("CARGO_MANIFEST_DIR"), "/../binaires_jeu/1.0.12.58_gamecore_2024-06-26").to_string());
    dirs.push(concat!(env!("CARGO_MANIFEST_DIR"), "/../binaires_jeu/1.0.12.74").to_string());
        let mut seen = 0;
        for dir in dirs {
            let path = std::path::Path::new(&dir).join(crate::noms::GAMECORE);
            let Ok(file) = std::fs::read(&path) else { continue };
            seen += 1;
            let img = Image::file(&file).unwrap();
            let s = locate(&img).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            eprintln!("{}: {s:x?}", path.display());
            // Adresses relevées (relatives) : 2020, 1.0.12.58 (même code que le GameCore du
            // 2024-06-26), 1.0.12.74.
            let nt = u32::from_le_bytes(file[0x3c..0x40].try_into().unwrap()) as usize;
            let known = match u32::from_le_bytes(file[nt + 8..nt + 12].try_into().unwrap()) {
                0x5ee1_0d21 => Some(Sites { receive: 0x33080, call: 0x32883, jump: 0x57_0415 }),
                0x654e_cd4e | 0x667c_6f5b => Some(Sites { receive: 0x37b50, call: 0x37353, jump: 0x59_0825 }),
                0x6af3_bcb5 => Some(Sites { receive: 0x37b50, call: 0x37353, jump: 0x59_0865 }),
                _ => None,
            };
            if let Some(k) = known {
                assert_eq!(s, k, "{}", path.display());
            }
        }
        if seen == 0 {
            eprintln!("aucun GameCore trouvé, test sauté");
        }
    }
}
