//! Traefik : son API (routeurs, services, certificats) et, s'il les publie, ses
//! métriques Prometheus.
//!
//! L'API de Traefik ne sait que lire : aucune méthode n'y modifie quoi que ce
//! soit. Elle est lue sur `/api/overview`, `/api/http/routers`,
//! `/api/http/services` et `/api/certificates` (servi par Traefik 3.7) ;
//! `/metrics`, s'il répond au même endroit, apporte le volume de requêtes et
//! la part de réponses 5xx.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **un routeur désactivé** par une erreur de configuration (service absent,
//!   middleware inconnu, règle invalide) : son site ne répond plus, et Traefik
//!   ne le dit que dans son journal ;
//! * **un serveur d'arrière-plan qui échoue à son contrôle de santé**, ou pire,
//!   un service dont plus aucun serveur ne répond (503 « no available server ») ;
//! * **un certificat qui n'arrive pas** : un routeur demande un certificat à un
//!   résolveur ACME qui n'y parvient pas, et Traefik sert son certificat
//!   auto-signé par défaut ;
//! * **un certificat qui n'est pas renouvelé** à temps.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | Protocole de l'API. |
//! | `port` | `8080` | Port de l'API (point d'entrée `traefik` par défaut). |
//! | `metrics` | `true` | Lit aussi `/metrics` au même endroit, s'il répond. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |

use std::collections::BTreeMap;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target};
use serde::Deserialize;

use crate::observability::client::{Auth, HttpClient};
use crate::observability::options::Options;
use crate::observability::prom::Exposition;
use crate::uptime::tags;

pub const DEFAULT_PORT: u16 = 8080;
/// Routeurs, services et serveurs décrits un par un, au plus.
const MAX_ITEMS: usize = 500;
/// Certificats décrits un par un, au plus.
const MAX_CERTIFICATES: usize = 200;
/// Longueur maximale d'un message d'erreur repris en étiquette.
const MAX_ERROR_LABEL: usize = 200;
const DAY_SECONDS: f64 = 86_400.0;

#[derive(Default)]
pub struct TraefikCollector;

impl TraefikCollector {
    pub fn new() -> Self {
        Self
    }
}

fn client(target: &Target) -> Result<(HttpClient, bool), ProbeError> {
    let options = Options::from_target(target, DEFAULT_PORT)?;
    let auth = Auth::from_credential(&target.credential)?;
    let http = crate::http::client(options.insecure_tls)?;
    let client = HttpClient::new(http, options.base_url, auth, options.request_timeout, "Traefik");
    Ok((client, tags::parse_bool(target, "metrics", true)?))
}

#[async_trait]
impl Collector for TraefikCollector {
    fn kind(&self) -> &'static str {
        "traefik"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let (client, with_metrics) = client(target)?;
        let api = read_api(&client).await?;
        let page = if with_metrics { read_metrics(&client).await? } else { None };
        let now = chrono::Utc::now();
        Ok(samples(&api, page.as_ref(), now.timestamp(), now.timestamp_millis()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        read_overview(&client(target)?.0).await?;
        Ok(Some("traefik".to_string()))
    }
}

// ------------------------------------------------------------------- API

#[derive(Debug, Default, Deserialize)]
pub struct Counts {
    #[serde(default)]
    pub total: f64,
    #[serde(default)]
    pub warnings: f64,
    #[serde(default)]
    pub errors: f64,
}

#[derive(Debug, Default, Deserialize)]
pub struct Section {
    #[serde(default)]
    pub routers: Counts,
    #[serde(default)]
    pub services: Counts,
    #[serde(default)]
    pub middlewares: Counts,
}

#[derive(Debug, Deserialize)]
pub struct Overview {
    pub http: Section,
    #[serde(default)]
    pub tcp: Section,
    #[serde(default)]
    pub udp: Section,
}

#[derive(Debug, Default, Deserialize)]
pub struct RouterTls {
    #[serde(default, rename = "certResolver")]
    pub cert_resolver: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Router {
    pub name: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub error: Vec<String>,
    #[serde(default)]
    pub rule: String,
    #[serde(default)]
    pub tls: Option<RouterTls>,
}

#[derive(Debug, Default, Deserialize)]
pub struct LoadBalancer {
    #[serde(default)]
    pub servers: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Service {
    pub name: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub error: Vec<String>,
    #[serde(default, rename = "loadBalancer")]
    pub load_balancer: Option<LoadBalancer>,
    #[serde(default, rename = "serverStatus")]
    pub server_status: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct Certificate {
    #[serde(default, rename = "commonName")]
    pub common_name: String,
    #[serde(default)]
    pub sans: Vec<String>,
    #[serde(default, rename = "notAfter")]
    pub not_after: String,
}

#[derive(Debug, Deserialize)]
struct Version {
    #[serde(rename = "Version")]
    version: String,
}

/// Ce que l'API a rendu.
pub struct Api {
    pub version: Option<String>,
    pub overview: Overview,
    pub routers: Vec<Router>,
    pub services: Vec<Service>,
    /// `None` quand `/api/certificates` n'existe pas (versions plus anciennes).
    pub certificates: Option<Vec<Certificate>>,
}

async fn read_overview(client: &HttpClient) -> Result<Overview, ProbeError> {
    let reply = client.get_json_raw("/api/overview").await?;
    if !reply.status.is_success() {
        return Err(client.status_error(reply.status, &reply.body, "/api/overview"));
    }
    serde_json::from_str(&reply.body).map_err(|_| {
        ProbeError::Protocol(format!(
            "{} does not look like the Traefik API. Check the device type, the port (8080 for \
             the traefik entry point) and that the API is enabled (api: {{}} in the static \
             configuration).",
            client.url("/api/overview")
        ))
    })
}

/// Un point d'accès facultatif : `None` s'il n'existe pas sur cette version.
async fn optional<T: serde::de::DeserializeOwned>(
    client: &HttpClient,
    path: &str,
) -> Result<Option<T>, ProbeError> {
    let reply = client.get_json_raw(path).await?;
    if reply.status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !reply.status.is_success() {
        return Err(client.status_error(reply.status, &reply.body, path));
    }
    serde_json::from_str(&reply.body)
        .map(Some)
        .map_err(|error| ProbeError::Protocol(format!("Unexpected response from {path}: {error}")))
}

async fn read_api(client: &HttpClient) -> Result<Api, ProbeError> {
    let overview = read_overview(client).await?;
    let path = format!("/api/http/routers?per_page={MAX_ITEMS}");
    let routers: Vec<Router> = client.get_json(&path).await?;
    let path = format!("/api/http/services?per_page={MAX_ITEMS}");
    let services: Vec<Service> = client.get_json(&path).await?;
    let path = format!("/api/certificates?per_page={MAX_CERTIFICATES}");
    let certificates = optional::<Vec<Certificate>>(client, &path).await?;
    let version = optional::<Version>(client, "/api/version").await?.map(|v| v.version);
    Ok(Api { version, overview, routers, services, certificates })
}

/// `/metrics`, s'il répond ici. Traefik le publie sur le point d'entrée de
/// l'API par défaut ; ailleurs, ou désactivé, il n'y a rien à lire.
async fn read_metrics(client: &HttpClient) -> Result<Option<Exposition>, ProbeError> {
    let reply = client.get_raw("/metrics").await?;
    if matches!(reply.status.as_u16(), 404 | 405) {
        return Ok(None);
    }
    if !reply.status.is_success() {
        return Err(client.status_error(reply.status, &reply.body, "/metrics"));
    }
    let page = Exposition::parse(&reply.body);
    Ok((page.has("traefik_entrypoint_requests_total") || page.has("traefik_config_reloads_total"))
        .then_some(page))
}

// ------------------------------------------------------------ conversion

/// `enabled` 0, `warning` 1, tout le reste (`disabled`) 2 : la même échelle
/// que les santés Redfish, pour qu'une règle « >= 2 » lise « hors service ».
fn status_level(status: &str) -> f64 {
    match status {
        "enabled" => 0.0,
        "warning" => 1.0,
        _ => 2.0,
    }
}

fn first_error(errors: &[String]) -> Option<String> {
    let text = errors.iter().map(|e| e.trim()).find(|e| !e.is_empty())?;
    Some(text.chars().take(MAX_ERROR_LABEL).collect())
}

/// Les noms d'hôte d'une règle : `Host(`a`)`, `Host(`a`, `b`)`, reliés par
/// `||`. `HostRegexp` et `HostSNI` ne désignent pas un nom exact et sont
/// ignorés.
pub fn rule_hosts(rule: &str) -> Vec<String> {
    let mut hosts = Vec::new();
    let mut rest = rule;
    while let Some(index) = rest.find("Host(") {
        let preceded_by_word =
            rest[..index].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_');
        rest = &rest[index + 5..];
        let Some(end) = rest.find(')') else { break };
        if !preceded_by_word {
            for part in rest[..end].split(',') {
                let name = part.trim().trim_matches(|c| c == '`' || c == '"' || c == '\'');
                if !name.is_empty() {
                    hosts.push(name.to_ascii_lowercase());
                }
            }
        }
        rest = &rest[end..];
    }
    hosts
}

/// Vrai si l'un des noms du certificat couvre `host` (joker d'un seul niveau).
pub fn covers(names: &[String], host: &str) -> bool {
    names.iter().any(|name| {
        let name = name.to_ascii_lowercase();
        match name.strip_prefix("*.") {
            Some(domain) => host
                .split_once('.')
                .is_some_and(|(label, parent)| !label.is_empty() && parent == domain),
            None => name == host,
        }
    })
}

fn parse_time(raw: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(raw.trim()).ok().map(|t| t.timestamp())
}

pub fn samples(api: &Api, page: Option<&Exposition>, now_s: i64, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let counter = |name: &str, value: f64| Sample::new(name, value, MetricKind::Counter, ts_ms);

    if let Some(version) = &api.version {
        out.push(gauge("traefik_version_info", 1.0).with_label("version", version.clone()));
    }

    // Totaux de l'aperçu : HTTP, TCP et UDP confondus, toujours complets.
    let sections = [&api.overview.http, &api.overview.tcp, &api.overview.udp];
    let sum = |pick: fn(&Section) -> &Counts, field: fn(&Counts) -> f64| -> f64 {
        sections.iter().map(|s| field(pick(s))).sum()
    };
    out.push(gauge("traefik_routers", sum(|s| &s.routers, |c| c.total)));
    out.push(gauge("traefik_routers_errors", sum(|s| &s.routers, |c| c.errors)));
    out.push(gauge("traefik_routers_warnings", sum(|s| &s.routers, |c| c.warnings)));
    out.push(gauge("traefik_services", sum(|s| &s.services, |c| c.total)));
    out.push(gauge("traefik_services_errors", sum(|s| &s.services, |c| c.errors)));
    out.push(gauge("traefik_middlewares_errors", sum(|s| &s.middlewares, |c| c.errors)));

    // Routeurs HTTP un par un ; ceux de Traefik lui-même (`@internal`) n'ont
    // rien à dire.
    let routers: Vec<&Router> =
        api.routers.iter().filter(|r| r.provider != "internal").take(MAX_ITEMS).collect();
    for router in &routers {
        out.push(
            gauge("traefik_router_status", status_level(&router.status))
                .with_label("router", router.name.clone())
                .with_label("provider", router.provider.clone()),
        );
        if router.status != "enabled"
            && let Some(error) = first_error(&router.error)
        {
            out.push(
                gauge("traefik_router_error_info", 1.0)
                    .with_label("router", router.name.clone())
                    .with_label("error", error),
            );
        }
    }

    // Résolveurs de certificats : les routeurs qui en demandent un, et ceux
    // dont aucun certificat détenu ne couvre encore les noms.
    let mut per_resolver: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for router in &routers {
        let Some(resolver) = router
            .tls
            .as_ref()
            .and_then(|tls| tls.cert_resolver.as_deref())
            .map(str::trim)
            .filter(|r| !r.is_empty())
        else {
            continue;
        };
        let entry = per_resolver.entry(resolver.to_string()).or_default();
        entry.0 += 1.0;
        if let Some(certificates) = &api.certificates {
            let hosts = rule_hosts(&router.rule);
            let uncovered = hosts.iter().any(|host| {
                !certificates.iter().any(|c| {
                    let mut names = c.sans.clone();
                    names.push(c.common_name.clone());
                    covers(&names, host)
                })
            });
            if uncovered {
                entry.1 += 1.0;
            }
        }
    }
    for (resolver, (count, uncovered)) in per_resolver {
        out.push(gauge("traefik_resolver_routers", count).with_label("resolver", resolver.clone()));
        if api.certificates.is_some() {
            out.push(
                gauge("traefik_resolver_routers_uncovered", uncovered)
                    .with_label("resolver", resolver),
            );
        }
    }

    // Services et leurs serveurs, tels que le contrôle de santé les voit.
    let mut servers_emitted = 0usize;
    for service in api.services.iter().filter(|s| s.provider != "internal").take(MAX_ITEMS) {
        let name = service.name.clone();
        out.push(
            gauge("traefik_service_status", status_level(&service.status))
                .with_label("service", name.clone()),
        );
        if service.status != "enabled"
            && let Some(error) = first_error(&service.error)
        {
            out.push(
                gauge("traefik_service_error_info", 1.0)
                    .with_label("service", name.clone())
                    .with_label("error", error),
            );
        }
        // Sans `serverStatus` (service pondéré, miroir, ou que nul routeur
        // n'utilise), Traefik ne dit rien de ses serveurs : rien n'est inventé.
        if service.server_status.is_empty() {
            continue;
        }
        let declared = service.load_balancer.as_ref().map_or(0, |lb| lb.servers.len());
        let up = service.server_status.values().filter(|s| s.eq_ignore_ascii_case("UP")).count();
        let total = declared.max(service.server_status.len());
        out.push(
            gauge("traefik_service_servers", total as f64).with_label("service", name.clone()),
        );
        out.push(
            gauge("traefik_service_servers_up", up as f64).with_label("service", name.clone()),
        );
        for (server, state) in &service.server_status {
            if servers_emitted >= MAX_ITEMS {
                break;
            }
            servers_emitted += 1;
            out.push(
                gauge(
                    "traefik_server_up",
                    if state.eq_ignore_ascii_case("UP") { 1.0 } else { 0.0 },
                )
                .with_label("service", name.clone())
                .with_label("server", server.clone()),
            );
        }
    }

    // Certificats détenus : l'API d'abord, les métriques à défaut.
    let mut expiries: BTreeMap<String, i64> = BTreeMap::new();
    if let Some(certificates) = &api.certificates {
        for certificate in certificates {
            let name = if certificate.common_name.is_empty() {
                certificate.sans.first().cloned().unwrap_or_default()
            } else {
                certificate.common_name.clone()
            };
            if let (false, Some(not_after)) = (name.is_empty(), parse_time(&certificate.not_after))
            {
                expiries.entry(name).and_modify(|t| *t = (*t).max(not_after)).or_insert(not_after);
            }
        }
    } else if let Some(page) = page {
        for line in page.family("traefik_tls_certs_not_after") {
            if let Some(cn) = line.label("cn").filter(|cn| !cn.is_empty()) {
                let not_after = line.value as i64;
                expiries
                    .entry(cn.to_string())
                    .and_modify(|t| *t = (*t).max(not_after))
                    .or_insert(not_after);
            }
        }
    }
    if api.certificates.is_some() || page.is_some_and(|p| p.has("traefik_tls_certs_not_after")) {
        out.push(gauge("traefik_certificates", expiries.len() as f64));
    }
    for (name, not_after) in expiries.into_iter().take(MAX_CERTIFICATES) {
        out.push(
            gauge("traefik_cert_expiry_days", (not_after - now_s) as f64 / DAY_SECONDS)
                .with_label("cert", name),
        );
    }

    // Métriques : volume et réponses 5xx, par point d'entrée puis par routeur.
    if let Some(page) = page {
        let is_5xx = |line: &crate::observability::prom::Line| {
            line.label("code").is_some_and(|code| code.starts_with('5'))
        };
        if let Some(total) = page.sum("traefik_entrypoint_requests_total") {
            out.push(counter("traefik_requests_total", total));
            out.push(counter(
                "traefik_requests_5xx_total",
                page.sum_where("traefik_entrypoint_requests_total", is_5xx).unwrap_or(0.0),
            ));
        }
        let mut per_router: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        for line in page.family("traefik_router_requests_total") {
            let Some(router) = line.label("router") else { continue };
            if router.ends_with("@internal") || !line.value.is_finite() {
                continue;
            }
            let entry = per_router.entry(router.to_string()).or_default();
            entry.0 += line.value;
            if is_5xx(line) {
                entry.1 += line.value;
            }
        }
        for (router, (total, errors)) in per_router.into_iter().take(MAX_ITEMS) {
            out.push(
                counter("traefik_router_requests_total", total)
                    .with_label("router", router.clone()),
            );
            out.push(
                counter("traefik_router_requests_5xx_total", errors).with_label("router", router),
            );
        }
        if let Some(reloads) = page.sum("traefik_config_reloads_total") {
            out.push(counter("traefik_config_reloads_total", reloads));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use axum::Router as Routes;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;
    use dumbmonit_proto::Credential;

    use super::*;
    use crate::uptime::tags::test_support::cible;

    // Captures d'un Traefik v3.7.13 (fournisseur fichier) : neuf routeurs dont
    // un désactivé (service absent), un service dont un serveur échoue à son
    // contrôle de santé, un autre sans aucun serveur valide, un certificat
    // auto-signé à dix jours et un résolveur ACME qui n'obtient rien.
    const OVERVIEW: &str = include_str!("testdata/overview.json");
    const ROUTERS: &str = include_str!("testdata/http_routers.json");
    const SERVICES: &str = include_str!("testdata/http_services.json");
    const CERTIFICATES: &str = include_str!("testdata/certificates.json");
    const VERSION: &str = include_str!("testdata/version.json");
    const METRICS: &str = include_str!("testdata/traefik_3.7.13.prom");
    /// Date de la capture : le certificat expire dix jours plus tard.
    const CAPTURED_AT: i64 = 1_790_808_400;

    fn api() -> Api {
        Api {
            version: Some("3.7.13".into()),
            overview: serde_json::from_str(OVERVIEW).unwrap(),
            routers: serde_json::from_str(ROUTERS).unwrap(),
            services: serde_json::from_str(SERVICES).unwrap(),
            certificates: Some(serde_json::from_str(CERTIFICATES).unwrap()),
        }
    }

    fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == name
                && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
        })
    }

    #[test]
    fn traefik_reel() {
        let page = Exposition::parse(METRICS);
        let samples = samples(&api(), Some(&page), CAPTURED_AT, 0);
        assert_eq!(find(&samples, "traefik_routers", &[]).unwrap().value, 9.0);
        assert_eq!(find(&samples, "traefik_routers_errors", &[]).unwrap().value, 1.0);
        let wiki = find(&samples, "traefik_router_status", &[("router", "wiki@file")]).unwrap();
        assert_eq!(wiki.value, 2.0);
        let error = find(&samples, "traefik_router_error_info", &[("router", "wiki@file")]);
        assert!(error.unwrap().labels["error"].contains("wiki-missing@file"));
        assert_eq!(
            find(&samples, "traefik_router_status", &[("router", "blog@file")]).unwrap().value,
            0.0
        );
        assert!(
            find(&samples, "traefik_router_status", &[("router", "acme-http@internal")]).is_none(),
            "les routeurs internes ne sont pas décrits"
        );

        // Un serveur sur deux en panne, puis un service sans aucun serveur.
        let down = find(
            &samples,
            "traefik_server_up",
            &[("service", "blog@file"), ("server", "http://10.99.99.99:80")],
        );
        assert_eq!(down.unwrap().value, 0.0);
        assert_eq!(
            find(&samples, "traefik_service_servers_up", &[("service", "blog@file")])
                .unwrap()
                .value,
            1.0
        );
        assert_eq!(
            find(&samples, "traefik_service_servers_up", &[("service", "legacy@file")])
                .unwrap()
                .value,
            0.0
        );

        // Le résolveur « le » n'a rien obtenu pour cloud.example.test, « nope »
        // n'existe pas : ni l'un ni l'autre ne couvre son routeur.
        for resolver in ["le", "nope"] {
            let uncovered =
                find(&samples, "traefik_resolver_routers_uncovered", &[("resolver", resolver)]);
            assert_eq!(uncovered.unwrap().value, 1.0, "{resolver}");
        }
        let days = find(&samples, "traefik_cert_expiry_days", &[("cert", "shop.example.test")]);
        assert!((days.unwrap().value - 10.0).abs() < 0.1, "{:?}", days.unwrap().value);

        // 13 réponses 503 sur 31 requêtes, toutes sur le routeur legacy.
        assert_eq!(find(&samples, "traefik_requests_total", &[]).unwrap().value, 31.0);
        assert_eq!(find(&samples, "traefik_requests_5xx_total", &[]).unwrap().value, 13.0);
        let legacy =
            find(&samples, "traefik_router_requests_5xx_total", &[("router", "legacy@file")]);
        assert_eq!((legacy.unwrap().value, legacy.unwrap().kind), (13.0, MetricKind::Counter));
        assert_eq!(
            find(&samples, "traefik_version_info", &[]).unwrap().labels["version"],
            "3.7.13"
        );
    }

    #[test]
    fn sans_api_des_certificats_les_metriques_prennent_le_relais() {
        let mut api = api();
        api.certificates = None;
        let page = Exposition::parse(METRICS);
        let out = samples(&api, Some(&page), CAPTURED_AT, 0);
        assert!(find(&out, "traefik_cert_expiry_days", &[("cert", "shop.example.test")]).is_some());
        assert!(find(&out, "traefik_resolver_routers_uncovered", &[]).is_none());
        assert_eq!(
            find(&out, "traefik_resolver_routers", &[("resolver", "le")]).unwrap().value,
            1.0
        );
        let out = samples(&api, None, CAPTURED_AT, 0);
        assert!(find(&out, "traefik_requests_total", &[]).is_none());
        assert!(find(&out, "traefik_certificates", &[]).is_none());
    }

    #[test]
    fn les_noms_d_une_regle_et_les_jokers() {
        assert_eq!(
            rule_hosts("Host(`a.example.test`) || (Host(`B.example.test`) && PathPrefix(`/x`))"),
            vec!["a.example.test", "b.example.test"]
        );
        assert_eq!(rule_hosts("Host(`a.test`, `b.test`)"), vec!["a.test", "b.test"]);
        assert!(rule_hosts("HostRegexp(`^.+\\.test$`) || HostSNI(`*`)").is_empty());
        let names = vec!["*.example.test".to_string()];
        assert!(covers(&names, "cloud.example.test"));
        assert!(!covers(&names, "example.test"));
        assert!(!covers(&names, "a.b.example.test"));
    }

    async fn serve(app: Routes) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        address.to_string()
    }

    fn authorised(headers: &HeaderMap) -> bool {
        // dumbmonit:ro-secret, en basic.
        headers.get("authorization").and_then(|v| v.to_str().ok())
            == Some("Basic ZHVtYm1vbml0OnJvLXNlY3JldA==")
    }

    fn reply(headers: HeaderMap, body: &'static str) -> (StatusCode, &'static str) {
        if authorised(&headers) { (StatusCode::OK, body) } else { (StatusCode::UNAUTHORIZED, "") }
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let app = Routes::new()
            .route("/api/overview", get(|h: HeaderMap| async move { reply(h, OVERVIEW) }))
            .route("/api/http/routers", get(|h: HeaderMap| async move { reply(h, ROUTERS) }))
            .route("/api/http/services", get(|h: HeaderMap| async move { reply(h, SERVICES) }))
            .route("/api/certificates", get(|h: HeaderMap| async move { reply(h, CERTIFICATES) }))
            .route("/api/version", get(|h: HeaderMap| async move { reply(h, VERSION) }))
            .route("/metrics", get(|h: HeaderMap| async move { reply(h, METRICS) }));
        let address = serve(app).await;
        let mut target = cible("traefik", &address, &[]);
        target.credential = Credential::UsernamePassword {
            username: "dumbmonit".into(),
            password: "ro-secret".into(),
        };
        let samples = TraefikCollector::new().probe(&target).await.unwrap();
        assert_eq!(
            find(&samples, "traefik_version_info", &[]).unwrap().labels["version"],
            "3.7.13"
        );
        assert!(find(&samples, "traefik_requests_total", &[]).is_some());

        let target = cible("traefik", &address, &[("metrics", "false")]);
        let error = TraefikCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }

    #[tokio::test]
    async fn un_traefik_plus_ancien_sans_certificats_ni_metriques() {
        let app = Routes::new()
            .route("/api/overview", get(|| async { OVERVIEW }))
            .route("/api/http/routers", get(|| async { ROUTERS }))
            .route("/api/http/services", get(|| async { SERVICES }));
        let address = serve(app).await;
        let samples =
            TraefikCollector::new().probe(&cible("traefik", &address, &[])).await.unwrap();
        assert!(find(&samples, "traefik_routers", &[]).is_some());
        assert!(find(&samples, "traefik_version_info", &[]).is_none());
        assert!(find(&samples, "traefik_requests_total", &[]).is_none());
    }

    #[tokio::test]
    async fn autre_chose_que_traefik() {
        let app = Routes::new().route("/api/overview", get(|| async { r#"{"status":"ok"}"# }));
        let address = serve(app).await;
        let error =
            TraefikCollector::new().probe(&cible("traefik", &address, &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)), "{error}");
    }

    /// Contre une vraie instance : `DUMBMONIT_TEST_TRAEFIK=hôte:port`,
    /// `DUMBMONIT_TEST_TRAEFIK_USER` et `DUMBMONIT_TEST_TRAEFIK_PASSWORD`.
    #[tokio::test]
    #[ignore = "demande un Traefik joignable"]
    async fn traefik_reel_joignable() {
        let address = std::env::var("DUMBMONIT_TEST_TRAEFIK").unwrap();
        let mut target = cible("traefik", &address, &[]);
        if let (Ok(username), Ok(password)) = (
            std::env::var("DUMBMONIT_TEST_TRAEFIK_USER"),
            std::env::var("DUMBMONIT_TEST_TRAEFIK_PASSWORD"),
        ) {
            target.credential = Credential::UsernamePassword { username, password };
        }
        for sample in TraefikCollector::new().probe(&target).await.unwrap() {
            println!("{} {:?} {}", sample.metric, sample.labels, sample.value);
        }
    }
}
