//! MinIO, par ses métriques Prometheus (`/minio/v2/metrics/cluster`).
//!
//! MinIO publie l'état de la grappe en texte Prometheus. Le jeton est un JWT
//! que `mc admin prometheus generate` fabrique : il ne donne accès qu'à ces
//! pages de métriques, aucun autre droit n'est demandé. Il part en
//! `Authorization: Bearer`, jamais dans l'URL. Si le serveur est lancé avec
//! `MINIO_PROMETHEUS_AUTH_TYPE=public`, la cible peut se passer de jeton.
//!
//! Une lecture obligatoire — la page de la grappe — et une facultative, la
//! page du nœud interrogé (durée de fonctionnement), qu'un serveur plus ancien
//! peut ne pas servir sans faire échouer le reste. Les familles de métriques
//! ont changé de nom d'une version à l'autre (`disk` devenu `drive`) : les deux
//! noms sont lus, et une famille absente ne produit simplement pas de mesure.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **un nœud ou un disque hors ligne** : la parité s'amenuise ;
//! * **une grappe qui se remplit** ;
//! * **une réparation (heal) qui échoue** ;
//! * **une réplication entre sites qui accumule des échecs** ;
//! * **des requêtes S3 qui échouent côté serveur** (5xx).
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | Protocole de l'API S3. |
//! | `port` | `9000` | Port de l'API S3 (la console, 9001, n'est pas utilisée). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};

use crate::observability::prom::Exposition;
use crate::rest::{RestClient, counter, flag, gauge, now_ms};

pub const DEFAULT_PORT: u16 = 9000;

const CLUSTER: &str = "/minio/v2/metrics/cluster";
const NODE: &str = "/minio/v2/metrics/node";

/// Une réparation est dite active si elle a eu lieu dans les cinq dernières minutes.
const HEAL_ACTIVE_SECONDS: f64 = 300.0;

#[derive(Default)]
pub struct MinioCollector;

impl MinioCollector {
    pub fn new() -> Self {
        Self
    }
}

fn client(target: &Target) -> Result<RestClient, ProbeError> {
    let client = RestClient::for_target(target, "http", DEFAULT_PORT, "MinIO")?;
    match &target.credential {
        Credential::None => Ok(client),
        Credential::ApiToken { token } if !token.trim().is_empty() => {
            Ok(client.with_header("Authorization", format!("Bearer {}", token.trim())))
        }
        other => Err(ProbeError::Config(format!(
            "MinIO expects the Prometheus token made by mc admin prometheus generate, or no \
             credential, configured: {other}"
        ))),
    }
}

async fn read(client: &RestClient, path: &str) -> Result<Exposition, ProbeError> {
    let (status, body) = client.get(path).await?;
    if !status.is_success() {
        return Err(client.status_error(status, &body, path));
    }
    Ok(Exposition::parse(&body))
}

/// La page de la grappe, qui doit ressembler à celle de MinIO.
async fn read_cluster(client: &RestClient) -> Result<Exposition, ProbeError> {
    let page = read(client, CLUSTER).await?;
    let known = page.has("minio_cluster_nodes_online_total")
        || page.has("minio_cluster_capacity_raw_total_bytes")
        || page.has("minio_cluster_drive_online_total")
        || page.has("minio_cluster_disk_online_total");
    if !known {
        return Err(ProbeError::Protocol(format!(
            "{} does not look like MinIO cluster metrics: check the address and the port (9000, \
             the S3 API; not the console).",
            client.url(CLUSTER)
        )));
    }
    Ok(page)
}

#[async_trait]
impl Collector for MinioCollector {
    fn kind(&self) -> &'static str {
        "minio"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let client = client(target)?;
        let cluster = read_cluster(&client).await?;
        // Lecture facultative : une panne réelle compte, un refus ou une page
        // absente non.
        let node = match read(&client, NODE).await {
            Ok(page) => Some(page),
            Err(error) if error.means_down() => return Err(error),
            Err(error) => {
                tracing::debug!(%error, "MinIO : métriques du nœud indisponibles");
                None
            }
        };
        Ok(samples(&cluster, node.as_ref(), now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        read_cluster(&client(target)?).await?;
        Ok(Some("minio".to_string()))
    }
}

/// La première famille présente parmi ces noms (ancien et nouveau nom).
fn first(page: &Exposition, names: &[&str]) -> Option<f64> {
    names.iter().find_map(|name| page.sum(name))
}

/// Part utilisée, en pour cent, d'une capacité dont on connaît le libre.
fn used_percent(free: f64, total: f64) -> Option<f64> {
    (total > 0.0).then(|| ((total - free) / total * 100.0).clamp(0.0, 100.0))
}

pub fn samples(cluster: &Exposition, node: Option<&Exposition>, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();

    if let Some(line) = cluster.first("minio_software_version_info") {
        out.push(
            g("minio_version_info", 1.0)
                .with_label("version", line.label("version").unwrap_or(""))
                .with_label("commit", line.label("commit").unwrap_or("")),
        );
    }

    let online = cluster.sum("minio_cluster_nodes_online_total");
    let offline = cluster.sum("minio_cluster_nodes_offline_total");
    if let Some(value) = online {
        out.push(g("minio_nodes_online", value));
    }
    if let Some(value) = offline {
        out.push(g("minio_nodes_offline", value));
    }

    let drives_online =
        first(cluster, &["minio_cluster_drive_online_total", "minio_cluster_disk_online_total"]);
    let drives_offline =
        first(cluster, &["minio_cluster_drive_offline_total", "minio_cluster_disk_offline_total"]);
    let drives_total = first(cluster, &["minio_cluster_drive_total", "minio_cluster_disk_total"])
        .or_else(|| Some(drives_online? + drives_offline?));
    if let Some(value) = drives_online {
        out.push(g("minio_drives_online", value));
    }
    if let Some(value) = drives_offline {
        out.push(g("minio_drives_offline", value));
    }
    if let Some(value) = drives_total {
        out.push(g("minio_drives", value));
    }

    let raw_total = cluster.sum("minio_cluster_capacity_raw_total_bytes");
    let raw_free = cluster.sum("minio_cluster_capacity_raw_free_bytes");
    if let Some(total) = raw_total {
        out.push(g("minio_capacity_raw_bytes", total));
    }
    if let Some(free) = raw_free {
        out.push(g("minio_capacity_raw_free_bytes", free));
    }
    if let (Some(total), Some(free)) = (raw_total, raw_free)
        && let Some(used) = used_percent(free, total)
    {
        out.push(g("minio_capacity_raw_used_percent", used));
    }
    let usable_total = cluster.sum("minio_cluster_capacity_usable_total_bytes");
    let usable_free = cluster.sum("minio_cluster_capacity_usable_free_bytes");
    if let Some(total) = usable_total {
        out.push(g("minio_capacity_usable_bytes", total));
    }
    if let Some(free) = usable_free {
        out.push(g("minio_capacity_usable_free_bytes", free));
    }
    if let (Some(total), Some(free)) = (usable_total, usable_free)
        && let Some(used) = used_percent(free, total)
    {
        out.push(g("minio_capacity_usable_used_percent", used));
    }

    for (family, metric) in [
        ("minio_cluster_usage_total_bytes", "minio_usage_bytes"),
        ("minio_cluster_usage_object_total", "minio_objects"),
        ("minio_cluster_bucket_total", "minio_buckets"),
        ("minio_heal_objects_error_total", "minio_heal_objects_failed"),
    ] {
        if let Some(value) = cluster.sum(family) {
            out.push(g(metric, value));
        }
    }

    // Temps écoulé depuis la dernière activité de réparation, -1 tant que la
    // première n'est pas terminée.
    if let Some(nanos) = cluster.sum("minio_heal_time_last_activity_nano_seconds")
        && nanos >= 0.0
    {
        let idle = nanos / 1e9;
        out.push(g("minio_heal_idle_seconds", idle));
        out.push(flag("minio_heal_active", idle < HEAL_ACTIVE_SECONDS, ts_ms));
    }

    if let Some(failed) = first(
        cluster,
        &["minio_bucket_replication_failed_count", "minio_cluster_replication_failed_count"],
    ) {
        out.push(g("minio_replication_failed_objects", failed));
    }

    if let Some(total) = cluster.sum("minio_s3_requests_total") {
        out.push(counter("minio_s3_requests_total", total, ts_ms));
    }
    if let Some(errors) = cluster.sum("minio_s3_requests_4xx_errors_total") {
        out.push(counter("minio_s3_requests_4xx_total", errors, ts_ms));
    }
    if let Some(errors) = cluster.sum("minio_s3_requests_5xx_errors_total") {
        out.push(counter("minio_s3_requests_5xx_total", errors, ts_ms));
    }

    if let Some(uptime) = node.and_then(|page| page.sum("minio_node_process_uptime_seconds")) {
        out.push(g("minio_uptime_seconds", uptime));
    }
    out
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::routing::get;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Pages construites d'après la documentation « Metrics and Alerts » de
    // MinIO (famille v2) : quatre nœuds dont un hors ligne, un disque perdu,
    // une réparation récente avec des échecs, une réplication en échec.
    const CLUSTER_TXT: &str = include_str!("testdata/cluster.txt");
    const NODE_TXT: &str = include_str!("testdata/node.txt");

    #[test]
    fn grappe_et_noeud() {
        let s = samples(&Exposition::parse(CLUSTER_TXT), Some(&Exposition::parse(NODE_TXT)), 0);
        let info = find(&s, "minio_version_info", &[]).unwrap();
        assert_eq!(info.labels["version"], "RELEASE.2025-04-22T22-12-26Z");
        assert_eq!(value(&s, "minio_nodes_online", &[]), 3.0);
        assert_eq!(value(&s, "minio_nodes_offline", &[]), 1.0);
        assert_eq!(value(&s, "minio_drives_offline", &[]), 1.0);
        assert_eq!(value(&s, "minio_drives", &[]), 16.0);
        assert!((value(&s, "minio_capacity_raw_used_percent", &[]) - 60.0).abs() < 1e-6);
        assert!((value(&s, "minio_capacity_usable_used_percent", &[]) - 60.0).abs() < 1e-6);
        assert_eq!(value(&s, "minio_objects", &[]), 482_113.0);
        assert_eq!(value(&s, "minio_buckets", &[]), 12.0);
        assert_eq!(value(&s, "minio_heal_objects_failed", &[]), 3.0);
        assert_eq!(value(&s, "minio_heal_idle_seconds", &[]), 45.0);
        assert_eq!(value(&s, "minio_heal_active", &[]), 1.0);
        assert_eq!(value(&s, "minio_replication_failed_objects", &[]), 4.0);
        assert_eq!(value(&s, "minio_s3_requests_total", &[]), 10_000.0);
        assert_eq!(value(&s, "minio_s3_requests_5xx_total", &[]), 7.0);
        assert_eq!(value(&s, "minio_uptime_seconds", &[]), 86_400.0);
    }

    #[test]
    fn anciens_noms_de_disques_et_pas_de_reparation() {
        let page = Exposition::parse(
            "minio_cluster_disk_online_total 4\nminio_cluster_disk_offline_total 0\n\
             minio_heal_time_last_activity_nano_seconds -1\n",
        );
        let s = samples(&page, None, 0);
        assert_eq!(value(&s, "minio_drives_online", &[]), 4.0);
        assert_eq!(value(&s, "minio_drives", &[]), 4.0, "online + offline");
        assert!(find(&s, "minio_heal_active", &[]).is_none(), "not finished once yet");
        assert!(find(&s, "minio_uptime_seconds", &[]).is_none());
        assert!(find(&s, "minio_replication_failed_objects", &[]).is_none());
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let app = Router::new().fallback(get(|headers: HeaderMap, uri: Uri| async move {
            if headers.get("authorization").and_then(|v| v.to_str().ok()) != Some("Bearer jwt-ok") {
                return (StatusCode::FORBIDDEN, "<Error><Code>AccessDenied</Code></Error>");
            }
            match uri.path() {
                "/minio/v2/metrics/cluster" => (StatusCode::OK, CLUSTER_TXT),
                // Un serveur qui ne sert pas la page du nœud.
                _ => (StatusCode::NOT_FOUND, ""),
            }
        }));
        let address = serve(app).await;
        let mut target = cible("minio", &format!("http://{address}"), &[]);
        target.credential = Credential::ApiToken { token: "jwt-ok".into() };
        let s = MinioCollector::new().probe(&target).await.unwrap();
        assert_eq!(value(&s, "minio_nodes_offline", &[]), 1.0);
        assert!(find(&s, "minio_uptime_seconds", &[]).is_none());

        target.credential = Credential::ApiToken { token: "expired".into() };
        let error = MinioCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn une_autre_page_n_est_pas_minio() {
        let app = Router::new().fallback(get(|| async { "up 1\n" }));
        let address = serve(app).await;
        let target = cible("minio", &format!("http://{address}"), &[]);
        let error = MinioCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)), "{error}");
    }
}
