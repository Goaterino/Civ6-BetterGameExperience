//! Congrès : le correctif de `WorldCongress.lua`, script de gameplay exécuté par chaque machine,
//! commun à Windows (ui.rs, proxy de GameCore) et macOS (startup_proxy/src/macos_lua.rs, chargeur).

pub(crate) const CIBLES_BLOCK: &str = include_str!("ui_congres_cibles.lua");

/// WorldCongress.lua (script de gameplay) précédé de `ui_congres_cibles.lua` : les cibles
/// « joueur » des résolutions tirées de `BGD_MajorsCibles()`, sans les spectateurs, et la liste des
/// luxes recalculée à chaque session (`BGD_LuxesARecalculer()`).
pub fn patch_congres_cibles(src: &[u8]) -> Result<Vec<u8>, String> {
    const APPEL: &str = "PlayerManager.GetAliveMajors()";
    // Le test du cache des luxes de WC_Validate_LuxuryBan, entre les parenthèses de son `if`.
    const LUXES: &str = "(cachedLuxuryResources == nil)";
    let text = String::from_utf8_lossy(src);
    if text.contains("BGD_MajorsCibles") {
        return Err("déjà corrigé".into());
    }
    let n = text.matches(APPEL).count();
    if n == 0 {
        return Err(format!("{APPEL} introuvable"));
    }
    // Octets hors appel laissés tels quels : les fichiers du jeu ne sont pas tous en UTF-8.
    let mut out = CIBLES_BLOCK.as_bytes().to_vec();
    out.extend_from_slice(b"\r\n");
    let mut i = 0;
    while i < src.len() {
        if src[i..].starts_with(APPEL.as_bytes()) {
            out.extend_from_slice(b"BGD_MajorsCibles()");
            i += APPEL.len();
        } else if src[i..].starts_with(LUXES.as_bytes()) {
            out.extend_from_slice(b"(cachedLuxuryResources == nil or BGD_LuxesARecalculer())");
            i += LUXES.len();
        } else {
            out.push(src[i]);
            i += 1;
        }
    }
    Ok(out)
}

