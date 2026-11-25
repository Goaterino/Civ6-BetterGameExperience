//! Chargeur macOS : `libBGE_Chargeur.dylib`, injectée dans le jeu par `DYLD_INSERT_LIBRARIES`
//! (option de lancement Steam posée par le lanceur, voir updater/README.md, « macOS »). C'est le
//! pendant du proxy de démarrage `winmm.dll` : il fait charger le proxy de GameCore à la place de
//! l'original.
//!
//! - Son constructeur ne fait rien hors de `Civ6_Exe_Child`, le jeu : `Civ6_Exe`, le lanceur d'Aspyr
//!   qui le démarre, reçoit aussi la variable.
//! - Il remplace les pointeurs d'import `_dlopen` de l'exe (`__got`, `__la_symbol_ptr` : des données,
//!   pas du code signé), comme le chargeur Windows remplace `LoadLibraryW/A` dans sa table d'imports.
//!   Mesuré sur un Mac ordinaire, SIP actif : analyses/macos/README.md.
//! - Quand l'exe ouvre `GameCore_XP2.dll`, le chargeur ouvre `BGD_GameCore_XP2.dylib` (dans
//!   `BGE_DYLIBS`, posé par le lanceur) en lui passant le chemin de l'original
//!   (`BGE_GAMECORE_ORIGINAL`). Le proxy charge l'original par ses propres imports, qui ne passent pas
//!   par cette accroche. Si le proxy manque ou n'a pas pu charger l'original, l'original est chargé tel
//!   quel : le jeu tourne sans BetterGameExperience.
//!
//! Journal commun aux deux dylibs : `~/Library/Application Support/BetterGameExperience/RolloverProxy.log`.

use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

#[path = "../../proxy/src/noms.rs"]
#[allow(dead_code)]
mod noms;
#[path = "../../proxy/src/version.rs"]
#[allow(dead_code)]
mod version;
#[path = "../../proxy/src/macho.rs"]
#[allow(dead_code)]
pub(crate) mod macho;
#[path = "../../proxy/src/journal_mac.rs"]
pub(crate) mod journal;
#[path = "../../proxy/src/patch_mac.rs"]
#[allow(dead_code)]
pub(crate) mod patch;
#[path = "macos_lua.rs"]
mod lua;
#[path = "macos_xml.rs"]
mod xml;
#[path = "options_texte.rs"]
#[allow(dead_code)]
mod options_texte;
#[path = "macos_options.rs"]
mod options;
#[path = "macos_pol.rs"]
mod pol;
#[path = "macos_salon.rs"]
mod salon;
#[path = "salon_texte.rs"]
#[allow(dead_code)]
mod salon_texte;
#[path = "xml_texte.rs"]
mod xml_texte;
#[path = "macos_absent.rs"]
mod absent;
#[path = "macos_congres.rs"]
mod congres;
#[path = "../../proxy/src/congres_texte.rs"]
mod congres_texte;

use journal::log;

static STATUS: AtomicU32 = AtomicU32::new(0);
const ST_CHARGE: u32 = 1;
const ST_ACCROCHE: u32 = 2;
const ST_PROXY: u32 = 4;

/// Export de diagnostic, et marque que le lanceur cherche dans le fichier : chargé (1), accroche
/// posée (2), proxy de GameCore chargé à la place de l'original (4).
#[no_mangle]
pub extern "C" fn StartupProxyStatus() -> u32 {
    STATUS.load(Ordering::Relaxed)
}

/// Nom de l'exe du jeu, que Steam lance par `%command%`.
const JEU: &str = "Civ6_Exe_Child";

extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *const c_char;
    fn dladdr(addr: *const c_void, info: *mut DlInfo) -> c_int;
}

#[repr(C)]
struct DlInfo {
    dli_fname: *const c_char,
    dli_fbase: *mut c_void,
    dli_sname: *const c_char,
    dli_saddr: *mut c_void,
}

fn derniere_erreur() -> String {
    let e = unsafe { dlerror() };
    if e.is_null() {
        "?".into()
    } else {
        unsafe { CStr::from_ptr(e) }.to_string_lossy().into_owned()
    }
}

/// Dossier des dylibs : `BGE_DYLIBS`, sinon celui de ce chargeur (essais à la main).
pub(crate) fn dossier_dylibs() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("BGE_DYLIBS").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    let mut info = DlInfo { dli_fname: std::ptr::null(), dli_fbase: std::ptr::null_mut(), dli_sname: std::ptr::null(), dli_saddr: std::ptr::null_mut() };
    if unsafe { dladdr(StartupProxyStatus as *const c_void, &mut info) } == 0 || info.dli_fname.is_null() {
        return None;
    }
    let moi = PathBuf::from(unsafe { CStr::from_ptr(info.dli_fname) }.to_string_lossy().into_owned());
    moi.parent().map(Path::to_path_buf)
}

/// Le chemin désigne-t-il l'original de GameCore (`…/GameCore_XP2.dll`) ?
fn est_gamecore(chemin: &str) -> bool {
    chemin.rsplit('/').next().is_some_and(|n| n.eq_ignore_ascii_case(noms::MAC_GAMECORE))
}

/// Adresse du vrai `dlopen`, pour les appels de l'exe qui ne concernent pas GameCore.
static VRAI_DLOPEN: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Vrai pendant que le chargeur ouvre le proxy : un `dlopen` de GameCore venu de l'exe à ce
    /// moment-là passe tel quel.
    static EN_COURS: Cell<bool> = const { Cell::new(false) };
}

/// Le `dlopen` que l'exe appelle désormais.
unsafe extern "C" fn accroche_dlopen(chemin: *const c_char, mode: c_int) -> *mut c_void {
    let vrai: unsafe extern "C" fn(*const c_char, c_int) -> *mut c_void = std::mem::transmute(VRAI_DLOPEN.load(Ordering::Relaxed));
    if chemin.is_null() || EN_COURS.with(Cell::get) {
        return vrai(chemin, mode);
    }
    let demande = CStr::from_ptr(chemin).to_string_lossy().into_owned();
    if !est_gamecore(&demande) {
        return vrai(chemin, mode);
    }
    match ouvrir_proxy(&demande, mode) {
        Some(h) => h,
        None => vrai(chemin, mode),
    }
}

/// Ouvre le proxy de GameCore à la place de `original`. `None` (l'original sera ouvert tel quel) si
/// le proxy manque, ne s'ouvre pas, ou n'a pas pu charger l'original.
unsafe fn ouvrir_proxy(original: &str, mode: c_int) -> Option<*mut c_void> {
    let proxy = dossier_dylibs()?.join(noms::MAC_GAMECORE_PROXY);
    if !proxy.is_file() {
        log(&format!("chargeur : {original} demandé, {} absent : original chargé", proxy.display()));
        return None;
    }
    std::env::set_var("BGE_GAMECORE_ORIGINAL", original);
    let c = CString::new(proxy.as_os_str().as_encoded_bytes()).ok()?;
    EN_COURS.with(|e| e.set(true));
    let h = dlopen(c.as_ptr(), mode);
    EN_COURS.with(|e| e.set(false));
    if h.is_null() {
        log(&format!("chargeur : ouverture de {} impossible ({}) : original chargé", proxy.display(), derniere_erreur()));
        return None;
    }
    // Le proxy dit s'il a chargé l'original (bit 2 de RolloverProxyStatus) : sans lui, il ne peut
    // rien relayer.
    let status = dlsym(h, c"RolloverProxyStatus".as_ptr());
    let ok = !status.is_null() && {
        let f: extern "C" fn() -> u32 = std::mem::transmute(status);
        f() & 2 != 0
    };
    if !ok {
        log("chargeur : le proxy de GameCore n'a pas chargé l'original : original chargé");
        dlclose(h);
        return None;
    }
    STATUS.fetch_or(ST_PROXY, Ordering::Relaxed);
    salon::PROXY.store(h as usize, Ordering::Relaxed);
    log(&format!("chargeur : {original} demandé, {} chargé à sa place", noms::MAC_GAMECORE_PROXY));
    Some(h)
}

/// Pose l'accroche de `dlopen` dans l'exe.
unsafe fn installer() {
    let Some((mh, slide)) = macho::executable() else {
        log("chargeur : exécutable principal introuvable parmi les images");
        return;
    };
    VRAI_DLOPEN.store(dlopen as *const () as usize, Ordering::Relaxed);
    match macho::hook_import(mh, slide, "_dlopen", accroche_dlopen as *const () as usize) {
        Ok(0) => log("chargeur : import dlopen introuvable ou non modifiable dans l'exe"),
        Ok(n) => {
            STATUS.fetch_or(ST_ACCROCHE, Ordering::Relaxed);
            log(&format!("chargeur : accroche de dlopen posée ({n} pointeur{})", if n > 1 { "s" } else { "" }));
        }
        Err(e) => log(&format!("chargeur : accroche de dlopen impossible : {e}")),
    }
    if let Err(e) = lua::installer(mh, slide) {
        log(&format!("lua : accroche impossible : {e}"));
    }
    if let Err(e) = options::installer(mh, slide) {
        log(&format!("options : natives impossibles : {e}"));
    }
    if let Err(e) = xml::installer(mh, slide) {
        log(&format!("xml : accroche impossible : {e}"));
    }
    if let Err(e) = absent::installer(mh, slide) {
        log(&format!("absent : {e}"));
    }
    if std::env::var_os("BGE_DIAG_FICHIERS").is_some() {
        lua::diag_fichiers(mh, slide);
    }
}

extern "C" fn demarrage() {
    STATUS.fetch_or(ST_CHARGE, Ordering::Relaxed);
    let exe = std::env::current_exe().unwrap_or_default();
    let nom = exe.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    // BGE_CHARGEUR_PARTOUT : essais hors du jeu (updater/macos/test).
    if nom != JEU && std::env::var_os("BGE_CHARGEUR_PARTOUT").is_none() {
        return;
    }
    log(&format!("démarrage : chargeur macOS chargé dans {}, BetterGameExperience {} ({})", exe.display(), version::VERSION, std::env::consts::ARCH));
    unsafe { installer() };
}

/// Constructeur de la dylib, appelé par dyld au chargement.
#[used]
#[link_section = "__DATA,__mod_init_func"]
static CONSTRUCTEUR: extern "C" fn() = demarrage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamecore_reconnu() {
        assert!(est_gamecore("/x/Civ6.app/Contents/MacOS/GameCore_XP2.dll"));
        assert!(est_gamecore("GameCore_XP2.dll"));
        assert!(!est_gamecore("/x/Civ6.app/Contents/MacOS/GameCore_XP1.dll"));
        assert!(!est_gamecore("/x/GameCore_XP2.dll/autre"));
    }
}

/// Message en haut de la pile Lua de `l` (l'erreur d'une compilation ratée), par
/// `hks_obj_tolstring`, exportée par l'exe. Vide s'il n'est pas lisible.
pub(crate) unsafe fn lua_message(l: *mut c_void) -> String {
    const RTLD_MAIN_ONLY: *mut c_void = -5isize as *mut c_void;
    let f = dlsym(RTLD_MAIN_ONLY, c"_Z17hks_obj_tolstringP9lua_StateP9HksObjectPm".as_ptr());
    if f.is_null() {
        return String::new();
    }
    let tolstring: unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, *mut usize) -> *const c_char = std::mem::transmute(f);
    let top = *((l as *const u8).add(0x48) as *const usize);
    let mut len = 0usize;
    let p = tolstring(l, (top - 16) as *mut c_void, &mut len);
    if p.is_null() {
        return String::new();
    }
    String::from_utf8_lossy(std::slice::from_raw_parts(p as *const u8, len)).into_owned()
}
