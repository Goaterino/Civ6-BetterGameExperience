//! Départ et fin de tour simultanés sous macOS : les scripts que le proxy de GameCore Windows
//! complète (proxy/src/depart.rs, `patch`), avec les mêmes blocs Lua :
//!
//! - `InGame.lua` (et son remplaçant du même nom) : `ui_depart.lua`, après le relais du verdict
//!   du salon (macos_salon.rs) ;
//! - `ChatPanel.lua` : `ui_depart_chat.lua` (signaux `.bgd_` cachés) ;
//! - `ActionPanel.lua` : `ui_depart_panneau.lua` (bouton de fin de tour en attente).
//!
//! Les détours vivent dans le proxy de GameCore (proxy/src/depart_mac.rs), qui exporte
//! `BgdDepartDispo`, `BgdDepartArmer`, `BgdDepartAttendre`, `BgdDepartPartir`, `BgdDepartEtat`,
//! `BgdDepartFinForcer` et `BgdDepartFinLever`, lus par `dlsym` sur `salon::PROXY`. Les natives
//! macOS ne rendent qu'une chaîne (macos_options.rs dit pourquoi) : une seule, `Civ6Proxy_MacDepart
//! (commande, a, b)`, et une cale Lua devant chaque bloc rend les fonctions qu'il attend
//! (`Civ6RolloverProxy_Depart*`, `Civ6RolloverProxy_Fin*`), des nombres comme sous Windows. La cale
//! passe toujours trois chaînes (`luaL_checklstring` lèverait une erreur sur `nil`), les nombres
//! par `string.format("%.0f")` : les hachages du jeu (`EXECUTE_SCRIPT` = -966415096) passent
//! entiers.
//!
//! Ce qui ne demande pas GameCore reste ici : le réglage `depart_signaux` (lu dans
//! rollover_proxy.cfg), la mémoire de l'hôte (`DepartMemoriser` / `DepartMemoire`) et le journal.

use super::journal::log;
use super::options::{argument_n, lire, publier, rendre, LuaCFn};
use super::salon::PROXY;
use std::ffi::{c_char, c_int, c_void, CString};
use std::sync::atomic::Ordering;
use std::sync::Mutex;

extern "C" {
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

pub(crate) const SCRIPT_CHAT: &str = "chatpanel.lua";
pub(crate) const SCRIPT_PANNEAU: &str = "actionpanel.lua";

/// Les blocs de Windows (proxy/src/depart.rs, `BLOC_PARTIE`, `BLOC_CHAT`, `BLOC_PANNEAU`).
const BLOC_PARTIE: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("../../proxy/src/ui_depart.lua"));
const BLOC_CHAT: &str = concat!(include_str!("../../textes/bgd_textes.lua"), include_str!("../../textes/bgd_texte.lua"), include_str!("../../proxy/src/ui_depart_chat.lua"));
const BLOC_PANNEAU: &str = include_str!("../../proxy/src/ui_depart_panneau.lua");

const CALE: &str = r#"
-- macOS : natives à une seule chaîne (startup_proxy/src/macos_depart.rs).
if Civ6Proxy_MacDepart ~= nil and Civ6RolloverProxy_DepartDispo == nil then
	local function BGD_N(v) return string.format("%.0f", tonumber(v) or 0); end
	local function BGD_T(v) if v == nil then return ""; end return tostring(v); end
	local function BGD_M(cmd, a, b) return Civ6Proxy_MacDepart(cmd, a or "", b or ""); end
	local function BGD_Nombres(s)
		local t = {};
		for x in string.gmatch(tostring(s or ""), "%S+") do t[#t + 1] = tonumber(x) or 0; end
		return t[1], t[2], t[3], t[4];
	end
	Civ6RolloverProxy_DepartDispo = function() return BGD_Nombres(BGD_M("dispo")); end
	Civ6RolloverProxy_DepartArmer = function(j, e) BGD_M("armer", BGD_N(j), BGD_N(e)); end
	Civ6RolloverProxy_DepartAttendre = function(j, e) BGD_M("attendre", BGD_N(j), BGD_N(e)); end
	Civ6RolloverProxy_DepartPartir = function(r) return BGD_Nombres(BGD_M("partir", BGD_T(r))); end
	Civ6RolloverProxy_DepartEtat = function() return BGD_Nombres(BGD_M("etat")); end
	Civ6RolloverProxy_DepartSignaux = function() return BGD_Nombres(BGD_M("signaux")); end
	Civ6RolloverProxy_DepartJournal = function(t) BGD_M("journal", BGD_T(t)); end
	Civ6RolloverProxy_FinForcer = function() return BGD_Nombres(BGD_M("fin_forcer")); end
	Civ6RolloverProxy_FinLever = function() BGD_M("fin_lever"); end
	Civ6RolloverProxy_DepartMemoriser = function(s, m) BGD_M("memoriser", BGD_T(s), BGD_N(m)); end
	Civ6RolloverProxy_DepartMemoire = function(s) return BGD_Nombres(BGD_M("memoire", BGD_T(s))); end
end
"#;

/// Une fonction exportée par le proxy de GameCore, None sans lui.
unsafe fn export(nom: &str) -> Option<usize> {
    let h = PROXY.load(Ordering::Relaxed) as *mut c_void;
    if h.is_null() {
        return None;
    }
    let c = CString::new(nom).ok()?;
    let p = dlsym(h, c.as_ptr());
    (!p.is_null()).then_some(p as usize)
}

/// Un nombre passé en chaîne par la cale (`"-966415096"`, ou `"1.0"`), 0 sinon.
fn nombre(s: &str) -> f64 {
    s.trim().parse::<f64>().ok().filter(|x| x.is_finite()).unwrap_or(0.0)
}

fn joueur(s: &str) -> i32 {
    nombre(s) as i32
}

/// Les hachages du jeu dépassent 2^31 : lus en nombre, rendus en u32 (comme `as i64 as u32` sous
/// Windows).
fn hachage(s: &str) -> u32 {
    nombre(s) as i64 as u32
}

/// `depart_signaux=on|off` (off), dernière occurrence.
fn signaux(cfg: &str) -> bool {
    cfg.lines().filter_map(|l| l.trim().strip_prefix("depart_signaux=")).last().is_some_and(|v| v.trim() == "on")
}

/// Hôte : les joueurs plus attendus (un bit par joueur), gardés d'un rechargement de l'interface
/// à l'autre pour une même session de salon.
static IGNORES: Mutex<(String, u64)> = Mutex::new((String::new(), 0));

fn memoriser(cle: &str, masque: f64) {
    let m = if masque.is_finite() && masque >= 0.0 { masque as u64 } else { 0 };
    *IGNORES.lock().unwrap_or_else(|e| e.into_inner()) = (cle.to_string(), m);
}

fn memoire(cle: &str) -> u64 {
    let g = IGNORES.lock().unwrap_or_else(|e| e.into_inner());
    if !cle.is_empty() && g.0 == cle { g.1 } else { 0 }
}

/// La commande de la cale, rendue en texte (nombres séparés par des espaces).
unsafe fn commande(cmd: &str, a: &str, b: &str) -> String {
    let appel_u32 = |nom: &str| export(nom).map(|f| std::mem::transmute::<usize, unsafe extern "C" fn() -> u32>(f)());
    match cmd {
        "dispo" => {
            let bits = appel_u32("BgdDepartDispo").unwrap_or(0);
            format!("{} {} {}", bits & 1, bits >> 1 & 1, bits >> 2 & 1)
        }
        "armer" | "attendre" => {
            if let Some(f) = export(if cmd == "armer" { "BgdDepartArmer" } else { "BgdDepartAttendre" }) {
                std::mem::transmute::<usize, unsafe extern "C" fn(i32, u32)>(f)(joueur(a), hachage(b));
            }
            String::new()
        }
        "partir" => {
            let raison = CString::new(if a.is_empty() { "départ" } else { a }.replace('\0', " ")).unwrap_or_default();
            let r = export("BgdDepartPartir").map_or(0, |f| std::mem::transmute::<usize, unsafe extern "C" fn(*const c_char) -> u32>(f)(raison.as_ptr()));
            r.to_string()
        }
        "etat" => {
            let mut v = [0f64; 4];
            if let Some(f) = export("BgdDepartEtat") {
                std::mem::transmute::<usize, unsafe extern "C" fn(*mut f64)>(f)(v.as_mut_ptr());
            }
            format!("{} {} {:.3} {}", v[0], v[1], v[2], v[3])
        }
        "signaux" => (lire().is_ok_and(|cfg| signaux(&cfg)) as u8).to_string(),
        "journal" => {
            log(&format!("depart : {a}"));
            String::new()
        }
        "fin_forcer" => appel_u32("BgdDepartFinForcer").unwrap_or(0).to_string(),
        "fin_lever" => {
            if let Some(f) = export("BgdDepartFinLever") {
                std::mem::transmute::<usize, unsafe extern "C" fn()>(f)();
            }
            String::new()
        }
        "memoriser" => {
            memoriser(a, nombre(b));
            String::new()
        }
        "memoire" => memoire(a).to_string(),
        _ => {
            log(&format!("depart : commande inconnue {cmd:?}"));
            String::new()
        }
    }
}

/// `Civ6Proxy_MacDepart(commande, a, b)` → une chaîne.
unsafe extern "C-unwind" fn mac_depart(l: *mut c_void) -> c_int {
    let (cmd, a, b) = (argument_n(l, 1), argument_n(l, 2), argument_n(l, 3));
    rendre(l, &commande(&cmd, &a, &b))
}

const NATIVES: [(LuaCFn, &str); 1] = [(mac_depart, "Civ6Proxy_MacDepart\0")];

/// `texte` suivi de la cale et du bloc. Natives non posées : le bloc est ajouté quand même (celui
/// d'InGame coupe la synchro de tour de MPH dans tous les cas, comme sous Windows), la cale ne
/// rend alors rien et les blocs voient leurs fonctions absentes.
unsafe fn ajouter(l: *mut c_void, mut texte: Vec<u8>, bloc: &str, quoi: &str) -> Vec<u8> {
    if !publier(l, &NATIVES) {
        log(&format!("depart : {quoi} : native non posée, départ simultané indisponible"));
    }
    texte.extend_from_slice(b"\r\n");
    texte.extend_from_slice(CALE.as_bytes());
    texte.extend_from_slice(bloc.as_bytes());
    texte
}

/// InGame.lua déjà corrigé par le salon (`texte`, qui commence par l'original), suivi du bloc du
/// départ simultané.
pub unsafe fn partie(l: *mut c_void, texte: Vec<u8>) -> Vec<u8> {
    ajouter(l, texte, BLOC_PARTIE, "InGame.lua")
}

/// ChatPanel.lua et ActionPanel.lua (nom déjà en minuscules) ; None pour un autre script.
pub unsafe fn lua(l: *mut c_void, nom: &str, src: &[u8]) -> Option<Result<(Vec<u8>, &'static str), String>> {
    let r = match nom {
        SCRIPT_CHAT => (ajouter(l, src.to_vec(), BLOC_CHAT, "ChatPanel.lua"), "depart : signaux du départ simultané cachés"),
        SCRIPT_PANNEAU => (ajouter(l, src.to_vec(), BLOC_PANNEAU, "ActionPanel.lua"), "depart : bouton de fin de tour en attente pendant la retenue du début de tour"),
        _ => return None,
    };
    Some(Ok(r))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres_de_la_cale() {
        assert_eq!(hachage("-966415096"), 0xc665_ad08, "PlayerOperations.EXECUTE_SCRIPT");
        assert_eq!(hachage("-966415096.0"), 0xc665_ad08);
        assert_eq!(hachage("3328552200"), 0xc665_ad08, "le même hachage, positif");
        assert_eq!(joueur("3"), 3);
        assert_eq!(joueur("-1"), -1);
        assert_eq!(joueur(""), 0);
        assert_eq!(nombre("nan"), 0.0);
        assert_eq!(nombre("1125899906842624"), (1u64 << 50) as f64, "masque de 50 joueurs");
    }

    #[test]
    fn reglage_et_memoire() {
        assert!(signaux("depart_signaux=on\n"));
        assert!(!signaux("depart_signaux=on\ndepart_signaux=off\n"), "dernière occurrence");
        assert!(!signaux("actif=on\n"));
        memoriser("jeton", 5.0);
        assert_eq!(memoire("jeton"), 5);
        assert_eq!(memoire("autre"), 0);
        assert_eq!(memoire(""), 0);
        memoriser("jeton", -1.0);
        assert_eq!(memoire("jeton"), 0);
    }

    #[test]
    fn commandes_sans_proxy() {
        // PROXY nul : rien n'est appelé, des réponses neutres.
        unsafe {
            assert_eq!(commande("dispo", "", ""), "0 0 0");
            assert_eq!(commande("etat", "", ""), "0 0 0.000 0");
            assert_eq!(commande("partir", "x", ""), "0");
            assert_eq!(commande("fin_forcer", "", ""), "0");
            assert_eq!(commande("armer", "1", "2"), "");
        }
    }

    /// Chaque fonction `Civ6RolloverProxy_*` qu'appellent les blocs est rendue par la cale, et la
    /// cale n'appelle que des commandes connues.
    #[test]
    fn cale() {
        for bloc in [BLOC_PARTIE, BLOC_CHAT, BLOC_PANNEAU] {
            for (i, _) in bloc.match_indices("Civ6RolloverProxy_") {
                let nom: String = bloc[i..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                if bloc[i + nom.len()..].starts_with('*') {
                    continue; // « Civ6RolloverProxy_Depart* » dans un commentaire
                }
                assert!(CALE.contains(&format!("{nom} = function(")), "{nom} attendue mais non rendue");
            }
        }
        for (i, _) in CALE.match_indices("BGD_M(\"") {
            let cmd: String = CALE[i + 7..].chars().take_while(|c| *c != '"').collect();
            assert!(
                ["dispo", "armer", "attendre", "partir", "etat", "signaux", "journal", "fin_forcer", "fin_lever", "memoriser", "memoire"].contains(&cmd.as_str()),
                "{cmd}"
            );
        }
        // Lua : blocs `if/function … end` équilibrés dans la cale (pas d'interpréteur ici).
        let mots: Vec<&str> = CALE.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').collect();
        let ouvre = mots.iter().filter(|m| matches!(**m, "if" | "function" | "for")).count();
        let ferme = mots.iter().filter(|m| **m == "end").count();
        assert_eq!(ouvre, ferme, "if/function/for et end");
        assert_eq!(CALE.matches('(').count(), CALE.matches(')').count());
    }

    #[test]
    fn blocs_de_windows() {
        assert!(BLOC_PARTIE.ends_with(include_str!("../../proxy/src/ui_depart.lua")));
        assert!(BLOC_PARTIE.contains("BGD_DEPART_SIMULTANE"));
        assert!(BLOC_CHAT.ends_with(include_str!("../../proxy/src/ui_depart_chat.lua")));
        assert!(BLOC_PANNEAU.contains("IsProcessingMessages"));
        assert_eq!(super::super::options_texte::script_name("/x/Base/Assets/UI/Popups/ChatPanel.lua"), SCRIPT_CHAT);
        assert_eq!(super::super::options_texte::script_name("/x/Base/Assets/UI/ActionPanel.lua"), SCRIPT_PANNEAU);
        assert_eq!(super::super::options_texte::script_name("/x/DLC/Expansion2/UI/Replacements/InGame.lua"), super::super::salon_texte::SCRIPT_PARTIE);
    }

    /// Les scripts du jeu macOS installé que reçoivent les blocs, et ce qu'ils doivent contenir.
    #[test]
    fn scripts_du_jeu() {
        let Some(home) = std::env::var_os("HOME") else { return };
        let ui = std::path::Path::new(&home).join("Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/Assets");
        let lire = |rel: &str| {
            let r = std::fs::read_to_string(ui.join(rel)).ok();
            if r.is_none() {
                eprintln!("absent, test sauté : {rel}");
            }
            r
        };
        if let Some(t) = lire("Base/Assets/UI/Popups/ChatPanel.lua") {
            // ui_depart_chat.lua reprend OnChat et AddChatEntry.
            assert!(t.contains("function OnChat") && t.contains("AddChatEntry("));
            assert!(t.contains("Controls.ChatEntryStack") && t.contains("m_ChatInstances"));
        }
        if let Some(t) = lire("Base/Assets/UI/ActionPanel.lua") {
            // ui_depart_panneau.lua remplace ces gestionnaires avant LateInitialize.
            assert!(t.contains("UI.IsProcessingMessages"));
            assert!(t.contains("function OnTurnTimerUpdated(") && t.contains("function OnEndTurnBlockingChanged("));
        }
        for rel in ["Base/Assets/UI/InGame.lua", "DLC/Expansion2/UI/Replacements/InGame.lua"] {
            if let Some(t) = lire(rel) {
                assert!(!t.contains("BGD_Depart"), "{rel} déjà corrigé ?");
            }
        }
    }
}
