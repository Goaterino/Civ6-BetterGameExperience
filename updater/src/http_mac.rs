//! Téléchargements HTTPS sous macOS, l'équivalent de http.rs : ureq, avec le TLS du système
//! (Security.framework, par native-tls) et le proxy des variables d'environnement. Pas de `curl`
//! lancé : tout reste dans le processus du lanceur. Mêmes règles que sous Windows : trois essais
//! pour une erreur réseau ou une erreur passagère du serveur, jamais pour un 404.

use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

/// Nom générique : l'API de GitHub refuse les requêtes sans User-Agent.
const USER_AGENT: &str = "BetterGameExperience-installer";
const ATTEMPTS: u32 = 3;
/// Plus que les DLL d'une release.
const MAX_SIZE: u64 = 256 * 1024 * 1024;

enum Failure {
    Network(String),
    Status(u16),
}

impl Failure {
    fn transient(&self) -> bool {
        match self {
            Failure::Network(_) => true,
            Failure::Status(code) => matches!(code, 408 | 429 | 500..=599),
        }
    }

    /// Détail non traduit, dans le texte `DOWNLOAD_OF_X_X`.
    fn detail(&self) -> String {
        match self {
            Failure::Status(code) => format!("HTTP {code}"),
            Failure::Network(msg) => msg.clone(),
        }
    }
}

fn agent() -> Result<ureq::Agent, Failure> {
    let tls = native_tls::TlsConnector::new().map_err(|e| Failure::Network(format!("TLS : {e}")))?;
    let mut builder = ureq::AgentBuilder::new()
        .user_agent(USER_AGENT)
        .tls_connector(Arc::new(tls))
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        // GitHub renvoie les fichiers d'une release vers un autre hôte.
        .redirects(5)
        .https_only(true);
    if let Some(proxy) = ["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|p| !p.is_empty()))
        .and_then(|p| ureq::Proxy::new(p).ok())
    {
        builder = builder.proxy(proxy);
    }
    Ok(builder.build())
}

/// Le contenu de `url` (HTTPS seulement), après les redirections, si la réponse est un 200. Le
/// message d'erreur est un détail à placer dans `DOWNLOAD_OF_X_X`.
pub fn get(url: &str) -> Result<Vec<u8>, String> {
    if !url.starts_with("https://") {
        return Err(format!("URL : {url}"));
    }
    let agent = agent().map_err(|e| e.detail())?;
    let mut attempt = 1;
    loop {
        match get_once(&agent, url) {
            Ok(data) => return Ok(data),
            Err(e) if e.transient() && attempt < ATTEMPTS => {
                std::thread::sleep(Duration::from_secs(attempt as u64));
                attempt += 1;
            }
            Err(e) => return Err(e.detail()),
        }
    }
}

fn get_once(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, Failure> {
    let response = match agent.get(url).call() {
        Ok(r) => r,
        Err(ureq::Error::Status(code, _)) => return Err(Failure::Status(code)),
        Err(ureq::Error::Transport(t)) => return Err(Failure::Network(t.to_string())),
    };
    if response.status() != 200 {
        return Err(Failure::Status(response.status()));
    }
    let mut data = Vec::new();
    response
        .into_reader()
        .take(MAX_SIZE)
        .read_to_end(&mut data)
        .map_err(|e| Failure::Network(e.to_string()))?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erreurs_passageres() {
        assert!(Failure::Network("dns".into()).transient());
        assert!(Failure::Status(503).transient());
        assert!(Failure::Status(429).transient());
        assert!(!Failure::Status(404).transient());
        assert_eq!(Failure::Status(404).detail(), "HTTP 404");
    }

    #[test]
    fn https_seulement() {
        assert!(get("http://example.com/x").is_err());
    }
}

/// Réseau : `cargo test --release -- --ignored`.
#[cfg(test)]
mod network {
    use super::*;

    #[test]
    #[ignore]
    fn redirection_et_404() {
        let data = get("https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest/download/manifest.json").unwrap();
        assert!(data.starts_with(b"{"));
        let err = get("https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest/download/absent.json").unwrap_err();
        assert_eq!(err, "HTTP 404");
    }
}
