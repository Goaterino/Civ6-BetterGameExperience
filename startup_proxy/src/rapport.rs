//! Signalement d'un bug depuis la partie (voir `rapport.lua` et `rapport_worker/README.md`).
//!
//! Un bouton « Signaler un bug (BGE) » est ajouté au menu Échap, juste sous « Options » :
//! - **XML.** Après la lecture d'`InGameTopOptionsMenu.xml`, l'élément du bouton `OptionsButton` est
//!   recopié juste après lui, sous l'ID `BGD_RapportButton`, sans son texte : même style, même taille,
//!   dans la même pile. Le menu reçoit aussi l'instance `PopupDialog` s'il ne l'a pas.
//! - **Lua.** `rapport.lua` est ajouté à la fin d'`InGameTopOptionsMenu.lua` (que MPH inclut depuis
//!   son remplaçant), après avoir posé trois fonctions natives : `Civ6Proxy_RapportEnvoyer(meta)`,
//!   `Civ6Proxy_RapportEtat()` et `Civ6Proxy_Journal(texte)`.
//!
//! L'envoi tourne dans un thread, pour ne pas figer le jeu : il attend la sauvegarde demandée par le
//! bloc (`BGE_rapport.Civ6Save`), rassemble les journaux, les derniers autosaves et les réglages,
//! les compresse en un zip, puis l'envoie au relais (`RELAIS`) par le `curl.exe` de Windows. Le
//! relais (Cloudflare Worker) garde le zip et ouvre un fil dans le forum Discord des rapports ; il
//! rend le numéro du rapport. Sans relais, ou s'il échoue, le zip est posé sur le Bureau.
//!
//! Le zip :
//! - `meta.json` : version de BGE, exe (DX11 ou DX12), ce que dit le bloc Lua (tour, joueur, civ,
//!   multijoueur…), la liste des fichiers ;
//! - `bge/RolloverProxy.log`, `bge/rollover_proxy.cfg` ;
//! - `logs/…` : le dossier `Logs` du jeu, `AppOptions.txt` et `Mods.sqlite` ;
//! - `saves/…` : la sauvegarde du rapport et les 3 derniers autosaves.
//!
//! Les fichiers texte de plus de 5 Mo sont coupés à leurs 5 derniers Mo, et le nom d'utilisateur
//! Windows est retiré des chemins (`C:\Users\<utilisateur>`).

use crate::options::{hks, publish_natives, push_str, Natives};
use crate::salon::lua_log;
use crate::log;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const LUA_BLOCK: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("rapport.lua"));
pub const SCRIPT: &str = "ingametopoptionsmenu.lua";
pub const XML: &str = "InGameTopOptionsMenu.xml";

/// URL du relais, gravée à la compilation (`BGE_RAPPORT_URL`, voir `rapport_worker/README.md`) ;
/// `rapport_url=` dans `rollover_proxy.cfg` la remplace, pour essayer un autre relais.
const RELAIS: Option<&str> = option_env!("BGE_RAPPORT_URL");

/// Bouton recopié, et ID de la copie.
const ANCRE: &str = r#"ID="OptionsButton""#;
const ID_BOUTON: &str = "BGD_RapportButton";

/// Nom de la sauvegarde demandée par le bloc Lua (`Network.SaveGame`).
const SAUVEGARDE: &str = "BGE_rapport";
/// Fichiers texte coupés au-delà, et fichiers binaires laissés de côté au-delà.
const MAX_TEXTE: usize = 5 << 20;
const MAX_BINAIRE: u64 = 16 << 20;
/// Taille au-delà de laquelle on retire des autosaves (le relais refuse au-delà de 50 Mo).
const MAX_ZIP: usize = 40 << 20;
const AUTOSAVES: usize = 3;
/// Rapports gardés dans `%LOCALAPPDATA%\BetterGameExperience\rapports`.
const GARDES: usize = 5;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// ---------------------------------------------------------------------------------------------
// État partagé avec le bloc Lua.

#[derive(Clone, Debug, PartialEq)]
pub struct Resultat {
    pub ok: bool,
    /// Numéro du rapport, ou raison de l'échec.
    pub texte: String,
    /// Le zip gardé sur la machine.
    pub archive: String,
}

enum Etat {
    Libre,
    EnCours,
    Fini(Resultat),
}

static ETAT: Mutex<Etat> = Mutex::new(Etat::Libre);

// ---------------------------------------------------------------------------------------------
// Textes, sans état : testés hors du jeu.

/// `InGameTopOptionsMenu.xml` avec la copie du bouton Options juste après lui, puis l'instance du
/// popup si le menu ne l'a pas.
pub fn patch_menu_xml(src: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(src).map_err(|_| "texte non UTF-8".to_string())?;
    if text.contains(&format!("ID=\"{ID_BOUTON}\"")) {
        return Err("bouton déjà présent".into());
    }
    let (debut, fin) = element(text, ANCRE).ok_or_else(|| format!("bouton OptionsButton introuvable (boutons : {})", ids(text)))?;
    let copie = copier_bouton(&text[debut..fin]);
    // Même retrait que le bouton recopié.
    let ligne = text[..debut].rfind('\n').map_or(0, |i| i + 1);
    let retrait: String = text[ligne..debut].chars().take_while(|c| c.is_whitespace()).collect();
    let mut out = String::with_capacity(text.len() + copie.len() + 64);
    out.push_str(&text[..fin]);
    out.push_str("\r\n");
    out.push_str(&retrait);
    out.push_str("<!-- BetterGameExperience : signalement d'un bug (startup_proxy/src/rapport.rs) -->\r\n");
    out.push_str(&retrait);
    out.push_str(&copie);
    out.push_str(&text[fin..]);
    ajouter_popup(&mut out)?;
    Ok(out.into_bytes())
}

/// L'instance `PopupDialog` à la fin du contexte (et son modèle), si le menu ne l'a pas : le bloc
/// Lua crée son popup sur ces contrôles, comme `maj` pour le menu principal de MPH.
fn ajouter_popup(text: &mut String) -> Result<(), String> {
    if text.contains(r#"<MakeInstance Name="PopupDialog""#) {
        return Ok(());
    }
    let fin = ["</ContextDefaults>", "</Context>"].iter().filter_map(|f| text.rfind(f)).max().ok_or("fin du contexte introuvable")?;
    let mut ajout = String::new();
    if !text.contains(r#"<Include File="PopupDialog""#) {
        ajout.push_str("  <Include File=\"PopupDialog\" />\r\n");
    }
    ajout.push_str("  <MakeInstance Name=\"PopupDialog\" />\r\n");
    text.insert_str(fin, &ajout);
    Ok(())
}

/// Début et fin de l'élément qui porte `ancre` dans sa balise ouvrante, enfants compris.
fn element(text: &str, ancre: &str) -> Option<(usize, usize)> {
    let at = text.find(ancre)?;
    let debut = text[..at].rfind('<')?;
    let nom: String = text[debut + 1..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    if nom.is_empty() {
        return None;
    }
    let fin_balise = text[at..].find('>')? + at + 1;
    if text[..fin_balise].ends_with("/>") {
        return Some((debut, fin_balise));
    }
    // Enfants : on cherche la fermeture du même niveau.
    let (ouvre, ferme) = (format!("<{nom}"), format!("</{nom}>"));
    let mut profondeur = 1;
    let mut i = fin_balise;
    while profondeur > 0 {
        let o = text[i..].find(&ouvre).map(|p| p + i);
        let f = text[i..].find(&ferme).map(|p| p + i)?;
        match o {
            Some(o) if o < f => {
                let fin_o = text[o..].find('>')? + o + 1;
                if !text[..fin_o].ends_with("/>") {
                    profondeur += 1;
                }
                i = fin_o;
            }
            _ => {
                profondeur -= 1;
                i = f + ferme.len();
            }
        }
    }
    Some((debut, i))
}

/// Le bouton Options recopié : ID changé, sans texte ni infobulle (le bloc Lua pose les siens), et
/// les ID de ses enfants préfixés, pour qu'aucun ne double un contrôle du menu.
fn copier_bouton(element: &str) -> String {
    let mut out = element.replacen(ANCRE, &format!("ID=\"{ID_BOUTON}\""), 1);
    for attr in ["String", "ToolTip"] {
        let motif = format!(" {attr}=\"");
        if let Some(a) = out.find(&motif) {
            if let Some(f) = out[a + motif.len()..].find('"') {
                out.replace_range(a..a + motif.len() + f + 1, "");
            }
        }
    }
    // Enfants : `ID="X"` après le premier devient `ID="BGD_Rapport_X"`.
    let tete = out.find(&format!("ID=\"{ID_BOUTON}\"")).map_or(0, |p| p + ID_BOUTON.len() + 5);
    let (a, b) = out.split_at(tete);
    format!("{a}{}", b.replace("ID=\"", "ID=\"BGD_Rapport_"))
}

/// Les ID des boutons du fichier, pour le journal quand l'ancre manque (autre version du menu).
fn ids(text: &str) -> String {
    let v: Vec<&str> = text.split("ID=\"").skip(1).filter_map(|s| s.split('"').next()).filter(|s| s.ends_with("Button")).collect();
    if v.is_empty() {
        "aucun".into()
    } else {
        v.join(", ")
    }
}

/// `InGameTopOptionsMenu.lua` suivi du bloc du rapport.
pub fn patch_menu_lua(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(LUA_BLOCK.as_bytes());
    out
}

/// Les lignes `clé=valeur` du bloc Lua, dans l'ordre ; les lignes sans `=` sont ignorées.
pub fn lire_meta(lignes: &str) -> Vec<(String, String)> {
    lignes
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `meta.json` : les champs du bloc Lua, puis ceux du proxy, puis la liste des fichiers.
pub fn meta_json(lua: &[(String, String)], proxy: &[(&str, String)], fichiers: &[(String, usize)]) -> String {
    let mut champs: Vec<String> = Vec::new();
    for (k, v) in lua {
        champs.push(format!("  {}: {}", json_str(k), json_str(v)));
    }
    for (k, v) in proxy {
        champs.push(format!("  {}: {}", json_str(k), json_str(v)));
    }
    let liste: Vec<String> = fichiers.iter().map(|(n, t)| format!("    {{\"nom\": {}, \"octets\": {t}}}", json_str(n))).collect();
    champs.push(format!("  \"fichiers\": [\n{}\n  ]", liste.join(",\n")));
    format!("{{\n{}\n}}\n", champs.join(",\n"))
}

/// Fin d'un fichier texte trop long : ses `max` derniers octets, à partir d'un début de ligne.
pub fn couper(data: Vec<u8>, max: usize) -> Vec<u8> {
    if data.len() <= max {
        return data;
    }
    let mut debut = data.len() - max;
    if let Some(nl) = data[debut..].iter().position(|&b| b == b'\n') {
        debut += nl + 1;
    }
    let mut out = format!("[BGE : {} premiers octets retirés]\r\n", debut).into_bytes();
    out.extend_from_slice(&data[debut..]);
    out
}

/// Le nom d'utilisateur retiré des chemins : `\Users\<nom>` et `/Users/<nom>`, casse ignorée.
pub fn anonymiser(data: &[u8], utilisateur: &str) -> Vec<u8> {
    if utilisateur.is_empty() {
        return data.to_vec();
    }
    let mut out = data.to_vec();
    for sep in ['\\', '/'] {
        let motif = format!("{sep}users{sep}{utilisateur}").into_bytes();
        let remplace = format!("{sep}Users{sep}<utilisateur>").into_bytes();
        let mut res = Vec::with_capacity(out.len());
        let mut i = 0;
        while i < out.len() {
            let suit = i + motif.len() <= out.len() && out[i..i + motif.len()].eq_ignore_ascii_case(&motif);
            // Le nom doit finir là : `\Users\Jean` ne doit pas prendre `\Users\Jeanne`.
            let fin_nom = out.get(i + motif.len()).is_none_or(|&b| !(b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.' || b >= 0x80));
            if suit && fin_nom {
                res.extend_from_slice(&remplace);
                i += motif.len();
            } else {
                res.push(out[i]);
                i += 1;
            }
        }
        out = res;
    }
    out
}

/// Numéro de rapport rendu par le relais : `{"id":"BGE-7F3A", …}`.
pub fn lire_id(reponse: &str) -> Option<String> {
    let at = reponse.find("\"id\"")? + 4;
    let reste = reponse[at..].trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
    let id = &reste[..reste.find('"')?];
    (!id.is_empty() && id.len() <= 32 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')).then(|| id.to_string())
}

/// `rapport_url=` du fichier de réglages, dernière occurrence.
pub fn url_du_fichier(cfg: &str) -> Option<String> {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("rapport_url=")).last().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

// ---------------------------------------------------------------------------------------------
// Zip (méthode deflate, sans zip64 : quelques dizaines de Mo au plus).

fn crc32(data: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let t = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, e) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *e = c;
        }
        t
    });
    !data.iter().fold(!0u32, |c, &b| t[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8))
}

/// Date et heure MS-DOS (UTC) d'un instant Unix.
fn dos_time(secs: u64) -> (u16, u16) {
    let (jours, s) = (secs / 86400, secs % 86400);
    // Date civile depuis le nombre de jours (algorithme de H. Hinnant).
    let z = jours as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let jour = doy - (153 * mp + 2) / 5 + 1;
    let mois = if mp < 10 { mp + 3 } else { mp - 9 };
    let annee = yoe + era * 400 + (mois <= 2) as i64;
    let date = (((annee - 1980).clamp(0, 127) as u16) << 9) | ((mois as u16) << 5) | jour as u16;
    let heure = (((s / 3600) as u16) << 11) | ((((s / 60) % 60) as u16) << 5) | ((s % 60) / 2) as u16;
    (heure, date)
}

pub fn zip(entrees: &[(String, Vec<u8>)], secs: u64) -> Vec<u8> {
    let (heure, date) = dos_time(secs);
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (nom, data) in entrees {
        let packed = miniz_oxide::deflate::compress_to_vec(data, 6);
        let (methode, corps): (u16, &[u8]) = if packed.len() < data.len() { (8, &packed) } else { (0, data) };
        let crc = crc32(data);
        let offset = out.len() as u32;
        let nom = nom.as_bytes();
        let tete = |sig: u32, central: bool| {
            let mut h = Vec::with_capacity(46 + nom.len());
            h.extend_from_slice(&sig.to_le_bytes());
            if central {
                h.extend_from_slice(&20u16.to_le_bytes()); // créé par : 2.0, MS-DOS
            }
            h.extend_from_slice(&20u16.to_le_bytes()); // version requise
            h.extend_from_slice(&0x0800u16.to_le_bytes()); // noms en UTF-8
            h.extend_from_slice(&methode.to_le_bytes());
            h.extend_from_slice(&heure.to_le_bytes());
            h.extend_from_slice(&date.to_le_bytes());
            h.extend_from_slice(&crc.to_le_bytes());
            h.extend_from_slice(&(corps.len() as u32).to_le_bytes());
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&(nom.len() as u16).to_le_bytes());
            h.extend_from_slice(&0u16.to_le_bytes()); // extra
            if central {
                h.extend_from_slice(&0u16.to_le_bytes()); // commentaire
                h.extend_from_slice(&0u16.to_le_bytes()); // disque
                h.extend_from_slice(&0u16.to_le_bytes()); // attributs internes
                h.extend_from_slice(&0u32.to_le_bytes()); // attributs externes
                h.extend_from_slice(&offset.to_le_bytes());
            }
            h.extend_from_slice(nom);
            h
        };
        let locale = tete(0x0403_4b50, false);
        let c = tete(0x0201_4b50, true);
        out.extend_from_slice(&locale);
        out.extend_from_slice(corps);
        central.extend_from_slice(&c);
    }
    let debut_central = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]); // disques
    out.extend_from_slice(&(entrees.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entrees.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&debut_central.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

// ---------------------------------------------------------------------------------------------
// Dossiers de Windows.

#[repr(C)]
struct Guid(u32, u16, u16, [u8; 8]);

const FOLDERID_DOCUMENTS: Guid = Guid(0xFDD3_9AD0, 0x238F, 0x46AF, [0xAD, 0xB4, 0x6C, 0x85, 0x48, 0x03, 0x69, 0xC7]);
const FOLDERID_DESKTOP: Guid = Guid(0xB4BF_CC3A, 0xDB2C, 0x424C, [0xB0, 0x29, 0x7F, 0xE9, 0x9A, 0x87, 0xC6, 0x41]);

/// Dossier connu de Windows (Documents, Bureau), redirections OneDrive comprises. shell32 et ole32
/// sont chargées à la demande : ce proxy est chargé très tôt, on n'alourdit pas ses imports.
fn dossier_connu(id: &Guid) -> Option<PathBuf> {
    use crate::win::{wide, GetProcAddress, LoadLibraryW};
    type GetPath = unsafe extern "system" fn(*const Guid, u32, *mut std::ffi::c_void, *mut *mut u16) -> i32;
    type Free = unsafe extern "system" fn(*mut std::ffi::c_void);
    unsafe {
        let shell = LoadLibraryW(wide("shell32.dll").as_ptr());
        let ole = LoadLibraryW(wide("ole32.dll").as_ptr());
        if shell.is_null() || ole.is_null() {
            return None;
        }
        let get = GetProcAddress(shell, b"SHGetKnownFolderPath\0".as_ptr());
        let free = GetProcAddress(ole, b"CoTaskMemFree\0".as_ptr());
        if get.is_null() || free.is_null() {
            return None;
        }
        let (get, free): (GetPath, Free) = (std::mem::transmute(get), std::mem::transmute(free));
        let mut p: *mut u16 = std::ptr::null_mut();
        let r = get(id, 0, std::ptr::null_mut(), &mut p);
        if r != 0 || p.is_null() {
            return None;
        }
        let n = (0..).take_while(|&i| *p.add(i) != 0).count();
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
        free(p as *mut _);
        Some(PathBuf::from(s))
    }
}

fn local_app_data() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

fn dossier_civ_local() -> Option<PathBuf> {
    Some(local_app_data()?.join("Firaxis Games").join("Sid Meier's Civilization VI"))
}

fn dossier_saves() -> Option<PathBuf> {
    let docs = dossier_connu(&FOLDERID_DOCUMENTS).or_else(|| std::env::var_os("USERPROFILE").map(|u| PathBuf::from(u).join("Documents")))?;
    Some(docs.join("My Games").join("Sid Meier's Civilization VI").join("Saves"))
}

fn dossier_rapports() -> Option<PathBuf> {
    Some(local_app_data()?.join("BetterGameExperience").join("rapports"))
}

// ---------------------------------------------------------------------------------------------
// Collecte et envoi.

fn modifie(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

/// Les `.Civ6Save` d'un dossier, du plus récent au plus ancien.
fn saves_de(dir: &Path) -> Vec<(SystemTime, PathBuf)> {
    let mut v: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("civ6save")))
        .filter_map(|p| Some((modifie(&p)?, p)))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v
}

/// La sauvegarde du rapport, écrite après `depart` : le jeu l'écrit sans attendre, on attend qu'elle
/// existe puis que sa taille ne bouge plus (20 s au plus).
fn attendre_sauvegarde(saves: &Path, depart: SystemTime) -> Option<PathBuf> {
    let seuil = depart - Duration::from_secs(2);
    let mut taille = None;
    for _ in 0..40 {
        let trouvee = ["Multi", "Single", "Hotseat"]
            .iter()
            .flat_map(|m| saves_de(&saves.join(m)))
            .filter(|(t, p)| *t >= seuil && p.file_stem().is_some_and(|s| s.to_string_lossy().starts_with(SAUVEGARDE)))
            .max_by_key(|(t, _)| *t)
            .map(|(_, p)| p);
        if let Some(p) = trouvee {
            let t = std::fs::metadata(&p).map(|m| m.len()).ok();
            if t.is_some() && t == taille {
                return Some(p);
            }
            taille = t;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    None
}

fn est_texte(p: &Path) -> bool {
    p.extension().is_some_and(|e| ["log", "txt", "csv", "cfg", "json", "ini", "xml"].iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// Un fichier pour le zip : texte coupé et anonymisé, binaire laissé de côté s'il est trop gros.
fn lire_fichier(p: &Path, utilisateur: &str) -> Option<Vec<u8>> {
    if est_texte(p) {
        let data = std::fs::read(p).ok()?;
        Some(anonymiser(&couper(data, MAX_TEXTE), utilisateur))
    } else {
        if std::fs::metadata(p).ok()?.len() > MAX_BINAIRE {
            log(&format!("rapport : {} laissé de côté (trop gros)", p.display()));
            return None;
        }
        std::fs::read(p).ok()
    }
}

fn nom_fichier(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

struct Collecte {
    base: Vec<(String, Vec<u8>)>,
    /// La sauvegarde du rapport puis les autosaves, du plus récent au plus ancien.
    saves: Vec<(String, Vec<u8>)>,
}

fn collecter(lua: &[(String, String)], depart: SystemTime) -> Collecte {
    let utilisateur = std::env::var("USERNAME").unwrap_or_default();
    let mut base = Vec::new();
    let mut ajouter = |nom: String, p: &Path| {
        if let Some(d) = lire_fichier(p, &utilisateur) {
            base.push((nom, d));
        }
    };
    if let Some(dir) = local_app_data() {
        ajouter("bge/RolloverProxy.log".into(), &dir.join("Civ6RolloverProxy").join("RolloverProxy.log"));
    }
    if let Some(dir) = crate::options::gamecore_dir() {
        ajouter(format!("bge/{}", crate::options::CFG_NAME), &dir.join(crate::options::CFG_NAME));
    }
    if let Some(civ) = dossier_civ_local() {
        for e in std::fs::read_dir(civ.join("Logs")).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_file() {
                ajouter(format!("logs/{}", nom_fichier(&p)), &p);
            }
        }
        for f in ["AppOptions.txt", "Mods.sqlite"] {
            ajouter(format!("logs/{f}"), &civ.join(f));
        }
    }

    let mut saves = Vec::new();
    if let Some(dir) = dossier_saves() {
        match attendre_sauvegarde(&dir, depart) {
            Some(p) => {
                if let Ok(d) = std::fs::read(&p) {
                    saves.push((format!("saves/{}", nom_fichier(&p)), d));
                }
            }
            None => log("rapport : sauvegarde du rapport introuvable, autosaves seuls"),
        }
        let multi = lua.iter().any(|(k, v)| k == "multijoueur" && v == "1");
        let dossiers: &[&str] = if multi { &["Multi"] } else { &["Single", "Hotseat"] };
        let mut auto: Vec<(SystemTime, PathBuf)> = dossiers.iter().flat_map(|m| saves_de(&dir.join(m).join("auto"))).collect();
        auto.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, p) in auto.into_iter().take(AUTOSAVES) {
            if let Ok(d) = std::fs::read(&p) {
                saves.push((format!("saves/auto/{}", nom_fichier(&p)), d));
            }
        }
    }
    Collecte { base, saves }
}

fn infos_proxy() -> Vec<(&'static str, String)> {
    let exe = std::env::current_exe().ok().map(|p| nom_fichier(&p)).unwrap_or_default();
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    vec![
        ("bge_version", crate::version::VERSION.to_string()),
        ("bge_release", crate::version::RELEASE.unwrap_or("locale").to_string()),
        ("exe", exe),
        ("date_unix", secs.to_string()),
    ]
}

/// Le zip du rapport : autosaves retirés, du plus ancien au plus récent, tant qu'il dépasse `MAX_ZIP`.
fn construire_zip(lua: &[(String, String)], collecte: Collecte) -> Vec<u8> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mut saves = collecte.saves;
    loop {
        let mut entrees: Vec<(String, Vec<u8>)> = collecte.base.iter().chain(saves.iter()).cloned().collect();
        let liste: Vec<(String, usize)> = entrees.iter().map(|(n, d)| (n.clone(), d.len())).collect();
        entrees.insert(0, ("meta.json".into(), meta_json(lua, &infos_proxy(), &liste).into_bytes()));
        let z = zip(&entrees, secs);
        if z.len() <= MAX_ZIP || saves.is_empty() {
            return z;
        }
        let (nom, _) = saves.pop().unwrap();
        log(&format!("rapport : {} retiré, zip trop gros ({} octets)", nom, z.len()));
    }
}

/// Garde les `GARDES` rapports les plus récents du dossier.
fn elaguer(dir: &Path) {
    let mut v: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")))
        .filter_map(|p| Some((modifie(&p)?, p)))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, p) in v.into_iter().skip(GARDES) {
        let _ = std::fs::remove_file(p);
    }
}

fn url_relais() -> Option<String> {
    let cfg = crate::options::read_config().map(|(t, _)| t).unwrap_or_default();
    url_du_fichier(&cfg).or_else(|| RELAIS.map(str::to_string)).filter(|u| u.starts_with("https://"))
}

/// Envoi par `curl.exe`, lancé dans le dossier du zip : les noms passés à `-F` restent simples.
fn envoyer_zip(url: &str, dir: &Path, nom: &str) -> Result<String, String> {
    let root = std::env::var_os("SystemRoot").unwrap_or("C:\\Windows".into());
    let out = Command::new(PathBuf::from(root).join("System32\\curl.exe"))
        .current_dir(dir)
        .args(["-sS", "--max-time", "300", "-w", "\n%{http_code}", "-H"])
        .arg(format!("X-BGE-Version: {}", crate::version::RELEASE.unwrap_or(crate::version::VERSION)))
        .arg("-F")
        .arg(format!("archive=@{nom};type=application/zip"))
        .arg(url)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("curl.exe : {e}"))?;
    let texte = String::from_utf8_lossy(&out.stdout).into_owned();
    let (corps, code) = texte.rsplit_once('\n').unwrap_or(("", texte.as_str()));
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("curl.exe : {}", err.trim()));
    }
    if code.trim() != "200" {
        let corps: String = corps.chars().take(200).collect();
        return Err(format!("le relais a répondu {} ({})", code.trim(), corps.trim()));
    }
    lire_id(corps).ok_or_else(|| "réponse du relais sans numéro de rapport".to_string())
}

fn executer(lua: Vec<(String, String)>, depart: SystemTime) -> Resultat {
    let collecte = collecter(&lua, depart);
    let z = construire_zip(&lua, collecte);
    let secs = depart.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let nom = format!("BGE_rapport_{secs}.zip");
    let Some(dir) = dossier_rapports() else {
        return Resultat { ok: false, texte: "dossier LOCALAPPDATA introuvable".into(), archive: String::new() };
    };
    let chemin = dir.join(&nom);
    if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&chemin, &z)) {
        return Resultat { ok: false, texte: format!("écriture de {} impossible : {e}", chemin.display()), archive: String::new() };
    }
    elaguer(&dir);
    log(&format!("rapport : {} écrit ({} octets)", chemin.display(), z.len()));
    let envoi = match url_relais() {
        Some(url) => envoyer_zip(&url, &dir, &nom),
        None => Err("relais non configuré".into()),
    };
    match envoi {
        Ok(id) => {
            log(&format!("rapport : envoyé, numéro {id}"));
            Resultat { ok: true, texte: id, archive: chemin.display().to_string() }
        }
        Err(e) => {
            log(&format!("rapport : envoi impossible : {e}"));
            // Sur le Bureau : le joueur le trouve sans chercher, pour le poster lui-même.
            let archive = dossier_connu(&FOLDERID_DESKTOP)
                .map(|d| d.join(&nom))
                .filter(|p| std::fs::copy(&chemin, p).is_ok())
                .unwrap_or(chemin);
            Resultat { ok: false, texte: e, archive: archive.display().to_string() }
        }
    }
}

/// Lance l'envoi dans un thread ; faux s'il y en a déjà un.
fn lancer(lua: Vec<(String, String)>) -> bool {
    {
        let mut e = ETAT.lock().unwrap();
        if matches!(*e, Etat::EnCours) {
            return false;
        }
        *e = Etat::EnCours;
    }
    let depart = SystemTime::now();
    log(&format!("rapport : envoi lancé ({})", lua.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", ")));
    std::thread::spawn(move || {
        let r = executer(lua, depart);
        *ETAT.lock().unwrap() = Etat::Fini(r);
    });
    true
}

// ---------------------------------------------------------------------------------------------
// Fonctions natives.

/// `Civ6Proxy_RapportEnvoyer(lignes)`, lignes `clé=valeur` → 1 ; ou nil et la raison.
unsafe extern "C-unwind" fn lua_envoyer(l: *mut std::ffi::c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let mut len = 0usize;
    let p = (h.to_lstring)(l, 1, &mut len);
    let lignes = if p.is_null() { String::new() } else { String::from_utf8_lossy(std::slice::from_raw_parts(p as *const u8, len)).into_owned() };
    if lancer(lire_meta(&lignes)) {
        (h.push_integer)(l, 1);
        1
    } else {
        (h.push_nil)(l);
        push_str(h, l, "un envoi est déjà en cours");
        2
    }
}

/// `Civ6Proxy_RapportEtat()` → "" tant que rien n'est fini ; puis, une seule fois, "ok", le numéro
/// et le zip gardé, ou "echec", la raison et le zip (sur le Bureau si possible).
unsafe extern "C-unwind" fn lua_etat(l: *mut std::ffi::c_void) -> i32 {
    let Some(h) = hks() else { return 0 };
    let fini = {
        let mut e = ETAT.lock().unwrap();
        match std::mem::replace(&mut *e, Etat::Libre) {
            Etat::Fini(r) => Some(r),
            autre => {
                *e = autre;
                None
            }
        }
    };
    match fini {
        None => {
            push_str(h, l, "");
            1
        }
        Some(r) => {
            push_str(h, l, if r.ok { "ok" } else { "echec" });
            push_str(h, l, &r.texte);
            push_str(h, l, &r.archive);
            3
        }
    }
}

pub const NATIVES: Natives = &[
    (lua_envoyer, b"Civ6Proxy_RapportEnvoyer\0"),
    (lua_etat, b"Civ6Proxy_RapportEtat\0"),
    (lua_log, b"Civ6Proxy_Journal\0"),
];

/// Appelée par l'accroche sur InGameTopOptionsMenu.lua : pose les natives, puis rend le texte à compiler.
pub unsafe fn prepare(l: *mut std::ffi::c_void, src: &[u8]) -> Vec<u8> {
    publish_natives(l, NATIVES, "rapport");
    patch_menu_lua(src)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MENU: &str = "<Context>\r\n\t<Stack ID=\"MainStack\">\r\n\t\t<GridButton ID=\"ReturnButton\" Style=\"MainButton\" String=\"LOC_GAME_MENU_RETURN\"/>\r\n\t\t<GridButton ID=\"OptionsButton\" Style=\"MainButton\" Size=\"250,41\" String=\"LOC_GAME_MENU_OPTIONS\" ToolTip=\"LOC_X\"/>\r\n\t\t<GridButton ID=\"ExitGameButton\" Style=\"MainButton\" String=\"LOC_GAME_MENU_EXIT_TO_DESKTOP\"/>\r\n\t</Stack>\r\n\t<Include File=\"PopupDialog\" />\r\n\t<MakeInstance Name=\"PopupDialog\" />\r\n</Context>";

    #[test]
    fn bouton_sous_options() {
        let out = String::from_utf8(patch_menu_xml(MENU.as_bytes()).unwrap()).unwrap();
        let (opt, rap, exit) = (out.find(ANCRE).unwrap(), out.find("ID=\"BGD_RapportButton\"").unwrap(), out.find("ExitGameButton").unwrap());
        assert!(opt < rap && rap < exit, "{out}");
        assert!(out.contains("\t\t<GridButton ID=\"BGD_RapportButton\" Style=\"MainButton\" Size=\"250,41\"/>"), "{out}");
        assert_eq!(out.matches("<MakeInstance Name=\"PopupDialog\"").count(), 1);
        assert!(patch_menu_xml(out.as_bytes()).is_err(), "pas deux fois");
        let err = patch_menu_xml(b"<Context><GridButton ID=\"ReturnButton\"/></Context>").unwrap_err();
        assert!(err.contains("ReturnButton"), "{err}");
    }

    #[test]
    fn bouton_avec_enfants() {
        let src = "<Context>\n  <Stack>\n    <GridButton ID=\"OptionsButton\" String=\"X\">\n      <Label ID=\"OptionsLabel\"/>\n      <GridButton ID=\"Inner\"></GridButton>\n    </GridButton>\n    <GridButton ID=\"Exit\"/>\n  </Stack>\n</Context>";
        let out = String::from_utf8(patch_menu_xml(src.as_bytes()).unwrap()).unwrap();
        assert!(out.contains("<GridButton ID=\"BGD_RapportButton\">\n      <Label ID=\"BGD_Rapport_OptionsLabel\"/>\n      <GridButton ID=\"BGD_Rapport_Inner\"></GridButton>\n    </GridButton>"), "{out}");
        assert!(out.find("BGD_RapportButton").unwrap() < out.find("ID=\"Exit\"").unwrap());
        assert!(out.contains("<MakeInstance Name=\"PopupDialog\" />"), "instance ajoutée si absente : {out}");
    }

    #[test]
    fn natives_employees() {
        assert!(patch_menu_lua(b"x = 1").starts_with(b"x = 1\r\n-- "));
        for &(_, name) in NATIVES {
            let name = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
            assert!(LUA_BLOCK.contains(name), "{name} posée mais pas utilisée");
        }
        assert!(LUA_BLOCK.contains(ID_BOUTON));
        assert!(LUA_BLOCK.contains(SAUVEGARDE));
    }

    #[test]
    fn textes_du_bloc() {
        let en = include_str!("../../textes/en_US.xml");
        let bloc = include_str!("rapport.lua");
        let cles: Vec<&str> = bloc.split('"').filter(|s| s.starts_with("LOC_BGD_")).collect();
        assert!(cles.len() >= 10, "{cles:?}");
        for cle in cles {
            assert!(en.contains(&format!("Tag=\"{cle}\"")), "{cle} manque dans textes/en_US.xml");
        }
    }

    #[test]
    fn meta() {
        let lua = lire_meta("tour=12\njoueur=Jean \"le\" Bon\nsans egal\n=x\n");
        assert_eq!(lua, [("tour".to_string(), "12".to_string()), ("joueur".to_string(), "Jean \"le\" Bon".to_string())]);
        let j = meta_json(&lua, &[("exe", "a\\b".into())], &[("logs/Lua.log".into(), 3)]);
        assert!(j.contains("\"joueur\": \"Jean \\\"le\\\" Bon\""), "{j}");
        assert!(j.contains("\"exe\": \"a\\\\b\""), "{j}");
        assert!(j.contains("{\"nom\": \"logs/Lua.log\", \"octets\": 3}"), "{j}");
    }

    #[test]
    fn coupe_et_anonymise() {
        assert_eq!(couper(b"abc".to_vec(), 10), b"abc");
        let c = String::from_utf8(couper(b"ligne1\nligne2\nligne3\n".to_vec(), 10)).unwrap();
        assert!(c.ends_with("\r\nligne3\n") && c.starts_with("[BGE : 14 premiers"), "{c}");
        let a = anonymiser(br"C:\Users\Jean\AppData et c:\users\JEAN\x, C:\Users\Jeanne\y, /Users/jean/z", "Jean");
        assert_eq!(String::from_utf8(a).unwrap(), r"C:\Users\<utilisateur>\AppData et c:\Users\<utilisateur>\x, C:\Users\Jeanne\y, /Users/<utilisateur>/z");
        assert_eq!(anonymiser(b"x", ""), b"x");
    }

    #[test]
    fn reponse_du_relais() {
        assert_eq!(lire_id(r#"{"id":"BGE-7F3A","url":"x"}"#).as_deref(), Some("BGE-7F3A"));
        assert_eq!(lire_id(r#"{ "id" : "BGE-1" }"#).as_deref(), Some("BGE-1"));
        assert_eq!(lire_id(r#"{"id":"<script>"}"#), None);
        assert_eq!(lire_id("erreur"), None);
        assert_eq!(url_du_fichier("a=1\nrapport_url=https://x\nrapport_url= https://y \n").as_deref(), Some("https://y"));
        assert_eq!(url_du_fichier("rapport_url=\n"), None);
    }

    #[test]
    fn zip_lisible() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(dos_time(1_700_000_000), ((22 << 11) | (13 << 5) | 10, ((2023 - 1980) << 9) | (11 << 5) | 14));
        let texte = "Lua.log ".repeat(1000).into_bytes();
        let z = zip(&[("a.txt".into(), texte.clone()), ("é/b.bin".into(), vec![1, 2, 3])], 1_700_000_000);
        assert!(z.len() < texte.len() / 4, "texte compressé");
        assert_eq!(&z[..4], b"PK\x03\x04");
        assert_eq!(&z[z.len() - 22..z.len() - 18], b"PK\x05\x06");
        assert_eq!(u16::from_le_bytes([z[z.len() - 12], z[z.len() - 11]]), 2);
    }
}
