//! HTTPS requests through WinHTTP, with no child process or window: the same API on Windows and
//! under Wine (Proton). Used by the update check (`maj`) and bug report upload (`rapport`),
//! always from a separate thread: the calls block. Redirects (GitHub sends release files to
//! another host) are followed by WinHTTP.

use crate::win::{wide, GetLastError};
use std::ffi::c_void;

type Handle = *mut c_void;

const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: u32 = 0;
const WINHTTP_FLAG_SECURE: u32 = 0x0080_0000;
const WINHTTP_QUERY_STATUS_CODE: u32 = 19;
const WINHTTP_QUERY_FLAG_NUMBER: u32 = 0x2000_0000;
/// Anything larger is not what we expect (manifest, API listing, relay response).
const MAX_RESPONSE: usize = 16 << 20;
const USER_AGENT: &str = "BetterGameExperience";

#[link(name = "winhttp")]
extern "system" {
    fn WinHttpOpen(agent: *const u16, access: u32, proxy: *const u16, bypass: *const u16, flags: u32) -> Handle;
    fn WinHttpConnect(session: Handle, server: *const u16, port: u16, reserved: u32) -> Handle;
    fn WinHttpOpenRequest(
        connect: Handle,
        verb: *const u16,
        path: *const u16,
        version: *const u16,
        referer: *const u16,
        accept: *const *const u16,
        flags: u32,
    ) -> Handle;
    fn WinHttpSetTimeouts(handle: Handle, resolve: i32, connect: i32, send: i32, receive: i32) -> i32;
    fn WinHttpSendRequest(
        request: Handle,
        headers: *const u16,
        headers_len: u32,
        body: *const c_void,
        body_len: u32,
        total_len: u32,
        context: usize,
    ) -> i32;
    fn WinHttpReceiveResponse(request: Handle, reserved: *mut c_void) -> i32;
    fn WinHttpQueryHeaders(request: Handle, level: u32, name: *const u16, buffer: *mut c_void, len: *mut u32, index: *mut u32) -> i32;
    fn WinHttpReadData(request: Handle, buffer: *mut c_void, to_read: u32, read: *mut u32) -> i32;
    fn WinHttpCloseHandle(handle: Handle) -> i32;
}

/// WinHTTP handle, closed on drop.
struct Guard(Handle);

impl Guard {
    fn new(handle: Handle, what: &str) -> Result<Guard, String> {
        if handle.is_null() {
            Err(format!("{what} : erreur WinHTTP {}", unsafe { GetLastError() }))
        } else {
            Ok(Guard(handle))
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        unsafe { WinHttpCloseHandle(self.0) };
    }
}

fn check(ok: i32, what: &str) -> Result<(), String> {
    if ok == 0 {
        Err(format!("{what} : erreur WinHTTP {}", unsafe { GetLastError() }))
    } else {
        Ok(())
    }
}

/// `https://host[:port]/path?query` → host, port, path with its query.
fn split_url(url: &str) -> Result<(&str, u16, &str), String> {
    let rest = url.strip_prefix("https://").ok_or_else(|| format!("adresse non HTTPS : {url}"))?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, port.parse().map_err(|_| format!("port invalide : {url}"))?),
        None => (authority, 443),
    };
    if host.is_empty() {
        return Err(format!("adresse sans hôte : {url}"));
    }
    Ok((host, port, path))
}

/// One request: HTTP status and response body. `timeout_ms` bounds each step (resolve, connect,
/// send, receive), not the total duration.
pub fn request(method: &str, url: &str, headers: &[(&str, &str)], body: &[u8], timeout_ms: i32) -> Result<(u32, Vec<u8>), String> {
    let (host, port, path) = split_url(url)?;
    let body_len = u32::try_from(body.len()).map_err(|_| "corps trop gros".to_string())?;
    let headers: String = headers.iter().map(|(k, v)| format!("{k}: {v}\r\n")).collect();
    let headers = wide(&headers);
    unsafe {
        let session = Guard::new(
            WinHttpOpen(wide(USER_AGENT).as_ptr(), WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, std::ptr::null(), std::ptr::null(), 0),
            "WinHttpOpen",
        )?;
        let connection = Guard::new(WinHttpConnect(session.0, wide(host).as_ptr(), port, 0), "WinHttpConnect")?;
        let request = Guard::new(
            WinHttpOpenRequest(
                connection.0,
                wide(method).as_ptr(),
                wide(path).as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                WINHTTP_FLAG_SECURE,
            ),
            "WinHttpOpenRequest",
        )?;
        check(WinHttpSetTimeouts(request.0, timeout_ms, timeout_ms, timeout_ms, timeout_ms), "WinHttpSetTimeouts")?;
        check(
            WinHttpSendRequest(
                request.0,
                headers.as_ptr(),
                // In characters, without the trailing NUL.
                (headers.len() - 1) as u32,
                if body.is_empty() { std::ptr::null() } else { body.as_ptr() as *const c_void },
                body_len,
                body_len,
                0,
            ),
            "WinHttpSendRequest",
        )?;
        check(WinHttpReceiveResponse(request.0, std::ptr::null_mut()), "WinHttpReceiveResponse")?;

        let mut status = 0u32;
        let mut size = 4u32;
        check(
            WinHttpQueryHeaders(
                request.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                std::ptr::null(),
                &mut status as *mut u32 as *mut c_void,
                &mut size,
                std::ptr::null_mut(),
            ),
            "WinHttpQueryHeaders",
        )?;

        let mut out = Vec::new();
        let mut chunk = [0u8; 16384];
        loop {
            let mut read = 0u32;
            check(WinHttpReadData(request.0, chunk.as_mut_ptr() as *mut c_void, chunk.len() as u32, &mut read), "WinHttpReadData")?;
            if read == 0 {
                break;
            }
            out.extend_from_slice(&chunk[..read as usize]);
            if out.len() > MAX_RESPONSE {
                return Err("réponse trop grosse".into());
            }
        }
        Ok((status, out))
    }
}

/// GET of a text (manifest, API listing): an error for any non-2xx status.
pub fn get(url: &str) -> Result<String, String> {
    let (status, body) = request("GET", url, &[], &[], 20_000)?;
    if !(200..300).contains(&status) {
        return Err(format!("HTTP {status}"));
    }
    Ok(String::from_utf8_lossy(&body).into_owned())
}

const BOUNDARY: &str = "----BGEfrontiere7d1c0a52";

/// `multipart/form-data` body holding a single file.
fn multipart_body(field: &str, filename: &str, content_type: &str, data: &[u8]) -> Vec<u8> {
    let mut body = format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n").into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// POST of one file as `multipart/form-data`: HTTP status and response body.
pub fn post_file(url: &str, headers: &[(&str, &str)], field: &str, filename: &str, content_type: &str, data: &[u8], timeout_ms: i32) -> Result<(u32, String), String> {
    let content = format!("multipart/form-data; boundary={BOUNDARY}");
    let mut all: Vec<(&str, &str)> = headers.to_vec();
    all.push(("Content-Type", &content));
    let (status, body) = request("POST", url, &all, &multipart_body(field, filename, content_type, data), timeout_ms)?;
    Ok((status, String::from_utf8_lossy(&body).into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses() {
        assert_eq!(split_url("https://github.com/a/b?x=1"), Ok(("github.com", 443, "/a/b?x=1")));
        assert_eq!(split_url("https://example.org"), Ok(("example.org", 443, "/")));
        assert_eq!(split_url("https://example.org:8443/x"), Ok(("example.org", 8443, "/x")));
        assert!(split_url("http://example.org/").is_err());
        assert!(split_url("https:///x").is_err());
        assert!(split_url("https://example.org:abc/x").is_err());
    }

    #[test]
    fn corps_multipart() {
        let body = multipart_body("archive", "r.zip", "application/zip", b"PK\x03\x04");
        let text = String::from_utf8_lossy(&body);
        assert!(text.starts_with(&format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"archive\"; filename=\"r.zip\"\r\n")));
        assert!(text.contains("Content-Type: application/zip\r\n\r\nPK"));
        assert!(text.ends_with(&format!("\r\n--{BOUNDARY}--\r\n")));
    }

    /// Network: `cargo test --release http -- --ignored`.
    #[test]
    #[ignore]
    fn get_github() {
        let body = get("https://api.github.com/repos/Goaterino/Civ6-BetterGameExperience/releases?per_page=1").unwrap();
        assert!(body.contains("tag_name"));
    }
}
