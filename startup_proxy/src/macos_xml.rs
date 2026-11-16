//! Lecture XML de l'exe macOS, le pendant de `hook_read_xml` sous Windows (proxy/src/exe.rs) :
//! chaque fichier XML de l'interface est lu par `load_file(doc, chemin, options)`, qui lit le
//! fichier entier dans le tampon du document (`doc + 8`, un `Platform::MemoryBuffer`) puis appelle
//! l'analyseur `parse(doc, texte, options)`. Retrouvées par une pile d'appels prise à l'ouverture
//! d'`Options.xml` (analyses/macos/README.md, étape 3), puis par leur signature.
//!
//! - L'accroche de `load_file` retient le chemin (en `wchar_t`, UTF-32 sous macOS) pour le fil
//!   courant, puis appelle l'originale par son trampoline.
//! - L'accroche de `parse`, appelée pendant ce chargement, peut remplacer le texte : elle l'écrit
//!   dans le tampon du document (`MemoryBuffer::GrowToFit`, exportée par l'exe), puis appelle
//!   l'analyseur d'origine avec lui. Hors d'un chargement de fichier (l'analyseur sert aussi aux
//!   textes en mémoire), elle passe la main telle quelle.
//!
//! Relevé sur Civ6 Aspyr 1.4.6, arm64 : `load_file` en 0x1005c57b8, `parse` en 0x1005c5920.
//! x86_64 : à relever.

// x86_64 : signatures à relever, l'accroche n'y est pas encore posée.
#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]

use super::journal::log;
use super::macho;
use super::patch;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(target_arch = "aarch64")]
const SIG_LOAD_FILE: &str = "ff c3 01 d1 f8 5f 03 a9 f6 57 04 a9 f4 4f 05 a9 fd 7b 06 a9 fd 83 01 91 a1 08 00 b4 f3 03 02 aa \
                             e8 03 01 aa f4 03 00 aa 09 14 40 f9 2a 19 40 f9 8a 00 00 b4";
#[cfg(target_arch = "aarch64")]
const SIG_PARSE: &str = "f6 57 bd a9 f4 4f 01 a9 fd 7b 02 a9 fd 83 00 91 f5 03 02 aa f4 03 01 aa f3 03 00 aa 1f e0 00 39 \
                         1f 24 00 f9 16 20 40 f9";
/// Prologue recopié dans le trampoline : quatre instructions de pile, sans adresse relative.
#[cfg(target_arch = "aarch64")]
const PROLOGUE: usize = 16;

static TRAMP_LOAD: AtomicUsize = AtomicUsize::new(0);
static TRAMP_PARSE: AtomicUsize = AtomicUsize::new(0);
static GROW_TO_FIT: AtomicUsize = AtomicUsize::new(0);
static COMPTE: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Chemin du fichier en cours de chargement sur ce fil.
    static CHEMIN: RefCell<Option<String>> = const { RefCell::new(None) };
}

type LoadFile = unsafe extern "C" fn(*mut c_void, *const u32, c_int) -> usize;
type Parse = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> usize;
type GrowToFit = unsafe extern "C" fn(*mut c_void, usize) -> usize;

/// Chemin en `wchar_t` (UTF-32), terminé par un NUL.
unsafe fn chemin(p: *const u32) -> String {
    if p.is_null() {
        return String::new();
    }
    (0..4096).map_while(|i| char::from_u32(*p.add(i)).filter(|&c| c != '\0')).collect()
}

/// Le fichier est-il `nom`, quel que soit le séparateur ou la casse ?
fn est(chemin: &str, nom: &str) -> bool {
    chemin.rsplit(['/', '\\']).next().is_some_and(|n| n.eq_ignore_ascii_case(nom))
}

/// Texte de remplacement pour ce fichier, s'il y en a un. Pour l'instant, en diagnostic seulement
/// (`BGE_DIAG_FICHIERS`), une marque invisible dans Options.xml : elle prouve que le remplacement
/// passe, sans rien changer à l'écran. L'onglet BetterGameExperience prendra sa place.
fn remplacement(chemin: &str, texte: &[u8]) -> Option<Vec<u8>> {
    if !est(chemin, "Options.xml") || (!cfg!(test) && std::env::var_os("BGE_DIAG_FICHIERS").is_none()) {
        return None;
    }
    let mut out = texte.to_vec();
    out.extend_from_slice(b"\n<!-- BetterGameExperience -->\n");
    Some(out)
}

unsafe extern "C" fn load_file(doc: *mut c_void, path: *const u32, options: c_int) -> usize {
    let original: LoadFile = std::mem::transmute(TRAMP_LOAD.load(Ordering::Relaxed));
    let c = chemin(path);
    if std::env::var_os("BGE_DIAG_FICHIERS").is_some() && COMPTE.fetch_add(1, Ordering::Relaxed) < 40 {
        log(&format!("xml : {c}"));
    }
    let avant = CHEMIN.with(|ch| ch.replace(Some(c)));
    let r = original(doc, path, options);
    CHEMIN.with(|ch| *ch.borrow_mut() = avant);
    r
}

unsafe extern "C" fn parse(doc: *mut c_void, texte: *const c_char, options: c_int) -> usize {
    let original: Parse = std::mem::transmute(TRAMP_PARSE.load(Ordering::Relaxed));
    let Some(c) = CHEMIN.with(|ch| ch.borrow_mut().take()) else {
        return original(doc, texte, options);
    };
    if texte.is_null() {
        return original(doc, texte, options);
    }
    let lu = CStr::from_ptr(texte).to_bytes();
    let Some(nouveau) = remplacement(&c, lu) else {
        return original(doc, texte, options);
    };
    // Le tampon du document, en doc + 8 : son pointeur de données est son premier champ.
    let tampon = (doc as *mut u8).add(8) as *mut c_void;
    let grow: GrowToFit = std::mem::transmute(GROW_TO_FIT.load(Ordering::Relaxed));
    if grow(tampon, nouveau.len() + 1) & 0xff == 0 {
        log(&format!("xml : {c} : tampon non agrandi, texte d'origine"));
        return original(doc, texte, options);
    }
    let data = *(tampon as *const *mut u8);
    std::ptr::copy_nonoverlapping(nouveau.as_ptr(), data, nouveau.len());
    *data.add(nouveau.len()) = 0;
    let r = original(doc, data as *const c_char, options);
    log(&format!("xml : {c} remplacé ({} → {} octets), analyse : {r}", lu.len(), nouveau.len()));
    r
}

extern "C" {
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}
/// `RTLD_MAIN_ONLY` : les symboles de l'exécutable principal.
const RTLD_MAIN_ONLY: *mut c_void = -5isize as *mut c_void;

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(mh: *const macho::MachHeader64, slide: isize) -> Result<(), String> {
    let grow = dlsym(RTLD_MAIN_ONLY, c"_ZN8Platform12MemoryBuffer9GrowToFitEm".as_ptr());
    if grow.is_null() {
        return Err("Platform::MemoryBuffer::GrowToFit introuvable".into());
    }
    GROW_TO_FIT.store(grow as usize, Ordering::Relaxed);
    let (debut, taille) = macho::text_section(mh, slide).ok_or("section __text de l'exe introuvable")?;
    let text = std::slice::from_raw_parts(debut as *const u8, taille);
    let unique = |nom: &str, sig: &str| -> Result<usize, String> {
        match patch::find(text, &patch::sig(sig))[..] {
            [i] => Ok(debut + i),
            ref v => Err(format!("{nom} trouvée {} fois au lieu d'une", v.len())),
        }
    };
    let (at_load, at_parse) = (unique("load_file", SIG_LOAD_FILE)?, unique("parse", SIG_PARSE)?);
    let (t, m) = patch::hook_with_trampoline(at_parse, PROLOGUE, parse as *const () as usize)?;
    TRAMP_PARSE.store(t, Ordering::Relaxed);
    let (t, _) = patch::hook_with_trampoline(at_load, PROLOGUE, load_file as *const () as usize)?;
    TRAMP_LOAD.store(t, Ordering::Relaxed);
    log(&format!(
        "xml : load_file accrochée en {:#x}, parse en {:#x} ({m})",
        at_load as isize - slide,
        at_parse as isize - slide
    ));
    Ok(())
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(_mh: *const macho::MachHeader64, _slide: isize) -> Result<(), String> {
    Err("lecture XML pas encore relevée en x86_64".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fichiers() {
        assert!(est(r"C:\Emu\AppAssets\Base\Assets\UI\Options.xml", "Options.xml"));
        assert!(est("Assets/Base/Assets/UI/options.xml", "Options.xml"));
        assert!(!est(r"C:\x\MyOptions.xml", "Options.xml"));
        assert!(remplacement("a/Options.xml", b"<x/>").is_some_and(|t| t.starts_with(b"<x/>")));
        assert!(remplacement("a/TopPanel.xml", b"<x/>").is_none());
    }
}
