//! Tailscale, par l'API publique (`https://api.tailscale.com/api/v2`).
//!
//! Une seule lecture : la liste des appareils du tailnet
//! (`/tailnet/{tailnet}/devices`), avec deux façons de s'authentifier :
//!
//! * un client OAuth limité à la portée `devices:core:read` (identifiant et
//!   secret), échangé contre un jeton d'une heure, gardé jusqu'à son
//!   expiration — la forme recommandée, la seule qui soit réellement en
//!   lecture seule ;
//! * un jeton d'accès à l'API (`tskey-api-…`), qui porte tous les droits de
//!   son propriétaire et expire au bout de 1 à 90 jours.
//!
//! C'est une API sur Internet : l'adresse de la cible est celle de l'API, pas
//! d'un appareil, et le garde-fou des adresses privées ne la concerne pas.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **un serveur hors ligne** : ni sa liaison à Tailscale, ni ce qu'il route ;
//! * **une clé de nœud qui expire** : l'appareil quitte le tailnet le jour dit
//!   et quelqu'un doit se reconnecter à la main ;
//! * **un appareil en attente d'approbation** (tailnets à approbation) ;
//! * **un client à mettre à jour**.
//!
//! Seuls les appareils étiquetés (`tag:…`) — des serveurs, en pratique — sont
//! surveillés hors ligne par défaut : un téléphone ou un portable éteint n'est
//! pas une panne. L'option `watch` change la liste. Les appareils partagés
//! depuis un autre tailnet (`isExternal`) sont comptés mais pas décrits.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `tailnet` | `-` | Le tailnet, `-` pour celui du jeton. |
//! | `watch` | vide | Appareils à garder en ligne : noms ou `tag:…`, ou `all` ; vide = les appareils étiquetés. |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::Value;

use crate::rest::{
    MAX_NAMED, RestClient, TokenCache, age_seconds, flag, gauge, now_ms, seconds_until, text,
};
use crate::selfhosted::options::tag;

pub const DEFAULT_PORT: u16 = 443;
pub const DEFAULT_ADDRESS: &str = "api.tailscale.com";
const TOKEN_PATH: &str = "/api/v2/oauth/token";

#[derive(Default)]
pub struct TailscaleCollector {
    tokens: TokenCache,
}

impl TailscaleCollector {
    pub fn new() -> Self {
        Self::default()
    }

    async fn authorize(&self, client: &mut RestClient, target: &Target) -> Result<(), ProbeError> {
        match &target.credential {
            Credential::ApiToken { token } if !token.trim().is_empty() => {
                client.set_header("Authorization", format!("Bearer {}", token.trim()));
                Ok(())
            }
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                let key = format!("{}:{}", target.id, username.trim());
                let token = match self.tokens.get(&key) {
                    Some(token) => token,
                    None => {
                        let (token, lifetime) =
                            oauth_token(client, username.trim(), password.trim()).await?;
                        self.tokens.put(&key, &token, lifetime);
                        token
                    }
                };
                client.set_header("Authorization", format!("Bearer {token}"));
                Ok(())
            }
            other => Err(ProbeError::Config(format!(
                "Tailscale expects an OAuth client (ID and secret) or an API access token, \
                 configured: {other}"
            ))),
        }
    }

    async fn devices(&self, target: &Target) -> Result<Value, ProbeError> {
        let mut client = RestClient::for_target(target, "https", DEFAULT_PORT, "Tailscale")?;
        self.authorize(&mut client, target).await?;
        let tailnet = tag(target, "tailnet").unwrap_or("-");
        if tailnet.contains(['/', '?', '#']) {
            return Err(ProbeError::Config(format!("Invalid tailnet name \"{tailnet}\"")));
        }
        let path = format!("/api/v2/tailnet/{tailnet}/devices?fields=all");
        let (status, body) = client.get(&path).await?;
        match status.as_u16() {
            200..=299 => client.decode(&body, &path),
            401 => {
                // Jeton gardé révoqué entre-temps : le prochain passage en redemandera un.
                if let Credential::UsernamePassword { username, .. } = &target.credential {
                    self.tokens.forget(&format!("{}:{}", target.id, username.trim()));
                }
                Err(ProbeError::Auth(
                    "Tailscale refused the credentials (401): the API access token expired or \
                     was revoked, or the OAuth client was deleted."
                        .to_string(),
                ))
            }
            403 => Err(ProbeError::Auth(format!(
                "Tailscale refused to list the devices of tailnet \"{tailnet}\" (403): give the \
                 OAuth client the devices:core:read scope, and check the tailnet name."
            ))),
            404 => Err(ProbeError::Config(format!(
                "Tailscale knows no tailnet \"{tailnet}\": leave the option empty (\"-\" is the \
                 tailnet of the credentials) or copy the name shown in the admin console."
            ))),
            _ => Err(client.status_error(status, &body, &path)),
        }
    }
}

/// Échange l'identifiant et le secret d'un client OAuth contre un jeton.
async fn oauth_token(
    client: &RestClient,
    client_id: &str,
    secret: &str,
) -> Result<(String, Duration), ProbeError> {
    let form =
        [("grant_type", "client_credentials"), ("client_id", client_id), ("client_secret", secret)];
    let (status, body) = client.post_form(TOKEN_PATH, &form).await?;
    if matches!(status.as_u16(), 400 | 401 | 403) {
        return Err(ProbeError::Auth(format!(
            "Tailscale refused the OAuth client ({status}): check the client ID and secret, \
             and that the client was not deleted."
        )));
    }
    if !status.is_success() {
        return Err(client.status_error(status, &body, TOKEN_PATH));
    }
    let reply: Value = client.decode(&body, TOKEN_PATH)?;
    let token = text(&reply, "access_token").ok_or_else(|| {
        ProbeError::Protocol("Tailscale returned no access_token for the OAuth client".to_string())
    })?;
    let lifetime = reply.get("expires_in").and_then(Value::as_u64).unwrap_or(3600);
    Ok((token.to_string(), Duration::from_secs(lifetime)))
}

#[async_trait]
impl Collector for TailscaleCollector {
    fn kind(&self) -> &'static str {
        "tailscale"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let devices = self.devices(target).await?;
        let watch = Watch::parse(tag(target, "watch"));
        Ok(samples(&devices, &watch, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        self.devices(target).await?;
        Ok(Some("tailscale".to_string()))
    }
}

/// Les appareils qui doivent rester en ligne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Watch {
    /// Ceux qui portent une étiquette (`tag:…`).
    Tagged,
    All,
    /// Des noms d'appareils ou des étiquettes.
    Listed(Vec<String>),
}

impl Watch {
    pub fn parse(raw: Option<&str>) -> Self {
        let items: Vec<String> = raw
            .unwrap_or_default()
            .split(',')
            .map(|item| item.trim().to_ascii_lowercase())
            .filter(|item| !item.is_empty())
            .collect();
        match items.as_slice() {
            [] => Self::Tagged,
            [all] if all == "all" => Self::All,
            _ => Self::Listed(items),
        }
    }

    fn covers(&self, name: &str, tags: &[&str]) -> bool {
        match self {
            Self::Tagged => !tags.is_empty(),
            Self::All => true,
            Self::Listed(items) => items
                .iter()
                .any(|item| item == &name.to_ascii_lowercase() || tags.iter().any(|t| t == item)),
        }
    }
}

/// Nom court : `nas` pour `nas.tail1234.ts.net`, sinon le nom de l'hôte.
fn short_name(device: &Value) -> Option<&str> {
    text(device, "name")
        .and_then(|name| name.split('.').next())
        .filter(|n| !n.is_empty())
        .or_else(|| text(device, "hostname"))
}

pub fn samples(reply: &Value, watch: &Watch, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();
    let devices: Vec<&Value> =
        reply.get("devices").and_then(Value::as_array).into_iter().flatten().collect();
    let (mut own, mut external, mut online, mut updates, mut unauthorized, mut offline_watched) =
        (0, 0, 0, 0, 0, 0);
    for device in &devices {
        if device.get("isExternal").and_then(Value::as_bool).unwrap_or(false) {
            external += 1;
            continue;
        }
        own += 1;
        let is_online = device.get("connectedToControl").and_then(Value::as_bool).unwrap_or(false);
        let update = device.get("updateAvailable").and_then(Value::as_bool).unwrap_or(false);
        let authorized = device.get("authorized").and_then(Value::as_bool).unwrap_or(true);
        online += usize::from(is_online);
        updates += usize::from(update);
        unauthorized += usize::from(!authorized);
        if own > MAX_NAMED {
            continue;
        }
        let Some(name) = short_name(device) else { continue };
        let tags: Vec<&str> = device
            .get("tags")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let watched = watch.covers(name, &tags);
        let os = text(device, "os").unwrap_or("unknown");
        let named = |sample: Sample| sample.with_label("device", name);
        out.push(named(flag("tailscale_device_online", is_online, ts_ms)));
        out.push(named(flag("tailscale_device_watched", watched, ts_ms)));
        out.push(named(flag("tailscale_device_update_available", update, ts_ms)));
        out.push(named(flag("tailscale_device_authorized", authorized, ts_ms)));
        // En ligne : `lastSeen` est absent ou vieux, le silence vaut zéro.
        let offline = if is_online {
            Some(0.0)
        } else {
            device.get("lastSeen").and_then(Value::as_str).and_then(|t| age_seconds(t, ts_ms))
        };
        if let Some(offline) = offline {
            out.push(named(g("tailscale_device_offline_seconds", offline)));
            if watched && !is_online {
                offline_watched += 1;
            }
        }
        let expiry_disabled =
            device.get("keyExpiryDisabled").and_then(Value::as_bool).unwrap_or(false);
        if !expiry_disabled
            && let Some(left) =
                device.get("expires").and_then(Value::as_str).and_then(|t| seconds_until(t, ts_ms))
        {
            out.push(named(g("tailscale_device_key_expiry_seconds", left)));
        }
        let version =
            text(device, "clientVersion").map(|v| v.split('-').next().unwrap_or(v)).unwrap_or("");
        out.push(
            named(g("tailscale_device_info", 1.0))
                .with_label("os", os)
                .with_label("client_version", version)
                .with_label("tags", tags.join(",")),
        );
    }
    out.push(g("tailscale_devices", own as f64));
    out.push(g("tailscale_devices_online", online as f64));
    out.push(g("tailscale_devices_external", external as f64));
    out.push(g("tailscale_devices_update_available", updates as f64));
    out.push(g("tailscale_devices_unauthorized", unauthorized as f64));
    out.push(g("tailscale_devices_watched_offline", offline_watched as f64));
    out
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::extract::Form;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::{get, post};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    /// Liste construite d'après les champs du client Go officiel
    /// (`tailscale-client-go-v2`, `devices.go`) : un routeur étiqueté à jour
    /// près, un NAS étiqueté hors ligne dont la clé expire dans cinq jours, un
    /// téléphone en attente d'approbation, un appareil partagé par un autre
    /// tailnet.
    const DEVICES: &str = include_str!("testdata/devices.json");

    const NOW: &str = "2026-09-30T09:00:00Z";

    fn now() -> i64 {
        chrono::DateTime::parse_from_rfc3339(NOW).unwrap().timestamp_millis()
    }

    #[test]
    fn appareils_du_tailnet() {
        let reply: Value = serde_json::from_str(DEVICES).unwrap();
        let s = samples(&reply, &Watch::Tagged, now());
        assert_eq!(value(&s, "tailscale_devices", &[]), 3.0);
        assert_eq!(value(&s, "tailscale_devices_external", &[]), 1.0);
        assert_eq!(value(&s, "tailscale_devices_online", &[]), 1.0);
        assert!(find(&s, "tailscale_device_online", &[("device", "shared-printer")]).is_none());
        assert_eq!(value(&s, "tailscale_device_offline_seconds", &[("device", "gateway")]), 0.0);
        assert_eq!(
            value(&s, "tailscale_device_offline_seconds", &[("device", "nas")]),
            47.0 * 60.0 + 16.0
        );
        assert_eq!(value(&s, "tailscale_device_watched", &[("device", "nas")]), 1.0);
        assert_eq!(value(&s, "tailscale_device_watched", &[("device", "bob-phone")]), 0.0);
        assert_eq!(value(&s, "tailscale_devices_watched_offline", &[]), 1.0);
        assert!(
            find(&s, "tailscale_device_key_expiry_seconds", &[("device", "gateway")]).is_none(),
            "key expiry disabled"
        );
        assert_eq!(
            value(&s, "tailscale_device_key_expiry_seconds", &[("device", "nas")]),
            5.0 * 86_400.0
        );
        assert_eq!(value(&s, "tailscale_device_update_available", &[("device", "gateway")]), 1.0);
        assert_eq!(value(&s, "tailscale_device_authorized", &[("device", "bob-phone")]), 0.0);
        assert_eq!(value(&s, "tailscale_devices_unauthorized", &[]), 1.0);
        let info = find(&s, "tailscale_device_info", &[("device", "gateway")]).unwrap();
        assert_eq!(info.labels["client_version"], "1.86.2");
        assert_eq!(info.labels["tags"], "tag:server");
    }

    #[test]
    fn la_liste_a_surveiller() {
        assert_eq!(Watch::parse(None), Watch::Tagged);
        assert_eq!(Watch::parse(Some(" ALL ")), Watch::All);
        let listed = Watch::parse(Some("bob-phone, tag:Router"));
        assert!(listed.covers("Bob-Phone", &[]));
        assert!(listed.covers("x", &["tag:router"]));
        assert!(!listed.covers("nas", &["tag:server"]));
    }

    #[tokio::test]
    async fn client_oauth_puis_jeton_garde() {
        let logins = Arc::new(AtomicUsize::new(0));
        let counted = logins.clone();
        let app = Router::new()
            .route(
                TOKEN_PATH,
                post(move |Form(form): Form<HashMap<String, String>>| {
                    let counted = counted.clone();
                    async move {
                        if form.get("client_secret").map(String::as_str) != Some("tskey-client-x") {
                            return (StatusCode::UNAUTHORIZED, r#"{"message":"invalid client"}"#.to_string());
                        }
                        counted.fetch_add(1, Ordering::SeqCst);
                        (
                            StatusCode::OK,
                            r#"{"access_token":"tskey-oauth","token_type":"Bearer","expires_in":3600,"scope":"devices:core:read"}"#.to_string(),
                        )
                    }
                }),
            )
            .route(
                "/api/v2/tailnet/{tailnet}/devices",
                get(|headers: HeaderMap| async move {
                    match headers.get("authorization").and_then(|v| v.to_str().ok()) {
                        Some("Bearer tskey-oauth") | Some("Bearer tskey-api-good") => {
                            (StatusCode::OK, DEVICES.to_string())
                        }
                        _ => (StatusCode::UNAUTHORIZED, r#"{"message":"unauthorized"}"#.to_string()),
                    }
                }),
            );
        let address = serve(app).await;
        let collector = TailscaleCollector::new();
        let mut target = cible("tailscale", &format!("http://{address}"), &[]);
        target.credential = Credential::UsernamePassword {
            username: "kClientId".into(),
            password: "tskey-client-x".into(),
        };
        let s = collector.probe(&target).await.unwrap();
        assert_eq!(value(&s, "tailscale_devices", &[]), 3.0);
        collector.probe(&target).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 1, "the token is kept for an hour");

        target.credential = Credential::ApiToken { token: "tskey-api-good".into() };
        assert!(collector.probe(&target).await.is_ok());
        target.credential = Credential::ApiToken { token: "tskey-api-revoked".into() };
        let error = collector.probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        target.credential =
            Credential::UsernamePassword { username: "other".into(), password: "wrong".into() };
        let error = collector.probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }
}
