//! Collectes complètes contre de faux contrôleurs UniFi.
//!
//! Les réponses d'erreur et de contrôleur vide sont réelles (UniFi Network
//! 10.6.106 auto-hébergé, compte View Only) ; les listes d'équipements et la
//! santé d'un réseau équipé suivent la documentation, faute d'équipement à
//! adopter dans l'environnement de test.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use serde_json::Value;

use super::*;
use crate::api_options::test_support::target;

const COOKIE_VALUE: &str = "unifises=f2fzIKrVORhNpD0IHRInOuA75ZUEoRxc";
const KEY: &str = "key-for-tests";

fn real(name: &str) -> &'static str {
    match name {
        "sysinfo" => include_str!("testdata/network_10.6.106/stat_sysinfo.json"),
        "login_refused" => include_str!("testdata/network_10.6.106/login_refused.json"),
        "login_required" => include_str!("testdata/network_10.6.106/login_required.json"),
        "no_site" => include_str!("testdata/network_10.6.106/no_site_context.json"),
        "critical" => include_str!("testdata/network_10.6.106/system_log_critical_empty.json"),
        "forbidden" => include_str!("testdata/network_10.6.106/integration_forbidden.json"),
        other => panic!("pas de fixture {other}"),
    }
}

/// Quatre clients connus d'un site : un poste nommé, en IP fixe et annoté, un
/// téléphone seulement annoncé par DHCP, une imprimante nommée mais sans IP
/// fixe ni note, et un téléphone invité — voir le commentaire de module sur
/// ceux qui comptent par défaut.
const REST_USER: &str = include_str!("testdata/documented/rest_user.json");

fn logged_in(headers: &HeaderMap) -> bool {
    headers.get("cookie").and_then(|v| v.to_str().ok()).is_some_and(|c| c.contains(COOKIE_VALUE))
}

#[derive(Default)]
struct Counters {
    logins: AtomicUsize,
    /// Fait expirer la session une fois.
    expire: AtomicUsize,
}

async fn serve(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{address}")
}

/// Un serveur auto-hébergé : la racine renvoie vers `/manage`, la session
/// s'ouvre sur `/api/login`.
async fn self_hosted(counters: Arc<Counters>) -> String {
    let stat =
        {
            let counters = counters.clone();
            move |Path((site, what)): Path<(String, String)>, headers: HeaderMap| {
                let counters = counters.clone();
                async move {
                    if !logged_in(&headers) || counters.expire.swap(0, Ordering::SeqCst) > 0 {
                        return (StatusCode::UNAUTHORIZED, real("login_required")).into_response();
                    }
                    if site != "default" {
                        return (StatusCode::UNAUTHORIZED, real("no_site")).into_response();
                    }
                    match what.as_str() {
                        "sysinfo" => real("sysinfo").into_response(),
                        "health" => include_str!("testdata/documented/classic_stat_health.json")
                            .into_response(),
                        "device" => include_str!("testdata/documented/classic_stat_device.json")
                            .into_response(),
                        _ => StatusCode::NOT_FOUND.into_response(),
                    }
                }
            }
        };
    let login = {
        let counters = counters.clone();
        move |body: axum::Json<Value>| {
            let counters = counters.clone();
            async move {
                if body["username"] != "dumbmonit" || body["password"] != "ViewOnly-Pass-42" {
                    return (StatusCode::BAD_REQUEST, real("login_refused")).into_response();
                }
                counters.logins.fetch_add(1, Ordering::SeqCst);
                (
                    StatusCode::OK,
                    [("set-cookie", format!("{COOKIE_VALUE}; Path=/; Secure; HttpOnly"))],
                    r#"{"meta":{"rc":"ok"},"data":[]}"#,
                )
                    .into_response()
            }
        }
    };
    let rest_user = {
        let counters = counters.clone();
        move |Path(site): Path<String>, headers: HeaderMap| {
            let counters = counters.clone();
            async move {
                if !logged_in(&headers) || counters.expire.swap(0, Ordering::SeqCst) > 0 {
                    return (StatusCode::UNAUTHORIZED, real("login_required")).into_response();
                }
                if site != "default" {
                    return (StatusCode::UNAUTHORIZED, real("no_site")).into_response();
                }
                REST_USER.into_response()
            }
        }
    };
    let app = Router::new()
        .route("/", get(|| async { Redirect::to("/manage") }))
        .route("/manage", get(|| async { "<html>UniFi Network</html>" }))
        .route("/api/login", post(login))
        .route("/api/s/{site}/stat/{what}", get(stat))
        .route("/api/s/{site}/rest/user", get(rest_user))
        .route(
            "/v2/api/site/{site}/system-log/critical",
            post(|headers: HeaderMap| async move {
                if !logged_in(&headers) {
                    return (StatusCode::UNAUTHORIZED, real("login_required")).into_response();
                }
                real("critical").into_response()
            }),
        );
    serve(app).await
}

fn login_credential(password: &str) -> Credential {
    Credential::UsernamePassword { username: "dumbmonit".into(), password: password.into() }
}

fn value(samples: &[Sample], metric: &str) -> Option<f64> {
    samples.iter().find(|s| s.metric == metric).map(|s| s.value)
}

fn labelled(samples: &[Sample], metric: &str, key: &str, label: &str) -> Option<f64> {
    samples
        .iter()
        .find(|s| s.metric == metric && s.labels.get(key).map(String::as_str) == Some(label))
        .map(|s| s.value)
}

#[tokio::test]
async fn un_compte_view_only_lit_tout_et_garde_sa_session() {
    let counters = Arc::new(Counters::default());
    let base = self_hosted(counters.clone()).await;
    let collector = UnifiCollector::new();
    let target = target(
        "unifi",
        &format!("{base}/manage/default/dashboard"),
        &[],
        login_credential("ViewOnly-Pass-42"),
    );

    let samples = collector.probe(&target).await.unwrap();
    assert_eq!(labelled(&samples, "unifi_info", "version", "10.6.106"), Some(1.0));
    assert_eq!(labelled(&samples, "unifi_devices", "state", "offline"), Some(1.0));
    assert_eq!(value(&samples, "unifi_internet_up"), Some(1.0));
    assert_eq!(value(&samples, "unifi_clients"), Some(41.0));
    assert_eq!(value(&samples, "unifi_alarms"), Some(0.0));
    assert_eq!(value(&samples, "unifi_scrape_errors"), Some(0.0));
    // Nommé, en IP fixe et annoté : suivi par défaut.
    assert_eq!(
        labelled(&samples, "client_device_last_seen_timestamp_seconds", "device", "Noe's desktop"),
        Some(1_790_798_700.0)
    );
    let desktop = samples
        .iter()
        .find(|s| {
            s.metric == "client_device_last_seen_timestamp_seconds"
                && s.labels.get("device").map(String::as_str) == Some("Noe's desktop")
        })
        .unwrap();
    assert_eq!(desktop.labels["kind"], "unifi");
    assert_eq!(desktop.labels["type"], "Wired");
    // Nommé, sans IP fixe ni note : le nom suffit.
    assert!(
        labelled(&samples, "client_device_last_seen_timestamp_seconds", "device", "Office printer")
            .is_some()
    );
    // Ni nommé, ni en IP fixe, ni annoté : pas suivi par défaut, pour ne pas
    // noyer l'alerte sous les clients transitoires.
    assert!(
        labelled(&samples, "client_device_last_seen_timestamp_seconds", "device", "iPhone-de-Bob")
            .is_none()
    );
    assert!(
        labelled(&samples, "client_device_last_seen_timestamp_seconds", "device", "android-xyz123")
            .is_none()
    );

    collector.probe(&target).await.unwrap();
    assert_eq!(counters.logins.load(Ordering::SeqCst), 1, "la session est réutilisée");

    counters.expire.store(1, Ordering::SeqCst);
    collector.probe(&target).await.unwrap();
    assert_eq!(counters.logins.load(Ordering::SeqCst), 2, "une session expirée est rouverte");
}

#[tokio::test]
async fn un_mot_de_passe_refuse_ou_un_site_inconnu_ne_sont_pas_des_pannes() {
    let base = self_hosted(Arc::new(Counters::default())).await;
    let error = UnifiCollector::new()
        .probe(&target("unifi", &base, &[], login_credential("faux")))
        .await
        .unwrap_err();
    assert!(matches!(error, ProbeError::Auth(_)), "{error:?}");

    let error = UnifiCollector::new()
        .probe(&target("unifi", &base, &[("site", "cabin")], login_credential("ViewOnly-Pass-42")))
        .await
        .unwrap_err();
    assert!(matches!(error, ProbeError::Config(ref m) if m.contains("cabin")), "{error:?}");
    assert!(!error.means_down());
}

/// Une console UniFi OS avec une clé d'API : l'API d'intégration sous
/// `/proxy/network`, et l'API classique qui accepte la même clé.
async fn console_with_key(classic_accepts_key: bool) -> String {
    let authorized =
        |headers: &HeaderMap| headers.get("x-api-key").and_then(|v| v.to_str().ok()) == Some(KEY);
    let guard = move |headers: HeaderMap, body: &'static str| -> Response {
        if !authorized(&headers) {
            return (StatusCode::UNAUTHORIZED, real("forbidden")).into_response();
        }
        body.into_response()
    };
    let app = Router::new()
        .route(
            "/proxy/network/integration/v1/info",
            get(move |h: HeaderMap| async move { guard(h, r#"{"applicationVersion":"10.0.160"}"#) }),
        )
        .route(
            "/proxy/network/integration/v1/sites",
            get(move |h: HeaderMap| async move {
                guard(h, include_str!("testdata/documented/integration_sites.json"))
            }),
        )
        .route(
            "/proxy/network/integration/v1/sites/{site}/devices",
            get(move |h: HeaderMap, Query(q): Query<std::collections::HashMap<String, String>>| async move {
                assert_eq!(q.get("offset").map(String::as_str), Some("0"));
                guard(h, include_str!("testdata/documented/integration_devices.json"))
            }),
        )
        .route(
            "/proxy/network/integration/v1/sites/{site}/devices/{id}/statistics/latest",
            get(move |h: HeaderMap| async move {
                guard(h, include_str!("testdata/documented/integration_statistics.json"))
            }),
        )
        .route(
            "/proxy/network/integration/v1/sites/{site}/clients",
            get(move |h: HeaderMap| async move {
                guard(h, r#"{"offset":0,"limit":1,"count":1,"totalCount":38,"data":[{"type":"WIRELESS"}]}"#)
            }),
        )
        .route(
            "/proxy/network/api/s/{site}/stat/health",
            get(move |h: HeaderMap| async move {
                if !classic_accepts_key {
                    return (StatusCode::UNAUTHORIZED, real("login_required")).into_response();
                }
                guard(h, include_str!("testdata/documented/classic_stat_health.json"))
            }),
        )
        .route(
            "/proxy/network/api/s/{site}/rest/user",
            get(move |h: HeaderMap| async move {
                if !classic_accepts_key {
                    return (StatusCode::UNAUTHORIZED, real("login_required")).into_response();
                }
                guard(h, REST_USER)
            }),
        );
    serve(app).await
}

#[tokio::test]
async fn une_cle_d_api_passe_par_l_api_d_integration() {
    let base = console_with_key(true).await;
    let samples = UnifiCollector::new()
        .probe(&target("unifi", &base, &[], Credential::ApiToken { token: KEY.into() }))
        .await
        .unwrap();
    assert_eq!(labelled(&samples, "unifi_info", "api", "integration"), Some(1.0));
    assert_eq!(labelled(&samples, "unifi_devices", "state", "online"), Some(2.0));
    assert_eq!(labelled(&samples, "unifi_device_cpu_percent", "device", "Gateway"), Some(12.4));
    assert_eq!(labelled(&samples, "unifi_device_up", "device", "Living room AP"), Some(0.0));
    assert_eq!(value(&samples, "unifi_clients"), Some(38.0));
    assert_eq!(value(&samples, "unifi_wan_up"), Some(1.0));
    assert_eq!(value(&samples, "unifi_scrape_errors"), Some(0.0));
    // La clé d'API lit aussi /rest/user, comme la santé classique plus haut.
    assert!(
        labelled(&samples, "client_device_last_seen_timestamp_seconds", "device", "Noe's desktop")
            .is_some()
    );
}

#[tokio::test]
async fn watched_clients_ajoute_des_clients_precis_ou_tous() {
    let counters = Arc::new(Counters::default());
    let base = self_hosted(counters.clone()).await;
    let collector = UnifiCollector::new();

    let named_only = target("unifi", &base, &[], login_credential("ViewOnly-Pass-42"));
    let samples = collector.probe(&named_only).await.unwrap();
    assert!(
        labelled(&samples, "client_device_last_seen_timestamp_seconds", "device", "iPhone-de-Bob")
            .is_none()
    );

    let listed = target(
        "unifi",
        &base,
        &[("watched_clients", "iPhone-de-Bob")],
        login_credential("ViewOnly-Pass-42"),
    );
    let samples = collector.probe(&listed).await.unwrap();
    assert!(
        labelled(&samples, "client_device_last_seen_timestamp_seconds", "device", "iPhone-de-Bob")
            .is_some(),
        "nommé dans l'option, il est suivi même sans marque dans UniFi"
    );
    assert!(
        labelled(&samples, "client_device_last_seen_timestamp_seconds", "device", "android-xyz123")
            .is_none(),
        "le téléphone invité n'est pas dans la liste"
    );

    let all =
        target("unifi", &base, &[("watched_clients", "all")], login_credential("ViewOnly-Pass-42"));
    let samples = collector.probe(&all).await.unwrap();
    assert!(
        labelled(&samples, "client_device_last_seen_timestamp_seconds", "device", "android-xyz123")
            .is_some(),
        "all suit tout le monde"
    );
}

#[test]
fn la_liste_de_clients_a_surveiller() {
    assert_eq!(ClientWatch::parse(None), ClientWatch::Default);
    assert_eq!(ClientWatch::parse(Some(" ALL ")), ClientWatch::All);
    let record = |mac: &str, name: Option<&str>, noted: bool, fixed: bool| ClientRecord {
        mac: mac.to_string(),
        name: name.map(str::to_string),
        hostname: None,
        noted,
        use_fixedip: fixed,
        is_wired: None,
        last_seen: None,
    };
    let noted = record("aa:bb:cc:00:00:09", None, true, false);
    assert!(ClientWatch::Default.covers(&noted), "un client marqué est suivi même par défaut");
    let anonymous = record("aa:bb:cc:00:00:10", None, false, false);
    assert!(!ClientWatch::Default.covers(&anonymous));
    assert!(ClientWatch::All.covers(&anonymous));
    let listed = ClientWatch::parse(Some("AA:BB:CC:00:00:10, Office printer"));
    assert!(listed.covers(&anonymous), "adresse MAC listée");
    assert!(!listed.covers(&record("aa:bb:cc:00:00:11", None, false, false)));
}

#[tokio::test]
async fn sans_l_api_classique_le_wan_est_saute_sans_erreur() {
    let base = console_with_key(false).await;
    let samples = UnifiCollector::new()
        .probe(&target("unifi", &base, &[], Credential::ApiToken { token: KEY.into() }))
        .await
        .unwrap();
    assert!(value(&samples, "unifi_wan_up").is_none());
    assert_eq!(value(&samples, "unifi_scrape_errors"), Some(0.0));
}

#[tokio::test]
async fn une_cle_refusee_n_est_pas_une_panne() {
    let base = console_with_key(true).await;
    let error = UnifiCollector::new()
        .probe(&target("unifi", &base, &[], Credential::ApiToken { token: "faux".into() }))
        .await
        .unwrap_err();
    assert!(matches!(error, ProbeError::Auth(_)), "{error:?}");
}

#[tokio::test]
async fn un_site_inconnu_de_la_cle_est_nomme() {
    let base = console_with_key(true).await;
    let error = UnifiCollector::new()
        .probe(&target(
            "unifi",
            &base,
            &[("site", "office")],
            Credential::ApiToken { token: KEY.into() },
        ))
        .await
        .unwrap_err();
    assert!(
        matches!(error, ProbeError::Config(ref m) if m.contains("default") && m.contains("k7h2x9qe"))
    );
    // Le nom affiché fait aussi l'affaire.
    let samples = UnifiCollector::new()
        .probe(&target(
            "unifi",
            &base,
            &[("site", "Cabin")],
            Credential::ApiToken { token: KEY.into() },
        ))
        .await
        .unwrap();
    assert!(value(&samples, "unifi_info").is_some());
}

#[test]
fn l_identifiant_et_le_site_sont_verifies() {
    assert!(matches!(
        Settings::from_target(&target("unifi", "unifi.lan", &[], Credential::None)),
        Err(ProbeError::Config(_))
    ));
    assert!(matches!(
        Settings::from_target(&target(
            "unifi",
            "unifi.lan",
            &[("site", "a/b")],
            login_credential("x")
        )),
        Err(ProbeError::Config(_))
    ));
    let settings = Settings::from_target(&target(
        "unifi",
        "https://unifi.lan:8443/manage/default/devices",
        &[],
        login_credential("x"),
    ))
    .unwrap();
    assert_eq!(settings.connection.base_url, "https://unifi.lan:8443");
    assert_eq!(settings.site, "default");
}
