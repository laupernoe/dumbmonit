//! HPE Aruba Networking Central (« new Central »), par son API REST publique.
//!
//! L'adresse de la cible est celle de la passerelle d'API de la région du
//! compte (`us1.api.central.arubanetworks.com`, `de1.api…`, `gb1.api…`) : c'est
//! un service sur Internet, pas un équipement du réseau local, et le
//! garde-fou des adresses privées ne le concerne pas.
//!
//! Deux façons de s'authentifier, toutes deux documentées par HPE :
//!
//! * un client d'API personnel (identifiant et secret), échangé auprès de
//!   `sso.common.cloud.hpe.com/as/token.oauth2` (`client_credentials`) contre
//!   un jeton de deux heures, gardé jusqu'à son expiration ;
//! * un jeton d'accès déjà obtenu (même durée de vie : à renouveler à la main).
//!
//! Une lecture obligatoire — la liste des appareils
//! (`GET /network-monitoring/v1/devices`, paginée par curseur) — et trois
//! facultatives, qu'un client sans le droit correspondant peut refuser sans
//! faire échouer le reste : les alertes actives
//! (`/network-notifications/v1/alerts`), la santé des clients et la santé des
//! appareils (`/network-monitoring/v1/tenant-client-health` et
//! `tenant-device-health`).
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **un point d'accès, un commutateur ou une passerelle hors ligne** vu du
//!   nuage Central (`status: OFFLINE`) ;
//! * **une alerte critique active** dans Central ;
//! * **des appareils dont la santé est mauvaise**.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `token_address` | `sso.common.cloud.hpe.com` | Hôte du service qui délivre les jetons. |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::Value;

use crate::rest::{MAX_NAMED, RestClient, TokenCache, flag, gauge, now_ms, number, text};
use crate::selfhosted::options::tag;

pub const DEFAULT_PORT: u16 = 443;
pub const DEFAULT_ADDRESS: &str = "us1.api.central.arubanetworks.com";
pub const DEFAULT_TOKEN_ADDRESS: &str = "sso.common.cloud.hpe.com";
const TOKEN_PATH: &str = "/as/token.oauth2";

/// Au plus dix pages de mille appareils.
const MAX_PAGES: usize = 10;
const DEVICES: &str = "/network-monitoring/v1/devices?limit=1000";
/// `status eq 'Active'`, déjà encodé.
const ALERTS: &str = "/network-notifications/v1/alerts?filter=status%20eq%20%27Active%27&limit=100";
const CLIENT_HEALTH: &str = "/network-monitoring/v1/tenant-client-health";
const DEVICE_HEALTH: &str = "/network-monitoring/v1/tenant-device-health";

#[derive(Default)]
pub struct ArubaCentralCollector {
    tokens: TokenCache,
}

impl ArubaCentralCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clé du jeton gardé : la cible et l'identifiant du client d'API.
    fn token_key(target: &Target) -> Option<String> {
        match &target.credential {
            Credential::UsernamePassword { username, .. } => {
                Some(format!("{}:{}", target.id, username.trim()))
            }
            _ => None,
        }
    }

    async fn authorize(&self, client: &mut RestClient, target: &Target) -> Result<(), ProbeError> {
        match &target.credential {
            Credential::ApiToken { token } if !token.trim().is_empty() => {
                client.set_header("Authorization", format!("Bearer {}", token.trim()));
                Ok(())
            }
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                let key = Self::token_key(target).unwrap_or_default();
                let token = match self.tokens.get(&key) {
                    Some(token) => token,
                    None => {
                        let (token, lifetime) =
                            oauth_token(target, username.trim(), password.trim()).await?;
                        self.tokens.put(&key, &token, lifetime);
                        token
                    }
                };
                client.set_header("Authorization", format!("Bearer {token}"));
                Ok(())
            }
            other => Err(ProbeError::Config(format!(
                "Aruba Central expects an API client (ID and secret) or an access token, \
                 configured: {other}"
            ))),
        }
    }

    async fn get(
        &self,
        client: &RestClient,
        target: &Target,
        path: &str,
    ) -> Result<Value, ProbeError> {
        let (status, body) = client.get(path).await?;
        match status.as_u16() {
            200..=299 => client.decode(&body, path),
            401 => {
                // Jeton gardé révoqué entre-temps : le prochain passage en redemandera un.
                if let Some(key) = Self::token_key(target) {
                    self.tokens.forget(&key);
                }
                Err(ProbeError::Auth(
                    "Aruba Central refused the credentials (401): the access token expired \
                     (they last two hours), or the API client was reset or deleted."
                        .to_string(),
                ))
            }
            403 => Err(ProbeError::Auth(format!(
                "Aruba Central refused {path} (403): the API client has no access to Central \
                 monitoring, or the base URL is not the one of its region."
            ))),
            _ => Err(client.status_error(status, &body, path)),
        }
    }

    /// Une lecture facultative : un refus ou une route absente ne fait rien
    /// échouer, une panne réelle si.
    async fn optional(
        &self,
        client: &RestClient,
        target: &Target,
        path: &str,
    ) -> Result<Option<Value>, ProbeError> {
        match self.get(client, target, path).await {
            Ok(reply) => Ok(Some(reply)),
            Err(error) if error.means_down() => Err(error),
            Err(error) => {
                tracing::debug!(path, %error, "Aruba Central : lecture facultative indisponible");
                Ok(None)
            }
        }
    }

    /// Tous les appareils, page après page (curseur `next`).
    async fn devices(
        &self,
        client: &RestClient,
        target: &Target,
    ) -> Result<Vec<Value>, ProbeError> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let path = match &cursor {
                Some(next) => format!("{DEVICES}&next={next}"),
                None => DEVICES.to_string(),
            };
            let reply = self.get(client, target, &path).await?;
            let Some(items) = reply.get("items").and_then(Value::as_array) else {
                return Err(ProbeError::Protocol(format!(
                    "{} did not answer like Aruba Central (no \"items\"): check the base URL \
                     of your region.",
                    client.url(DEVICES)
                )));
            };
            out.extend(items.iter().cloned());
            cursor = match reply.get("next") {
                Some(Value::String(next)) if safe_cursor(next) => Some(next.clone()),
                Some(Value::Number(next)) => Some(next.to_string()),
                _ => None,
            };
            if cursor.is_none() {
                break;
            }
        }
        Ok(out)
    }

    async fn replies(&self, target: &Target) -> Result<Replies, ProbeError> {
        let mut client = RestClient::for_target(target, "https", DEFAULT_PORT, "Aruba Central")?;
        self.authorize(&mut client, target).await?;
        let (devices, alerts, clients, health) = tokio::join!(
            self.devices(&client, target),
            self.optional(&client, target, ALERTS),
            self.optional(&client, target, CLIENT_HEALTH),
            self.optional(&client, target, DEVICE_HEALTH),
        );
        Ok(Replies {
            devices: devices?,
            alerts: alerts?,
            client_health: clients?,
            device_health: health?,
        })
    }
}

/// Un curseur n'est recopié dans l'URL que s'il n'a rien d'une injection.
fn safe_cursor(raw: &str) -> bool {
    !raw.is_empty()
        && raw.len() <= 128
        && raw.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '=' | '.'))
}

/// Échange l'identifiant et le secret d'un client d'API contre un jeton.
async fn oauth_token(
    target: &Target,
    client_id: &str,
    secret: &str,
) -> Result<(String, Duration), ProbeError> {
    // Le service de jetons n'est pas la passerelle d'API : autre hôte, mêmes
    // délais.
    let mut sso = target.clone();
    sso.address = tag(target, "token_address").unwrap_or(DEFAULT_TOKEN_ADDRESS).to_string();
    sso.tags = target
        .tags
        .iter()
        .filter(|(key, _)| key.as_str() == "request_timeout_seconds")
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let client = RestClient::for_target(&sso, "https", DEFAULT_PORT, "HPE GreenLake sign-in")?;
    let form =
        [("grant_type", "client_credentials"), ("client_id", client_id), ("client_secret", secret)];
    let (status, body) = client.post_form(TOKEN_PATH, &form).await?;
    if matches!(status.as_u16(), 400 | 401 | 403) {
        return Err(ProbeError::Auth(format!(
            "HPE GreenLake refused the API client ({status}): check the client ID and secret, \
             and that the client was not reset."
        )));
    }
    if !status.is_success() {
        return Err(client.status_error(status, &body, TOKEN_PATH));
    }
    let reply: Value = client.decode(&body, TOKEN_PATH)?;
    let token = text(&reply, "access_token").ok_or_else(|| {
        ProbeError::Protocol(
            "HPE GreenLake returned no access_token for the API client".to_string(),
        )
    })?;
    let lifetime = reply.get("expires_in").and_then(Value::as_u64).unwrap_or(900);
    Ok((token.to_string(), Duration::from_secs(lifetime)))
}

#[async_trait]
impl Collector for ArubaCentralCollector {
    fn kind(&self) -> &'static str {
        "arubacentral"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let replies = self.replies(target).await?;
        Ok(samples(&replies, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let mut client = RestClient::for_target(target, "https", DEFAULT_PORT, "Aruba Central")?;
        self.authorize(&mut client, target).await?;
        self.get(&client, target, "/network-monitoring/v1/devices?limit=1").await?;
        Ok(Some("arubacentral".to_string()))
    }
}

pub struct Replies {
    pub devices: Vec<Value>,
    pub alerts: Option<Value>,
    pub client_health: Option<Value>,
    pub device_health: Option<Value>,
}

/// `ACCESS_POINT` → `access_point`.
fn device_type(raw: &str) -> &'static str {
    match raw {
        "ACCESS_POINT" => "access_point",
        "SWITCH" => "switch",
        "GATEWAY" => "gateway",
        _ => "other",
    }
}

/// Les groupes de santé (`Good`, `Fair`, `Poor`) d'une entrée de santé.
fn health_groups(entry: &Value) -> Vec<(String, f64)> {
    entry
        .pointer("/health/groups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|group| {
            Some((text(group, "name")?.to_ascii_lowercase(), number(group.get("value"))?))
        })
        .collect()
}

fn slug(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace(' ', "_")
}

pub fn samples(r: &Replies, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();

    let mut by_type: BTreeMap<&'static str, (f64, f64)> = BTreeMap::new();
    let mut offline_total = 0.0;
    for (index, device) in r.devices.iter().enumerate() {
        let kind = device_type(text(device, "deviceType").unwrap_or(""));
        let online = match text(device, "status") {
            Some("ONLINE") => Some(true),
            Some("OFFLINE") => Some(false),
            _ => None,
        };
        let entry = by_type.entry(kind).or_insert((0.0, 0.0));
        entry.0 += 1.0;
        if online == Some(false) {
            entry.1 += 1.0;
            offline_total += 1.0;
        }
        if index >= MAX_NAMED {
            continue;
        }
        let Some(name) = text(device, "deviceName").or_else(|| text(device, "serialNumber")) else {
            continue;
        };
        let serial = text(device, "serialNumber").unwrap_or("");
        let site = text(device, "siteName").unwrap_or("");
        let named = |sample: Sample| {
            sample.with_label("device", name).with_label("type", kind).with_label("serial", serial)
        };
        if let Some(online) = online {
            out.push(
                named(flag("arubacentral_device_online", online, ts_ms)).with_label("site", site),
            );
            if online && let Some(millis) = number(device.get("uptimeInMillis")) {
                out.push(named(g("arubacentral_device_uptime_seconds", millis / 1000.0)));
            }
        }
        out.push(
            named(g("arubacentral_device_info", 1.0))
                .with_label("model", text(device, "model").unwrap_or(""))
                .with_label("firmware", text(device, "firmwareVersion").unwrap_or(""))
                .with_label("ip", text(device, "ipv4").unwrap_or(""))
                .with_label("site", site),
        );
    }
    out.push(g("arubacentral_devices", r.devices.len() as f64));
    out.push(g("arubacentral_devices_offline", offline_total));
    for (kind, (count, offline)) in &by_type {
        out.push(g("arubacentral_devices_by_type", *count).with_label("type", *kind));
        out.push(g("arubacentral_devices_offline_by_type", *offline).with_label("type", *kind));
    }

    if let Some(alerts) = &r.alerts {
        let items: Vec<&Value> =
            alerts.get("items").and_then(Value::as_array).into_iter().flatten().collect();
        let total = number(alerts.get("total")).unwrap_or(items.len() as f64);
        out.push(g("arubacentral_alerts_active", total));
        // Les cent plus récentes : la gravité « critical » est toujours publiée,
        // à zéro s'il n'y en a pas, pour que l'alerte puisse se résoudre.
        let mut severities: BTreeMap<String, f64> = BTreeMap::new();
        severities.insert("critical".to_string(), 0.0);
        for item in items {
            let severity = text(item, "severity").unwrap_or("unknown").to_ascii_lowercase();
            *severities.entry(severity).or_insert(0.0) += 1.0;
        }
        for (severity, count) in severities {
            out.push(
                g("arubacentral_alerts_active_by_severity", count).with_label("severity", severity),
            );
        }
    }

    if let Some(health) = &r.client_health {
        for entry in health.get("clientTypes").and_then(Value::as_array).into_iter().flatten() {
            let Some(name) = text(entry, "name") else { continue };
            let kind = slug(name);
            let groups = health_groups(entry);
            out.push(
                g("arubacentral_clients", groups.iter().map(|(_, value)| value).sum())
                    .with_label("type", kind.as_str()),
            );
            for (group, value) in groups {
                out.push(
                    g("arubacentral_clients_by_health", value)
                        .with_label("type", kind.as_str())
                        .with_label("health", group),
                );
            }
        }
    }

    if let Some(health) = &r.device_health {
        let mut poor = 0.0;
        for entry in health.get("deviceTypes").and_then(Value::as_array).into_iter().flatten() {
            let Some(name) = text(entry, "name") else { continue };
            let kind = slug(name);
            for (group, value) in health_groups(entry) {
                if group == "poor" {
                    poor += value;
                }
                out.push(
                    g("arubacentral_devices_by_health", value)
                        .with_label("type", kind.as_str())
                        .with_label("health", group),
                );
            }
        }
        out.push(g("arubacentral_devices_poor_health", poor));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::extract::Form;
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::routing::{get, post};

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après les exemples de la référence d'API de HPE
    // Aruba Networking Central (`getDevicesV1`, `getAlertListV1`,
    // `tenantClientHealthV1`, `tenantDeviceHealthV1`) : deux points d'accès
    // dont un hors ligne, un commutateur, une passerelle, trois alertes dont
    // une critique.
    const DEVICES_JSON: &str = include_str!("testdata/devices.json");
    const ALERTS_JSON: &str = include_str!("testdata/alerts.json");
    const CLIENTS_JSON: &str = include_str!("testdata/tenant_client_health.json");
    const HEALTH_JSON: &str = include_str!("testdata/tenant_device_health.json");

    const NOW: i64 = 1_790_000_000_000;

    fn json(raw: &str) -> Value {
        serde_json::from_str(raw).unwrap()
    }

    fn items() -> Vec<Value> {
        json(DEVICES_JSON)["items"].as_array().unwrap().clone()
    }

    fn replies() -> Replies {
        Replies {
            devices: items(),
            alerts: Some(json(ALERTS_JSON)),
            client_health: Some(json(CLIENTS_JSON)),
            device_health: Some(json(HEALTH_JSON)),
        }
    }

    #[test]
    fn appareils_alertes_et_sante() {
        let s = samples(&replies(), NOW);
        assert_eq!(value(&s, "arubacentral_devices", &[]), 4.0);
        assert_eq!(value(&s, "arubacentral_devices_offline", &[]), 1.0);
        assert_eq!(value(&s, "arubacentral_devices_by_type", &[("type", "access_point")]), 2.0);
        assert_eq!(
            value(&s, "arubacentral_devices_offline_by_type", &[("type", "access_point")]),
            1.0
        );
        assert_eq!(value(&s, "arubacentral_device_online", &[("device", "ap-lobby")]), 1.0);
        assert_eq!(value(&s, "arubacentral_device_online", &[("device", "ap-warehouse")]), 0.0);
        assert_eq!(
            value(&s, "arubacentral_device_uptime_seconds", &[("device", "core-sw1")]),
            86_400.0
        );
        assert!(
            find(&s, "arubacentral_device_uptime_seconds", &[("device", "ap-warehouse")]).is_none(),
            "an offline device has no uptime"
        );
        let info = find(&s, "arubacentral_device_info", &[("device", "ap-lobby")]).unwrap();
        assert_eq!(info.labels["model"], "AP-515");
        assert_eq!(info.labels["firmware"], "10.7.1.0");

        assert_eq!(value(&s, "arubacentral_alerts_active", &[]), 3.0);
        assert_eq!(
            value(&s, "arubacentral_alerts_active_by_severity", &[("severity", "critical")]),
            1.0
        );
        assert_eq!(
            value(&s, "arubacentral_alerts_active_by_severity", &[("severity", "minor")]),
            2.0
        );

        assert_eq!(value(&s, "arubacentral_clients", &[("type", "wireless")]), 47.0);
        assert_eq!(
            value(
                &s,
                "arubacentral_clients_by_health",
                &[("type", "wireless"), ("health", "poor")]
            ),
            2.0
        );
        assert_eq!(value(&s, "arubacentral_devices_poor_health", &[]), 1.0);
        assert_eq!(
            value(
                &s,
                "arubacentral_devices_by_health",
                &[("type", "access_points"), ("health", "good")]
            ),
            14.0
        );
    }

    #[test]
    fn sans_les_lectures_facultatives() {
        let r = Replies { alerts: None, client_health: None, device_health: None, ..replies() };
        let s = samples(&r, NOW);
        assert_eq!(value(&s, "arubacentral_devices", &[]), 4.0);
        assert!(find(&s, "arubacentral_alerts_active", &[]).is_none());
        assert!(find(&s, "arubacentral_clients", &[]).is_none());
        assert!(find(&s, "arubacentral_devices_poor_health", &[]).is_none());
    }

    #[test]
    fn la_gravite_critique_reste_publiee_a_zero() {
        let mut r = replies();
        r.alerts = Some(json(r#"{"count":0,"total":0,"next":null,"items":[]}"#));
        let s = samples(&r, NOW);
        assert_eq!(
            value(&s, "arubacentral_alerts_active_by_severity", &[("severity", "critical")]),
            0.0
        );
    }

    #[test]
    fn curseurs() {
        assert!(safe_cursor("2"));
        assert!(safe_cursor("abc-DEF_1="));
        assert!(!safe_cursor("2&limit=5"));
        assert!(!safe_cursor(""));
    }

    fn page(items: &[Value], next: Option<&str>) -> String {
        serde_json::json!({ "items": items, "count": items.len(), "total": 4, "next": next })
            .to_string()
    }

    #[tokio::test]
    async fn client_oauth_pages_et_jeton_garde() {
        let logins = Arc::new(AtomicUsize::new(0));
        let counted = logins.clone();
        let all = items();
        let app = Router::new()
            .route(
                TOKEN_PATH,
                post(move |Form(form): Form<HashMap<String, String>>| {
                    let counted = counted.clone();
                    async move {
                        if form.get("client_secret").map(String::as_str) != Some("s3cret")
                            || form.get("grant_type").map(String::as_str)
                                != Some("client_credentials")
                        {
                            return (
                                StatusCode::UNAUTHORIZED,
                                r#"{"error":"invalid_client"}"#.to_string(),
                            );
                        }
                        counted.fetch_add(1, Ordering::SeqCst);
                        (
                            StatusCode::OK,
                            r#"{"access_token":"tok-1","token_type":"Bearer","expires_in":7199}"#
                                .to_string(),
                        )
                    }
                }),
            )
            .fallback(get(move |headers: HeaderMap, uri: Uri| {
                let all = all.clone();
                async move {
                    if headers.get("authorization").and_then(|v| v.to_str().ok())
                        != Some("Bearer tok-1")
                    {
                        return (StatusCode::UNAUTHORIZED, String::new());
                    }
                    match uri.path() {
                        "/network-monitoring/v1/devices" => {
                            if uri.query().unwrap_or_default().contains("next=2") {
                                (StatusCode::OK, page(&all[2..], None))
                            } else {
                                (StatusCode::OK, page(&all[..2], Some("2")))
                            }
                        }
                        "/network-notifications/v1/alerts" => {
                            assert!(uri.query().unwrap_or_default().contains("Active"));
                            (StatusCode::OK, ALERTS_JSON.to_string())
                        }
                        // Le client d'API n'a pas le droit de lire la santé.
                        _ => (StatusCode::FORBIDDEN, String::new()),
                    }
                }
            }));
        let address = serve(app).await;
        let url = format!("http://{address}");
        let collector = ArubaCentralCollector::new();
        let mut target = cible("arubacentral", &url, &[("token_address", url.as_str())]);
        target.credential = Credential::UsernamePassword {
            username: "client-id".into(),
            password: "s3cret".into(),
        };
        let s = collector.probe(&target).await.unwrap();
        assert_eq!(value(&s, "arubacentral_devices", &[]), 4.0, "both pages are read");
        assert_eq!(value(&s, "arubacentral_alerts_active", &[]), 3.0);
        assert!(find(&s, "arubacentral_clients", &[]).is_none(), "403 on an optional read");
        collector.probe(&target).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 1, "the token is kept for its lifetime");

        target.credential = Credential::ApiToken { token: "tok-1".into() };
        assert!(collector.probe(&target).await.is_ok());
        target.credential = Credential::ApiToken { token: "expired".into() };
        let error = collector.probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
        target.credential =
            Credential::UsernamePassword { username: "other".into(), password: "wrong".into() };
        let error = collector.probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }

    #[tokio::test]
    async fn un_identifiant_absent_est_une_erreur_de_reglage() {
        let target = cible("arubacentral", "unreachable.invalid:1", &[]);
        let error = ArubaCentralCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }
}
