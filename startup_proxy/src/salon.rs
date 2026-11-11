//! Salon multijoueur : chaque joueur annonce sa version de BetterGameDLLs, et l'hôte établit si tous
//! les humains ont la même (voir `salon.lua`).
//!
//! L'accroche `luaL_loadbuffer` d'`options` ajoute `salon.lua` à la fin de `StagingRoom.lua`, ou de
//! son remplaçant par un mod (MPH en a un, du même nom), après avoir posé dans la table globale du
//! contexte cinq fonctions natives : `Civ6Proxy_Version()`, `Civ6Proxy_Jeton()`,
//! `Civ6Proxy_Journal(texte)`, `Civ6Proxy_FixerPol(actif, raison)` et
//! `Civ6Proxy_VersionInstallee()`.
//!
//! **Le BGE Check.** Après la lecture de `StagingRoom.xml` (ou de celui de MPH), `patch_staging_xml`
//! ajoute juste après le bouton `BackButton` de la barre du haut un bouton `BGD_CheckButton`, placé
//! comme le « MOD CHECK » de MPH (`Anchor="R,C" Offset="180,0"`) mais à gauche (`Anchor="L,C"`) :
//! symétrique par rapport à l'axe vertical du centre de l'écran. À la fin du contexte, un minuteur
//! `BGD_SalonMinuteur` (environ 0,5 s) rythme l'annonce d'arrivée et le délai des réponses.
//!
//! **Les correctifs POL** (« proxy-only lobbies ») changent la simulation : le proxy de GameCore ne
//! les applique que si `BgdPolActifs` (export de ce proxy, voir lib.rs) rend 1. Cet état est tenu
//! par l'interface :
//! - dans le salon, `salon.lua` le met à 0 à chaque ouverture, puis au verdict de l'hôte ;
//! - au chargement d'une partie locale (solo, hotseat), `chargement.lua`, ajouté à la fin de
//!   `LoadScreen.lua`, le met à 1 : une seule machine, rien à désynchroniser ;
//! - en cours de partie, l'hôte relaie le verdict à qui rejoint ou se reconnecte : `partie.lua`,
//!   ajouté à la fin de `InGame.lua`, y ajoute son jeton sans changer l'état.
//!
//! Une machine qui n'appliquerait pas ces correctifs ne doit pas s'annoncer : `Civ6Proxy_Version()`
//! rend alors nil et la raison (proxy de GameCore absent, désactivé, d'une autre version que ce
//! proxy, ou correctifs non posés).
//!
//! La version annoncée est celle de ce proxy, mais les correctifs POL sont ceux du proxy de GameCore :
//! après une installation partielle (une seule des deux DLL mise à jour), deux machines annonceraient
//! la même version sans appliquer les mêmes règles. Les deux DLL compilent `proxy/src/version.rs` ;
//! leurs `VERSION` doivent être égales, lues par l'export `BgdVersion` du proxy de GameCore s'il est
//! chargé, sinon par sa marque `BgdVersion=<version>` dans le fichier (`version_du_fichier`).

use crate::options::{hks, publish_natives, push_str, Natives};
use crate::version::{MARQUE, VERSION};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

pub const LUA_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("salon.lua"));
pub const SCRIPT: &str = "stagingroom.lua";
pub const XML: &str = "StagingRoom.xml";
/// Bouton de l'hôte, juste après celui-ci (même parent : la barre du haut du salon).
const ANCRE_XML: &str = r#"ID="BackButton""#;
const BOUTON_CHECK: &str = r#"<GridButton ID="BGD_CheckButton" Style="ShellButtonOrnateFlat" Anchor="L,C" Offset="180,0" Size="150,38" String="BGE CHECK" Hidden="1"/>"#;
/// Minuteur du salon : une animation d'une demi-seconde environ (vitesse 2), relancée à sa fin.
const MINUTEUR: &str = r#"  <!-- BetterGameExperience : minuteur du salon (startup_proxy/src/salon.rs) -->
  <SlideAnim ID="BGD_SalonMinuteur" Size="1,1" Begin="0,0" End="0,0" Function="Root" Speed="2" Cycle="Once" Stopped="1"/>"#;
pub const CHARGEMENT_BLOCK: &str = include_str!("chargement.lua");
pub const SCRIPT_CHARGEMENT: &str = "loadscreen.lua";
pub const PARTIE_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("partie.lua"));
pub const SCRIPT_PARTIE: &str = "ingame.lua";
/// Option de partie « No First/Last moves (BGD) » (départ et fin de tour simultanés, depart_tour/README.md).
pub const OPTION_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("option_partie.lua"));
pub const SCRIPT_OPTION: &str = "setupparameters.lua";

/// Correctifs POL actifs pour la partie en cours (voir l'en-tête). Désactivés tant que l'interface
/// n'a rien dit.
static POL: AtomicBool = AtomicBool::new(false);

pub fn pol_actifs() -> bool {
    POL.load(Ordering::Relaxed)
}

/// Bits de `RolloverProxyStatus`, l'export d'état du proxy de GameCore (proxy/src/lib.rs).
const ST_ORIGINAL: u32 = 2;
const ST_POL: u32 = 16;

/// `actif=off` dans le fichier de réglages, dernière occurrence comme `read_config`.
pub fn actif_off(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("actif=")).last().is_some_and(|v| v.trim() == "off")
}

/// Pourquoi cette machine n'appliquerait pas les correctifs POL, ou None si elle les appliquera.
/// Proxy de GameCore déjà chargé : il le dit lui-même (bit ST_POL). Sinon, il le sera au lancement
/// de la partie, avec le fichier de réglages tel qu'il est.
/// Ensuite, dans les deux cas, sa version doit être celle de ce proxy (`version_differente`).
fn pol_indisponible() -> Option<String> {
    let m = unsafe { crate::win::GetModuleHandleW(crate::win::wide(crate::noms::GAMECORE_PROXY).as_ptr()) };
    unsafe {
        let p = if m.is_null() { std::ptr::null_mut() } else { crate::win::GetProcAddress(m, b"RolloverProxyStatus\0".as_ptr()) };
        if !p.is_null() {
            let status: extern "system" fn() -> u32 = std::mem::transmute(p);
            let st = status();
            if st & ST_ORIGINAL != 0 {
                if st & ST_POL == 0 {
                    return Some("le proxy de GameCore chargé n'a pas posé les correctifs POL (actif=off, ou jeu d'une autre version)".to_string());
                }
                return version_differente(version_chargee(m).as_deref());
            }
        }
    }
    match crate::options::read_config() {
        Ok((_, false)) => Some("proxy de GameCore non installé".into()),
        Ok((cfg, true)) if actif_off(&cfg) => Some("proxys désactivés (actif=off)".into()),
        Ok(_) if !m.is_null() => version_differente(unsafe { version_chargee(m) }.as_deref()),
        Ok(_) => version_differente(version_du_fichier().as_deref()),
        Err(e) => Some(e),
    }
}

/// None si le proxy de GameCore a la version de ce proxy ; sinon la raison, journalisée avec les
/// deux versions (une fois par raison). `gamecore` : sa version, None s'il n'en dit pas (proxy plus
/// ancien que l'export `BgdVersion`, compté comme une autre version).
fn version_differente(gamecore: Option<&str>) -> Option<String> {
    let raison = raison_version(gamecore)?;
    static DERNIERE: Mutex<String> = Mutex::new(String::new());
    let mut d = DERNIERE.lock().unwrap_or_else(|e| e.into_inner());
    if *d != raison {
        crate::log(&format!("salon : {raison}"));
        raison.clone_into(&mut d);
    }
    Some(raison)
}

pub fn raison_version(gamecore: Option<&str>) -> Option<String> {
    let nom = crate::noms::GAMECORE_PROXY;
    match gamecore {
        Some(v) if v == VERSION => None,
        Some(v) => Some(format!("versions différentes : winmm.dll {VERSION}, {nom} {v} (installation partielle ?)")),
        None => Some(format!("versions différentes : winmm.dll {VERSION}, {nom} sans version (antérieur à l'export BgdVersion)")),
    }
}

/// Version du proxy de GameCore chargé, par son export `BgdVersion` (chaîne terminée par un NUL).
unsafe fn version_chargee(m: crate::win::HMODULE) -> Option<String> {
    let p = crate::win::GetProcAddress(m, b"BgdVersion\0".as_ptr());
    if p.is_null() {
        return None;
    }
    let f: extern "system" fn() -> *const u8 = std::mem::transmute(p);
    let s = f();
    if s.is_null() {
        return None;
    }
    // 64 octets au plus : une version tient en quelques-uns.
    let octets: Vec<u8> = (0..64).map(|i| *s.add(i)).take_while(|&b| b != 0).collect();
    version_valide(&octets)
}

/// Version du proxy de GameCore pas encore chargé (premier salon d'une session) : sa marque dans le
/// fichier. Relue seulement si le fichier change (taille ou date).
fn version_du_fichier() -> Option<String> {
    type Cache = Option<((u64, Option<std::time::SystemTime>), Option<String>)>;
    static CACHE: Mutex<Cache> = Mutex::new(None);
    let chemin = crate::options::gamecore_dir()?.join(crate::noms::GAMECORE_PROXY);
    let meta = std::fs::metadata(&chemin).ok()?;
    let cle = (meta.len(), meta.modified().ok());
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, v)) = c.as_ref() {
        if *k == cle {
            return v.clone();
        }
    }
    // Une lecture ratée (fichier verrouillé un instant) n'est pas retenue : le salon suivant réessaie.
    let data = std::fs::read(&chemin).ok()?;
    let v = version_marquee(&data);
    *c = Some((cle, v.clone()));
    v
}

/// La première marque `BgdVersion=<version>\0` du fichier, version valide.
pub fn version_marquee(data: &[u8]) -> Option<String> {
    let marque = MARQUE.as_bytes();
    let mut debut = 0;
    while let Some(i) = data[debut..].windows(marque.len()).position(|w| w == marque) {
        let apres = &data[debut + i + marque.len()..];
        if let Some(fin) = apres.iter().take(64).position(|&b| b == 0) {
            if let Some(v) = version_valide(&apres[..fin]) {
                return Some(v);
            }
        }
        debut += i + 1;
    }
    None
}

/// Une version comme celles de version.rs : `0.8.0`, `1.0.0-rc.1`.
fn version_valide(octets: &[u8]) -> Option<String> {
    let ok = !octets.is_empty() && octets.len() <= 32 && octets.iter().all(|&b| b.is_ascii_alphanumeric() || b"._+-".contains(&b));
    ok.then(|| String::from_utf8_lossy(octets).into_owned())
}

/// StagingRoom.lua suivi du bloc du salon. Octets laissés tels quels, comme pour Options.lua.
///
/// Le bloc tourne dans sa propre fonction : ses locales ne comptent pas dans les registres de la
/// fonction principale du script, déjà près de la limite chez MPH (146 locales au premier niveau ;
/// HavokScript a refusé le script à 192 : « Function or expression requires too many registers »).
pub fn patch_staging_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n(function()\r\n");
    out.extend_from_slice(LUA_BLOCK.as_bytes());
    out.extend_from_slice(b"\r\nend)();\r\n");
    out
}

/// `StagingRoom.xml` avec le bouton du BGE Check après `BackButton` (même retrait) et le minuteur à
/// la fin du contexte.
pub fn patch_staging_xml(src: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(src).map_err(|_| "texte non UTF-8".to_string())?;
    if text.contains("ID=\"BGD_CheckButton\"") {
        return Err("bouton déjà présent".into());
    }
    let (debut, fin) = crate::rapport::element(text, ANCRE_XML).ok_or_else(|| "bouton BackButton introuvable".to_string())?;
    let ligne = text[..debut].rfind('\n').map_or(0, |i| i + 1);
    let retrait: String = text[ligne..debut].chars().take_while(|c| c.is_whitespace()).collect();
    let mut out = String::with_capacity(text.len() + BOUTON_CHECK.len() + MINUTEUR.len() + 128);
    out.push_str(&text[..fin]);
    out.push_str("\r\n");
    out.push_str(&retrait);
    out.push_str("<!-- BetterGameExperience : BGE Check, symétrique du MOD CHECK de MPH (startup_proxy/src/salon.rs) -->\r\n");
    out.push_str(&retrait);
    out.push_str(BOUTON_CHECK);
    out.push_str(&text[fin..]);
    let fin = crate::rapport::fin_du_contexte(&out)?;
    out.insert_str(fin, &format!("{MINUTEUR}\r\n"));
    Ok(out.into_bytes())
}

/// Jeton d'une ouverture du salon : l'heure en nanosecondes, l'identifiant du processus et un
/// compteur, en hexadécimal. Il suffit qu'il diffère d'une ouverture à l'autre et d'une machine à
/// l'autre ; ce n'est pas un secret.
pub fn new_token() -> String {
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}{:x}{n:x}", std::process::id())
}

/// `Civ6Proxy_Version()` → la version ; ou nil et la raison si cette machine n'appliquerait pas les
/// correctifs POL.
unsafe extern "C-unwind" fn lua_version(l: *mut std::ffi::c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    match pol_indisponible() {
        None => {
            push_str(h, l, VERSION);
            1
        }
        Some(raison) => {
            (h.push_nil)(l);
            push_str(h, l, &raison);
            2
        }
    }
}

/// `Civ6Proxy_VersionInstallee()` → la version de ce proxy, qu'il s'annonce ou non (BGE Check).
unsafe extern "C-unwind" fn lua_version_installee(l: *mut std::ffi::c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    push_str(h, l, VERSION);
    1
}

pub(crate) unsafe fn arg_str(l: *mut std::ffi::c_void, i: i32) -> Option<String> {
    let h = hks()?;
    let mut len = 0usize;
    let p = (h.to_lstring)(l, i, &mut len);
    (!p.is_null()).then(|| String::from_utf8_lossy(std::slice::from_raw_parts(p as *const u8, len)).into_owned())
}

/// `Civ6Proxy_FixerPol(actif, raison)`, actif 1 ou 0 : l'état lu par le proxy de GameCore. Journalisé
/// quand il change.
unsafe extern "C-unwind" fn lua_fix_pol(l: *mut std::ffi::c_void) -> i32 {
    let actif = arg_str(l, 1).is_some_and(|v| v.trim() == "1");
    if POL.swap(actif, Ordering::Relaxed) != actif {
        let raison = arg_str(l, 2).unwrap_or_default();
        crate::log(&format!("pol : correctifs POL {} ({raison})", if actif { "actifs" } else { "désactivés" }));
    }
    0
}

/// `Civ6Proxy_PolActifs()` → 1 si les correctifs POL sont actifs sur cette machine, sinon 0.
unsafe extern "C-unwind" fn lua_pol_state(l: *mut std::ffi::c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    (h.push_integer)(l, pol_actifs() as i32);
    1
}

/// `Civ6Proxy_GardeMessages()` → le nombre de lectures que la garde des messages réseau a corrigées
/// depuis le dernier appel (messages.rs), remis à 0.
unsafe extern "C-unwind" fn lua_garde(l: *mut std::ffi::c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    (h.push_integer)(l, crate::messages::a_annoncer().min(i32::MAX as u32) as i32);
    1
}

unsafe extern "C-unwind" fn lua_token(l: *mut std::ffi::c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    push_str(h, l, &new_token());
    1
}

pub(crate) unsafe extern "C-unwind" fn lua_log(l: *mut std::ffi::c_void) -> i32 {
    if let Some(t) = arg_str(l, 1) {
        crate::log(&t);
    }
    0
}

pub const NATIVES: Natives = &[
    (lua_version, b"Civ6Proxy_Version\0"),
    (lua_token, b"Civ6Proxy_Jeton\0"),
    (lua_log, b"Civ6Proxy_Journal\0"),
    (lua_fix_pol, b"Civ6Proxy_FixerPol\0"),
    (lua_version_installee, b"Civ6Proxy_VersionInstallee\0"),
];

const CHARGEMENT_NATIVES: Natives = &[(lua_log, b"Civ6Proxy_Journal\0"), (lua_fix_pol, b"Civ6Proxy_FixerPol\0")];
const PARTIE_NATIVES: Natives = &[
    (lua_log, b"Civ6Proxy_Journal\0"),
    (lua_pol_state, b"Civ6Proxy_PolActifs\0"),
    (lua_garde, b"Civ6Proxy_GardeMessages\0"),
    (crate::relink::lua_pairs, b"Civ6Proxy_RelayedPairs\0"),
    (crate::relink::lua_start, b"Civ6Proxy_RelinkStart\0"),
    (crate::relink::lua_state, b"Civ6Proxy_RelinkState\0"),
    (crate::relink::lua_forget, b"Civ6Proxy_RelayForgetPair\0"),
];
const OPTION_NATIVES: Natives = &[(lua_log, b"Civ6Proxy_Journal\0")];

/// Appelée par l'accroche sur StagingRoom.lua : pose les natives, puis rend le texte à compiler.
pub unsafe fn prepare(l: *mut std::ffi::c_void, src: &[u8]) -> Vec<u8> {
    publish_natives(l, NATIVES, "salon");
    patch_staging_lua(src)
}

/// LoadScreen.lua suivi de `chargement.lua`.
pub fn patch_load_screen_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(CHARGEMENT_BLOCK.as_bytes());
    out
}

/// Appelée par l'accroche sur LoadScreen.lua.
pub unsafe fn prepare_chargement(l: *mut std::ffi::c_void, src: &[u8]) -> Vec<u8> {
    publish_natives(l, CHARGEMENT_NATIVES, "chargement");
    patch_load_screen_lua(src)
}

/// InGame.lua suivi de `partie.lua`.
pub fn patch_in_game_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(PARTIE_BLOCK.as_bytes());
    out
}

/// Appelée par l'accroche sur InGame.lua.
pub unsafe fn prepare_partie(l: *mut std::ffi::c_void, src: &[u8]) -> Vec<u8> {
    publish_natives(l, PARTIE_NATIVES, "partie");
    patch_in_game_lua(src)
}

/// SetupParameters.lua suivi de `option_partie.lua`.
pub fn patch_setup_parameters_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(OPTION_BLOCK.as_bytes());
    out
}

/// Appelée par l'accroche sur SetupParameters.lua, inclus par chaque écran de réglage de partie.
pub unsafe fn prepare_option(l: *mut std::ffi::c_void, src: &[u8]) -> Vec<u8> {
    publish_natives(l, OPTION_NATIVES, "option");
    patch_setup_parameters_lua(src)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jetons_distincts() {
        let (a, b) = (new_token(), new_token());
        assert_ne!(a, b);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()), "{a}");
    }

    #[test]
    fn bloc_ajoute() {
        let out = patch_staging_lua(b"Initialize();");
        assert!(out.starts_with(b"Initialize();\r\n(function()\r\n-- "));
        assert!(out.ends_with(b"\r\nend)();\r\n"), "bloc dans sa propre fonction");
        assert!(LUA_BLOCK.contains("BGE - "));
        for &(_, name) in NATIVES {
            let name = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
            assert!(LUA_BLOCK.contains(name), "{name} posée mais pas utilisée par le bloc");
        }
    }

    #[test]
    fn bouton_du_check() {
        let src = "<Context>\r\n  <Container ID=\"TopPanel\">\r\n    <Grid>\r\n      <GridButton ID=\"BackButton\" Anchor=\"R,C\" Offset=\"1,0\"/>\r\n      <Stack ID=\"LeftTopButtonStack\"/>\r\n    </Grid>\r\n  </Container>\r\n</Context>\r\n";
        let out = String::from_utf8(patch_staging_xml(src.as_bytes()).unwrap()).unwrap();
        let (back, bouton, pile) = (out.find("BackButton").unwrap(), out.find(BOUTON_CHECK).unwrap(), out.find("LeftTopButtonStack").unwrap());
        assert!(back < bouton && bouton < pile, "{out}");
        assert!(out.contains(&format!("\r\n      {BOUTON_CHECK}")), "même retrait que BackButton : {out}");
        let (minuteur, fin) = (out.find("BGD_SalonMinuteur").unwrap(), out.rfind("</Context>").unwrap());
        assert!(pile < minuteur && minuteur < fin);
        assert!(BOUTON_CHECK.contains(r#"Anchor="L,C" Offset="180,0" Size="150,38""#), "symétrique du MOD CHECK de MPH");
        assert!(patch_staging_xml(out.as_bytes()).is_err(), "pas deux fois");
        assert!(patch_staging_xml(b"<Context></Context>").is_err(), "sans BackButton");
        assert!(LUA_BLOCK.contains("BGD_CheckButton") && LUA_BLOCK.contains("BGD_SalonMinuteur"));
    }

    #[test]
    fn version_du_proxy_de_gamecore() {
        assert_eq!(raison_version(Some(VERSION)), None);
        let r = raison_version(Some("0.0.1")).unwrap();
        assert!(r.contains(&format!("winmm.dll {VERSION}")) && r.contains("BGD_GameCore_XP2.dll 0.0.1"), "{r}");
        let r = raison_version(None).unwrap();
        assert!(r.contains(&format!("winmm.dll {VERSION}")) && r.contains("sans version"), "{r}");
        // La marque dans le fichier, telle que la grave proxy/src/lib.rs.
        let mut dll = b"MZ\0\0BgdVersion\0BgdVersion=\x01\0".to_vec();
        dll.extend_from_slice(format!("xx{MARQUE}0.9.1-rc.2\0yy").as_bytes());
        assert_eq!(version_marquee(&dll).as_deref(), Some("0.9.1-rc.2"));
        assert_eq!(version_marquee(b"MZ RolloverProxyStatus BgdVersion\0"), None, "proxy plus ancien : pas de marque");
        assert_eq!(version_marquee(format!("{MARQUE}0.8.0").as_bytes()), None, "sans NUL final");
        assert_eq!(version_valide(b"0.8.0"), Some("0.8.0".into()));
        assert_eq!(version_valide(b""), None);
        assert_eq!(version_valide(b"0.8 0"), None);
    }

    #[test]
    fn interrupteur_general() {
        assert!(actif_off("mode=coalesce\r\nactif=off\r\n"));
        assert!(!actif_off("actif=off\nactif=on\n"), "la dernière occurrence compte");
        assert!(!actif_off("mode=coalesce\n"), "absent : actif");
    }

    #[test]
    fn bloc_de_chargement() {
        assert!(patch_load_screen_lua(b"Initialize();").starts_with(b"Initialize();\r\n-- "));
        assert!(patch_in_game_lua(b"Initialize();").starts_with(b"Initialize();\r\n-- "));
        for &(_, name) in PARTIE_NATIVES {
            let name = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
            assert!(PARTIE_BLOCK.contains(name), "{name} posée mais pas utilisée");
        }
        assert!(patch_setup_parameters_lua(b"x = 1").starts_with(b"x = 1\r\n-- "));
        for &(_, name) in OPTION_NATIVES {
            let name = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
            assert!(OPTION_BLOCK.contains(name), "{name} posée mais pas utilisée");
        }
        for &(_, name) in CHARGEMENT_NATIVES {
            let name = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
            assert!(CHARGEMENT_BLOCK.contains(name), "{name} posée mais pas utilisée");
        }
    }

    #[test]
    fn version_lisible() {
        // Annonce « <version>@<jeton>#<jeton> », verdict « <version>@<jeton>:<1|0>:<jetons> » : la
        // version ne doit contenir aucun de ces séparateurs.
        assert!(!VERSION.is_empty() && !VERSION.contains(['@', '#', ':', ',']), "{VERSION}");
    }
}
