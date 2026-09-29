//! L'API XML de MDaemon (`/MdMgmtWS/`), servie par l'administration à distance.
//!
//! Une requête est un document `<MDaemon><API><Request>…` envoyé en POST avec
//! une authentification HTTP Basic, l'utilisateur étant l'adresse de messagerie
//! complète du compte. Toute réponse, quelle que soit l'opération, porte la même
//! enveloppe — c'est la seule partie du format que MDaemon Technologies documente
//! publiquement, et la seule que ce module lit :
//!
//! ```xml
//! <MDaemon>
//!   <API productversion="23.5.1" serviceversion="23.5.1.6">
//!     <Response version="23.5" et="0.026282">
//!       <Status id="0" value="0x00000000" message="The operation completed successfully."/>
//!     </Response>
//!   </API>
//! </MDaemon>
//! ```
//!
//! L'opération appelée est `GetVersionInfo` : elle ne lit ni ne change aucun
//! compte. Son contenu propre n'est pas interprété ; `productversion` donne la
//! version, `Status` dit si le compte a eu le droit de l'appeler.
//!
//! Aucun type de ce module ne dérive `Debug` quand il porte un secret.

use std::time::{Duration, Instant};

use dumbmonit_proto::ProbeError;
use reqwest::StatusCode;

/// Opération appelée : lecture seule, sans paramètre.
pub const OPERATION: &str = "GetVersionInfo";

/// Taille maximale de réponse acceptée : l'enveloppe tient en quelques
/// centaines d'octets.
const MAX_BODY: usize = 256 * 1024;

/// Corps de la requête. `version` est celle du format de requête, pas celle du
/// serveur : `20.0.0` est acceptée par toutes les versions qui ont l'API.
pub fn request_body(operation: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <MDaemon>\n  <API>\n    <Request version=\"20.0.0\" echo=\"0\" verbose=\"0\">\n      \
         <Operation>{operation}</Operation>\n      <Parameters/>\n    </Request>\n  </API>\n\
         </MDaemon>\n"
    )
}

/// L'enveloppe d'une réponse.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Envelope {
    pub product_version: Option<String>,
    pub service_version: Option<String>,
    /// Code de retour, `0x00000000` en cas de succès.
    pub status_value: Option<String>,
    pub status_message: Option<String>,
}

impl Envelope {
    /// Vrai si l'opération a abouti. Un code illisible compte comme un échec.
    pub fn succeeded(&self) -> bool {
        self.status_value
            .as_deref()
            .and_then(|raw| {
                let hex = raw.trim().trim_start_matches("0x").trim_start_matches("0X");
                u32::from_str_radix(hex, 16).ok()
            })
            .is_some_and(|code| code == 0)
    }
}

/// Lit l'enveloppe. Un document sans élément `<API>` n'est pas une réponse de
/// l'API XML — typiquement la page d'aide HTML d'un autre service sur ce port.
pub fn parse_envelope(body: &str) -> Result<Envelope, ProbeError> {
    let api = find_tag(body, "API").ok_or_else(|| {
        ProbeError::Protocol(
            "The answer is not an MDaemon XML API response (no <API> element)".to_string(),
        )
    })?;
    let mut envelope = Envelope {
        product_version: attribute(api.1, "productversion"),
        service_version: attribute(api.1, "serviceversion"),
        ..Envelope::default()
    };
    if let Some((_, status)) = find_tag(&body[api.0..], "Status") {
        envelope.status_value = attribute(status, "value");
        envelope.status_message = attribute(status, "message");
    }
    Ok(envelope)
}

/// Trouve la première balise ouvrante `<name …>` et renvoie la position qui la
/// suit et le texte de ses attributs.
fn find_tag<'a>(body: &'a str, name: &str) -> Option<(usize, &'a str)> {
    let needle = format!("<{name}");
    let mut from = 0;
    while let Some(found) = body[from..].find(&needle) {
        let start = from + found + needle.len();
        let next = body[start..].chars().next()?;
        if next.is_whitespace() || next == '>' || next == '/' {
            // La fin de balise est le premier `>` hors guillemets.
            let mut quote: Option<char> = None;
            for (offset, c) in body[start..].char_indices() {
                match (quote, c) {
                    (Some(q), c) if c == q => quote = None,
                    (Some(_), _) => {}
                    (None, '"' | '\'') => quote = Some(c),
                    (None, '>') => {
                        let end = start + offset;
                        return Some((end + 1, &body[start..end]));
                    }
                    _ => {}
                }
            }
            return None;
        }
        from = start;
    }
    None
}

/// Valeur d'un attribut, entités décodées. Le nom est comparé exactement.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let mut rest = tag;
    loop {
        let trimmed = rest.trim_start();
        let eq = trimmed.find('=')?;
        let key = trimmed[..eq].trim();
        let after = trimmed[eq + 1..].trim_start();
        let quote = after.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let close = after[1..].find(quote)?;
        let value = &after[1..1 + close];
        if key == name {
            return Some(decode_entities(value));
        }
        rest = &after[close + 2..];
    }
}

fn decode_entities(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let Some(semi) = tail.find(';').filter(|&semi| semi <= 10) else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let entity = &tail[1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &tail[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Client de l'API XML. Ne dérive pas `Debug` : il porte le mot de passe.
pub struct XmlApiClient {
    pub http: reqwest::Client,
    /// URL complète du service : `https://mail.lan:444/MdMgmtWS/`.
    pub url: String,
    pub username: String,
    pub password: String,
    pub timeout: Duration,
}

/// Réponse d'un appel : l'enveloppe et le temps de réponse.
pub struct Call {
    pub envelope: Envelope,
    pub seconds: f64,
}

impl XmlApiClient {
    pub async fn call(&self, operation: &str) -> Result<Call, ProbeError> {
        let started = Instant::now();
        let response = self
            .http
            .post(&self.url)
            .timeout(self.timeout)
            .basic_auth(&self.username, Some(&self.password))
            .header(reqwest::header::CONTENT_TYPE, "text/xml; charset=utf-8")
            .body(request_body(operation))
            .send()
            .await
            .map_err(|error| map_transport(&error, "the MDaemon XML API", self.timeout))?;
        let status = response.status();
        if !status.is_success() {
            // Le corps d'un refus n'est jamais repris : il pourrait refléter
            // l'identifiant soumis.
            return Err(status_error(status));
        }
        let body = response
            .bytes()
            .await
            .map_err(|error| map_transport(&error, "the MDaemon XML API", self.timeout))?;
        if body.len() > MAX_BODY {
            return Err(ProbeError::Protocol(
                "The MDaemon XML API answer is unexpectedly large".to_string(),
            ));
        }
        let envelope = parse_envelope(&String::from_utf8_lossy(&body))?;
        Ok(Call { envelope, seconds: started.elapsed().as_secs_f64() })
    }
}

fn status_error(status: StatusCode) -> ProbeError {
    match status {
        StatusCode::UNAUTHORIZED => ProbeError::Auth(
            "MDaemon refused the account. The user name must be the full email address, \
             the account must not be frozen or disabled, and it must be allowed to use \
             the XML API"
                .to_string(),
        ),
        StatusCode::FORBIDDEN => ProbeError::Auth(
            "MDaemon refused the XML API request from this address. Since MDaemon 24, \
             the XML API only answers the addresses allowed under its Address \
             Restrictions: add the address DumbMonit connects from"
                .to_string(),
        ),
        StatusCode::NOT_FOUND => ProbeError::Config(
            "No XML API at this address: check the Remote Administration port \
             (api_port) and whether it serves HTTPS (api_tls)"
                .to_string(),
        ),
        s if s.is_server_error() => {
            ProbeError::Unreachable(format!("MDaemon XML API: HTTP {}", s.as_u16()))
        }
        s => ProbeError::Protocol(format!("MDaemon XML API: HTTP {}", s.as_u16())),
    }
}

/// Seuls `Timeout` et `Unreachable` comptent comme une panne. Un certificat
/// refusé est une affaire de configuration.
pub fn map_transport(error: &reqwest::Error, what: &str, timeout: Duration) -> ProbeError {
    if error.is_timeout() {
        return ProbeError::Timeout(timeout);
    }
    let chain = cause_chain(error);
    let lower = chain.to_lowercase();
    if ["certificate", "unknownissuer", "notvalidfor", "certexpired"]
        .iter()
        .any(|motif| lower.contains(motif))
    {
        return ProbeError::Config(format!(
            "TLS certificate of {what} rejected. Install a trusted certificate on the \
             server, or tick \"Accept an unverifiable certificate\" in the options to \
             knowingly accept it"
        ));
    }
    if error.is_decode() {
        return ProbeError::Protocol(format!("Unreadable response from {what}"));
    }
    ProbeError::Unreachable(format!("{what}: {chain}"))
}

fn cause_chain(error: &reqwest::Error) -> String {
    let mut message = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = include_str!("testdata/xmlapi-status-ok.xml");
    const ERROR: &str = include_str!("testdata/xmlapi-status-error.xml");
    const OK_FR: &str = include_str!("testdata/xmlapi-status-ok-fr.xml");

    #[test]
    fn l_enveloppe_documentee_donne_la_version_et_le_verdict() {
        let envelope = parse_envelope(OK).unwrap();
        assert_eq!(envelope.product_version.as_deref(), Some("23.5.1"));
        assert_eq!(envelope.service_version.as_deref(), Some("23.5.1.6"));
        assert_eq!(envelope.status_value.as_deref(), Some("0x00000000"));
        assert_eq!(
            envelope.status_message.as_deref(),
            Some("The operation completed successfully.")
        );
        assert!(envelope.succeeded());
    }

    #[test]
    fn un_code_non_nul_est_un_echec_de_l_operation() {
        let envelope = parse_envelope(ERROR).unwrap();
        assert!(!envelope.succeeded());
        assert_eq!(envelope.status_value.as_deref(), Some("0x81420082"));
        assert_eq!(envelope.product_version.as_deref(), Some("23.5.1"));
    }

    #[test]
    fn les_entites_sont_decodees_et_la_langue_ne_compte_pas() {
        let envelope = parse_envelope(OK_FR).unwrap();
        assert!(envelope.succeeded(), "le verdict se lit sur le code, pas sur le message");
        assert_eq!(envelope.status_message.as_deref(), Some("L'opération a réussi."));
        assert_eq!(envelope.product_version.as_deref(), Some("23.5.0"));
    }

    #[test]
    fn une_page_qui_n_est_pas_l_api_est_une_erreur_de_protocole() {
        let html = "<html><head><title>MDaemon Remote Administration</title></head></html>";
        assert!(matches!(parse_envelope(html), Err(ProbeError::Protocol(_))));
        // `<APIs>` n'est pas `<API>`, et un statut absent n'est pas un succès.
        let envelope =
            parse_envelope("<MDaemon><APIs x=\"1\"/><API productversion='26.0.4'/></MDaemon>")
                .unwrap();
        assert_eq!(envelope.product_version.as_deref(), Some("26.0.4"));
        assert!(!envelope.succeeded());
    }

    #[test]
    fn un_chevron_entre_guillemets_ne_ferme_pas_la_balise() {
        let body = "<MDaemon><API productversion=\"a>b\" serviceversion=\"1\"><Response>\
                    <Status value=\"0x0\" message=\"x > y\"/></Response></API></MDaemon>";
        let envelope = parse_envelope(body).unwrap();
        assert_eq!(envelope.product_version.as_deref(), Some("a>b"));
        assert_eq!(envelope.status_message.as_deref(), Some("x > y"));
        assert!(envelope.succeeded());
    }

    #[test]
    fn la_requete_suit_le_format_documente() {
        let body = request_body(OPERATION);
        assert!(body.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
        assert!(body.contains("<Request version=\"20.0.0\" echo=\"0\" verbose=\"0\">"));
        assert!(body.contains("<Operation>GetVersionInfo</Operation>"));
        assert!(body.trim_end().ends_with("</MDaemon>"));
    }

    #[test]
    fn un_refus_est_une_erreur_d_authentification_et_pas_une_panne() {
        for status in [StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN] {
            let error = status_error(status);
            assert!(matches!(error, ProbeError::Auth(_)));
            assert!(!error.means_down());
        }
        assert!(status_error(StatusCode::SERVICE_UNAVAILABLE).means_down());
        assert!(matches!(status_error(StatusCode::NOT_FOUND), ProbeError::Config(_)));
    }
}
