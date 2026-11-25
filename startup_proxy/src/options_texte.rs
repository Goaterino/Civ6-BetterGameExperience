//! Textes de l'onglet BetterGameExperience, communs à Windows (options.rs) et macOS
//! (macos_options.rs) : insertion dans Options.xml et Options.lua, fusion du fichier de réglages.
//! Sans appel au système : testés sur les vrais fichiers du jeu (options.rs, tests).

pub const LUA_BLOCK: &str = concat!(
    include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("options_proxy.lua"),
    // Its own function: the locals stay out of Options.lua's chunk and its register limit.
    "\ndo\nlocal ok, err = pcall(function()\n",
    include_str!("../../proxy/src/mods/pings/catalog.lua"), "\n",
    include_str!("../../proxy/src/mods/pings/bindings.lua"), "\n",
    include_str!("../../proxy/src/mods/pings/prefs.lua"), "\n",
    include_str!("ping_volume.lua"),
    "\nend)\nif not ok then print('BGE team pings options: ' .. tostring(err)) end\nend\n"
);
pub(crate) const XML_TAB: &str = include_str!("options_proxy_tab.xml");
pub(crate) const XML_PANEL: &str = include_str!("options_proxy_panel.xml");

/// Points d'insertion dans Options.xml : après le bouton d'onglet `AppTab`, et avant le panneau
/// des raccourcis clavier, dans le conteneur des panneaux.
pub(crate) const TAB_ANCHOR: &str = r#"<GridButton ID="AppTab""#;
pub(crate) const PANEL_ANCHOR: &str = r#"<Container ID="KeyBindings""#;
/// Bouton « Réinitialiser BGE », juste après le bouton Confirmer, sous les panneaux : caché, sauf sur
/// l'onglet BGE, où options_proxy.lua place les deux côte à côte. Sans Confirmer, pas de bouton.
pub(crate) const CONFIRM_ANCHOR: &str = r#"<GridButton ID="ConfirmButton""#;
pub(crate) const XML_RESET: &str = r#"<GridButton ID="ProxyResetButton" Style="ButtonRed" AnchorSide="I,O" Anchor="c,b" Size="300,41" String="" Hidden="1"/>"#;

pub const CFG_NAME: &str = "rollover_proxy.cfg";


// ---------------------------------------------------------------------------------------------
// Textes, sans état : testés sur les vrais fichiers (voir les tests en bas).

pub(crate) fn script_name(chunk: &str) -> String {
    chunk.rsplit(['/', '\\']).next().unwrap_or("").trim_start_matches('@').to_ascii_lowercase()
}

/// Options.xml avec le bouton d'onglet et le panneau.
pub fn patch_options_xml(src: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(src).map_err(|_| "texte non UTF-8".to_string())?;
    if text.contains(r#"ID="ProxyTab""#) {
        return Err("onglet déjà présent".into());
    }
    let tab = text.find(TAB_ANCHOR).ok_or("bouton AppTab introuvable")?;
    let tab_end = text[tab..].find("/>").ok_or("bouton AppTab non fermé")? + tab + 2;
    let panel = text.find(PANEL_ANCHOR).ok_or("panneau KeyBindings introuvable")?;
    if panel < tab_end {
        return Err("panneau KeyBindings avant les onglets".into());
    }
    let confirm_end = text[panel..]
        .find(CONFIRM_ANCHOR)
        .and_then(|c| text[panel + c..].find("/>").map(|e| panel + c + e + 2));
    let mut out = String::with_capacity(text.len() + XML_TAB.len() + XML_PANEL.len() + XML_RESET.len() + 16);
    out.push_str(&text[..tab_end]);
    out.push_str("\r\n");
    out.push_str(XML_TAB.trim_end());
    out.push_str(&text[tab_end..panel]);
    out.push_str(XML_PANEL.trim_end());
    out.push_str("\r\n");
    out.push_str(include_str!("ping_volume.xml"));
    match confirm_end {
        Some(end) => {
            out.push_str(&text[panel..end]);
            out.push_str("\r\n\t\t\t\t");
            out.push_str(XML_RESET);
            out.push_str(&text[end..]);
        }
        None => out.push_str(&text[panel..]),
    }
    Ok(out.into_bytes())
}

/// Ligne du bloc qui dit s'il montre les réglages de diagnostic (`diag = true`) : réécrite en
/// `true` quand le proxy est compilé avec la feature `diag` (builds locaux, pas la release).
pub(crate) const DIAG_LINE: &str = "local PROXY_DIAG = false;";

/// Ligne du bloc qui porte la version affichée dans le titre de l'onglet : réécrite avec celle de la
/// release (`version::RELEASE`), ou celle du salon (`version::VERSION`) pour une compilation locale.
pub(crate) const VERSION_LINE: &str = "local PROXY_VERSION = nil;";

/// Le bloc de l'onglet : version dans le titre, réglages de diagnostic compris avec la feature `diag`.
pub(crate) fn lua_block() -> String {
    let version = super::version::RELEASE.unwrap_or(super::version::VERSION);
    let block = LUA_BLOCK.replacen(VERSION_LINE, &format!("local PROXY_VERSION = \"{version}\";"), 1);
    if cfg!(feature = "diag") {
        block.replacen(DIAG_LINE, "local PROXY_DIAG = true;", 1)
    } else {
        block
    }
}

// Scope the extra actions to the stock key-binding screen. Other consumers of Input
// (including standalone mods) keep the engine object.
pub(crate) const KEY_BINDING_INIT: &[u8] = b"function InitializeKeyBinding()";
pub(crate) const PING_BINDINGS: &str = concat!(
    "\nlocal Input = (function(engine)\n",
    include_str!("../../proxy/src/mods/pings/catalog.lua"), "\n",
    include_str!("../../proxy/src/mods/pings/bindings.lua"), "\n",
    include_str!("ping_bindings.lua"), "\nend)(Input)\n"
);

/// Options.lua suivi du bloc de l'onglet. Octets laissés tels quels : les fichiers du jeu ne sont
/// pas tous en UTF-8.
pub fn patch_options_lua(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    if let Some(at) = src.windows(KEY_BINDING_INIT.len()).position(|w| w == KEY_BINDING_INIT) {
        let cut = at + KEY_BINDING_INIT.len();
        out.extend_from_slice(&src[..cut]);
        out.extend_from_slice(PING_BINDINGS.as_bytes());
        out.extend_from_slice(&src[cut..]);
    } else {
        out.extend_from_slice(src);
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(lua_block().as_bytes());
    out
}

/// Entrées ajoutées au limiteur d'images, après celle du jeu qu'elles suivent : (ancre, ajout).
pub(crate) const TICK_ENTRIES: [(&str, &str); 2] = [
    (r#"{"LOC_OPTIONS_TICK_INTERVAL_30_FPS", 32},"#, r#" {"45 FPS", 21},"#),
    (r#"{"LOC_OPTIONS_TICK_INTERVAL_60_FPS", 16},"#, r#" {"75 FPS", 12}, {"90 FPS", 10}, {"100 FPS", 9},"#),
];

/// Options.lua avec les entrées de `TICK_ENTRIES` dans la liste du limiteur d'images, ou `None` si
/// une ancre manque (autre version du jeu) : la liste reste alors celle du jeu.
pub fn patch_tick_intervals(src: &[u8]) -> Option<Vec<u8>> {
    let mut out = src.to_vec();
    for (anchor, extra) in TICK_ENTRIES {
        let at = out.windows(anchor.len()).position(|w| w == anchor.as_bytes())? + anchor.len();
        out.splice(at..at, extra.bytes());
    }
    Some(out)
}

/// Le fichier de réglages avec les `changes` (`clé=valeur`, une par ligne) : chaque clé prend sa
/// nouvelle valeur à sa première ligne et perd ses lignes suivantes, les autres lignes restent
/// telles quelles, et les clés absentes sont ajoutées à la fin.
pub fn merge_config(existing: &str, changes: &str) -> Result<String, String> {
    let mut pending: Vec<(String, String)> = Vec::new();
    for line in changes.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (k, v) = line.split_once('=').ok_or_else(|| format!("ligne sans « = » : {line}"))?;
        let (k, v) = (k.trim(), v.trim());
        if k.is_empty() || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(format!("clé invalide : {k}"));
        }
        if !v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.') {
            return Err(format!("valeur invalide pour {k} : {v}"));
        }
        pending.retain(|(pk, _)| pk != k);
        pending.push((k.to_string(), v.to_string()));
    }
    // read_config garde la dernière occurrence d'une clé : les suivantes de chaque clé changée sont
    // retirées.
    let mut done: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for line in existing.lines() {
        let key = line.split_once('=').map(|(k, _)| k.trim());
        if key.is_some_and(|k| done.iter().any(|d| d == k)) {
            continue;
        }
        match key.and_then(|k| pending.iter().position(|(pk, _)| pk == k)) {
            Some(i) => {
                let (k, v) = pending.remove(i);
                out.push(format!("{k}={v}"));
                done.push(k);
            }
            None => out.push(line.to_string()),
        }
    }
    out.extend(pending.into_iter().map(|(k, v)| format!("{k}={v}")));
    let mut text = out.join("\r\n");
    text.push_str("\r\n");
    Ok(text)
}

/// Le fichier de réglages sans les lignes des `keys` (une par ligne) : sans elles, le proxy prend
/// leur valeur par défaut. Les autres lignes (commentaires, clés que l'onglet ne connaît pas) restent.
pub fn reset_config(existing: &str, keys: &str) -> Result<String, String> {
    let keys: Vec<&str> = keys.lines().map(str::trim).filter(|k| !k.is_empty()).collect();
    if let Some(k) = keys.iter().find(|k| !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')) {
        return Err(format!("clé invalide : {k}"));
    }
    let kept: Vec<&str> = existing
        .lines()
        .filter(|line| !line.split_once('=').is_some_and(|(k, _)| keys.contains(&k.trim())))
        .collect();
    if kept.iter().all(|l| l.trim().is_empty()) {
        return Ok(String::new());
    }
    let mut text = kept.join("\r\n");
    text.push_str("\r\n");
    Ok(text)
}
