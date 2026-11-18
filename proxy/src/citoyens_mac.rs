//! Choix des cases travaillées, sous macOS : le pendant de citoyens.rs (voir son en-tête). Correctif
//! POL. La règle est commune avec Windows (citoyens_regle.rs).
//!
//! Retrouvé dans GameCore_XP2 arm64 (Civ6 Aspyr 1.4.6), adresses relatives à l'image, en partant de
//! `SetFavoredYield` (Lua, `0x759d18` → `0x2c08ec` → placement `0x2bf290`) : la notation des
//! emplacements (`180110ba0` sous Windows) est `0x2bf5e8`, et le choix (`18010f050`) y est en ligne,
//! de `0x2bf964` à `0x2bff6c`. À l'entrée du choix : candidats de `x26` à `x26 + 0x18 × w23` (même
//! disposition que sous Windows), citoyens à placer `K` en `[x21 + 0x68]` (`x21`, l'objet des
//! citoyens de la ville, aussi en `[sp + 0x38]`), déficit de nourriture `T` en `[sp + 0x14]`. Le
//! choix recopie les emplacements retenus au début de `x26`, puis `0x2bff70` pose un citoyen sur
//! chacun, de `x26` à `x19`, avec `w23` relu en `[sp + 0x18]`.
//!
//! Le détour remplace la première instruction du choix (`ldr w27, [x21, #0x68]`, `0x2bf968`) par un
//! saut vers `relais`, qui appelle `choix(candidats, n, K, T)` en gardant les registres volatils :
//! un choix rendu, il saute en `0x2bff70` avec `x19` à sa fin ; sinon, il rejoue l'instruction et
//! reprend le choix du jeu en `0x2bf96c`.

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]

use super::citoyens_regle::{choisir, choix_du_jeu, Candidat};
use super::journal::log;
use super::{patch, pol};
use std::sync::atomic::{AtomicU32, Ordering};

/// La notation des emplacements, choix compris, en entier : le détour se fie à ses registres et à
/// sa pile.
const NOTATION: usize = 0x2bf5e8;
const TAILLE: usize = 0xb54;
const EMPREINTE: u64 = 0xc8c2_fa43_43c5_6a26;
const SITE: usize = 0x2bf968;
/// `ldr w27, [x21, #0x68]`, l'instruction remplacée.
const INSTRUCTION: u32 = 0xb9406abb;
const REPRISE: usize = 0x2bf96c;
const POSE_DES_CITOYENS: usize = 0x2bff70;

static DIFFS: AtomicU32 = AtomicU32::new(0);
const MAX_DIFF_LOGS: u32 = 30;

/// Adresses absolues lues par `relais`.
static mut ADRESSE_REPRISE: usize = 0;
static mut ADRESSE_POSE: usize = 0;

fn fnv(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

fn bilan(cands: &[Candidat], sel: &[usize], seuil: i32) -> String {
    let (f, fa, o) = sel.iter().fold((0i64, 0i64, 0i64), |(f, fa, o), &i| {
        (f + cands[i].nourriture as i64, fa + cands[i].favori as i64, o + cands[i].autre as i64)
    });
    let cases: Vec<String> = sel.iter().map(|&i| format!("{}/{}/{}", cands[i].nourriture, cands[i].favori, cands[i].autre)).collect();
    format!("nourriture {f}/{seuil}, favori {fa}, autre {o} : {}", cases.join(" "))
}

/// Le choix corrigé écrit au début des candidats, et sa fin ; nul pour laisser celui du jeu.
unsafe extern "C" fn choix(debut: *mut Candidat, n: u32, k: u32, seuil: i32) -> *mut Candidat {
    if k == 0 || n == 0 || debut.is_null() || !pol::actifs() {
        return std::ptr::null_mut();
    }
    let cands: Vec<Candidat> = std::slice::from_raw_parts(debut, n as usize).to_vec();
    let triplets: Vec<(i32, i32, i32)> = cands.iter().map(|c| (c.nourriture, c.favori, c.autre)).collect();
    let Some(sel) = choisir(&triplets, k as usize, seuil) else {
        return std::ptr::null_mut();
    };
    if DIFFS.load(Ordering::Relaxed) < MAX_DIFF_LOGS {
        let jeu = choix_du_jeu(&triplets, k as usize, seuil);
        let score = |s: &[usize]| {
            let f: i64 = s.iter().map(|&i| triplets[i].0 as i64).sum();
            (f >= seuil as i64, s.iter().map(|&i| triplets[i].1 as i64).sum::<i64>(), s.iter().map(|&i| triplets[i].2 as i64).sum::<i64>())
        };
        if score(&jeu) != score(&sel) {
            let d = DIFFS.fetch_add(1, Ordering::Relaxed) + 1;
            log(&format!(
                "citoyens : choix corrigé ({d}), {k} citoyens. Jeu : {}. BGD : {}",
                bilan(&cands, &jeu, seuil),
                bilan(&cands, &sel, seuil)
            ));
        }
    }
    for (j, &i) in sel.iter().enumerate() {
        *debut.add(j) = cands[i];
    }
    debut.add(sel.len())
}

/// Atteint par un `b` depuis `SITE`, dans la pile de `0x2bf5e8` : `x0`-`x17` et `x29`/`x30` gardés
/// autour de l'appel (`[sp + 0x14]` et `[sp + 0x18]` du jeu sont à `+0xb0` une fois la place prise).
#[cfg(target_arch = "aarch64")]
#[unsafe(naked)]
unsafe extern "C" fn relais() {
    core::arch::naked_asm!(
        "sub sp, sp, #0xb0",
        "stp x0, x1, [sp, #0x00]",
        "stp x2, x3, [sp, #0x10]",
        "stp x4, x5, [sp, #0x20]",
        "stp x6, x7, [sp, #0x30]",
        "stp x8, x9, [sp, #0x40]",
        "stp x10, x11, [sp, #0x50]",
        "stp x12, x13, [sp, #0x60]",
        "stp x14, x15, [sp, #0x70]",
        "stp x16, x17, [sp, #0x80]",
        "stp x29, x30, [sp, #0x90]",
        "mov x0, x26",
        "mov w1, w23",
        "ldr w2, [x21, #0x68]",
        "ldr w3, [sp, #0xc4]",
        "bl {choix}",
        "str x0, [sp, #0xa0]",
        "ldp x0, x1, [sp, #0x00]",
        "ldp x2, x3, [sp, #0x10]",
        "ldp x4, x5, [sp, #0x20]",
        "ldp x6, x7, [sp, #0x30]",
        "ldp x8, x9, [sp, #0x40]",
        "ldp x10, x11, [sp, #0x50]",
        "ldp x12, x13, [sp, #0x60]",
        "ldp x14, x15, [sp, #0x70]",
        "ldp x16, x17, [sp, #0x80]",
        "ldp x29, x30, [sp, #0x90]",
        "ldr x16, [sp, #0xa0]",
        "add sp, sp, #0xb0",
        "cbz x16, 2f",
        "mov x19, x16",
        "ldr w23, [sp, #0x18]",
        "adrp x16, {pose}@PAGE",
        "ldr x16, [x16, {pose}@PAGEOFF]",
        "br x16",
        "2:",
        "ldr w27, [x21, #0x68]",
        "adrp x16, {reprise}@PAGE",
        "ldr x16, [x16, {reprise}@PAGEOFF]",
        "br x16",
        choix = sym choix,
        pose = sym ADRESSE_POSE,
        reprise = sym ADRESSE_REPRISE,
    );
}

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    let code = std::slice::from_raw_parts((base + NOTATION) as *const u8, TAILLE);
    if fnv(code) != EMPREINTE || std::ptr::read_unaligned((base + SITE) as *const u32) != INSTRUCTION {
        return Err(format!("version inattendue (notation des emplacements en {NOTATION:#x}), aucun détour"));
    }
    ADRESSE_REPRISE = base + REPRISE;
    ADRESSE_POSE = base + POSE_DES_CITOYENS;
    let site = base + SITE;
    let page = patch::executable_near(&patch::jump(relais as *const () as usize), site, 120 << 20)?;
    let b = patch::branche(site, page).ok_or("relais hors de portée, aucun détour")?;
    let methode = patch::write_code(site, &b)?;
    Ok(format!("choix des cases travaillées corrigé (choix en ligne de {NOTATION:#x} détourné en {SITE:#x}, {methode})"))
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(_base: usize) -> Result<String, String> {
    Err("pas encore relevé en x86_64".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notation_du_jeu() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::super::macho::tranche_arm64(&b).expect("tranche arm64");
        let e = fnv(&t[NOTATION..NOTATION + TAILLE]);
        assert_eq!(e, EMPREINTE, "empreinte {e:#x}");
        let mot = |a: usize| u32::from_le_bytes(t[a..a + 4].try_into().unwrap());
        assert_eq!(mot(SITE), INSTRUCTION, "ldr w27, [x21, #0x68]");
        // cbz w23 juste avant : aucun candidat, le choix est sauté.
        assert_eq!(mot(SITE - 4) & 0xff00_001f, 0x3400_0017, "cbz w23");
        // 0x2bff70 : ldr x21, [sp, #0x38] ; 0x2bf9a0 : ldr w23, [sp, #0x18].
        assert_eq!(mot(POSE_DES_CITOYENS), 0xf9401ff5);
        assert_eq!(mot(0x2bf9a0), 0xb9401bf7);
    }
}
