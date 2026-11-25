//! Diplomatie reçue pendant le passage de tour, sous macOS : le pendant de fin_tour.rs (voir son
//! en-tête pour le défaut et le correctif). Correctif POL.
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image :
//! - `EndTurn` (`0x6428a0`, chaîne `Game Turn Ended`) lève le drapeau de fin de tour (`0x13936e5`),
//!   `BeginTurn` (`0x642338`, `Game Turn: %i`) le baisse, `0x643808` le lit (`ldrb w0` puis `ret`) ;
//! - l'`Execute` des messages est la cinquième entrée de leur vtable, retrouvée par leur nom RTTI
//!   (`N8GameCore8Messages15DiplomacyActionE`…). `GameUpdate::Execute` (`0x84761c`) lit le drapeau
//!   avant l'étape de mise à jour (`0x42d2c0`, `Game::Processor::Update` en `0x642e9c` puis deux
//!   autres) ; les trois `Execute` de diplomatie finissent par un saut vers elle sans le lire :
//!   `DiplomacyAction` en `0x846138`, `DiplomacyDeal` en `0x8464c4`, `DiplomacySession` en
//!   `0x846a60`, chacun juste après `bl 0x42aea8` (le jeu, rendu dans `x0`) et la restauration de
//!   la pile (deux instructions pour `DiplomacySession`).
//!
//! Les trois sauts sont redirigés vers des relais (`mov w1, #quel` puis saut absolu vers
//! `etape_apres_diplomatie`), placés à moins de 128 Mo pour rester à portée d'un `b`.
//!
//! En x86_64 (Rosetta, Mac Intel), même chemin, autre compilation (module `x86`) : `EndTurn`
//! (`0x642670`) lève le drapeau `0x13df675`, `BeginTurn` (`0x6420f0`) le baisse, `0x6437a0` le lit
//! (`movzbl` puis `ret`), `GameUpdate::Execute` (`0x8258d0`) l'appelle avant l'étape (`0x425870`,
//! `call 0x642c80` puis deux autres). Les trois `Execute` (`0x824540`, `0x824880`, `0x824d40`)
//! finissent par `call 0x423240` (le jeu, dans `rax`), `mov rdi, rax`, les `pop`, puis `jmp rel32`
//! vers l'étape : en `0x82455e`, `0x8248a4` et `0x824e1b`. Chaque `jmp` est redirigé vers un relais
//! (`mov esi, quel` puis `jmp [rip+0]`) à moins de 1 Go, à portée du `rel32`.

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]

use super::journal::log;
use super::{patch, pol};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const ETAPE: usize = 0x42d2c0;
/// `stp x29, x30, [sp, #-0x10]!` puis `mov x29, sp`.
const ETAPE_OCTETS: [u8; 8] = [0xfd, 0x7b, 0xbf, 0xa9, 0xfd, 0x03, 0x00, 0x91];
const TOUR_FINI: usize = 0x643808;
/// `adrp x8, …` / `ldrb w0, [x8, #0x6e5]` / `ret`.
const TOUR_FINI_OCTETS: [u8; 12] = [0x88, 0x6a, 0x00, 0x90, 0x00, 0x95, 0x5b, 0x39, 0xc0, 0x03, 0x5f, 0xd6];
/// Rend le jeu (`adrp`, `add`, `ldr x0`, `ret`), appelée juste avant chaque site.
const JEU: usize = 0x42aea8;

struct Appel {
    message: &'static str,
    /// `b ETAPE`, précédé de la restauration de la pile (une ou deux instructions)…
    site: usize,
    /// … elle-même précédée de `bl JEU`, à cet écart avant le site.
    jeu: usize,
}

const APPELS: [Appel; 3] = [
    Appel { message: "DiplomacyAction", site: 0x846138, jeu: 8 },
    Appel { message: "DiplomacyDeal", site: 0x8464c4, jeu: 8 },
    Appel { message: "DiplomacySession", site: 0x846a60, jeu: 12 },
];

/// Écart entre deux relais.
const RELAIS: usize = 32;

/// Relevé dans la tranche x86_64 de GameCore_XP2 (Civ6 Aspyr 1.4.6), adresses relatives à l'image.
#[cfg_attr(target_arch = "aarch64", allow(dead_code))]
mod x86 {
    pub const ETAPE: usize = 0x425870;
    /// L'étape entière (`push rbp` … `call` … `call` … `pop rbp`, `jmp`) : FNV-1a 64.
    pub const ETAPE_TAILLE: usize = 20;
    pub const ETAPE_EMPREINTE: u64 = 0x1715_735e_298b_5ca6;
    pub const TOUR_FINI: usize = 0x6437a0;
    /// `push rbp` / `mov rbp, rsp` / `movzbl eax, [rip+…]` (le drapeau) / `pop rbp` / `ret`.
    pub const TOUR_FINI_OCTETS: [u8; 13] = [0x55, 0x48, 0x89, 0xe5, 0x0f, 0xb6, 0x05, 0xca, 0xbe, 0xd9, 0x00, 0x5d, 0xc3];
    /// Rend le jeu (`lea rax, [rip+…]` / `mov rax, [rax]`), appelée juste avant chaque site.
    pub const JEU: usize = 0x423240;

    pub struct Appel {
        /// L'`Execute` entier, empreinte FNV-1a 64 (offsets, appels et site de cette version).
        pub execute: usize,
        pub taille: usize,
        pub empreinte: u64,
        /// `jmp rel32` vers `ETAPE`, précédé de `mov rdi, rax` et des `pop`…
        pub site: usize,
        /// … eux-mêmes précédés de `call rel32` vers `JEU`, à cet écart avant le site.
        pub jeu: usize,
    }

    /// Même ordre que `APPELS` : l'indice passé à l'accroche désigne le même message.
    pub const APPELS: [Appel; 3] = [
        Appel { execute: 0x824540, taille: 0x30, empreinte: 0x7283_246b_4563_2ae2, site: 0x82455e, jeu: 9 },
        Appel { execute: 0x824880, taille: 0x30, empreinte: 0x3ad8_bac5_ab89_8660, site: 0x8248a4, jeu: 9 },
        Appel { execute: 0x824d40, taille: 0xe0, empreinte: 0x1e49_b9a4_22a3_c945, site: 0x824e1b, jeu: 12 },
    ];
}

/// Le lecteur du drapeau de fin de tour et l'étape de mise à jour, selon l'architecture.
#[cfg(target_arch = "aarch64")]
const ICI: (usize, usize) = (TOUR_FINI, ETAPE);
#[cfg(target_arch = "x86_64")]
const ICI: (usize, usize) = (x86::TOUR_FINI, x86::ETAPE);

static BASE: AtomicUsize = AtomicUsize::new(0);
static SAUTEES: AtomicU32 = AtomicU32::new(0);
const MAX_LOGS: u32 = 20;

fn gc(addr: usize) -> usize {
    BASE.load(Ordering::Relaxed) + addr
}

unsafe fn mot(addr: usize) -> u32 {
    std::ptr::read_unaligned(addr as *const u32)
}

unsafe fn octets(addr: usize, n: usize) -> &'static [u8] {
    std::slice::from_raw_parts(addr as *const u8, n)
}

/// Remplace l'étape de mise à jour à la fin de l'`Execute` du message `quel` (indice dans `APPELS`).
unsafe extern "C" fn etape_apres_diplomatie(jeu: usize, quel: u32) {
    let tour_fini: unsafe extern "C" fn() -> u8 = std::mem::transmute(gc(ICI.0));
    if tour_fini() != 0 && pol::actifs() {
        let k = SAUTEES.fetch_add(1, Ordering::Relaxed) + 1;
        if k <= MAX_LOGS {
            let message = APPELS.get(quel as usize).map_or("?", |a| a.message);
            log(&format!("fin de tour : {message} exécuté après la fin du tour, le tour suivant attend le contrôle de synchro ({k})"));
        }
        return;
    }
    let etape: unsafe extern "C" fn(usize) = std::mem::transmute(gc(ICI.1));
    etape(jeu);
}

unsafe fn site_attendu(a: &Appel) -> bool {
    let at = gc(a.site);
    patch::cible_branche(at, mot(at)) == Some(gc(ETAPE))
        && mot(at) & 0x8000_0000 == 0
        && patch::cible_branche(at - a.jeu, mot(at - a.jeu)) == Some(gc(JEU))
        && mot(at - a.jeu) & 0x8000_0000 != 0
}

/// Relais `quel` : `mov w1, #quel`, puis saut absolu vers l'accroche.
fn relais(quel: u32) -> Vec<u8> {
    let mut v = patch::relais(&[0x5280_0001 | (quel << 5)], etape_apres_diplomatie as *const () as usize);
    v.resize(RELAIS, 0);
    v
}

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    BASE.store(base, Ordering::Relaxed);
    if octets(gc(TOUR_FINI), TOUR_FINI_OCTETS.len()) != TOUR_FINI_OCTETS
        || octets(gc(ETAPE), ETAPE_OCTETS.len()) != ETAPE_OCTETS
        || !APPELS.iter().all(|a| site_attendu(a))
    {
        return Err("version inattendue de GameCore, aucun appel redirigé".into());
    }
    let code: Vec<u8> = (0..APPELS.len() as u32).flat_map(relais).collect();
    let page = patch::executable_near(&code, gc(APPELS[0].site), 120 << 20)?;
    let mut sauts = Vec::with_capacity(APPELS.len());
    for (i, a) in APPELS.iter().enumerate() {
        sauts.push(patch::branche(gc(a.site), page + i * RELAIS).ok_or("relais hors de portée, aucun appel redirigé")?);
    }
    let mut methode = "";
    for (a, saut) in APPELS.iter().zip(sauts) {
        methode = patch::write_code(gc(a.site), &saut)?;
    }
    Ok(format!("mise à jour après DiplomacyAction/Deal/Session suspendue pendant le passage de tour (3 appels redirigés, {methode})"))
}

/// `jmp [rip+0]` vers `cible` : le saut de `patch::jump` en x86_64, encodable sur toute architecture.
#[cfg_attr(target_arch = "aarch64", allow(dead_code))]
fn saut_x86(cible: usize) -> Vec<u8> {
    let mut v = vec![0xff, 0x25, 0, 0, 0, 0];
    v.extend_from_slice(&(cible as u64).to_le_bytes());
    v
}

/// Relais x86_64 `quel` : `mov esi, quel` (le jeu reste dans `rdi`), puis saut absolu vers `cible`.
#[cfg_attr(target_arch = "aarch64", allow(dead_code))]
fn relais_x86(quel: u32, cible: usize) -> Vec<u8> {
    let mut v = vec![0xbe];
    v.extend_from_slice(&quel.to_le_bytes());
    v.extend(saut_x86(cible));
    v.resize(RELAIS, 0xcc);
    v
}

#[cfg_attr(target_arch = "aarch64", allow(dead_code))]
fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

/// Le site x86_64 : `jmp rel32` vers l'étape, `call rel32` vers `JEU` avant, `Execute` entier attendu.
#[cfg_attr(target_arch = "aarch64", allow(dead_code))]
unsafe fn site_attendu_x86(a: &x86::Appel) -> bool {
    let at = gc(a.site);
    fnv(octets(gc(a.execute), a.taille)) == a.empreinte
        && octets(at, 1)[0] == 0xe9
        && patch::cible_rel32_x86(at, octets(at, 5)) == Some(gc(x86::ETAPE))
        && octets(at - a.jeu, 1)[0] == 0xe8
        && patch::cible_rel32_x86(at - a.jeu, octets(at - a.jeu, 5)) == Some(gc(x86::JEU))
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    BASE.store(base, Ordering::Relaxed);
    if octets(gc(x86::TOUR_FINI), x86::TOUR_FINI_OCTETS.len()) != x86::TOUR_FINI_OCTETS
        || fnv(octets(gc(x86::ETAPE), x86::ETAPE_TAILLE)) != x86::ETAPE_EMPREINTE
        || !x86::APPELS.iter().all(|a| site_attendu_x86(a))
    {
        return Err("version inattendue de GameCore, aucun appel redirigé".into());
    }
    let accroche = etape_apres_diplomatie as *const () as usize;
    let code: Vec<u8> = (0..x86::APPELS.len() as u32).flat_map(|quel| relais_x86(quel, accroche)).collect();
    let page = patch::executable_near(&code, gc(x86::APPELS[0].site), 1 << 30)?;
    let mut sauts = Vec::with_capacity(x86::APPELS.len());
    for (i, a) in x86::APPELS.iter().enumerate() {
        sauts.push(patch::rel32_x86(0xe9, gc(a.site), page + i * RELAIS).ok_or("relais hors de portée, aucun appel redirigé")?);
    }
    let mut methode = "";
    for (a, saut) in x86::APPELS.iter().zip(sauts) {
        methode = patch::write_code(gc(a.site), &saut)?;
    }
    Ok(format!("mise à jour après DiplomacyAction/Deal/Session suspendue pendant le passage de tour (3 appels redirigés, {methode})"))
}

/// Tranche x86_64 d'un binaire universel (macho.rs n'a que `tranche_arm64`), pour les tests des
/// correctifs qui lisent le jeu installé.
#[cfg(test)]
pub(super) fn tranche_x86_64(b: &[u8]) -> Option<&[u8]> {
    const CPU_X86_64: u32 = 0x0100_0007;
    let be = |o: usize| b.get(o..o + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap()));
    if be(0)? != 0xcafe_babe {
        return None;
    }
    (0..be(4)? as usize).find_map(|i| {
        let e = 8 + 20 * i;
        (be(e)? == CPU_X86_64).then(|| b.get(be(e + 8)? as usize..(be(e + 8)? + be(e + 12)?) as usize)).flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relais_arm64() {
        let r = relais(2);
        assert_eq!(&r[..12], &[0x41, 0x00, 0x80, 0x52, 0x50, 0x00, 0x00, 0x58, 0x00, 0x02, 0x1f, 0xd6]);
        assert_eq!(r.len(), RELAIS);
    }

    #[test]
    fn relais_x86_64() {
        let r = relais_x86(2, 0x1122_3344_5566_7788);
        // mov esi, 2 ; jmp [rip+0] ; .quad
        assert_eq!(&r[..19], &[0xbe, 2, 0, 0, 0, 0xff, 0x25, 0, 0, 0, 0, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]);
        assert_eq!(r.len(), RELAIS);
        assert!(r[19..].iter().all(|&b| b == 0xcc));
    }

    /// Les octets vérifiés au chargement correspondent au GameCore arm64 installé, s'il est là.
    #[test]
    fn sites_du_jeu() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let Some(t) = super::super::macho::tranche_arm64(&b) else { panic!("pas de tranche arm64") };
        // Image : __TEXT en 0, au début de la tranche.
        let mot = |a: usize| u32::from_le_bytes(t[a..a + 4].try_into().unwrap());
        assert_eq!(&t[TOUR_FINI..TOUR_FINI + 12], &TOUR_FINI_OCTETS);
        assert_eq!(&t[ETAPE..ETAPE + 8], &ETAPE_OCTETS);
        for a in &APPELS {
            assert_eq!(patch::cible_branche(a.site, mot(a.site)), Some(ETAPE), "{}", a.message);
            assert_eq!(patch::cible_branche(a.site - a.jeu, mot(a.site - a.jeu)), Some(JEU), "{}", a.message);
        }
        // Tranche x86_64 : image en 0 également.
        let t = tranche_x86_64(&b).expect("tranche x86_64");
        assert_eq!(&t[x86::TOUR_FINI..x86::TOUR_FINI + 13], &x86::TOUR_FINI_OCTETS);
        assert_eq!(fnv(&t[x86::ETAPE..x86::ETAPE + x86::ETAPE_TAILLE]), x86::ETAPE_EMPREINTE);
        for (a, m) in x86::APPELS.iter().zip(&APPELS) {
            assert_eq!(fnv(&t[a.execute..a.execute + a.taille]), a.empreinte, "{} x86_64", m.message);
            assert!((a.execute..a.execute + a.taille).contains(&a.site), "{} x86_64", m.message);
            assert_eq!(t[a.site], 0xe9, "{} x86_64", m.message);
            assert_eq!(patch::cible_rel32_x86(a.site, &t[a.site..]), Some(x86::ETAPE), "{} x86_64", m.message);
            assert_eq!(t[a.site - a.jeu], 0xe8, "{} x86_64", m.message);
            assert_eq!(patch::cible_rel32_x86(a.site - a.jeu, &t[a.site - a.jeu..]), Some(x86::JEU), "{} x86_64", m.message);
        }
    }
}
