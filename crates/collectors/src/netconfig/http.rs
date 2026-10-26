//! Relevé d'une configuration par une API HTTPS (OPNsense, Palo Alto).
//!
//! Le chemin vient du profil, sans rien y interpoler ; l'identifiant part en
//! authentification basique ou dans l'en-tête que le profil nomme. Le client
//! partagé ne suit les redirections que vers le même hôte : l'identifiant ne
//! quitte jamais sa destination.

use std::time::Duration;

use dumbmonit_proto::ProbeError;

use super::{MAX_CONFIG_BYTES, append_bounded, too_large};
use super::profile::{HttpAuth, HttpSpec};

/// Identifiant d'une requête : couple identifiant / secret, ou jeton.
pub enum HttpCredential<'a> {
    Basic { username: &'a str, password: &'a str },
    Token(&'a str),
}

fn request_error(error: reqwest::Error, timeout: Duration) -> ProbeError {
    if error.is_timeout() {
        ProbeError::Timeout(timeout)
    } else if crate::http::is_certificate_error(&error) {
        ProbeError::Config(format!(
            "the device's certificate was refused ({error}); enable \"Accept self-signed \
             certificate\" only for a device of your own"
        ))
    } else if error.is_connect() {
        ProbeError::Unreachable(error.to_string())
    } else {
        ProbeError::Protocol(error.to_string())
    }
}

/// `GET https://host:port<path>` ; rend le corps, borné à [`MAX_CONFIG_BYTES`].
pub async fn fetch(
    host: &str,
    port: u16,
    spec: &HttpSpec,
    credential: &HttpCredential<'_>,
    insecure_tls: bool,
    timeout: Duration,
) -> Result<String, ProbeError> {
    // L'hôte est l'adresse saisie : rien qui puisse en faire une autre adresse
    // web (identifiant `user@`, chemin, requête) et emmener l'identifiant
    // ailleurs que vers l'équipement.
    if host.is_empty() || host.chars().any(|c| "/@?#\\%".contains(c) || c.is_whitespace()) {
        return Err(ProbeError::Config(format!("invalid device address \"{host}\"")));
    }
    // Une IPv6 s'écrit entre crochets dans une adresse web.
    let host = if host.contains(':') { format!("[{host}]") } else { host.to_owned() };
    let url = format!("https://{host}:{port}{}", spec.path);
    let client = crate::http::client(insecure_tls)?;
    let mut request = client.get(&url).timeout(timeout);
    request = match (spec.auth, credential) {
        (HttpAuth::Basic, HttpCredential::Basic { username, password }) => {
            request.basic_auth(username, Some(password))
        }
        (HttpAuth::Header, HttpCredential::Token(token)) => {
            let name = spec.header.as_deref().unwrap_or("Authorization");
            request.header(name, *token)
        }
        (HttpAuth::Basic, HttpCredential::Token(_)) => {
            return Err(ProbeError::Config(
                "this profile needs a user name and password (API key and secret)".into(),
            ));
        }
        (HttpAuth::Header, HttpCredential::Basic { .. }) => {
            return Err(ProbeError::Config("this profile needs an API token".into()));
        }
    };
    let mut response = request.send().await.map_err(|e| request_error(e, timeout))?;
    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(ProbeError::Auth(format!("the device answered HTTP {status}")));
    }
    if !status.is_success() {
        return Err(ProbeError::Protocol(format!("the device answered HTTP {status}")));
    }
    // Une taille annoncée trop grande est refusée d'emblée, sans rien lire.
    if response.content_length().is_some_and(|length| length > MAX_CONFIG_BYTES as u64) {
        return Err(too_large());
    }
    let mut body: Vec<u8> = Vec::new();
    let mut total = 0usize;
    while let Some(chunk) = response.chunk().await.map_err(|e| request_error(e, timeout))? {
        append_bounded(&mut body, &mut total, &chunk)?;
    }
    let text = String::from_utf8_lossy(&body).into_owned();
    // PAN-OS répond 200 avec `status="error"` quand la clé est refusée ou la
    // commande inconnue.
    let head: String = text.chars().take(300).collect();
    if head.contains("status=\"error\"") {
        return Err(ProbeError::Auth("the device refused the request (status=\"error\")".into()));
    }
    Ok(text.replace("\r\n", "\n"))
}
