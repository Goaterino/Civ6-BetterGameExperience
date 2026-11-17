//! Écriture dans le code du jeu sous macOS, le pendant de patch.rs. La méthode dépend de
//! l'architecture, mesurée sur un Mac ordinaire, SIP actif, avec la signature du jeu
//! (analyses/macos/README.md, « Résultats sur un Mac ordinaire ») :
//!
//! - **arm64** : toute page de code rendue inscriptible tue le processus. On copie les pages dans
//!   une zone `MAP_JIT` (droit `allow-jit` du jeu), on y écrit, puis on pose la copie à l'adresse
//!   d'origine par `mach_vm_remap`.
//! - **x86_64** (Rosetta, ou Mac Intel) : `mach_vm_protect` avec `VM_PROT_COPY` rend une copie
//!   privée de la page inscriptible ; on écrit, puis on la repasse en lecture et exécution. Sous
//!   Rosetta, la copie posée par `mach_vm_remap` est écrite mais le code traduit ne change pas.
//!
//! À faire au démarrage, avant que le jeu n'exécute ces pages : la copie remplace la page entière.

use std::ffi::{c_int, c_void};

extern "C" {
    static mach_task_self_: u32;
    fn getpagesize() -> c_int;
    fn sys_icache_invalidate(start: *mut c_void, len: usize);
    fn mach_vm_protect(task: u32, addr: u64, size: u64, set_maximum: c_int, prot: c_int) -> c_int;
    #[cfg(target_arch = "aarch64")]
    fn mmap(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, off: i64) -> *mut c_void;
    #[cfg(target_arch = "aarch64")]
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
    #[cfg(target_arch = "aarch64")]
    fn pthread_jit_write_protect_np(enabled: c_int);
    #[cfg(target_arch = "aarch64")]
    fn mach_vm_remap(
        target: u32,
        addr: *mut u64,
        size: u64,
        mask: u64,
        flags: c_int,
        src_task: u32,
        src_addr: u64,
        copy: c_int,
        cur: *mut c_int,
        max: *mut c_int,
        inherit: u32,
    ) -> c_int;
}

const VM_PROT_READ: c_int = 1;
const VM_PROT_WRITE: c_int = 2;
const VM_PROT_EXECUTE: c_int = 4;
const VM_PROT_COPY: c_int = 0x10;

/// Pages entières qui couvrent `[addr, addr + len)`.
fn pages(addr: usize, len: usize) -> (usize, usize) {
    let ps = unsafe { getpagesize() } as usize;
    let debut = addr & !(ps - 1);
    let fin = (addr + len + ps - 1) & !(ps - 1);
    (debut, fin - debut)
}

/// Écrit `bytes` à `addr`, dans le code du jeu. Rend la méthode employée.
#[cfg(target_arch = "aarch64")]
pub unsafe fn write_code(addr: usize, bytes: &[u8]) -> Result<&'static str, String> {
    const PROT_RWX: c_int = 7;
    const MAP_PRIVATE: c_int = 0x2;
    const MAP_ANON: c_int = 0x1000;
    const MAP_JIT: c_int = 0x800;
    const VM_FLAGS_FIXED: c_int = 0;
    const VM_FLAGS_OVERWRITE: c_int = 0x4000;
    const VM_INHERIT_COPY: u32 = 1;
    let (debut, len) = pages(addr, bytes.len());
    let copie = mmap(std::ptr::null_mut(), len, PROT_RWX, MAP_PRIVATE | MAP_ANON | MAP_JIT, -1, 0);
    if copie as isize == -1 {
        return Err(format!("mmap MAP_JIT : {}", std::io::Error::last_os_error()));
    }
    pthread_jit_write_protect_np(0);
    std::ptr::copy_nonoverlapping(debut as *const u8, copie as *mut u8, len);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), (copie as *mut u8).add(addr - debut), bytes.len());
    pthread_jit_write_protect_np(1);
    let (mut dest, mut cur, mut max) = (debut as u64, 0, 0);
    let kr = mach_vm_remap(
        mach_task_self_,
        &mut dest,
        len as u64,
        0,
        VM_FLAGS_FIXED | VM_FLAGS_OVERWRITE,
        mach_task_self_,
        copie as u64,
        1,
        &mut cur,
        &mut max,
        VM_INHERIT_COPY,
    );
    munmap(copie, len);
    if kr != 0 {
        return Err(format!("mach_vm_remap : erreur {kr}"));
    }
    if cur & VM_PROT_EXECUTE == 0 {
        return Err("mach_vm_remap : page posée non exécutable".into());
    }
    sys_icache_invalidate(addr as *mut c_void, bytes.len());
    Ok("copie MAP_JIT posée par mach_vm_remap")
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn write_code(addr: usize, bytes: &[u8]) -> Result<&'static str, String> {
    let (debut, len) = pages(addr, bytes.len());
    let kr = mach_vm_protect(mach_task_self_, debut as u64, len as u64, 0, VM_PROT_READ | VM_PROT_WRITE | VM_PROT_COPY);
    if kr != 0 {
        return Err(format!("mach_vm_protect RW|COPY : erreur {kr}"));
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), addr as *mut u8, bytes.len());
    let kr = mach_vm_protect(mach_task_self_, debut as u64, len as u64, 0, VM_PROT_READ | VM_PROT_EXECUTE);
    if kr != 0 {
        return Err(format!("mach_vm_protect RX : erreur {kr}"));
    }
    sys_icache_invalidate(addr as *mut c_void, bytes.len());
    Ok("mach_vm_protect avec VM_PROT_COPY")
}

/// Saut absolu vers `target`, sans registre à préserver : `x16` est libre à l'entrée d'une
/// fonction arm64 (registre d'appel inter-modules) ; en x86_64, `jmp [rip+0]` n'en touche aucun.
#[cfg(target_arch = "aarch64")]
pub fn jump(target: usize) -> Vec<u8> {
    // ldr x16, #8 ; br x16 ; .quad target
    let mut v = vec![0x50, 0x00, 0x00, 0x58, 0x00, 0x02, 0x1f, 0xd6];
    v.extend_from_slice(&(target as u64).to_le_bytes());
    v
}

#[cfg(target_arch = "x86_64")]
pub fn jump(target: usize) -> Vec<u8> {
    let mut v = vec![0xff, 0x25, 0, 0, 0, 0];
    v.extend_from_slice(&(target as u64).to_le_bytes());
    v
}

/// Positions de `sig` dans `hay`, les octets à `None` étant libres.
pub fn find(hay: &[u8], sig: &[Option<u8>]) -> Vec<usize> {
    if sig.is_empty() || hay.len() < sig.len() {
        return Vec::new();
    }
    let first = sig.iter().position(Option::is_some).unwrap_or(0);
    let b0 = sig[first].unwrap_or(0);
    (0..=hay.len() - sig.len())
        .filter(|&i| hay[i + first] == b0 && sig.iter().enumerate().all(|(k, s)| s.is_none_or(|b| hay[i + k] == b)))
        .collect()
}

/// Signature écrite en hexadécimal, `??` pour un octet libre : `"e4 03 ?? aa"`.
pub fn sig(hex: &str) -> Vec<Option<u8>> {
    hex.split_whitespace().map(|b| if b == "??" { None } else { u8::from_str_radix(b, 16).ok() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature() {
        let hay = [0u8, 1, 2, 3, 1, 9, 3];
        assert_eq!(find(&hay, &sig("01 ?? 03")), vec![1, 4]);
        assert_eq!(find(&hay, &sig("02 03")), vec![2]);
        assert!(find(&hay, &sig("07")).is_empty());
    }

    #[test]
    fn branches() {
        // b 0x42d2c0 en 0x846138 (GameCore arm64, fin de DiplomacyAction::Execute).
        assert_eq!(branche(0x846138, 0x42d2c0), Some([0x62, 0x9c, 0xef, 0x17]));
        assert_eq!(cible_branche(0x846138, u32::from_le_bytes([0x62, 0x9c, 0xef, 0x17])), Some(0x42d2c0));
        // bl 0x42aea8 en 0x846130.
        assert_eq!(cible_branche(0x846130, u32::from_le_bytes([0x5e, 0x93, 0xef, 0x97])), Some(0x42aea8));
        assert_eq!(branche(0, 1 << 27), None);
        assert_eq!(cible_branche(0, 0xd65f03c0), None);
    }
}

/// Mémoire exécutable pour les trampolines : `MAP_JIT` (droit `allow-jit` du jeu). En arm64, une
/// page RW passée RX tue le processus ; sous Rosetta, `MAP_JIT` passe aussi. `hint` : adresse
/// souhaitée (0 : n'importe où), que le noyau suit si elle est libre.
unsafe fn executable_at(bytes: &[u8], hint: usize) -> Result<usize, String> {
    extern "C" {
        #[link_name = "mmap"]
        fn mmap_jit(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, off: i64) -> *mut c_void;
    }
    const PROT_RWX: c_int = 7;
    const MAP_PRIVATE: c_int = 0x2;
    const MAP_ANON: c_int = 0x1000;
    const MAP_JIT: c_int = 0x800;
    let (_, len) = pages(0, bytes.len());
    let p = mmap_jit(hint as *mut c_void, len, PROT_RWX, MAP_PRIVATE | MAP_ANON | MAP_JIT, -1, 0);
    if p as isize == -1 {
        return Err(format!("mmap MAP_JIT : {}", std::io::Error::last_os_error()));
    }
    #[cfg(target_arch = "aarch64")]
    pthread_jit_write_protect_np(0);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len());
    #[cfg(target_arch = "aarch64")]
    pthread_jit_write_protect_np(1);
    sys_icache_invalidate(p, bytes.len());
    Ok(p as usize)
}

unsafe fn executable(bytes: &[u8]) -> Result<usize, String> {
    executable_at(bytes, 0)
}

/// Mémoire exécutable à moins de `portee` octets de `pres` : pour qu'un branchement relatif du code
/// du jeu l'atteigne (`b` arm64 : ±128 Mo). Essaie des adresses de 2 en 2 Mo de part et d'autre.
#[allow(dead_code)]
pub unsafe fn executable_near(bytes: &[u8], pres: usize, portee: usize) -> Result<usize, String> {
    extern "C" {
        fn munmap(addr: *mut c_void, len: usize) -> c_int;
    }
    const PAS: usize = 2 << 20;
    let (_, len) = pages(0, bytes.len());
    for k in 1..portee / PAS {
        for hint in [pres.checked_add(k * PAS), pres.checked_sub(k * PAS)].into_iter().flatten() {
            let p = executable_at(bytes, hint & !(PAS - 1))?;
            if p.abs_diff(pres) + len < portee {
                return Ok(p);
            }
            munmap(p as *mut c_void, len);
        }
    }
    Err(format!("pas de mémoire exécutable à moins de {} Mo de {pres:#x}", portee >> 20))
}

/// `b` arm64 de `depuis` vers `vers`, s'il est à portée (±128 Mo).
#[allow(dead_code)]
pub fn branche(depuis: usize, vers: usize) -> Option<[u8; 4]> {
    let ecart = vers as i64 - depuis as i64;
    (ecart % 4 == 0 && (-(1i64 << 27)..1i64 << 27).contains(&ecart))
        .then(|| (0x1400_0000u32 | ((ecart >> 2) as u32 & 0x03ff_ffff)).to_le_bytes())
}

/// Relais arm64 : les instructions `avant` (sans adresse relative), puis un saut absolu vers
/// `cible` par `x16` (`ldr x16, #8` ; `br x16` ; l'adresse).
#[allow(dead_code)]
pub fn relais(avant: &[u32], cible: usize) -> Vec<u8> {
    let mut v: Vec<u8> = avant.iter().flat_map(|m| m.to_le_bytes()).collect();
    v.extend_from_slice(&0x5800_0050u32.to_le_bytes());
    v.extend_from_slice(&0xd61f_0200u32.to_le_bytes());
    v.extend_from_slice(&(cible as u64).to_le_bytes());
    v
}

/// Cible d'un `b` ou `bl` arm64, ou None si ce n'en est pas un.
#[allow(dead_code)]
pub fn cible_branche(at: usize, mot: u32) -> Option<usize> {
    if mot & 0x7c00_0000 != 0x1400_0000 {
        return None;
    }
    let imm = ((mot & 0x03ff_ffff) << 6) as i32 >> 6;
    Some((at as i64 + imm as i64 * 4) as usize)
}

/// Détourne la fonction en `at` vers `hook`. Ses `prologue` premiers octets, recopiés tels quels
/// (ils ne doivent contenir aucune instruction relative à leur adresse), puis un saut vers la
/// suite, forment le trampoline rendu : l'appeler, c'est appeler la fonction d'origine.
pub unsafe fn hook_with_trampoline(at: usize, prologue: usize, hook: usize) -> Result<(usize, &'static str), String> {
    let slot = std::sync::atomic::AtomicUsize::new(0);
    let m = hook_with_trampoline_publie(at, prologue, hook, &slot)?;
    Ok((slot.load(std::sync::atomic::Ordering::Relaxed), m))
}

/// Comme `hook_with_trampoline`, mais le trampoline est rangé dans `slot` avant que le saut ne soit
/// écrit : une accroche appelée aussitôt le trouve. Rend la méthode d'écriture.
pub unsafe fn hook_with_trampoline_publie(
    at: usize,
    prologue: usize,
    hook: usize,
    slot: &std::sync::atomic::AtomicUsize,
) -> Result<&'static str, String> {
    let saut = jump(hook);
    if prologue < saut.len() {
        return Err(format!("prologue de {prologue} octets, le saut en demande {}", saut.len()));
    }
    let mut tramp = std::slice::from_raw_parts(at as *const u8, prologue).to_vec();
    tramp.extend(jump(at + prologue));
    let tramp = executable(&tramp)?;
    slot.store(tramp, std::sync::atomic::Ordering::Release);
    write_code(at, &saut)
}
