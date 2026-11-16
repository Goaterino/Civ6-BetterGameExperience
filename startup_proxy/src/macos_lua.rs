//! Accroche de `luaL_loadbuffer` dans l'exe macOS, le pendant de l'accroche de son import sous
//! Windows (options.rs) : chaque script du jeu y passe, une fois lu (`LuaSystem`, son seul appelant).
//!
//! HavokScript est lié dans l'exe, sans symboles : `luaL_loadbuffer(L, buff, size, name)` est
//! retrouvée par sa signature. C'est une enveloppe de six instructions qui décale ses arguments,
//! ajoute les réglages du compilateur (`G(L) + 0x558`) et saute à `hksL_loadbuffer(L, réglages,
//! buff, size, name)`. On remplace son début par un saut vers notre accroche, qui appelle
//! `hksL_loadbuffer` elle-même : pas de trampoline, pas d'instruction déplacée.
//!
//! Relevé sur Civ6 Aspyr 1.4.6 (analyses/macos/README.md, étape 3) :
//! - arm64 : `luaL_loadbuffer` en 0x100985898, `hksL_loadbuffer` en 0x10099ab68 ;
//! - x86_64 : `luaL_loadbuffer` en 0x100a1ed80, `hksL_loadbuffer` en 0x100a34c90.

use super::macho;
use super::patch;
use super::journal::log;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::atomic::{AtomicUsize, Ordering};

/// mov x4,x3 ; mov x3,x2 ; mov x2,x1 ; ldr x8,[x0,#0x10] ; add x1,x8,#0x558 ; b hksL_loadbuffer
#[cfg(target_arch = "aarch64")]
const SIGNATURE: &str = "e4 03 03 aa e3 03 02 aa e2 03 01 aa 08 08 40 f9 01 61 15 91 ?? ?? ?? ??";
/// push rbp ; mov rbp,rsp ; mov r8,rcx ; mov rcx,rdx ; mov rdx,rsi ; mov esi,0x558 ;
/// add rsi,[rdi+0x10] ; pop rbp ; jmp hksL_loadbuffer
#[cfg(target_arch = "x86_64")]
const SIGNATURE: &str = "55 48 89 e5 49 89 c8 48 89 d1 48 89 f2 be 58 05 00 00 48 03 77 10 5d e9 ?? ?? ?? ??";

/// Décalage des réglages du compilateur dans l'état global de HavokScript (`G(L)`, en `L + 0x10`).
const REGLAGES: usize = 0x558;

/// Cible du saut final de l'enveloppe, à l'adresse `at` : `hksL_loadbuffer`.
#[cfg(target_arch = "aarch64")]
unsafe fn cible(at: usize) -> Option<usize> {
    let b = at + 20;
    let w = std::ptr::read_unaligned(b as *const u32);
    if w >> 26 != 0b000101 {
        return None;
    }
    let imm = ((w & 0x03ff_ffff) << 6) as i32 >> 6;
    Some(b.wrapping_add_signed(imm as isize * 4))
}

#[cfg(target_arch = "x86_64")]
unsafe fn cible(at: usize) -> Option<usize> {
    let rel = std::ptr::read_unaligned((at + 24) as *const i32);
    Some((at + 28).wrapping_add_signed(rel as isize))
}

type HksLoadbuffer = unsafe extern "C" fn(*mut c_void, *const c_void, *const c_char, usize, *const c_char) -> c_int;

static HKS_LOADBUFFER: AtomicUsize = AtomicUsize::new(0);
static COMPTE: AtomicUsize = AtomicUsize::new(0);

/// Le `luaL_loadbuffer` du jeu désormais. Pour l'instant, il note les scripts compilés.
unsafe extern "C" fn loadbuffer(l: *mut c_void, buff: *const c_char, size: usize, name: *const c_char) -> c_int {
    let n = COMPTE.fetch_add(1, Ordering::Relaxed);
    if n < 400 && std::env::var_os("BGE_DIAG_FICHIERS").is_some() {
        let nom = if name.is_null() { "(sans nom)".into() } else { CStr::from_ptr(name).to_string_lossy() };
        log(&format!("lua : {nom} ({size} octets)"));
        if nom.ends_with("Options.lua") && std::env::var_os("BGE_DIAG_FICHIERS").is_some() {
            pile("lua : pile de Options.lua");
        }
    }
    let hks: HksLoadbuffer = std::mem::transmute(HKS_LOADBUFFER.load(Ordering::Relaxed));
    let global = *((l as *const u8).add(0x10) as *const usize);
    hks(l, (global + REGLAGES) as *const c_void, buff, size, name)
}

pub unsafe fn installer(mh: *const macho::MachHeader64, slide: isize) -> Result<(), String> {
    let (debut, taille) = macho::text_section(mh, slide).ok_or("section __text de l'exe introuvable")?;
    let text = std::slice::from_raw_parts(debut as *const u8, taille);
    let trouves = patch::find(text, &patch::sig(SIGNATURE));
    let at = match trouves[..] {
        [i] => debut + i,
        _ => return Err(format!("luaL_loadbuffer trouvée {} fois au lieu d'une", trouves.len())),
    };
    SLIDE.store(slide as usize, Ordering::Relaxed);
    let hks = cible(at).ok_or("saut de luaL_loadbuffer illisible")?;
    HKS_LOADBUFFER.store(hks, Ordering::Relaxed);
    let saut = patch::jump(loadbuffer as *const () as usize);
    let methode = patch::write_code(at, &saut)?;
    let relu = std::slice::from_raw_parts(at as *const u8, saut.len());
    if relu != saut.as_slice() {
        return Err(format!("saut écrit ({methode}) mais relu différent : {relu:02x?}"));
    }
    log(&format!(
        "lua : luaL_loadbuffer accrochée en {:#x}, hksL_loadbuffer en {:#x} ({methode})",
        at as isize - slide,
        hks as isize - slide
    ));
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Diagnostic (BGE_DIAG_FICHIERS) : les fichiers XML que l'exe ouvre, et pour Options.xml, la pile
// d'appels en adresses de l'exe. Sert à retrouver la lecture XML (étape 3).

extern "C" {
    #[link_name = "fopen$DARWIN_EXTSN"]
    fn fopen_ext(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn backtrace(buf: *mut *mut c_void, size: c_int) -> c_int;
}

static SLIDE: AtomicUsize = AtomicUsize::new(0);
static COMPTE_XML: AtomicUsize = AtomicUsize::new(0);

unsafe fn pile(titre: &str) {
    let mut pile = [std::ptr::null_mut(); 32];
    let n = backtrace(pile.as_mut_ptr(), pile.len() as c_int).max(0) as usize;
    let slide = SLIDE.load(Ordering::Relaxed);
    let lignes: Vec<String> = pile[..n].iter().map(|&a| format!("{:#x}", (a as usize).wrapping_sub(slide))).collect();
    log(&format!("{titre} (adresses de l'exe) {}", lignes.join(" ")));
}

extern "C" {
    fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn pread(fd: c_int, buf: *mut c_void, n: usize, off: i64) -> isize;
    fn mmap(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, off: i64) -> *mut c_void;
    fn fdopen(fd: c_int, mode: *const c_char) -> *mut c_void;
}

static COMPTE_FD: AtomicUsize = AtomicUsize::new(0);

/// Chemin du descripteur ; note les fichiers .xml et .lua, avec la pile pour Options.xml.
unsafe fn diag_fd(quoi: &str, fd: c_int) {
    if fd < 0 {
        return;
    }
    let mut buf = [0u8; 1024];
    if fcntl(fd, 50 /* F_GETPATH */, buf.as_mut_ptr()) == -1 {
        return;
    }
    let p = CStr::from_ptr(buf.as_ptr() as *const c_char).to_string_lossy();
    // Les fichiers de l'interface seulement (les textes de la base de données sont des centaines).
    if !p.contains("/UI/") || !p.to_ascii_lowercase().ends_with(".xml") {
        return;
    }
    if p.ends_with("/Options.xml") {
        log(&format!("fichier : {quoi} {p}"));
        pile(&format!("fichier : pile de {quoi} Options.xml"));
    } else if COMPTE_FD.fetch_add(1, Ordering::Relaxed) < 200 {
        log(&format!("fichier : {quoi} {p}"));
    }
}

unsafe extern "C" fn diag_read(fd: c_int, buf: *mut c_void, n: usize) -> isize {
    diag_fd("read", fd);
    read(fd, buf, n)
}
unsafe extern "C" fn diag_pread(fd: c_int, buf: *mut c_void, n: usize, off: i64) -> isize {
    diag_fd("pread", fd);
    pread(fd, buf, n, off)
}
unsafe extern "C" fn diag_mmap(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, off: i64) -> *mut c_void {
    diag_fd("mmap", fd);
    mmap(addr, len, prot, flags, fd, off)
}
unsafe extern "C" fn diag_fdopen(fd: c_int, mode: *const c_char) -> *mut c_void {
    diag_fd("fdopen", fd);
    fdopen(fd, mode)
}

unsafe extern "C" fn diag_fopen(path: *const c_char, mode: *const c_char) -> *mut c_void {
    if !path.is_null() {
        let p = CStr::from_ptr(path).to_string_lossy();
        if COMPTE_XML.fetch_add(1, Ordering::Relaxed) < 600 {
            log(&format!("fichier : fopen {p}"));
            if p.ends_with("/Options.xml") || p.ends_with("/Options.lua") || p.ends_with("/MainMenu.xml") {
                let mut pile = [std::ptr::null_mut(); 24];
                let n = backtrace(pile.as_mut_ptr(), pile.len() as c_int).max(0) as usize;
                let slide = SLIDE.load(Ordering::Relaxed);
                let lignes: Vec<String> = pile[..n].iter().map(|&a| format!("{:#x}", (a as usize).wrapping_sub(slide))).collect();
                log(&format!("fichier : pile (adresses de l'exe) {}", lignes.join(" ")));
            }
        }
    }
    fopen_ext(path, mode)
}

static COMPTE_CF: AtomicUsize = AtomicUsize::new(0);

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFReadStreamCreateWithFile(alloc: *const c_void, url: *const c_void) -> *mut c_void;
    fn CFURLGetFileSystemRepresentation(url: *const c_void, resolve: u8, buf: *mut u8, len: isize) -> u8;
}

unsafe extern "C" fn diag_cfstream(alloc: *const c_void, url: *const c_void) -> *mut c_void {
    if COMPTE_CF.fetch_add(1, Ordering::Relaxed) < 300 {
        let mut buf = [0u8; 1024];
        let p = if !url.is_null() && CFURLGetFileSystemRepresentation(url, 1, buf.as_mut_ptr(), buf.len() as isize) != 0 {
            CStr::from_ptr(buf.as_ptr() as *const c_char).to_string_lossy().into_owned()
        } else {
            "?".into()
        };
        log(&format!("fichier : CFReadStreamCreateWithFile {p}"));
    }
    CFReadStreamCreateWithFile(alloc, url)
}

pub unsafe fn diag_fichiers(mh: *const macho::MachHeader64, slide: isize) {
    for (nom, f) in [
        ("_read", diag_read as *const () as usize),
        ("_pread", diag_pread as *const () as usize),
        ("_mmap", diag_mmap as *const () as usize),
        ("_fdopen", diag_fdopen as *const () as usize),
    ] {
        match macho::hook_import(mh, slide, nom, f) {
            Ok(n) => log(&format!("fichier : diagnostic de {nom} posé ({n} pointeur(s))")),
            Err(e) => log(&format!("fichier : diagnostic de {nom} impossible : {e}")),
        }
    }
    match macho::hook_import(mh, slide, "_CFReadStreamCreateWithFile", diag_cfstream as *const () as usize) {
        Ok(n) => log(&format!("fichier : diagnostic de CFReadStreamCreateWithFile posé ({n} pointeur(s))")),
        Err(e) => log(&format!("fichier : diagnostic de CFReadStreamCreateWithFile impossible : {e}")),
    }
    SLIDE.store(slide as usize, Ordering::Relaxed);
    match macho::hook_import(mh, slide, "_fopen$DARWIN_EXTSN", diag_fopen as *const () as usize) {
        Ok(n) => log(&format!("fichier : diagnostic de fopen posé ({n} pointeur(s))")),
        Err(e) => log(&format!("fichier : diagnostic de fopen impossible : {e}")),
    }
}
