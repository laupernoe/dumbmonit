//! Accès HTTP à l'API REST de Pi-hole v6 (`/api`).
//!
//! L'authentification se fait en deux temps : `POST /api/auth` avec le mot de
//! passe rend un identifiant de session (`sid`), que chaque requête suivante
//! porte dans l'en-tête `X-FTL-SID`. Aucune protection CSRF à satisfaire : elle
//! ne concerne que la session par cookie du navigateur.
//!
//! Un Pi-hole sans mot de passe répond à tout sans session ; sa réponse à
//! `POST /api/auth` dit alors `valid: true` sans `sid`.

use std::time::Duration;

use dumbmonit_proto::ProbeError;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::json;

/// Longueur maximale du corps d'erreur repris dans un message.
const MAX_ERROR_BODY: usize = 200;

const SID_HEADER: &str = "X-FTL-SID";

pub struct Client {
    http: reqwest::Client,
    base_url: String,
    timeout: Duration,
}

/// Une réponse lue en entier, quel que soit son code.
pub struct Reply {
    pub status: StatusCode,
    pub body: String,
}

#[derive(Debug, Deserialize)]
struct AuthReply {
    session: AuthSession,
}

#[derive(Debug, Deserialize)]
struct AuthSession {
    #[serde(default)]
    valid: bool,
    #[serde(default)]
    sid: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

impl Client {
    pub fn new(http: reqwest::Client, base_url: String, timeout: Duration) -> Self {
        Self { http, base_url, timeout }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// Ouvre une session. `Ok(None)` : Pi-hole n'a pas de mot de passe, les
    /// requêtes passent sans session.
    pub async fn login(&self, password: &str) -> Result<Option<String>, ProbeError> {
        let path = "/api/auth";
        let request = self.http.post(self.url(path)).json(&json!({ "password": password }));
        let reply = self.send(request, path).await?;
        if reply.status == StatusCode::UNAUTHORIZED {
            return Err(ProbeError::Auth(
                "Pi-hole refused the password. Check the app password: generating a new one \
                 in Pi-hole replaces the previous one."
                    .to_string(),
            ));
        }
        if !reply.status.is_success() {
            return Err(self.status_error(reply.status, &reply.body, path));
        }
        let auth: AuthReply = serde_json::from_str(&reply.body).map_err(|error| {
            ProbeError::Protocol(format!(
                "{} does not look like Pi-hole v6: {error}",
                self.url(path)
            ))
        })?;
        if !auth.session.valid {
            return Err(ProbeError::Auth(format!(
                "Pi-hole refused the session: {}",
                auth.session.message.unwrap_or_default()
            )));
        }
        Ok(auth.session.sid.filter(|sid| !sid.is_empty()))
    }

    /// Un `GET` dont le code est laissé à l'appelant, qui doit pouvoir
    /// reconnaître une session expirée (401).
    pub async fn get(&self, path: &str, sid: Option<&str>) -> Result<Reply, ProbeError> {
        let mut request =
            self.http.get(self.url(path)).header(reqwest::header::ACCEPT, "application/json");
        if let Some(sid) = sid {
            request = request.header(SID_HEADER, sid);
        }
        self.send(request, path).await
    }

    pub fn decode<T: DeserializeOwned>(&self, reply: &Reply, path: &str) -> Result<T, ProbeError> {
        if !reply.status.is_success() {
            return Err(self.status_error(reply.status, &reply.body, path));
        }
        serde_json::from_str(&reply.body).map_err(|error| {
            ProbeError::Protocol(format!("Unexpected response from {path}: {error}"))
        })
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        path: &str,
    ) -> Result<Reply, ProbeError> {
        let response = request
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|error| self.transport(&error, path))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| self.transport(&error, path))?;
        Ok(Reply { status, body })
    }

    /// Traduit un code HTTP en erreur de sonde : un mot de passe refusé ne doit
    /// jamais passer pour « serveur injoignable ».
    pub fn status_error(&self, status: StatusCode, body: &str, path: &str) -> ProbeError {
        let excerpt: String = body.chars().take(MAX_ERROR_BODY).collect();
        let excerpt = excerpt.trim();
        match status {
            StatusCode::UNAUTHORIZED => ProbeError::Auth(format!(
                "Pi-hole asks for a password on {path} (401): enter its app password."
            )),
            StatusCode::FORBIDDEN => ProbeError::Auth(format!(
                "Pi-hole accepted the password but refused {path} (403): {excerpt}"
            )),
            // `api_seats_exceeded` : toutes les sessions de l'API sont prises.
            StatusCode::TOO_MANY_REQUESTS => ProbeError::Auth(format!(
                "Pi-hole has no free API session (429): too many sessions are open. They expire \
                 after 30 minutes of inactivity, or raise webserver.api.max_sessions. {excerpt}"
            )),
            StatusCode::NOT_FOUND => ProbeError::Protocol(format!(
                "{} has no {path}: this is not Pi-hole v6 or later, or the address or port is \
                 wrong. Pi-hole 5 and earlier have no REST API.",
                self.base_url
            )),
            status if status.is_server_error() => {
                ProbeError::Unreachable(format!("Pi-hole answered {status} on {path}: {excerpt}"))
            }
            status => {
                ProbeError::Protocol(format!("Pi-hole answered {status} on {path}: {excerpt}"))
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

    fn client() -> Client {
        Client::new(reqwest::Client::new(), "http://pi.hole:80".into(), Duration::from_secs(5))
    }

    #[test]
    fn un_refus_n_est_pas_une_panne() {
        for status in
            [StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN, StatusCode::TOO_MANY_REQUESTS]
        {
            let error = client().status_error(status, "", "/api/stats/summary");
            assert!(matches!(error, ProbeError::Auth(_)), "{status}");
            assert!(!error.means_down());
        }
        let error = client().status_error(StatusCode::NOT_FOUND, "", "/api/auth");
        assert!(matches!(error, ProbeError::Protocol(ref m) if m.contains("Pi-hole 5")));
        assert!(client().status_error(StatusCode::BAD_GATEWAY, "", "/api/auth").means_down());
    }
}
