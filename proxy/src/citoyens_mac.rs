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
//!
//! En x86_64 (Mac Intel, Rosetta), le choix est aussi en ligne (pas de `18010f050` à part) :
//! placement `0x2a6130` (appelé par `SetFavoredYield`, `0x761d00` → `0x2a7870`), notation
//! `0x2a6440`, choix de `0x2a682e` à `0x2a6f95`. À l'entrée du choix (`test ebx, ebx ; je`,
//! aucun candidat) : candidats de `r12` à `r12 + 0x18 × ebx`, objet des citoyens en
//! `[rbp - 0x80]`, `K` en `[obj + 0x68]`, déficit `T` en `[rbp - 0xbc]`. Le choix finit en
//! `0x2a6f95` (`add rbx, r12`), puis `0x2a6f98` (`cmp rbx, r12 ; je`) pose un citoyen sur chaque
//! emplacement de `r12` à `rbx`. Le détour remplace les deux instructions qui suivent le test
//! (`mov [rbp - 0x78], r12 ; mov rax, [rbp - 0x80]`, 8 octets en `0x2a6836`) par un `jmp rel32`
//! vers une page proche, qui saute vers `relais` : il rejoue la première, garde les registres
//! volatils (et `xmm0`-`xmm15`) autour de `choix`, puis saute en `0x2a6f98` avec `rbx` à la fin
//! du choix rendu, ou rejoue la seconde et reprend en `0x2a683e`. Les drapeaux ne sont pas
//! gardés : les deux suites les recalculent (`test r12d` en `0x2a6842`, `cmp` en `0x2a6f98`).

#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]
// Les relevés x86_64 ne servent, en arm64, qu'aux tests qui lisent le jeu installé.
#![cfg_attr(target_arch = "aarch64", allow(dead_code))]

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

/// x86_64 : la notation en entier, le site (deux instructions de 4 octets) et les reprises.
const X86_NOTATION: usize = 0x2a6440;
const X86_TAILLE: usize = 0xd50;
const X86_EMPREINTE: u64 = 0x288f_2100_2bcd_eaab;
const X86_SITE: usize = 0x2a6836;
/// `mov [rbp - 0x78], r12 ; mov rax, [rbp - 0x80]`, les instructions remplacées.
const X86_INSTRUCTIONS: [u8; 8] = [0x4c, 0x89, 0x65, 0x88, 0x48, 0x8b, 0x45, 0x80];
const X86_REPRISE: usize = 0x2a683e;
const X86_POSE_DES_CITOYENS: usize = 0x2a6f98;

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

/// Atteint par un `jmp` depuis `X86_SITE` (via une page proche), dans le cadre de `0x2a6440` :
/// `rsp` y est aligné sur 16 (cinq `push` et `sub rsp, 0xd8` après `push rbp`), et le reste
/// pendant l'appel (`0x150` octets pris). `rbx` (le nombre de candidats, puis la fin du choix),
/// `r12` (les candidats) et `rbp` sont gardés par `choix` (registres non volatils).
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
unsafe extern "C" fn relais() {
    core::arch::naked_asm!(
        "mov qword ptr [rbp - 0x78], r12",
        "sub rsp, 0x150",
        "movdqu xmmword ptr [rsp + 0x00], xmm0",
        "movdqu xmmword ptr [rsp + 0x10], xmm1",
        "movdqu xmmword ptr [rsp + 0x20], xmm2",
        "movdqu xmmword ptr [rsp + 0x30], xmm3",
        "movdqu xmmword ptr [rsp + 0x40], xmm4",
        "movdqu xmmword ptr [rsp + 0x50], xmm5",
        "movdqu xmmword ptr [rsp + 0x60], xmm6",
        "movdqu xmmword ptr [rsp + 0x70], xmm7",
        "movdqu xmmword ptr [rsp + 0x80], xmm8",
        "movdqu xmmword ptr [rsp + 0x90], xmm9",
        "movdqu xmmword ptr [rsp + 0xa0], xmm10",
        "movdqu xmmword ptr [rsp + 0xb0], xmm11",
        "movdqu xmmword ptr [rsp + 0xc0], xmm12",
        "movdqu xmmword ptr [rsp + 0xd0], xmm13",
        "movdqu xmmword ptr [rsp + 0xe0], xmm14",
        "movdqu xmmword ptr [rsp + 0xf0], xmm15",
        "mov qword ptr [rsp + 0x100], rax",
        "mov qword ptr [rsp + 0x108], rcx",
        "mov qword ptr [rsp + 0x110], rdx",
        "mov qword ptr [rsp + 0x118], rsi",
        "mov qword ptr [rsp + 0x120], rdi",
        "mov qword ptr [rsp + 0x128], r8",
        "mov qword ptr [rsp + 0x130], r9",
        "mov qword ptr [rsp + 0x138], r10",
        "mov qword ptr [rsp + 0x140], r11",
        "mov rdi, r12",
        "mov esi, ebx",
        "mov rax, qword ptr [rbp - 0x80]",
        "mov edx, dword ptr [rax + 0x68]",
        "mov ecx, dword ptr [rbp - 0xbc]",
        "call {choix}",
        "mov qword ptr [rsp + 0x148], rax",
        "movdqu xmm0, xmmword ptr [rsp + 0x00]",
        "movdqu xmm1, xmmword ptr [rsp + 0x10]",
        "movdqu xmm2, xmmword ptr [rsp + 0x20]",
        "movdqu xmm3, xmmword ptr [rsp + 0x30]",
        "movdqu xmm4, xmmword ptr [rsp + 0x40]",
        "movdqu xmm5, xmmword ptr [rsp + 0x50]",
        "movdqu xmm6, xmmword ptr [rsp + 0x60]",
        "movdqu xmm7, xmmword ptr [rsp + 0x70]",
        "movdqu xmm8, xmmword ptr [rsp + 0x80]",
        "movdqu xmm9, xmmword ptr [rsp + 0x90]",
        "movdqu xmm10, xmmword ptr [rsp + 0xa0]",
        "movdqu xmm11, xmmword ptr [rsp + 0xb0]",
        "movdqu xmm12, xmmword ptr [rsp + 0xc0]",
        "movdqu xmm13, xmmword ptr [rsp + 0xd0]",
        "movdqu xmm14, xmmword ptr [rsp + 0xe0]",
        "movdqu xmm15, xmmword ptr [rsp + 0xf0]",
        "mov rax, qword ptr [rsp + 0x100]",
        "mov rcx, qword ptr [rsp + 0x108]",
        "mov rdx, qword ptr [rsp + 0x110]",
        "mov rsi, qword ptr [rsp + 0x118]",
        "mov rdi, qword ptr [rsp + 0x120]",
        "mov r8, qword ptr [rsp + 0x128]",
        "mov r9, qword ptr [rsp + 0x130]",
        "mov r10, qword ptr [rsp + 0x138]",
        "mov r11, qword ptr [rsp + 0x140]",
        "cmp qword ptr [rsp + 0x148], 0",
        "je 2f",
        "mov rbx, qword ptr [rsp + 0x148]",
        "add rsp, 0x150",
        "jmp qword ptr [rip + {pose}]",
        "2:",
        "add rsp, 0x150",
        "mov rax, qword ptr [rbp - 0x80]",
        "jmp qword ptr [rip + {reprise}]",
        choix = sym choix,
        pose = sym ADRESSE_POSE,
        reprise = sym ADRESSE_REPRISE,
    );
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(base: usize) -> Result<String, String> {
    let code = std::slice::from_raw_parts((base + X86_NOTATION) as *const u8, X86_TAILLE);
    let site = std::slice::from_raw_parts((base + X86_SITE) as *const u8, X86_INSTRUCTIONS.len());
    if fnv(code) != X86_EMPREINTE || site != X86_INSTRUCTIONS {
        return Err(format!("version inattendue (notation des emplacements en {X86_NOTATION:#x}), aucun détour"));
    }
    ADRESSE_REPRISE = base + X86_REPRISE;
    ADRESSE_POSE = base + X86_POSE_DES_CITOYENS;
    let site = base + X86_SITE;
    // Le `jmp rel32` porte à ±2 Go : la page qui saute vers `relais` est prise à moins de 1 Go.
    let page = patch::executable_near(&patch::jump(relais as *const () as usize), site, 1 << 30)?;
    let saut = patch::rel32_x86(0xe9, site, page).ok_or("relais hors de portée, aucun détour")?;
    let mut octets = saut.to_vec();
    octets.resize(X86_INSTRUCTIONS.len(), 0x90);
    let methode = patch::write_code(site, &octets)?;
    Ok(format!("choix des cases travaillées corrigé (choix en ligne de {X86_NOTATION:#x} détourné en {X86_SITE:#x}, {methode})"))
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

    #[test]
    fn notation_du_jeu_x86_64() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/GameCore_XP2.dll");
        let Ok(b) = std::fs::read(&p) else { return };
        let t = super::super::spectateurs::tranche_x86_64(&b).expect("tranche x86_64");
        let e = fnv(&t[X86_NOTATION..X86_NOTATION + X86_TAILLE]);
        assert_eq!(e, X86_EMPREINTE, "empreinte {e:#x}");
        assert_eq!(t[X86_SITE..X86_SITE + 8], X86_INSTRUCTIONS);
        let cible = |at: usize, n: usize| (at as i64 + n as i64 + i32::from_le_bytes(t[at + n - 4..at + n].try_into().unwrap()) as i64) as usize;
        // test ebx, ebx ; je (fin) juste avant : aucun candidat, le choix est sauté.
        assert_eq!(t[X86_SITE - 8..X86_SITE - 6], [0x85, 0xdb]);
        assert_eq!(t[X86_SITE - 6..X86_SITE - 4], [0x0f, 0x84]);
        assert_eq!(cible(X86_SITE - 6, 6), 0x2a6fcd);
        // Seule entrée du choix : jae depuis la boucle des candidats.
        assert_eq!(t[0x2a650c..0x2a650e], [0x0f, 0x83]);
        assert_eq!(cible(0x2a650c, 6), X86_SITE - 8);
        // Reprise : mov r12d, [rax + 0x68] (K), puis test r12d, r12d.
        assert_eq!(t[X86_REPRISE..X86_REPRISE + 7], [0x44, 0x8b, 0x60, 0x68, 0x45, 0x85, 0xe4]);
        // T : mov [rbp - 0xbc], eax au début de la notation.
        assert_eq!(t[0x2a6491..0x2a6497], [0x89, 0x85, 0x44, 0xff, 0xff, 0xff]);
        // Pose : add rbx, r12 (fin du choix) ; cmp rbx, r12 ; je (fin) ; puis la boucle qui relit
        // [rbp - 0x80] et appelle 0x2a6070 sur chaque emplacement.
        assert_eq!(t[X86_POSE_DES_CITOYENS - 3..X86_POSE_DES_CITOYENS + 4], [0x4c, 0x01, 0xe3, 0x4c, 0x39, 0xe3, 0x74]);
        assert_eq!(X86_POSE_DES_CITOYENS + 5 + t[X86_POSE_DES_CITOYENS + 4] as usize, 0x2a6fcd);
        assert_eq!(t[0x2a6fb3..0x2a6fb7], [0x48, 0x8b, 0x7d, 0x80]);
        assert_eq!(patch::cible_rel32_x86(0x2a6fbf, &t[0x2a6fbf..0x2a6fc4]), Some(0x2a6070));
        // Pile alignée sur 16 au site : push rbp, cinq push, sub rsp, 0xd8.
        assert_eq!(t[X86_NOTATION..X86_NOTATION + 20], [0x55, 0x48, 0x89, 0xe5, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec, 0xd8, 0, 0, 0]);
    }

    /// Imite le site de `0x2a6440` : `rbp` sur un faux cadre, `r12` les candidats, `rbx` leur
    /// nombre, `rsp` aligné sur 16, des valeurs connues dans les registres volatils et `xmm7`.
    /// Les reprises de `relais` pointent sur ses étiquettes, qui rangent les registres dans le cadre.
    #[cfg(target_arch = "x86_64")]
    #[unsafe(naked)]
    unsafe extern "C" fn harnais(cadre: usize, candidats: usize, n: u64) {
        core::arch::naked_asm!(
            "push rbp",
            "push rbx",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            "sub rsp, 8",
            "mov rbp, rdi",
            "mov r12, rsi",
            "mov rbx, rdx",
            "lea rax, [rip + 3f]",
            "mov qword ptr [rip + {reprise}], rax",
            "lea rax, [rip + 4f]",
            "mov qword ptr [rip + {pose}], rax",
            "mov qword ptr [rbp - 0x170], rsp",
            "mov rax, 0x1001",
            "mov rcx, 0x1002",
            "mov rdx, 0x1003",
            "mov rsi, 0x1004",
            "mov rdi, 0x1005",
            "mov r8, 0x1006",
            "mov r9, 0x1007",
            "mov r10, 0x1008",
            "mov r11, 0x1009",
            "movq xmm7, r11",
            "jmp {relais}",
            "4:",
            "mov qword ptr [rbp - 0x150], 1",
            "3:",
            "mov qword ptr [rbp - 0x100], rax",
            "mov qword ptr [rbp - 0x108], rcx",
            "mov qword ptr [rbp - 0x110], rdx",
            "mov qword ptr [rbp - 0x118], rsi",
            "mov qword ptr [rbp - 0x120], rdi",
            "mov qword ptr [rbp - 0x128], r8",
            "mov qword ptr [rbp - 0x130], r9",
            "mov qword ptr [rbp - 0x138], r10",
            "mov qword ptr [rbp - 0x140], r11",
            "movq rax, xmm7",
            "mov qword ptr [rbp - 0x148], rax",
            "mov qword ptr [rbp - 0x158], rbx",
            "mov qword ptr [rbp - 0x160], r12",
            "mov qword ptr [rbp - 0x168], rsp",
            "add rsp, 8",
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbx",
            "pop rbp",
            "ret",
            relais = sym super::relais,
            pose = sym super::ADRESSE_POSE,
            reprise = sym super::ADRESSE_REPRISE,
        );
    }

    /// Le relais x86_64 exécuté (Rosetta sur un Mac arm64) : `K` nul, `choix` laisse le jeu, sans
    /// consulter l'état POL. Rejoue les deux instructions remplacées et rend tous les registres.
    #[cfg(target_arch = "x86_64")]
    #[test]
    fn relais_x86_64_rend_la_main_au_jeu() {
        let mut cadre = vec![0u64; 0x40];
        let mut citoyens = vec![0u32; 0x20];
        citoyens[0x68 / 4] = 0;
        let candidats = vec![Candidat { plot: 0, nourriture: 2, favori: 1, autre: 0, reste: 0 }; 3];
        let rbp = cadre.as_mut_ptr() as usize + 0x180;
        let lire = |d: usize| unsafe { std::ptr::read((rbp - d) as *const u64) };
        unsafe {
            std::ptr::write((rbp - 0x80) as *mut u64, citoyens.as_mut_ptr() as u64);
            std::ptr::write((rbp - 0xbc) as *mut i32, 4);
            harnais(rbp, candidats.as_ptr() as usize, 3);
        }
        assert_eq!(lire(0x150), 0, "pas de choix rendu : reprise du jeu");
        assert_eq!(lire(0x78), candidats.as_ptr() as u64, "mov [rbp - 0x78], r12 rejoué");
        assert_eq!(lire(0x100), citoyens.as_ptr() as u64, "mov rax, [rbp - 0x80] rejoué");
        for (k, d) in (0x108..=0x140).step_by(8).enumerate() {
            assert_eq!(lire(d), 0x1002 + k as u64, "registre volatil {k} rendu");
        }
        assert_eq!(lire(0x148), 0x1009, "xmm7 rendu");
        assert_eq!(lire(0x158), 3, "rbx rendu");
        assert_eq!(lire(0x160), candidats.as_ptr() as u64, "r12 rendu");
        assert_eq!(lire(0x168), lire(0x170), "pile rendue");
        assert_eq!(lire(0x170) % 16, 0, "pile alignée au site");
    }
}
