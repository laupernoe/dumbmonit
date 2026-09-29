//! Accès HTTP aux quatre serveurs.
//!
//! Tout ce qui touche au réseau est ici ; le reste ne manipule que du texte ou
//! des structures déjà désérialisées, et se teste donc sans serveur en face.
//!
//! L'authentification est celle du proxy ou du serveur : rien, un couple
//! utilisateur / mot de passe en HTTP « basic » (VictoriaMetrics et VictoriaLogs
//! avec `-httpAuth.*`, un Loki derrière nginx, un compte Graylog), ou un jeton
//! « bearer » (vmauth, un proxy OIDC). Graylog est l'exception : son jeton
//! d'accès voyage en « basic », le jeton comme nom d'utilisateur et le mot
//! `token` comme mot de passe.

use std::time::Duration;

use dumbmonit_proto::{Credential, ProbeError};
use reqwest::StatusCode;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Longueur maximale du corps d'erreur repris dans un message.
const MAX_ERROR_BODY: usize = 200;

/// En-tête que Graylog exige sur toute requête autre que `GET` (sa protection
/// CSRF) ; sans effet ailleurs.
const REQUESTED_BY: &str = "X-Requested-By";

#[derive(Clone)]
pub enum Auth {
    None,
    Basic { username: String, password: String },
    Bearer(String),
}

impl std::fmt::Debug for Auth {
    // Aucun secret ne sort d'ici, pas même dans une trace de débogage.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "Auth::None",
            Self::Basic { .. } => "Auth::Basic(…)",
            Self::Bearer(_) => "Auth::Bearer(…)",
        })
    }
}

impl Auth {
    /// L'authentification d'un serveur qui accepte les trois formes.
    pub fn from_credential(credential: &Credential) -> Result<Self, ProbeError> {
        match credential {
            Credential::None => Ok(Self::None),
            Credential::ApiToken { token } if !token.trim().is_empty() => {
                Ok(Self::Bearer(token.trim().to_string()))
            }
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                Ok(Self::Basic {
                    username: username.trim().to_string(),
                    password: password.clone(),
                })
            }
            other => Err(ProbeError::Config(format!(
                "Expected no credential, a user name and password, or a bearer token; configured: {other}"
            ))),
        }
    }

    /// L'authentification Graylog : un jeton d'accès (envoyé en `jeton:token`)
    /// ou un compte. Graylog ne répond à rien d'utile sans l'un des deux.
    pub fn graylog(credential: &Credential) -> Result<Self, ProbeError> {
        match credential {
            Credential::ApiToken { token } if !token.trim().is_empty() => Ok(Self::Basic {
                username: token.trim().to_string(),
                password: "token".to_string(),
            }),
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                Ok(Self::Basic {
                    username: username.trim().to_string(),
                    password: password.clone(),
                })
            }
            other => Err(ProbeError::Config(format!(
                "Graylog expects an access token or a user name and password, configured: {other}"
            ))),
        }
    }
}

pub struct HttpClient {
    http: reqwest::Client,
    base_url: String,
    auth: Auth,
    timeout: Duration,
    /// Nom du produit, pour des messages d'erreur qui disent qui a répondu.
    product: &'static str,
}

/// Une réponse lue en entier, quel que soit son code.
pub struct Reply {
    pub status: StatusCode,
    pub body: String,
}

impl HttpClient {
    pub fn new(
        http: reqwest::Client,
        base_url: String,
        auth: Auth,
        timeout: Duration,
        product: &'static str,
    ) -> Self {
        Self { http, base_url, auth, timeout, product }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// Un `GET` dont le code est laissé à l'appelant : `/health` et `/ready`
    /// répondent 503 avec une phrase utile, ce n'est pas une panne de transport.
    pub async fn get_raw(&self, path: &str) -> Result<Reply, ProbeError> {
        let response = self.send(self.http.get(self.url(path)), path).await?;
        let status = response.status();
        let body = response.text().await.map_err(|error| self.transport(&error, path))?;
        Ok(Reply { status, body })
    }

    /// Un `GET` qui doit réussir, rendu en texte.
    pub async fn get_text(&self, path: &str) -> Result<String, ProbeError> {
        let reply = self.get_raw(path).await?;
        if !reply.status.is_success() {
            return Err(self.status_error(reply.status, &reply.body, path));
        }
        Ok(reply.body)
    }

    /// Un `GET` JSON qui doit réussir.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ProbeError> {
        let request =
            self.http.get(self.url(path)).header(reqwest::header::ACCEPT, "application/json");
        let response = self.send(request, path).await?;
        self.decode(response, path).await
    }

    /// Un `POST` JSON qui ne modifie rien (une lecture groupée) et doit réussir.
    pub async fn post_json<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ProbeError> {
        let request = self
            .http
            .post(self.url(path))
            .header(reqwest::header::ACCEPT, "application/json")
            .header(REQUESTED_BY, "DumbMonit")
            .json(body);
        let response = self.send(request, path).await?;
        self.decode(response, path).await
    }

    async fn decode<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
        path: &str,
    ) -> Result<T, ProbeError> {
        let status = response.status();
        let body = response.text().await.map_err(|error| self.transport(&error, path))?;
        if !status.is_success() {
            return Err(self.status_error(status, &body, path));
        }
        serde_json::from_str(&body).map_err(|error| {
            ProbeError::Protocol(format!("Unexpected response from {path}: {error}"))
        })
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        path: &str,
    ) -> Result<reqwest::Response, ProbeError> {
        let request = match &self.auth {
            Auth::None => request,
            Auth::Basic { username, password } => request.basic_auth(username, Some(password)),
            Auth::Bearer(token) => request.bearer_auth(token),
        };
        request.timeout(self.timeout).send().await.map_err(|error| self.transport(&error, path))
    }

    /// Traduit un code HTTP en erreur de sonde : un identifiant refusé ne doit
    /// jamais passer pour « serveur injoignable », un 503 jamais pour une erreur
    /// de configuration.
    pub fn status_error(&self, status: StatusCode, body: &str, path: &str) -> ProbeError {
        let product = self.product;
        let excerpt: String = body.chars().take(MAX_ERROR_BODY).collect();
        let excerpt = excerpt.trim();
        match status {
            StatusCode::UNAUTHORIZED => ProbeError::Auth(format!(
                "{product} refused the credentials on {path} (401). Check the user name and \
                 password or the token."
            )),
            StatusCode::FORBIDDEN => ProbeError::Auth(format!(
                "{product} accepted the credentials but refused {path} (403): the account lacks \
                 the permission to read it."
            )),
            StatusCode::NOT_FOUND => ProbeError::Protocol(format!(
                "{product} has no {path} at {}: check the address, the port, and the path \
                 prefix if it sits behind a reverse proxy.",
                self.base_url
            )),
            status if status.is_server_error() => {
                ProbeError::Unreachable(format!("{product} answered {status} on {path}: {excerpt}"))
            }
            status => {
                ProbeError::Protocol(format!("{product} answered {status} on {path}: {excerpt}"))
            }
        }
    }

    fn transport(&self, error: &reqwest::Error, path: &str) -> ProbeError {
        if error.is_timeout() {
            return ProbeError::Timeout(self.timeout);
        }
        if error.is_builder() {
            return ProbeError::Config(format!("Malformed request to {path}: {error}"));
        }
        ProbeError::Unreachable(format!("{path}: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> HttpClient {
        HttpClient::new(
            reqwest::Client::new(),
            "http://loki.lan:3100".into(),
            Auth::None,
            Duration::from_secs(5),
            "Loki",
        )
    }

    #[test]
    fn un_refus_d_identifiant_n_est_pas_une_panne() {
        let error = client().status_error(StatusCode::UNAUTHORIZED, "", "/metrics");
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
        let error = client().status_error(StatusCode::FORBIDDEN, "", "/api/system");
        assert!(matches!(error, ProbeError::Auth(ref m) if m.contains("permission")));
    }

    #[test]
    fn une_erreur_serveur_compte_comme_une_indisponibilite() {
        assert!(
            client().status_error(StatusCode::SERVICE_UNAVAILABLE, "", "/metrics").means_down()
        );
        let error = client().status_error(StatusCode::NOT_FOUND, "", "/metrics");
        assert!(matches!(error, ProbeError::Protocol(ref m) if m.contains("http://loki.lan:3100")));
    }

    #[test]
    fn le_jeton_graylog_part_en_basic_avec_le_mot_token() {
        let auth = Auth::graylog(&Credential::ApiToken { token: " abc \n".into() }).unwrap();
        assert!(matches!(auth, Auth::Basic { ref username, ref password }
            if username == "abc" && password == "token"));
        assert!(Auth::graylog(&Credential::None).is_err());
        assert!(!format!("{auth:?}").contains("abc"), "le jeton ne sort pas en Debug");
    }

    #[test]
    fn les_trois_formes_d_identifiant_sont_acceptees_ailleurs() {
        assert!(matches!(Auth::from_credential(&Credential::None), Ok(Auth::None)));
        assert!(matches!(
            Auth::from_credential(&Credential::ApiToken { token: "t".into() }),
            Ok(Auth::Bearer(_))
        ));
        assert!(matches!(
            Auth::from_credential(&Credential::UsernamePassword {
                username: "u".into(),
                password: "p".into()
            }),
            Ok(Auth::Basic { .. })
        ));
        assert!(
            Auth::from_credential(&Credential::SnmpCommunity { community: "public".into() })
                .is_err()
        );
    }
}
