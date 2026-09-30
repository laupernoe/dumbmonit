//! Accès HTTP à l'API `/control` d'AdGuard Home.
//!
//! L'authentification est le « basic » HTTP d'un utilisateur déclaré dans
//! `AdGuardHome.yaml`. Une installation sans utilisateur répond sans
//! identifiant : `Credential::None` est alors accepté.

use std::time::Duration;

use dumbmonit_proto::{Credential, ProbeError};
use reqwest::StatusCode;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Longueur maximale du corps d'erreur repris dans un message.
const MAX_ERROR_BODY: usize = 200;

#[derive(Clone)]
pub struct Login {
    username: String,
    password: String,
}

impl std::fmt::Debug for Login {
    // Aucun secret ne sort d'ici, pas même dans une trace de débogage.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Login(…)")
    }
}

impl Login {
    pub fn from_credential(credential: &Credential) -> Result<Option<Self>, ProbeError> {
        match credential {
            Credential::None => Ok(None),
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                Ok(Some(Self { username: username.trim().to_string(), password: password.clone() }))
            }
            other => Err(ProbeError::Config(format!(
                "AdGuard Home expects a user name and password, or none; configured: {other}"
            ))),
        }
    }
}

pub struct Client {
    http: reqwest::Client,
    base_url: String,
    login: Option<Login>,
    timeout: Duration,
}

impl Client {
    pub fn new(
        http: reqwest::Client,
        base_url: String,
        login: Option<Login>,
        timeout: Duration,
    ) -> Self {
        Self { http, base_url, login, timeout }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// Un `GET` JSON qui doit réussir.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ProbeError> {
        let request = self.http.get(self.url(path));
        self.run(request, path, self.timeout).await
    }

    /// Un `POST` JSON qui ne modifie rien : la lecture du dernier contrôle de
    /// version, ou le test d'un serveur amont. `timeout` borne la réponse.
    pub async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
        timeout: Duration,
    ) -> Result<T, ProbeError> {
        let request = self.http.post(self.url(path)).json(body);
        self.run(request, path, timeout).await
    }

    async fn run<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
        path: &str,
        timeout: Duration,
    ) -> Result<T, ProbeError> {
        let request = request.header(reqwest::header::ACCEPT, "application/json");
        let request = match &self.login {
            Some(login) => request.basic_auth(&login.username, Some(&login.password)),
            None => request,
        };
        let response = request
            .timeout(timeout)
            .send()
            .await
            .map_err(|error| transport(&error, path, timeout))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| transport(&error, path, timeout))?;
        if !status.is_success() {
            return Err(self.status_error(status, &body, path));
        }
        serde_json::from_str(&body).map_err(|error| {
            ProbeError::Protocol(format!(
                "Unexpected response from {path}: {error}. Is this AdGuard Home?"
            ))
        })
    }

    /// Traduit un code HTTP en erreur de sonde : un identifiant refusé ne doit
    /// jamais passer pour « serveur injoignable ».
    pub fn status_error(&self, status: StatusCode, body: &str, path: &str) -> ProbeError {
        let excerpt: String = body.chars().take(MAX_ERROR_BODY).collect();
        let excerpt = excerpt.trim();
        match status {
            StatusCode::UNAUTHORIZED => ProbeError::Auth(format!(
                "AdGuard Home refused the credentials on {path} (401). Check the user name and \
                 password: after five failed attempts AdGuard Home blocks the address for 15 \
                 minutes."
            )),
            StatusCode::FORBIDDEN => ProbeError::Auth(format!(
                "AdGuard Home refused {path} (403): the login may be blocked after too many \
                 failed attempts. {excerpt}"
            )),
            StatusCode::NOT_FOUND => ProbeError::Protocol(format!(
                "No {path} at {}: check the address and the port of the AdGuard Home web \
                 interface (not the DNS port).",
                self.base_url
            )),
            status if status.is_server_error() => ProbeError::Unreachable(format!(
                "AdGuard Home answered {status} on {path}: {excerpt}"
            )),
            status => {
                ProbeError::Protocol(format!("AdGuard Home answered {status} on {path}: {excerpt}"))
            }
        }
    }
}

fn transport(error: &reqwest::Error, path: &str, timeout: Duration) -> ProbeError {
    if error.is_timeout() {
        return ProbeError::Timeout(timeout);
    }
    if error.is_builder() {
        return ProbeError::Config(format!("Malformed request to {path}: {error}"));
    }
    ProbeError::Unreachable(format!("{path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> Client {
        Client::new(
            reqwest::Client::new(),
            "http://dns.lan:80".into(),
            None,
            Duration::from_secs(5),
        )
    }

    #[test]
    fn un_refus_d_identifiant_n_est_pas_une_panne() {
        let error = client().status_error(StatusCode::UNAUTHORIZED, "", "/control/status");
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
        assert!(client().status_error(StatusCode::BAD_GATEWAY, "", "/x").means_down());
        let error = client().status_error(StatusCode::NOT_FOUND, "", "/control/status");
        assert!(matches!(error, ProbeError::Protocol(ref m) if m.contains("http://dns.lan:80")));
    }

    #[test]
    fn seuls_l_absence_d_identifiant_et_le_compte_sont_acceptes() {
        assert!(Login::from_credential(&Credential::None).unwrap().is_none());
        let login = Login::from_credential(&Credential::UsernamePassword {
            username: " dumbmonit ".into(),
            password: "secret".into(),
        })
        .unwrap()
        .unwrap();
        assert_eq!(login.username, "dumbmonit");
        assert!(!format!("{login:?}").contains("secret"));
        assert!(Login::from_credential(&Credential::ApiToken { token: "t".into() }).is_err());
    }
}
