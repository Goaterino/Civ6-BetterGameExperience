//! Chargement du proxy de GameCore : l'exe charge `GameCore_XP2_FinalRelease.dll` par
//! `LoadLibraryW` ou `LoadLibraryA` (ses deux imports de chargement) au lancement d'une partie
//! Gathering Storm. Les deux entrées de sa table d'imports sont détournées : si le fichier demandé
//! est l'original et que `BGD_GameCore_XP2.dll` est posé dans le même dossier, c'est lui qui est
//! chargé, et il charge l'original sous son vrai nom (ses propres imports ne passent pas par ce
//! détour). Sinon le chargement passe tel quel.

use crate::exe::{iat_slot, write_slot};
use crate::{log, noms, win};
use std::sync::atomic::{AtomicUsize, Ordering};

static REAL_W: AtomicUsize = AtomicUsize::new(0);
static REAL_A: AtomicUsize = AtomicUsize::new(0);

type LoadW = unsafe extern "system" fn(*const u16) -> win::HMODULE;
type LoadA = unsafe extern "system" fn(*const u8) -> win::HMODULE;

/// Dans DllMain : la table d'imports de l'exe est déjà remplie.
pub unsafe fn install(exe: usize) {
    for (func, hook, real) in [("LoadLibraryW", load_w as *const () as usize, &REAL_W), ("LoadLibraryA", load_a as *const () as usize, &REAL_A)] {
        let Some(slot) = iat_slot(exe, "kernel32.dll", func) else {
            log(&format!("chargeur : import {func} introuvable dans l'exe"));
            continue;
        };
        real.store(*(slot as *const usize), Ordering::Relaxed);
        if !write_slot(slot, hook) {
            real.store(0, Ordering::Relaxed);
            log(&format!("chargeur : import {func} non modifiable"));
        }
    }
}

/// Position du nom de fichier dans le chemin, si le chemin désigne l'original de GameCore (casse
/// ignorée, avec ou sans `.dll`).
fn gamecore(path: &[u16]) -> Option<usize> {
    let start = path.iter().rposition(|&c| c == '\\' as u16 || c == '/' as u16).map_or(0, |i| i + 1);
    let name = String::from_utf16_lossy(&path[start..]).to_ascii_lowercase();
    let original = noms::GAMECORE.to_ascii_lowercase();
    (name == original || Some(name.as_str()) == original.strip_suffix(".dll")).then_some(start)
}

/// Chemin du proxy de GameCore (NUL final compris), à côté de l'original demandé, s'il y est posé.
fn redirection(path: &[u16]) -> Option<Vec<u16>> {
    let start = gamecore(path)?;
    let mut proxy: Vec<u16> = path[..start].to_vec();
    proxy.extend(noms::GAMECORE_PROXY.encode_utf16());
    proxy.push(0);
    let present = unsafe { win::GetFileAttributesW(proxy.as_ptr()) } != win::INVALID_FILE_ATTRIBUTES;
    let shown = String::from_utf16_lossy(path);
    if !present {
        log(&format!("chargeur : {shown} demandée, {} absent : original chargé", noms::GAMECORE_PROXY));
        return None;
    }
    log(&format!("chargeur : {shown} demandée, {} chargé à sa place", noms::GAMECORE_PROXY));
    Some(proxy)
}

unsafe fn wide_len(s: *const u16) -> usize {
    (0..).take_while(|&i| *s.add(i) != 0).count()
}

unsafe extern "system" fn load_w(name: *const u16) -> win::HMODULE {
    let real: LoadW = std::mem::transmute(REAL_W.load(Ordering::Relaxed));
    if !name.is_null() {
        if let Some(proxy) = redirection(std::slice::from_raw_parts(name, wide_len(name))) {
            let h = real(proxy.as_ptr());
            if !h.is_null() {
                return h;
            }
            log(&format!("chargeur : chargement du proxy impossible, erreur {} : original chargé", win::GetLastError()));
        }
    }
    real(name)
}

unsafe extern "system" fn load_a(name: *const u8) -> win::HMODULE {
    let real: LoadA = std::mem::transmute(REAL_A.load(Ordering::Relaxed));
    if !name.is_null() {
        // Le chemin en ANSI, converti pour être comparé et chargé en UTF-16.
        let n = win::MultiByteToWideChar(win::CP_ACP, 0, name, -1, std::ptr::null_mut(), 0);
        if n > 1 {
            let mut path = vec![0u16; n as usize];
            win::MultiByteToWideChar(win::CP_ACP, 0, name, -1, path.as_mut_ptr(), n);
            if let Some(proxy) = redirection(&path[..n as usize - 1]) {
                let h = win::LoadLibraryW(proxy.as_ptr());
                if !h.is_null() {
                    return h;
                }
                log(&format!("chargeur : chargement du proxy impossible, erreur {} : original chargé", win::GetLastError()));
            }
        }
    }
    real(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn reconnait_l_original() {
        assert_eq!(gamecore(&w(r"C:\Jeu\DLC\Expansion2\Binaries\Win64\GameCore_XP2_FinalRelease.dll")), Some(37));
        assert_eq!(gamecore(&w("gamecore_xp2_finalrelease")), Some(0));
        assert_eq!(gamecore(&w("C:/x/GAMECORE_XP2_FINALRELEASE.DLL")), Some(5));
        assert_eq!(gamecore(&w(r"C:\x\GameCore_XP1_FinalRelease.dll")), None);
        assert_eq!(gamecore(&w(r"C:\x\BGD_GameCore_XP2.dll")), None);
        assert_eq!(gamecore(&w(r"C:\x\GameCore_XP2_FinalRelease.dll.bak")), None);
    }

    #[test]
    fn redirige_vers_le_proxy_present() {
        let dir = std::env::temp_dir().join(format!("bgd_chargeur_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let original = format!(r"{}\{}", dir.display(), noms::GAMECORE);
        assert!(redirection(&w(&original)).is_none(), "proxy absent");
        std::fs::write(dir.join(noms::GAMECORE_PROXY), b"").unwrap();
        let proxy = redirection(&w(&original)).expect("proxy présent");
        assert_eq!(String::from_utf16_lossy(&proxy[..proxy.len() - 1]), format!(r"{}\{}", dir.display(), noms::GAMECORE_PROXY));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
