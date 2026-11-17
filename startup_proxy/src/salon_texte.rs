//! Salon multijoueur : les textes ajoutés aux scripts et au XML du jeu, et ce qui ne dépend pas du
//! système (jetons, versions, réglages). Commun à Windows (salon.rs) et macOS (macos_salon.rs).

use super::version::MARQUE;
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) const LUA_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("salon.lua"));
pub(crate) const SCRIPT: &str = "stagingroom.lua";
pub(crate) const XML: &str = "StagingRoom.xml";
/// Bouton de l'hôte, juste après celui-ci (même parent : la barre du haut du salon).
pub(crate) const ANCRE_XML: &str = r#"ID="BackButton""#;
pub(crate) const BOUTON_CHECK: &str = r#"<GridButton ID="BGD_CheckButton" Style="ShellButtonOrnateFlat" Anchor="L,C" Offset="180,0" Size="150,38" String="BGE CHECK" Hidden="1"/>"#;
/// Minuteur du salon : une animation d'une demi-seconde environ (vitesse 2), relancée à sa fin.
pub(crate) const MINUTEUR: &str = r#"  <!-- BetterGameExperience : minuteur du salon (startup_proxy/src/salon.rs) -->
  <SlideAnim ID="BGD_SalonMinuteur" Size="1,1" Begin="0,0" End="0,0" Function="Root" Speed="2" Cycle="Once" Stopped="1"/>"#;
pub(crate) const CHARGEMENT_BLOCK: &str = include_str!("chargement.lua");
pub(crate) const SCRIPT_CHARGEMENT: &str = "loadscreen.lua";
pub(crate) const PARTIE_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("partie.lua"));
pub(crate) const SCRIPT_PARTIE: &str = "ingame.lua";
/// Option de partie « No First/Last moves (BGD) » (départ et fin de tour simultanés, depart_tour/README.md).
pub(crate) const OPTION_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("option_partie.lua"));
pub(crate) const SCRIPT_OPTION: &str = "setupparameters.lua";

/// `actif=off` dans le fichier de réglages, dernière occurrence comme `read_config`.
pub(crate) fn actif_off(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("actif=")).last().is_some_and(|v| v.trim() == "off")
}

/// La première marque `BgdVersion=<version>\0` du fichier, version valide.
pub(crate) fn version_marquee(data: &[u8]) -> Option<String> {
    let marque = MARQUE.as_bytes();
    let mut debut = 0;
    while let Some(i) = data[debut..].windows(marque.len()).position(|w| w == marque) {
        let apres = &data[debut + i + marque.len()..];
        if let Some(fin) = apres.iter().take(64).position(|&b| b == 0) {
            if let Some(v) = version_valide(&apres[..fin]) {
                return Some(v);
            }
        }
        debut += i + 1;
    }
    None
}

/// Une version comme celles de version.rs : `0.8.0`, `1.0.0-rc.1`.
pub(crate) fn version_valide(octets: &[u8]) -> Option<String> {
    let ok = !octets.is_empty() && octets.len() <= 32 && octets.iter().all(|&b| b.is_ascii_alphanumeric() || b"._+-".contains(&b));
    ok.then(|| String::from_utf8_lossy(octets).into_owned())
}

/// StagingRoom.lua suivi du bloc du salon. Octets laissés tels quels, comme pour Options.lua.
///
/// Le bloc tourne dans sa propre fonction : ses locales ne comptent pas dans les registres de la
/// fonction principale du script, déjà près de la limite chez MPH (146 locales au premier niveau ;
/// HavokScript a refusé le script à 192 : « Function or expression requires too many registers »).
pub(crate) fn patch_staging_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n(function()\r\n");
    out.extend_from_slice(LUA_BLOCK.as_bytes());
    out.extend_from_slice(b"\r\nend)();\r\n");
    out
}

/// `StagingRoom.xml` avec le bouton du BGE Check après `BackButton` (même retrait) et le minuteur à
/// la fin du contexte.
pub(crate) fn patch_staging_xml(src: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(src).map_err(|_| "texte non UTF-8".to_string())?;
    if text.contains("ID=\"BGD_CheckButton\"") {
        return Err("bouton déjà présent".into());
    }
    let (debut, fin) = super::xml_texte::element(text, ANCRE_XML).ok_or_else(|| "bouton BackButton introuvable".to_string())?;
    let ligne = text[..debut].rfind('\n').map_or(0, |i| i + 1);
    let retrait: String = text[ligne..debut].chars().take_while(|c| c.is_whitespace()).collect();
    let mut out = String::with_capacity(text.len() + BOUTON_CHECK.len() + MINUTEUR.len() + 128);
    out.push_str(&text[..fin]);
    out.push_str("\r\n");
    out.push_str(&retrait);
    out.push_str("<!-- BetterGameExperience : BGE Check, symétrique du MOD CHECK de MPH (startup_proxy/src/salon.rs) -->\r\n");
    out.push_str(&retrait);
    out.push_str(BOUTON_CHECK);
    out.push_str(&text[fin..]);
    let fin = super::xml_texte::fin_du_contexte(&out)?;
    out.insert_str(fin, &format!("{MINUTEUR}\r\n"));
    Ok(out.into_bytes())
}

/// Jeton d'une ouverture du salon : l'heure en nanosecondes, l'identifiant du processus et un
/// compteur, en hexadécimal. Il suffit qu'il diffère d'une ouverture à l'autre et d'une machine à
/// l'autre ; ce n'est pas un secret.
pub(crate) fn new_token() -> String {
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}{:x}{n:x}", std::process::id())
}

/// LoadScreen.lua suivi de `chargement.lua`.
pub(crate) fn patch_load_screen_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(CHARGEMENT_BLOCK.as_bytes());
    out
}

/// InGame.lua suivi de `partie.lua`.
pub(crate) fn patch_in_game_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(PARTIE_BLOCK.as_bytes());
    out
}

/// SetupParameters.lua suivi de `option_partie.lua`.
pub(crate) fn patch_setup_parameters_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(OPTION_BLOCK.as_bytes());
    out
}

