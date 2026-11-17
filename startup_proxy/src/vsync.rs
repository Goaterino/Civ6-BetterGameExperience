//! VSync coupée pendant l'écran de chargement (`vsync_chargement=on`, par défaut ; DX12 seulement).
//!
//! Avec la VSync, le thread principal reste bloqué dans `Present` jusqu'au rafraîchissement suivant
//! de l'écran, et il tient pendant ce temps un verrou du pilote graphique. Pendant un chargement, le
//! thread de chargement crée environ 2 000 textures des caches du terrain
//! (`ID3D12Device::CreateCommittedResource`) : chacune attend la sortie de `Present`, soit 2 à 3 s
//! de StartupGame où ni le CPU ni le GPU ne travaillent (analyses/chargement/README.md, « Ventilation
//! du reste »). Capture avec `VSync 0` : StartupGame plus court de 1,8 à 2,9 s.
//!
//! Le moteur présente par `140973470` (DX12) : `Present(SyncInterval = VSync ? 1 : 0, Flags)`, avec
//! `Flags = 0` quand la VSync est demandée. On remplace l'appel `call [rax+0x40]` par un appel à
//! `presenter`, qui passe l'intervalle à 0 entre `LoadGameViewState::Begin()` et `::End()`, et
//! seulement si le jeu demandait 1 : sans VSync, ou hors chargement, l'appel est le même qu'avant.
//! Les drapeaux restent ceux du jeu (0) : pas de `DXGI_PRESENT_ALLOW_TEARING`, que la chaîne de
//! présentation n'accepte peut-être pas.
//!
//! Début et fin du chargement : `140083a20` et `140083d30` (DX12), méthodes virtuelles de
//! LoadGameViewState. Le code de `::Begin()` existe deux fois à l'identique (un autre état du jeu) :
//! on garde celle qui cite la chaîne `LoadGameViewState::Begin()`.

use crate::{exe, log, motifs, patch, win};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// `mov rcx,[rsi+0x58] ; mov r8d,edi ; mov edx,ebx ; mov rax,[rcx] ; call [rax+0x40] ;
/// mov rcx,[rsi+0x58] ; mov ebx,eax`. Les 11 octets à partir de +4 sont réécrits en
/// `mov r8d,edi ; mov edx,ebx ; call presenter ; nop`.
const SITE_PRESENT: &str = "48 8B 4E 58 44 8B C7 8B D3 48 8B 01 FF 50 40 48 8B 4E 58 8B D8";
const PRESENT_DEBUT: usize = 4;
const PRESENT_LONG: usize = 11;

/// Prologues de `::Begin()` (14 octets recopiés) et `::End()` (17 octets), sans adressage relatif.
const DEBUT: &str = "40 55 41 54 41 56 48 8D AC 24 60 E0 FF FF B8 A0 20 00 00 E8 ?? ?? ?? ?? 48 2B E0 48 89 9C 24 D0 20 00 00 48 8D 95 C8 1F 00 00 4C 8B F1";
const DEBUT_COPIE: usize = 14;
const FIN: &str = "40 55 41 56 48 8D AC 24 48 E0 FF FF B8 B8 20 00 00 E8 ?? ?? ?? ?? 48 2B E0 48 89 9C 24 B0 20 00 00 48 8D 95 E0 1F 00 00";
const FIN_COPIE: usize = 17;

static CHARGEMENT: AtomicBool = AtomicBool::new(false);
/// Nombre de `LoadGameViewState::Begin()` depuis le lancement, si le détour est posé.
static DEBUTS: AtomicU64 = AtomicU64::new(0);
static BORNES: AtomicBool = AtomicBool::new(false);
static ORIG_DEBUT: AtomicUsize = AtomicUsize::new(0);
static ORIG_FIN: AtomicUsize = AtomicUsize::new(0);

type Methode = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;
type Present = unsafe extern "system" fn(*mut c_void, u32, u32) -> i32;

unsafe extern "system" fn debut(a: usize, b: usize, c: usize, d: usize) -> usize {
    CHARGEMENT.store(true, Ordering::Release);
    DEBUTS.fetch_add(1, Ordering::AcqRel);
    let orig: Methode = std::mem::transmute(ORIG_DEBUT.load(Ordering::Relaxed));
    orig(a, b, c, d)
}

unsafe extern "system" fn fin(a: usize, b: usize, c: usize, d: usize) -> usize {
    let orig: Methode = std::mem::transmute(ORIG_FIN.load(Ordering::Relaxed));
    let r = orig(a, b, c, d);
    CHARGEMENT.store(false, Ordering::Release);
    crate::ramasse_miettes::fin_de_chargement();
    r
}

/// `IDXGISwapChain::Present` (vtable +0x40), intervalle 0 au lieu de 1 pendant le chargement.
unsafe extern "system" fn presenter(chaine: *mut c_void, intervalle: u32, drapeaux: u32) -> i32 {
    let intervalle = intervalle_voulu(intervalle, CHARGEMENT.load(Ordering::Acquire));
    let vtable = *(chaine as *const *const usize);
    let present: Present = std::mem::transmute(*vtable.add(8));
    present(chaine, intervalle, drapeaux)
}

/// Seule la VSync demandée par le jeu (intervalle 1) est levée, et seulement pendant le chargement.
fn intervalle_voulu(intervalle: u32, chargement: bool) -> u32 {
    if chargement && intervalle == 1 { 0 } else { intervalle }
}

/// Nombre de chargements commencés depuis le lancement ; `None` si le début n'est pas détourné.
/// Sert aussi au remap allégé (remap_leger.rs).
pub fn debuts() -> Option<u64> {
    BORNES.load(Ordering::Acquire).then(|| DEBUTS.load(Ordering::Acquire))
}

/// Le réglage `vsync_chargement` (dernière occurrence ; `on` par défaut).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("vsync_chargement=")).last().map(str::trim) != Some("off")
}

/// La chaîne `texte` dans la section `.rdata` du module (première occurrence).
unsafe fn chaine(module: usize, texte: &[u8]) -> Option<usize> {
    let pe = module + exe::rd::<u32>(module + 0x3c) as usize;
    let sections = exe::rd::<u16>(pe + 6) as usize;
    let premiere = pe + 24 + exe::rd::<u16>(pe + 20) as usize;
    (0..sections).map(|i| premiere + 40 * i).find(|&s| exe::rd::<[u8; 8]>(s) == *b".rdata\0\0").and_then(|s| {
        let (taille, rva) = (exe::rd::<u32>(s + 8) as usize, exe::rd::<u32>(s + 12) as usize);
        let rdata = std::slice::from_raw_parts((module + rva) as *const u8, taille);
        rdata.windows(texte.len()).position(|w| w == texte).map(|i| module + rva + i)
    })
}

/// Un `lea r64, [rip+disp32]` vers `cible` dans les `portee` octets qui suivent `at` ?
unsafe fn cite(at: usize, portee: usize, cible: usize) -> bool {
    let code = std::slice::from_raw_parts(at as *const u8, portee + 7);
    (0..portee).any(|i| {
        matches!(code[i], 0x48 | 0x4c)
            && code[i + 1] == 0x8d
            && code[i + 2] & 0xc7 == 0x05
            && (at + i + 7).wrapping_add(i32::from_le_bytes(code[i + 3..i + 7].try_into().unwrap()) as isize as usize) == cible
    })
}

/// La seule occurrence de `motif` qui cite `texte` dans ses 0x400 premiers octets.
unsafe fn fonction(exe_base: usize, motif: &str, texte: &[u8]) -> Result<usize, String> {
    let s = chaine(exe_base, texte).ok_or_else(|| format!("{} introuvable", String::from_utf8_lossy(texte)))?;
    let trouvees: Vec<usize> = motifs::chercher(exe_base, &motifs::motif(motif)).into_iter().filter(|&f| cite(f, 0x400, s)).collect();
    match trouvees[..] {
        [f] => Ok(f),
        _ => Err(format!("{} : {} fonctions", String::from_utf8_lossy(texte), trouvees.len())),
    }
}

/// Détours du début et de la fin de l'écran de chargement (DX12 et DX11) : sans effet par eux-mêmes.
unsafe fn poser_bornes(exe_base: usize) -> Result<(), String> {
    let debut_at = fonction(exe_base, DEBUT, b"LoadGameViewState::Begin()")?;
    let fin_at = fonction(exe_base, FIN, b"LoadGameViewState::End()")?;
    let prologue_debut: [u8; DEBUT_COPIE] = exe::rd(debut_at);
    let prologue_fin: [u8; FIN_COPIE] = exe::rd(fin_at);
    ORIG_DEBUT.store(
        patch::install_at("LoadGameViewState::Begin", debut_at, &prologue_debut, debut_at + DEBUT_COPIE, debut as *const () as usize)?,
        Ordering::Relaxed,
    );
    ORIG_FIN.store(
        patch::install_at("LoadGameViewState::End", fin_at, &prologue_fin, fin_at + FIN_COPIE, fin as *const () as usize)?,
        Ordering::Relaxed,
    );
    BORNES.store(true, Ordering::Release);
    Ok(())
}

unsafe fn poser(exe_base: usize) -> Result<(), String> {
    let sites = motifs::chercher(exe_base, &motifs::motif(SITE_PRESENT));
    let [site] = sites[..] else {
        return Err(format!("{} Present sites (DX12 expected)", sites.len()));
    };

    let at = site + PRESENT_DEBUT;
    let page = patch::alloc_near(at).ok_or("pas de page proche")?;
    let mut saut = [0u8; 14];
    saut[..6].copy_from_slice(&[0xff, 0x25, 0, 0, 0, 0]);
    saut[6..].copy_from_slice(&(presenter as *const () as u64).to_le_bytes());
    std::ptr::copy_nonoverlapping(saut.as_ptr(), page as *mut u8, saut.len());
    let rel = patch::rel32(at + 5, page).ok_or("page trop loin")?;
    let mut code = [0u8; PRESENT_LONG];
    code[..5].copy_from_slice(&[0x44, 0x8b, 0xc7, 0x8b, 0xd3]);
    code[5] = 0xe8;
    code[6..10].copy_from_slice(&rel.to_le_bytes());
    code[10] = 0x90;

    let mut old = 0u32;
    if win::VirtualProtect(at as *mut c_void, PRESENT_LONG, win::PAGE_EXECUTE_READWRITE, &mut old) == 0 {
        return Err("VirtualProtect a échoué".into());
    }
    std::ptr::copy_nonoverlapping(code.as_ptr(), at as *mut u8, PRESENT_LONG);
    win::VirtualProtect(at as *mut c_void, PRESENT_LONG, old, &mut old);
    win::FlushInstructionCache(win::GetCurrentProcess(), at as *const c_void, PRESENT_LONG);
    Ok(())
}

pub fn install(exe_base: usize) {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) {
        log("vsync: not installed (actif=off)");
        return;
    }
    if let Err(e) = unsafe { poser_bornes(exe_base) } {
        log(&format!("vsync: loading screen bounds not found ({e})"));
        return;
    }
    if !enabled(&cfg) {
        log("vsync: kept during loading screens (vsync_chargement=off)");
        return;
    }
    match unsafe { poser(exe_base) } {
        Ok(()) => log("vsync: lifted during loading screens when the game asks for it"),
        Err(e) => log(&format!("vsync: not installed ({e})")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglage() {
        assert!(enabled(""));
        assert!(!enabled("vsync_chargement=off\n"));
        assert!(enabled("vsync_chargement=off\nvsync_chargement=on\n"));
    }

    #[test]
    fn seule_la_vsync_demandee_est_levee() {
        assert_eq!(intervalle_voulu(1, true), 0);
        assert_eq!(intervalle_voulu(1, false), 1, "hors chargement");
        assert_eq!(intervalle_voulu(0, true), 0, "sans VSync : rien ne change");
        assert_eq!(intervalle_voulu(2, true), 2, "autre intervalle : rien ne change");
    }

    #[test]
    fn motifs_uniques_dans_l_exe_dx12() {
        let Ok(exe) = std::fs::read(r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam\CivilizationVI_DX12.exe")
        else {
            return;
        };
        assert_eq!(motifs::trouver(&exe, &motifs::motif(SITE_PRESENT)).len(), 1);
        assert_eq!(motifs::trouver(&exe, &motifs::motif(DEBUT)).len(), 2, "départagées par la chaîne");
        assert_eq!(motifs::trouver(&exe, &motifs::motif(FIN)).len(), 1);
    }
}
