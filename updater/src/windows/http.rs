//! Téléchargements HTTPS par WinHTTP, la pile HTTP de Windows (TLS de Schannel, proxy du système),
//! dans le processus de l'installeur : pas de pile TLS à embarquer, et pas de processus `curl.exe`
//! lancé sans fenêtre, que les antivirus prennent pour un comportement de « dropper ».
#![allow(non_snake_case)]

use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::time::Duration;

use crate::win::wide;

const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: u32 = 0;
/// Proxy du système, configuration automatique comprise (Windows 8.1 et suivants).
const WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY: u32 = 4;
const WINHTTP_FLAG_SECURE: u32 = 0x0080_0000;
const WINHTTP_QUERY_STATUS_CODE: u32 = 19;
const WINHTTP_QUERY_FLAG_NUMBER: u32 = 0x2000_0000;
const FORMAT_MESSAGE_IGNORE_INSERTS: u32 = 0x200;
const FORMAT_MESSAGE_FROM_HMODULE: u32 = 0x800;
const FORMAT_MESSAGE_FROM_SYSTEM: u32 = 0x1000;

/// Nom générique : l'API de GitHub refuse les requêtes sans User-Agent.
const USER_AGENT: &str = "BetterGameExperience-installer";
/// Comme `curl --retry 2` : trois essais en tout, pour une erreur réseau ou une erreur passagère du
/// serveur (408, 429, 5xx), jamais pour un 404.
const ATTEMPTS: u32 = 3;

#[link(name = "winhttp")]
extern "system" {
    fn WinHttpOpen(agent: *const u16, access: u32, proxy: *const u16, bypass: *const u16, flags: u32) -> *mut c_void;
    fn WinHttpConnect(session: *mut c_void, server: *const u16, port: u16, reserved: u32) -> *mut c_void;
    fn WinHttpOpenRequest(
        connect: *mut c_void,
        verb: *const u16,
        object: *const u16,
        version: *const u16,
        referrer: *const u16,
        accept: *const *const u16,
        flags: u32,
    ) -> *mut c_void;
    fn WinHttpSendRequest(
        request: *mut c_void,
        headers: *const u16,
        headers_len: u32,
        optional: *mut c_void,
        optional_len: u32,
        total_len: u32,
        context: usize,
    ) -> i32;
    fn WinHttpReceiveResponse(request: *mut c_void, reserved: *mut c_void) -> i32;
    fn WinHttpQueryHeaders(
        request: *mut c_void,
        level: u32,
        name: *const u16,
        buffer: *mut c_void,
        len: *mut u32,
        index: *mut u32,
    ) -> i32;
    fn WinHttpReadData(request: *mut c_void, buffer: *mut c_void, len: u32, read: *mut u32) -> i32;
    fn WinHttpCloseHandle(handle: *mut c_void) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> u32;
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn FormatMessageW(flags: u32, source: *const c_void, id: u32, lang: u32, buf: *mut u16, size: u32, args: *mut c_void) -> u32;
}

/// Handle WinHTTP, fermé à la fin de sa portée.
struct Handle(*mut c_void);

impl Handle {
    fn new(h: *mut c_void) -> Result<Self, Failure> {
        if h.is_null() {
            Err(Failure::last())
        } else {
            Ok(Handle(h))
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { WinHttpCloseHandle(self.0) };
    }
}

enum Failure {
    /// Erreur de WinHTTP (`ERROR_WINHTTP_*`) : réseau, nom, TLS…
    Network(u32),
    Status(u32),
}

impl Failure {
    fn last() -> Self {
        Failure::Network(unsafe { GetLastError() })
    }

    fn transient(&self) -> bool {
        match self {
            Failure::Network(_) => true,
            Failure::Status(code) => matches!(code, 408 | 429 | 500..=599),
        }
    }

    /// Détail non traduit, dans le texte `DOWNLOAD_OF_X_X` : code HTTP, ou erreur WinHTTP avec le
    /// message de Windows, dans la langue de Windows.
    fn detail(&self) -> String {
        match self {
            Failure::Status(code) => format!("HTTP {code}"),
            Failure::Network(code) => match system_message(*code) {
                Some(msg) => format!("WinHTTP {code} : {msg}"),
                None => format!("WinHTTP {code}"),
            },
        }
    }
}

fn system_message(code: u32) -> Option<String> {
    let mut buf = [0u16; 512];
    let module = unsafe { GetModuleHandleW(wide("winhttp.dll").as_ptr()) };
    let flags = FORMAT_MESSAGE_FROM_HMODULE | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS;
    let n = unsafe { FormatMessageW(flags, module, code, 0, buf.as_mut_ptr(), buf.len() as u32, null_mut()) };
    let msg = String::from_utf16_lossy(&buf[..n as usize]).trim().to_string();
    (!msg.is_empty()).then_some(msg)
}

/// `https://hôte/chemin?requête` → (hôte, chemin et requête).
fn split_url(url: &str) -> Option<(&str, &str)> {
    let rest = url.strip_prefix("https://")?;
    let (host, path) = rest.find('/').map_or((rest, "/"), |i| (&rest[..i], &rest[i..]));
    (!host.is_empty()).then_some((host, path))
}

/// Le contenu de `url` (HTTPS seulement), après les redirections, si la réponse est un 200. Le
/// message d'erreur est un détail à placer dans `DOWNLOAD_OF_X_X`.
pub fn get(url: &str) -> Result<Vec<u8>, String> {
    let (host, path) = split_url(url).ok_or_else(|| format!("URL : {url}"))?;
    let mut attempt = 1;
    loop {
        match get_once(host, path) {
            Ok(data) => return Ok(data),
            Err(e) if e.transient() && attempt < ATTEMPTS => {
                std::thread::sleep(Duration::from_secs(attempt as u64));
                attempt += 1;
            }
            Err(e) => return Err(e.detail()),
        }
    }
}

fn get_once(host: &str, path: &str) -> Result<Vec<u8>, Failure> {
    let agent = wide(USER_AGENT);
    let open = |access| unsafe { WinHttpOpen(agent.as_ptr(), access, null(), null(), 0) };
    let mut session = open(WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY);
    if session.is_null() {
        session = open(WINHTTP_ACCESS_TYPE_DEFAULT_PROXY);
    }
    let session = Handle::new(session)?;
    let connect = Handle::new(unsafe { WinHttpConnect(session.0, wide(host).as_ptr(), 443, 0) })?;
    let verb = wide("GET");
    let object = wide(path);
    // Redirections suivies par WinHTTP (GitHub renvoie les fichiers d'une release vers un autre
    // hôte), sauf de HTTPS vers HTTP.
    let request = Handle::new(unsafe {
        WinHttpOpenRequest(connect.0, verb.as_ptr(), object.as_ptr(), null(), null(), null(), WINHTTP_FLAG_SECURE)
    })?;
    unsafe {
        if WinHttpSendRequest(request.0, null(), 0, null_mut(), 0, 0, 0) == 0 {
            return Err(Failure::last());
        }
        if WinHttpReceiveResponse(request.0, null_mut()) == 0 {
            return Err(Failure::last());
        }
    }
    let mut status = 0u32;
    let mut len = std::mem::size_of::<u32>() as u32;
    let level = WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER;
    if unsafe { WinHttpQueryHeaders(request.0, level, null(), (&mut status as *mut u32).cast(), &mut len, null_mut()) } == 0 {
        return Err(Failure::last());
    }
    if status != 200 {
        return Err(Failure::Status(status));
    }
    let mut data = Vec::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let mut read = 0u32;
        if unsafe { WinHttpReadData(request.0, buf.as_mut_ptr().cast(), buf.len() as u32, &mut read) } == 0 {
            return Err(Failure::last());
        }
        if read == 0 {
            return Ok(data);
        }
        data.extend_from_slice(&buf[..read as usize]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoupage_des_adresses() {
        assert_eq!(
            split_url("https://github.com/a/b/releases/latest/download/manifest.json"),
            Some(("github.com", "/a/b/releases/latest/download/manifest.json"))
        );
        assert_eq!(split_url("https://api.github.com/repos/a/b/releases?per_page=30"), Some(("api.github.com", "/repos/a/b/releases?per_page=30")));
        assert_eq!(split_url("https://example.com"), Some(("example.com", "/")));
        assert_eq!(split_url("http://example.com/x"), None);
        assert_eq!(split_url("https:///x"), None);
    }

    #[test]
    fn erreurs_passageres() {
        assert!(Failure::Network(12002).transient());
        assert!(Failure::Status(503).transient());
        assert!(Failure::Status(429).transient());
        assert!(!Failure::Status(404).transient());
        assert_eq!(Failure::Status(404).detail(), "HTTP 404");
        assert!(Failure::Network(12007).detail().starts_with("WinHTTP 12007"));
    }
}

/// Réseau : `cargo test --release -- --ignored`.
#[cfg(test)]
mod network {
    use super::*;

    #[test]
    #[ignore]
    fn redirection_et_404() {
        // releases/latest/download redirige vers l'hôte des fichiers de GitHub.
        let data = get("https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest/download/manifest.json").unwrap();
        assert!(data.starts_with(b"{"));
        let err = get("https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest/download/absent.json").unwrap_err();
        assert_eq!(err, "HTTP 404");
    }
}
