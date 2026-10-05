//! Configuration TLS de la connexion LDAPS ou StartTLS.
//!
//! Le certificat d'un contrôleur de domaine est presque toujours émis par
//! l'autorité de l'entreprise (AD CS), que ni le magasin Mozilla ni celui du
//! système ne connaissent. Trois cas :
//!
//! * `ca_cert` renseigné : seule cette autorité est crue — le cas recommandé ;
//! * sinon, les racines Mozilla embarquées plus le magasin du système (où une
//!   autorité privée peut être montée en volume) ;
//! * `insecure_tls` : aucun contrôle, la connexion reste chiffrée mais n'importe
//!   qui sur le chemin peut se faire passer pour le contrôleur.

use std::sync::Arc;

use base64::Engine;
use dumbmonit_proto::ProbeError;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};

use super::options::Options;
use crate::uptime::tls::handshake::provider;

pub fn client_config(options: &Options) -> Result<Arc<ClientConfig>, ProbeError> {
    let builder = ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .map_err(|error| ProbeError::Config(format!("invalid TLS configuration: {error}")))?;
    let config = if options.insecure_tls {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerification))
            .with_no_client_auth()
    } else {
        builder.with_root_certificates(roots(options.ca_cert.as_deref())?).with_no_client_auth()
    };
    Ok(Arc::new(config))
}

fn roots(ca_cert: Option<&str>) -> Result<RootCertStore, ProbeError> {
    let mut store = RootCertStore::empty();
    if let Some(raw) = ca_cert {
        for der in parse_ca(raw)? {
            store
                .add(der)
                .map_err(|error| ProbeError::Config(format!("CA certificate rejected: {error}")))?;
        }
        return Ok(store);
    }
    store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let native = rustls_native_certs::load_native_certs();
    for der in native.certs {
        // Un certificat illisible du magasin système ne doit pas empêcher les
        // autres de servir.
        let _ = store.add(der);
    }
    Ok(store)
}

/// Lit un ou plusieurs certificats PEM, collés tels quels, sur une seule
/// ligne, ou encodés une fois de plus en base64 (`ca.crt` d'un secret).
pub fn parse_ca(raw: &str) -> Result<Vec<CertificateDer<'static>>, ProbeError> {
    let invalid = || {
        ProbeError::Config(
            "CA certificate: expected a PEM certificate (-----BEGIN CERTIFICATE-----), or its \
             base64 encoding"
                .to_string(),
        )
    };
    let raw = raw.trim();
    let pem = if raw.contains("-----BEGIN") {
        raw.to_string()
    } else {
        let compact: String = raw.split_whitespace().collect();
        let bytes =
            base64::engine::general_purpose::STANDARD.decode(compact).map_err(|_| invalid())?;
        String::from_utf8(bytes).map_err(|_| invalid())?
    };
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    let mut certificates = Vec::new();
    let mut rest = pem.as_str();
    while let Some(start) = rest.find(BEGIN) {
        let after = &rest[start + BEGIN.len()..];
        let end = after.find(END).ok_or_else(invalid)?;
        let body: String = after[..end].split_whitespace().collect();
        let der = base64::engine::general_purpose::STANDARD.decode(body).map_err(|_| invalid())?;
        certificates.push(CertificateDer::from(der));
        rest = &after[end + END.len()..];
    }
    if certificates.is_empty() {
        return Err(invalid());
    }
    Ok(certificates)
}

/// Accepte tout certificat. N'est employé que sur demande explicite
/// (`insecure_tls`).
#[derive(Debug)]
struct NoVerification;

impl ServerCertVerifier for NoVerification {
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
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        provider().signature_verification_algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Un certificat auto-signé quelconque : seul le découpage PEM est testé.
    const PEM: &str = "-----BEGIN CERTIFICATE-----\nMIIBszCCAVmgAwIBAgIUQ2Vy\ndGlmaWNhdA==\n-----END CERTIFICATE-----\n";

    #[test]
    fn un_pem_colle_tel_quel_sur_une_ligne_ou_en_base64() {
        assert_eq!(parse_ca(PEM).unwrap().len(), 1);
        let one_line = PEM.replace('\n', " ");
        assert_eq!(parse_ca(&one_line).unwrap().len(), 1);
        let encoded = base64::engine::general_purpose::STANDARD.encode(PEM);
        assert_eq!(parse_ca(&encoded).unwrap().len(), 1);
        let two = format!("{PEM}{PEM}");
        assert_eq!(parse_ca(&two).unwrap().len(), 2);
        assert!(parse_ca("pas un certificat !").is_err());
        assert!(parse_ca("-----BEGIN CERTIFICATE-----\nAAAA").is_err());
    }
}
