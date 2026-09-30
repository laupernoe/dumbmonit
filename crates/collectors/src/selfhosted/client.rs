//! Accès HTTP aux applications auto-hébergées.
//!
//! Tout ce qui touche au réseau est ici ; le reste ne manipule que des
//! structures déjà désérialisées, et se teste donc sans serveur en face.
//!
//! Chaque application a sa façon de présenter un jeton : un en-tête dédié
//! (`NC-Token`, `x-api-key`, `X-Plex-Token`), un `Authorization` au format
//! maison (`Token …`, `MediaBrowser Token="…"`) ou un compte en HTTP « basic ».
//! Le client ne connaît que des en-têtes et un éventuel couple
//! utilisateur / mot de passe ; c'est chaque module qui les compose.

use std::time::Duration;

use dumbmonit_proto::ProbeError;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;

/// Longueur maximale du corps d'erreur repris dans un message.
const MAX_ERROR_BODY: usize = 200;

/// Ce qui accompagne chaque requête : des en-têtes (dont le jeton), et un
/// compte en « basic » le cas échéant.
#[derive(Clone, Default)]
pub struct Auth {
    pub headers: Vec<(&'static str, String)>,
    pub basic: Option<(String, String)>,
}

impl std::fmt::Debug for Auth {
    // Aucun secret ne sort d'ici, pas même dans une trace de débogage.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.headers.iter().map(|(name, _)| *name).collect();
        write!(f, "Auth {{ headers: {names:?}, basic: {} }}", self.basic.is_some())
    }
}

impl Auth {
    pub fn header(name: &'static str, value: impl Into<String>) -> Self {
        Self { headers: vec![(name, value.into())], basic: None }
    }

    pub fn basic(username: &str, password: &str) -> Self {
        Self { headers: Vec::new(), basic: Some((username.to_string(), password.to_string())) }
    }

    pub fn with_header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
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

    /// Un `GET` JSON qui doit réussir.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ProbeError> {
        let (status, body) = self.get(path).await?;
        if !status.is_success() {
            return Err(self.status_error(status, &body, path));
        }
        self.decode(&body, path)
    }

    /// Désérialise un corps déjà lu, avec un message qui dit qui a répondu.
    pub fn decode<T: DeserializeOwned>(&self, body: &str, path: &str) -> Result<T, ProbeError> {
        serde_json::from_str(body).map_err(|error| {
            ProbeError::Protocol(format!(
                "{} sent an unexpected answer on {path}: {error}",
                self.product
            ))
        })
    }

    /// Un `GET` dont le code est laissé à l'appelant.
    pub async fn get(&self, path: &str) -> Result<(StatusCode, String), ProbeError> {
        let mut request =
            self.http.get(self.url(path)).header(reqwest::header::ACCEPT, "application/json");
        for (name, value) in &self.auth.headers {
            request = request.header(*name, value);
        }
        if let Some((username, password)) = &self.auth.basic {
            request = request.basic_auth(username, Some(password));
        }
        let response = request
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|error| self.transport(&error, path))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| self.transport(&error, path))?;
        Ok((status, body))
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
                "{product} refused the credentials on {path} (401). Check the token."
            )),
            StatusCode::FORBIDDEN => ProbeError::Auth(format!(
                "{product} accepted the credentials but refused {path} (403): the account or \
                 key lacks the permission to read it."
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
            "http://immich.lan:2283".into(),
            Auth::default(),
            Duration::from_secs(5),
            "Immich",
        )
    }

    #[test]
    fn un_refus_d_identifiant_n_est_pas_une_panne() {
        let error = client().status_error(StatusCode::UNAUTHORIZED, "", "/api/server/about");
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
        let error = client().status_error(StatusCode::FORBIDDEN, "", "/api/jobs");
        assert!(matches!(error, ProbeError::Auth(ref m) if m.contains("permission")));
    }

    #[test]
    fn une_erreur_serveur_compte_comme_une_indisponibilite() {
        assert!(client().status_error(StatusCode::BAD_GATEWAY, "", "/api").means_down());
        let error = client().status_error(StatusCode::NOT_FOUND, "", "/api");
        assert!(matches!(error, ProbeError::Protocol(ref m) if m.contains("http://immich.lan")));
    }

    #[test]
    fn le_jeton_ne_sort_pas_en_debug() {
        let auth = Auth::header("x-api-key", "s3cr3t").with_header("Accept", "x");
        assert!(!format!("{auth:?}").contains("s3cr3t"));
        assert!(!format!("{:?}", Auth::basic("u", "motdepasse")).contains("motdepasse"));
    }
}
