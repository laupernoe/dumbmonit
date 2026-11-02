//! Le canal des agents est-il joignable à l'adresse que les machines vont
//! composer ?
//!
//! L'écran d'ajout d'un agent pose la question avant d'afficher la commande
//! d'installation : un port 8443 non publié par Docker (ou fermé par un
//! pare-feu) est la première cause d'une installation qui « ne joint pas le
//! serveur ». Le serveur se compose lui-même à cette adresse, mène la poignée
//! de main TLS jusqu'au bout et vérifie que c'est bien lui qui répond (son
//! autorité épinglée dans la chaîne) ; aucune donnée n'est échangée, rien de
//! secret n'est présenté.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use dumbmonit_proto::AGENT_TLS_SERVER_NAME;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use super::pki;

/// Délai de chaque étape (résolution, connexion, poignée de main).
const STEP_TIMEOUT: Duration = Duration::from_secs(5);

/// Ce que la tentative a montré.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Le canal répond, et c'est bien ce serveur.
    Ok,
    /// Le nom ne se résout pas depuis le serveur.
    Dns,
    /// Connexion refusée : port non publié, ou rien n'y écoute.
    Refused,
    /// Pas de réponse : pare-feu qui jette les paquets, mauvaise adresse.
    Timeout,
    /// Quelque chose répond, mais pas en TLS 1.3.
    Tls,
    /// Un autre service répond en TLS sur ce port (autre autorité).
    Foreign,
}

/// Vérificateur qui accepte tout et retient la chaîne présentée : la
/// connexion est refermée aussitôt, seule la comparaison des empreintes
/// compte.
#[derive(Debug)]
struct Capture {
    chain: Mutex<Vec<Vec<u8>>>,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for Capture {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if let Ok(mut chain) = self.chain.lock() {
            chain.push(end_entity.as_ref().to_vec());
            chain.extend(intermediates.iter().map(|cert| cert.as_ref().to_vec()));
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

/// Compose `host:port` comme le ferait un agent et dit ce qui répond.
pub async fn check(host: &str, port: u16, pins: &[String]) -> Outcome {
    let host = host.trim().trim_matches(['[', ']']);
    let addresses =
        match tokio::time::timeout(STEP_TIMEOUT, tokio::net::lookup_host((host, port))).await {
            Ok(Ok(found)) => found.collect::<Vec<_>>(),
            Ok(Err(_)) => return Outcome::Dns,
            Err(_) => return Outcome::Timeout,
        };
    let Some(address) = addresses.first().copied() else { return Outcome::Dns };
    let tcp = match tokio::time::timeout(STEP_TIMEOUT, TcpStream::connect(address)).await {
        Ok(Ok(tcp)) => tcp,
        // Refusée, ou réseau injoignable : dans les deux cas, rien n'écoute
        // là pour les agents.
        Ok(Err(_)) => return Outcome::Refused,
        Err(_) => return Outcome::Timeout,
    };
    let provider = super::provider();
    let capture = Arc::new(Capture { chain: Mutex::new(Vec::new()), provider: provider.clone() });
    let Ok(builder) = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
    else {
        return Outcome::Tls;
    };
    let config =
        builder.dangerous().with_custom_certificate_verifier(capture.clone()).with_no_client_auth();
    let Ok(name) = ServerName::try_from(AGENT_TLS_SERVER_NAME) else { return Outcome::Tls };
    let connector = TlsConnector::from(Arc::new(config));
    match tokio::time::timeout(STEP_TIMEOUT, connector.connect(name, tcp)).await {
        Ok(Ok(_tls)) => {}
        Ok(Err(_)) => return Outcome::Tls,
        Err(_) => return Outcome::Timeout,
    }
    let chain = capture.chain.lock().map(|chain| chain.clone()).unwrap_or_default();
    if chain.iter().any(|der| pins.contains(&pki::fingerprint(der))) {
        Outcome::Ok
    } else {
        Outcome::Foreign
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_closed_port_is_reported_as_refused() {
        // Un port tout juste libéré : plus rien n'y écoute.
        let port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        assert_eq!(check("127.0.0.1", port, &["x".into()]).await, Outcome::Refused);
    }

    #[tokio::test]
    async fn something_that_is_not_tls_is_reported_as_such() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            if let Ok((mut socket, _)) = listener.accept().await {
                let _ = socket.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
            }
        });
        assert_eq!(check("127.0.0.1", port, &["x".into()]).await, Outcome::Tls);
    }

    #[tokio::test]
    async fn an_unknown_name_is_a_resolution_failure() {
        // Selon le résolveur de la machine de test, l'échec est immédiat ou
        // arrive au bout du délai : les deux disent « injoignable ».
        let outcome = check("nom-inexistant.invalid", 8443, &[]).await;
        assert!(matches!(outcome, Outcome::Dns | Outcome::Timeout), "{outcome:?}");
    }
}
