//! Version de BetterGameDLLs, annoncée aux autres joueurs dans le salon multijoueur par le proxy de
//! démarrage (`startup_proxy/src/salon.rs`). Les correctifs POL (« proxy-only lobbies »), ceux qui
//! changent la simulation, ne s'activeront que si tous les humains de la partie annoncent exactement
//! la même : l'augmenter à chaque version qui change la simulation ou l'échange du salon.

pub const VERSION: &str = "0.10.0";

/// Préfixe de la marque `BgdVersion=<VERSION>` gravée dans le proxy de GameCore (export `BgdVersion`,
/// proxy/src/lib.rs). Les deux DLL compilent ce fichier : le proxy de démarrage compare leur
/// `VERSION` avant de s'annoncer dans le salon, par l'export si ce proxy est chargé, sinon en
/// cherchant la marque dans le fichier.
pub const MARQUE: &str = "BgdVersion=";

/// Numéro de la release GitHub qui a produit ces DLL : `BGD_RELEASE`, posé par le workflow de release
/// (.github/workflows/release.yml). Aucun pour une compilation locale, qui ne cherche donc pas de
/// mise à jour (`startup_proxy/src/maj.rs`).
pub const RELEASE: Option<&str> = match option_env!("BGD_RELEASE") {
    Some(v) if !v.is_empty() => Some(v),
    _ => None,
};
