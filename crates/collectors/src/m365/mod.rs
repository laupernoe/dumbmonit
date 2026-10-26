//! Microsoft 365, par Microsoft Graph (`https://graph.microsoft.com/v1.0`).
//!
//! L'accès passe par une application Microsoft Entra (flux « client
//! credentials ») : l'identifiant et le secret de l'application sont échangés
//! contre un jeton d'une heure, gardé jusqu'à son expiration, puis chaque
//! lecture est un GET. Rien n'est jamais écrit.
//!
//! Ce qui est lu :
//!
//! * l'état des services (`/admin/serviceAnnouncement/healthOverviews`,
//!   `ServiceHealth.Read.All`) : lecture principale, la sonde échoue sans elle ;
//! * les incidents et avis en cours (`/admin/serviceAnnouncement/issues`,
//!   `ServiceHealth.Read.All`) ;
//! * les licences (`/subscribedSkus`, `LicenseAssignment.Read.All` ou
//!   `Organization.Read.All`) : unités consommées contre unités achetées ;
//! * les utilisateurs à risque (`/identityProtection/riskyUsers`,
//!   `IdentityRiskyUser.Read.All`, licence Entra ID P2) et, à défaut, les
//!   échecs de connexion de la dernière heure (`/auditLogs/signIns`,
//!   `AuditLog.Read.All`, licence P1).
//!
//! Les lectures facultatives (tout sauf l'état des services) ne font pas
//! échouer la sonde quand Microsoft les refuse : la série correspondante est
//! omise et `<lecture>_readable` vaut 0. Un 401 ou un 403 sur la lecture
//! principale est une erreur d'authentification, un 5xx ou un délai dépassé
//! une panne.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `tenant_id` | (obligatoire) | Identifiant du répertoire (locataire). |
//! | `login_url` | `https://login.microsoftonline.com` | Service de connexion (clouds souverains, tests). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::collections::BTreeMap;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::Value;

use crate::cloud;
use crate::rest::{MAX_NAMED, RestClient, TokenCache, flag, gauge, now_ms, number, text};
use crate::selfhosted::options::tag;

pub const DEFAULT_PORT: u16 = 443;
pub const DEFAULT_ADDRESS: &str = "graph.microsoft.com";
pub const DEFAULT_LOGIN: &str = "https://login.microsoftonline.com";
const SCOPE: &str = "https://graph.microsoft.com/.default";
const PRODUCT: &str = "Microsoft 365";
/// Préfixe des séries (le serveur y ajoute `dumbmonit_`).
const PREFIX: &str = "m365";
/// Pages suivies au plus pour une même liste.
const MAX_PAGES: usize = 20;
/// Incidents décrits un par un sur la page.
const MAX_INCIDENT_ITEMS: usize = 20;

const HEALTH_PATH: &str = "/v1.0/admin/serviceAnnouncement/healthOverviews";
const ISSUES_PATH: &str =
    "/v1.0/admin/serviceAnnouncement/issues?$filter=isResolved%20eq%20false&$top=100";
const SKUS_PATH: &str = "/v1.0/subscribedSkus";
const RISKY_PATH: &str =
    "/v1.0/identityProtection/riskyUsers?$filter=riskState%20eq%20'atRisk'&$top=500";

/// Une liste Graph lue en entier, ou le code du refus qui l'en a empêchée.
pub(crate) enum Fetched {
    Values(Vec<Value>),
    /// 400, 403 ou 404 : fonction non autorisée, sous licence ou absente.
    Denied(u16),
}

impl Fetched {
    pub(crate) fn values(self) -> Option<Vec<Value>> {
        match self {
            Self::Values(values) => Some(values),
            Self::Denied(_) => None,
        }
    }
}

/// Lit une liste Graph en suivant `@odata.nextLink` (au plus `max_pages`
/// pages). Un lien qui ne part pas de l'hôte interrogé est ignoré.
pub(crate) async fn fetch_all(
    client: &RestClient,
    first: &str,
    max_pages: usize,
) -> Result<Fetched, ProbeError> {
    let mut path = first.to_string();
    let mut values = Vec::new();
    for _ in 0..max_pages {
        let (status, body) = client.get(&path).await?;
        match status.as_u16() {
            200..=299 => {}
            code @ (400 | 403 | 404) => return Ok(Fetched::Denied(code)),
            _ => return Err(client.status_error(status, &body, &path)),
        }
        let page: Value = client.decode(&body, &path)?;
        if let Some(items) = page.get("value").and_then(Value::as_array) {
            values.extend(items.iter().cloned());
        }
        let next = page
            .get("@odata.nextLink")
            .and_then(Value::as_str)
            .and_then(|link| link.strip_prefix(client.base_url()))
            .filter(|rest| !rest.is_empty());
        match next {
            Some(rest) => path = rest.to_string(),
            None => break,
        }
    }
    Ok(Fetched::Values(values))
}

/// L'erreur d'une lecture obligatoire refusée.
pub(crate) fn denied(code: u16, path: &str, permission: &str) -> ProbeError {
    let path = path.split('?').next().unwrap_or(path);
    match code {
        403 => ProbeError::Auth(format!(
            "Microsoft Graph refused {path} (403): grant the application the {permission} \
             application permission and give admin consent."
        )),
        404 => ProbeError::Config(format!(
            "Microsoft Graph has no {path} for this tenant: check the tenant ID and the login \
             URL."
        )),
        _ => ProbeError::Protocol(format!("Microsoft Graph rejected {path} ({code}).")),
    }
}

/// Client Graph muni d'un jeton d'accès (échangé, ou repris du cache), avec la
/// clé sous laquelle ce jeton est gardé.
pub(crate) async fn connect(
    tokens: &TokenCache,
    target: &Target,
    product: &'static str,
) -> Result<(RestClient, String), ProbeError> {
    let tenant = cloud::url_segment("tenant_id", tag(target, "tenant_id").unwrap_or_default())?;
    let Credential::UsernamePassword { username, password } = &target.credential else {
        return Err(ProbeError::Config(format!(
            "{product} expects the client ID and client secret of an Entra application, \
             configured: {}",
            target.credential
        )));
    };
    let (client_id, secret) = (username.trim(), password.trim());
    if client_id.is_empty() || secret.is_empty() {
        return Err(ProbeError::Config(format!(
            "{product} needs the client ID and the client secret of the Entra application"
        )));
    }
    let login_url = tag(target, "login_url").unwrap_or(DEFAULT_LOGIN);
    if !(login_url.starts_with("https://") || login_url.starts_with("http://")) {
        return Err(ProbeError::Config(format!(
            "Invalid login URL \"{login_url}\": expected https://login.microsoftonline.com or the \
             login URL of your cloud"
        )));
    }
    let mut client = RestClient::for_target(target, "https", DEFAULT_PORT, product)?;
    let key = format!("{}:{tenant}:{client_id}", target.id);
    let token = match tokens.get(&key) {
        Some(token) => token,
        None => {
            let login = RestClient::for_base(target, login_url, product)?;
            let (token, lifetime) =
                cloud::microsoft_token(&login, &tenant, client_id, secret, SCOPE).await?;
            tokens.put(&key, &token, lifetime);
            token
        }
    };
    client.set_header("Authorization", format!("Bearer {token}"));
    Ok((client, key))
}

#[derive(Default)]
pub struct M365Collector {
    tokens: TokenCache,
}

impl M365Collector {
    pub fn new() -> Self {
        Self::default()
    }

    async fn read(&self, target: &Target, ts_ms: i64) -> Result<Reading, ProbeError> {
        let (client, key) = connect(&self.tokens, target, PRODUCT).await?;
        let result = collect(&client, ts_ms).await;
        if matches!(result, Err(ProbeError::Auth(_))) {
            // Jeton révoqué entre-temps : le prochain passage en redemandera un.
            self.tokens.forget(&key);
        }
        result
    }
}

/// Ce que Graph a répondu, avant d'en faire des séries.
pub struct Reading {
    pub health: Vec<Value>,
    pub issues: Option<Vec<Value>>,
    pub skus: Option<Vec<Value>>,
    pub risk: Risk,
}

/// Le risque lu : utilisateurs à risque, sinon échecs de connexion, sinon rien.
pub enum Risk {
    Users(Vec<Value>),
    Signins(usize),
    Unavailable,
}

/// Début de la fenêtre des échecs de connexion, au format du filtre OData.
fn signins_path(ts_ms: i64) -> String {
    let since = chrono::DateTime::from_timestamp_millis(ts_ms - 3_600_000).unwrap_or_default();
    format!(
        "/v1.0/auditLogs/signIns?$filter=createdDateTime%20ge%20{}%20and%20\
         status/errorCode%20ne%200&$top=100",
        since.format("%Y-%m-%dT%H:%M:%SZ")
    )
}

async fn collect(client: &RestClient, ts_ms: i64) -> Result<Reading, ProbeError> {
    let health = match fetch_all(client, HEALTH_PATH, MAX_PAGES).await? {
        Fetched::Values(values) => values,
        Fetched::Denied(code) => return Err(denied(code, HEALTH_PATH, "ServiceHealth.Read.All")),
    };
    let issues = fetch_all(client, ISSUES_PATH, MAX_PAGES).await?.values();
    let skus = fetch_all(client, SKUS_PATH, MAX_PAGES).await?.values();
    let risk = match fetch_all(client, RISKY_PATH, MAX_PAGES).await?.values() {
        Some(users) => Risk::Users(users),
        // Pas de licence P2 (ou droit absent) : repli sur les échecs de connexion.
        None => match fetch_all(client, &signins_path(ts_ms), 1).await?.values() {
            Some(failures) => Risk::Signins(failures.len()),
            None => Risk::Unavailable,
        },
    };
    Ok(Reading { health, issues, skus, risk })
}

#[async_trait]
impl Collector for M365Collector {
    fn kind(&self) -> &'static str {
        "m365"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let ts_ms = now_ms();
        let reading = self.read(target, ts_ms).await?;
        Ok(samples(&reading, ts_ms))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        self.read(target, now_ms()).await?;
        Ok(Some("m365".to_string()))
    }
}

/// Un état de santé de service qui n'appelle aucune action.
fn service_healthy(status: &str) -> bool {
    matches!(
        status,
        "serviceOperational"
            | "serviceRestored"
            | "postIncidentReviewPublished"
            | "falsePositive"
            | "resolved"
            | "resolvedExternal"
            | "mitigated"
            | "mitigatedExternal"
    )
}

/// Libellé court et stable d'un état (il sert d'étiquette `detail`).
fn status_label(status: &str) -> &'static str {
    match status {
        "serviceOperational" => "Operational",
        "investigating" => "Investigating",
        "restoringService" => "Restoring service",
        "verifyingService" => "Verifying service",
        "serviceRestored" => "Restored",
        "postIncidentReviewPublished" => "Post-incident review",
        "serviceDegradation" => "Degraded",
        "serviceInterruption" => "Interrupted",
        "extendedRecovery" => "Extended recovery",
        "falsePositive" => "False positive",
        "investigationSuspended" => "Investigation suspended",
        "resolved" | "resolvedExternal" => "Resolved",
        "mitigated" | "mitigatedExternal" => "Mitigated",
        "confirmed" => "Confirmed",
        "reported" => "Reported",
        _ => "Unknown",
    }
}

/// Licences gratuites ou illimitées en pratique : une jauge n'a pas de sens.
fn is_free_sku(part: &str, enabled: f64) -> bool {
    let part = part.to_ascii_uppercase();
    enabled >= 1_000_000.0
        || ["FREE", "VIRAL", "EXPLORATORY", "POWER_BI_STANDARD", "WINDOWS_STORE"]
            .iter()
            .any(|word| part.contains(word))
}

pub fn samples(reading: &Reading, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(&format!("{PREFIX}_{name}"), value, ts_ms);
    let f = |name: &str, value: bool| flag(&format!("{PREFIX}_{name}"), value, ts_ms);
    let mut out = Vec::new();

    // Services.
    let (mut services, mut degraded) = (0usize, 0usize);
    for service in &reading.health {
        let Some(name) = text(service, "service") else { continue };
        let status = text(service, "status").unwrap_or("unknown");
        let ok = service_healthy(status);
        services += 1;
        degraded += usize::from(!ok);
        if services <= MAX_NAMED {
            out.push(f("service_ok", ok).with_label("service", name));
            out.push(cloud::item(PREFIX, "Services", name, ok, status_label(status), ts_ms));
        }
    }
    out.push(g("services", services as f64));
    out.push(g("services_degraded", degraded as f64));
    out.push(cloud::figure(PREFIX, "Services degraded", "", degraded as f64, ts_ms));

    // Incidents et avis en cours.
    out.push(f("issues_readable", reading.issues.is_some()));
    if let Some(issues) = &reading.issues {
        let (mut incidents, mut advisories, mut listed) = (0usize, 0usize, 0usize);
        let mut by_service: BTreeMap<(&str, &str), usize> = BTreeMap::new();
        for issue in issues {
            if issue.get("isResolved").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            let is_incident = text(issue, "classification") == Some("incident");
            let class = if is_incident { "incident" } else { "advisory" };
            incidents += usize::from(is_incident);
            advisories += usize::from(!is_incident);
            let service = text(issue, "service").unwrap_or("unknown");
            *by_service.entry((service, class)).or_default() += 1;
            if is_incident
                && listed < MAX_INCIDENT_ITEMS
                && let Some(title) = text(issue, "title")
            {
                listed += 1;
                out.push(cloud::item(PREFIX, "Active incidents", title, false, "Incident", ts_ms));
            }
        }
        out.push(g("incidents_active", incidents as f64));
        out.push(g("advisories_active", advisories as f64));
        for ((service, class), count) in by_service.into_iter().take(MAX_NAMED) {
            out.push(
                g("service_issues", count as f64)
                    .with_label("service", service)
                    .with_label("classification", class),
            );
        }
        out.push(cloud::figure(PREFIX, "Active incidents", "", incidents as f64, ts_ms));
    }

    // Licences.
    out.push(f("licenses_readable", reading.skus.is_some()));
    if let Some(skus) = &reading.skus {
        let (mut exhausted, mut named) = (0usize, 0usize);
        for sku in skus {
            let Some(part) = text(sku, "skuPartNumber") else { continue };
            let enabled = number(sku.pointer("/prepaidUnits/enabled")).unwrap_or(0.0);
            let consumed = number(sku.get("consumedUnits")).unwrap_or(0.0);
            if enabled <= 0.0 || is_free_sku(part, enabled) {
                continue;
            }
            let percent = consumed / enabled * 100.0;
            let full = percent >= 100.0;
            exhausted += usize::from(full);
            named += 1;
            if named > MAX_NAMED {
                continue;
            }
            let named_sku = |sample: Sample| sample.with_label("sku", part);
            out.push(named_sku(g("license_used_percent", percent)));
            out.push(named_sku(g("license_consumed", consumed)));
            out.push(named_sku(g("license_enabled", enabled)));
            out.push(named_sku(g("license_available", (enabled - consumed).max(0.0))));
            let detail = if full {
                "Exhausted"
            } else if percent >= 95.0 {
                "Nearly full"
            } else {
                "Available"
            };
            out.push(cloud::item(PREFIX, "Licenses", part, !full, detail, ts_ms));
        }
        out.push(g("licenses_exhausted", exhausted as f64));
        out.push(cloud::figure(PREFIX, "Licenses exhausted", "", exhausted as f64, ts_ms));
    }

    // Risque.
    match &reading.risk {
        Risk::Users(users) => {
            let high = users.iter().filter(|u| text(u, "riskLevel") == Some("high")).count();
            out.push(f("risky_readable", true));
            out.push(g("risky_users", users.len() as f64));
            out.push(g("risky_users_high", high as f64));
            out.push(cloud::figure(PREFIX, "Risky users", "", users.len() as f64, ts_ms));
        }
        Risk::Signins(failures) => {
            out.push(f("risky_readable", false));
            out.push(f("signins_readable", true));
            out.push(g("signin_failures_1h", *failures as f64));
            out.push(cloud::figure(PREFIX, "Failed sign-ins (1 h)", "", *failures as f64, ts_ms));
        }
        Risk::Unavailable => {
            out.push(f("risky_readable", false));
            out.push(f("signins_readable", false));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::extract::{Form, Query};
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::{get, post};

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après les exemples de la documentation Microsoft
    // Graph v1.0 (serviceHealth, issues, subscribedSku, riskyUser, signIn).
    const HEALTH: &str = include_str!("testdata/health_overviews.json");
    const ISSUES_1: &str = include_str!("testdata/issues_page1.json");
    const ISSUES_2: &str = include_str!("testdata/issues_page2.json");
    const SKUS: &str = include_str!("testdata/subscribed_skus.json");
    const RISKY: &str = include_str!("testdata/risky_users.json");
    const SIGNINS: &str = include_str!("testdata/signins.json");

    fn list(raw: &str) -> Vec<Value> {
        let v: Value = serde_json::from_str(raw).unwrap();
        v["value"].as_array().unwrap().clone()
    }

    fn reading(risk: Risk) -> Reading {
        let mut issues = list(&ISSUES_1.replace("{{base}}", "http://x"));
        issues.extend(list(ISSUES_2));
        Reading { health: list(HEALTH), issues: Some(issues), skus: Some(list(SKUS)), risk }
    }

    #[test]
    fn series_de_la_lecture_complete() {
        let s = samples(&reading(Risk::Users(list(RISKY))), 1_000);
        assert_eq!(value(&s, "m365_services", &[]), 4.0);
        assert_eq!(value(&s, "m365_services_degraded", &[]), 1.0);
        assert_eq!(value(&s, "m365_service_ok", &[("service", "Exchange Online")]), 0.0);
        assert_eq!(value(&s, "m365_service_ok", &[("service", "Microsoft Entra")]), 1.0);
        let item = find(&s, "m365_item_ok", &[("item", "Exchange Online")]).unwrap();
        assert_eq!(item.labels["group"], "Services");
        assert_eq!(item.labels["detail"], "Degraded");
        assert_eq!(value(&s, "m365_incidents_active", &[]), 1.0);
        assert_eq!(value(&s, "m365_advisories_active", &[]), 1.0);
        assert_eq!(
            value(
                &s,
                "m365_service_issues",
                &[("service", "Exchange Online"), ("classification", "incident")]
            ),
            1.0
        );
        assert!(
            find(&s, "m365_item_ok", &[("item", "Users may be unable to send email")]).is_some()
        );
        assert_eq!(value(&s, "m365_issues_readable", &[]), 1.0);
        assert_eq!(value(&s, "m365_licenses_readable", &[]), 1.0);
        assert_eq!(value(&s, "m365_license_used_percent", &[("sku", "ENTERPRISEPACK")]), 100.0);
        assert_eq!(value(&s, "m365_license_used_percent", &[("sku", "SPE_E3")]), 96.0);
        assert_eq!(
            value(&s, "m365_license_used_percent", &[("sku", "O365_BUSINESS_PREMIUM")]),
            20.0
        );
        assert_eq!(value(&s, "m365_license_available", &[("sku", "SPE_E3")]), 4.0);
        assert_eq!(value(&s, "m365_license_enabled", &[("sku", "SPE_E3")]), 100.0);
        assert_eq!(value(&s, "m365_license_consumed", &[("sku", "SPE_E3")]), 96.0);
        assert_eq!(value(&s, "m365_licenses_exhausted", &[]), 1.0);
        for ignored in ["FLOW_FREE", "POWER_BI_STANDARD", "EMS"] {
            assert!(find(&s, "m365_license_used_percent", &[("sku", ignored)]).is_none());
        }
        let sku = find(&s, "m365_item_ok", &[("item", "SPE_E3")]).unwrap();
        assert_eq!(sku.value, 1.0);
        assert_eq!(sku.labels["detail"], "Nearly full");
        assert_eq!(value(&s, "m365_item_ok", &[("item", "ENTERPRISEPACK")]), 0.0);
        assert_eq!(value(&s, "m365_risky_readable", &[]), 1.0);
        assert_eq!(value(&s, "m365_risky_users", &[]), 2.0);
        assert_eq!(value(&s, "m365_risky_users_high", &[]), 1.0);
        assert!(find(&s, "m365_signins_readable", &[]).is_none());
        assert_eq!(value(&s, "m365_figure", &[("name", "Risky users")]), 2.0);
    }

    #[test]
    fn repli_sur_les_echecs_de_connexion() {
        let s = samples(&reading(Risk::Signins(list(SIGNINS).len())), 1_000);
        assert_eq!(value(&s, "m365_risky_readable", &[]), 0.0);
        assert_eq!(value(&s, "m365_signins_readable", &[]), 1.0);
        assert_eq!(value(&s, "m365_signin_failures_1h", &[]), 3.0);
        assert!(find(&s, "m365_risky_users", &[]).is_none());
        let s = samples(&reading(Risk::Unavailable), 1_000);
        assert_eq!(value(&s, "m365_risky_readable", &[]), 0.0);
        assert_eq!(value(&s, "m365_signins_readable", &[]), 0.0);
        assert!(find(&s, "m365_signin_failures_1h", &[]).is_none());
    }

    #[test]
    fn lectures_refusees_omises() {
        let reading =
            Reading { health: list(HEALTH), issues: None, skus: None, risk: Risk::Unavailable };
        let s = samples(&reading, 1_000);
        assert_eq!(value(&s, "m365_issues_readable", &[]), 0.0);
        assert_eq!(value(&s, "m365_licenses_readable", &[]), 0.0);
        assert!(find(&s, "m365_incidents_active", &[]).is_none());
        assert!(find(&s, "m365_license_used_percent", &[]).is_none());
        assert_eq!(value(&s, "m365_services_degraded", &[]), 1.0);
    }

    #[test]
    fn fenetre_des_echecs_de_connexion() {
        let at = chrono::DateTime::parse_from_rfc3339("2026-10-26T09:00:00Z")
            .unwrap()
            .timestamp_millis();
        let path = signins_path(at);
        assert!(path.contains("createdDateTime%20ge%202026-10-26T08:00:00Z"), "{path}");
        assert!(path.contains("status/errorCode%20ne%200"), "{path}");
    }

    /// Comportement du faux Graph : statut de l'état des services, des
    /// utilisateurs à risque et des connexions.
    #[derive(Clone, Copy)]
    struct Behaviour {
        health: u16,
        risky: u16,
        signins: u16,
    }

    const OK: Behaviour = Behaviour { health: 200, risky: 200, signins: 200 };

    fn reply(status: u16, body: &str) -> (StatusCode, String) {
        let status = StatusCode::from_u16(status).unwrap();
        if status.is_success() {
            (status, body.to_string())
        } else {
            (status, r#"{"error":{"code":"Forbidden","message":"denied"}}"#.to_string())
        }
    }

    /// Faux Microsoft (connexion et Graph sur le même port) ; compte les
    /// échanges de jeton.
    async fn fake(b: Behaviour) -> (String, Arc<AtomicUsize>) {
        let logins = Arc::new(AtomicUsize::new(0));
        let counted = logins.clone();
        let app = Router::new()
            .route(
                "/contoso/oauth2/v2.0/token",
                post(move |Form(form): Form<HashMap<String, String>>| {
                    let counted = counted.clone();
                    async move {
                        let good = form.get("client_secret").map(String::as_str) == Some("s3cret")
                            && form.get("scope").map(String::as_str)
                                == Some("https://graph.microsoft.com/.default");
                        if !good {
                            return (
                                StatusCode::UNAUTHORIZED,
                                r#"{"error":"invalid_client","error_description":"AADSTS7000215: Invalid client secret provided."}"#.to_string(),
                            );
                        }
                        counted.fetch_add(1, Ordering::SeqCst);
                        (
                            StatusCode::OK,
                            r#"{"token_type":"Bearer","expires_in":3599,"access_token":"graph-token"}"#.to_string(),
                        )
                    }
                }),
            )
            .route(
                "/v1.0/admin/serviceAnnouncement/healthOverviews",
                get(move |headers: HeaderMap| async move {
                    if headers.get("authorization").and_then(|v| v.to_str().ok())
                        != Some("Bearer graph-token")
                    {
                        return (StatusCode::UNAUTHORIZED, "{}".to_string());
                    }
                    reply(b.health, HEALTH)
                }),
            )
            .route(
                "/v1.0/admin/serviceAnnouncement/issues",
                get(|headers: HeaderMap, Query(query): Query<HashMap<String, String>>| async move {
                    if query.get("$filter").map(String::as_str) != Some("isResolved eq false") {
                        return (StatusCode::BAD_REQUEST, "{}".to_string());
                    }
                    if query.contains_key("$skiptoken") {
                        return (StatusCode::OK, ISSUES_2.to_string());
                    }
                    let host = headers.get("host").and_then(|h| h.to_str().ok()).unwrap_or_default();
                    (StatusCode::OK, ISSUES_1.replace("{{base}}", &format!("http://{host}")))
                }),
            )
            .route("/v1.0/subscribedSkus", get(|| async { (StatusCode::OK, SKUS.to_string()) }))
            .route(
                "/v1.0/identityProtection/riskyUsers",
                get(move |Query(query): Query<HashMap<String, String>>| async move {
                    if query.get("$filter").map(String::as_str) != Some("riskState eq 'atRisk'") {
                        return (StatusCode::BAD_REQUEST, "{}".to_string());
                    }
                    reply(b.risky, RISKY)
                }),
            )
            .route(
                "/v1.0/auditLogs/signIns",
                get(move |Query(query): Query<HashMap<String, String>>| async move {
                    let filter = query.get("$filter").map(String::as_str).unwrap_or_default();
                    if !filter.contains("createdDateTime ge ") || !filter.contains("status/errorCode ne 0") {
                        return (StatusCode::BAD_REQUEST, "{}".to_string());
                    }
                    reply(b.signins, SIGNINS)
                }),
            );
        (serve(app).await, logins)
    }

    fn target(address: &str) -> Target {
        let login = format!("http://{address}");
        let mut t =
            cible("m365", &login, &[("tenant_id", "contoso"), ("login_url", login.as_str())]);
        t.credential = Credential::UsernamePassword {
            username: "client-id".into(),
            password: "s3cret".into(),
        };
        t
    }

    #[tokio::test]
    async fn sonde_complete_avec_pagination_et_jeton_garde() {
        let (address, logins) = fake(OK).await;
        let collector = M365Collector::new();
        let t = target(&address);
        let s = collector.probe(&t).await.unwrap();
        assert_eq!(value(&s, "m365_services_degraded", &[]), 1.0);
        assert_eq!(value(&s, "m365_incidents_active", &[]), 1.0);
        assert_eq!(value(&s, "m365_advisories_active", &[]), 1.0, "second page followed");
        assert_eq!(value(&s, "m365_licenses_exhausted", &[]), 1.0);
        assert_eq!(value(&s, "m365_risky_users", &[]), 2.0);
        collector.probe(&t).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 1, "the token is kept until it expires");
        assert_eq!(collector.discover(&t).await.unwrap().as_deref(), Some("m365"));
    }

    #[tokio::test]
    async fn sans_licence_p2_repli_sur_les_connexions() {
        let (address, _) = fake(Behaviour { risky: 403, ..OK }).await;
        let s = M365Collector::new().probe(&target(&address)).await.unwrap();
        assert_eq!(value(&s, "m365_risky_readable", &[]), 0.0);
        assert_eq!(value(&s, "m365_signins_readable", &[]), 1.0);
        assert_eq!(value(&s, "m365_signin_failures_1h", &[]), 3.0);
        assert_eq!(value(&s, "m365_services_degraded", &[]), 1.0);
    }

    #[tokio::test]
    async fn risque_illisible_sans_echec_de_la_sonde() {
        let (address, _) = fake(Behaviour { risky: 403, signins: 403, ..OK }).await;
        let s = M365Collector::new().probe(&target(&address)).await.unwrap();
        assert_eq!(value(&s, "m365_risky_readable", &[]), 0.0);
        assert_eq!(value(&s, "m365_signins_readable", &[]), 0.0);
        assert_eq!(value(&s, "m365_services", &[]), 4.0);
    }

    #[tokio::test]
    async fn identifiants_refuses() {
        let (address, _) = fake(OK).await;
        let mut t = target(&address);
        t.credential =
            Credential::UsernamePassword { username: "client-id".into(), password: "wrong".into() };
        let error = M365Collector::new().probe(&t).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn droit_manquant_sur_la_lecture_principale() {
        let (address, _) = fake(Behaviour { health: 403, ..OK }).await;
        let error = M365Collector::new().probe(&target(&address)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(error.to_string().contains("ServiceHealth.Read.All"), "{error}");
    }

    #[tokio::test]
    async fn panne_de_microsoft() {
        let (address, _) = fake(Behaviour { health: 503, ..OK }).await;
        let error = M365Collector::new().probe(&target(&address)).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[tokio::test]
    async fn reglages_invalides() {
        let collector = M365Collector::new();
        let mut t = target("127.0.0.1:1");
        t.tags.remove("tenant_id");
        assert!(matches!(collector.probe(&t).await.unwrap_err(), ProbeError::Config(_)));
        let mut t = target("127.0.0.1:1");
        t.tags.insert("tenant_id".into(), "a/b".into());
        assert!(matches!(collector.probe(&t).await.unwrap_err(), ProbeError::Config(_)));
        let mut t = target("127.0.0.1:1");
        t.tags.insert("login_url".into(), "login.example".into());
        assert!(matches!(collector.probe(&t).await.unwrap_err(), ProbeError::Config(_)));
        let mut t = target("127.0.0.1:1");
        t.credential = Credential::ApiToken { token: "x".into() };
        assert!(matches!(collector.probe(&t).await.unwrap_err(), ProbeError::Config(_)));
    }
}
