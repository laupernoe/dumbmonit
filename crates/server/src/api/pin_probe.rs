//! Quel certificat les machines verront-elles à l'adresse que l'administrateur
//! a ouverte dans son navigateur ?
//!
//! L'empreinte `sha256//…` d'une commande d'installation (ou de l'assistant du
//! paquet Synology) est celle du certificat autosigné de DumbMonit. Or derrière
//! un mandataire inverse (DSM, Traefik, Nginx…) ou une redirection de port,
//! l'adresse ouverte présente un *autre* certificat : l'agent, qui n'accepte que
//! la clé épinglée, le refuserait. Le serveur se compose donc lui-même à cette
//! adresse, relève le certificat feuille présenté, et regarde s'il est de
//! confiance (autorités du système) ou s'il est le nôtre.
//!
//! Garde anti-SSRF : on ne sonde jamais que l'hôte et le port que la requête
//! HTTP elle-même a visés (`Host` ou `X-Forwarded-Host`), jamais une adresse
//! que l'appelant écrirait ailleurs. Le sondage est borné à quelques secondes
//! et ne bloque jamais : au moindre échec, le comportement d'origine demeure.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::HeaderMap;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use crate::web_tls::cert::certificate_key_pin;

/// Durée maximale de tout le sondage (résolution, connexion, poignée de main,
/// vérification par les autorités).
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// D'où vient l'empreinte affichée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinSource {
    /// Le certificat autosigné de DumbMonit, confirmé à l'adresse ouverte.
    Own,
    /// Un autre certificat autosigné (mandataire inverse) : son empreinte.
    Proxy,
    /// Un certificat de confiance à cette adresse : aucune empreinte à saisir.
    Trusted,
}

impl PinSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Own => "own",
            Self::Proxy => "proxy",
            Self::Trusted => "trusted",
        }
    }
}

/// Ce que le sondage a relevé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    /// Épingle de la clé du certificat feuille présenté.
    pub pin: String,
    /// Les autorités du système l'acceptent.
    pub trusted: bool,
}

/// Décision pure : l'empreinte à écrire, et son origine, d'après ce que le
/// sondage a vu et l'empreinte propre de DumbMonit.
///
/// - sondage impossible : comportement d'origine, l'empreinte propre ;
/// - certificat de confiance : aucune empreinte (seulement si on en aurait
///   donné une, pour ne pas commenter un certificat fourni) ;
/// - même clé que la nôtre : l'empreinte propre, confirmée ;
/// - autre certificat non fiable : la sienne, celle du mandataire ;
/// - sans empreinte propre (certificat fourni) et non fiable : inchangé.
pub fn decide(seen: Option<&Seen>, own: Option<String>) -> (Option<String>, Option<PinSource>) {
    let Some(seen) = seen else { return (own, None) };
    if seen.trusted {
        let source = own.is_some().then_some(PinSource::Trusted);
        return (None, source);
    }
    match own {
        Some(own) if own == seen.pin => (Some(own), Some(PinSource::Own)),
        Some(_) => (Some(seen.pin.clone()), Some(PinSource::Proxy)),
        None => (None, None),
    }
}

/// `hôte:port` d'une autorité (sans schéma ni chemin), nom en minuscules et
/// crochets d'IPv6 retirés ; `default` si le port manque.
fn host_port(authority: &str, default: u16) -> Option<(String, u16)> {
    let authority = authority.trim();
    let (host, port) = match authority.strip_prefix('[') {
        Some(inside) => {
            let (host, after) = inside.split_once(']')?;
            (host, after.strip_prefix(':'))
        }
        None => match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        },
    };
    if host.is_empty() {
        return None;
    }
    let port = match port {
        Some(port) => port.parse().ok()?,
        None => default,
    };
    Some((host.to_ascii_lowercase(), port))
}

/// Autorité d'une URL de base `https://hôte[:port]/…`.
fn base_authority(base_url: &str) -> Option<(String, u16)> {
    let rest = base_url.split_once("://").map_or(base_url, |(_, rest)| rest);
    host_port(rest.split('/').next()?, 443)
}

/// L'hôte et le port à sonder : ceux de `base_url`, à condition que la requête
/// HTTP entrante ait visé la même adresse (`X-Forwarded-Host`, sinon `Host`).
/// C'est la garde contre le SSRF : une URL de base écrite à la main, qui n'est
/// pas l'adresse par laquelle l'administrateur est venu, n'est jamais sondée.
pub fn probe_target(base_url: &str, headers: &HeaderMap) -> Option<(String, u16)> {
    let target = base_authority(base_url)?;
    let header = |name: &str| {
        let value = headers.get(name)?.to_str().ok()?;
        // Plusieurs mandataires en chaîne : le premier est celui du client.
        value.split(',').next().map(str::to_string)
    };
    let seen = header("x-forwarded-host").or_else(|| header("host"))?;
    // Le navigateur est venu en HTTPS : sans port, c'est 443.
    (host_port(&seen, 443)? == target).then_some(target)
}

/// Vérificateur qui accepte tout et retient le certificat feuille : la
/// connexion est refermée aussitôt.
#[derive(Debug)]
struct Capture {
    leaf: Mutex<Option<Vec<u8>>>,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for Capture {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if let Ok(mut leaf) = self.leaf.lock() {
            *leaf = Some(end_entity.as_ref().to_vec());
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

/// Poignée de main permissive : le certificat feuille présenté, en DER.
async fn leaf_certificate(host: &str, port: u16) -> Option<Vec<u8>> {
    let tcp = TcpStream::connect((host, port)).await.ok()?;
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let capture = Arc::new(Capture { leaf: Mutex::new(None), provider: provider.clone() });
    let config = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
        .ok()?
        .dangerous()
        .with_custom_certificate_verifier(capture.clone())
        .with_no_client_auth();
    let name = ServerName::try_from(host.to_string()).ok()?;
    TlsConnector::from(Arc::new(config)).connect(name, tcp).await.ok()?;
    capture.leaf.lock().ok()?.clone()
}

/// Les autorités du système acceptent-elles ce que l'adresse présente ? Toute
/// réponse HTTP, même une erreur, prouve que la poignée de main est passée.
async fn trusted_by_system(host: &str, port: u16) -> bool {
    let host = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
    let Ok(client) = reqwest::Client::builder()
        .timeout(PROBE_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
    else {
        return false;
    };
    client.head(format!("https://{host}:{port}/")).send().await.is_ok()
}

/// Sonde `hôte:port` : certificat feuille et confiance. `None` au moindre échec
/// ou dépassement de délai.
pub async fn probe(host: &str, port: u16) -> Option<Seen> {
    let work = async {
        let pin = certificate_key_pin(&leaf_certificate(host, port).await?)?;
        let trusted = trusted_by_system(host, port).await;
        Some(Seen { pin, trusted })
    };
    tokio::time::timeout(PROBE_TIMEOUT, work).await.ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWN: &str = "sha256//AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    const OTHER: &str = "sha256//BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=";

    fn seen(pin: &str, trusted: bool) -> Seen {
        Seen { pin: pin.into(), trusted }
    }

    #[test]
    fn without_a_probe_the_own_pin_is_kept() {
        assert_eq!(decide(None, Some(OWN.into())), (Some(OWN.into()), None));
        assert_eq!(decide(None, None), (None, None));
    }

    #[test]
    fn our_own_certificate_is_confirmed() {
        let got = decide(Some(&seen(OWN, false)), Some(OWN.into()));
        assert_eq!(got, (Some(OWN.into()), Some(PinSource::Own)));
    }

    #[test]
    fn another_self_signed_certificate_is_the_proxys() {
        let got = decide(Some(&seen(OTHER, false)), Some(OWN.into()));
        assert_eq!(got, (Some(OTHER.into()), Some(PinSource::Proxy)));
    }

    #[test]
    fn a_trusted_certificate_needs_no_pin() {
        let got = decide(Some(&seen(OTHER, true)), Some(OWN.into()));
        assert_eq!(got, (None, Some(PinSource::Trusted)));
        // Certificat fourni (aucune empreinte propre) : rien à commenter.
        assert_eq!(decide(Some(&seen(OTHER, true)), None), (None, None));
        // Fourni mais inconnu des autorités : inchangé.
        assert_eq!(decide(Some(&seen(OTHER, false)), None), (None, None));
    }

    #[test]
    fn the_source_has_a_stable_name() {
        assert_eq!(PinSource::Own.as_str(), "own");
        assert_eq!(PinSource::Proxy.as_str(), "proxy");
        assert_eq!(PinSource::Trusted.as_str(), "trusted");
    }

    #[test]
    fn authorities_are_read_with_their_default_port() {
        assert_eq!(host_port("NAS.lan:8090", 443), Some(("nas.lan".into(), 8090)));
        assert_eq!(host_port("nas.lan", 443), Some(("nas.lan".into(), 443)));
        assert_eq!(host_port("[fd00::1]:8080", 443), Some(("fd00::1".into(), 8080)));
        assert_eq!(host_port("[fd00::1]", 443), Some(("fd00::1".into(), 443)));
        assert_eq!(host_port("nas:abc", 443), None);
        assert_eq!(host_port("", 443), None);
        assert_eq!(base_authority("https://nas.lan:8090/x"), Some(("nas.lan".into(), 8090)));
        assert_eq!(base_authority("https://nas.lan"), Some(("nas.lan".into(), 443)));
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        map
    }

    #[test]
    fn only_the_address_the_request_was_sent_to_is_probed() {
        let same = headers(&[("host", "nas.lan:8090")]);
        assert_eq!(
            probe_target("https://nas.lan:8090", &same),
            Some(("nas.lan".to_string(), 8090))
        );
        // Une URL de base écrite ailleurs que l'adresse visitée : jamais sondée.
        assert_eq!(probe_target("https://169.254.169.254:443", &same), None);
        assert_eq!(probe_target("https://nas.lan:9999", &same), None);
        // Derrière un mandataire : l'hôte transmis fait foi, premier de la chaîne.
        let proxied = headers(&[("host", "127.0.0.1:8080"), ("x-forwarded-host", "monit.lan, a")]);
        let forwarded = Some(("monit.lan".to_string(), 443));
        assert_eq!(probe_target("https://monit.lan", &proxied), forwarded);
        assert_eq!(probe_target("https://127.0.0.1:8080", &proxied), None);
        assert_eq!(probe_target("https://nas.lan", &HeaderMap::new()), None);
    }
}
