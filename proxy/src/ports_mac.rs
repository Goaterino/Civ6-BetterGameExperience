//! Téléportation vers une ville au départ d'un port, sous macOS : le pendant de ports.rs (voir son
//! en-tête et analyses/teleportation/README.md). Correctif POL.
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image : la vtable de
//! `N8GameCore4Unit10Operations14TeleportToCityE` a la même disposition que sous Windows, décalée
//! d'une entrée (deux destructeurs) : `CanStart` en `0xbe663c`, le droit à l'opération en
//! `0xbe6aac` (propriété `3f9140c1`), la destination en `0xbe6b9c`. `CanStart` prend l'objet sous
//! l'unité (`contexte->vtable[0x38]`) et appelle en `0xbe66d8` le test du centre-ville
//! (`0xa8c674`, district `{propriétaire +0x1c, genre 3, id +0x10}` par `vtable[8]`, type
//! `8a32007d`), l'unité étant dans `x20`. Sur Mac, le propriétaire de l'unité est en `+0x130` et
//! son domaine en `+0x2a8` (lu par la destination : 0, `DOMAIN_SEA`, ouvre les districts).
//!
//! Ce seul `bl` est redirigé vers un relais `mov x1, x20` puis saut vers `depart_valide`.

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]

use super::journal::log;
use super::{patch, pol};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const SITE: usize = 0xbe66d8;
const EST_CENTRE_VILLE: usize = 0xa8c674;
/// Début de `EST_CENTRE_VILLE` : son cadre de pile.
const CENTRE_OCTETS: [u8; 16] = [0xff, 0xc3, 0x00, 0xd1, 0xf4, 0x4f, 0x01, 0xa9, 0xfd, 0x7b, 0x02, 0xa9, 0xfd, 0x83, 0x00, 0x91];
/// `blr x8` (l'objet sous l'unité) avant le site, `cbz w0, …` après.
const AVANT: u32 = 0xd63f_0100;
const APRES: [u8; 4] = [0xc0, 0x0d, 0x00, 0x34];

const U_PROPRIETAIRE: usize = 0x130;
const U_DOMAINE: usize = 0x2a8;
const DOMAIN_SEA: i32 = 0;
const O_ID: usize = 0x10;
const O_PROPRIETAIRE: usize = 0x1c;
const GENRE_DISTRICT: i16 = 3;

static BASE: AtomicUsize = AtomicUsize::new(0);
static DEPARTS: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

fn gc(addr: usize) -> usize {
    BASE.load(Ordering::Relaxed) + addr
}

unsafe fn rd<T: Copy>(addr: usize) -> T {
    std::ptr::read_unaligned(addr as *const T)
}

/// District désigné par l'objet, retrouvé comme le fait `EST_CENTRE_VILLE`.
unsafe fn district(objet: usize) -> usize {
    #[repr(C)]
    struct Id {
        proprietaire: i16,
        genre: i16,
        id: u32,
    }
    let cle = Id { proprietaire: rd::<i8>(objet + O_PROPRIETAIRE) as i16, genre: GENRE_DISTRICT, id: rd::<u32>(objet + O_ID) };
    let vtable: usize = rd(objet);
    let chercher: unsafe extern "C" fn(usize, *const Id) -> usize = std::mem::transmute(rd::<usize>(vtable + 8));
    chercher(objet, &cle)
}

/// Remplace `EST_CENTRE_VILLE` dans `CanStart` : centre-ville, ou tout district pour une unité
/// navale, comme à l'arrivée. Rend 0 ou 1 sur 32 bits (l'appelant teste `w0` entier).
unsafe extern "C" fn depart_valide(objet: usize, unite: usize) -> u32 {
    let original: unsafe extern "C" fn(usize) -> u32 = std::mem::transmute(gc(EST_CENTRE_VILLE));
    if original(objet) & 1 != 0 {
        return 1;
    }
    if objet == 0 || unite == 0 || rd::<i32>(unite + U_DOMAINE) != DOMAIN_SEA || !pol::actifs() {
        return 0;
    }
    if district(objet) == 0 {
        return 0;
    }
    let k = DEPARTS.fetch_add(1, Ordering::Relaxed) + 1;
    if k <= MAX_LOGS {
        log(&format!(
            "ports : départ accepté depuis un district hors centre-ville (joueur {}, district {:#x}) ({k})",
            rd::<i32>(unite + U_PROPRIETAIRE),
            rd::<u32>(objet + O_ID)
        ));
    }
    1
}

unsafe fn site_attendu() -> bool {
    let at = gc(SITE);
    let mot = rd::<u32>(at);
    mot & 0x8000_0000 != 0
        && patch::cible_branche(at, mot) == Some(gc(EST_CENTRE_VILLE))
        && rd::<u32>(at - 4) == AVANT
        && rd::<[u8; 4]>(at + 4) == APRES
        && rd::<[u8; 16]>(gc(EST_CENTRE_VILLE)) == CENTRE_OCTETS
}

/// `mov x1, x20` (l'unité), puis saut vers `depart_valide`.
fn relais() -> Vec<u8> {
    patch::relais(&[0xaa14_03e1], depart_valide as *const () as usize)
}

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    BASE.store(base, Ordering::Relaxed);
    if !site_attendu() {
        return Err(format!("version inattendue en {SITE:#x}, aucun appel redirigé"));
    }
    let page = patch::executable_near(&relais(), gc(SITE), 120 << 20)?;
    let mut bl = patch::branche(gc(SITE), page).ok_or("relais hors de portée, aucun appel redirigé")?;
    bl[3] |= 0x80; // b → bl
    let methode = patch::write_code(gc(SITE), &bl)?;
    Ok(format!("départ de TELEPORT_TO_CITY accepté depuis un district (appel redirigé en {SITE:#x}, {methode})"))
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(_base: usize) -> Result<String, String> {
    Err("pas encore relevé en x86_64".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relais_arm64() {
        let r = relais();
        assert_eq!(&r[..12], &[0xe1, 0x03, 0x14, 0xaa, 0x50, 0x00, 0x00, 0x58, 0x00, 0x02, 0x1f, 0xd6]);
        let mut bl = patch::branche(SITE, EST_CENTRE_VILLE).unwrap();
        bl[3] |= 0x80;
        assert_eq!(bl, [0xe7, 0x97, 0xfa, 0x97], "le bl d'origine, réencodé");
    }

    #[test]
    fn site_du_jeu() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::super::macho::tranche_arm64(&b).expect("tranche arm64");
        let mot = |a: usize| u32::from_le_bytes(t[a..a + 4].try_into().unwrap());
        assert_eq!(patch::cible_branche(SITE, mot(SITE)), Some(EST_CENTRE_VILLE));
        assert_eq!(mot(SITE - 4), AVANT);
        assert_eq!(&t[SITE + 4..SITE + 8], &APRES);
        assert_eq!(&t[EST_CENTRE_VILLE..EST_CENTRE_VILLE + 16], &CENTRE_OCTETS);
    }
}
