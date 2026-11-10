//! Accroches dans l'exe, communes aux deux proxys : ce proxy de GameCore et le proxy de démarrage
//! (`startup_proxy`, qui inclut ce fichier par `#[path]`).
//!
//! - **Table d'imports.** `iat_slot` trouve l'entrée d'un import ; `write_slot` la remplace.
//!   Deux proxys s'y enchaînent d'eux-mêmes : le second lit l'accroche du premier comme fonction
//!   d'origine.
//! - **Lecture XML.** `140a2d280` (DX12 ; `140a1f3f0` en DX11 ; `140a2d700` et `140a1fc80` au build 26407122) lit un fichier XML entier dans un
//!   tampon du jeu (`GameBuf`). Elle est retrouvée par une signature. Le proxy de démarrage la
//!   détourne le premier, au lancement : le proxy de GameCore trouve alors à sa place le
//!   `jmp [rip+0]` de ce détour, et l'enchaîne (`hook_read_xml`).

use crate::{patch, win};
use std::ffi::{c_char, c_void, CStr};

pub unsafe fn rd<T: Copy>(at: usize) -> T {
    std::ptr::read_unaligned(at as *const T)
}

/// Section `.text` du module : (début, taille).
pub unsafe fn text_section(module: usize) -> Option<(usize, usize)> {
    let nt = module + rd::<u32>(module + 0x3c) as usize;
    let count = rd::<u16>(nt + 6) as usize;
    let first = nt + 24 + rd::<u16>(nt + 20) as usize;
    (0..count).map(|i| first + 40 * i).find(|&s| &rd::<[u8; 8]>(s)[..5] == b".text").map(|s| {
        let size = rd::<u32>(s + 8) as usize;
        (module + rd::<u32>(s + 12) as usize, size)
    })
}

/// Le nom importé est-il `func`, tel quel ou décoré en C++ (`?func@@…`) ? L'exe importe les
/// fonctions de HavokScript sous leur nom décoré : `?luaL_loadbuffer@@YAHPEAUlua_State@@PEBD_K1@Z`.
pub fn is_import(name: &[u8], func: &str) -> bool {
    name == func.as_bytes() || name.strip_prefix(b"?").and_then(|n| n.strip_prefix(func.as_bytes())).is_some_and(|n| n.starts_with(b"@@"))
}

/// Entrée de la table d'imports du module pour `dll!func`.
pub unsafe fn iat_slot(module: usize, dll: &str, func: &str) -> Option<usize> {
    let nt = module + rd::<u32>(module + 0x3c) as usize;
    // PE32+ : répertoire des imports, deuxième entrée des répertoires de données.
    let imports = rd::<u32>(nt + 24 + 0x78) as usize;
    if imports == 0 {
        return None;
    }
    let mut d = module + imports;
    loop {
        let (oft, name, ft) = (rd::<u32>(d) as usize, rd::<u32>(d + 12) as usize, rd::<u32>(d + 16) as usize);
        if name == 0 {
            return None;
        }
        let dll_name = CStr::from_ptr((module + name) as *const c_char).to_string_lossy();
        if dll_name.eq_ignore_ascii_case(dll) {
            let names = if oft != 0 { oft } else { ft };
            for i in 0.. {
                let thunk = rd::<u64>(module + names + i * 8);
                if thunk == 0 {
                    break;
                }
                if thunk & (1 << 63) != 0 {
                    continue;
                }
                let f = CStr::from_ptr((module + thunk as usize + 2) as *const c_char);
                if is_import(f.to_bytes(), func) {
                    return Some(module + ft + i * 8);
                }
            }
            return None;
        }
        d += 20;
    }
}

pub unsafe fn write_slot(slot: usize, value: usize) -> bool {
    let mut old = 0u32;
    if win::VirtualProtect(slot as *mut c_void, 8, win::PAGE_EXECUTE_READWRITE, &mut old) == 0 {
        return false;
    }
    *(slot as *mut usize) = value;
    win::VirtualProtect(slot as *mut c_void, 8, old, &mut old);
    true
}

/// Le chemin finit-il par `/nom` ou `\nom` (ou est-il `nom`), sans tenir compte de la casse ?
pub fn is_file(path: &str, name: &str) -> bool {
    // Sans allocation : appelée des dizaines de fois par script compilé.
    let (p, n) = (path.as_bytes(), name.as_bytes());
    if p.len() < n.len() || !p[p.len() - n.len()..].eq_ignore_ascii_case(n) {
        return false;
    }
    p.len() == n.len() || matches!(p[p.len() - n.len() - 1], b'/' | b'\\')
}

/// Tampon du jeu rempli par la lecture d'un fichier (voir `1409971c0`, qui le réserve).
#[repr(C)]
pub struct GameBuf {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
    /// Objet allocateur : table virtuelle `alloc(this, taille, alignement)`, `free(this, ptr)`.
    pub alloc: *const *const usize,
}

impl GameBuf {
    /// Le texte lu, sans le NUL final qu'ajoute la lecture.
    pub unsafe fn text(&self) -> Option<&[u8]> {
        if self.ptr.is_null() || self.len == 0 {
            return None;
        }
        let len = if *self.ptr.add(self.len - 1) == 0 { self.len - 1 } else { self.len };
        Some(std::slice::from_raw_parts(self.ptr, len))
    }
}

/// Remplace le contenu du tampon, NUL final compris.
pub unsafe fn replace_buffer(b: &mut GameBuf, data: &[u8]) -> bool {
    let need = data.len() + 1;
    if need > b.cap {
        if b.alloc.is_null() {
            return false;
        }
        let vt = *b.alloc;
        let alloc: unsafe extern "C" fn(*const c_void, usize, usize) -> *mut u8 = std::mem::transmute(*vt);
        let free: unsafe extern "C" fn(*const c_void, *mut u8) = std::mem::transmute(*vt.add(1));
        let p = alloc(b.alloc as *const c_void, need, 0x10);
        if p.is_null() {
            return false;
        }
        if !b.ptr.is_null() {
            free(b.alloc as *const c_void, b.ptr);
        }
        b.ptr = p;
        b.cap = need;
    }
    std::ptr::copy_nonoverlapping(data.as_ptr(), b.ptr, data.len());
    *b.ptr.add(data.len()) = 0;
    b.len = need;
    true
}

pub type ReadXml = unsafe extern "C" fn(*const u16, *mut GameBuf) -> i32;

/// Prologue de la lecture XML, puis son appel à l'ouverture (rel32 libre) et le test qui suit.
pub const XML_SIG: [u8; 0x47] = [
    0x48, 0x8b, 0xc4, 0x48, 0x89, 0x58, 0x08, 0x57, 0x48, 0x83, 0xec, 0x50, 0x48, 0x8b, 0xfa, 0x48, 0xc7, 0x42,
    0x08, 0x00, 0x00, 0x00, 0x00, 0xbb, 0x01, 0x00, 0x00, 0x00, 0x48, 0xc7, 0x40, 0xc8, 0xff, 0xff, 0xff, 0xff,
    0x48, 0x8d, 0x50, 0xc8, 0x89, 0x58, 0xd4, 0x89, 0x58, 0xd0, 0x48, 0xc7, 0x40, 0xd8, 0x00, 0x00, 0x00, 0x00,
    0x48, 0xc7, 0x40, 0xe0, 0x00, 0x00, 0x00, 0x00, 0xe8, 0x3d, 0x92, 0xf6, 0xff, 0x3b, 0xc3, 0x74, 0x50,
];
/// Octets du rel32 de l'appel, différents d'un exe à l'autre.
const XML_SIG_FREE: std::ops::Range<usize> = 0x3e..0x42;
/// mov rax,rsp / mov [rax+8],rbx / push rdi / sub rsp,50h / mov rdi,rdx / mov qword [rdx+8],0 :
/// aucune instruction relative, recopiées telles quelles.
pub const XML_PROLOGUE: usize = 23;
/// Longueur du saut absolu d'un détour (`patch`) : `jmp [rip+0]` puis l'adresse.
const JMP_LEN: usize = 14;
const JMP_ABS: [u8; 6] = [0xff, 0x25, 0, 0, 0, 0];

/// Positions de la signature en un seul passage : (intacte, déjà détournée). Détournée, ses 14
/// premiers octets sont le saut absolu d'un détour déjà posé, au lieu du début du prologue.
pub fn find_sigs(hay: &[u8]) -> (Vec<usize>, Vec<usize>) {
    let (mut intactes, mut detournees) = (Vec::new(), Vec::new());
    for (at, w) in hay.windows(XML_SIG.len()).enumerate() {
        // La suite, commune aux deux formes, d'abord : elle écarte presque toutes les positions.
        let suite = |i: usize| XML_SIG_FREE.contains(&i) || w[i] == XML_SIG[i];
        if !(JMP_LEN..XML_SIG.len()).all(suite) {
            continue;
        }
        if w[..JMP_LEN] == XML_SIG[..JMP_LEN] {
            intactes.push(at);
        } else if w.starts_with(&JMP_ABS) {
            detournees.push(at);
        }
    }
    (intactes, detournees)
}

/// Positions de la signature, intacte ou (`detoured`) déjà détournée.
pub fn find_sig(hay: &[u8], detoured: bool) -> Vec<usize> {
    let (intactes, detournees) = find_sigs(hay);
    if detoured { detournees } else { intactes }
}

/// Détour posé sur la lecture XML.
pub struct XmlHook {
    pub site: usize,
    /// Trampoline à appeler pour lire le fichier : la fonction d'origine, ou le détour précédent.
    pub tramp: usize,
    /// Octets du site avant le détour, remis par `unhook_read_xml`.
    pub saved: [u8; JMP_LEN],
    /// Un autre proxy avait déjà détourné la lecture.
    pub chained: bool,
}

/// Détourne la lecture XML de l'exe vers `hook`. Si un autre proxy l'a déjà fait, son saut absolu
/// est recopié tel quel dans le trampoline : il ne dépend pas de son adresse. Notre détour appelle
/// donc le sien, qui appelle la fonction d'origine.
pub unsafe fn hook_read_xml(exe: usize, hook: usize) -> Result<XmlHook, String> {
    let (start, size) = text_section(exe).ok_or("section .text de l'exe introuvable")?;
    let text = std::slice::from_raw_parts(start as *const u8, size);
    let (intactes, detournees) = find_sigs(text);
    let (at, chained) = match (&intactes[..], &detournees[..]) {
        ([at], []) => (start + at, false),
        ([], [at]) => (start + at, true),
        (a, b) => return Err(format!("lecture XML trouvée {} fois, et {} fois déjà détournée, au lieu d'une", a.len(), b.len())),
    };
    let saved: [u8; JMP_LEN] = rd(at);
    let (prologue, resume) = if chained { (&saved[..], at + JMP_LEN) } else { (&XML_SIG[..XML_PROLOGUE], at + XML_PROLOGUE) };
    let tramp = patch::install_at("lecture XML", at, prologue, resume, hook)?;
    Ok(XmlHook { site: at, tramp, saved, chained })
}

/// Remet le site tel qu'il était avant `hook_read_xml` : la fonction d'origine, ou le détour
/// précédent.
pub unsafe fn unhook_read_xml(h: &XmlHook) {
    patch::restore_at(h.site, &h.saved);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_deja_detournee() {
        let mut code = vec![0x90u8; 16];
        code.extend_from_slice(&XML_SIG);
        code.extend_from_slice(&[0xcc; 16]);
        assert_eq!(find_sig(&code, false), vec![16]);
        assert!(find_sig(&code, true).is_empty());
        // Le détour d'un autre proxy : jmp [rip+0] et une adresse quelconque.
        code[16..22].copy_from_slice(&JMP_ABS);
        code[22..30].copy_from_slice(&0x7ff8_1234_5678u64.to_le_bytes());
        assert!(find_sig(&code, false).is_empty());
        assert_eq!(find_sig(&code, true), vec![16]);
        // Le reste de la signature doit toujours correspondre.
        code[16 + 0x44] ^= 1;
        assert!(find_sig(&code, true).is_empty());
    }
}
