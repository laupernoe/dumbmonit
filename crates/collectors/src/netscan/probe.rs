//! Sondes d'un port : connexion TCP, poignée de main TLS (certificat relevé
//! sans jamais être vérifié), requête HTTP anonyme et lecture de bannière.
//!
//! Rien n'est jamais authentifié : une requête `GET`, une bannière que le
//! service écrit de lui-même, ou quelques octets inoffensifs d'un protocole
//! (un `PING` Redis, une demande TLS PostgreSQL). Aucune donnée applicative
//! sensible, aucun identifiant.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

/// Taille maximale d'un corps HTTP lu : de quoi tenir un `<head>` et un en-tête
/// de page, pas un téléversement.
const MAX_BODY: usize = 64 * 1024;
/// Octets de bannière lus au plus.
const MAX_BANNER: usize = 4096;
/// Durée totale d'une lecture de réponse HTTP, en multiples du délai par
/// lecture : un service qui égrène un octet de temps en temps ne retient pas la
/// sonde indéfiniment.
const HTTP_READ_BUDGET: u32 = 3;

/// Tout ce qu'on a pu observer d'un port ouvert, à confronter aux empreintes.
#[derive(Debug, Default, Clone)]
pub struct PortEvidence {
    pub tls: bool,
    /// Sujet, émetteur et noms alternatifs du certificat présenté, concaténés.
    pub cert: Option<String>,
    /// Bannière spontanée (SSH, SMTP, MySQL…), en texte lisible.
    pub banner: Option<String>,
    /// Réponse HTTP par chemin demandé (une empreinte peut viser `/api/version`).
    pub http: std::collections::HashMap<String, HttpResponse>,
}

impl PortEvidence {
    /// Réponse HTTP pour un chemin donné.
    pub fn response(&self, path: &str) -> Option<&HttpResponse> {
        let path = if path.is_empty() { "/" } else { path };
        self.http.get(path)
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    /// En-têtes, noms en minuscules.
    pub headers: Vec<(String, String)>,
    pub title: Option<String>,
    pub body: String,
}

impl HttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers.iter().find(|(k, _)| *k == name).map(|(_, v)| v.as_str())
    }
}

/// Ouvre un port et récolte ce qu'il révèle sans authentification.
///
/// `paths` : les chemins HTTP à interroger (vide = aucun essai HTTP, pour un
/// port qui ne parle jamais HTTP). `want_banner` : lire la bannière spontanée.
/// `banner_timeout` : délai court pour la lecture de bannière.
pub async fn observe(
    ip: IpAddr,
    port: u16,
    timeout: Duration,
    banner_timeout: Duration,
    paths: &[String],
    want_banner: bool,
) -> Option<PortEvidence> {
    // Première connexion : TCP. Un échec = port fermé/filtré, rien à dire.
    let stream = connect(ip, port, timeout).await?;
    let mut evidence = PortEvidence::default();

    // Bannière spontanée : lire ce qui vient sans rien envoyer. Un service HTTP
    // ne dit rien tant qu'on ne demande rien ; une lecture courte le confirme.
    // Chaque étape suivante ouvre sa propre connexion : celle-ci est fermée
    // aussitôt, pour qu'une sonde ne tienne jamais deux sockets à la fois (le
    // permis du balayage vaut une socket).
    let banner = if want_banner {
        read_quietly(stream, banner_timeout).await
    } else {
        drop(stream);
        None
    };
    if let Some(bytes) = banner
        && !bytes.is_empty()
    {
        evidence.banner = Some(printable(&bytes));
        // Un service qui a parlé spontanément ne parle pas HTTP sur la même
        // connexion : on n'insiste pas avec une requête HTTP.
        return Some(evidence);
    }

    if paths.is_empty() {
        return Some(evidence);
    }

    // Décision TLS, une fois : poignée de main sur une connexion fraîche. Si elle
    // aboutit, le port parle TLS et on tient son certificat.
    let Some(stream) = connect(ip, port, timeout).await else { return Some(evidence) };
    let tls = match tls_handshake(stream, ip, timeout).await {
        Ok((tls_stream, cert)) => {
            evidence.tls = true;
            evidence.cert = cert;
            // La première réponse est lue sur la connexion déjà ouverte.
            drop(tls_stream);
            true
        }
        Err(()) => false,
    };

    // Une requête par chemin, sur une connexion dédiée (HTTP/1.0 « Connection:
    // close » : on ne réutilise pas la connexion).
    for path in paths {
        if let Some(response) = fetch(ip, port, tls, path, timeout).await {
            evidence
                .http
                .insert(if path.is_empty() { "/".to_string() } else { path.clone() }, response);
        }
    }
    Some(evidence)
}

/// Ouvre une connexion (TLS ou claire) et lit la réponse à `GET path`.
async fn fetch(
    ip: IpAddr,
    port: u16,
    tls: bool,
    path: &str,
    timeout: Duration,
) -> Option<HttpResponse> {
    let stream = connect(ip, port, timeout).await?;
    if tls {
        let (tls_stream, _) = tls_handshake(stream, ip, timeout).await.ok()?;
        http_get(tls_stream, ip, path, timeout).await
    } else {
        http_get(stream, ip, path, timeout).await
    }
}

/// Mène un échange sur mesure (octets envoyés, réponse lue) pour une empreinte
/// de bannière provoquée. Connexion dédiée, jamais réutilisée.
pub async fn exchange(ip: IpAddr, port: u16, send: &[u8], timeout: Duration) -> Option<String> {
    let mut stream = connect(ip, port, timeout).await?;
    if tokio::time::timeout(timeout, stream.write_all(send)).await.ok()?.is_err() {
        return None;
    }
    let bytes = read_quietly(stream, timeout).await?;
    Some(printable(&bytes))
}

async fn connect(ip: IpAddr, port: u16, timeout: Duration) -> Option<TcpStream> {
    tokio::time::timeout(timeout, TcpStream::connect(SocketAddr::new(ip, port))).await.ok()?.ok()
}

/// Lit ce qui arrive dans le délai, sans rien envoyer. `Some([])` si rien n'est
/// venu avant l'échéance (le service attend qu'on parle d'abord).
async fn read_quietly<S>(mut stream: S, timeout: Duration) -> Option<Vec<u8>>
where
    S: AsyncReadExt + Unpin,
{
    let mut buffer = vec![0u8; MAX_BANNER];
    match tokio::time::timeout(timeout, stream.read(&mut buffer)).await {
        Ok(Ok(0)) => Some(Vec::new()),
        Ok(Ok(n)) => {
            buffer.truncate(n);
            Some(buffer)
        }
        Ok(Err(_)) => None,
        Err(_) => Some(Vec::new()),
    }
}

/// Tente une poignée de main TLS et renvoie le flux chiffré plus le descriptif
/// du certificat présenté. Le vérificateur accepte tout : on identifie, on ne
/// protège rien, et les certificats auto-signés sont la règle en réseau local.
async fn tls_handshake(
    stream: TcpStream,
    ip: IpAddr,
    timeout: Duration,
) -> Result<(tokio_rustls::client::TlsStream<TcpStream>, Option<String>), ()> {
    let config = tls_config();
    let connector = TlsConnector::from(config);
    // Un nom littéral : on se connecte par IP, le SNI importe peu pour relever
    // un certificat.
    let server_name = ServerName::IpAddress(ip.into());
    let handshake = connector.connect(server_name, stream);
    let tls_stream =
        tokio::time::timeout(timeout, handshake).await.map_err(|_| ())?.map_err(|_| ())?;
    let cert = {
        let (_, connection) = tls_stream.get_ref();
        connection.peer_certificates().and_then(|chain| chain.first()).map(describe_cert)
    };
    Ok((tls_stream, cert))
}

/// Envoie une requête HTTP anonyme et lit la réponse (status, en-têtes, corps
/// borné, titre).
async fn http_get<S>(
    mut stream: S,
    ip: IpAddr,
    path: &str,
    timeout: Duration,
) -> Option<HttpResponse>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let path = if path.is_empty() { "/" } else { path };
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {ip}\r\nUser-Agent: DumbMonit-Discovery\r\nAccept: */*\r\nConnection: close\r\n\r\n"
    );
    if tokio::time::timeout(timeout, stream.write_all(request.as_bytes())).await.ok()?.is_err() {
        return None;
    }

    let mut raw = Vec::with_capacity(8192);
    let mut chunk = vec![0u8; 8192];
    let deadline = tokio::time::Instant::now() + timeout.saturating_mul(HTTP_READ_BUDGET);
    loop {
        let step = (tokio::time::Instant::now() + timeout).min(deadline);
        match tokio::time::timeout_at(step, stream.read(&mut chunk)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                raw.extend_from_slice(&chunk[..n]);
                if raw.len() >= MAX_BODY {
                    raw.truncate(MAX_BODY);
                    break;
                }
            }
            _ => break,
        }
    }
    parse_http(&raw)
}

/// Découpe une réponse HTTP brute. Tolérant : une réponse tronquée au milieu du
/// corps reste exploitable.
pub fn parse_http(raw: &[u8]) -> Option<HttpResponse> {
    let text = String::from_utf8_lossy(raw);
    let split =
        text.find("\r\n\r\n").map(|i| (i, 4)).or_else(|| text.find("\n\n").map(|i| (i, 2)))?;
    let (head, body) = (&text[..split.0], &text[split.0 + split.1..]);
    let mut lines = head.lines();
    let status_line = lines.next()?;
    // « HTTP/1.1 200 OK »
    let status = status_line.split_whitespace().nth(1)?.parse::<u16>().ok()?;
    let mut headers = Vec::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    let title = extract_title(body);
    Some(HttpResponse { status, headers, title, body: body.to_string() })
}

/// Contenu de la première balise `<title>`, espaces normalisés.
pub fn extract_title(body: &str) -> Option<String> {
    let lower = body.to_ascii_lowercase();
    let start = lower.find("<title")?;
    let open_end = lower[start..].find('>')? + start + 1;
    let end = lower[open_end..].find("</title>")? + open_end;
    let title: String = body[open_end..end].split_whitespace().collect::<Vec<_>>().join(" ");
    (!title.is_empty()).then_some(title)
}

/// Rend une suite d'octets en texte lisible : l'UTF-8 valide tel quel, le reste
/// remplacé. Les bannières binaires (MySQL) gardent leurs parties imprimables.
fn printable(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\u{fffd}', ".")
}

/// Sujet + émetteur + noms alternatifs d'un certificat, pour la condition
/// `cert` des empreintes. Sans dépendance de parsing lourde : on extrait les
/// chaînes imprimables du DER, ce qui suffit à y chercher un « Proxmox » ou un
/// « Synology ».
fn describe_cert(cert: &CertificateDer<'_>) -> String {
    let mut out = String::new();
    let der = cert.as_ref();
    let mut i = 0;
    while i < der.len() {
        // Chaînes ASN.1 courantes : PrintableString (0x13), UTF8String (0x0c),
        // IA5String (0x16, SAN DNS). Tag, longueur courte, contenu.
        if matches!(der[i], 0x13 | 0x0c | 0x16) && i + 1 < der.len() {
            let len = der[i + 1] as usize;
            if len > 0 && len < 128 && i + 2 + len <= der.len() {
                if let Ok(text) = std::str::from_utf8(&der[i + 2..i + 2 + len])
                    && text.chars().all(|c| !c.is_control())
                {
                    out.push_str(text);
                    out.push(' ');
                }
                i += 2 + len;
                continue;
            }
        }
        i += 1;
    }
    out.trim().to_string()
}

fn tls_config() -> Arc<ClientConfig> {
    use std::sync::OnceLock;
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            // Fournisseur explicite (ring), comme la sonde TLS : ne dépend pas d'un
            // fournisseur par défaut installé dans le processus.
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let mut config = ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .expect("versions TLS par défaut")
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(AcceptAll))
                .with_no_client_auth();
            // ALPN http/1.1 : certains serveurs n'offrent la page qu'avec.
            config.alpn_protocols = vec![b"http/1.1".to_vec()];
            Arc::new(config)
        })
        .clone()
}

/// Vérificateur qui accepte tout : la découverte identifie, elle ne protège
/// aucun canal (même raison que la sonde d'expiration TLS).
#[derive(Debug)]
struct AcceptAll;

impl ServerCertVerifier for AcceptAll {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_http_reads_status_headers_and_title() {
        let raw = b"HTTP/1.1 200 OK\r\nServer: pve-api-daemon/3.0\r\nContent-Type: text/html\r\n\r\n<html><head><title>Proxmox Virtual Environment</title></head></html>";
        let response = parse_http(raw).expect("réponse");
        assert_eq!(response.status, 200);
        assert_eq!(response.header("server"), Some("pve-api-daemon/3.0"));
        assert_eq!(response.title.as_deref(), Some("Proxmox Virtual Environment"));
    }

    #[test]
    fn parse_http_tolerates_lf_only_and_missing_body() {
        let raw = b"HTTP/1.0 401 Unauthorized\nWWW-Authenticate: Basic\n\n";
        let response = parse_http(raw).expect("réponse");
        assert_eq!(response.status, 401);
        assert_eq!(response.header("www-authenticate"), Some("Basic"));
        assert_eq!(response.title, None);
    }

    #[test]
    fn title_is_whitespace_normalised() {
        assert_eq!(
            extract_title("<TITLE>\n  Home -\tCUPS  </TITLE>"),
            Some("Home - CUPS".to_string())
        );
        assert_eq!(extract_title("<html>no title</html>"), None);
    }

    #[test]
    fn printable_keeps_text_and_tames_binary() {
        assert_eq!(printable(b"SSH-2.0-OpenSSH_9.6"), "SSH-2.0-OpenSSH_9.6");
        assert!(printable(&[0x0a, 0x00, 0x35, 0x2e]).contains("5."));
    }
}
