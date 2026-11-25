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
//!
//! En x86_64 (Mac Intel, Rosetta), mêmes décalages de structure :
//! - annulation permise `0x891aa0` (bit 2 de `+0x50`, appelée en `0x9eeb0e`, `0x9eecfc`,
//!   `0x9eef82`), segment courant `[0x13da2f0]`, lignes `[0x13e0050]` ; segment de ce type
//!   `0x891c40` (28 appelants, comme `0x8ae7d4`) ;
//! - indice de résolution `0x696400`, faveur `0x8a6c40`, paramètres `0xae7190`, coût des voix
//!   `0xaea280`, changement de faveur `0x8a9240` (relevés dans le vote `0xc38740`), élément de
//!   proposition `0xc38b40` et sa méthode `+0x20` `0xc3a800` (relevés dans `0xc38a00`).
//! Les booléens rendus par le jeu le sont dans `al` : la réécriture rend 0 ou 1 dans `eax`.

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]
// Les relevés x86_64 ne servent, en arm64, qu'aux tests qui lisent le jeu installé.
#![cfg_attr(target_arch = "aarch64", allow(dead_code))]

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

/// x86_64 : la fonction réécrite, puis celles qu'appelle le proxy (mêmes rôles, même ordre).
const X86_ANNULABLE: Fonction = Fonction { nom: "annulation permise", adresse: 0x891aa0, taille: 0x60, empreinte: 0xf3f0_6f4d_440c_d2f3 };
const X86_APPELEES: [Fonction; 8] = [
    Fonction { nom: "segment courant", adresse: 0x891c40, taille: 0x60, empreinte: 0xae37_e9f2_6805_cc73 },
    Fonction { nom: "indice de résolution", adresse: 0x696400, taille: 0x110, empreinte: 0x632b_1076_8729_98b8 },
    Fonction { nom: "faveur du joueur", adresse: 0x8a6c40, taille: 0x30, empreinte: 0x07e2_d63e_2450_095a },
    Fonction { nom: "paramètres du congrès", adresse: 0xae7190, taille: 0x10, empreinte: 0x245c_ba0a_4357_3c9b },
    Fonction { nom: "coût des voix", adresse: 0xaea280, taille: 0x80, empreinte: 0x3f22_72f1_acd1_284c },
    Fonction { nom: "changement de faveur", adresse: 0x8a9240, taille: 0x1d0, empreinte: 0x6952_ef22_292e_fdbf },
    Fonction { nom: "élément de proposition", adresse: 0xc38b40, taille: 0x130, empreinte: 0x4b9a_46b9_b6f8_8dc4 },
    Fonction { nom: "voix d'une proposition", adresse: 0xc3a800, taille: 0x20, empreinte: 0xa2d8_d891_0077_25db },
];
// Comme en arm64, les votes (0xc38740, 0xc38a00) sont laissés à spectateurs_mac.rs.

/// Adresses x86_64 (voir l'en-tête).
const X86_EST_SEGMENT: usize = 0x891c40;
const X86_INDICE_RESOLUTION: usize = 0x696400;
const X86_FAVEUR_JOUEUR: usize = 0x8a6c40;
const X86_PARAMETRES_CONGRES: usize = 0xae7190;
const X86_COUT_VOIX: usize = 0xaea280;
const X86_CHANGER_FAVEUR: usize = 0x8a9240;
const X86_ELEMENT_PROPOSITION: usize = 0xc38b40;
const X86_SEGMENT_COURANT: usize = 0x13da2f0;
const X86_SEGMENTS: usize = 0x13e0050;
const EST_SEGMENT: usize = if cfg!(target_arch = "x86_64") { X86_EST_SEGMENT } else { 0x8ae7d4 };
const INDICE_RESOLUTION: usize = if cfg!(target_arch = "x86_64") { X86_INDICE_RESOLUTION } else { 0x69241c };
const FAVEUR_JOUEUR: usize = if cfg!(target_arch = "x86_64") { X86_FAVEUR_JOUEUR } else { 0x8c23b0 };
const PARAMETRES_CONGRES: usize = if cfg!(target_arch = "x86_64") { X86_PARAMETRES_CONGRES } else { 0xade0f4 };
const COUT_VOIX: usize = if cfg!(target_arch = "x86_64") { X86_COUT_VOIX } else { 0xae0f4c };
const CHANGER_FAVEUR: usize = if cfg!(target_arch = "x86_64") { X86_CHANGER_FAVEUR } else { 0x8c460c };
const ELEMENT_PROPOSITION: usize = if cfg!(target_arch = "x86_64") { X86_ELEMENT_PROPOSITION } else { 0xc1e1dc };

const SEGMENT_COURANT: usize = if cfg!(target_arch = "x86_64") { X86_SEGMENT_COURANT } else { 0x138e460 };
const SEGMENTS: usize = if cfg!(target_arch = "x86_64") { X86_SEGMENTS } else { 0x1394090 };
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
pub unsafe fn installer(base: usize) -> Result<String, String> {
    if !super::spectateurs::poses() {
        return Err("détours des votes absents (spectateurs), pas de retour au vote".into());
    }
    for f in std::iter::once(&X86_ANNULABLE).chain(X86_APPELEES.iter()) {
        if fnv(std::slice::from_raw_parts((base + f.adresse) as *const u8, f.taille)) != f.empreinte {
            return Err(format!("version inattendue ({} en {:#x}), aucun détour", f.nom, f.adresse));
        }
    }
    BASE.store(base, Ordering::Relaxed);
    // La fonction est réécrite : pas de trampoline, le saut (14 octets) couvre son début.
    let methode = patch::write_code(base + X86_ANNULABLE.adresse, &patch::jump(hook_annulable as *const () as usize))?;
    POSE.store(true, Ordering::Relaxed);
    Ok(format!("annulation de fin de tour pendant le vote, faveur rendue au second vote ({methode})"))
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

    #[test]
    fn fonctions_du_jeu_x86_64() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::super::spectateurs::tranche_x86_64(&b).expect("tranche x86_64");
        for f in std::iter::once(&X86_ANNULABLE).chain(X86_APPELEES.iter()) {
            let e = fnv(&t[f.adresse..f.adresse + f.taille]);
            assert_eq!(e, f.empreinte, "{} : empreinte {e:#x}", f.nom);
        }
        assert!(14 <= X86_ANNULABLE.taille, "le saut tient dans la fonction");
        let appel = |a: usize| patch::cible_rel32_x86(a, &t[a..a + 5]);
        let rip = |a: usize| (a as i64 + 4 + i32::from_le_bytes(t[a..a + 4].try_into().unwrap()) as i64) as usize;
        // Les trois appelants de l'annulation.
        for site in [0x9eeb0e, 0x9eecfc, 0x9eef82] {
            assert_eq!(appel(site), Some(X86_ANNULABLE.adresse), "appel en {site:#x}");
        }
        // mov eax, [rip + segment courant] ; mov rcx, [rip + lignes], dans l'annulation.
        assert_eq!(rip(0x891aa6), X86_SEGMENT_COURANT);
        assert_eq!(rip(0x891ab7), X86_SEGMENTS);
        // Le vote appelle indice, faveur, paramètres, coût et changement de faveur.
        for (site, cible) in [(0xc3875c, X86_INDICE_RESOLUTION), (0xc38957, X86_FAVEUR_JOUEUR), (0xc389b0, X86_PARAMETRES_CONGRES), (0xc389bb, X86_COUT_VOIX), (0xc389cc, X86_CHANGER_FAVEUR), (0xc38a19, X86_ELEMENT_PROPOSITION)] {
            assert_eq!(appel(site), Some(cible), "appel en {site:#x}");
        }
        // Résolutions en +0x120 (nombre +0x130, éléments de 0x218), faveur en +0xc8 et +0xe0.
        assert_eq!(t[0xc388a8..0xc388bc], [0x8b, 0x81, 0x30, 0x01, 0, 0, 0x48, 0x69, 0xc0, 0x18, 0x02, 0, 0, 0x48, 0x03, 0x81, 0x20, 0x01, 0, 0]);
        assert_eq!(t[0xc38964..0xc3896a], [0x8b, 0xb3, 0xc8, 0, 0, 0]);
        assert_eq!(t[0x8a9381..0x8a9387], [0x89, 0x99, 0xe0, 0, 0, 0]);
        // Méthode +0x20 de l'élément : voix en +0x10 + 4 × joueur.
        assert_eq!(t[0xc3a80b..0xc3a80f], [0x89, 0x54, 0x87, 0x10]);
    }
}
