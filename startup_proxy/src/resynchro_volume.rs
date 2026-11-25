//! Hôte seulement : un joueur qui reçoit trop de réparations à un contrôle de synchro de fin de tour
//! fait recharger tout le monde (`reload_all`), au lieu de lui renvoyer chaque tour des archives qui
//! ne convergent pas.
//!
//! Rapports BGE-2890AC et BGE-AEA3D7 (`analyses/passage_de_tour_reseau`) : un joueur revenu en plein
//! tour recevait à chaque contrôle 5,5 à 5,9 Mo de deltas sur 74 à 83 archives (7,24 Mo et 107 au
//! tour de son retour), plus que la sauvegarde entière (4,18 Mo), pendant 30 tours, et la barrière
//! de synchro coûtait environ 45 s de plus par tour. Ces deltas sont surtout ceux du jeu : supprimer
//! `reparation_synchro` n'y changeait rien, et `reload_all` ne se déclenche que pour des objets d'un
//! seul côté. Un joueur sain reçoit 3 à 16 Ko par contrôle, et au plus une dizaine d'archives.
//!
//! **Seuils** (`rollover_proxy.cfg`, lus à chaque contrôle) : `resynchro_volume_ko=500` (Ko de deltas
//! envoyés au joueur à un contrôle) ou `resynchro_volume_archives=30` (archives réparées). Un seul
//! contrôle suffit : un gros delta ne se stabilise pas, autant ne pas perdre de tour.
//! `resynchro_volume=off` coupe tout. Le volume est mesuré par `sync_notes` (taille de chaque delta
//! que le gestionnaire de l'hôte copie dans la réparation).
//!
//! **Garde-fou** : au plus un rechargement de tout le monde par ce chemin tous les [`PERIODE`]
//! tours. Un nouveau dépassement pendant la période (le contrôle de la partie rechargée compris) ne
//! recharge rien : la période devient une pause, annoncée une fois dans le chat avec le tour de
//! reprise. Si un chargement recréait lui-même la divergence, on ne rechargerait donc pas en boucle.
//! Un rechargement déjà en cours (un autre joueur, ou le chemin des objets d'un seul côté) n'est ni
//! compté ni gêné. Le tour vient de `partie.lua` (`Civ6Proxy_ResyncVolume(tour)`) ; un tour plus
//! petit que celui du dernier rechargement (autre partie, sauvegarde plus ancienne) remet la garde
//! à zéro.

use crate::log;
use std::ffi::c_void;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

/// Tours entre deux rechargements de tout le monde déclenchés par le volume.
pub const PERIODE: i32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seuils {
    pub actif: bool,
    pub octets: u64,
    pub archives: u32,
}

impl Default for Seuils {
    fn default() -> Self {
        Seuils { actif: true, octets: 500 * 1024, archives: 30 }
    }
}

/// Les réglages (dernière occurrence de chaque clé).
pub fn seuils(cfg: &str) -> Seuils {
    let mut s = Seuils::default();
    for l in cfg.lines().map(str::trim) {
        if let Some(v) = l.strip_prefix("resynchro_volume=") {
            s.actif = v.trim() != "off";
        } else if let Some(v) = l.strip_prefix("resynchro_volume_ko=") {
            if let Ok(n) = v.trim().parse::<u64>() {
                s.octets = n.max(1) * 1024;
            }
        } else if let Some(v) = l.strip_prefix("resynchro_volume_archives=") {
            if let Ok(n) = v.trim().parse::<u32>() {
                s.archives = n.max(1);
            }
        }
    }
    s
}

pub fn depasse(s: &Seuils, octets: u64, archives: u32) -> bool {
    s.actif && (octets >= s.octets || archives >= s.archives)
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Garde {
    /// Tour du dernier rechargement déclenché ici.
    dernier: Option<i32>,
    /// La pause de cette période a été annoncée.
    annonce: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Recharger tout le monde.
    Recharger,
    /// Dépassement pendant la période : rien, annoncé (reprise au tour donné).
    Pause { reprise: i32 },
    /// Dépassement pendant une pause déjà annoncée : rien.
    Silence,
}

impl Garde {
    /// Un dépassement au tour `tour`.
    pub fn decider(&mut self, tour: i32) -> Decision {
        let Some(t) = self.dernier else { return Decision::Recharger };
        if tour < t {
            *self = Garde::default();
            return Decision::Recharger;
        }
        if tour - t >= PERIODE {
            return Decision::Recharger;
        }
        if std::mem::replace(&mut self.annonce, true) {
            Decision::Silence
        } else {
            Decision::Pause { reprise: t + PERIODE }
        }
    }

    /// Le rechargement a démarré.
    pub fn recharge(&mut self, tour: i32) {
        *self = Garde { dernier: Some(tour), annonce: false };
    }
}

static TOUR: AtomicI32 = AtomicI32::new(-1);
static GARDE: Mutex<Garde> = Mutex::new(Garde { dernier: None, annonce: false });
/// Pour `partie.lua` : "R|joueur|archives|Ko" (rechargement lancé) ou "P|joueur|archives|Ko|reprise".
/// Gardées à travers le rechargement (le Lua de la partie rechargée les lit).
static ANNONCES: Mutex<Vec<String>> = Mutex::new(Vec::new());
const MAX_ANNONCES: usize = 16;

fn annoncer(a: String) {
    let mut g = ANNONCES.lock().unwrap_or_else(|e| e.into_inner());
    if g.len() < MAX_ANNONCES {
        g.push(a);
    }
}

/// Fin d'un contrôle de fin de tour traité par l'hôte (`sync_notes`) : `archives` réparées pour
/// `joueur`, `octets` de deltas.
pub fn apres_controle(joueur: i32, archives: u32, octets: u64) {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    let s = seuils(&cfg);
    if !depasse(&s, octets, archives) {
        return;
    }
    let ko = octets / 1024;
    let tour = TOUR.load(Ordering::Relaxed);
    if tour < 0 || !crate::reload_all::host_ready(&cfg) {
        log(&format!("resync volume: player {joueur}: {archives} archive(s), {ko} KB repaired; no reload of everyone (turn unknown or reload_all unavailable)"));
        return;
    }
    // Un rechargement en cours, ou impossible maintenant : ni compté, ni gêné.
    let etat = unsafe { crate::reload_all::state_for(tour, joueur) };
    if etat != crate::reload_all::CAN_START {
        log(&format!("resync volume: player {joueur}: {archives} archive(s), {ko} KB repaired; reload of everyone not possible now (state {etat})"));
        return;
    }
    let mut garde = GARDE.lock().unwrap_or_else(|e| e.into_inner());
    match garde.decider(tour) {
        Decision::Recharger => {
            let r = unsafe { crate::reload_all::start_for(tour, joueur) };
            if r == crate::reload_all::CAN_START {
                garde.recharge(tour);
                log(&format!("resync volume: player {joueur}: {archives} archive(s), {ko} KB repaired at turn {tour}: the host reloads everyone"));
                annoncer(format!("R|{joueur}|{archives}|{ko}"));
            } else {
                log(&format!("resync volume: player {joueur}: reload of everyone did not start (state {r})"));
            }
        }
        Decision::Pause { reprise } => {
            log(&format!("resync volume: player {joueur}: {archives} archive(s), {ko} KB repaired again at turn {tour}, after the reload of everyone: no automatic reload before turn {reprise}"));
            annoncer(format!("P|{joueur}|{archives}|{ko}|{reprise}"));
        }
        Decision::Silence => {
            log(&format!("resync volume: player {joueur}: {archives} archive(s), {ko} KB repaired at turn {tour} (paused)"));
        }
    }
}

/// `Civ6Proxy_ResyncVolume(tour)` : note le tour en cours ; rend les annonces depuis le dernier
/// appel, séparées par ";" (vide sans rien).
pub(crate) unsafe extern "C-unwind" fn lua_resync(l: *mut c_void) -> i32 {
    let Some(h) = crate::options::hks() else { return 0 };
    let mut len = 0usize;
    let texte = (h.to_lstring)(l, 1, &mut len);
    if !texte.is_null() {
        if let Some(t) = std::str::from_utf8(std::slice::from_raw_parts(texte.cast::<u8>(), len)).ok().and_then(|t| t.trim().parse::<f64>().ok()) {
            TOUR.store(t as i32, Ordering::Relaxed);
        }
    }
    let annonces = std::mem::take(&mut *ANNONCES.lock().unwrap_or_else(|e| e.into_inner())).join(";");
    crate::options::push_str(h, l, &annonces);
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglages() {
        assert_eq!(seuils(""), Seuils::default());
        let s = seuils("resynchro_volume_ko=800\nresynchro_volume_archives=50\nresynchro_volume_ko=abc\n");
        assert_eq!(s, Seuils { actif: true, octets: 800 * 1024, archives: 50 });
        assert!(!seuils("resynchro_volume=off\r\n").actif);
        assert!(seuils("resynchro_volume=off\nresynchro_volume=on\n").actif, "la dernière occurrence l'emporte");
    }

    /// Les cas des rapports : un joueur sain ne déclenche rien, le joueur 6 de BGE-2890AC si.
    #[test]
    fn seuils_des_rapports() {
        let s = Seuils::default();
        assert!(!depasse(&s, 16 * 1024, 10), "joueur sain au plus haut observé");
        assert!(depasse(&s, 5_800 * 1024, 80), "joueur 6, chaque tour");
        assert!(depasse(&s, 7_240 * 1024, 107), "joueur 6, tour du retour");
        assert!(depasse(&s, 100 * 1024, 30), "beaucoup de petites archives");
        assert!(!depasse(&Seuils { actif: false, ..s }, 9_000 * 1024, 200));
    }

    #[test]
    fn un_rechargement_par_periode() {
        let mut g = Garde::default();
        assert_eq!(g.decider(55), Decision::Recharger);
        g.recharge(55);
        // Le contrôle de la partie rechargée dépasse encore : pause annoncée une fois.
        assert_eq!(g.decider(55), Decision::Pause { reprise: 65 });
        assert_eq!(g.decider(56), Decision::Silence);
        assert_eq!(g.decider(64), Decision::Silence);
        // Fin de la période : un nouveau rechargement possible, puis une nouvelle pause.
        assert_eq!(g.decider(65), Decision::Recharger);
        g.recharge(65);
        assert_eq!(g.decider(66), Decision::Pause { reprise: 75 });
    }

    #[test]
    fn tour_plus_petit_remet_a_zero() {
        let mut g = Garde::default();
        g.recharge(80);
        assert_eq!(g.decider(12), Decision::Recharger, "autre partie ou sauvegarde plus ancienne");
        assert_eq!(g, Garde::default());
    }
}
