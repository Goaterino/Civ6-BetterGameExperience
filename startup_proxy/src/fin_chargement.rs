//! Post-chargement sans la pause entre les ticks de l'écran de chargement
//! (`pause_fin_chargement=on`, par défaut).
//!
//! Pendant un écran de chargement, le jeu (1.0.12.72 et 1.0.12.74, DX11 et DX12) fait un tick
//! toutes les `LoadGameTickIntervalInMS` (AppOptions.txt, `[Performance]`, 16 par défaut) :
//! `LoadGameViewState::Begin()` range cette valeur dans la globale de l'intervalle des ticks par une
//! fonction d'une ligne (`mov [rip+tick], ecx ; ret`, `1406ff130` en DX11), et `::End()` y remet
//! `TickIntervalInMS` de la même façon. Tant que le chargement tourne sur d'autres threads, cette
//! pause ne coûte rien ; mais la dernière phase, le post-chargement (`PostLoadTime`), est faite par
//! le thread principal, une étape par tick : le cache de la vue du terrain cuit au plus six cases
//! par appel à `WorldView_PostLoadUpdate`, et le rechargement d'une petite carte demande 80 appels.
//! À 16 ms le tick, cela fait 1,37 s ; sans pause, 0,09 à 0,21 s (VSync coupée).
//!
//! Le module pose un relais de deux instructions sur le seul appel à `WorldView_PostLoadUpdate` : il
//! écrit 0 dans la globale de l'intervalle, puis saute à la fonction, sans toucher aux registres ni
//! aux arguments. Le post-chargement enchaîne donc ses ticks sans pause ; `::End()` remet
//! l'intervalle normal juste après, comme toujours. L'étape de configuration, GameCore et
//! `StartupGame` gardent leurs ticks de 16 ms (sans pause, la boucle leur prend du temps CPU :
//! 0,3 à 0,4 s de plus, mesuré).
//!
//! Avec la VSync, chaque image attend encore l'écran : `vsync` la coupe pendant les écrans de
//! chargement (DX12 et DX11).

use crate::{exe, log, motifs, patch, win};
use std::ffi::c_void;

/// `mov rax,[rip+app] ; mov ecx,[rax+champ] ; call fixe_tick` dans `LoadGameViewState::Begin()` et
/// `::End()` ; `fixe_tick` est `mov [rip+tick],ecx ; ret`.
const SET_TICK_CALL: &str = "48 8B 05 ?? ?? ?? ?? 8B 88 ?? ?? ?? ?? E8 ?? ?? ?? ??";
const SET_TICK_CALL_AT: usize = 13;
const POSTLOAD_UPDATE: &[u8] = b"WorldView_PostLoadUpdate\0";

/// Le réglage `pause_fin_chargement` (dernière occurrence ; `on` par défaut).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("pause_fin_chargement=")).last().map(str::trim) != Some("off")
}

/// La globale de l'intervalle des ticks, d'après la fonction qu'appellent le début et la fin de
/// l'écran de chargement.
unsafe fn tick_global(exe_base: usize) -> Result<usize, String> {
    let mut setters: Vec<usize> = motifs::chercher(exe_base, &motifs::motif(SET_TICK_CALL))
        .into_iter()
        .map(|at| motifs::cible_rel32(at + SET_TICK_CALL_AT))
        .filter(|&t| exe::rd::<[u8; 2]>(t) == [0x89, 0x0d] && exe::rd::<u8>(t + 6) == 0xc3)
        .collect();
    setters.sort();
    setters.dedup();
    let [setter] = setters[..] else { return Err(format!("{} tick interval setters", setters.len())) };
    Ok((setter as i64 + 6 + exe::rd::<i32>(setter + 2) as i64) as usize)
}

/// Début de la fonction qui contient `addr`, d'après le répertoire des exceptions (`.pdata`), en
/// remontant les infos de déroulement chaînées jusqu'à l'entrée principale.
unsafe fn func_start(module: usize, addr: usize) -> Option<usize> {
    let nt = module + exe::rd::<u32>(module + 0x3c) as usize;
    let (dir, size) = (exe::rd::<u32>(nt + 24 + 112 + 3 * 8) as usize, exe::rd::<u32>(nt + 24 + 112 + 3 * 8 + 4) as usize);
    let rva = addr.checked_sub(module)?;
    let entry = |i: usize| -> (usize, usize, usize) {
        let e = module + dir + 12 * i;
        (exe::rd::<u32>(e) as usize, exe::rd::<u32>(e + 4) as usize, exe::rd::<u32>(e + 8) as usize)
    };
    let (mut lo, mut hi) = (0, size / 12);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if entry(mid).0 <= rva { lo = mid + 1 } else { hi = mid }
    }
    let (mut begin, end, mut unwind) = entry(lo.checked_sub(1)?);
    if rva >= end {
        return None;
    }
    for _ in 0..8 {
        if exe::rd::<u8>(module + unwind) >> 3 & 4 == 0 {
            return Some(module + begin);
        }
        let codes = exe::rd::<u8>(module + unwind + 2) as usize;
        let chained = module + unwind + 4 + ((codes + 1) & !1) * 2;
        begin = exe::rd::<u32>(chained) as usize;
        unwind = exe::rd::<u32>(chained + 8) as usize;
    }
    None
}

/// Le seul `call rel32` vers `WorldView_PostLoadUpdate` (la fonction qui cite son propre nom).
unsafe fn postload_call(exe_base: usize) -> Result<(usize, usize), String> {
    let (start, size) = exe::text_section(exe_base).ok_or(".text not found")?;
    let nt = exe_base + exe::rd::<u32>(exe_base + 0x3c) as usize;
    let image = std::slice::from_raw_parts(exe_base as *const u8, exe::rd::<u32>(nt + 24 + 56) as usize);
    let name = (1..image.len() - POSTLOAD_UPDATE.len())
        .find(|&i| image[i - 1] == 0 && &image[i..i + POSTLOAD_UPDATE.len()] == POSTLOAD_UPDATE)
        .map(|i| exe_base + i)
        .ok_or("WorldView_PostLoadUpdate not found")?;
    let text = std::slice::from_raw_parts(start as *const u8, size);
    let mut funcs: Vec<usize> = (0..size - 7)
        .filter(|&i| matches!(text[i], 0x48 | 0x4c) && text[i + 1] == 0x8d && text[i + 2] & 0xc7 == 0x05)
        .filter(|&i| (start + i + 7).wrapping_add(i32::from_le_bytes(text[i + 3..i + 7].try_into().unwrap()) as isize as usize) == name)
        .filter_map(|i| func_start(exe_base, start + i))
        .collect();
    funcs.dedup();
    let [func] = funcs[..] else { return Err(format!("{} functions cite WorldView_PostLoadUpdate", funcs.len())) };
    let calls: Vec<usize> = (0..size - 5).filter(|&i| text[i] == 0xe8 && motifs::cible_rel32(start + i) == func).map(|i| start + i).collect();
    let [call] = calls[..] else { return Err(format!("{} calls to WorldView_PostLoadUpdate", calls.len())) };
    Ok((call, func))
}

unsafe fn poser(exe_base: usize) -> Result<(usize, usize), String> {
    let tick = tick_global(exe_base)?;
    let (call, func) = postload_call(exe_base)?;
    let page = patch::alloc_near(call).ok_or("no page near the exe")?;
    // mov dword [rip+tick], 0 ; jmp [rip+0] ; dq WorldView_PostLoadUpdate
    let mut code = vec![0xc7, 0x05];
    let rel = (tick as i64 - (page as i64 + 10)) as i32;
    if (page as i64 + 10 + rel as i64) as usize != tick {
        return Err("tick global out of range".into());
    }
    code.extend_from_slice(&rel.to_le_bytes());
    code.extend_from_slice(&0u32.to_le_bytes());
    code.extend_from_slice(&[0xff, 0x25, 0, 0, 0, 0]);
    code.extend_from_slice(&(func as u64).to_le_bytes());
    std::ptr::copy_nonoverlapping(code.as_ptr(), page as *mut u8, code.len());
    win::FlushInstructionCache(win::GetCurrentProcess(), page as *const c_void, code.len());
    let r = patch::rel32(call, page).ok_or("page out of range")?;
    patch::write_rel32(call, r);
    Ok((call, tick))
}

pub fn install(exe_base: usize) {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("post-load: not installed (actif=off)");
        return;
    }
    if !enabled(&cfg) {
        log("post-load: load tick kept (pause_fin_chargement=off)");
        return;
    }
    match unsafe { poser(exe_base) } {
        Ok((call, tick)) => log(&format!(
            "post-load: ticked continuously (call {:#x}, tick interval {:#x})",
            call - exe_base + 0x1_4000_0000,
            tick - exe_base + 0x1_4000_0000
        )),
        Err(e) => log(&format!("post-load: not installed ({e})")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting() {
        assert!(enabled(""));
        assert!(!enabled("pause_fin_chargement=off\n"));
        assert!(enabled("pause_fin_chargement=off\npause_fin_chargement=on\n"));
    }

    const EXE_DIR: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam";

    /// Les sites dans chaque exe connu qui est là (DX12 et DX11 de chaque version, dans l'ordre de
    /// `crate::test_exe`) : une seule fonction qui fixe l'intervalle des ticks, une seule fonction
    /// qui cite `WorldView_PostLoadUpdate`, avec un seul appelant, et le site `Present` DX11 de
    /// `vsync` dans l'exe DX11 seulement.
    #[test]
    fn sites_in_installed_exes() {
        let mut found = 0;
        for (i, (exe_name, dx11)) in [("CivilizationVI_DX12.exe", false), ("CivilizationVI.exe", true)].iter().cycle().take(6).enumerate() {
            let path = crate::test_exe(EXE_DIR, i, exe_name);
            let Ok(file) = std::fs::read(&path) else { continue };
            let image = map(&file);
            let base = image.as_ptr() as usize;
            unsafe {
                tick_global(base).unwrap_or_else(|e| panic!("{path}: {e}"));
                postload_call(base).unwrap_or_else(|e| panic!("{path}: {e}"));
                let present = motifs::chercher(base, &motifs::motif(crate::vsync::SITE_PRESENT_DX11)).len();
                assert_eq!(present, *dx11 as usize, "{path}: DX11 Present sites");
            }
            found += 1;
        }
        if found == 0 {
            eprintln!("no Civilization VI exe found, test skipped");
        }
    }

    /// Projette les en-têtes et les sections d'un fichier PE à leurs RVA, comme le chargeur de
    /// Windows.
    fn map(file: &[u8]) -> Vec<u8> {
        let rd32 = |o: usize| u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
        let rd16 = |o: usize| u16::from_le_bytes(file[o..o + 2].try_into().unwrap()) as usize;
        let pe = rd32(0x3c);
        let mut image = vec![0u8; rd32(pe + 24 + 56)];
        let headers = rd32(pe + 24 + 60);
        image[..headers].copy_from_slice(&file[..headers]);
        let first = pe + 24 + rd16(pe + 20);
        for i in 0..rd16(pe + 6) {
            let s = first + 40 * i;
            let (vsize, rva, raw_size, raw) = (rd32(s + 8), rd32(s + 12), rd32(s + 16), rd32(s + 20));
            let n = raw_size.min(vsize.max(raw_size)).min(file.len() - raw).min(image.len() - rva);
            image[rva..rva + n].copy_from_slice(&file[raw..raw + n]);
        }
        image
    }
}
