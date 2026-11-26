//! Version de BetterGameDLLs, annoncée aux autres joueurs dans le salon multijoueur par le proxy de
//! démarrage (`startup_proxy/src/salon.rs`). Les correctifs POL (« proxy-only lobbies »), ceux qui
//! changent la simulation, ne s'activeront que si tous les humains de la partie annoncent exactement
//! la même : l'augmenter à chaque version qui change la simulation ou l'échange du salon.

pub const VERSION: &str = "0.15.0";

/// Niveau de la protection des départs : un joueur qui part garde un emplacement humain
/// (proxy/src/drop_slot.rs), l'hôte finit ses tours (startup_proxy/src/absent_turn.rs) et chaque
/// machine exécute cette fin de tour de la même façon (proxy/src/absent_reactivation.rs). Annoncé
/// dans le salon à côté de la version (startup_proxy/src/salon.lua) : cette protection s'applique
/// aussi entre versions différentes, si tous les humains annoncent le même niveau. L'augmenter à
/// chaque changement de ce que font ces trois modules, comme `VERSION` pour les correctifs POL.
pub const NIVEAU_DEPARTS: u32 = 1;

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
