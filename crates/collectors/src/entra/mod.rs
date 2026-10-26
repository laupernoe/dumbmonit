//! Microsoft Entra ID, par Microsoft Graph (`https://graph.microsoft.com/v1.0`).
//!
//! Même accès que Microsoft 365 (`crate::m365`) : une application Entra en
//! « client credentials », un jeton d'une heure gardé jusqu'à son expiration,
//! des lectures seules. Ce qui est lu :
//!
//! * les applications (`/applications`, `Application.Read.All`) : la date
//!   d'expiration de chaque secret et de chaque certificat. Un secret expiré
//!   casse en silence l'application qui s'en sert, c'est la panne que ce
//!   module existe pour prévenir ;
//! * l'organisation (`/organization`, `Organization.Read.All`) : la dernière
//!   synchronisation avec l'Active Directory local (Entra Connect) ;
//! * les administrateurs sans méthode MFA enregistrée
//!   (`/reports/authenticationMethods/userRegistrationDetails`,
//!   `Reports.Read.All`, licence Entra ID P1 ou P2) : lecture facultative, un
//!   refus donne `mfa_readable` = 0 sans faire échouer la sonde.
//!
//! Les applications et l'organisation sont des lectures principales : sans
//! elles, 401/403 est une erreur d'authentification.
//!
//! Un secret expiré reste souvent dans l'application pendant des années : les
//! séries `credential_expiry_seconds` ne gardent que les secrets non expirés et
//! ceux expirés depuis moins de 30 jours ; `credentials_expired` les compte
//! tous.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `tenant_id` | (obligatoire) | Identifiant du répertoire (locataire). |
//! | `login_url` | `https://login.microsoftonline.com` | Service de connexion (clouds souverains, tests). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use serde_json::Value;

use crate::cloud;
use crate::m365::{Fetched, connect, denied, fetch_all};
use crate::rest::{MAX_NAMED, TokenCache, flag, gauge, now_ms, seconds_until, text};

const PRODUCT: &str = "Microsoft Entra ID";
/// Préfixe des séries (le serveur y ajoute `dumbmonit_`).
const PREFIX: &str = "entra";
const MAX_PAGES: usize = 20;
/// Secrets décrits un par un sur la page.
const MAX_ITEMS: usize = 50;
/// Un secret expiré depuis plus longtemps n'a plus de série.
const RECENT_EXPIRY_SECS: f64 = 30.0 * 86_400.0;
const DAY_SECS: f64 = 86_400.0;

const APPLICATIONS_PATH: &str =
    "/v1.0/applications?$select=id,appId,displayName,passwordCredentials,keyCredentials&$top=999";
const ORGANIZATION_PATH: &str = "/v1.0/organization";
const ADMINS_PATH: &str = "/v1.0/reports/authenticationMethods/userRegistrationDetails?\
                           $filter=isAdmin%20eq%20true&$top=999";

#[derive(Default)]
pub struct EntraCollector {
    tokens: TokenCache,
}

impl EntraCollector {
    pub fn new() -> Self {
        Self::default()
    }

    async fn read(&self, target: &Target) -> Result<Reading, ProbeError> {
        let (client, key) = connect(&self.tokens, target, PRODUCT).await?;
        let result = collect(&client).await;
        if matches!(result, Err(ProbeError::Auth(_))) {
            // Jeton révoqué entre-temps : le prochain passage en redemandera un.
            self.tokens.forget(&key);
        }
        result
    }
}

/// Ce que Graph a répondu, avant d'en faire des séries.
pub struct Reading {
    pub applications: Vec<Value>,
    /// La première (et en pratique l'unique) organisation.
    pub organization: Value,
    pub admins: Option<Vec<Value>>,
}

async fn collect(client: &crate::rest::RestClient) -> Result<Reading, ProbeError> {
    let applications = match fetch_all(client, APPLICATIONS_PATH, MAX_PAGES).await? {
        Fetched::Values(values) => values,
        Fetched::Denied(code) => {
            return Err(denied(code, APPLICATIONS_PATH, "Application.Read.All"));
        }
    };
    let organization = match fetch_all(client, ORGANIZATION_PATH, 1).await? {
        Fetched::Values(values) => values.into_iter().next().unwrap_or(Value::Null),
        Fetched::Denied(code) => {
            return Err(denied(code, ORGANIZATION_PATH, "Organization.Read.All"));
        }
    };
    let admins = fetch_all(client, ADMINS_PATH, MAX_PAGES).await?.values();
    Ok(Reading { applications, organization, admins })
}

#[async_trait]
impl Collector for EntraCollector {
    fn kind(&self) -> &'static str {
        "entra"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let reading = self.read(target).await?;
        Ok(samples(&reading, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        self.read(target).await?;
        Ok(Some("entra".to_string()))
    }
}

/// Un secret ou un certificat d'application, prêt à publier.
struct Expiring<'a> {
    left: f64,
    app: &'a str,
    kind: &'static str,
    key: String,
}

/// Libellé stable (il sert d'étiquette `detail`) d'une échéance.
fn expiry_label(left: f64) -> &'static str {
    if left < 0.0 {
        "Expired"
    } else if left < 7.0 * DAY_SECS {
        "Expires within 7 days"
    } else if left < 30.0 * DAY_SECS {
        "Expires within 30 days"
    } else {
        "Expires within 90 days"
    }
}

pub fn samples(reading: &Reading, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(&format!("{PREFIX}_{name}"), value, ts_ms);
    let f = |name: &str, value: bool| flag(&format!("{PREFIX}_{name}"), value, ts_ms);
    let mut out = Vec::new();

    // Secrets et certificats des applications.
    let mut credentials: Vec<Expiring> = Vec::new();
    let (mut total, mut expired, mut expiring) = (0usize, 0usize, 0usize);
    for app in &reading.applications {
        let name = text(app, "displayName").unwrap_or("(unnamed)");
        for (field, kind) in [("passwordCredentials", "secret"), ("keyCredentials", "certificate")]
        {
            for credential in app.get(field).and_then(Value::as_array).into_iter().flatten() {
                let Some(left) =
                    text(credential, "endDateTime").and_then(|t| seconds_until(t, ts_ms))
                else {
                    continue;
                };
                total += 1;
                expired += usize::from(left < 0.0);
                expiring += usize::from((0.0..30.0 * DAY_SECS).contains(&left));
                if left < -RECENT_EXPIRY_SECS {
                    continue;
                }
                let id: String =
                    text(credential, "keyId").unwrap_or("unknown").chars().take(8).collect();
                let key = match text(credential, "displayName") {
                    Some(label) => format!("{label} ({id})"),
                    None => id,
                };
                credentials.push(Expiring { left, app: name, kind, key });
            }
        }
    }
    credentials.sort_by(|a, b| a.left.total_cmp(&b.left));
    for credential in credentials.iter().take(MAX_NAMED) {
        out.push(
            g("credential_expiry_seconds", credential.left)
                .with_label("app", credential.app)
                .with_label("kind", credential.kind)
                .with_label("key", credential.key.as_str()),
        );
    }
    for credential in credentials.iter().filter(|c| c.left < 90.0 * DAY_SECS).take(MAX_ITEMS) {
        let name = format!("{} ({})", credential.app, credential.key);
        let ok = credential.left >= 30.0 * DAY_SECS;
        out.push(cloud::item(
            PREFIX,
            "Credentials",
            &name,
            ok,
            expiry_label(credential.left),
            ts_ms,
        ));
    }
    out.push(g("applications", reading.applications.len() as f64));
    out.push(g("credentials", total as f64));
    out.push(g("credentials_expired", expired as f64));
    out.push(g("credentials_expiring_30d", expiring as f64));
    out.push(cloud::figure(PREFIX, "Credentials expiring", "", expiring as f64, ts_ms));
    out.push(cloud::figure(PREFIX, "Credentials expired", "", expired as f64, ts_ms));

    // Synchronisation avec l'Active Directory local.
    let sync = reading.organization.get("onPremisesSyncEnabled").and_then(Value::as_bool);
    out.push(f("ad_sync_enabled", sync == Some(true)));
    if sync == Some(true)
        && let Some(last) = text(&reading.organization, "onPremisesLastSyncDateTime")
        && let Some(left) = seconds_until(last, ts_ms)
    {
        let age = (-left).max(0.0);
        out.push(g("ad_sync_age_seconds", age));
        let late = age > 3.0 * 3600.0;
        out.push(cloud::item(
            PREFIX,
            "Directory",
            "On-premises sync",
            !late,
            if late { "Late" } else { "In sync" },
            ts_ms,
        ));
    }

    // Administrateurs sans MFA.
    out.push(f("mfa_readable", reading.admins.is_some()));
    if let Some(admins) = &reading.admins {
        let without: Vec<&str> = admins
            .iter()
            .filter(|a| a.get("isMfaRegistered").and_then(Value::as_bool) == Some(false))
            .filter_map(|a| text(a, "userPrincipalName"))
            .collect();
        out.push(g("admins", admins.len() as f64));
        out.push(g("admin_without_mfa", without.len() as f64));
        for admin in without.iter().take(MAX_NAMED) {
            out.push(f("admin_mfa_registered", false).with_label("admin", *admin));
            out.push(cloud::item(
                PREFIX,
                "Administrators",
                admin,
                false,
                "No MFA registered",
                ts_ms,
            ));
        }
        out.push(cloud::figure(PREFIX, "Admins without MFA", "", without.len() as f64, ts_ms));
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
    use dumbmonit_proto::Credential as Secret;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après les exemples de la documentation Microsoft
    // Graph v1.0 (application, passwordCredential, keyCredential, organization,
    // userRegistrationDetails).
    const APPS_1: &str = include_str!("testdata/applications_page1.json");
    const APPS_2: &str = include_str!("testdata/applications_page2.json");
    const ORG: &str = include_str!("testdata/organization.json");
    const ADMINS: &str = include_str!("testdata/user_registration_details.json");

    const NOW: &str = "2026-10-26T09:00:00Z";

    fn now() -> i64 {
        chrono::DateTime::parse_from_rfc3339(NOW).unwrap().timestamp_millis()
    }

    fn list(raw: &str) -> Vec<Value> {
        let v: Value = serde_json::from_str(raw).unwrap();
        v["value"].as_array().unwrap().clone()
    }

    fn reading() -> Reading {
        let mut applications = list(&APPS_1.replace("{{base}}", "http://x"));
        applications.extend(list(APPS_2));
        Reading { applications, organization: list(ORG).remove(0), admins: Some(list(ADMINS)) }
    }

    #[test]
    fn echeances_des_secrets() {
        let s = samples(&reading(), now());
        assert_eq!(value(&s, "entra_applications", &[]), 4.0);
        assert_eq!(value(&s, "entra_credentials", &[]), 5.0);
        assert_eq!(value(&s, "entra_credentials_expired", &[]), 2.0);
        assert_eq!(value(&s, "entra_credentials_expiring_30d", &[]), 1.0);
        assert_eq!(
            value(
                &s,
                "entra_credential_expiry_seconds",
                &[("app", "Backup Agent"), ("kind", "secret"), ("key", "prod (aaaa1111)")]
            ),
            10.0 * 86_400.0
        );
        let expired = value(
            &s,
            "entra_credential_expiry_seconds",
            &[("app", "Intranet"), ("kind", "secret"), ("key", "bbbb2222")],
        );
        assert!((expired + 6.0 * 86_400.0).abs() < 1.0, "{expired}");
        assert!(
            value(
                &s,
                "entra_credential_expiry_seconds",
                &[("app", "Intranet"), ("kind", "certificate"), ("key", "CN=intranet (cccc3333)")]
            ) > 90.0 * 86_400.0
        );
        assert!(
            find(&s, "entra_credential_expiry_seconds", &[("app", "Old App")]).is_none(),
            "expired for more than 30 days: counted, not published"
        );
        let item =
            find(&s, "entra_item_ok", &[("item", "Backup Agent (prod (aaaa1111))")]).unwrap();
        assert_eq!(item.labels["group"], "Credentials");
        assert_eq!(item.labels["detail"], "Expires within 30 days");
        assert_eq!(item.value, 0.0);
        assert!(find(&s, "entra_item_ok", &[("item", "Healthy (ci (eeee5555))")]).is_none());
        assert_eq!(value(&s, "entra_figure", &[("name", "Credentials expiring")]), 1.0);
    }

    #[test]
    fn synchronisation_et_administrateurs() {
        let s = samples(&reading(), now());
        assert_eq!(value(&s, "entra_ad_sync_enabled", &[]), 1.0);
        assert_eq!(value(&s, "entra_ad_sync_age_seconds", &[]), 5_400.0);
        assert_eq!(value(&s, "entra_item_ok", &[("item", "On-premises sync")]), 1.0);
        assert_eq!(value(&s, "entra_mfa_readable", &[]), 1.0);
        assert_eq!(value(&s, "entra_admins", &[]), 3.0);
        assert_eq!(value(&s, "entra_admin_without_mfa", &[]), 2.0);
        assert_eq!(value(&s, "entra_admin_mfa_registered", &[("admin", "bob@contoso.com")]), 0.0);
        assert!(
            find(&s, "entra_admin_mfa_registered", &[("admin", "alice@contoso.com")]).is_none()
        );
    }

    #[test]
    fn sans_synchronisation_ni_droit_mfa() {
        let mut r = reading();
        r.organization =
            serde_json::json!({"displayName": "Contoso", "onPremisesSyncEnabled": null});
        r.admins = None;
        let s = samples(&r, now());
        assert_eq!(value(&s, "entra_ad_sync_enabled", &[]), 0.0);
        assert!(find(&s, "entra_ad_sync_age_seconds", &[]).is_none());
        assert_eq!(value(&s, "entra_mfa_readable", &[]), 0.0);
        assert!(find(&s, "entra_admin_without_mfa", &[]).is_none());
        assert_eq!(value(&s, "entra_credentials", &[]), 5.0, "applications still read");
    }

    #[test]
    fn synchronisation_en_retard() {
        let mut r = reading();
        r.organization = serde_json::json!({
            "onPremisesSyncEnabled": true,
            "onPremisesLastSyncDateTime": "2026-10-26T04:00:00Z"
        });
        let s = samples(&r, now());
        assert_eq!(value(&s, "entra_ad_sync_age_seconds", &[]), 5.0 * 3600.0);
        let item = find(&s, "entra_item_ok", &[("item", "On-premises sync")]).unwrap();
        assert_eq!(item.value, 0.0);
        assert_eq!(item.labels["detail"], "Late");
    }

    #[derive(Clone, Copy)]
    struct Behaviour {
        apps: u16,
        org: u16,
        admins: u16,
    }

    const OK: Behaviour = Behaviour { apps: 200, org: 200, admins: 200 };

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
                        if form.get("client_secret").map(String::as_str) != Some("s3cret") {
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
                "/v1.0/applications",
                get(move |headers: HeaderMap, Query(query): Query<HashMap<String, String>>| async move {
                    if headers.get("authorization").and_then(|v| v.to_str().ok())
                        != Some("Bearer graph-token")
                    {
                        return (StatusCode::UNAUTHORIZED, "{}".to_string());
                    }
                    if query.get("$top").map(String::as_str) != Some("999") {
                        return (StatusCode::BAD_REQUEST, "{}".to_string());
                    }
                    if b.apps != 200 {
                        return reply(b.apps, "");
                    }
                    if query.contains_key("$skiptoken") {
                        return (StatusCode::OK, APPS_2.to_string());
                    }
                    let host = headers.get("host").and_then(|h| h.to_str().ok()).unwrap_or_default();
                    (StatusCode::OK, APPS_1.replace("{{base}}", &format!("http://{host}")))
                }),
            )
            .route(
                "/v1.0/organization",
                get(move || async move { reply(b.org, ORG) }),
            )
            .route(
                "/v1.0/reports/authenticationMethods/userRegistrationDetails",
                get(move |Query(query): Query<HashMap<String, String>>| async move {
                    if query.get("$filter").map(String::as_str) != Some("isAdmin eq true") {
                        return (StatusCode::BAD_REQUEST, "{}".to_string());
                    }
                    reply(b.admins, ADMINS)
                }),
            );
        (serve(app).await, logins)
    }

    fn target(address: &str) -> Target {
        let login = format!("http://{address}");
        let mut t =
            cible("entra", &login, &[("tenant_id", "contoso"), ("login_url", login.as_str())]);
        t.credential =
            Secret::UsernamePassword { username: "client-id".into(), password: "s3cret".into() };
        t
    }

    #[tokio::test]
    async fn sonde_complete_avec_pagination_et_jeton_garde() {
        let (address, logins) = fake(OK).await;
        let collector = EntraCollector::new();
        let t = target(&address);
        let s = collector.probe(&t).await.unwrap();
        assert_eq!(value(&s, "entra_applications", &[]), 4.0, "second page followed");
        assert!(find(&s, "entra_credential_expiry_seconds", &[("app", "Healthy")]).is_some());
        assert!(find(&s, "entra_ad_sync_age_seconds", &[]).is_some());
        assert_eq!(value(&s, "entra_admin_without_mfa", &[]), 2.0);
        collector.probe(&t).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 1, "the token is kept until it expires");
        assert_eq!(collector.discover(&t).await.unwrap().as_deref(), Some("entra"));
    }

    #[tokio::test]
    async fn sans_licence_pas_de_mfa_mais_la_sonde_reussit() {
        let (address, _) = fake(Behaviour { admins: 403, ..OK }).await;
        let s = EntraCollector::new().probe(&target(&address)).await.unwrap();
        assert_eq!(value(&s, "entra_mfa_readable", &[]), 0.0);
        assert!(find(&s, "entra_admin_without_mfa", &[]).is_none());
        assert_eq!(value(&s, "entra_credentials", &[]), 5.0);
    }

    #[tokio::test]
    async fn identifiants_refuses() {
        let (address, _) = fake(OK).await;
        let mut t = target(&address);
        t.credential =
            Secret::UsernamePassword { username: "client-id".into(), password: "wrong".into() };
        let error = EntraCollector::new().probe(&t).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn droit_manquant_sur_une_lecture_principale() {
        let (address, _) = fake(Behaviour { apps: 403, ..OK }).await;
        let error = EntraCollector::new().probe(&target(&address)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(error.to_string().contains("Application.Read.All"), "{error}");
        let (address, _) = fake(Behaviour { org: 403, ..OK }).await;
        let error = EntraCollector::new().probe(&target(&address)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(error.to_string().contains("Organization.Read.All"), "{error}");
    }

    #[tokio::test]
    async fn panne_de_microsoft() {
        let (address, _) = fake(Behaviour { apps: 503, ..OK }).await;
        let error = EntraCollector::new().probe(&target(&address)).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[tokio::test]
    async fn reglages_invalides() {
        let collector = EntraCollector::new();
        let mut t = target("127.0.0.1:1");
        t.tags.remove("tenant_id");
        assert!(matches!(collector.probe(&t).await.unwrap_err(), ProbeError::Config(_)));
        let mut t = target("127.0.0.1:1");
        t.credential = Secret::ApiToken { token: "x".into() };
        assert!(matches!(collector.probe(&t).await.unwrap_err(), ProbeError::Config(_)));
    }
}
