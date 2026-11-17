//! Onglet BetterGameExperience de l'écran d'options sous macOS, le pendant d'options.rs. Le texte
//! inséré est le même (options_texte.rs) : bouton et panneau dans Options.xml (par macos_xml.rs),
//! bloc Lua à la fin d'Options.lua (par macos_lua.rs).
//!
//! Les fonctions natives diffèrent. L'exe macOS n'exporte de HavokScript que ce dont GameCore se
//! sert : ni `lua_setfield`, ni `lua_pushfstring`, ni `lua_cpcall`. On s'en passe :
//! - une native est créée par `hks_pushnamedcclosure` (exportée), son nom poussé par
//!   `lua_pushlstring` (retrouvée par signature), puis elle est écrite dans la table globale du
//!   contexte par `hks_obj_rawset` (exportée, sans métaméthode : rien ne peut lever d'erreur Lua) ;
//! - la table globale est l'objet en `L + 0x70`, le sommet de la pile en `L + 0x48`, chaque valeur
//!   y prend 16 octets (relevé dans `LuaUtils::ExposeLibraryToThread`) ;
//! - chaque native rend une seule chaîne, `1\n…` ou `0\n<erreur>`, qu'une cale Lua (`CALE`) décode
//!   pour garder les fonctions qu'attend options_proxy.lua : `Civ6Proxy_LireReglages`,
//!   `Civ6Proxy_EcrireReglages`, `Civ6Proxy_ReinitialiserReglages`.
//!
//! Le fichier de réglages est `~/Library/Application Support/BetterGameExperience/rollover_proxy.cfg`.
//! Le proxy de GameCore macOS ne le lit pas encore (aucun correctif porté) : « déjà lu » vaut 0.

// x86_64 : lua_pushlstring à relever, l'onglet n'y a pas encore ses natives.
#![cfg_attr(target_arch = "x86_64", allow(dead_code, unused_imports))]

use super::journal::{data_dir, log};
use super::macho;
use super::options_texte::{merge_config, patch_options_lua, patch_tick_intervals, reset_config, CFG_NAME};
use super::patch;
use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Début de `lua_pushlstring(L, s, len)` : prologue, puis la graine du hachage des chaînes
/// (`0x6b6f7265`). Relevé sur Civ6 Aspyr 1.4.6, arm64 0x10090f828.
#[cfg(target_arch = "aarch64")]
const SIG_PUSHLSTRING: &str = "f4 4f be a9 fd 7b 01 a9 fd 43 00 91 f3 03 00 aa a8 4c 8e 52 e8 6d ad 72 43 00 08 0b e8 03 80 52";

const L_TOP: usize = 0x48;
const L_GLOBALES: usize = 0x70;
const OBJET: usize = 16;

pub(crate) type LuaCFn = unsafe extern "C-unwind" fn(*mut c_void) -> c_int;
type PushClosure = unsafe extern "C-unwind" fn(*mut c_void, LuaCFn, c_int, *const c_char, c_int);
type PushLString = unsafe extern "C-unwind" fn(*mut c_void, *const c_char, usize);
type RawSet = unsafe extern "C-unwind" fn(*mut c_void, *const c_void, *const c_void, *const c_void);
type CheckLString = unsafe extern "C-unwind" fn(*mut c_void, c_int, *mut usize) -> *const c_char;

static PUSH_CLOSURE: AtomicUsize = AtomicUsize::new(0);
static PUSH_LSTRING: AtomicUsize = AtomicUsize::new(0);
static RAWSET: AtomicUsize = AtomicUsize::new(0);
static CHECK_LSTRING: AtomicUsize = AtomicUsize::new(0);

extern "C" {
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}
const RTLD_MAIN_ONLY: *mut c_void = -5isize as *mut c_void;

/// Cale Lua placée devant le bloc de l'onglet : les fonctions qu'il attend, sur les natives macOS.
const CALE: &str = r#"
-- macOS : les fonctions natives rendent une seule chaîne, « 1\n… » ou « 0\n<erreur> » (macos_options.rs).
if Civ6Proxy_MacLire ~= nil then
	local function bge_resultat(r)
		local ok, reste = string.match(r or "", "^(%d)\n(.*)$");
		return ok == "1", reste or "";
	end
	Civ6Proxy_LireReglages = function()
		local ok, reste = bge_resultat(Civ6Proxy_MacLire());
		if not ok then return nil, reste; end
		local installe, lu, texte = string.match(reste, "^(%d)\n(%d)\n(.*)$");
		return texte, tonumber(installe), tonumber(lu);
	end
	local function bge_edition(f)
		return function(lignes)
			local ok, reste = bge_resultat(f(tostring(lignes)));
			if not ok then return nil, reste; end
			return 1, tonumber(reste);
		end
	end
	Civ6Proxy_EcrireReglages = bge_edition(Civ6Proxy_MacEcrire);
	Civ6Proxy_ReinitialiserReglages = bge_edition(Civ6Proxy_MacReinitialiser);
end
"#;

fn chemin_reglages() -> Option<std::path::PathBuf> {
    Some(data_dir()?.join(CFG_NAME))
}

/// Le proxy de GameCore est-il posé ?
fn proxy_installe() -> bool {
    std::env::var_os("BGE_DYLIBS")
        .map(std::path::PathBuf::from)
        .or_else(|| data_dir().map(|d| d.join(super::noms::MAC_DOSSIER_DYLIBS)))
        .is_some_and(|d| d.join(super::noms::MAC_GAMECORE_PROXY).is_file())
}

pub(crate) fn lire() -> Result<String, String> {
    let path = chemin_reglages().ok_or("dossier de BetterGameExperience introuvable")?;
    match std::fs::read_to_string(&path) {
        Ok(t) => Ok(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("lecture de {CFG_NAME} impossible : {e}")),
    }
}

fn ecrire(texte: &str) -> Result<(), String> {
    let path = chemin_reglages().ok_or("dossier de BetterGameExperience introuvable")?;
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(&path, texte).map_err(|e| format!("écriture de {} impossible : {e}", path.display()))
}

fn modifier(changes: &str) -> Result<(), String> {
    ecrire(&merge_config(&lire()?, changes)?)?;
    log(&format!("options : {CFG_NAME} enregistré ({})", changes.lines().collect::<Vec<_>>().join(", ")));
    Ok(())
}

fn reinitialiser(cles: &str) -> Result<(), String> {
    let existant = lire()?;
    if existant.is_empty() {
        return Ok(());
    }
    ecrire(&reset_config(&existant, cles)?)?;
    log(&format!("options : {CFG_NAME} réinitialisé ({})", cles.lines().collect::<Vec<_>>().join(", ")));
    Ok(())
}

/// Premier argument, une chaîne (la cale passe toujours une chaîne).
unsafe fn argument(l: *mut c_void) -> String {
    argument_n(l, 1)
}

/// Pousse `s` (une seule valeur de retour).
pub(crate) unsafe fn rendre(l: *mut c_void, s: &str) -> c_int {
    let push: PushLString = std::mem::transmute(PUSH_LSTRING.load(Ordering::Relaxed));
    push(l, s.as_ptr() as *const c_char, s.len());
    1
}

/// Argument `i` en chaîne (un nombre est converti, comme par `tostring`).
pub(crate) unsafe fn argument_n(l: *mut c_void, i: c_int) -> String {
    let check: CheckLString = std::mem::transmute(CHECK_LSTRING.load(Ordering::Relaxed));
    let mut len = 0usize;
    let p = check(l, i, &mut len);
    if p.is_null() {
        return String::new();
    }
    String::from_utf8_lossy(std::slice::from_raw_parts(p as *const u8, len)).into_owned()
}

unsafe extern "C-unwind" fn mac_lire(l: *mut c_void) -> c_int {
    match lire() {
        Ok(t) => rendre(l, &format!("1\n{}\n0\n{t}", proxy_installe() as u8)),
        Err(e) => rendre(l, &format!("0\n{e}")),
    }
}

unsafe extern "C-unwind" fn mac_ecrire(l: *mut c_void) -> c_int {
    match modifier(&argument(l)) {
        Ok(()) => rendre(l, "1\n0"),
        Err(e) => {
            log(&format!("options : {e}"));
            rendre(l, &format!("0\n{e}"))
        }
    }
}

unsafe extern "C-unwind" fn mac_reinitialiser(l: *mut c_void) -> c_int {
    match reinitialiser(&argument(l)) {
        Ok(()) => rendre(l, "1\n0"),
        Err(e) => {
            log(&format!("options : {e}"));
            rendre(l, &format!("0\n{e}"))
        }
    }
}

const NATIVES: [(LuaCFn, &str); 3] = [
    (mac_lire, "Civ6Proxy_MacLire\0"),
    (mac_ecrire, "Civ6Proxy_MacEcrire\0"),
    (mac_reinitialiser, "Civ6Proxy_MacReinitialiser\0"),
];

/// Écrit les natives dans la table globale du contexte de `l` : closure, puis nom, puis
/// `rawset(globales, nom, closure)`, et la pile revient où elle était.
pub(crate) unsafe fn publier(l: *mut c_void, natives: &[(LuaCFn, &str)]) -> bool {
    if PUSH_LSTRING.load(Ordering::Relaxed) == 0 {
        return false;
    }
    let closure: PushClosure = std::mem::transmute(PUSH_CLOSURE.load(Ordering::Relaxed));
    let pushs: PushLString = std::mem::transmute(PUSH_LSTRING.load(Ordering::Relaxed));
    let rawset: RawSet = std::mem::transmute(RAWSET.load(Ordering::Relaxed));
    let top = (l as *mut u8).add(L_TOP) as *mut usize;
    let globales = (l as *const u8).add(L_GLOBALES) as *const c_void;
    for &(f, nom) in natives {
        let avant = *top;
        closure(l, f, 0, nom.as_ptr() as *const c_char, 0);
        pushs(l, nom.as_ptr() as *const c_char, nom.len() - 1);
        let sommet = *top;
        if sommet != avant + 2 * OBJET {
            *top = avant;
            log(&format!("options : pile inattendue en posant {} ({:#x} → {:#x})", nom.trim_end_matches('\0'), avant, sommet));
            return false;
        }
        rawset(l, globales, (sommet - OBJET) as *const c_void, (sommet - 2 * OBJET) as *const c_void);
        *top = avant;
    }
    true
}

/// Options.lua avec le limiteur d'images étendu et le bloc de l'onglet, précédé de la cale macOS si
/// les natives ont été posées dans le contexte de `l`.
pub unsafe fn options_lua(l: *mut c_void, src: &[u8]) -> Vec<u8> {
    let natives = publier(l, &NATIVES);
    if !natives {
        log("options : natives non posées, l'onglet dira que les réglages sont indisponibles");
    }
    let src = patch_tick_intervals(src).unwrap_or_else(|| {
        log("options : liste du limiteur d'images introuvable, entrées non ajoutées");
        src.to_vec()
    });
    let mut texte = patch_options_lua(&src);
    if natives {
        // patch_options_lua finit par le bloc de l'onglet : la cale va juste devant.
        let at = texte.len() - super::options_texte::lua_block().len();
        texte.splice(at..at, CALE.bytes());
    }
    texte
}

#[cfg(target_arch = "aarch64")]
pub unsafe fn installer(mh: *const macho::MachHeader64, slide: isize) -> Result<(), String> {
    let get = |nom: &std::ffi::CStr| {
        let p = dlsym(RTLD_MAIN_ONLY, nom.as_ptr());
        (!p.is_null()).then_some(p as usize).ok_or_else(|| format!("{} introuvable", nom.to_string_lossy()))
    };
    PUSH_CLOSURE.store(get(c"_Z21hks_pushnamedcclosureP9lua_StatePFiS0_EiPKci")?, Ordering::Relaxed);
    RAWSET.store(get(c"_Z14hks_obj_rawsetP9lua_StatePK9HksObjectS3_S3_")?, Ordering::Relaxed);
    CHECK_LSTRING.store(get(c"_Z17luaL_checklstringP9lua_StateiPm")?, Ordering::Relaxed);
    let (debut, taille) = macho::text_section(mh, slide).ok_or("section __text de l'exe introuvable")?;
    let text = std::slice::from_raw_parts(debut as *const u8, taille);
    let at = match patch::find(text, &patch::sig(SIG_PUSHLSTRING))[..] {
        [i] => debut + i,
        ref v => return Err(format!("lua_pushlstring trouvée {} fois au lieu d'une", v.len())),
    };
    PUSH_LSTRING.store(at, Ordering::Relaxed);
    log(&format!("options : lua_pushlstring en {:#x}, natives prêtes", at as isize - slide));
    Ok(())
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn installer(_mh: *const macho::MachHeader64, _slide: isize) -> Result<(), String> {
    Err("lua_pushlstring pas encore relevée en x86_64".into())
}

#[cfg(test)]
mod tests {
    use super::super::options_texte::{patch_options_lua, patch_options_xml, patch_tick_intervals};

    /// Les fichiers du jeu macOS installé, s'il l'est.
    fn fichier(nom: &str) -> Option<Vec<u8>> {
        let home = std::env::var_os("HOME")?;
        let p = std::path::Path::new(&home)
            .join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/Assets/Base/Assets/UI")
            .join(nom);
        let r = std::fs::read(&p).ok();
        if r.is_none() {
            eprintln!("absent, test sauté : {}", p.display());
        }
        r
    }

    #[test]
    fn options_du_jeu_macos() {
        if let Some(xml) = fichier("Options.xml") {
            let t = String::from_utf8(patch_options_xml(&xml).expect("ancres d'Options.xml")).unwrap();
            assert!(t.contains(r#"ID="ProxyTab""#) && t.contains(r#"ID="ProxyResetButton""#));
        }
        if let Some(lua) = fichier("Options.lua") {
            let tick = patch_tick_intervals(&lua).expect("liste du limiteur d'images");
            let t = patch_options_lua(&tick);
            assert!(t.ends_with(super::super::options_texte::lua_block().as_bytes()));
            assert!(t.windows(8).any(|w| w == b"\"75 FPS\""));
        }
    }
}
