//! Images Mach-O chargées (macOS) : l'exécutable principal, et ses pointeurs d'import. Le pendant
//! d'exe.rs (PE) pour les deux dylibs macOS, commun au chargeur et au proxy de GameCore.
//!
//! Le jeu est lié à l'ancienne (`LC_DYLD_INFO_ONLY`, sans « chained fixups ») : chaque import a une
//! case dans `__DATA_CONST,__got` ou `__DATA,__la_symbol_ptr`, retrouvée par la table des symboles
//! indirects. Ces cases sont des données : les réécrire ne touche pas au code signé.

use std::ffi::{c_char, c_int, c_void, CStr};

const MH_EXECUTE: u32 = 2;
const LC_SEGMENT_64: u32 = 0x19;
const LC_SYMTAB: u32 = 0x2;
const LC_DYSYMTAB: u32 = 0xb;
const SECTION_TYPE: u32 = 0xff;
const S_NON_LAZY_SYMBOL_POINTERS: u32 = 0x6;
const S_LAZY_SYMBOL_POINTERS: u32 = 0x7;
const INDIRECT_SYMBOL_LOCAL: u32 = 0x8000_0000;
const INDIRECT_SYMBOL_ABS: u32 = 0x4000_0000;

#[repr(C)]
pub struct MachHeader64 {
    magic: u32,
    cputype: i32,
    cpusubtype: i32,
    filetype: u32,
    ncmds: u32,
    sizeofcmds: u32,
    flags: u32,
    reserved: u32,
}

#[repr(C)]
struct LoadCommand {
    cmd: u32,
    cmdsize: u32,
}

#[repr(C)]
struct SegmentCommand64 {
    cmd: u32,
    cmdsize: u32,
    segname: [u8; 16],
    vmaddr: u64,
    vmsize: u64,
    fileoff: u64,
    filesize: u64,
    maxprot: i32,
    initprot: i32,
    nsects: u32,
    flags: u32,
}

#[repr(C)]
struct Section64 {
    sectname: [u8; 16],
    segname: [u8; 16],
    addr: u64,
    size: u64,
    offset: u32,
    align: u32,
    reloff: u32,
    nreloc: u32,
    flags: u32,
    reserved1: u32,
    reserved2: u32,
    reserved3: u32,
}

#[repr(C)]
struct SymtabCommand {
    cmd: u32,
    cmdsize: u32,
    symoff: u32,
    nsyms: u32,
    stroff: u32,
    strsize: u32,
}

#[repr(C)]
struct DysymtabCommand {
    cmd: u32,
    cmdsize: u32,
    ilocalsym: u32,
    nlocalsym: u32,
    iextdefsym: u32,
    nextdefsym: u32,
    iundefsym: u32,
    nundefsym: u32,
    tocoff: u32,
    ntoc: u32,
    modtaboff: u32,
    nmodtab: u32,
    extrefsymoff: u32,
    nextrefsyms: u32,
    indirectsymoff: u32,
    nindirectsyms: u32,
    extreloff: u32,
    nextrel: u32,
    locreloff: u32,
    nlocrel: u32,
}

#[repr(C)]
struct Nlist64 {
    n_strx: u32,
    n_type: u8,
    n_sect: u8,
    n_desc: u16,
    n_value: u64,
}

extern "C" {
    fn _dyld_image_count() -> u32;
    fn _dyld_get_image_header(i: u32) -> *const MachHeader64;
    fn _dyld_get_image_vmaddr_slide(i: u32) -> isize;
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
    fn getpagesize() -> c_int;
    static mach_task_self_: u32;
    fn mach_vm_protect(task: u32, addr: u64, size: u64, set_maximum: c_int, prot: c_int) -> c_int;
}

const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const VM_PROT_COPY: c_int = 0x10;

/// L'exécutable principal (`MH_EXECUTE`) et son décalage. Pas forcément l'image 0 : en arm64 sous
/// macOS 26, l'image 0 est la dylib insérée.
pub fn executable() -> Option<(*const MachHeader64, isize)> {
    unsafe {
        (0.._dyld_image_count()).find_map(|i| {
            let h = _dyld_get_image_header(i);
            (!h.is_null() && (*h).filetype == MH_EXECUTE).then(|| (h, _dyld_get_image_vmaddr_slide(i)))
        })
    }
}

/// Les commandes de chargement de l'image.
unsafe fn commands(mh: *const MachHeader64) -> impl Iterator<Item = *const LoadCommand> {
    let mut lc = mh.add(1) as *const LoadCommand;
    (0..(*mh).ncmds).map(move |_| {
        let cur = lc;
        lc = (lc as *const u8).add((*lc).cmdsize as usize) as *const LoadCommand;
        cur
    })
}

fn nom16(n: &[u8; 16]) -> &[u8] {
    &n[..n.iter().position(|&c| c == 0).unwrap_or(16)]
}

/// Rend inscriptible la page de données qui contient `addr` (`__DATA_CONST` est en lecture seule
/// une fois l'image liée).
unsafe fn writable(addr: usize) -> bool {
    let ps = getpagesize() as usize;
    let page = addr & !(ps - 1);
    mprotect(page as *mut c_void, ps, PROT_READ | PROT_WRITE) == 0
        || mach_vm_protect(mach_task_self_, page as u64, ps as u64, 0, PROT_READ | PROT_WRITE | VM_PROT_COPY) == 0
}

/// Remplace par `hook` chaque case d'import du symbole `name` (`_dlopen`) dans l'image `mh`. Rend le
/// nombre de cases remplacées.
pub unsafe fn hook_import(mh: *const MachHeader64, slide: isize, name: &str, hook: usize) -> Result<usize, String> {
    let (mut st, mut dt, mut linkedit) = (None, None, None);
    for lc in commands(mh) {
        match (*lc).cmd {
            LC_SYMTAB => st = Some(&*(lc as *const SymtabCommand)),
            LC_DYSYMTAB => dt = Some(&*(lc as *const DysymtabCommand)),
            LC_SEGMENT_64 => {
                let seg = &*(lc as *const SegmentCommand64);
                if nom16(&seg.segname) == b"__LINKEDIT" {
                    linkedit = Some(seg);
                }
            }
            _ => {}
        }
    }
    let (Some(st), Some(dt), Some(linkedit)) = (st, dt, linkedit) else {
        return Err("tables de symboles introuvables".into());
    };
    let base = (slide as u64).wrapping_add(linkedit.vmaddr).wrapping_sub(linkedit.fileoff) as usize;
    let syms = (base + st.symoff as usize) as *const Nlist64;
    let strings = (base + st.stroff as usize) as *const c_char;
    let indirects = (base + dt.indirectsymoff as usize) as *const u32;

    let mut n = 0;
    for lc in commands(mh) {
        if (*lc).cmd != LC_SEGMENT_64 {
            continue;
        }
        let seg = &*(lc as *const SegmentCommand64);
        let sects = std::slice::from_raw_parts((seg as *const SegmentCommand64).add(1) as *const Section64, seg.nsects as usize);
        for sect in sects {
            let kind = sect.flags & SECTION_TYPE;
            if kind != S_LAZY_SYMBOL_POINTERS && kind != S_NON_LAZY_SYMBOL_POINTERS {
                continue;
            }
            let cases = (slide as u64).wrapping_add(sect.addr) as *mut usize;
            for k in 0..(sect.size as usize / 8) {
                let ix = *indirects.add(sect.reserved1 as usize + k);
                if ix & (INDIRECT_SYMBOL_ABS | INDIRECT_SYMBOL_LOCAL) != 0 || ix >= st.nsyms {
                    continue;
                }
                let sym = CStr::from_ptr(strings.add((*syms.add(ix as usize)).n_strx as usize));
                if sym.to_bytes() != name.as_bytes() {
                    continue;
                }
                let case = cases.add(k);
                if !writable(case as usize) {
                    continue;
                }
                case.write_volatile(hook);
                n += 1;
            }
        }
    }
    Ok(n)
}

/// Section `__TEXT,__text` de l'image, en mémoire : (début, taille).
pub unsafe fn text_section(mh: *const MachHeader64, slide: isize) -> Option<(usize, usize)> {
    for lc in commands(mh) {
        if (*lc).cmd != LC_SEGMENT_64 {
            continue;
        }
        let seg = &*(lc as *const SegmentCommand64);
        if nom16(&seg.segname) != b"__TEXT" {
            continue;
        }
        let sects = std::slice::from_raw_parts((seg as *const SegmentCommand64).add(1) as *const Section64, seg.nsects as usize);
        if let Some(s) = sects.iter().find(|s| nom16(&s.sectname) == b"__text") {
            return Some(((slide as u64).wrapping_add(s.addr) as usize, s.size as usize));
        }
    }
    None
}
