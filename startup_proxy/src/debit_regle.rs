//! Règles des gros morceaux réseau, communes à Windows (debit_steam.rs) et à macOS
//! (macos_debit.rs) : tailles, verdict de `Civ6Proxy_MorceauxReseau` et suffixe de la version
//! annoncée. Aucun accès au jeu ici.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

/// Taille des morceaux de la release, et valeur de `morceaux_reseau` quand la clé manque.
pub const DEFAULT_CHUNK: u32 = 16384;
/// Taille des morceaux du jeu (0x48e).
pub const GAME_CHUNK: u32 = 1166;
/// Tampon de réception agrandi : marge pour l'en-tête du transport.
pub const RECV_BUFFER: u32 = 0x2_0000;
/// Un pair hors liste vu depuis moins longtemps empêche les gros morceaux.
pub const STRANGER_MEMORY: Duration = Duration::from_secs(30);

/// Liste `"id,id"` de SteamID ; vide : taille du jeu. Gros morceaux seulement en partie réseau avec
/// les correctifs POL actifs, et sans pair hors liste vu récemment.
pub fn decide(ids: &str, pol: bool, network: bool, hooked: bool, approved: &mut BTreeSet<u64>, strangers: &mut BTreeMap<u64, Instant>, now: Instant) -> Result<(), String> {
    approved.clear();
    approved.extend(ids.split(',').filter_map(|s| s.trim().parse::<u64>().ok()));
    strangers.retain(|id, seen| !approved.contains(id) && now.duration_since(*seen) < STRANGER_MEMORY);
    if approved.is_empty() {
        return Err("aucun joueur équipé listé".into());
    }
    if !pol || !network {
        return Err("correctifs POL inactifs ou partie locale".into());
    }
    if !hooked {
        return Err("SteamNetworking non détourné".into());
    }
    if let Some(id) = strangers.keys().next() {
        return Err(format!("pair {} hors de la liste vu récemment", id & 0xffff_ffff));
    }
    Ok(())
}

/// Suffixe de la version annoncée : `-t0` si le tampon de réception n'a pas pu être agrandi (cette
/// machine planterait sur les gros morceaux des autres) ; sinon rien, et `-m<taille>` avec
/// `morceaux_reseau` dans un build de diagnostic (`diag`, `configured` non nul).
pub fn suffix(buffer_ok: bool, configured: u32, diag: bool) -> String {
    if !buffer_ok {
        return "-t0".into();
    }
    match configured {
        n if n != 0 && diag => format!("-m{n}"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffixe() {
        assert_eq!(suffix(false, DEFAULT_CHUNK, false), "-t0");
        assert_eq!(suffix(false, DEFAULT_CHUNK, true), "-t0");
        assert_eq!(suffix(true, DEFAULT_CHUNK, false), "");
        assert_eq!(suffix(true, DEFAULT_CHUNK, true), "-m16384");
        assert_eq!(suffix(true, 0, true), "");
    }
}
