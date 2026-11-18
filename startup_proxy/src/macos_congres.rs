//! Congrès mondial sous macOS : les scripts et l'écran que le proxy de GameCore Windows corrige
//! (proxy/src/ui.rs), avec les textes communs (proxy/src/congres_texte.rs).
//!
//! - `WorldCongress.lua` (gameplay) : cibles des résolutions sans les spectateurs ;
//! - `WorldCongressPopup.lua` : fenêtre en lecture seule pour un spectateur, réouverture pour
//!   revenir au vote ;
//! - `WorldCongressIntro.lua` : l'intro une seule fois par tour et par session ;
//! - `WorldCongressBetweenTurns.lua` et `.xml` : le bouton de retour au vote.
//!
//! L'état vient de l'export `BgdCongres` du proxy de GameCore (proxy/src/macos.rs) : bit 0,
//! spectateurs posés ; bit 1, retour au vote posé ; bit 2, spectateurs actifs ; bit 3, retour au
//! vote actif. Les natives ne rendent qu'une chaîne : la cale rend `Civ6RolloverProxy_Pol()` et
//! `Civ6RolloverProxy_Revote()`, des nombres, comme sous Windows.

use super::options::{publier, rendre, LuaCFn};
use super::salon::PROXY;
use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::Ordering;

extern "C" {
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

pub(crate) const SCRIPT_CIBLES: &str = "worldcongress.lua";
pub(crate) const SCRIPT_POPUP: &str = "worldcongresspopup.lua";
pub(crate) const SCRIPT_INTRO: &str = "worldcongressintro.lua";
pub(crate) const SCRIPT_ATTENTE: &str = "worldcongressbetweenturns.lua";
pub(crate) const XML_ATTENTE: &str = "WorldCongressBetweenTurns.xml";

const SPECTATEURS_POSES: u32 = 1;
const REVOTE_POSE: u32 = 2;

/// Bits de `BgdCongres`, None sans proxy de GameCore ou sans l'export.
pub(crate) fn etat() -> Option<u32> {
    let h = PROXY.load(Ordering::Relaxed) as *mut c_void;
    if h.is_null() {
        return None;
    }
    let p = unsafe { dlsym(h, c"BgdCongres".as_ptr()) };
    (!p.is_null()).then(|| unsafe { std::mem::transmute::<*mut c_void, extern "C" fn() -> u32>(p)() })
}

/// `Civ6Proxy_MacCongres()` → les bits de `BgdCongres`, en texte.
unsafe extern "C-unwind" fn congres(l: *mut c_void) -> c_int {
    rendre(l, &etat().unwrap_or(0).to_string())
}

const NATIVES: [(LuaCFn, &str); 1] = [(congres, "Civ6Proxy_MacCongres\0")];
const CALE: &str = r#"
-- macOS : natives à une seule chaîne (startup_proxy/src/macos_congres.rs).
if Civ6Proxy_MacCongres ~= nil then
	Civ6RolloverProxy_Pol = function() return math.floor((tonumber(Civ6Proxy_MacCongres()) or 0) / 4) % 2; end
	Civ6RolloverProxy_Revote = function() return math.floor((tonumber(Civ6Proxy_MacCongres()) or 0) / 8) % 2; end
end
"#;

/// Le script corrigé, la cale insérée entre l'original et les blocs ajoutés (`corrige` commence par
/// `src`).
fn avec_cale(src: &[u8], mut corrige: Vec<u8>) -> Vec<u8> {
    corrige.splice(src.len()..src.len(), CALE.bytes());
    corrige
}

unsafe fn natives(l: *mut c_void) -> Result<(), String> {
    if publier(l, &NATIVES) { Ok(()) } else { Err("native non posée".into()) }
}

/// Texte de remplacement du script `nom` (déjà en minuscules) et ce qui a changé, None s'il n'est
/// pas du congrès. Err : laissé tel quel.
pub unsafe fn lua(l: *mut c_void, nom: &str, src: &[u8]) -> Option<Result<(Vec<u8>, &'static str), String>> {
    let r = match nom {
        SCRIPT_CIBLES => cibles(l, src).map(|t| (t, "congrès : spectateurs retirés des cibles des résolutions, luxes recalculés à chaque session")),
        SCRIPT_POPUP => popup(l, src),
        SCRIPT_INTRO => super::congres_texte::patch_congres_intro(src).map(|t| (t, "congrès : intro du congrès une seule fois par tour et par session")),
        SCRIPT_ATTENTE => attente(l, src).map(|t| (t, "congrès : bouton de retour au vote")),
        _ => return None,
    };
    Some(r)
}

/// WorldCongress.lua (script de gameplay, exécuté par chaque machine), précédé de la cale :
/// seulement si les détours des spectateurs sont posés, comme sous Windows.
unsafe fn cibles(l: *mut c_void, src: &[u8]) -> Result<Vec<u8>, String> {
    if etat().unwrap_or(0) & SPECTATEURS_POSES == 0 {
        return Err("proxy de GameCore sans les détours des spectateurs".into());
    }
    let corrige = super::congres_texte::patch_congres_cibles(src)?;
    natives(l)?;
    let mut out = CALE.as_bytes().to_vec();
    out.extend_from_slice(&corrige);
    Ok(out)
}

unsafe fn popup(l: *mut c_void, src: &[u8]) -> Result<(Vec<u8>, &'static str), String> {
    let spectateurs = etat().unwrap_or(0) & SPECTATEURS_POSES != 0;
    let corrige = super::congres_texte::patch_congres(src, spectateurs)?;
    natives(l)?;
    let quoi = if spectateurs { "congrès : fenêtre du congrès en lecture seule pour les spectateurs, retour au vote" } else { "congrès : retour au vote" };
    Ok((avec_cale(src, corrige), quoi))
}

unsafe fn attente(l: *mut c_void, src: &[u8]) -> Result<Vec<u8>, String> {
    let corrige = super::congres_texte::patch_congres_attente(src)?;
    natives(l)?;
    Ok(avec_cale(src, corrige))
}

/// WorldCongressBetweenTurns.xml avec le bouton, si le retour au vote est posé (comme Windows).
pub fn xml_attente(texte: &[u8]) -> Result<Vec<u8>, String> {
    if etat().unwrap_or(0) & REVOTE_POSE == 0 {
        return Err("retour au vote non posé".into());
    }
    super::congres_texte::patch_congres_attente_xml(texte)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::congres_texte::{ATTENTE_BLOCK, CONGRES_BLOCK, VOTE_BLOCK};

    fn jeu(rel: &str) -> Option<Vec<u8>> {
        let home = std::env::var_os("HOME")?;
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/Assets")
            .join(rel);
        std::fs::read(p).ok()
    }

    /// Les scripts du jeu macOS installé acceptent les correctifs communs.
    #[test]
    fn scripts_du_jeu() {
        if let Some(src) = jeu("DLC/Expansion2/Scripts/WorldCongress.lua") {
            let out = String::from_utf8_lossy(&super::super::congres_texte::patch_congres_cibles(&src).unwrap()).into_owned();
            assert!(out.matches("BGD_MajorsCibles()").count() >= 5, "validateurs à cible joueur");
        }
        if let Some(src) = jeu("DLC/Expansion2/UI/Additions/WorldCongressPopup.lua") {
            let t = avec_cale(&src, super::super::congres_texte::patch_congres(&src, true).unwrap());
            let t = String::from_utf8_lossy(&t);
            let (cale, bloc) = (t.find("Civ6Proxy_MacCongres").unwrap(), t.find("Civ6RolloverProxy (src/ui_congres.lua)").unwrap());
            assert!(cale < bloc && t.ends_with(VOTE_BLOCK), "cale avant les blocs");
        }
        if let Some(src) = jeu("DLC/Expansion2/UI/Additions/WorldCongressIntro.lua") {
            super::super::congres_texte::patch_congres_intro(&src).unwrap();
        }
        if let Some(src) = jeu("DLC/Expansion2/UI/Additions/WorldCongressBetweenTurns.lua") {
            let t = avec_cale(&src, super::super::congres_texte::patch_congres_attente(&src).unwrap());
            assert!(t.starts_with(&src) && t.ends_with(ATTENTE_BLOCK.as_bytes()));
        }
        if let Some(src) = jeu("DLC/Expansion2/UI/Additions/WorldCongressBetweenTurns.xml") {
            assert!(String::from_utf8(super::super::congres_texte::patch_congres_attente_xml(&src).unwrap()).unwrap().contains("BGD_Revoter"));
        }
    }

    /// Les natives qu'attendent les blocs sont rendues par la cale.
    #[test]
    fn cale() {
        for bloc in [CONGRES_BLOCK, VOTE_BLOCK, ATTENTE_BLOCK, super::super::congres_texte::CIBLES_BLOCK] {
            for (i, _) in bloc.match_indices("Civ6RolloverProxy_") {
                let nom: String = bloc[i..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                assert!(CALE.contains(&format!("{nom} = function()")), "{nom} attendue mais non rendue");
            }
        }
    }
}
