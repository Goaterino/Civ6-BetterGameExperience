//! Remap de MPH, premier chargement de l'hôte sans terrain (`remap_leger=on`, par défaut ; seulement
//! avec `remap_host_load`, voir [remap.rs](remap.rs)) : analyses/chargement/remap_premier_chargement.md.
//!
//! Au remap, l'hôte charge la nouvelle partie, puis la recharge depuis sa propre sauvegarde (S1)
//! avant que les clients ne chargent S2. Le premier chargement ne sert qu'à produire S1 : son
//! terrain 3D, sa WorldView et PostLoadTime (environ 11 s sur 22) sont jetés.
//!
//! `GameViewState::StartupGame` (`14007f5f0` DX12, `14007e970` DX11) prend un mot de drapeaux en
//! 3ᵉ argument (`GameViewState+0xd80`) : avec le bit 2, celui du World Builder, il saute la
//! synthèse du terrain et la WorldView, et l'étape d'après-chargement, la mise à jour et la
//! fermeture de `GameViewState` le respectent. L'appel de l'état 9 de la machine à états du
//! chargement (`140088eab` DX12, `1400882db` DX11) passe par `demarrer`, qui ajoute le bit 2 quand
//! le Lua de l'hôte l'a armé juste avant `Network.RestartGame()` (`Civ6Proxy_RemapLeger`), une seule
//! fois, et seulement dans les 120 s. GameCore, l'interface InGame et ses gestionnaires (BSM…), la
//! sauvegarde et l'instantané suivent le même chemin. Le chargement de la copie qui suit passe par
//! le même site sans armement.
//!
//! Garde-fou : `Civ6Proxy_RemapLegerActif()` dit si la partie en cours a démarré sans terrain ;
//! remap.lua ne laisse alors jamais l'hôte y jouer (nouvelle demande de rechargement, puis remap
//! complet).

use crate::{log, motifs};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// État 9 de `140084850` : `call [rax+1B0h]` (IGameContext, WORLD_BUILDER) … drapeaux en r8d,
/// puis `call StartupGame` en +0x40.
const SITE: &str = "FF 90 B0 01 00 00 48 8B 8D ?? ?? ?? ?? 0F B6 F8 48 85 C9 74 06 48 8B 11 FF 52 08 40 84 FF 41 8B DE \
                    41 0F 45 DD E8 ?? ?? ?? ?? 48 8B 0D ?? ?? ?? ?? 44 8B C3 8B D6 48 8B F8 48 8B 89 ?? ?? 00 00 E8";
const SITE_APPEL: usize = 0x40;
/// Dans StartupGame : `test byte [r13+0D80h],2 ; jnz` avant la synthèse du terrain. S'il manque,
/// rien n'est posé.
const RESPECTE: &str = "41 F6 85 80 0D 00 00 02 75 ?? 48 8D 4D ?? E8 ?? ?? ?? ?? 0F B6 95 ?? ?? 00 00 48 8D 4D ?? E8";
const SANS_TERRAIN: u32 = 2;
const VALIDITE: Duration = Duration::from_secs(120);

static ORIG: AtomicUsize = AtomicUsize::new(0);
static POSE: AtomicBool = AtomicBool::new(false);
/// Instant de l'armement, et nombre de chargements commencés à ce moment (vsync.rs).
static ARME: Mutex<Option<(Instant, Option<u64>)>> = Mutex::new(None);
/// La partie en cours a démarré sans terrain.
static LEGERE: AtomicBool = AtomicBool::new(false);

type Startup = unsafe extern "C" fn(*mut c_void, u32, u32) -> usize;

/// Un armement de moins de 120 s, consommé, et pour le chargement qui suit directement : si un
/// autre chargement a commencé depuis (redémarrage qui n'a pas abouti, puis sauvegarde ou salon),
/// il est jeté.
fn prendre(debuts: Option<u64>) -> bool {
    ARME.lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .is_some_and(|(t, avant)| t.elapsed() <= VALIDITE && avant.zip(debuts).is_none_or(|(a, d)| d == a + 1))
}

unsafe extern "C" fn demarrer(gvs: *mut c_void, p2: u32, drapeaux: u32) -> usize {
    let orig: Startup = std::mem::transmute(ORIG.load(Ordering::Relaxed));
    let leger = prendre(crate::vsync::debuts());
    LEGERE.store(leger, Ordering::Relaxed);
    if leger {
        log("remap : host's first load started without the 3D terrain (it is reloaded from its own save at once)");
        return orig(gvs, p2, drapeaux | SANS_TERRAIN);
    }
    orig(gvs, p2, drapeaux)
}

/// `Civ6Proxy_RemapLeger()` : l'hôte va relancer la partie et la recharger aussitôt.
pub unsafe extern "C-unwind" fn lua_armer(_l: *mut c_void) -> i32 {
    if POSE.load(Ordering::Relaxed) {
        *ARME.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), crate::vsync::debuts()));
    }
    0
}

/// `Civ6Proxy_RemapLegerActif()` → 1 si la partie en cours a démarré sans terrain, 0 sinon.
pub unsafe extern "C-unwind" fn lua_actif(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    (h.push_integer)(l, LEGERE.load(Ordering::Relaxed) as i32);
    1
}

/// Le réglage `remap_leger` (dernière occurrence ; `on` par défaut).
pub fn enabled(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("remap_leger=")).last().map(str::trim) != Some("off")
}

pub fn install(exe_base: usize) {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    if crate::salon::actif_off(&cfg) || !crate::remap::enabled(&cfg) {
        log("remap : light first load not installed (actif=off or remap_host_load=off)");
        return;
    }
    if !enabled(&cfg) {
        log("remap : host's first load builds the terrain (remap_leger=off)");
        return;
    }
    unsafe {
        let sites = motifs::chercher(exe_base, &motifs::motif(SITE));
        let controle = motifs::chercher(exe_base, &motifs::motif(RESPECTE));
        let ([site], [_]) = (&sites[..], &controle[..]) else {
            log(&format!("remap : light first load not installed ({} sites, {} terrain tests)", sites.len(), controle.len()));
            return;
        };
        match motifs::rediriger_appel(site + SITE_APPEL, demarrer as *const () as usize) {
            Ok(ancienne) => {
                ORIG.store(ancienne, Ordering::Relaxed);
                POSE.store(true, Ordering::Relaxed);
                log("remap : host's first load at a remap will skip the 3D terrain");
            }
            Err(e) => log(&format!("remap : light first load not installed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglage() {
        assert!(enabled(""));
        assert!(!enabled("remap_leger=off\n"));
    }

    #[test]
    fn armement_consomme_une_fois() {
        *ARME.lock().unwrap() = Some((Instant::now(), Some(4)));
        assert!(prendre(Some(5)));
        assert!(!prendre(Some(5)), "une seule fois");
        *ARME.lock().unwrap() = Some((Instant::now() - VALIDITE - Duration::from_secs(1), Some(4)));
        assert!(!prendre(Some(5)), "périmé");
        *ARME.lock().unwrap() = Some((Instant::now(), Some(4)));
        assert!(!prendre(Some(6)), "un autre chargement a commencé entre-temps");
        *ARME.lock().unwrap() = Some((Instant::now(), None));
        assert!(prendre(None), "sans compteur : la durée seule");
    }

    #[test]
    fn motifs_uniques_dans_les_exes() {
        for nom in ["CivilizationVI_DX12.exe", "CivilizationVI.exe"] {
            let Ok(exe) = std::fs::read(format!(
                r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam\{nom}"
            )) else {
                return;
            };
            let s = motifs::trouver(&exe, &motifs::motif(SITE));
            assert_eq!(s.len(), 1, "{nom}");
            assert_eq!(exe[s[0] + SITE_APPEL], 0xe8);
            assert_eq!(motifs::trouver(&exe, &motifs::motif(RESPECTE)).len(), 1, "{nom}");
        }
    }
}
