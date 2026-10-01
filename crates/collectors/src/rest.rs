//! Client HTTP commun aux intégrations d'équipements commerciaux interrogés par
//! une API REST ou GraphQL (pfSense, Unraid, Veeam, Tailscale, FortiGate,
//! Sophos).
//!
//! Même contrat que les autres clients du dépôt : le client `reqwest` partagé
//! (`crate::http`), un délai par requête, des en-têtes d'authentification posés
//! à chaque appel et jamais affichés en `Debug`, et une traduction des statuts
//! HTTP qui sépare ce qui est une panne (5xx, injoignable, délai dépassé) de ce
//! qui est un réglage à corriger (401, 403, 404).

use std::time::Duration;

use dumbmonit_proto::{MetricKind, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub use crate::selfhosted::options::Options;

/// Longueur maximale de l'extrait de réponse recopié dans un message d'erreur.
const MAX_ERROR_BODY: usize = 200;

/// Au plus autant d'éléments nommés par famille (passerelles, disques,
/// tunnels, appareils…) : au-delà, seuls les comptes restent exacts.
pub const MAX_NAMED: usize = 200;

pub struct RestClient {
    http: reqwest::Client,
    base_url: String,
    headers: Vec<(&'static str, String)>,
    timeout: Duration,
    product: &'static str,
}

impl std::fmt::Debug for RestClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.headers.iter().map(|(name, _)| *name).collect();
        write!(f, "RestClient {{ {} {}, headers: {names:?} }}", self.product, self.base_url)
    }
}

impl RestClient {
    /// Client de la cible : racine, protocole, port, certificat et délai lus
    /// dans ses étiquettes.
    pub fn for_target(
        target: &Target,
        default_scheme: &'static str,
        default_port: u16,
        product: &'static str,
    ) -> Result<Self, ProbeError> {
        let options = Options::from_target(target, default_scheme, default_port)?;
        Ok(Self {
            http: crate::http::client(options.insecure_tls)?,
            base_url: options.base_url,
            headers: Vec::new(),
            timeout: options.request_timeout,
            product,
        })
    }

    /// Ajoute un en-tête envoyé avec chaque requête.
    pub fn with_header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }

    /// Remplace (ou pose) un en-tête, typiquement le jeton obtenu après connexion.
    pub fn set_header(&mut self, name: &'static str, value: impl Into<String>) {
        self.headers.retain(|(existing, _)| !existing.eq_ignore_ascii_case(name));
        self.headers.push((name, value.into()));
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn product(&self) -> &'static str {
        self.product
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    fn prepare(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut request = request.header(reqwest::header::ACCEPT, "application/json");
        for (name, value) in &self.headers {
            request = request.header(*name, value);
        }
        request.timeout(self.timeout)
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        path: &str,
    ) -> Result<(StatusCode, String), ProbeError> {
        let response =
            self.prepare(request).send().await.map_err(|error| self.transport(&error, path))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| self.transport(&error, path))?;
        Ok((status, body))
    }

    /// GET brut : statut et corps, sans interprétation.
    pub async fn get(&self, path: &str) -> Result<(StatusCode, String), ProbeError> {
        self.send(self.http.get(self.url(path)), path).await
    }

    /// GET d'un document JSON ; tout statut non 2xx devient une erreur typée.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ProbeError> {
        let (status, body) = self.get(path).await?;
        if !status.is_success() {
            return Err(self.status_error(status, &body, path));
        }
        self.decode(&body, path)
    }

    /// POST d'un formulaire `application/x-www-form-urlencoded`.
    pub async fn post_form(
        &self,
        path: &str,
        form: &[(&str, &str)],
    ) -> Result<(StatusCode, String), ProbeError> {
        self.send(self.http.post(self.url(path)).form(form), path).await
    }

    /// POST d'un corps JSON.
    pub async fn post_json<B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(StatusCode, String), ProbeError> {
        self.send(self.http.post(self.url(path)).json(body), path).await
    }

    pub fn decode<T: DeserializeOwned>(&self, body: &str, path: &str) -> Result<T, ProbeError> {
        serde_json::from_str(body).map_err(|error| {
            ProbeError::Protocol(format!(
                "{} sent an unexpected answer on {path}: {error}",
                self.product
            ))
        })
    }

    pub fn status_error(&self, status: StatusCode, body: &str, path: &str) -> ProbeError {
        let product = self.product;
        let excerpt: String = body.chars().take(MAX_ERROR_BODY).collect();
        let excerpt = excerpt.trim();
        match status {
            StatusCode::UNAUTHORIZED => ProbeError::Auth(format!(
                "{product} refused the credentials on {path} (401). Check the key or the \
                 account."
            )),
            StatusCode::FORBIDDEN => ProbeError::Auth(format!(
                "{product} accepted the credentials but refused {path} (403): the account or \
                 key lacks the permission to read it, or this address is not allowed to use the \
                 API."
            )),
            StatusCode::NOT_FOUND => ProbeError::Protocol(format!(
                "{product} has no {path} at {}: check the address, the port, and that the API \
                 is installed and enabled.",
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

/// Jetons d'accès obtenus par une connexion (OAuth), gardés jusqu'à un peu
/// avant leur expiration : se reconnecter à chaque interrogation ouvrirait une
/// session par minute sur l'équipement.
#[derive(Default)]
pub struct TokenCache {
    tokens: std::sync::Mutex<std::collections::HashMap<String, (String, std::time::Instant)>>,
}

impl TokenCache {
    /// Marge retirée à la durée de vie annoncée.
    const MARGIN: Duration = Duration::from_secs(60);

    pub fn get(&self, key: &str) -> Option<String> {
        let tokens = self.tokens.lock().unwrap_or_else(|poison| poison.into_inner());
        tokens
            .get(key)
            .filter(|(_, until)| *until > std::time::Instant::now())
            .map(|(token, _)| token.clone())
    }

    pub fn put(&self, key: &str, token: &str, lifetime: Duration) {
        let lifetime = lifetime.saturating_sub(Self::MARGIN);
        if lifetime.is_zero() {
            return;
        }
        let mut tokens = self.tokens.lock().unwrap_or_else(|poison| poison.into_inner());
        tokens.insert(key.to_string(), (token.to_string(), std::time::Instant::now() + lifetime));
    }

    pub fn forget(&self, key: &str) {
        self.tokens.lock().unwrap_or_else(|poison| poison.into_inner()).remove(key);
    }
}

pub fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

pub fn counter(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Counter, ts_ms)
}

pub fn flag(name: &str, value: bool, ts_ms: i64) -> Sample {
    gauge(name, if value { 1.0 } else { 0.0 }, ts_ms)
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Un nombre, qu'il arrive en nombre, en texte (`"2260992"`, `"12.5%"`,
/// `"0.4ms"`) ou en booléen : les API PHP et les scalaires GraphQL 64 bits ne
/// sont pas constants là-dessus. Le suffixe d'unité est ignoré.
pub fn number(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let s = s.trim();
            let end = s
                .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')))
                .unwrap_or(s.len());
            s[..end].parse().ok().filter(|v: &f64| v.is_finite())
        }
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Le texte d'un champ, s'il en est un et n'est pas vide.
pub fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

/// Âge en secondes d'un horodatage RFC 3339, `None` s'il ne se lit pas ou
/// s'il est la date nulle (`0001-01-01T00:00:00Z`) que certaines API rendent
/// pour « jamais ».
pub fn age_seconds(raw: &str, now_ms: i64) -> Option<f64> {
    let at = chrono::DateTime::parse_from_rfc3339(raw.trim()).ok()?;
    if at.timestamp() <= 0 {
        return None;
    }
    Some(((now_ms - at.timestamp_millis()) as f64 / 1000.0).max(0.0))
}

/// Secondes restantes jusqu'à un horodatage RFC 3339 (négatif s'il est passé).
pub fn seconds_until(raw: &str, now_ms: i64) -> Option<f64> {
    let at = chrono::DateTime::parse_from_rfc3339(raw.trim()).ok()?;
    if at.timestamp() <= 0 {
        return None;
    }
    Some((at.timestamp_millis() - now_ms) as f64 / 1000.0)
}

#[cfg(test)]
pub(crate) mod test_support {
    use axum::Router;

    /// Sert une application axum sur un port libre de la boucle locale.
    pub async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        address.to_string()
    }

    /// Le premier échantillon de ce nom dont les étiquettes correspondent.
    pub fn find<'a>(
        samples: &'a [dumbmonit_proto::Sample],
        name: &str,
        labels: &[(&str, &str)],
    ) -> Option<&'a dumbmonit_proto::Sample> {
        samples.iter().find(|s| {
            s.metric == name
                && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
        })
    }

    /// La valeur de cet échantillon, qui doit exister.
    pub fn value(samples: &[dumbmonit_proto::Sample], name: &str, labels: &[(&str, &str)]) -> f64 {
        find(samples, name, labels).unwrap_or_else(|| panic!("no sample {name} {labels:?}")).value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn les_nombres_arrivent_sous_toutes_les_formes() {
        assert_eq!(number(Some(&json!(12))), Some(12.0));
        assert_eq!(number(Some(&json!("12.5%"))), Some(12.5));
        assert_eq!(number(Some(&json!("0.4ms"))), Some(0.4));
        assert_eq!(number(Some(&json!(" 2260992 "))), Some(2_260_992.0));
        assert_eq!(number(Some(&json!(true))), Some(1.0));
        assert_eq!(number(Some(&json!("~"))), None);
        assert_eq!(number(Some(&json!(null))), None);
        assert_eq!(number(None), None);
    }

    #[test]
    fn un_jeton_garde_expire_avant_l_equipement() {
        let cache = TokenCache::default();
        cache.put("a", "t1", Duration::from_secs(900));
        assert_eq!(cache.get("a").as_deref(), Some("t1"));
        cache.put("b", "t2", Duration::from_secs(30));
        assert_eq!(cache.get("b"), None, "shorter than the margin: not kept");
        cache.forget("a");
        assert_eq!(cache.get("a"), None);
    }

    #[test]
    fn la_date_nulle_veut_dire_jamais() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-09-30T12:00:00Z")
            .unwrap()
            .timestamp_millis();
        assert_eq!(age_seconds("2026-09-30T11:59:00Z", now), Some(60.0));
        assert_eq!(age_seconds("0001-01-01T00:00:00Z", now), None);
        assert_eq!(seconds_until("2026-10-01T12:00:00Z", now), Some(86_400.0));
        assert_eq!(seconds_until("pas une date", now), None);
    }
}
