//! Congrès mondial : revenir au vote après avoir fini son tour, sans payer deux fois. Correctif POL :
//! il change la simulation, il n'est appliqué que si tous les joueurs ont le proxy (voir `pol`).
//! Analyse : congres/README.md.
//!
//! - **Annulation.** Les segments du congrès ont `AllowTurnUnready="false"`. `18005b640` lit ce
//!   drapeau (bit 2 de `+0x50` du segment courant) pour toutes les demandes d'annulation : demande
//!   de l'exe (`18005d8c0`), message `PlayerTurnUnready` (`180039fb0`, par `18005d870`) et demande
//!   locale (`180036e90`), ainsi qu'un troisième chemin (`18005c98a`). La fonction est réécrite à
//!   l'identique, sauf pendant le vote (`TURNSEG_WORLDCONGRESS_1` et `_2`) où elle répond « permis ».
//!   Le reste des conditions du jeu tient toujours : `CanUnreadyTurn` (`1802f76a0`, segment en tours
//!   simultanés, au moins un joueur encore actif) et l'exe, qui refuse si le minuteur du joueur est
//!   écoulé (`1401c9c10`).
//! - **Faveur.** Voter (`1803c0ba0` sur une résolution, `1803c0d40` sur une proposition) débite le
//!   coût des voix et **remplace** les voix du joueur. Un second vote paierait donc deux fois. Les
//!   détours de `spectateurs` appellent `avant_vote` / `avant_discussion` : si le joueur a déjà des
//!   voix sur ce sujet, leur coût lui est rendu avant le nouveau vote (et repris si le jeu refuse le
//!   vote). Le compteur de faveur gagnée (`+0xd8` du joueur), que le crédit augmente, est remis à sa
//!   valeur.

use crate::patch::{self, IMAGE_BASE};
use crate::{log, pol};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

// Hachages des segments (`~CRC32`, comme `DB.MakeHash`).
const WC_1: u32 = 0xb52a_667b;
const WC_2: u32 = 0x2c23_37c1;

// Données de GameCore.
const SEGMENT_COURANT: u64 = 0x1_80b6_17c0; // u32, indice de la ligne TurnSegments du segment
const SEGMENTS: u64 = 0x1_80b8_ad80; // pointeur vers le tableau des lignes
const S_DRAPEAUX: usize = 0x50; // bit 2 : AllowTurnUnready
const S_TYPE: usize = 0x18; // hachage du type de segment

// Fonctions de GameCore appelées par le proxy.
const EST_SEGMENT: u64 = 0x1_8005_b680; // (hachage) -> segment courant de ce type
const INDICE_RESOLUTION: u64 = 0x1_801b_0e00; // (hachage de résolution) -> indice
const FAVEUR_JOUEUR: u64 = 0x1_8026_c770; // (joueur) -> sa faveur
const PARAMETRES_CONGRES: u64 = 0x1_8037_0f30; // () -> paramètres du coût des voix
const COUT_VOIX: u64 = 0x1_8037_1020; // (paramètres, voix) -> coût en faveur
const CHANGER_FAVEUR: u64 = 0x1_8026_c410; // (faveur, delta, raison)
const ECRIRE_VARIABLE: u64 = 0x1_8064_a6d0; // (variable synchronisée, &valeur)
const DISCUSSION: u64 = 0x1_803b_c9c0; // (congrès, proposition) -> élément de m_Discussions

// Décalages.
const C_RESOLUTIONS: usize = 0x118; // éléments de m_Resolutions
const C_NB_RESOLUTIONS: usize = 0x128;
const TAILLE_RESOLUTION: usize = 0x218;
const R_VOIX: usize = 0x104; // i32 par joueur
const D_VOIX: usize = 0x10; // i32 signé par joueur (contre : négatif)
const F_VALEUR: usize = 0xc0; // faveur
const F_GAGNEE: usize = 0xc8; // variable synchronisée de la faveur gagnée (valeur en +0x10)
const F_GAGNEE_VALEUR: usize = 0xd8;
const RAISON_RESOLUTION: u32 = 1; // raisons de 1803c0ba0 et 1803c0d40
const RAISON_DISCUSSION: u32 = 2;

const ANNULABLE: patch::Site = patch::Site {
    name: "annulation de fin de tour permise par le segment",
    address: 0x1_8005_b640,
    // mov eax,[rip+…] / cmp eax,-1 / je / mov ecx,eax / mov rax,[rip+…] : réécrite entièrement.
    expected: &[0x8b, 0x05, 0x7a, 0x61, 0xb0, 0x00, 0x83, 0xf8, 0xff, 0x74, 0x25, 0x8b, 0xc8, 0x48, 0x8b, 0x05, 0x2c, 0xf7, 0xb2, 0x00],
    copy: (0, 0),
    resume: 0,
};

/// Octets attendus en tête des fonctions appelées : une autre version du jeu ne les appelle pas.
const APPELEES: [(u64, &[u8]); 9] = [
    // Même lecture du segment que 18005b640, comparée au hachage.
    (EST_SEGMENT, &[0x8b, 0x05, 0x3a, 0x61, 0xb0, 0x00, 0x83, 0xf8, 0xff, 0x74, 0x24, 0x8b, 0xd0]),
    (INDICE_RESOLUTION, &[0x40, 0x53, 0x48, 0x83, 0xec, 0x20, 0x8b, 0xd9, 0xe8]),
    (FAVEUR_JOUEUR, &[0x40, 0x53, 0x48, 0x83, 0xec, 0x20, 0x48, 0x63, 0xd9, 0xe8]),
    (PARAMETRES_CONGRES, &[0x48, 0x8b, 0x05, 0x31, 0xd3, 0x81, 0x00, 0xc3]),
    (COUT_VOIX, &[0x40, 0x53, 0x48, 0x83, 0xec, 0x20, 0x8b, 0xc2, 0xc7, 0x44, 0x24, 0x40, 0x00, 0x05, 0x00, 0x00]),
    // mov r9d,[rcx+0C0h] : la faveur est bien en +0xc0.
    (CHANGER_FAVEUR, &[0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x6c, 0x24, 0x18, 0x48, 0x89, 0x74, 0x24, 0x20, 0x57, 0x48, 0x83, 0xec, 0x30, 0x44, 0x8b, 0x89, 0xc0, 0x00, 0x00, 0x00]),
    (ECRIRE_VARIABLE, &[0x48, 0x89, 0x5c, 0x24, 0x18, 0x57, 0x48, 0x83, 0xec, 0x30, 0x48, 0x8b, 0x41, 0x08]),
    (DISCUSSION, &[0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xec, 0x20, 0x48, 0x8b, 0x02]),
    // Le vote sur une proposition écrit en +0x10 + 4 × joueur (méthode +0x20 de l'élément).
    (0x1_803c_0950, &[0x83, 0xfa, 0x3f, 0x77, 0x0b, 0x48, 0x63, 0xc2, 0x44, 0x89, 0x44, 0x81, 0x10, 0xb0, 0x01, 0xc3]),
];

static BASE: AtomicUsize = AtomicUsize::new(0);
static POSE: AtomicBool = AtomicBool::new(false);
static ANNULATIONS: AtomicU32 = AtomicU32::new(0);
static REMBOURSEMENTS: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 40;

unsafe fn rd<T: Copy>(at: usize) -> T {
    std::ptr::read_unaligned(at as *const T)
}

fn adresse(base: usize, a: u64) -> usize {
    base + (a - IMAGE_BASE) as usize
}

unsafe fn est_segment(base: usize, h: u32) -> bool {
    let f: unsafe extern "system" fn(u32) -> u8 = std::mem::transmute(adresse(base, EST_SEGMENT));
    f(h) != 0
}

/// Ligne du segment courant, comme `18005b640` : None quand le jeu répond « permis » d'office.
unsafe fn segment_courant(base: usize) -> Option<usize> {
    let indice: u32 = rd(adresse(base, SEGMENT_COURANT));
    if indice == u32::MAX {
        return None;
    }
    let lignes: usize = rd(adresse(base, SEGMENTS));
    let ligne: usize = rd(lignes + indice as usize * 8);
    if ligne == 0 {
        return None;
    }
    let segment: usize = rd(ligne + 0x60);
    (segment != 0).then_some(segment)
}

/// Décision de `annulable` : le drapeau du segment, levé pendant le vote si le correctif est actif.
pub fn decision(drapeau: bool, type_segment: u32, actif: bool) -> bool {
    drapeau || (actif && (type_segment == WC_1 || type_segment == WC_2))
}

/// `18005b640` réécrite. Son argument (le joueur, dans ecx) n'est pas lu, pas plus que par le jeu.
unsafe extern "system" fn hook_annulable() -> u64 {
    let base = BASE.load(Ordering::Relaxed);
    let Some(segment) = segment_courant(base) else { return 1 };
    let drapeau = rd::<u8>(segment + S_DRAPEAUX) >> 2 & 1 != 0;
    let type_segment: u32 = rd(segment + S_TYPE);
    let permis = decision(drapeau, type_segment, !drapeau && actif());
    if permis && !drapeau {
        let k = ANNULATIONS.fetch_add(1, Ordering::Relaxed) + 1;
        if k <= MAX_LOGS {
            log(&format!("congrès : annulation de fin de tour permise pendant le vote ({type_segment:08x}, {k})"));
        }
    }
    permis as u64
}

/// Crédit rendu avant un nouveau vote : de quoi le reprendre si le jeu refuse ce vote.
pub struct Remboursement {
    faveur: usize,
    cout: i32,
    raison: u32,
}

/// Rend au joueur le coût de `voix` déjà comptées, sans toucher à sa faveur gagnée.
unsafe fn rembourser(base: usize, joueur: u32, voix: i32, raison: u32, quoi: &str) -> Option<Remboursement> {
    let voix = voix.unsigned_abs() as i32;
    if voix == 0 {
        return None;
    }
    let faveur_de: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(adresse(base, FAVEUR_JOUEUR));
    let params: unsafe extern "system" fn() -> usize = std::mem::transmute(adresse(base, PARAMETRES_CONGRES));
    let cout_de: unsafe extern "system" fn(usize, i32) -> i32 = std::mem::transmute(adresse(base, COUT_VOIX));
    let changer: unsafe extern "system" fn(usize, i32, u32) = std::mem::transmute(adresse(base, CHANGER_FAVEUR));
    let ecrire: unsafe extern "system" fn(usize, *const i32) = std::mem::transmute(adresse(base, ECRIRE_VARIABLE));
    let faveur = faveur_de(joueur);
    if faveur == 0 {
        return None;
    }
    let cout = cout_de(params(), voix);
    if cout <= 0 {
        return None;
    }
    let gagnee: i32 = rd(faveur + F_GAGNEE_VALEUR);
    let avant: i32 = rd(faveur + F_VALEUR);
    changer(faveur, cout, raison);
    ecrire(faveur + F_GAGNEE, &gagnee);
    let k = REMBOURSEMENTS.fetch_add(1, Ordering::Relaxed) + 1;
    if k <= MAX_LOGS {
        log(&format!("congrès : joueur {joueur} revote sur {quoi} : {voix} voix déjà comptées, {cout} de faveur rendue ({avant} -> {}) ({k})", rd::<i32>(faveur + F_VALEUR)));
    }
    Some(Remboursement { faveur, cout, raison })
}

/// Le jeu a refusé le vote qui suivait le remboursement : on reprend le crédit.
pub unsafe fn reprendre(r: Remboursement) {
    let changer: unsafe extern "system" fn(usize, i32, u32) = std::mem::transmute(adresse(BASE.load(Ordering::Relaxed), CHANGER_FAVEUR));
    changer(r.faveur, -r.cout, r.raison);
    log(&format!("congrès : vote refusé par le jeu, {} de faveur reprise", r.cout));
}

/// Avant `1803c0ba0(congrès, joueur, résolution, voix)` : rembourse les voix déjà comptées.
pub unsafe fn avant_vote(congres: usize, joueur: u32, resolution: u32) -> Option<Remboursement> {
    if !actif() || joueur >= 64 {
        return None;
    }
    let base = BASE.load(Ordering::Relaxed);
    if !est_segment(base, WC_1) {
        return None;
    }
    let indice_de: unsafe extern "system" fn(u32) -> i32 = std::mem::transmute(adresse(base, INDICE_RESOLUTION));
    let indice = indice_de(resolution);
    let (debut, n): (usize, u32) = (rd(congres + C_RESOLUTIONS), rd(congres + C_NB_RESOLUTIONS));
    if debut == 0 || indice < 0 {
        return None;
    }
    let element = (0..n as usize).map(|i| debut + i * TAILLE_RESOLUTION).find(|&e| rd::<i32>(e) == indice)?;
    let voix: i32 = rd(element + R_VOIX + joueur as usize * 4);
    rembourser(base, joueur, voix, RAISON_RESOLUTION, &format!("la résolution {resolution:08x}"))
}

/// Avant `1803c0d40(congrès, joueur, proposition, voix)` : idem pour une proposition ou une urgence.
pub unsafe fn avant_discussion(congres: usize, joueur: u32, discussion: usize) -> Option<Remboursement> {
    if !actif() || joueur >= 64 || discussion == 0 {
        return None;
    }
    let base = BASE.load(Ordering::Relaxed);
    if !est_segment(base, WC_1) && !est_segment(base, WC_2) {
        return None;
    }
    let element_de: unsafe extern "system" fn(usize, usize) -> usize = std::mem::transmute(adresse(base, DISCUSSION));
    let element = element_de(congres, discussion);
    if element == 0 {
        return None;
    }
    let voix: i32 = rd(element + D_VOIX + joueur as usize * 4);
    rembourser(base, joueur, voix, RAISON_DISCUSSION, "une proposition")
}

/// Détours posés et correctifs POL actifs : GameCore et l'interface (`Civ6RolloverProxy_Revote`)
/// décident de la même façon.
pub fn actif() -> bool {
    POSE.load(Ordering::Relaxed) && crate::spectateurs::poses() && pol::actifs()
}

/// Détour posé (l'état POL se décide ensuite, en partie) : l'écran d'attente reçoit son bouton.
pub fn pose() -> bool {
    POSE.load(Ordering::Relaxed)
}

pub unsafe fn on_gamecore_loaded(base: usize) {
    if !crate::spectateurs::poses() {
        log("congrès : détours des votes absents (spectateurs), pas de retour au vote");
        return;
    }
    if let Err(e) = patch::check(base, &[ANNULABLE]) {
        log(&format!("congrès : version inattendue, aucun détour : {e}"));
        return;
    }
    for (a, octets) in APPELEES {
        let have = std::slice::from_raw_parts(adresse(base, a) as *const u8, octets.len());
        if have != octets {
            log(&format!("congrès : version inattendue en {a:#x} ({have:02x?}), aucun détour"));
            return;
        }
    }
    BASE.store(base, Ordering::Relaxed);
    // La fonction est réécrite : le trampoline (vide) n'est jamais appelé.
    let at = adresse(base, ANNULABLE.address);
    match patch::install_at(ANNULABLE.name, at, &[], at, hook_annulable as *const () as usize) {
        Ok(_) => {
            POSE.store(true, Ordering::Relaxed);
            log("congrès : détour posé (annulation de fin de tour pendant le vote, faveur rendue au second vote)");
        }
        Err(e) => log(&format!("congrès : échec de la pose : {e}")),
    }
}

pub unsafe fn on_proxy_unload() {
    if POSE.swap(false, Ordering::Relaxed) {
        let base = BASE.swap(0, Ordering::Relaxed);
        if base != 0 {
            patch::restore(base, &ANNULABLE);
        }
    }
}

/// Nouvelle partie : les journaux recommencent.
pub fn reset() {
    ANNULATIONS.store(0, Ordering::Relaxed);
    REMBOURSEMENTS.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annulation_pendant_le_vote_seulement() {
        // Segment normal : le drapeau du jeu, actif ou non.
        assert!(decision(true, 0x1234, false));
        assert!(!decision(false, 0x1234, true));
        // Vote : permis si le correctif est actif.
        assert!(decision(false, WC_1, true));
        assert!(decision(false, WC_2, true));
        assert!(!decision(false, WC_1, false));
        // Résultats (TURNSEG_WORLDCONGRESS_RESOLUTION) : jamais.
        assert!(!decision(false, 0xd74f_83e5, true));
    }

    #[test]
    fn hachages_des_segments() {
        let h = |s: &str| !crc32(s.as_bytes());
        assert_eq!(h("TURNSEG_WORLDCONGRESS_1"), WC_1);
        assert_eq!(h("TURNSEG_WORLDCONGRESS_2"), WC_2);
    }

    fn crc32(b: &[u8]) -> u32 {
        let mut c = !0u32;
        for &x in b {
            c ^= x as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
            }
        }
        !c
    }
}
