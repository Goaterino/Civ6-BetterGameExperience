//! Lecture d'une release : manifeste signé, puis fichiers contrôlés par leur SHA-256 (voir
//! release/README.md). Les téléchargements passent par le curl.exe de Windows, sans pile TLS à embarquer.

use std::fs;
use std::path::{Path, PathBuf};
use std::os::windows::process::CommandExt;
use std::process::Command;

use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::lang::{self, t};

/// Pas de fenêtre de console pour curl.exe : l'installeur n'en a pas.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const RELEASES: &str = "https://github.com/Goaterino/Civ6BetterGameDLLs/releases";
/// Liste des releases publiées (les brouillons n'y sont pas sans jeton), pour le canal pré-release :
/// `releases/latest` ne désigne jamais une pré-release.
const API_RELEASES: &str = "https://api.github.com/repos/Goaterino/Civ6BetterGameDLLs/releases?per_page=30";
const PUBLIC_KEY: &str = include_str!("../../release/public_key.txt");

#[derive(Deserialize)]
pub struct Manifest {
    pub version: String,
    pub files: Vec<File>,
}

#[derive(Deserialize)]
pub struct File {
    pub name: String,
    pub dest: String,
    pub sha256: String,
}

/// Dossier de travail, où le manifeste et les fichiers sont téléchargés avant l'installation.
pub fn work_dir() -> PathBuf {
    std::env::temp_dir().join("BetterGameDLLs")
}

/// Choix du canal, gardé dans `%LOCALAPPDATA%\BetterGameDLLs\canal.txt` : `prerelease`, sinon le
/// canal release. L'option de lancement Steam et la notification du jeu le suivent aussi.
fn channel_file() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("BetterGameDLLs\\canal.txt"))
}

/// Canal pré-release : la release publiée la plus récente, pré-release ou non.
pub fn prerelease() -> bool {
    channel_file().and_then(|p| fs::read_to_string(p).ok()).is_some_and(|c| c.trim() == "prerelease")
}

pub fn set_prerelease(on: bool) {
    if let Some(path) = channel_file() {
        let _ = fs::create_dir_all(path.parent().unwrap());
        let _ = fs::write(path, if on { "prerelease" } else { "release" });
    }
}

/// Version posée par la dernière installation, gardée dans `%LOCALAPPDATA%\BetterGameDLLs\installe.txt`
/// (les DLL d'une compilation locale n'ont pas de numéro) : sert à reconnaître une pré-release posée.
fn installed_file() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("BetterGameDLLs\\installe.txt"))
}

pub fn installed_version() -> Option<String> {
    installed_file().and_then(|p| fs::read_to_string(p).ok()).map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// `Some(version)` après une installation, `None` après une désinstallation.
pub fn remember_installed(version: Option<&str>) {
    let Some(path) = installed_file() else { return };
    let _ = match version {
        Some(v) => fs::create_dir_all(path.parent().unwrap()).and_then(|_| fs::write(&path, v)),
        None => fs::remove_file(&path),
    };
}

/// Vrai si la version `a` (sans le `v`) est plus récente que `b`.
pub fn is_newer(a: &str, b: &str) -> bool {
    matches!((version_key(&format!("v{a}")), version_key(&format!("v{b}"))), (Some(a), Some(b)) if a > b)
}

/// Télécharge dans `work` le manifeste de la dernière release du canal choisi, et vérifie sa
/// signature.
pub fn fetch_manifest(work: &Path) -> Result<Manifest, String> {
    fs::create_dir_all(work).map_err(|e| format!("{}: {e}", work.display()))?;
    // Avec BGD_DIST, le dossier local tient lieu de release, quel que soit le canal.
    let base = if prerelease() && std::env::var_os("BGD_DIST").is_none() {
        format!("{RELEASES}/download/{}", newest_tag(work)?)
    } else {
        format!("{RELEASES}/latest/download")
    };
    for name in ["manifest.json", "manifest.json.sig"] {
        download(&format!("{base}/{name}"), &work.join(name))?;
    }
    verify_manifest(work)
}

#[derive(Deserialize)]
struct ReleaseInfo {
    tag_name: String,
    #[serde(default)]
    draft: bool,
}

/// Tag de la release publiée au plus grand numéro, pré-releases comprises.
fn newest_tag(work: &Path) -> Result<String, String> {
    let path = work.join("releases.json");
    download(API_RELEASES, &path)?;
    let data = fs::read(&path).map_err(|e| format!("releases.json : {e}"))?;
    let list: Vec<ReleaseInfo> =
        serde_json::from_slice(&data).map_err(|e| lang::f(["liste des releases illisible : {}", "unreadable release list: {}"], &[&e]))?;
    pick_newest(&list).ok_or_else(|| t(["aucune release publiée", "no published release"]).to_string())
}

fn pick_newest(list: &[ReleaseInfo]) -> Option<String> {
    list.iter().filter(|r| !r.draft).filter_map(|r| Some((version_key(&r.tag_name)?, &r.tag_name))).max().map(|(_, tag)| tag.clone())
}

/// `v1.2.3` ou `v1.2.3-rc.1` : une pré-release passe avant la version finale du même numéro.
fn version_key(tag: &str) -> Option<(u32, u32, u32, bool, String)> {
    let v = tag.strip_prefix('v')?;
    let (core, suffix) = v.split_once('-').map_or((v, None), |(c, s)| (c, Some(s)));
    let mut it = core.split('.').map(|n| n.parse().ok());
    let key = (it.next()??, it.next()??, it.next()??);
    it.next().is_none().then(|| (key.0, key.1, key.2, suffix.is_none(), suffix.unwrap_or("").to_string()))
}

/// Télécharge dans `work` les fichiers du manifeste qui n'y sont pas déjà, et les contrôle.
pub fn fetch_files(m: &Manifest, work: &Path) -> Result<(), String> {
    for f in &m.files {
        if check_file(work, f).is_err() {
            download(&format!("{RELEASES}/download/v{}/{}", m.version, f.name), &work.join(&f.name))?;
            check_file(work, f)?;
        }
    }
    Ok(())
}

/// Relit une release déjà téléchargée dans `work` (côté administrateur, avant d'installer) :
/// signature du manifeste et SHA-256 de chaque fichier.
pub fn load(work: &Path) -> Result<Manifest, String> {
    let m = verify_manifest(work)?;
    for f in &m.files {
        check_file(work, f)?;
    }
    Ok(m)
}

fn verify_manifest(work: &Path) -> Result<Manifest, String> {
    let b64 = base64::engine::general_purpose::STANDARD;
    let key: [u8; 32] = b64.decode(PUBLIC_KEY.trim()).unwrap().try_into().unwrap();
    let key = VerifyingKey::from_bytes(&key).unwrap();
    let data = fs::read(work.join("manifest.json")).map_err(|e| format!("manifest.json : {e}"))?;
    let sig = fs::read_to_string(work.join("manifest.json.sig")).map_err(|e| format!("manifest.json.sig : {e}"))?;
    let sig = b64.decode(sig.trim()).ok().and_then(|s| <[u8; 64]>::try_from(s).ok());
    let sig = sig.ok_or(t(["signature du manifeste illisible", "unreadable manifest signature"]))?;
    key.verify_strict(&data, &Signature::from_bytes(&sig)).map_err(|_| t(["signature du manifeste invalide", "invalid manifest signature"]))?;
    serde_json::from_slice(&data).map_err(|e| lang::f(["manifeste illisible : {}", "unreadable manifest: {}"], &[&e]))
}

fn check_file(work: &Path, f: &File) -> Result<(), String> {
    if sha256(&work.join(&f.name))? == f.sha256 {
        Ok(())
    } else {
        Err(lang::f(["{} : SHA-256 différent de celui du manifeste", "{}: SHA-256 differs from the manifest"], &[&f.name]))
    }
}

pub fn sha256(path: &Path) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect())
}

/// `BGD_DIST`, s'il est défini, remplace GitHub par un dossier local (tests).
fn download(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(dir) = std::env::var_os("BGD_DIST") {
        let name = url.rsplit('/').next().unwrap();
        return fs::copy(Path::new(&dir).join(name), dest).map(drop).map_err(|e| format!("{name} : {e}"));
    }
    let curl = Path::new(&std::env::var_os("SystemRoot").unwrap_or("C:\\Windows".into())).join("System32\\curl.exe");
    let out = Command::new(curl)
        .args(["-fsSL", "--retry", "2", "-o"])
        .arg(dest)
        .arg(url)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("curl.exe : {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(lang::f(["téléchargement de {} : {}", "download of {}: {}"], &[&url, &String::from_utf8_lossy(&out.stderr).trim()]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(tag: &str, draft: bool) -> ReleaseInfo {
        ReleaseInfo { tag_name: tag.into(), draft }
    }

    #[test]
    fn derniere_release_du_canal_prerelease() {
        let list = [rel("v0.4.0", false), rel("v0.5.0-rc.1", false), rel("v0.6.0", true), rel("essai", false), rel("v0.10.0-rc.2", false)];
        assert_eq!(pick_newest(&list).as_deref(), Some("v0.10.0-rc.2"));
        assert_eq!(pick_newest(&[rel("v0.5.0-rc.1", false), rel("v0.5.0", false)]).as_deref(), Some("v0.5.0"));
        assert_eq!(pick_newest(&[rel("v0.5.0-rc.1", false), rel("v0.5.0-rc.2", false)]).as_deref(), Some("v0.5.0-rc.2"));
        assert_eq!(pick_newest(&[rel("v1.0.0", true)]), None);
    }

    #[test]
    fn comparaison() {
        assert!(is_newer("0.4.1", "0.4.0"));
        assert!(is_newer("0.5.0-rc.1", "0.4.0"));
        assert!(is_newer("0.5.0", "0.5.0-rc.1"));
        assert!(!is_newer("0.4.0", "0.4.0"));
        assert!(!is_newer("0.4.0", "0.5.0-rc.1"));
        assert!(!is_newer("dev", "0.4.0"));
    }

    #[test]
    fn liste_de_l_api() {
        let json = r#"[{"tag_name":"v0.4.1","draft":false,"prerelease":true,"assets":[]},{"tag_name":"v0.4.0","draft":false,"prerelease":false}]"#;
        let list: Vec<ReleaseInfo> = serde_json::from_str(json).unwrap();
        assert_eq!(pick_newest(&list).as_deref(), Some("v0.4.1"));
    }
}

/// Réseau : `cargo test --release -- --ignored`. Les deux canaux, sur les releases publiées.
#[cfg(test)]
mod network {
    use super::*;

    #[test]
    #[ignore]
    fn deux_canaux_sur_github() {
        let dir = std::env::temp_dir().join("bgd_test_canaux");
        std::env::set_var("LOCALAPPDATA", &dir);
        for on in [false, true] {
            set_prerelease(on);
            assert_eq!(prerelease(), on);
            let work = dir.join(if on { "pre" } else { "rel" });
            let m = fetch_manifest(&work).unwrap();
            println!("prerelease={on} : {}", m.version);
        }
    }
}
