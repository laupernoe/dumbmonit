//! Microsoft Azure, par l'API Azure Resource Manager (`https://management.azure.com`).
//!
//! Deux lectures sur un abonnement, avec un principal de service (application
//! Entra, flux « client credentials ») qui n'a que le rôle **Reader** :
//!
//! * **Resource Health** (`Microsoft.ResourceHealth/availabilityStatuses`) :
//!   l'état de disponibilité que la plate-forme Azure annonce pour chaque
//!   ressource (`Available`, `Unavailable`, `Degraded`, `Unknown`). C'est la
//!   lecture principale : si elle échoue, la sonde échoue ;
//! * **les alertes Azure Monitor** actives (`Microsoft.AlertsManagement/alerts`,
//!   déclenchées, nouvelles ou acquittées), comptées par sévérité `Sev0`…`Sev4`.
//!   Lecture facultative : refusée ou indisponible, elle n'est pas comptée et
//!   `azure_alerts_readable` vaut 0.
//!
//! Une ressource que son propriétaire a arrêtée (machine virtuelle désallouée :
//! Resource Health la rapporte `Unavailable` avec une cause « initiée par
//! l'utilisateur ») n'est pas une panne : elle est comptée à part
//! (`resources_stopped`) et n'alerte pas.
//!
//! C'est une API sur Internet : l'adresse de la cible est celle de l'API
//! (`management.azure.com`), le garde-fou des adresses privées ne la concerne
//! pas. Le jeton (une heure) est gardé jusqu'à son expiration.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `tenant_id` | — (requis) | Identifiant du locataire Entra de l'application. |
//! | `subscription_id` | — (requis) | Abonnement à surveiller. |
//! | `login_url` | `https://login.microsoftonline.com` | Service de connexion (cloud souverain, tests). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::collections::HashSet;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde_json::Value;

use crate::cloud;
use crate::rest::{MAX_NAMED, RestClient, TokenCache, flag, gauge, now_ms, text};
use crate::selfhosted::options::tag;

pub const DEFAULT_PORT: u16 = 443;
pub const DEFAULT_ADDRESS: &str = "management.azure.com";
const DEFAULT_LOGIN_URL: &str = "https://login.microsoftonline.com";
const SCOPE: &str = "https://management.azure.com/.default";
const HEALTH_API_VERSION: &str = "2022-10-01";
const ALERTS_API_VERSION: &str = "2019-05-05-preview";
/// Pages lues au plus par liste : un abonnement énorme ne bloque pas la sonde.
const MAX_PAGES: usize = 20;
const PREFIX: &str = "azure";

#[derive(Default)]
pub struct AzureCollector {
    tokens: TokenCache,
}

/// Réglages lus sur les étiquettes de la cible.
struct Settings {
    tenant: String,
    subscription: String,
    login_url: String,
}

impl Settings {
    fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let tenant = cloud::url_segment("tenant_id", tag(target, "tenant_id").unwrap_or_default())?;
        let subscription = cloud::url_segment(
            "subscription_id",
            tag(target, "subscription_id").unwrap_or_default(),
        )?;
        let login_url = tag(target, "login_url").unwrap_or(DEFAULT_LOGIN_URL).trim_end_matches('/');
        if !(login_url.starts_with("https://") || login_url.starts_with("http://")) {
            return Err(ProbeError::Config(format!(
                "Invalid login_url \"{login_url}\": expected a full URL such as \
                 {DEFAULT_LOGIN_URL}"
            )));
        }
        Ok(Self { tenant, subscription, login_url: login_url.to_string() })
    }
}

/// Ce que la sonde a lu : Resource Health, et les alertes si elles se lisent.
struct Snapshot {
    health: Vec<Value>,
    alerts: Option<Vec<Value>>,
}

/// Résultat de la lecture d'une liste paginée.
enum Listing {
    Items(Vec<Value>),
    /// Statut non 2xx, corps et chemin de la page refusée.
    Refused(StatusCode, String, String),
}

impl AzureCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pose le jeton d'accès sur le client ; renvoie la clé sous laquelle il est gardé.
    async fn authorize(
        &self,
        target: &Target,
        client: &mut RestClient,
        settings: &Settings,
    ) -> Result<String, ProbeError> {
        let (client_id, secret) = match &target.credential {
            Credential::UsernamePassword { username, password }
                if !username.trim().is_empty() && !password.trim().is_empty() =>
            {
                (username.trim(), password.trim())
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "Azure expects the client ID and a client secret of an Entra application, \
                     configured: {other}"
                )));
            }
        };
        let key = format!("{}:{}:{client_id}", target.id, settings.tenant);
        let token = match self.tokens.get(&key) {
            Some(token) => token,
            None => {
                let login = RestClient::for_base(target, &settings.login_url, "Microsoft")?;
                let (token, lifetime) =
                    cloud::microsoft_token(&login, &settings.tenant, client_id, secret, SCOPE)
                        .await?;
                self.tokens.put(&key, &token, lifetime);
                token
            }
        };
        client.set_header("Authorization", format!("Bearer {token}"));
        Ok(key)
    }

    async fn snapshot(&self, target: &Target) -> Result<Snapshot, ProbeError> {
        let settings = Settings::from_target(target)?;
        let mut client = RestClient::for_target(target, "https", DEFAULT_PORT, "Azure")?;
        let key = self.authorize(target, &mut client, &settings).await?;
        let subscription = &settings.subscription;

        let first = format!(
            "/subscriptions/{subscription}/providers/Microsoft.ResourceHealth/\
             availabilityStatuses?api-version={HEALTH_API_VERSION}"
        );
        let health = match list_all(&client, first).await? {
            Listing::Items(items) => items,
            Listing::Refused(status, body, path) => {
                return Err(self.health_error(&client, &key, subscription, status, &body, &path));
            }
        };

        // Alertes : lecture facultative, un refus n'est pas une panne.
        let mut alerts = Vec::new();
        let mut readable = true;
        for state in ["New", "Acknowledged"] {
            let path = format!(
                "/subscriptions/{subscription}/providers/Microsoft.AlertsManagement/alerts?\
                 api-version={ALERTS_API_VERSION}&monitorCondition=Fired&alertState={state}"
            );
            match list_all(&client, path).await? {
                Listing::Items(items) => alerts.extend(items),
                Listing::Refused(..) => {
                    readable = false;
                    break;
                }
            }
        }
        Ok(Snapshot { health, alerts: readable.then_some(alerts) })
    }

    fn health_error(
        &self,
        client: &RestClient,
        key: &str,
        subscription: &str,
        status: StatusCode,
        body: &str,
        path: &str,
    ) -> ProbeError {
        match status.as_u16() {
            401 => {
                // Jeton gardé révoqué entre-temps : le prochain passage en redemandera un.
                self.tokens.forget(key);
                ProbeError::Auth(
                    "Azure refused the access token (401): the application or its secret may \
                     have been deleted."
                        .to_string(),
                )
            }
            403 => ProbeError::Auth(format!(
                "Azure refused to read the Resource Health of subscription \"{subscription}\" \
                 (403): give the application the Reader role on that subscription."
            )),
            400 | 404 => ProbeError::Config(format!(
                "Azure knows no subscription \"{subscription}\" for this application ({status}): \
                 copy the Subscription ID from the Azure portal, and check the tenant."
            )),
            _ => client.status_error(status, body, path),
        }
    }
}

/// Lit une liste ARM page après page (`value` + `nextLink`), au plus `MAX_PAGES`.
async fn list_all(client: &RestClient, first: String) -> Result<Listing, ProbeError> {
    let mut items = Vec::new();
    let mut path = first;
    for _ in 0..MAX_PAGES {
        let (status, body) = client.get(&path).await?;
        if !status.is_success() {
            return Ok(Listing::Refused(status, body, path));
        }
        let page: Value = client.decode(&body, &path)?;
        if let Some(values) = page.get("value").and_then(Value::as_array) {
            items.extend(values.iter().cloned());
        }
        match text(&page, "nextLink").and_then(next_path) {
            Some(next) => path = next,
            None => break,
        }
    }
    Ok(Listing::Items(items))
}

/// Chemin et requête d'un `nextLink` : l'hôte est ignoré, on reste sur celui de la cible.
fn next_path(link: &str) -> Option<String> {
    if link.starts_with('/') {
        return Some(link.to_string());
    }
    let (_, rest) = link.split_once("://")?;
    let at = rest.find('/')?;
    Some(rest[at..].to_string())
}

#[async_trait]
impl Collector for AzureCollector {
    fn kind(&self) -> &'static str {
        "azure"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let snapshot = self.snapshot(target).await?;
        Ok(samples(&snapshot.health, snapshot.alerts.as_deref(), now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        self.snapshot(target).await?;
        Ok(Some("azure".to_string()))
    }
}

/// Une ressource décrite par l'identifiant d'une entrée Resource Health.
#[derive(Debug, PartialEq, Eq)]
struct Resource {
    name: String,
    /// `Microsoft.Sql/servers/databases`.
    kind: String,
    /// Dernier type : `databases`.
    short_kind: String,
    group: String,
}

impl Resource {
    /// `None` pour l'entrée de l'abonnement lui-même (aucune ressource derrière).
    fn parse(id: &str) -> Option<Self> {
        let lower = id.to_ascii_lowercase();
        let health_at = lower.rfind("/providers/microsoft.resourcehealth/")?;
        let base = &id[..health_at];
        let base_lower = &lower[..health_at];
        let marker = "/providers/";
        let providers_at = base_lower.rfind(marker)?;
        let parts: Vec<&str> = base[providers_at + marker.len()..].split('/').collect();
        if parts.len() < 3 {
            return None;
        }
        let name = (*parts.last()?).to_string();
        let types: Vec<&str> = parts[1..].iter().step_by(2).copied().collect();
        let short_kind = types.last()?.to_string();
        let kind = format!("{}/{}", parts[0], types.join("/"));
        let group_marker = "/resourcegroups/";
        let group = base_lower
            .find(group_marker)
            .and_then(|at| base[at + group_marker.len()..].split('/').next())
            .unwrap_or_default()
            .to_string();
        Some(Self { name, kind, short_kind, group })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Health {
    Available,
    Degraded,
    Unavailable,
    /// Arrêtée par son propriétaire : pas une panne.
    Stopped,
    Unknown,
}

fn classify(state: &str, reason: &str) -> Health {
    let reason: String =
        reason.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_lowercase();
    let user_initiated = reason.contains("userinitiated") || reason.contains("customerinitiated");
    match state.to_ascii_lowercase().as_str() {
        "available" => Health::Available,
        "degraded" => Health::Degraded,
        "unavailable" if user_initiated => Health::Stopped,
        "unavailable" => Health::Unavailable,
        _ => Health::Unknown,
    }
}

/// Nom de la règle d'alerte : dernier segment de son identifiant ARM.
fn rule_name(essentials: &Value) -> &str {
    text(essentials, "alertRule")
        .and_then(|rule| rule.rsplit('/').next())
        .filter(|name| !name.is_empty())
        .or_else(|| text(essentials, "targetResourceName"))
        .unwrap_or("alert")
}

/// Rang d'une sévérité `Sev0`…`Sev4`.
fn severity_rank(severity: &str) -> Option<usize> {
    let prefix = severity.get(..3).filter(|p| p.eq_ignore_ascii_case("sev"))?;
    let rank: usize = severity[prefix.len()..].parse().ok()?;
    (rank < 5).then_some(rank)
}

pub fn samples(health: &[Value], alerts: Option<&[Value]>, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(&format!("{PREFIX}_{name}"), value, ts_ms);
    let mut out = Vec::new();
    let (mut total, mut available, mut degraded, mut unavailable, mut stopped, mut unknown) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut named = 0usize;
    for entry in health {
        let Some(resource) = text(entry, "id").and_then(Resource::parse) else { continue };
        let properties = entry.get("properties");
        let state = properties.and_then(|p| text(p, "availabilityState")).unwrap_or("Unknown");
        let reason = properties.and_then(|p| text(p, "reasonType")).unwrap_or_default();
        let verdict = classify(state, reason);
        total += 1;
        let (detail, ok) = match verdict {
            Health::Available => {
                available += 1;
                continue;
            }
            Health::Unknown => {
                unknown += 1;
                continue;
            }
            Health::Degraded => {
                degraded += 1;
                ("Degraded", false)
            }
            Health::Unavailable => {
                unavailable += 1;
                ("Unavailable", false)
            }
            Health::Stopped => {
                stopped += 1;
                ("Stopped by user", true)
            }
        };
        named += 1;
        if named > MAX_NAMED {
            continue;
        }
        let label = format!("{} ({})", resource.name, resource.short_kind);
        out.push(cloud::item(PREFIX, "Resources", &label, ok, detail, ts_ms));
        if !ok {
            out.push(
                g("resource_unhealthy", 1.0)
                    .with_label("resource", &resource.name)
                    .with_label("type", &resource.kind)
                    .with_label("resource_group", &resource.group)
                    .with_label("state", detail),
            );
        }
    }
    out.push(g("resources", total as f64));
    out.push(g("resources_available", available as f64));
    out.push(g("resources_degraded", degraded as f64));
    out.push(g("resources_unavailable", unavailable as f64));
    out.push(g("resources_stopped", stopped as f64));
    out.push(g("resources_unknown", unknown as f64));
    out.push(cloud::figure(PREFIX, "Resources", "", total as f64, ts_ms));
    out.push(cloud::figure(PREFIX, "Unavailable", "", unavailable as f64, ts_ms));
    out.push(cloud::figure(PREFIX, "Degraded", "", degraded as f64, ts_ms));

    out.push(flag(&format!("{PREFIX}_alerts_readable"), alerts.is_some(), ts_ms));
    if let Some(alerts) = alerts {
        let mut by_severity = [0usize; 5];
        let mut seen = HashSet::new();
        let mut listed = 0usize;
        for alert in alerts {
            let Some(essentials) = alert.get("properties").and_then(|p| p.get("essentials")) else {
                continue;
            };
            let severity = text(essentials, "severity").unwrap_or("Sev4");
            if let Some(rank) = severity_rank(severity) {
                by_severity[rank] += 1;
            }
            let target = text(essentials, "targetResourceName").unwrap_or("subscription");
            let name = format!("{} on {target}", rule_name(essentials));
            if listed < MAX_NAMED && seen.insert(name.clone()) {
                listed += 1;
                out.push(cloud::item(PREFIX, "Alerts", &name, false, severity, ts_ms));
            }
        }
        for (rank, count) in by_severity.iter().enumerate() {
            out.push(
                g("alerts_active", *count as f64).with_label("severity", format!("Sev{rank}")),
            );
        }
        let all = alerts.len();
        out.push(g("alerts_total", all as f64));
        out.push(g("alerts_high_active", (by_severity[0] + by_severity[1]) as f64));
        out.push(cloud::figure(PREFIX, "Active alerts", "", all as f64, ts_ms));
    }
    out
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::extract::{RawQuery, State};
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::{get, post};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    /// Réponses construites d'après la documentation de l'API REST Resource
    /// Health (`availabilityStatuses - List By Subscription Id`, 2022-10-01) et
    /// de l'API Alerts Management (`Alerts - Get All`, 2019-05-05-preview).
    const HEALTH_1: &str = include_str!("testdata/health_page1.json");
    const HEALTH_2: &str = include_str!("testdata/health_page2.json");
    const ALERTS_NEW: &str = include_str!("testdata/alerts_new.json");
    const ALERTS_ACK: &str = include_str!("testdata/alerts_acknowledged.json");

    const NOW: i64 = 1_792_000_000_000;

    fn values(raw: &str) -> Vec<Value> {
        let page: Value = serde_json::from_str(raw).unwrap();
        page["value"].as_array().unwrap().clone()
    }

    fn all_health() -> Vec<Value> {
        let mut health = values(HEALTH_1);
        health.extend(values(HEALTH_2));
        health
    }

    fn all_alerts() -> Vec<Value> {
        let mut alerts = values(ALERTS_NEW);
        alerts.extend(values(ALERTS_ACK));
        alerts
    }

    #[test]
    fn etats_de_disponibilite() {
        let s = samples(&all_health(), Some(&all_alerts()), NOW);
        assert_eq!(value(&s, "azure_resources", &[]), 6.0, "the subscription entry is no resource");
        assert_eq!(value(&s, "azure_resources_available", &[]), 2.0);
        assert_eq!(value(&s, "azure_resources_unavailable", &[]), 1.0);
        assert_eq!(value(&s, "azure_resources_degraded", &[]), 1.0);
        assert_eq!(value(&s, "azure_resources_stopped", &[]), 1.0);
        assert_eq!(value(&s, "azure_resources_unknown", &[]), 1.0);
        let down = find(&s, "azure_resource_unhealthy", &[("resource", "vm-db")]).unwrap();
        assert_eq!(down.labels["type"], "Microsoft.Compute/virtualMachines");
        assert_eq!(down.labels["resource_group"], "rg-data");
        assert_eq!(down.labels["state"], "Unavailable");
        let sql = find(&s, "azure_resource_unhealthy", &[("resource", "orders")]).unwrap();
        assert_eq!(sql.labels["type"], "Microsoft.Sql/servers/databases");
        assert_eq!(sql.labels["state"], "Degraded");
        assert!(
            find(&s, "azure_resource_unhealthy", &[("resource", "vm-batch")]).is_none(),
            "a deallocated VM is not an outage"
        );
        let stopped = find(
            &s,
            "azure_item_ok",
            &[("group", "Resources"), ("item", "vm-batch (virtualMachines)")],
        )
        .unwrap();
        assert_eq!(stopped.value, 1.0);
        assert_eq!(stopped.labels["detail"], "Stopped by user");
        assert_eq!(
            value(
                &s,
                "azure_item_ok",
                &[("group", "Resources"), ("item", "vm-db (virtualMachines)")]
            ),
            0.0
        );
        assert_eq!(value(&s, "azure_figure", &[("name", "Resources")]), 6.0);
        assert_eq!(value(&s, "azure_figure", &[("name", "Unavailable")]), 1.0);
    }

    #[test]
    fn alertes_par_severite() {
        let s = samples(&all_health(), Some(&all_alerts()), NOW);
        assert_eq!(value(&s, "azure_alerts_readable", &[]), 1.0);
        assert_eq!(value(&s, "azure_alerts_active", &[("severity", "Sev0")]), 1.0);
        assert_eq!(value(&s, "azure_alerts_active", &[("severity", "Sev1")]), 1.0);
        assert_eq!(value(&s, "azure_alerts_active", &[("severity", "Sev2")]), 0.0);
        assert_eq!(value(&s, "azure_alerts_active", &[("severity", "Sev3")]), 1.0);
        assert_eq!(value(&s, "azure_alerts_active", &[("severity", "Sev4")]), 0.0);
        assert_eq!(value(&s, "azure_alerts_total", &[]), 3.0);
        assert_eq!(value(&s, "azure_alerts_high_active", &[]), 2.0);
        let item = find(&s, "azure_item_ok", &[("group", "Alerts"), ("item", "High CPU on vm-db")])
            .unwrap();
        assert_eq!(item.value, 0.0);
        assert_eq!(item.labels["detail"], "Sev1");
        assert_eq!(value(&s, "azure_figure", &[("name", "Active alerts")]), 3.0);
    }

    #[test]
    fn alertes_illisibles() {
        let s = samples(&all_health(), None, NOW);
        assert_eq!(value(&s, "azure_alerts_readable", &[]), 0.0);
        assert!(find(&s, "azure_alerts_active", &[]).is_none());
        assert!(find(&s, "azure_alerts_high_active", &[]).is_none());
        assert_eq!(value(&s, "azure_resources", &[]), 6.0, "Resource Health is still reported");
    }

    #[test]
    fn identifiants_de_ressources() {
        let sub =
            "/subscriptions/s/providers/Microsoft.ResourceHealth/availabilityStatuses/current";
        assert_eq!(Resource::parse(sub), None);
        let group = "/subscriptions/s/resourceGroups/rg/providers/Microsoft.ResourceHealth/\
                     availabilityStatuses/current";
        assert_eq!(Resource::parse(group), None);
        let vm = "/subscriptions/s/resourceGroups/Rg-1/providers/Microsoft.Compute/\
                  virtualMachines/vm1/providers/Microsoft.ResourceHealth/availabilityStatuses/current";
        let vm = Resource::parse(vm).unwrap();
        assert_eq!(vm.name, "vm1");
        assert_eq!(vm.kind, "Microsoft.Compute/virtualMachines");
        assert_eq!(vm.group, "Rg-1");
    }

    #[test]
    fn chemin_d_un_lien_suivant() {
        assert_eq!(
            next_path("https://management.azure.com/subscriptions/s/x?api-version=1&$skiptoken=a")
                .as_deref(),
            Some("/subscriptions/s/x?api-version=1&$skiptoken=a")
        );
        assert_eq!(next_path("/subscriptions/s").as_deref(), Some("/subscriptions/s"));
        assert_eq!(next_path("n'importe quoi"), None);
    }

    /// Faux Azure : service de connexion et Resource Manager sur le même port.
    struct Fake {
        logins: AtomicUsize,
        login_status: u16,
        health_status: u16,
        alerts_status: u16,
    }

    impl Fake {
        fn ok() -> Self {
            Self {
                logins: AtomicUsize::new(0),
                login_status: 200,
                health_status: 200,
                alerts_status: 200,
            }
        }
    }

    async fn fake(state: Arc<Fake>) -> String {
        async fn token(State(fake): State<Arc<Fake>>) -> (StatusCode, String) {
            if fake.login_status != 200 {
                return (
                    StatusCode::from_u16(fake.login_status).unwrap(),
                    r#"{"error":"invalid_client","error_description":"AADSTS7000215: Invalid client secret provided."}"#.to_string(),
                );
            }
            fake.logins.fetch_add(1, Ordering::SeqCst);
            (
                StatusCode::OK,
                r#"{"token_type":"Bearer","expires_in":3599,"access_token":"tok-azure"}"#
                    .to_string(),
            )
        }
        async fn health(
            State(fake): State<Arc<Fake>>,
            headers: HeaderMap,
            RawQuery(query): RawQuery,
        ) -> (StatusCode, String) {
            if headers.get("authorization").and_then(|v| v.to_str().ok())
                != Some("Bearer tok-azure")
            {
                return (StatusCode::UNAUTHORIZED, "{}".to_string());
            }
            if fake.health_status != 200 {
                let body = r#"{"error":{"code":"AuthorizationFailed","message":"no"}}"#;
                return (StatusCode::from_u16(fake.health_status).unwrap(), body.to_string());
            }
            let query = query.unwrap_or_default();
            assert!(query.contains("api-version=2022-10-01"), "{query}");
            let page = if query.contains("skiptoken=p2") { HEALTH_2 } else { HEALTH_1 };
            (StatusCode::OK, page.to_string())
        }
        async fn alerts(
            State(fake): State<Arc<Fake>>,
            RawQuery(query): RawQuery,
        ) -> (StatusCode, String) {
            if fake.alerts_status != 200 {
                return (
                    StatusCode::from_u16(fake.alerts_status).unwrap(),
                    r#"{"error":{"code":"AuthorizationFailed"}}"#.to_string(),
                );
            }
            let query = query.unwrap_or_default();
            assert!(query.contains("monitorCondition=Fired"), "{query}");
            let page =
                if query.contains("alertState=Acknowledged") { ALERTS_ACK } else { ALERTS_NEW };
            (StatusCode::OK, page.to_string())
        }
        let app = Router::new()
            .route("/{tenant}/oauth2/v2.0/token", post(token))
            .route(
                "/subscriptions/{sub}/providers/Microsoft.ResourceHealth/availabilityStatuses",
                get(health),
            )
            .route("/subscriptions/{sub}/providers/Microsoft.AlertsManagement/alerts", get(alerts))
            .with_state(state);
        serve(app).await
    }

    fn cible_azure(address: &str) -> Target {
        let url = format!("http://{address}");
        let mut target = cible(
            "azure",
            &url,
            &[
                ("tenant_id", "contoso.onmicrosoft.com"),
                ("subscription_id", "00000000-0000-0000-0000-0000000000a1"),
                ("login_url", url.as_str()),
            ],
        );
        target.credential = Credential::UsernamePassword {
            username: "app-id".into(),
            password: "app-secret".into(),
        };
        target
    }

    #[tokio::test]
    async fn sonde_complete_pagination_et_jeton_garde() {
        let state = Arc::new(Fake::ok());
        let address = fake(state.clone()).await;
        let collector = AzureCollector::new();
        let target = cible_azure(&address);
        let s = collector.probe(&target).await.unwrap();
        assert_eq!(value(&s, "azure_resources", &[]), 6.0, "both pages are read");
        assert_eq!(value(&s, "azure_alerts_total", &[]), 3.0, "New and Acknowledged");
        collector.probe(&target).await.unwrap();
        assert_eq!(state.logins.load(Ordering::SeqCst), 1, "the token is kept");
        assert_eq!(collector.discover(&target).await.unwrap().as_deref(), Some("azure"));
    }

    #[tokio::test]
    async fn alertes_refusees_sans_echec() {
        let state = Arc::new(Fake { alerts_status: 403, ..Fake::ok() });
        let address = fake(state).await;
        let s = AzureCollector::new().probe(&cible_azure(&address)).await.unwrap();
        assert_eq!(value(&s, "azure_alerts_readable", &[]), 0.0);
        assert_eq!(value(&s, "azure_resources_unavailable", &[]), 1.0);
    }

    #[tokio::test]
    async fn identifiants_refuses() {
        let state = Arc::new(Fake { login_status: 401, ..Fake::ok() });
        let address = fake(state).await;
        let error = AzureCollector::new().probe(&cible_azure(&address)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());

        let state = Arc::new(Fake { health_status: 403, ..Fake::ok() });
        let address = fake(state).await;
        let error = AzureCollector::new().probe(&cible_azure(&address)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(error.to_string().contains("Reader"), "{error}");
    }

    #[tokio::test]
    async fn abonnement_inconnu_est_une_erreur_de_configuration() {
        let state = Arc::new(Fake { health_status: 404, ..Fake::ok() });
        let address = fake(state).await;
        let error = AzureCollector::new().probe(&cible_azure(&address)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[tokio::test]
    async fn panne_du_service_alerte() {
        let state = Arc::new(Fake { health_status: 503, ..Fake::ok() });
        let address = fake(state).await;
        let error = AzureCollector::new().probe(&cible_azure(&address)).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[tokio::test]
    async fn options_obligatoires() {
        let mut target = cible("azure", "http://127.0.0.1:1", &[]);
        target.credential =
            Credential::UsernamePassword { username: "a".into(), password: "b".into() };
        let error = AzureCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        let mut target = cible(
            "azure",
            "http://127.0.0.1:1",
            &[("tenant_id", "t"), ("subscription_id", "s"), ("login_url", "ftp://x")],
        );
        target.credential =
            Credential::UsernamePassword { username: "a".into(), password: "b".into() };
        let error = AzureCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        target.credential = Credential::None;
        let error = AzureCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }
}
