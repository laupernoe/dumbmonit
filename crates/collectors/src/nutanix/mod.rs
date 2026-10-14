//! Nutanix AHV, par les API REST v4 de Prism Central (`/api/<espace>/v4.0/…`,
//! disponibles en version stable depuis pc.2024.3).
//!
//! # Pourquoi v4, et Prism Central
//!
//! Nutanix a déclaré obsolètes les API v1, v2.0 et v3 au profit de v4, qui
//! seule continue d'évoluer. v4 est servie par Prism Central, qui voit tous
//! les clusters qui lui sont rattachés : une seule cible DumbMonit couvre un
//! site entier. Les chemins et noms de champs viennent des clients officiels
//! générés (`nutanix/ntnx-api-golang-clients`, étiquettes `*-go-client/v4.0.1`),
//! eux-mêmes produits à partir des spécifications OpenAPI publiées.
//!
//! # Ce qui est lu
//!
//! | Ressource | Chemin |
//! |---|---|
//! | Clusters | `clustermgmt/v4.0/config/clusters` |
//! | Hôtes | `clustermgmt/v4.0/config/hosts` |
//! | VM AHV | `vmm/v4.0/ahv/config/vms` |
//! | Conteneurs de stockage | `clustermgmt/v4.0/config/storage-containers` |
//! | Remplissage d'un conteneur | `clustermgmt/v4.0/stats/storage-containers/{extId}` |
//! | Alertes non résolues | `monitoring/v4.0/serviceability/alerts?$filter=isResolved eq false` |
//!
//! Les listes sont paginées (`$page` à partir de 0, `$limit` au plus 100). Les
//! quatre premières sont obligatoires ; le remplissage des conteneurs et les
//! alertes sont des vues annexes, qui ne font pas échouer la collecte si un
//! rôle restreint les refuse.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `9440` | Port HTTPS de Prism Central. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui de Prism par défaut). |
//! | `request_timeout_seconds` | `15` | Délai par appel. |

pub mod metrics;

use async_trait::async_trait;
use base64::Engine;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::Value;

use crate::rest::{RestClient, now_ms};

pub const DEFAULT_PORT: u16 = 9440;

const CLUSTERS: &str = "/api/clustermgmt/v4.0/config/clusters";
const HOSTS: &str = "/api/clustermgmt/v4.0/config/hosts";
const VMS: &str = "/api/vmm/v4.0/ahv/config/vms";
const CONTAINERS: &str = "/api/clustermgmt/v4.0/config/storage-containers";
const CONTAINER_STATS: &str = "/api/clustermgmt/v4.0/stats/storage-containers";
const ALERTS: &str = "/api/monitoring/v4.0/serviceability/alerts";

/// Filtre OData des alertes encore ouvertes, déjà encodé pour l'URL.
const UNRESOLVED: &str = "$filter=isResolved%20eq%20false";

/// Taille de page maximale acceptée par les API v4.
const PAGE_SIZE: usize = 100;

/// Au plus autant de pages par liste : 1 000 éléments, bien au-delà d'un
/// homelab ou d'une PME ; les comptes restent ceux annoncés par Prism.
const MAX_PAGES: usize = 10;

/// Au plus autant de conteneurs dont le remplissage est demandé (un appel
/// chacun).
const MAX_CONTAINER_STATS: usize = 50;

/// Fenêtre des statistiques de remplissage : la dernière valeur des trente
/// dernières minutes, par pas de cinq.
const STATS_WINDOW_SECS: i64 = 30 * 60;
const STATS_INTERVAL_SECS: u32 = 300;

#[derive(Default)]
pub struct NutanixCollector;

impl NutanixCollector {
    pub fn new() -> Self {
        Self
    }
}

fn client(target: &Target) -> Result<RestClient, ProbeError> {
    let (username, password) = match &target.credential {
        Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
            (username.trim().to_string(), password.clone())
        }
        other => {
            return Err(ProbeError::Config(format!(
                "Nutanix expects a user name and password, configured: {other}"
            )));
        }
    };
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
    Ok(RestClient::for_target(target, "https", DEFAULT_PORT, "Nutanix Prism Central")?
        .with_header("Authorization", format!("Basic {encoded}")))
}

/// Une liste v4 complète : `data` de chaque page, et le total annoncé par
/// `metadata.totalAvailableResults` (à défaut, le nombre d'éléments lus).
pub struct Listing {
    pub items: Vec<Value>,
    pub total: usize,
}

async fn list(client: &RestClient, path: &str, query: &str) -> Result<Listing, ProbeError> {
    let mut items = Vec::new();
    let mut total = None;
    for page in 0..MAX_PAGES {
        let separator = if query.is_empty() { "" } else { "&" };
        let url = format!("{path}?{query}{separator}$page={page}&$limit={PAGE_SIZE}");
        let body: Value = client.get_json(&url).await?;
        if total.is_none() {
            total = body
                .pointer("/metadata/totalAvailableResults")
                .and_then(Value::as_u64)
                .map(|n| n as usize);
        }
        let data = body.get("data").and_then(Value::as_array).cloned().unwrap_or_default();
        let received = data.len();
        items.extend(data);
        if received < PAGE_SIZE || total.is_some_and(|total| items.len() >= total) {
            break;
        }
    }
    let total = total.unwrap_or(items.len()).max(items.len());
    Ok(Listing { items, total })
}

/// Dernier remplissage connu d'un conteneur : `(capacité, utilisé)`, en
/// octets. `None` : statistiques refusées ou vides, le conteneur reste
/// compté sans pourcentage.
async fn container_usage(
    client: &RestClient,
    ext_id: &str,
    now_ms: i64,
) -> Option<(Option<f64>, Option<f64>)> {
    // Un identifiant v4 est un UUID : rien d'autre ne part dans le chemin.
    if ext_id.is_empty() || !ext_id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return None;
    }
    let end = chrono::DateTime::from_timestamp_millis(now_ms)?;
    let start = end - chrono::Duration::seconds(STATS_WINDOW_SECS);
    let format = |at: chrono::DateTime<chrono::Utc>| at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let url = format!(
        "{CONTAINER_STATS}/{ext_id}?$startTime={}&$endTime={}&$samplingInterval={STATS_INTERVAL_SECS}&$statType=LAST",
        format(start),
        format(end),
    );
    let body: Value = client.get_json(&url).await.ok()?;
    let stats = body.get("data")?;
    Some((
        metrics::latest(stats.get("storageCapacityBytes")),
        metrics::latest(stats.get("storageUsageBytes")),
    ))
}

impl NutanixCollector {
    async fn collect(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let client = client(target)?;
        let ts_ms = now_ms();
        let clusters = list(&client, CLUSTERS, "").await?;
        let hosts = list(&client, HOSTS, "").await?;
        let vms = list(&client, VMS, "").await?;
        let containers = list(&client, CONTAINERS, "").await?;

        let mut usage = Vec::new();
        for container in metrics::watched_containers(&containers.items).take(MAX_CONTAINER_STATS) {
            let ext_id = metrics::container_id(container).unwrap_or_default();
            if let Some(found) = container_usage(&client, ext_id, ts_ms).await {
                usage.push((ext_id.to_string(), found));
            }
        }

        let names = metrics::cluster_names(&clusters.items);
        let mut out = metrics::cluster_samples(&clusters.items, ts_ms);
        out.extend(metrics::host_samples(&hosts, ts_ms));
        out.extend(metrics::vm_samples(&vms, &names, ts_ms));
        out.extend(metrics::storage_container_samples(&containers.items, &usage, ts_ms));
        // Vue annexe : refusées (rôle sans accès à la surveillance), les
        // alertes n'empêchent pas le reste de la collecte.
        if let Ok(alerts) = list(&client, ALERTS, UNRESOLVED).await {
            out.extend(metrics::alert_samples(&alerts, ts_ms));
        }
        Ok(out)
    }
}

#[async_trait]
impl Collector for NutanixCollector {
    fn kind(&self) -> &str {
        "nutanix"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        self.collect(target).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let client = client(target)?;
        client.get_json::<Value>(&format!("{CLUSTERS}?$limit=1")).await?;
        Ok(Some("nutanix".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::extract::{Path, Query, State};
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    const CLUSTERS_JSON: &str = include_str!("testdata/clusters.json");
    const HOSTS_JSON: &str = include_str!("testdata/hosts.json");
    const VMS_JSON: &str = include_str!("testdata/vms.json");
    const CONTAINERS_JSON: &str = include_str!("testdata/storage_containers.json");
    const CONTAINER_STATS_JSON: &str = include_str!("testdata/storage_container_stats.json");
    const ALERTS_JSON: &str = include_str!("testdata/alerts.json");

    type Seen = Arc<Mutex<Vec<HashMap<String, String>>>>;

    fn authorised(headers: &HeaderMap) -> bool {
        let expected = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("dumbmonit:s3cret")
        );
        headers.get("authorization").and_then(|v| v.to_str().ok()) == Some(expected.as_str())
    }

    fn reply(headers: &HeaderMap, body: &'static str) -> (StatusCode, String) {
        if !authorised(headers) {
            return (StatusCode::UNAUTHORIZED, r#"{"data":{"error":[]}}"#.to_string());
        }
        (StatusCode::OK, body.to_string())
    }

    fn app(seen: Seen, alerts_forbidden: bool) -> Router {
        let listed = |body: &'static str| {
            move |State(seen): State<Seen>,
                  headers: HeaderMap,
                  Query(query): Query<HashMap<String, String>>| async move {
                seen.lock().unwrap().push(query);
                reply(&headers, body)
            }
        };
        Router::new()
            .route(CLUSTERS, get(listed(CLUSTERS_JSON)))
            .route(HOSTS, get(listed(HOSTS_JSON)))
            .route(VMS, get(listed(VMS_JSON)))
            .route(CONTAINERS, get(listed(CONTAINERS_JSON)))
            .route(
                &format!("{CONTAINER_STATS}/{{id}}"),
                get(
                    |headers: HeaderMap,
                     Path(id): Path<String>,
                     Query(query): Query<HashMap<String, String>>| async move {
                        assert!(query.contains_key("$startTime") && query.contains_key("$endTime"));
                        if id != "c1a7e8a0-0000-4000-8000-000000000001" {
                            return (StatusCode::NOT_FOUND, "{}".to_string());
                        }
                        reply(&headers, CONTAINER_STATS_JSON)
                    },
                ),
            )
            .route(
                ALERTS,
                get(
                    move |headers: HeaderMap, Query(query): Query<HashMap<String, String>>| async move {
                        if alerts_forbidden {
                            return (StatusCode::FORBIDDEN, "{}".to_string());
                        }
                        assert_eq!(query.get("$filter").map(String::as_str), Some("isResolved eq false"));
                        reply(&headers, ALERTS_JSON)
                    },
                ),
            )
            .with_state(seen)
    }

    fn target(address: &str, password: &str) -> Target {
        let mut target = cible("nutanix", address, &[]);
        target.credential = Credential::UsernamePassword {
            username: "dumbmonit".into(),
            password: password.into(),
        };
        target
    }

    #[tokio::test]
    async fn a_full_collection_reads_every_resource() {
        let seen = Seen::default();
        let address = serve(app(seen.clone(), false)).await;
        let samples = NutanixCollector::new()
            .probe(&target(&format!("http://{address}"), "s3cret"))
            .await
            .unwrap();
        assert_eq!(value(&samples, "nutanix_clusters", &[]), 1.0);
        assert_eq!(value(&samples, "nutanix_hosts", &[]), 3.0);
        assert_eq!(value(&samples, "nutanix_vms", &[]), 3.0);
        assert_eq!(value(&samples, "nutanix_storage_containers", &[]), 2.0);
        let used = value(
            &samples,
            "nutanix_storage_container_used_percent",
            &[("container", "default-container")],
        );
        assert!((used - 92.0).abs() < 0.01, "{used}");
        assert_eq!(value(&samples, "nutanix_alerts_critical", &[]), 1.0);
        // Chaque liste demande des pages de 100 à partir de la page 0.
        for query in seen.lock().unwrap().iter() {
            assert_eq!(query.get("$page").map(String::as_str), Some("0"));
            assert_eq!(query.get("$limit").map(String::as_str), Some("100"));
        }
    }

    #[tokio::test]
    async fn refused_alerts_leave_the_rest_of_the_collection() {
        let address = serve(app(Seen::default(), true)).await;
        let samples = NutanixCollector::new()
            .probe(&target(&format!("http://{address}"), "s3cret"))
            .await
            .unwrap();
        assert!(find(&samples, "nutanix_vms", &[]).is_some());
        assert!(find(&samples, "nutanix_alerts_critical", &[]).is_none());
    }

    #[tokio::test]
    async fn wrong_credentials_are_an_auth_error() {
        let address = serve(app(Seen::default(), false)).await;
        let error = NutanixCollector::new()
            .probe(&target(&format!("http://{address}"), "wrong"))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn a_missing_credential_is_a_configuration_error() {
        let mut target = target("http://127.0.0.1:1", "s3cret");
        target.credential = Credential::None;
        let error = NutanixCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[tokio::test]
    async fn an_unreachable_prism_means_down() {
        let error = NutanixCollector::new()
            .probe(&target("http://127.0.0.1:1", "s3cret"))
            .await
            .unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[tokio::test]
    async fn a_container_id_that_is_not_a_uuid_is_never_requested() {
        let client = client(&target("http://127.0.0.1:1", "s3cret")).unwrap();
        assert!(container_usage(&client, "../../config/clusters", 0).await.is_none());
    }
}
