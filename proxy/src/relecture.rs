//! Une variable relue sur place redevient ce que l'hôte a écrit (`relecture_fidele=on`, par défaut ;
//! adresses du GameCore_XP2 1.0.12.72, traduites pour les autres builds par `patch`).
//!
//! Le client relit une variable d'AutoArchive dans l'objet qu'il a déjà : lecture complète
//! (vtable `+0x08`) quand l'hôte renvoie l'archive en données de création (`reparation_synchro`,
//! « already exists on client »), et application d'un delta (vtable `+0x10`, qui saute à la même
//! lecture complète). Ces lectures sont écrites pour un objet neuf. Sur un objet en service, quatre
//! d'entre elles ne rendent pas ce que l'hôte a écrit : le delta qui devait réparer la variable la
//! laisse différente, ou la casse, et l'IA du client agit ensuite autrement que celle de l'hôte.
//! Mesuré en renvoyant chaque archive en données de création, sans divergence, puis en comparant
//! octet par octet l'écriture de l'hôte et celle du client après relecture : les 68 autres types
//! d'archive se relisent exactement. Ce module corrige trois de ces lectures ; la quatrième, celle
//! des arbres de comportement, a déjà sa réparation dans le jeu, que seule la relecture sur place
//! ne lance pas (`relecture_sur_place`, proxy de démarrage).
//!
//! - **Opérations d'une unité** (`Unit.m_kOperations`, lecture `180041c30`). La lecture ajoute
//!   chaque opération lue à celles de l'unité sans les vider : après une relecture, l'unité suit
//!   ses opérations en double. Les opérations en place sont désormais libérées (`180372fc0`, ce que
//!   fait la comparaison de la variable pour sa copie temporaire) avant la lecture.
//! - **Gestionnaires de boosts** (`AI::Research` et `AI::CultureAi`, `m_boostManager`, lecture
//!   `1803d51b0`). Même ajout, sur une liste de paires d'entiers, vidée désormais avant la lecture.
//!   La mise en place qui suit la lecture (`1803d6080`) reconstruit sa table de boosts depuis zéro
//!   (`1803d7e20`) : la refaire ne double rien.
//! - **Analyseurs de résolutions** (`AI::CultureAi.m_resolutionAnalyzers`, vecteur lu par
//!   `180405940`, chaque analyseur par `180411630`). La lecture d'un analyseur lit ses quatre derniers
//!   champs (`+0x10` à `+0x1c`, son score compris) puis les remet à 0, -1, 0 et 0 (`180411781`) :
//!   l'analyseur relu perd ses valeurs, que l'hôte garde. La remise à zéro est sautée pendant la
//!   relecture d'un vecteur en service.
//! - **Arbres de comportement** (`BehaviorTree.m_definitions`, lecture `180546fe0`), pas corrigés
//!   ici. Chaque paramètre d'un nœud est {table partagée, numéro de la table, indice}. La lecture
//!   d'un nœud met tout le nœud à zéro (`180566f60`) puis relit numéro et indice, mais pas le
//!   pointeur de table, que pose l'édition de liens de l'arbre (`180543f50`, depuis les tables
//!   partagées de son propriétaire). Le constructeur de l'arbre (`18053e4d0`) inscrit sur
//!   `m_definitions` une DeltaDesyncCallback (`180543fd0`) qui refait cette édition de liens : le
//!   chemin de delta la lance après avoir appliqué la variable, la relecture sur place d'origine
//!   non, et l'arbre y restait sans liens (son écriture donne -1 pour chaque indice).
//!
//! Chaque correction ne s'applique qu'à un objet en service (liste non vide avant la lecture) : le
//! chargement d'une partie, qui lit dans des objets neufs, reste celui du jeu d'origine, et l'hôte ne
//! relit rien. Ce n'est donc pas un correctif POL : un client qui a ce proxy converge vers l'hôte,
//! que les autres joueurs l'aient ou non.
//!
//! Réglage : `relecture_fidele=on|off` (on par défaut). Les sites sont vérifiés avant d'en poser un
//! seul.

use crate::exe::rd;
use crate::patch::{self, Site};
use crate::{log, win};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

// ------------------------------------------------------------------ sites (GameCore_XP2 1.0.12.72)

/// Lecture des opérations d'une unité (flux, objet) : version, nombre, puis chaque opération,
/// ajoutée au vecteur `+0xd0` (nombre en `+0xe0`).
const OPS_READ: Site = Site {
    name: "unit operations read",
    address: 0x1_8004_1c30,
    // push rbp ; push rsi ; push rdi ; push r14 ; push r15 ; lea rbp,[rsp-0x37] ; sub rsp,0xd0
    expected: &[
        0x40, 0x55, 0x56, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0x6C, 0x24, 0xC9, 0x48, 0x81, 0xEC, 0xD0, 0x00, 0x00, 0x00,
    ],
    copy: (0, 20),
    resume: 20,
};
/// Lecture d'un gestionnaire de boosts (flux, gestionnaire) : joueur, `+0x38`, puis les paires
/// ajoutées au vecteur `+0x08` (nombre en `+0x18`), puis la mise en place `1803d6080`.
const BOOSTS_READ: Site = Site {
    name: "boost manager read",
    address: 0x1_803d_51b0,
    // push rbp ; push rbx ; push rsi ; push rdi ; push r12 ; push r14 ; push r15 ; mov rbp,rsp
    expected: &[0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8B, 0xEC],
    copy: (0, 14),
    resume: 14,
};
/// Lecture d'un vecteur d'analyseurs de résolutions (flux, vecteur : nombre en `+0x10`, remis à 0
/// d'entrée), chaque analyseur par `180411630`.
const ANALYZERS_READ: Site = Site {
    name: "resolution analyzers read",
    address: 0x1_8040_5940,
    // mov r11,rsp ; mov [r11+0x18],rbx ; mov [r11+0x20],rbp ; push rsi ; sub rsp,0x50
    expected: &[0x4C, 0x8B, 0xDC, 0x49, 0x89, 0x5B, 0x18, 0x49, 0x89, 0x6B, 0x20, 0x56, 0x48, 0x83, 0xEC, 0x50],
    copy: (0, 16),
    resume: 16,
};
/// Fin de la lecture d'un analyseur (`rsi` l'analyseur, `rax` nul) : remise à zéro des champs lus.
const ANALYZER_RESET: Site = Site {
    name: "resolution analyzer reset",
    address: 0x1_8041_1781,
    // mov [rsi+0x10],eax ; mov dword [rsi+0x14],-1 ; mov [rsi+0x18],rax
    expected: &[0x89, 0x46, 0x10, 0xC7, 0x46, 0x14, 0xFF, 0xFF, 0xFF, 0xFF, 0x48, 0x89, 0x46, 0x18],
    copy: (0, 14),
    resume: 14,
};
const SITES: [Site; 4] = [OPS_READ, BOOSTS_READ, ANALYZERS_READ, ANALYZER_RESET];

/// Libère les opérations [début, début + n) (objet, début, n) : la référence partagée de chacune.
/// Appelée, pas détournée : ses octets sont seulement vérifiés avec les sites.
const OPS_RELEASE: Site = Site {
    name: "unit operations release",
    address: 0x1_8037_2fc0,
    // sub rsp,0x28 ; test r8d,r8d ; je +0x63 ; mov [rsp+0x30],rbx
    expected: &[0x48, 0x83, 0xEC, 0x28, 0x45, 0x85, 0xC0, 0x74, 0x63, 0x48, 0x89, 0x5C, 0x24, 0x30],
    copy: (0, 0),
    resume: 0,
};
const OPS_DATA: usize = 0xd0;
const OPS_COUNT: usize = 0xe0;
const BOOSTS_COUNT: usize = 0x18;
const ANALYZERS_COUNT: usize = 0x10;

// ------------------------------------------------------------------ état

static ON: AtomicBool = AtomicBool::new(true);
static BASE: AtomicUsize = AtomicUsize::new(0);
static TRAMPS: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
/// Octet de décision du relais de la remise à zéro : non nul pendant la relecture d'un vecteur
/// d'analyseurs en service.
static KEEP: AtomicUsize = AtomicUsize::new(0);
/// Lignes de journal écrites (plafond `MAX_LINES`).
static LINES: AtomicU32 = AtomicU32::new(0);
const MAX_LINES: u32 = 200;

pub fn set_enabled(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

fn say(msg: String) {
    let n = LINES.fetch_add(1, Ordering::Relaxed);
    if n < MAX_LINES {
        log(&msg);
    } else if n == MAX_LINES {
        log("re-read: log limit reached, further corrections not logged");
    }
}

fn tramp(i: usize) -> usize {
    TRAMPS[i].load(Ordering::Relaxed)
}

fn global(address: u64) -> usize {
    BASE.load(Ordering::Relaxed) + patch::rva(address)
}

// ------------------------------------------------------------------ détours

type ReadFn = unsafe extern "system-unwind" fn(usize, usize) -> usize;
type ReleaseFn = unsafe extern "system-unwind" fn(usize, usize, u32);

unsafe extern "system-unwind" fn ops_read_hook(stream: usize, ops: usize) -> usize {
    let orig: ReadFn = std::mem::transmute(tramp(0));
    if ON.load(Ordering::Relaxed) && ops != 0 {
        let n: u32 = rd(ops + OPS_COUNT);
        let data: usize = rd(ops + OPS_DATA);
        if n != 0 && data != 0 {
            let release: ReleaseFn = std::mem::transmute(global(OPS_RELEASE.address));
            release(ops, data, n);
            *((ops + OPS_COUNT) as *mut u32) = 0;
            say(format!("re-read: {n} unit operation(s) released before reading them again"));
        }
    }
    orig(stream, ops)
}

unsafe extern "system-unwind" fn boosts_read_hook(stream: usize, mgr: usize) -> usize {
    let orig: ReadFn = std::mem::transmute(tramp(1));
    if ON.load(Ordering::Relaxed) && mgr != 0 {
        let n: u32 = rd(mgr + BOOSTS_COUNT);
        if n != 0 {
            *((mgr + BOOSTS_COUNT) as *mut u32) = 0;
            say(format!("re-read: boost list of {n} pair(s) emptied before reading it again"));
        }
    }
    orig(stream, mgr)
}

unsafe extern "system-unwind" fn analyzers_read_hook(stream: usize, vec: usize) -> usize {
    let orig: ReadFn = std::mem::transmute(tramp(2));
    let n: u32 = if vec != 0 { rd(vec + ANALYZERS_COUNT) } else { 0 };
    let keep = ON.load(Ordering::Relaxed) && n != 0;
    let flag = KEEP.load(Ordering::Relaxed) as *mut u8;
    *flag = keep as u8;
    let r = orig(stream, vec);
    *flag = 0;
    if keep {
        say(format!("re-read: {} resolution analyzer(s) kept the values they read", rd::<u32>(vec + ANALYZERS_COUNT)));
    }
    r
}

/// Relais de la remise à zéro, placé au site par un saut (registres de la lecture en cours) :
/// octet de décision non nul -> reprise après la remise à zéro ; sinon le trampoline, qui la fait.
///
/// ```text
/// +0x00 cmp byte [rip+X],0   ; X -> +0x30
/// +0x07 jne +6               ; -> +0x0f
/// +0x09 jmp [rip+X]          ; X -> +0x20 (trampoline)
/// +0x0f jmp [rip+X]          ; X -> +0x28 (reprise)
/// ```
unsafe fn build_relay(page: usize, tramp: usize, resume: usize) {
    let mut code = vec![0x80, 0x3D];
    code.extend_from_slice(&(0x30i32 - 0x07).to_le_bytes());
    code.push(0x00);
    code.extend_from_slice(&[0x75, 0x06]);
    code.extend_from_slice(&[0xFF, 0x25]);
    code.extend_from_slice(&(0x20i32 - 0x0f).to_le_bytes());
    code.extend_from_slice(&[0xFF, 0x25]);
    code.extend_from_slice(&(0x28i32 - 0x15).to_le_bytes());
    code.resize(0x20, 0xCC);
    code.extend_from_slice(&(tramp as u64).to_le_bytes());
    code.extend_from_slice(&(resume as u64).to_le_bytes());
    code.push(0);
    std::ptr::copy_nonoverlapping(code.as_ptr(), page as *mut u8, code.len());
}

// ------------------------------------------------------------------ pose

pub unsafe fn on_gamecore_loaded(base: usize) {
    if !ON.load(Ordering::Relaxed) {
        log("re-read: off (relecture_fidele=off), no hooks");
        return;
    }
    if let Err(e) = patch::check(base, &SITES).and_then(|_| patch::check(base, &[OPS_RELEASE])) {
        log(&format!("re-read: unexpected game version, no hooks: {e}"));
        return;
    }
    let relay = win::VirtualAlloc(std::ptr::null_mut(), 0x1000, win::MEM_COMMIT | win::MEM_RESERVE, win::PAGE_EXECUTE_READWRITE) as usize;
    if relay == 0 {
        log("re-read: VirtualAlloc failed, no hooks");
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    KEEP.store(relay + 0x30, Ordering::Relaxed);
    // La remise à zéro d'abord : son relais doit être prêt avant que la lecture du vecteur le règle.
    let reset_at = base + patch::rva(ANALYZER_RESET.address);
    let resume = reset_at + ANALYZER_RESET.resume;
    match patch::install_at(ANALYZER_RESET.name, reset_at, &patch::as_loaded(ANALYZER_RESET.address, ANALYZER_RESET.expected), resume, relay) {
        Ok(t) => build_relay(relay, t, resume),
        Err(e) => {
            log(&format!("re-read: failed to hook {}: {e}; no hooks", ANALYZER_RESET.name));
            BASE.store(0, Ordering::Relaxed);
            return;
        }
    }
    let hooks: [usize; 3] = [
        ops_read_hook as *const () as usize,
        boosts_read_hook as *const () as usize,
        analyzers_read_hook as *const () as usize,
    ];
    for (i, s) in SITES[..3].iter().enumerate() {
        match patch::install(base, s, hooks[i]) {
            Ok(t) => TRAMPS[i].store(t, Ordering::Relaxed),
            Err(e) => {
                log(&format!("re-read: failed to hook {}: {e}; hooks removed", s.name));
                for s in SITES[..i].iter().chain([&ANALYZER_RESET]) {
                    patch::restore(base, s);
                }
                BASE.store(0, Ordering::Relaxed);
                return;
            }
        }
    }
    log("re-read: 4 hooks installed");
}

pub unsafe fn on_proxy_unload() {
    let base = BASE.swap(0, Ordering::Relaxed);
    if base != 0 {
        for s in &SITES {
            patch::restore(base, s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque site a ses octets attendus dans le GameCore installé, quel que soit son build (test
    /// sauté sans lui).
    #[test]
    fn sites_match_installed_gamecore() {
        let path = r"C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\DLC\Expansion2\Binaries\Win64\GameCore_XP2_FinalRelease.dll";
        let Ok(file) = std::fs::read(path) else {
            eprintln!("GameCore missing, test skipped");
            return;
        };
        let rd32 = |o: usize| u32::from_le_bytes(file[o..o + 4].try_into().unwrap()) as usize;
        let rd16 = |o: usize| u16::from_le_bytes(file[o..o + 2].try_into().unwrap()) as usize;
        let nt = rd32(0x3c);
        let sections = nt + 24 + rd16(nt + 20);
        let offset = |rva: usize| {
            (0..rd16(nt + 6)).map(|i| sections + 40 * i).find_map(|h| {
                let (va, size, raw) = (rd32(h + 12), rd32(h + 16), rd32(h + 20));
                (rva >= va && rva < va + size).then(|| rva - va + raw)
            })
        };
        for s in SITES.iter().chain([&OPS_RELEASE]) {
            let o = offset(patch::rva(s.address)).unwrap_or_else(|| panic!("{}: not in the file", s.name));
            assert_eq!(patch::as_known(s.address, &file[o..o + s.expected.len()]), s.expected, "{}", s.name);
            assert!(s.expected.len() >= 14 && s.copy.1 <= s.expected.len() && s.resume >= s.copy.1, "{}", s.name);
        }
    }

    /// Le relais saute au trampoline quand l'octet de décision est nul, à la reprise sinon.
    #[test]
    fn relay_layout() {
        let mut page = vec![0u8; 0x40];
        unsafe { build_relay(page.as_mut_ptr() as usize, 0x1111, 0x2222) };
        // cmp byte [rip+0x29],0 -> +0x30 ; jne +6 -> +0x0f
        assert_eq!(&page[..9], &[0x80, 0x3D, 0x29, 0, 0, 0, 0x00, 0x75, 0x06]);
        assert_eq!(i32::from_le_bytes(page[0x0b..0x0f].try_into().unwrap()) + 0x0f, 0x20);
        assert_eq!(i32::from_le_bytes(page[0x11..0x15].try_into().unwrap()) + 0x15, 0x28);
        assert_eq!(u64::from_le_bytes(page[0x20..0x28].try_into().unwrap()), 0x1111);
        assert_eq!(u64::from_le_bytes(page[0x28..0x30].try_into().unwrap()), 0x2222);
        assert_eq!(page[0x30], 0);
    }
}
