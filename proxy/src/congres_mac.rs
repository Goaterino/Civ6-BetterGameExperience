//! Retour au vote du congrès, sous macOS : le pendant de congres.rs (voir son en-tête). Correctif
//! POL. La règle d'annulation est commune avec Windows (congres_regle.rs) ; les détours de vote de
//! spectateurs_mac.rs appellent `avant_vote` / `avant_discussion` et `reprendre`.
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image :
//! - annulation permise par le segment `0x8ae624()` : indice du segment courant en `[0x138e460]`,
//!   lignes en `[0x1394090]`, segment en `ligne+0x60`, drapeau `AllowTurnUnready` bit 2 de `+0x50`,
//!   type en `+0x18` (comme Windows). Ses trois appelants (`0x9f56a8`, `0x9f5b7c`, `0x9f5dd4`) lui
//!   passent le joueur, qu'elle ne lit pas. Segment courant de ce type : `0x8ae7d4(hachage)` ;
//! - vote sur une résolution `0xc1de04` : indice `0x69241c(hachage)`, résolutions du congrès en
//!   `+0x120` (nombre `+0x130`, éléments de `0x218`, indice en `+0`, voix en `+0x104`) ;
//! - vote sur une proposition `0xc1e0a0(congrès, joueur, proposition, voix)` (opération
//!   `WORLD_CONGRESS_DISCUSSION_VOTE`) : élément `0xc1e1dc(congrès, proposition)`, voix signées en
//!   `+0x10` (méthode `+0x20`, `0xc1fdcc`) ;
//! - faveur `0x8c23b0(joueur)`, valeur en `+0xc8`, faveur gagnée en `+0xe0` (augmentée par un
//!   crédit, suivi des modifications en ligne) ; paramètres `0xade0f4()`, coût des voix
//!   `0xae0f4c(paramètres, voix)`, `0x8c460c(faveur, delta, raison)` (raison 1 : résolution, 2 :
//!   proposition).

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]

use super::congres_regle::{decision, WC_1, WC_2};
use super::journal::log;
use super::{patch, pol};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

struct Fonction {
    nom: &'static str,
    adresse: usize,
    taille: usize,
    empreinte: u64,
}

const ANNULABLE: Fonction = Fonction { nom: "annulation permise", adresse: 0x8ae624, taille: 88, empreinte: 0xadfcfb488f135b92 };
/// Fonctions appelées par le proxy : une autre version du jeu ne les appelle pas.
const APPELEES: [Fonction; 8] = [
    Fonction { nom: "segment courant", adresse: 0x8ae7d4, taille: 84, empreinte: 0xdb8e1615b5618c02 },
    Fonction { nom: "indice de résolution", adresse: 0x69241c, taille: 256, empreinte: 0x59ac8655c1b61cea },
    Fonction { nom: "faveur du joueur", adresse: 0x8c23b0, taille: 48, empreinte: 0xde46ab5703782090 },
    Fonction { nom: "paramètres du congrès", adresse: 0xade0f4, taille: 12, empreinte: 0x06c789058aea6b0d },
    Fonction { nom: "coût des voix", adresse: 0xae0f4c, taille: 136, empreinte: 0xd9e8df81f309ec0b },
    Fonction { nom: "changement de faveur", adresse: 0x8c460c, taille: 460, empreinte: 0x275fcad99abb30f9 },
    Fonction { nom: "élément de proposition", adresse: 0xc1e1dc, taille: 300, empreinte: 0x36221d51ebcc6528 },
    // Le vote sur une proposition écrit en +0x10 + 4 × joueur (méthode +0x20 de l'élément).
    Fonction { nom: "voix d'une proposition", adresse: 0xc1fdcc, taille: 28, empreinte: VOIX_PROPOSITION },
];
// Les deux votes (0xc1de04, 0xc1e0a0) ne sont pas vérifiés ici : spectateurs_mac.rs, posé avant
// (exigé), les a vérifiés puis a réécrit leur début, qui ne correspond donc plus à l'empreinte.
const VOIX_PROPOSITION: u64 = 0x9737_e81b_3ed8_b616;

const EST_SEGMENT: usize = 0x8ae7d4;
const INDICE_RESOLUTION: usize = 0x69241c;
const FAVEUR_JOUEUR: usize = 0x8c23b0;
const PARAMETRES_CONGRES: usize = 0xade0f4;
const COUT_VOIX: usize = 0xae0f4c;
const CHANGER_FAVEUR: usize = 0x8c460c;
const ELEMENT_PROPOSITION: usize = 0xc1e1dc;

const SEGMENT_COURANT: usize = 0x138e460;
const SEGMENTS: usize = 0x1394090;
const L_SEGMENT: usize = 0x60;
const S_DRAPEAUX: usize = 0x50;
const S_TYPE: usize = 0x18;

const C_RESOLUTIONS: usize = 0x120;
const C_NB_RESOLUTIONS: usize = 0x130;
const TAILLE_RESOLUTION: usize = 0x218;
const R_VOIX: usize = 0x104;
const D_VOIX: usize = 0x10;
const F_VALEUR: usize = 0xc8;
const F_GAGNEE: usize = 0xe0;
const RAISON_RESOLUTION: u32 = 1;
const RAISON_DISCUSSION: u32 = 2;

static BASE: AtomicUsize = AtomicUsize::new(0);
static POSE: AtomicBool = AtomicBool::new(false);
static ANNULATIONS: AtomicU32 = AtomicU32::new(0);
static REMBOURSEMENTS: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 40;

fn gc(addr: usize) -> usize {
    BASE.load(Ordering::Relaxed) + addr
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

unsafe fn est_segment(h: u32) -> bool {
    let f: unsafe extern "C" fn(u32) -> u32 = std::mem::transmute(gc(EST_SEGMENT));
    f(h) & 1 != 0
}

/// Segment courant, comme `0x8ae624` : None quand le jeu répond « permis » d'office.
unsafe fn segment_courant() -> Option<usize> {
    let indice: u32 = rd(gc(SEGMENT_COURANT));
    if indice == u32::MAX {
        return None;
    }
    let lignes: usize = rd(gc(SEGMENTS));
    let ligne: usize = rd(lignes + indice as usize * 8);
    if ligne == 0 {
        return None;
    }
    let segment: usize = rd(ligne + L_SEGMENT);
    (segment != 0).then_some(segment)
}

/// `0x8ae624` réécrite. Son argument (le joueur) n'est pas lu, pas plus que par le jeu.
unsafe extern "C" fn hook_annulable() -> u32 {
    let Some(segment) = segment_courant() else { return 1 };
    let drapeau = rd::<u8>(segment + S_DRAPEAUX) >> 2 & 1 != 0;
    let type_segment: u32 = rd(segment + S_TYPE);
    let permis = decision(drapeau, type_segment, !drapeau && actif());
    if permis && !drapeau {
        let k = ANNULATIONS.fetch_add(1, Ordering::Relaxed) + 1;
        if k <= MAX_LOGS {
            log(&format!("congrès : annulation de fin de tour permise pendant le vote ({type_segment:08x}, {k})"));
        }
    }
    permis as u32
}

/// Crédit rendu avant un nouveau vote : de quoi le reprendre si le jeu refuse ce vote.
pub struct Remboursement {
    faveur: usize,
    cout: i32,
    raison: u32,
}

/// Rend au joueur le coût de `voix` déjà comptées, sans toucher à sa faveur gagnée.
unsafe fn rembourser(joueur: u32, voix: i32, raison: u32, quoi: &str) -> Option<Remboursement> {
    let voix = voix.unsigned_abs() as i32;
    if voix == 0 {
        return None;
    }
    let faveur_de: unsafe extern "C" fn(u32) -> usize = std::mem::transmute(gc(FAVEUR_JOUEUR));
    let params: unsafe extern "C" fn() -> usize = std::mem::transmute(gc(PARAMETRES_CONGRES));
    let cout_de: unsafe extern "C" fn(usize, i32) -> i32 = std::mem::transmute(gc(COUT_VOIX));
    let changer: unsafe extern "C" fn(usize, i32, u32) = std::mem::transmute(gc(CHANGER_FAVEUR));
    let faveur = faveur_de(joueur);
    if faveur == 0 {
        return None;
    }
    let cout = cout_de(params(), voix);
    if cout <= 0 {
        return None;
    }
    let gagnee: i32 = rd(faveur + F_GAGNEE);
    let avant: i32 = rd(faveur + F_VALEUR);
    changer(faveur, cout, raison);
    // Le crédit a marqué la faveur gagnée modifiée (suivi en ligne) : on remet sa valeur.
    std::ptr::write_unaligned((faveur + F_GAGNEE) as *mut i32, gagnee);
    let k = REMBOURSEMENTS.fetch_add(1, Ordering::Relaxed) + 1;
    if k <= MAX_LOGS {
        log(&format!(
            "congrès : joueur {joueur} revote sur {quoi} : {voix} voix déjà comptées, {cout} de faveur rendue ({avant} -> {}) ({k})",
            rd::<i32>(faveur + F_VALEUR)
        ));
    }
    Some(Remboursement { faveur, cout, raison })
}

/// Le jeu a refusé le vote qui suivait le remboursement : on reprend le crédit.
pub unsafe fn reprendre(r: Remboursement) {
    let changer: unsafe extern "C" fn(usize, i32, u32) = std::mem::transmute(gc(CHANGER_FAVEUR));
    changer(r.faveur, -r.cout, r.raison);
    log(&format!("congrès : vote refusé par le jeu, {} de faveur reprise", r.cout));
}

/// Avant `0xc1de04(congrès, joueur, résolution, voix)` : rembourse les voix déjà comptées.
pub unsafe fn avant_vote(congres: usize, joueur: u32, resolution: u32) -> Option<Remboursement> {
    if !actif() || joueur >= 64 || !est_segment(WC_1) {
        return None;
    }
    let indice_de: unsafe extern "C" fn(u32) -> i32 = std::mem::transmute(gc(INDICE_RESOLUTION));
    let indice = indice_de(resolution);
    let (debut, n): (usize, u32) = (rd(congres + C_RESOLUTIONS), rd(congres + C_NB_RESOLUTIONS));
    if debut == 0 || indice < 0 {
        return None;
    }
    let element = (0..n as usize).map(|i| debut + i * TAILLE_RESOLUTION).find(|&e| rd::<i32>(e) == indice)?;
    let voix: i32 = rd(element + R_VOIX + joueur as usize * 4);
    rembourser(joueur, voix, RAISON_RESOLUTION, &format!("la résolution {resolution:08x}"))
}

/// Avant `0xc1e0a0(congrès, joueur, proposition, voix)` : idem pour une proposition ou une urgence.
pub unsafe fn avant_discussion(congres: usize, joueur: u32, discussion: usize) -> Option<Remboursement> {
    if !actif() || joueur >= 64 || discussion == 0 || !(est_segment(WC_1) || est_segment(WC_2)) {
        return None;
    }
    let element_de: unsafe extern "C" fn(usize, usize) -> usize = std::mem::transmute(gc(ELEMENT_PROPOSITION));
    let element = element_de(congres, discussion);
    if element == 0 {
        return None;
    }
    let voix: i32 = rd(element + D_VOIX + joueur as usize * 4);
    rembourser(joueur, voix, RAISON_DISCUSSION, "une proposition")
}

/// Détours posés et correctifs POL actifs : GameCore et l'interface (`Civ6RolloverProxy_Revote`)
/// décident de la même façon.
pub fn actif() -> bool {
    POSE.load(Ordering::Relaxed) && super::spectateurs::poses() && pol::actifs()
}

/// Détour posé (l'état POL se décide ensuite, en partie) : l'écran d'attente reçoit son bouton.
pub fn pose() -> bool {
    POSE.load(Ordering::Relaxed)
}

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    if !super::spectateurs::poses() {
        return Err("détours des votes absents (spectateurs), pas de retour au vote".into());
    }
    for f in std::iter::once(&ANNULABLE).chain(APPELEES.iter()) {
        if fnv(std::slice::from_raw_parts((base + f.adresse) as *const u8, f.taille)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    BASE.store(base, Ordering::Relaxed);
    // La fonction est réécrite : pas de trampoline, le saut couvre son début.
    let methode = patch::write_code(base + ANNULABLE.adresse, &patch::jump(hook_annulable as *const () as usize))?;
    POSE.store(true, Ordering::Relaxed);
    Ok(format!("annulation de fin de tour pendant le vote, faveur rendue au second vote ({methode})"))
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(_base: usize) -> Result<String, String> {
    Err("pas encore relevé en x86_64".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonctions_du_jeu() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::super::macho::tranche_arm64(&b).expect("tranche arm64");
        for f in std::iter::once(&ANNULABLE).chain(APPELEES.iter()) {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
        }
        assert!(patch::jump(0).len() <= ANNULABLE.taille, "le saut tient dans la fonction");
        let mot = |a: usize| u32::from_le_bytes(t[a..a + 4].try_into().unwrap());
        // Les trois appelants de l'annulation.
        for site in [0x9f56a8, 0x9f5b7c, 0x9f5dd4] {
            assert_eq!(patch::cible_branche(site, mot(site)), Some(ANNULABLE.adresse), "appel en {site:#x}");
        }
    }
}
