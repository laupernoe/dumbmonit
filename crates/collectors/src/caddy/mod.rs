//! Caddy : son API d'administration (`:2019`) et ses métriques.
//!
//! Trois lectures, toutes en `GET` : `/config/apps/http/servers` (la
//! configuration HTTP chargée, jamais celle de `tls`, où dorment les jetons des
//! fournisseurs DNS), `/reverse_proxy/upstreams` (les serveurs d'arrière-plan et
//! leurs échecs récents) et `/metrics` (santé des serveurs, rechargements,
//! erreurs et réponses 5xx).
//!
//! L'API d'administration peut aussi recharger et arrêter Caddy : elle ne doit
//! être joignable que par DumbMonit. La notice propose de la garder sur
//! `localhost` et d'en publier une vue en lecture seule, protégée par mot de
//! passe, que Caddy sert lui-même.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | Protocole de l'API d'administration. |
//! | `port` | `2019` | Port de l'API d'administration (ou de sa vue en lecture seule). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |

use std::collections::BTreeMap;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target};
use serde::Deserialize;

use crate::observability::client::{Auth, HttpClient};
use crate::observability::options::Options;
use crate::observability::prom::{Exposition, Line};

pub const DEFAULT_PORT: u16 = 2019;
/// Serveurs d'arrière-plan décrits un par un, au plus.
const MAX_UPSTREAMS: usize = 500;
const SERVERS_PATH: &str = "/config/apps/http/servers";
const UPSTREAMS_PATH: &str = "/reverse_proxy/upstreams";

#[derive(Default)]
pub struct CaddyCollector;

impl CaddyCollector {
    pub fn new() -> Self {
        Self
    }
}

fn client(target: &Target) -> Result<HttpClient, ProbeError> {
    let options = Options::from_target(target, DEFAULT_PORT)?;
    let auth = Auth::from_credential(&target.credential)?;
    let http = crate::http::client(options.insecure_tls)?;
    Ok(HttpClient::new(http, options.base_url, auth, options.request_timeout, "Caddy"))
}

#[async_trait]
impl Collector for CaddyCollector {
    fn kind(&self) -> &'static str {
        "caddy"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let client = client(target)?;
        let servers = read_servers(&client).await?;
        let upstreams: Vec<Upstream> = get(&client, UPSTREAMS_PATH).await?;
        let page = read_metrics(&client).await?;
        let ts_ms = chrono::Utc::now().timestamp_millis();
        Ok(samples(servers.as_ref(), &upstreams, page.as_ref(), ts_ms))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        read_servers(&client(target)?).await?;
        Ok(Some("caddy".to_string()))
    }
}

/// Un serveur HTTP de la configuration : ses adresses d'écoute et ses routes.
#[derive(Debug, Default, Deserialize)]
pub struct Server {
    #[serde(default)]
    pub listen: Vec<String>,
    #[serde(default)]
    pub routes: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Upstream {
    pub address: String,
    #[serde(default)]
    pub num_requests: f64,
    #[serde(default)]
    pub fails: f64,
}

/// Un `GET` de l'API d'administration, avec l'explication d'un refus propre à
/// Caddy : l'en-tête `Host` ou `Origin` n'est pas dans `origins`.
async fn get<T: serde::de::DeserializeOwned>(
    client: &HttpClient,
    path: &str,
) -> Result<T, ProbeError> {
    let reply = client.get_json_raw(path).await?;
    if reply.status == reqwest::StatusCode::FORBIDDEN && reply.body.contains("not allowed") {
        return Err(ProbeError::Auth(format!(
            "Caddy refused {path}: {}. Add the address DumbMonit uses to origins in the admin \
             options, or use the read-only view described in the setup guide.",
            reply.body.trim()
        )));
    }
    if !reply.status.is_success() {
        return Err(client.status_error(reply.status, &reply.body, path));
    }
    serde_json::from_str(&reply.body).map_err(|_| {
        ProbeError::Protocol(format!(
            "{} does not look like Caddy's admin API. Check the device type and the port \
             (2019 by default).",
            client.url(path)
        ))
    })
}

/// La configuration HTTP chargée ; `None` quand Caddy tourne sans (`null`).
async fn read_servers(client: &HttpClient) -> Result<Option<BTreeMap<String, Server>>, ProbeError> {
    get(client, SERVERS_PATH).await
}

/// `/metrics` de l'API d'administration. Toujours servi par Caddy ; une vue en
/// lecture seule qui ne le laisse pas passer n'est pas une erreur.
async fn read_metrics(client: &HttpClient) -> Result<Option<Exposition>, ProbeError> {
    let reply = client.get_raw("/metrics").await?;
    if matches!(reply.status.as_u16(), 403..=405) {
        return Ok(None);
    }
    if !reply.status.is_success() {
        return Err(client.status_error(reply.status, &reply.body, "/metrics"));
    }
    let page = Exposition::parse(&reply.body);
    Ok((page.has("caddy_config_last_reload_successful")
        || page.has("caddy_reverse_proxy_upstreams_healthy"))
    .then_some(page))
}

fn is_5xx(line: &Line) -> bool {
    line.label("code").is_some_and(|code| code.starts_with('5'))
}

pub fn samples(
    servers: Option<&BTreeMap<String, Server>>,
    upstreams: &[Upstream],
    page: Option<&Exposition>,
    ts_ms: i64,
) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let counter = |name: &str, value: f64| Sample::new(name, value, MetricKind::Counter, ts_ms);

    let servers_count = servers.map_or(0, BTreeMap::len);
    out.push(gauge("caddy_config_loaded", if servers_count > 0 { 1.0 } else { 0.0 }));
    out.push(gauge("caddy_servers", servers_count as f64));
    let routes: usize = servers.map_or(0, |s| s.values().map(|server| server.routes.len()).sum());
    out.push(gauge("caddy_routes", routes as f64));

    // Les serveurs d'arrière-plan : la liste de l'API, la santé des métriques.
    let healthy: BTreeMap<&str, f64> = page
        .map(|p| {
            p.family("caddy_reverse_proxy_upstreams_healthy")
                .filter_map(|line| Some((line.label("upstream")?, line.value)))
                .collect()
        })
        .unwrap_or_default();
    let mut addresses: Vec<&str> = upstreams.iter().map(|u| u.address.as_str()).collect();
    addresses.extend(healthy.keys().copied());
    addresses.sort_unstable();
    addresses.dedup();
    out.push(gauge("caddy_upstreams", addresses.len() as f64));
    // Sain : ni le contrôle actif (la métrique) ni le contrôle passif (les
    // échecs retenus, qui ne s'accumulent que si `fail_duration` est posé) ne
    // l'écartent. Sans métriques, rien ne dit si un contrôle actif existe : la
    // santé n'est alors pas publiée, seuls les échecs retenus le sont.
    let fails = |address: &str| upstreams.iter().find(|u| u.address == address).map(|u| u.fails);
    let is_healthy = |address: &str| {
        healthy.get(address).is_none_or(|v| *v >= 1.0) && fails(address).is_none_or(|f| f < 1.0)
    };
    if page.is_some() {
        let unhealthy = addresses.iter().filter(|a| !is_healthy(a)).count();
        out.push(gauge("caddy_upstreams_unhealthy", unhealthy as f64));
    }
    for address in addresses.into_iter().take(MAX_UPSTREAMS) {
        if page.is_some() {
            out.push(
                gauge("caddy_upstream_healthy", if is_healthy(address) { 1.0 } else { 0.0 })
                    .with_label("upstream", address),
            );
        }
        if let Some(upstream) = upstreams.iter().find(|u| u.address == address) {
            out.push(gauge("caddy_upstream_fails", upstream.fails).with_label("upstream", address));
            out.push(
                gauge("caddy_upstream_requests", upstream.num_requests)
                    .with_label("upstream", address),
            );
        }
    }

    let Some(page) = page else { return out };
    if let Some(ok) = page.first("caddy_config_last_reload_successful") {
        out.push(gauge("caddy_config_last_reload_ok", if ok.value >= 1.0 { 1.0 } else { 0.0 }));
    }
    if let Some(at) = page.first("caddy_config_last_reload_success_timestamp_seconds") {
        out.push(gauge("caddy_config_last_reload_timestamp_seconds", at.value));
    }
    // Les requêtes ne sont comptées que si l'option globale `metrics` est
    // posée ; sans elle ces familles n'existent pas, et rien n'est inventé.
    if let Some(total) = page.sum("caddy_http_requests_total") {
        out.push(counter("caddy_requests_total", total));
        let durations = "caddy_http_request_duration_seconds_count";
        out.push(counter(
            "caddy_requests_5xx_total",
            page.sum_where(durations, is_5xx).unwrap_or(0.0),
        ));
        out.push(counter(
            "caddy_request_errors_total",
            page.sum("caddy_http_request_errors_total").unwrap_or(0.0),
        ));
        let totals = page.sum_by("caddy_http_requests_total", "server");
        let errors = page.sum_by("caddy_http_request_errors_total", "server");
        let mut server_5xx: BTreeMap<String, f64> = BTreeMap::new();
        for line in page.family(durations).filter(|l| is_5xx(l)) {
            *server_5xx.entry(line.label("server").unwrap_or_default().to_string()).or_default() +=
                line.value;
        }
        for (server, total) in totals.into_iter().filter(|(s, _)| !s.is_empty()) {
            let fivexx = server_5xx.get(&server).copied().unwrap_or(0.0);
            let failed = errors.get(&server).copied().unwrap_or(0.0);
            out.push(
                counter("caddy_server_requests_total", total).with_label("server", server.clone()),
            );
            out.push(
                counter("caddy_server_requests_5xx_total", fivexx)
                    .with_label("server", server.clone()),
            );
            out.push(
                counter("caddy_server_request_errors_total", failed).with_label("server", server),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get as route_get;
    use dumbmonit_proto::Credential;

    use super::*;
    use crate::uptime::tags::test_support::cible;

    // Captures d'un Caddy 2.11.4, option globale `metrics` posée : deux
    // serveurs, un serveur d'arrière-plan sain, un qui échoue au contrôle
    // actif, un troisième écarté par le contrôle passif après un 502.
    const SERVERS: &str = include_str!("testdata/servers.json");
    const UPSTREAMS: &str = include_str!("testdata/upstreams.json");
    const METRICS: &str = include_str!("testdata/caddy_2.11.4.prom");

    fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == name
                && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
        })
    }

    fn real() -> Vec<Sample> {
        let servers: Option<BTreeMap<String, Server>> = serde_json::from_str(SERVERS).unwrap();
        let upstreams: Vec<Upstream> = serde_json::from_str(UPSTREAMS).unwrap();
        samples(servers.as_ref(), &upstreams, Some(&Exposition::parse(METRICS)), 0)
    }

    #[test]
    fn caddy_reel() {
        let samples = real();
        assert_eq!(find(&samples, "caddy_config_loaded", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "caddy_servers", &[]).unwrap().value, 2.0);
        assert_eq!(find(&samples, "caddy_upstreams", &[]).unwrap().value, 3.0);
        assert_eq!(find(&samples, "caddy_upstreams_unhealthy", &[]).unwrap().value, 2.0);
        let sain = find(&samples, "caddy_upstream_healthy", &[("upstream", "whoami:80")]);
        assert_eq!(sain.unwrap().value, 1.0);
        let passif = find(&samples, "caddy_upstream_fails", &[("upstream", "10.99.99.98:80")]);
        assert_eq!(passif.unwrap().value, 1.0);
        assert_eq!(find(&samples, "caddy_config_last_reload_ok", &[]).unwrap().value, 1.0);
        let passif = find(&samples, "caddy_upstream_healthy", &[("upstream", "10.99.99.98:80")]);
        assert_eq!(passif.unwrap().value, 0.0, "écarté par le contrôle passif");
        assert_eq!(find(&samples, "caddy_requests_total", &[]).unwrap().value, 14.0);
        // Deux 502 et six 503, tous sur srv1.
        assert_eq!(find(&samples, "caddy_requests_5xx_total", &[]).unwrap().value, 8.0);
        let srv1 = find(&samples, "caddy_server_requests_5xx_total", &[("server", "srv1")]);
        assert_eq!((srv1.unwrap().value, srv1.unwrap().kind), (8.0, MetricKind::Counter));
        assert_eq!(
            find(&samples, "caddy_server_request_errors_total", &[("server", "srv1")])
                .unwrap()
                .value,
            8.0
        );
    }

    #[test]
    fn un_caddy_sans_configuration_ni_metriques_http() {
        let samples = samples(
            None,
            &[],
            Some(&Exposition::parse("caddy_config_last_reload_successful 0\n")),
            0,
        );
        assert_eq!(find(&samples, "caddy_config_loaded", &[]).unwrap().value, 0.0);
        assert_eq!(find(&samples, "caddy_config_last_reload_ok", &[]).unwrap().value, 0.0);
        assert!(find(&samples, "caddy_requests_total", &[]).is_none());
        assert_eq!(find(&samples, "caddy_upstreams_unhealthy", &[]).unwrap().value, 0.0);
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        address.to_string()
    }

    /// La vue en lecture seule de la notice : mot de passe, et `/metrics`
    /// que l'on peut ne pas laisser passer.
    #[tokio::test]
    async fn interrogation_complete_par_la_vue_en_lecture_seule() {
        fn guarded(headers: &HeaderMap, body: &'static str) -> (StatusCode, &'static str) {
            // dumbmonit:ro-secret, en basic.
            let ok = headers.get("authorization").and_then(|v| v.to_str().ok())
                == Some("Basic ZHVtYm1vbml0OnJvLXNlY3JldA==");
            if ok { (StatusCode::OK, body) } else { (StatusCode::UNAUTHORIZED, "") }
        }
        let app = Router::new()
            .route(SERVERS_PATH, route_get(|h: HeaderMap| async move { guarded(&h, SERVERS) }))
            .route(UPSTREAMS_PATH, route_get(|h: HeaderMap| async move { guarded(&h, UPSTREAMS) }))
            .route("/metrics", route_get(|| async { (StatusCode::FORBIDDEN, "") }));
        let address = serve(app).await;
        let mut target = cible("caddy", &address, &[]);
        target.credential = Credential::UsernamePassword {
            username: "dumbmonit".into(),
            password: "ro-secret".into(),
        };
        let samples = CaddyCollector::new().probe(&target).await.unwrap();
        assert_eq!(find(&samples, "caddy_upstreams", &[]).unwrap().value, 3.0);
        // Sans métriques, la santé n'est pas publiée ; les échecs retenus, si.
        assert!(find(&samples, "caddy_upstream_healthy", &[]).is_none());
        assert!(find(&samples, "caddy_upstream_fails", &[]).is_some());

        let error = CaddyCollector::new().probe(&cible("caddy", &address, &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }

    #[tokio::test]
    async fn un_hote_refuse_par_origins_est_explique() {
        let app = Router::new().route(
            SERVERS_PATH,
            route_get(|| async {
                (StatusCode::FORBIDDEN, r#"{"error":"host not allowed: 10.0.0.5:2019"}"#)
            }),
        );
        let address = serve(app).await;
        let error = CaddyCollector::new().probe(&cible("caddy", &address, &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(ref m) if m.contains("origins")), "{error}");
    }

    /// Contre une vraie instance : `DUMBMONIT_TEST_CADDY=hôte:port`, et pour
    /// la vue en lecture seule `DUMBMONIT_TEST_CADDY_USER` et `…_PASSWORD`.
    #[tokio::test]
    #[ignore = "demande un Caddy joignable"]
    async fn caddy_reel_joignable() {
        let address = std::env::var("DUMBMONIT_TEST_CADDY").unwrap();
        let mut target = cible("caddy", &address, &[]);
        if let (Ok(username), Ok(password)) = (
            std::env::var("DUMBMONIT_TEST_CADDY_USER"),
            std::env::var("DUMBMONIT_TEST_CADDY_PASSWORD"),
        ) {
            target.credential = Credential::UsernamePassword { username, password };
        }
        for sample in CaddyCollector::new().probe(&target).await.unwrap() {
            println!("{} {:?} {}", sample.metric, sample.labels, sample.value);
        }
    }
}
