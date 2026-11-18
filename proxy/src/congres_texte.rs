//! Congrès : les correctifs de `WorldCongress.lua` (script de gameplay exécuté par chaque machine)
//! et des écrans du congrès, communs à Windows (ui.rs, proxy de GameCore) et macOS
//! (startup_proxy/src/macos_lua.rs et macos_xml.rs, chargeur).

pub(crate) const CIBLES_BLOCK: &str = include_str!("ui_congres_cibles.lua");
pub(crate) const CONGRES_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("ui_congres.lua"));
pub(crate) const VOTE_BLOCK: &str = include_str!("ui_congres_vote.lua");
pub(crate) const ATTENTE_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("ui_congres_attente.lua"));
pub(crate) const ATTENTE_XML: &str = include_str!("ui_congres_attente.xml");
pub(crate) const INTRO_BLOCK: &str = include_str!("ui_congres_intro.lua");

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


/// WorldCongressPopup.lua suivi de `ui_congres.lua` si `spectateurs` (la fenêtre du congrès en
/// lecture seule pour un spectateur), puis de `ui_congres_vote.lua` (retour au vote). Les blocs
/// lisent les locales du fichier : on vérifie qu'elles y sont.
pub fn patch_congres(src: &[u8], spectateurs: bool) -> Result<Vec<u8>, String> {
    let has = |pat: &str| src.windows(pat.len()).any(|w| w == pat.as_bytes());
    if has("Civ6RolloverProxy (src/ui_congres") {
        return Err("déjà corrigé".into());
    }
    for pat in [
        "local m_IsInSession",
        "local m_CurrentPhase",
        "local m_CurrentStage",
        "local m_StartingFavor",
        "local m_WorkingFavor",
        "local PHASE_STEP_MAX",
        "local WORLD_CONGRESS_STAGE_1",
        "local WORLD_CONGRESS_STAGE_2",
        "local m_kResolutionVotes",
        "local m_kResolutionChoices",
        "local m_kProposalVotes",
        "function UpdateWorkingFavor(",
        "function OnAccept(",
        "function SetupWorldCongress(",
        "function OnLocalPlayerTurnEnd(",
        "function OnResumeCongress(",
    ] {
        if !has(pat) {
            return Err(format!("{pat} introuvable"));
        }
    }
    let mut out = src.to_vec();
    if spectateurs {
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(CONGRES_BLOCK.as_bytes());
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(VOTE_BLOCK.as_bytes());
    Ok(out)
}

/// WorldCongressBetweenTurns.lua suivi de `ui_congres_attente.lua` : bouton de retour au vote.
pub fn patch_congres_attente(src: &[u8]) -> Result<Vec<u8>, String> {
    let has = |pat: &str| src.windows(pat.len()).any(|w| w == pat.as_bytes());
    if has("Civ6RolloverProxy (src/ui_congres_attente.lua)") {
        return Err("déjà corrigé".into());
    }
    for pat in [
        "local m_kPlayerStages",
        "local m_uiPlayersByID",
        "local COLOR_WAITING",
        "local LOC_PROPOSAL_WAITING",
        "function UpdateCanContinue(",
        "function OnHide(",
    ] {
        if !has(pat) {
            return Err(format!("{pat} introuvable"));
        }
    }
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(ATTENTE_BLOCK.as_bytes());
    Ok(out)
}

/// WorldCongressBetweenTurns.xml : le bouton `BGD_Revoter` sous la description de l'écran d'attente.
pub fn patch_congres_attente_xml(src: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(src).map_err(|_| "texte non UTF-8".to_string())?;
    if text.contains("BGD_Revoter") {
        return Err("déjà corrigé".into());
    }
    let at = text.find("LOC_WORLD_CONGRESS_BETWEEN_TURNS_DESCRIPTION").ok_or("description introuvable")?;
    let end = text[at..].find("/>").ok_or("description non fermée")? + at + 2;
    let mut out = String::with_capacity(text.len() + ATTENTE_XML.len() + 2);
    out.push_str(&text[..end]);
    out.push_str("\r\n");
    out.push_str(ATTENTE_XML.trim_end());
    out.push_str(&text[end..]);
    Ok(out.into_bytes())
}

/// WorldCongressIntro.lua suivi de `ui_congres_intro.lua` : l'intro ne s'ouvre qu'une fois par tour
/// et par session, même si l'annulation de fin de tour d'un autre joueur relance l'événement.
pub fn patch_congres_intro(src: &[u8]) -> Result<Vec<u8>, String> {
    let has = |pat: &str| src.windows(pat.len()).any(|w| w == pat.as_bytes());
    if has("Civ6RolloverProxy (src/ui_congres_intro.lua)") {
        return Err("déjà corrigé".into());
    }
    // Les rappels des événements appellent OnOpen par son nom global : l'enveloppe les atteint.
    for pat in ["function OnOpen(", "OnOpen(1); end", "OnOpen(2); end"] {
        if !has(pat) {
            return Err(format!("{pat} introuvable"));
        }
    }
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(INTRO_BLOCK.as_bytes());
    Ok(out)
}

