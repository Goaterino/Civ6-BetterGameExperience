//! Proxy de GameCore macOS : `BGD_GameCore_XP2.dylib`, ouvert par le chargeur
//! (startup_proxy/src/macos.rs) à la place de `Civ6.app/Contents/MacOS/GameCore_XP2.dll`.
//!
//! Le GameCore macOS n'exporte que `DllCreateGameContext` et `DllDestroyGameContext`, que l'exe
//! cherche par `dlsym` (contre 157 exports sous Windows). Le constructeur charge l'original, dont le
//! chemin vient du chargeur (`BGE_GAMECORE_ORIGINAL`), et chaque export saute à la fonction de
//! l'original : un saut, pas un appel, qui garde tous les registres et la pile de l'appelant, quelle
//! que soit la signature de la fonction (inconnue). Comme sous Windows, l'original reste chargé
//! ensuite.
//!
//! Aucun correctif n'est encore posé (étapes 3 et 4 de analyses/macos/README.md) : le jeu tourne
//! avec BetterGameExperience chargé, sans changement.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::sync::atomic::{AtomicU32, Ordering};

#[path = "noms.rs"]
#[allow(dead_code)]
mod noms;
#[path = "version.rs"]
#[allow(dead_code)]
mod version;
#[path = "journal_mac.rs"]
#[allow(dead_code)]
mod journal;
#[path = "macho.rs"]
#[allow(dead_code)]
mod macho;

use journal::log;

static STATUS: AtomicU32 = AtomicU32::new(0);
const ST_CHARGE: u32 = 1;
const ST_ORIGINAL: u32 = 2;

/// Export de diagnostic, marque que le lanceur cherche dans le fichier, et réponse au chargeur :
/// chargé (1), original chargé et exports relayés (2). Sans le bit 2, le chargeur ouvre l'original
/// lui-même.
#[no_mangle]
pub extern "C" fn RolloverProxyStatus() -> u32 {
    STATUS.load(Ordering::Relaxed)
}

/// `BgdVersion=<VERSION>` terminée par un NUL, comme sous Windows (lib_windows.rs).
const LONGUEUR_MARQUE: usize = version::MARQUE.len() + version::VERSION.len() + 1;
#[used]
static MARQUE_VERSION: [u8; LONGUEUR_MARQUE] = {
    let (m, v) = (version::MARQUE.as_bytes(), version::VERSION.as_bytes());
    let mut out = [0u8; LONGUEUR_MARQUE];
    let mut i = 0;
    while i < m.len() {
        out[i] = m[i];
        i += 1;
    }
    let mut j = 0;
    while j < v.len() {
        out[i + j] = v[j];
        j += 1;
    }
    out
};

#[no_mangle]
pub extern "C" fn BgdVersion() -> *const u8 {
    MARQUE_VERSION[version::MARQUE.len()..].as_ptr()
}

/// Adresses des deux exports dans l'original, remplies par le constructeur.
static mut CREATE: usize = 0;
static mut DESTROY: usize = 0;

/// Saut vers la fonction dont l'adresse est dans `$slot`.
macro_rules! relais {
    ($name:ident, $slot:ident) => {
        #[no_mangle]
        #[unsafe(naked)]
        pub unsafe extern "C" fn $name() {
            #[cfg(target_arch = "aarch64")]
            core::arch::naked_asm!(
                "adrp x16, {slot}@PAGE",
                "ldr x16, [x16, {slot}@PAGEOFF]",
                "br x16",
                slot = sym $slot,
            );
            #[cfg(target_arch = "x86_64")]
            core::arch::naked_asm!("jmp qword ptr [rip + {slot}]", slot = sym $slot);
        }
    };
}
relais!(DllCreateGameContext, CREATE);
relais!(DllDestroyGameContext, DESTROY);

extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}
const RTLD_NOW: c_int = 0x2;
const RTLD_LOCAL: c_int = 0x4;

fn derniere_erreur() -> String {
    let e = unsafe { dlerror() };
    if e.is_null() {
        "?".into()
    } else {
        unsafe { CStr::from_ptr(e) }.to_string_lossy().into_owned()
    }
}

/// L'original : le chemin donné par le chargeur, sinon `GameCore_XP2.dll` à côté de l'exe.
fn chemin_original() -> Option<String> {
    if let Some(p) = std::env::var_os("BGE_GAMECORE_ORIGINAL").filter(|p| !p.is_empty()) {
        return Some(p.to_string_lossy().into_owned());
    }
    let exe = std::env::current_exe().ok()?;
    Some(exe.with_file_name(noms::MAC_GAMECORE).to_string_lossy().into_owned())
}

/// Charge l'original et remplit les relais.
unsafe fn charger_original() -> Result<String, String> {
    let chemin = chemin_original().ok_or("chemin de l'original introuvable")?;
    if chemin.rsplit('/').next().is_some_and(|n| n.eq_ignore_ascii_case(noms::MAC_GAMECORE_PROXY)) {
        return Err(format!("l'original désigne le proxy lui-même ({chemin})"));
    }
    let c = CString::new(chemin.as_str()).map_err(|e| e.to_string())?;
    let h = dlopen(c.as_ptr(), RTLD_NOW | RTLD_LOCAL);
    if h.is_null() {
        return Err(format!("ouverture de {chemin} impossible : {}", derniere_erreur()));
    }
    let create = dlsym(h, c"DllCreateGameContext".as_ptr()) as usize;
    let destroy = dlsym(h, c"DllDestroyGameContext".as_ptr()) as usize;
    if create == 0 || destroy == 0 {
        return Err(format!("{chemin} n'exporte pas DllCreateGameContext et DllDestroyGameContext"));
    }
    CREATE = create;
    DESTROY = destroy;
    Ok(chemin)
}

extern "C" fn demarrage() {
    STATUS.fetch_or(ST_CHARGE, Ordering::Relaxed);
    let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
    log(&format!("proxy chargé dans {exe}, BetterGameExperience {}", version::VERSION));
    match unsafe { charger_original() } {
        Ok(chemin) => {
            STATUS.fetch_or(ST_ORIGINAL, Ordering::Relaxed);
            log(&format!("original chargé : {chemin}, DllCreateGameContext et DllDestroyGameContext relayés"));
        }
        Err(e) => log(&format!("proxy : {e}")),
    }
}

/// Constructeur de la dylib, appelé par dyld au chargement (dans le `dlopen` du chargeur).
#[used]
#[link_section = "__DATA,__mod_init_func"]
static CONSTRUCTEUR: extern "C" fn() = demarrage;
