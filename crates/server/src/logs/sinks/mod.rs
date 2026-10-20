//! Expéditeurs, un module par service, et ce qu'ils partagent : client HTTP,
//! connexions TCP/TLS persistantes, envoi UDP.
//!
//! Un expéditeur reçoit un lot et l'envoie en entier ou échoue : il ne
//! réessaie pas. La perte est le prix accepté d'un serveur qui ne se bloque
//! jamais sur ses journaux ; c'est la file de `ship` qui la compte.
//!
//! Aucun message d'erreur ne doit porter de secret : les erreurs reqwest sont
//! dépouillées de leur URL, et tout texte renvoyé par un service est expurgé.

pub mod elasticsearch;
pub mod gelf;
pub mod loki;
pub mod syslog;
pub mod victorialogs;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Url;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::Mutex;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::pki_types::ServerName;

use super::Record;
use super::scrub;

/// Délai d'un envoi, connexion comprise.
pub const SEND_TIMEOUT: Duration = Duration::from_secs(10);

/// Un expéditeur de lots.
#[async_trait]
pub trait Sink: Send + Sync {
    async fn send(&self, batch: &[Arc<Record>]) -> Result<(), String>;
}

// --------------------------------------------------------------------------
// Adresses
// --------------------------------------------------------------------------

/// Sépare `hôte:port` (IPv6 entre crochets), le port retombant sur `default`.
pub fn host_port(address: &str, default: u16) -> Result<(String, u16), String> {
    let address = address.trim();
    if address.is_empty() {
        return Err("the address is required (host:port)".into());
    }
    if address.contains("://") {
        return Err(format!("`{address}`: expected host:port, not a URL"));
    }
    if let Some(rest) = address.strip_prefix('[') {
        let (host, after) =
            rest.split_once(']').ok_or_else(|| format!("`{address}`: unclosed IPv6 bracket"))?;
        let port = match after.strip_prefix(':') {
            Some(port) => parse_port(address, port)?,
            None if after.is_empty() => default,
            None => return Err(format!("`{address}`: invalid address")),
        };
        return Ok((host.to_string(), port));
    }
    match address.rsplit_once(':') {
        // Plus d'un deux-points sans crochets : une IPv6 nue, sans port.
        Some((host, _)) if host.contains(':') => Ok((address.to_string(), default)),
        Some((host, port)) => Ok((host.to_string(), parse_port(address, port)?)),
        None => Ok((address.to_string(), default)),
    }
}

fn parse_port(address: &str, port: &str) -> Result<u16, String> {
    port.parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| format!("`{address}`: invalid port `{port}`"))
}

pub fn parse_url(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw.trim()).map_err(|_| format!("`{}` is not a valid URL", raw.trim()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("the URL must start with http:// or https://".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("put credentials in the username and password fields, not in the URL".into());
    }
    Ok(url)
}

/// Refuse une destination qui viserait ce serveur lui-même.
async fn vet(host: &str, port: u16) -> Result<(), String> {
    crate::notify::guard::vet_host(host, port).await.map_err(|error| error.to_string())
}

// --------------------------------------------------------------------------
// HTTP
// --------------------------------------------------------------------------

pub fn http_client(insecure_tls: bool) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(SEND_TIMEOUT)
        .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION")))
        // Les identifiants ne partent que vers l'adresse saisie.
        .redirect(reqwest::redirect::Policy::none())
        .danger_accept_invalid_certs(insecure_tls)
        .build()
        .map_err(|error| format!("HTTP client: {error}"))
}

/// Authentification HTTP facultative.
#[derive(Clone, Default)]
pub struct HttpAuth {
    pub username: Option<String>,
    pub password: Option<String>,
    /// Valeur complète de l'en-tête `Authorization` (« Bearer … », « ApiKey … »).
    pub authorization: Option<String>,
}

impl HttpAuth {
    fn apply(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(value) = &self.authorization {
            return request.header(reqwest::header::AUTHORIZATION, value);
        }
        match (&self.username, &self.password) {
            (Some(user), password) => request.basic_auth(user, password.as_deref()),
            (None, Some(password)) => request.basic_auth("", Some(password)),
            (None, None) => request,
        }
    }
}

/// POST d'un corps, réussi sur tout statut 2xx ; rend le corps de la réponse.
pub async fn post(
    client: &reqwest::Client,
    url: &Url,
    content_type: &str,
    body: Vec<u8>,
    auth: &HttpAuth,
    headers: &[(&str, String)],
) -> Result<String, String> {
    if let (Some(host), Some(port)) = (url.host_str(), url.port_or_known_default()) {
        vet(host.trim_matches(|c| c == '[' || c == ']'), port).await?;
    }
    let mut request =
        client.post(url.clone()).header(reqwest::header::CONTENT_TYPE, content_type).body(body);
    for (name, value) in headers {
        request = request.header(*name, value);
    }
    let response = auth
        .apply(request)
        .send()
        .await
        .map_err(|error| scrub::text(&error.without_url().to_string()))?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        let detail: String = text.chars().take(300).collect();
        return Err(scrub::text(&format!("HTTP {}: {}", status.as_u16(), detail.trim())));
    }
    Ok(text)
}

// --------------------------------------------------------------------------
// Flux TCP et TLS
// --------------------------------------------------------------------------

type Stream = Box<dyn AsyncWrite + Unpin + Send>;

/// Connexion persistante, rouverte au lot suivant après une erreur.
pub struct StreamSink {
    host: String,
    port: u16,
    tls: Option<Arc<tokio_rustls::rustls::ClientConfig>>,
    conn: Mutex<Option<Stream>>,
}

impl StreamSink {
    pub fn new(
        host: String,
        port: u16,
        tls: Option<Arc<tokio_rustls::rustls::ClientConfig>>,
    ) -> Self {
        Self { host, port, tls, conn: Mutex::new(None) }
    }

    async fn connect(&self) -> Result<Stream, String> {
        vet(&self.host, self.port).await?;
        let tcp = TcpStream::connect((self.host.as_str(), self.port)).await.map_err(|error| {
            format!("connection to {}:{} failed: {error}", self.host, self.port)
        })?;
        let _ = tcp.set_nodelay(true);
        match &self.tls {
            None => Ok(Box::new(tcp)),
            Some(config) => {
                let name = ServerName::try_from(self.host.clone())
                    .map_err(|_| format!("`{}` is not a valid TLS server name", self.host))?;
                let stream = TlsConnector::from(config.clone())
                    .connect(name, tcp)
                    .await
                    .map_err(|error| format!("TLS handshake failed: {error}"))?;
                Ok(Box::new(stream))
            }
        }
    }

    /// Écrit les trames d'un lot à la suite.
    pub async fn write_frames(&self, frames: Vec<Vec<u8>>) -> Result<(), String> {
        let mut guard = self.conn.lock().await;
        if guard.is_none() {
            *guard = Some(self.connect().await?);
        }
        let Some(stream) = guard.as_mut() else { return Err("not connected".into()) };
        let mut result = Ok(());
        for frame in &frames {
            if let Err(error) = stream.write_all(frame).await {
                result = Err(format!("write failed: {error}"));
                break;
            }
        }
        if result.is_ok()
            && let Err(error) = stream.flush().await
        {
            result = Err(format!("write failed: {error}"));
        }
        if result.is_err() {
            *guard = None;
        }
        result
    }
}

// --------------------------------------------------------------------------
// UDP
// --------------------------------------------------------------------------

/// Envoie chaque datagramme à `host:port`.
pub async fn send_datagrams(host: &str, port: u16, datagrams: &[Vec<u8>]) -> Result<(), String> {
    vet(host, port).await?;
    let target = tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| format!("cannot resolve {host}: {error}"))?
        .next()
        .ok_or_else(|| format!("cannot resolve {host}"))?;
    let local = if target.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
    let socket = UdpSocket::bind(local).await.map_err(|error| format!("UDP socket: {error}"))?;
    for datagram in datagrams {
        socket
            .send_to(datagram, target)
            .await
            .map_err(|error| format!("UDP send to {target} failed: {error}"))?;
    }
    Ok(())
}

/// Configuration TLS : l'autorité collée seule, ou les racines habituelles.
pub fn tls_config(
    ca_cert: Option<&str>,
    insecure: bool,
) -> Result<Arc<tokio_rustls::rustls::ClientConfig>, String> {
    dumbmonit_collectors::activedirectory::ldap_client_config(ca_cert, insecure)
        .map_err(|error| error.to_string())
}

/// Coupe un texte à `max` octets sans briser un caractère.
pub fn truncate(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses() {
        assert_eq!(host_port("graylog:12201", 1).unwrap(), ("graylog".into(), 12201));
        assert_eq!(host_port("graylog", 514).unwrap(), ("graylog".into(), 514));
        assert_eq!(host_port("[::1]:6514", 1).unwrap(), ("::1".into(), 6514));
        assert_eq!(host_port("fe80::1", 514).unwrap(), ("fe80::1".into(), 514));
        assert!(host_port("udp://x:1", 1).is_err());
        assert!(host_port("x:0", 1).is_err());
        assert!(parse_url("http://u:p@x/").is_err());
    }
}
