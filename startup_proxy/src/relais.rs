//! Relais des exports de winmm vers la winmm de System32 (voir build.rs).

use crate::win;

include!(concat!(env!("OUT_DIR"), "/winmm_relais.rs"));

/// Case d'un export que la winmm de cette version de Windows n'a pas : rend 0.
extern "system" fn absente() -> usize {
    0
}

/// Charge la winmm de System32, par son chemin complet (le nom seul désignerait ce proxy, déjà
/// chargé), et remplit les cases. Appelée au début de DllMain : les modules qui importent winmm
/// sont initialisés après elle. Rend le chemin et le nombre d'exports absents.
pub unsafe fn init() -> Result<(String, usize), String> {
    let cases = &mut *std::ptr::addr_of_mut!(CASES);
    cases.fill(absente as *const () as usize);
    let mut buf = [0u16; 260];
    let n = win::GetSystemDirectoryW(buf.as_mut_ptr(), buf.len() as u32) as usize;
    if n == 0 || n >= buf.len() {
        return Err(format!("dossier système introuvable, erreur {}", win::GetLastError()));
    }
    let path = format!(r"{}\winmm.dll", String::from_utf16_lossy(&buf[..n]));
    let h = win::LoadLibraryW(win::wide(&path).as_ptr());
    if h.is_null() {
        return Err(format!("chargement de {path} impossible, erreur {}", win::GetLastError()));
    }
    let mut absents = 0;
    for (case, (nom, ordinal)) in cases.iter_mut().zip(EXPORTS.iter()) {
        let p = match nom {
            Some(n) => win::GetProcAddress(h, n.as_ptr()),
            None => win::GetProcAddress(h, *ordinal as usize as *const u8),
        };
        if p.is_null() {
            absents += 1;
        } else {
            *case = p as usize;
        }
    }
    Ok((path, absents))
}
