//! CrowdSec : ses métriques Prometheus, et la santé de son API locale (LAPI).
//!
//! L'essentiel vient de `/metrics` (port 6060) : décisions actives par
//! origine, alertes gardées par la LAPI, débordements de scénarios, requêtes
//! de chaque bouncer, lignes lues et lignes que les parseurs n'ont pas su lire.
//! La LAPI (port 8080) est interrogée sur `/health` ; avec une clé de bouncer,
//! une recherche de décisions sur une adresse de documentation (192.0.2.1)
//! prouve en plus que l'authentification et la base répondent. Aucune décision
//! n'est jamais créée ni supprimée, et la liste des adresses bannies n'est
//! jamais téléchargée.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **un bouncer qui ne tire plus** : le pare-feu ou le proxy continue de
//!   tourner avec une liste figée, les nouvelles attaques passent ;
//! * **une LAPI arrêtée** : plus aucune décision n'est prise ni distribuée ;
//! * **plus aucune ligne lue** : l'acquisition est cassée (fichier renommé,
//!   conteneur recréé), CrowdSec ne voit plus rien et ne dit rien.
//!
//! L'âge du dernier tirage d'un bouncer n'est pas publié par CrowdSec ; il se
//! déduit du compteur de ses requêtes : tant qu'il avance, le bouncer tire.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | Protocole de `/metrics`. |
//! | `port` | `6060` | Port de `/metrics`. |
//! | `lapi` | `true` | Vérifie aussi la LAPI sur ce même hôte. |
//! | `lapi_port` | `8080` | Port de la LAPI. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |

use std::collections::BTreeMap;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};

use crate::observability::client::{Auth, HttpClient};
use crate::observability::options::Options;
use crate::observability::prom::Exposition;
use crate::uptime::tags;

pub const DEFAULT_PORT: u16 = 6060;
pub const DEFAULT_LAPI_PORT: u16 = 8080;
/// Sources de journaux décrites une par une, au plus.
const MAX_SOURCES: usize = 50;
/// Bouncers et machines décrits un par un, au plus.
const MAX_CLIENTS: usize = 100;
/// Adresse de documentation (RFC 5737) : aucune décision ne peut la viser.
const PROBE_IP: &str = "192.0.2.1";

#[derive(Default)]
pub struct CrowdsecCollector;

impl CrowdsecCollector {
    pub fn new() -> Self {
        Self
    }
}

struct Clients {
    metrics: HttpClient,
    lapi: Option<HttpClient>,
    bouncer_key: Option<String>,
}

fn clients(target: &Target) -> Result<Clients, ProbeError> {
    let options = Options::from_target(target, DEFAULT_PORT)?;
    let bouncer_key = match &target.credential {
        Credential::None => None,
        Credential::ApiToken { token } if !token.trim().is_empty() => Some(token.trim().to_string()),
        other => {
            return Err(ProbeError::Config(format!(
                "CrowdSec expects no credential or a bouncer API key, configured: {other}"
            )));
        }
    };
    let http = crate::http::client(options.insecure_tls)?;
    let lapi = if tags::parse_bool(target, "lapi", true)? {
        let port = tags::parse_u32(target, "lapi_port", u32::from(DEFAULT_LAPI_PORT), 1..=65535)?;
        let mut url = reqwest::Url::parse(&options.base_url)
            .map_err(|error| ProbeError::Config(format!("Invalid address: {error}")))?;
        url.set_path("");
        let _ = url.set_port(Some(port as u16));
        let base = url.as_str().trim_end_matches('/').to_string();
        Some(HttpClient::new(http.clone(), base, Auth::None, options.request_timeout, "CrowdSec LAPI"))
    } else {
        None
    };
    let metrics =
        HttpClient::new(http, options.base_url, Auth::None, options.request_timeout, "CrowdSec");
    Ok(Clients { metrics, lapi, bouncer_key })
}

#[async_trait]
impl Collector for CrowdsecCollector {
    fn kind(&self) -> &'static str {
        "crowdsec"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let clients = clients(target)?;
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let page = read_metrics(&clients.metrics).await?;
        let mut samples = samples(&page, ts_ms);
        if let Some(lapi) = &clients.lapi {
            samples.extend(check_lapi(lapi, clients.bouncer_key.as_deref(), ts_ms).await?);
        }
        Ok(samples)
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        read_metrics(&clients(target)?.metrics).await?;
        Ok(Some("crowdsec".to_string()))
    }
}

async fn read_metrics(client: &HttpClient) -> Result<Exposition, ProbeError> {
    let page = Exposition::parse(&client.get_text("/metrics").await?);
    if !page.has("cs_info") {
        return Err(ProbeError::Protocol(format!(
            "{} does not look like CrowdSec: its /metrics has no cs_info. Check the device type \
             and the port (prometheus.listen_port in config.yaml, 6060 by default).",
            client.url("/metrics")
        )));
    }
    Ok(page)
}

/// `/health`, puis, avec une clé, une recherche de décisions qui ne peut rien
/// rendre. Une LAPI arrêtée est une mesure (`lapi_up = 0`), pas une erreur :
/// les métriques de l'agent restent lisibles et utiles.
async fn check_lapi(
    lapi: &HttpClient,
    bouncer_key: Option<&str>,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let up = match lapi.get_raw("/health").await {
        Ok(reply) => reply.status.is_success(),
        Err(error) if error.means_down() => false,
        Err(error) => return Err(error),
    };
    let mut out = vec![gauge("crowdsec_lapi_up", if up { 1.0 } else { 0.0 })];
    if let (true, Some(key)) = (up, bouncer_key) {
        let path = format!("/v1/decisions?ip={PROBE_IP}");
        let reply = lapi.get_with_header(&path, "X-Api-Key", key).await?;
        match reply.status.as_u16() {
            200 => out.push(gauge("crowdsec_lapi_decisions_ok", 1.0)),
            401 | 403 => {
                return Err(ProbeError::Auth(
                    "The CrowdSec LAPI refused the bouncer API key (403). Check the key, or \
                     create one with: cscli bouncers add dumbmonit"
                        .to_string(),
                ));
            }
            _ => out.push(gauge("crowdsec_lapi_decisions_ok", 0.0)),
        }
    }
    Ok(out)
}

pub fn samples(page: &Exposition, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let counter = |name: &str, value: f64| Sample::new(name, value, MetricKind::Counter, ts_ms);

    if let Some(version) = page.first("cs_info").and_then(|line| line.label("version")) {
        // `v1.7.0-c3036e21` : la version, sans le commit.
        let version = version.trim_start_matches('v');
        let version = version.split_once('-').map_or(version, |(v, _)| v);
        out.push(gauge("crowdsec_version_info", 1.0).with_label("version", version));
    }

    // LAPI : présentes seulement là où elle tourne.
    if page.has("cs_active_decisions") || page.has("cs_lapi_route_requests_total") {
        out.push(gauge("crowdsec_decisions", page.sum("cs_active_decisions").unwrap_or(0.0)));
        for (origin, value) in page.sum_by("cs_active_decisions", "origin") {
            if !origin.is_empty() {
                out.push(gauge("crowdsec_decisions_by_origin", value).with_label("origin", origin));
            }
        }
        out.push(gauge("crowdsec_alerts", page.sum("cs_alerts").unwrap_or(0.0)));
        let bouncers = page.sum_by("cs_lapi_bouncer_requests_total", "bouncer");
        out.push(gauge("crowdsec_bouncers_seen", bouncers.len() as f64));
        for (bouncer, value) in bouncers.into_iter().take(MAX_CLIENTS) {
            out.push(
                counter("crowdsec_bouncer_requests_total", value).with_label("bouncer", bouncer),
            );
        }
        for (machine, value) in
            page.sum_by("cs_lapi_machine_requests_total", "machine").into_iter().take(MAX_CLIENTS)
        {
            out.push(
                counter("crowdsec_machine_requests_total", value).with_label("machine", machine),
            );
        }
        if let Some(total) = page.sum("cs_lapi_route_requests_total") {
            out.push(counter("crowdsec_lapi_requests_total", total));
        }
    }

    // Agent : présentes seulement là où il lit des journaux.
    if page.has("cs_parser_hits_total") || page.has("cs_bucket_overflowed_total") {
        out.push(counter(
            "crowdsec_scenario_overflows_total",
            page.sum("cs_bucket_overflowed_total").unwrap_or(0.0),
        ));
        let read = page.sum("cs_parser_hits_total").unwrap_or(0.0);
        let unparsed = page.sum("cs_parser_hits_ko_total").unwrap_or(0.0);
        out.push(counter("crowdsec_lines_read_total", read));
        out.push(counter("crowdsec_lines_parsed_total", page.sum("cs_parser_hits_ok_total").unwrap_or(0.0)));
        out.push(counter("crowdsec_lines_unparsed_total", unparsed));
        let totals = page.sum_by("cs_parser_hits_total", "source");
        let failures: BTreeMap<String, f64> = page.sum_by("cs_parser_hits_ko_total", "source");
        for (source, value) in totals.into_iter().filter(|(s, _)| !s.is_empty()).take(MAX_SOURCES) {
            let ko = failures.get(&source).copied().unwrap_or(0.0);
            out.push(counter("crowdsec_source_lines_total", value).with_label("source", source.clone()));
            out.push(counter("crowdsec_source_lines_unparsed_total", ko).with_label("source", source));
        }
        if let Some(buckets) = page.sum("cs_buckets") {
            out.push(gauge("crowdsec_buckets", buckets));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;

    use super::*;
    use crate::uptime::tags::test_support::cible;

    /// `/metrics` d'un CrowdSec 1.7.0 (image officielle, LAPI et agent dans le
    /// même conteneur) après une attaque SSH simulée : quatre décisions, quatre
    /// alertes, deux bouncers qui ont tiré, deux lignes que les parseurs n'ont
    /// pas reconnues. Les lignes `_bucket` des histogrammes ont été retirées.
    const METRICS: &str = include_str!("testdata/crowdsec_1.7.0.prom");

    fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == name
                && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
        })
    }

    #[test]
    fn crowdsec_reel() {
        let samples = samples(&Exposition::parse(METRICS), 0);
        assert_eq!(find(&samples, "crowdsec_version_info", &[]).unwrap().labels["version"], "1.7.0");
        assert_eq!(find(&samples, "crowdsec_decisions", &[]).unwrap().value, 4.0);
        assert_eq!(
            find(&samples, "crowdsec_decisions_by_origin", &[("origin", "crowdsec")]).unwrap().value,
            4.0
        );
        assert_eq!(find(&samples, "crowdsec_alerts", &[]).unwrap().value, 4.0);
        assert_eq!(find(&samples, "crowdsec_bouncers_seen", &[]).unwrap().value, 2.0);
        let firewall =
            find(&samples, "crowdsec_bouncer_requests_total", &[("bouncer", "firewall")]).unwrap();
        assert_eq!((firewall.value, firewall.kind), (1.0, MetricKind::Counter));
        assert_eq!(find(&samples, "crowdsec_scenario_overflows_total", &[]).unwrap().value, 4.0);
        assert_eq!(find(&samples, "crowdsec_lines_read_total", &[]).unwrap().value, 14.0);
        assert_eq!(find(&samples, "crowdsec_lines_unparsed_total", &[]).unwrap().value, 2.0);
        let source = find(
            &samples,
            "crowdsec_source_lines_unparsed_total",
            &[("source", "/var/log/auth.log")],
        );
        assert_eq!(source.unwrap().value, 2.0);
    }

    #[test]
    fn une_lapi_seule_ne_publie_pas_de_lignes_lues() {
        let page = Exposition::parse(
            "cs_info{version=\"v1.6.8-debian\"} 0\n\
             cs_lapi_route_requests_total{method=\"GET\",route=\"/v1/decisions/stream\"} 12\n",
        );
        let samples = samples(&page, 0);
        assert_eq!(find(&samples, "crowdsec_version_info", &[]).unwrap().labels["version"], "1.6.8");
        assert_eq!(find(&samples, "crowdsec_decisions", &[]).unwrap().value, 0.0);
        assert!(find(&samples, "crowdsec_lines_read_total", &[]).is_none());
    }

    async fn serve(app: Router) -> (String, u16) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (address.to_string(), address.port())
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let (metrics, _) = serve(Router::new().route("/metrics", get(|| async { METRICS }))).await;
        let (_, lapi_port) = serve(
            Router::new().route("/health", get(|| async { r#"{"status":"up"}"# })).route(
                "/v1/decisions",
                get(|headers: HeaderMap| async move {
                    match headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
                        Some("bouncer-key") => (StatusCode::OK, "null"),
                        _ => (StatusCode::FORBIDDEN, r#"{"message":"access forbidden"}"#),
                    }
                }),
            ),
        )
        .await;
        let port = lapi_port.to_string();
        let mut target = cible("crowdsec", &metrics, &[("lapi_port", &port)]);
        target.credential = Credential::ApiToken { token: "bouncer-key".into() };
        let samples = CrowdsecCollector::new().probe(&target).await.unwrap();
        assert_eq!(find(&samples, "crowdsec_lapi_up", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "crowdsec_lapi_decisions_ok", &[]).unwrap().value, 1.0);

        target.credential = Credential::ApiToken { token: "wrong".into() };
        let error = CrowdsecCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");

        // LAPI arrêtée : une mesure, pas une erreur.
        let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let closed_port = closed.local_addr().unwrap().port().to_string();
        drop(closed);
        let target = cible("crowdsec", &metrics, &[("lapi_port", &closed_port)]);
        let samples = CrowdsecCollector::new().probe(&target).await.unwrap();
        assert_eq!(find(&samples, "crowdsec_lapi_up", &[]).unwrap().value, 0.0);

        let target = cible("crowdsec", &metrics, &[("lapi", "false")]);
        let samples = CrowdsecCollector::new().probe(&target).await.unwrap();
        assert!(find(&samples, "crowdsec_lapi_up", &[]).is_none());
    }

    #[tokio::test]
    async fn un_autre_exportateur_n_est_pas_crowdsec() {
        let (metrics, _) =
            serve(Router::new().route("/metrics", get(|| async { "go_info{version=\"go1\"} 1\n" })))
                .await;
        let target = cible("crowdsec", &metrics, &[("lapi", "false")]);
        let error = CrowdsecCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)), "{error}");
    }

    /// Contre une vraie instance : `DUMBMONIT_TEST_CROWDSEC=hôte` et
    /// `DUMBMONIT_TEST_CROWDSEC_KEY` (clé de bouncer).
    #[tokio::test]
    #[ignore = "demande un CrowdSec joignable"]
    async fn crowdsec_reel_joignable() {
        let address = std::env::var("DUMBMONIT_TEST_CROWDSEC").unwrap();
        let token = std::env::var("DUMBMONIT_TEST_CROWDSEC_KEY").unwrap();
        let mut target = cible("crowdsec", &address, &[]);
        target.credential = Credential::ApiToken { token };
        for sample in CrowdsecCollector::new().probe(&target).await.unwrap() {
            println!("{} {:?} {}", sample.metric, sample.labels, sample.value);
        }
    }
}
