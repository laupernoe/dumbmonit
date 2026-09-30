//! Accès à l'API REST de RouterOS (`/rest`, RouterOS 7.1 et suivants).
//!
//! Tout ce qui touche au réseau est ici. L'authentification est un compte
//! RouterOS en HTTP « basic », renvoyé à chaque requête : l'API REST n'a pas
//! de session.
//!
//! RouterOS range les refus de politique parmi les erreurs internes : un
//! groupe sans `api` répond `500 {"detail":"std failure: not allowed (9)"}`,
//! un groupe sans `read` `500 {"detail":"not enough permissions (9)"}`, et un
//! groupe sans `rest-api` un simple 401. Relevé sur RouterOS 7.23.7 avec un
//! compte neuf par combinaison — une politique changée sur un groupe existant
//! n'est pas toujours prise en compte tout de suite. Les trois sont traduits
//! en `ProbeError::Auth`, avec la politique qui manque.

use std::time::Duration;

use dumbmonit_proto::{Credential, ProbeError};
use reqwest::StatusCode;
use serde::Deserialize;
use serde::de::DeserializeOwned;

/// Longueur maximale du corps d'erreur repris dans un message.
const MAX_ERROR_BODY: usize = 200;

/// Les politiques du groupe de lecture, telles que la notice les fait poser.
pub const POLICIES: &str = "read,api,rest-api";

pub struct Login {
    username: String,
    password: String,
}

impl std::fmt::Debug for Login {
    // Aucun secret ne sort d'ici, pas même dans une trace de débogage.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Login({}, …)", self.username)
    }
}

impl Login {
    pub fn from_credential(credential: &Credential) -> Result<Self, ProbeError> {
        match credential {
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                Ok(Self { username: username.trim().to_string(), password: password.clone() })
            }
            other => Err(ProbeError::Config(format!(
                "RouterOS expects the user name and password of a read-only user, configured: \
                 {other}"
            ))),
        }
    }
}

/// Le corps d'erreur de RouterOS : `{"error":500,"message":"…","detail":"…"}`.
#[derive(Debug, Default, Deserialize)]
struct ErrorBody {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    detail: Option<String>,
}

pub struct Client {
    http: reqwest::Client,
    base_url: String,
    login: Login,
    timeout: Duration,
}

/// Ce que devient une réponse qui n'est pas un succès.
#[derive(Debug)]
pub enum Failure {
    /// Le menu n'existe pas sur ce routeur : `/system/routerboard` sur un CHR,
    /// un menu apparu dans une version plus récente.
    Missing,
    Error(ProbeError),
}

impl From<ProbeError> for Failure {
    fn from(error: ProbeError) -> Self {
        Self::Error(error)
    }
}

impl Failure {
    pub fn into_error(self, path: &str) -> ProbeError {
        match self {
            Self::Missing => {
                ProbeError::Protocol(format!("RouterOS has no {path} menu on this device"))
            }
            Self::Error(error) => error,
        }
    }
}

impl Client {
    pub fn new(http: reqwest::Client, base_url: String, login: Login, timeout: Duration) -> Self {
        Self { http, base_url, login, timeout }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/rest{path}", self.base_url)
    }

    /// Un `GET` JSON. Rien d'autre ne part vers le routeur.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Failure> {
        let response = self
            .http
            .get(self.url(path))
            .basic_auth(&self.login.username, Some(&self.login.password))
            .header(reqwest::header::ACCEPT, "application/json")
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|error| self.transport(&error, path))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| self.transport(&error, path))?;
        if !status.is_success() {
            return Err(self.status_error(status, &body, path));
        }
        serde_json::from_str(&body).map_err(|error| {
            Failure::Error(ProbeError::Protocol(format!(
                "Unexpected response from /rest{path}: {error}"
            )))
        })
    }

    /// Traduit un code HTTP en erreur de sonde : un identifiant ou une
    /// politique refusés ne doivent jamais passer pour « routeur injoignable ».
    pub fn status_error(&self, status: StatusCode, body: &str, path: &str) -> Failure {
        let parsed: ErrorBody = serde_json::from_str(body).unwrap_or_default();
        let detail = parsed.detail.or(parsed.message).unwrap_or_else(|| {
            let excerpt: String = body.chars().take(MAX_ERROR_BODY).collect();
            excerpt.trim().to_string()
        });
        let lower = detail.to_ascii_lowercase();
        Failure::Error(match status {
            StatusCode::UNAUTHORIZED => ProbeError::Auth(format!(
                "RouterOS refused the credentials on /rest{path} (401). Check the user name and \
                 password, and that the user's group has the rest-api policy ({POLICIES})."
            )),
            StatusCode::FORBIDDEN => ProbeError::Auth(format!(
                "RouterOS refused /rest{path} (403): check the user's allowed address and the \
                 address list of the www-ssl service."
            )),
            _ if lower.contains("not allowed") => ProbeError::Auth(format!(
                "RouterOS accepted the user but refused /rest{path}: its group lacks the api \
                 policy. Give the group {POLICIES}."
            )),
            _ if lower.contains("not enough permissions") => ProbeError::Auth(format!(
                "RouterOS accepted the user but refused /rest{path}: its group lacks the read \
                 policy. Give the group {POLICIES}."
            )),
            StatusCode::BAD_REQUEST if lower.contains("no such command") => {
                return Failure::Missing;
            }
            StatusCode::NOT_FOUND => ProbeError::Protocol(format!(
                "No REST API at {}/rest: RouterOS 7.1 or later is required, and the address must \
                 be the router's web service (www-ssl, or www).",
                self.base_url
            )),
            status if status.is_server_error() => ProbeError::Unreachable(format!(
                "RouterOS answered {status} on /rest{path}: {detail}"
            )),
            status => {
                ProbeError::Protocol(format!("RouterOS answered {status} on /rest{path}: {detail}"))
            }
        })
    }

    fn transport(&self, error: &reqwest::Error, path: &str) -> Failure {
        Failure::Error(if error.is_timeout() {
            ProbeError::Timeout(self.timeout)
        } else if error.is_builder() {
            ProbeError::Config(format!("Malformed request to /rest{path}: {error}"))
        } else {
            ProbeError::Unreachable(format!("/rest{path}: {error}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> Client {
        Client::new(
            reqwest::Client::new(),
            "https://router.lan:443".into(),
            Login { username: "dumbmonit".into(), password: "secret".into() },
            Duration::from_secs(5),
        )
    }

    fn error(status: StatusCode, body: &str) -> ProbeError {
        client().status_error(status, body, "/system/resource").into_error("/system/resource")
    }

    /// Les trois refus relevés sur RouterOS 7.23.7, un compte neuf par groupe.
    #[test]
    fn les_refus_de_politique_sont_des_erreurs_d_identifiant() {
        let sans_rest =
            error(StatusCode::UNAUTHORIZED, r#"{"error":401,"message":"Unauthorized"}"#);
        assert!(matches!(sans_rest, ProbeError::Auth(ref m) if m.contains("rest-api")));
        let sans_api = error(
            StatusCode::INTERNAL_SERVER_ERROR,
            r#"{"detail":"std failure: not allowed (9)","error":500,"message":"Internal Server Error"}"#,
        );
        assert!(matches!(sans_api, ProbeError::Auth(ref m) if m.contains("api policy")));
        let sans_read = error(
            StatusCode::INTERNAL_SERVER_ERROR,
            r#"{"detail":"not enough permissions (9)","error":500,"message":"Internal Server Error"}"#,
        );
        assert!(matches!(sans_read, ProbeError::Auth(ref m) if m.contains("read policy")));
        for e in [sans_rest, sans_api, sans_read] {
            assert!(!e.means_down());
        }
    }

    #[test]
    fn un_menu_absent_n_est_pas_une_erreur_de_transport() {
        let failure = client().status_error(
            StatusCode::BAD_REQUEST,
            r#"{"detail":"no such command or directory (routerboard)","error":400,"message":"Bad Request"}"#,
            "/system/routerboard",
        );
        assert!(matches!(failure, Failure::Missing));
    }

    #[test]
    fn une_erreur_serveur_ordinaire_compte_comme_une_indisponibilite() {
        assert!(error(StatusCode::SERVICE_UNAVAILABLE, "").means_down());
        let old = error(StatusCode::NOT_FOUND, "");
        assert!(matches!(old, ProbeError::Protocol(ref m) if m.contains("7.1")));
    }

    #[test]
    fn seul_un_compte_est_accepte_et_le_mot_de_passe_ne_sort_pas() {
        let login = Login::from_credential(&Credential::UsernamePassword {
            username: " dumbmonit ".into(),
            password: "secret".into(),
        })
        .unwrap();
        assert_eq!(login.username, "dumbmonit");
        assert!(!format!("{login:?}").contains("secret"));
        assert!(Login::from_credential(&Credential::None).is_err());
        assert!(Login::from_credential(&Credential::ApiToken { token: "t".into() }).is_err());
    }
}
