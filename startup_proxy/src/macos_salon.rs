//! Salon multijoueur sous macOS, le pendant de salon.rs (voir son en-tête pour l'annonce, le verdict
//! et le BGE Check). Les textes sont communs (salon_texte.rs) : `salon.lua` à la fin de
//! `StagingRoom.lua`, le bouton du BGE Check et le minuteur dans `StagingRoom.xml` (macos_xml.rs),
//! `partie.lua` à la fin d'`InGame.lua`.
//!
//! Les natives rendent une seule chaîne (macos_options.rs dit pourquoi) ; une cale Lua devant chaque
//! bloc rend les fonctions qu'il attend : `Civ6Proxy_Version()` (la version, ou nil et la raison) et
//! `Civ6Proxy_PolActifs()` (un nombre). Pas de `Civ6Proxy_MediasPings`, ni de garde des messages ni
//! de relais réseau : ces fonctions-là ne sont pas portées, les blocs s'en passent.
//!
//! **Version annoncée.** Tant que tous les correctifs POL ne sont pas portés, le Mac annonce
//! `<version>-macos-partiel` : seul dans le salon (ou avec d'autres Mac de la même version), le
//! verdict est 1 et les correctifs portés s'appliquent ; avec un joueur Windows, les versions
//! diffèrent, le verdict est 0 pour tous et le BGE Check le montre. Sans quoi un Mac et un PC
//! annonceraient la même version sans appliquer les mêmes règles.

use super::journal::log;
use super::options::{argument_n, lire, publier, rendre, LuaCFn};
use super::salon_texte::{actif_off, new_token, patch_in_game_lua, patch_staging_lua, version_marquee, version_valide};
use super::version::VERSION;
use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub(crate) use super::salon_texte::{SCRIPT, SCRIPT_PARTIE, XML};

/// Suffixe de la version annoncée tant que des correctifs POL manquent au proxy de GameCore macOS.
const PARTIEL: &str = "-macos-partiel";

/// Bits de `RolloverProxyStatus` du proxy de GameCore (proxy/src/macos.rs).
const ST_ORIGINAL: u32 = 2;
const ST_POL: u32 = 16;

/// Le proxy de GameCore ouvert par le chargeur (macos.rs), 0 tant qu'il ne l'est pas.
pub(crate) static PROXY: AtomicUsize = AtomicUsize::new(0);

extern "C" {
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

/// La version que cette machine annonce.
pub fn annonce() -> String {
    format!("{VERSION}{PARTIEL}")
}

/// Pourquoi cette machine n'appliquerait pas les correctifs POL, ou None. Proxy de GameCore déjà
/// chargé : il le dit lui-même (bit `ST_POL`) ; sinon il le sera au lancement, avec les réglages et
/// le fichier tels qu'ils sont. Dans les deux cas, sa version doit être celle de ce chargeur.
fn indisponible() -> Option<String> {
    if lire().is_ok_and(|cfg| actif_off(&cfg)) {
        return Some("proxys désactivés (actif=off)".into());
    }
    let h = PROXY.load(Ordering::Relaxed) as *mut c_void;
    if !h.is_null() {
        unsafe {
            let p = dlsym(h, c"RolloverProxyStatus".as_ptr());
            let st = if p.is_null() { 0 } else { std::mem::transmute::<*mut c_void, extern "C" fn() -> u32>(p)() };
            if st & ST_ORIGINAL != 0 && st & ST_POL == 0 {
                return Some("le proxy de GameCore chargé n'a pas posé les correctifs POL (actif=off, ou jeu d'une autre version)".into());
            }
            let p = dlsym(h, c"BgdVersion".as_ptr());
            let v = if p.is_null() {
                None
            } else {
                let s = std::mem::transmute::<*mut c_void, extern "C" fn() -> *const u8>(p)();
                (!s.is_null()).then(|| version_valide(&(0..64).map(|i| *s.add(i)).take_while(|&b| b != 0).collect::<Vec<u8>>())).flatten()
            };
            return version_differente(v.as_deref());
        }
    }
    let Some(chemin) = super::dossier_dylibs().map(|d| d.join(super::noms::MAC_GAMECORE_PROXY)) else {
        return Some("dossier des dylibs introuvable".into());
    };
    match std::fs::read(&chemin) {
        Ok(data) => version_differente(version_marquee(&data).as_deref()),
        Err(_) => Some("proxy de GameCore non installé".into()),
    }
}

/// None si le proxy de GameCore a la version de ce chargeur ; sinon la raison, journalisée une fois.
fn version_differente(gamecore: Option<&str>) -> Option<String> {
    let nom = super::noms::MAC_GAMECORE_PROXY;
    let raison = match gamecore {
        Some(v) if v == VERSION => return None,
        Some(v) => format!("versions différentes : chargeur {VERSION}, {nom} {v} (installation partielle ?)"),
        None => format!("versions différentes : chargeur {VERSION}, {nom} sans version"),
    };
    static DERNIERE: Mutex<String> = Mutex::new(String::new());
    let mut d = DERNIERE.lock().unwrap_or_else(|e| e.into_inner());
    if *d != raison {
        log(&format!("salon : {raison}"));
        raison.clone_into(&mut d);
    }
    Some(raison)
}

/// `Civ6Proxy_MacVersion()` → `1\n<version annoncée>`, ou `0\n<raison>`.
unsafe extern "C-unwind" fn mac_version(l: *mut c_void) -> c_int {
    match indisponible() {
        None => rendre(l, &format!("1\n{}", annonce())),
        Some(r) => rendre(l, &format!("0\n{r}")),
    }
}

/// `Civ6Proxy_Jeton()` → une chaîne neuve à chaque appel.
unsafe extern "C-unwind" fn jeton(l: *mut c_void) -> c_int {
    rendre(l, &new_token())
}

/// `Civ6Proxy_VersionInstallee()` → la version de cette machine, qu'elle s'annonce ou non.
unsafe extern "C-unwind" fn version_installee(l: *mut c_void) -> c_int {
    rendre(l, &annonce())
}

/// `Civ6Proxy_MacPolActifs()` → `1` ou `0`.
unsafe extern "C-unwind" fn pol_actifs(l: *mut c_void) -> c_int {
    rendre(l, if super::pol::BgdPolActifs() != 0 { "1" } else { "0" })
}

/// `Civ6Proxy_Journal(texte)`.
pub(crate) unsafe extern "C-unwind" fn journal(l: *mut c_void) -> c_int {
    log(&argument_n(l, 1));
    0
}

const NATIVES_SALON: [(LuaCFn, &str); 5] = [
    (mac_version, "Civ6Proxy_MacVersion\0"),
    (jeton, "Civ6Proxy_Jeton\0"),
    (journal, "Civ6Proxy_Journal\0"),
    (super::pol::fixer_pol, "Civ6Proxy_FixerPol\0"),
    (version_installee, "Civ6Proxy_VersionInstallee\0"),
];
const NATIVES_PARTIE: [(LuaCFn, &str); 2] = [(journal, "Civ6Proxy_Journal\0"), (pol_actifs, "Civ6Proxy_MacPolActifs\0")];

const CALE_SALON: &str = r#"
-- macOS : natives à une seule chaîne (startup_proxy/src/macos_salon.rs).
if Civ6Proxy_MacVersion ~= nil then
	Civ6Proxy_Version = function()
		local ok, reste = string.match(tostring(Civ6Proxy_MacVersion()), "^(%d)\n(.*)$");
		if ok == "1" then return reste; end
		return nil, reste or "?";
	end
end
"#;
const CALE_PARTIE: &str = r#"
-- macOS : natives à une seule chaîne (startup_proxy/src/macos_salon.rs).
if Civ6Proxy_MacPolActifs ~= nil then
	Civ6Proxy_PolActifs = function() return tonumber(Civ6Proxy_MacPolActifs()) or 0; end
end
"#;

/// Le bloc ajouté à la fin du script, précédé de la cale (`script` finit par le bloc).
fn avec_cale(src: &[u8], script: Vec<u8>, cale: &str) -> Vec<u8> {
    let mut out = script;
    out.splice(src.len()..src.len(), cale.bytes());
    out
}

/// StagingRoom.lua avec le bloc du salon, si les natives sont posées.
pub unsafe fn staging_lua(l: *mut c_void, src: &[u8]) -> Vec<u8> {
    if !publier(l, &NATIVES_SALON) {
        log("salon : natives non posées, pas d'annonce");
        return src.to_vec();
    }
    avec_cale(src, patch_staging_lua(src), CALE_SALON)
}

/// InGame.lua avec le relais du verdict, si les natives sont posées.
pub unsafe fn partie_lua(l: *mut c_void, src: &[u8]) -> Vec<u8> {
    if !publier(l, &NATIVES_PARTIE) {
        log("partie : natives non posées, pas de relais du verdict");
        return src.to_vec();
    }
    avec_cale(src, patch_in_game_lua(src), CALE_PARTIE)
}

#[cfg(test)]
mod tests {
    use super::super::salon_texte::{LUA_BLOCK, PARTIE_BLOCK};
    use super::*;

    #[test]
    fn version_annoncee() {
        let v = annonce();
        assert!(v.starts_with(VERSION) && v != VERSION, "{v}");
        assert_eq!(version_valide(v.as_bytes()).as_deref(), Some(v.as_str()), "lisible par le BGE Check");
        assert!(!v.contains(['@', '#', ':', ',']), "{v}");
    }

    /// Le BGE Check s'ajoute aux salons installés (jeu de base, et MPH s'il est là).
    #[test]
    fn salons_installes() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let steam = std::path::Path::new(&home).join("Library/Application Support/Steam/steamapps");
        let base = steam.join("common/Sid Meier's Civilization VI/Civ6.app/Contents/Assets/Base/Assets/UI/FrontEnd/Multiplayer/StagingRoom.xml");
        let mut vus = 0;
        let mods = std::fs::read_dir(steam.join("workshop/content/289070")).into_iter().flatten().flatten().map(|e| e.path().join("ui/stagingroom.xml"));
        for chemin in std::iter::once(base).chain(mods) {
            let Ok(src) = std::fs::read(&chemin) else { continue };
            let out = super::super::salon_texte::patch_staging_xml(&src).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
            assert!(String::from_utf8(out).unwrap().contains("BGD_CheckButton"));
            vus += 1;
        }
        eprintln!("{vus} salon(s) vérifié(s)");
    }

    #[test]
    fn cales() {
        let s = avec_cale(b"Initialize();", patch_staging_lua(b"Initialize();"), CALE_SALON);
        let t = String::from_utf8(s).unwrap();
        let (cale, bloc) = (t.find("Civ6Proxy_MacVersion").unwrap(), t.find("(function()").unwrap());
        assert!(t.starts_with("Initialize();") && cale < bloc, "cale entre le script et le bloc");
        // Chaque native qu'attend un bloc est posée, ou rendue par la cale, ou absente à dessein.
        let posees: Vec<&str> = NATIVES_SALON.iter().chain(&NATIVES_PARTIE).map(|(_, n)| n.trim_end_matches('\0')).collect();
        let sans = ["Civ6Proxy_MediasPings", "Civ6Proxy_GardeMessages", "Civ6Proxy_RelayedPairs", "Civ6Proxy_RelinkStart", "Civ6Proxy_RelinkState", "Civ6Proxy_RelayForgetPair"];
        for bloc in [LUA_BLOCK, PARTIE_BLOCK] {
            for (i, _) in bloc.match_indices("Civ6Proxy_") {
                let nom: String = bloc[i..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                let par_cale = nom == "Civ6Proxy_Version" || nom == "Civ6Proxy_PolActifs";
                assert!(posees.contains(&nom.as_str()) || par_cale || sans.contains(&nom.as_str()), "{nom} attendue mais ni posée ni rendue");
            }
        }
    }
}
