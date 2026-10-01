//! Kubernetes (et k3s, k0s, MicroK8s…) : l'état du cluster lu sur son API.
//!
//! Le collecteur lit, avec le jeton d'un compte de service limité à `get` et
//! `list`, huit collections : la version, les nœuds, les pods, les
//! déploiements, StatefulSets et DaemonSets, les volumes persistants, et les
//! événements d'avertissement (filtrés côté serveur, `type=Warning`). Rien
//! n'est jamais écrit, aucun secret ni aucune ConfigMap n'est lisible avec le
//! rôle documenté. Les listes longues sont lues par pages de 500.
//!
//! Le certificat de l'API est signé par l'autorité propre au cluster : on la
//! donne telle que `kubectl` l'imprime (le champ `ca.crt` du secret du jeton,
//! en base64), et seule elle est alors acceptée. À défaut, on peut renoncer à
//! la vérification.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `6443` | Port de l'API. |
//! | `ca_cert` | | Autorité du cluster, en base64 ou en PEM. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |
//! | `exclude_namespaces` | | Espaces de noms ignorés, séparés par des virgules. |

pub mod metrics;
pub mod model;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;

use crate::api_options::{Connection, parse_list, tag};
use metrics::Cluster;
use model::{Claim, Event, List, Node, Pod, Version, Workload};

pub const DEFAULT_PORT: u16 = 6443;
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Taille des pages de liste.
const PAGE_SIZE: usize = 500;
/// Pages lues au plus par collection : 10 000 objets, bien au-delà d'un homelab.
const MAX_PAGES: usize = 20;
/// Longueur maximale du corps d'erreur repris dans un message.
const MAX_ERROR_BODY: usize = 300;

#[derive(Default)]
pub struct KubernetesCollector;

impl KubernetesCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for KubernetesCollector {
    fn kind(&self) -> &'static str {
        "kubernetes"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let (client, excluded) = Client::from_target(target)?;
        let cluster = client.read().await?.without_namespaces(&excluded);
        Ok(metrics::samples(&cluster, chrono::Utc::now()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let (client, _) = Client::from_target(target)?;
        client.get::<Version>("/version").await?;
        // La version seule répond sans droit particulier : la lecture des nœuds
        // prouve que le rôle est bien en place.
        client.get::<List<Node>>("/api/v1/nodes?limit=1").await?;
        Ok(Some("kubernetes".to_string()))
    }
}

/// Accès à l'API : jeton porteur, pagination, et des erreurs qui disent quoi
/// corriger.
struct Client {
    http: reqwest::Client,
    base_url: String,
    token: String,
    timeout: Duration,
}

impl Client {
    fn from_target(target: &Target) -> Result<(Self, Vec<String>), ProbeError> {
        let connection = Connection::from_target(
            target,
            "https",
            DEFAULT_PORT,
            DEFAULT_PORT,
            DEFAULT_REQUEST_TIMEOUT,
        )?;
        let token = match &target.credential {
            Credential::ApiToken { token } if !token.trim().is_empty() => token.trim().to_string(),
            other => {
                return Err(ProbeError::Config(format!(
                    "Kubernetes expects the token of a service account, configured: {other}"
                )));
            }
        };
        let http = match (connection.insecure_tls, tag(target, "ca_cert")) {
            (false, Some(ca)) => client_trusting(&decode_ca(ca)?)?,
            (insecure, _) => crate::http::client(insecure)?,
        };
        let excluded = parse_list(tag(target, "exclude_namespaces"));
        Ok((
            Self {
                http,
                base_url: connection.base_url,
                token,
                timeout: connection.request_timeout,
            },
            excluded,
        ))
    }

    /// Lit tout le cluster, les huit collections en parallèle.
    async fn read(&self) -> Result<Cluster, ProbeError> {
        let (version, nodes, pods, deployments, statefulsets, daemonsets, claims, warnings) = tokio::try_join!(
            self.get::<Version>("/version"),
            self.list::<Node>("/api/v1/nodes"),
            self.list::<Pod>("/api/v1/pods"),
            self.list::<Workload>("/apis/apps/v1/deployments"),
            self.list::<Workload>("/apis/apps/v1/statefulsets"),
            self.list::<Workload>("/apis/apps/v1/daemonsets"),
            self.list::<Claim>("/api/v1/persistentvolumeclaims"),
            self.list::<Event>("/api/v1/events?fieldSelector=type%3DWarning"),
        )?;
        Ok(Cluster {
            version,
            nodes,
            pods,
            deployments,
            statefulsets,
            daemonsets,
            claims,
            warnings,
        })
    }

    /// Une collection entière, page après page.
    async fn list<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>, ProbeError> {
        let separator = if path.contains('?') { '&' } else { '?' };
        let mut items = Vec::new();
        let mut token: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let mut url = format!("{path}{separator}limit={PAGE_SIZE}");
            if let Some(next) = &token {
                url.push_str("&continue=");
                url.push_str(&urlencoding(next));
            }
            let page: List<T> = self.get(&url).await?;
            items.extend(page.items);
            token = page.metadata.continue_token.filter(|t| !t.is_empty());
            if token.is_none() {
                break;
            }
        }
        Ok(items)
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ProbeError> {
        let response = self
            .http
            .get(format!("{}{path}", self.base_url))
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|error| self.transport(&error, path))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| self.transport(&error, path))?;
        if !status.is_success() {
            return Err(status_error(status, &body, path, &self.base_url));
        }
        serde_json::from_str(&body).map_err(|error| {
            ProbeError::Protocol(format!("Unexpected response from {path}: {error}"))
        })
    }

    fn transport(&self, error: &reqwest::Error, path: &str) -> ProbeError {
        if error.is_timeout() {
            return ProbeError::Timeout(self.timeout);
        }
        let chain = error_chain(error);
        if chain.to_ascii_lowercase().contains("certificate") {
            return ProbeError::Config(format!(
                "The certificate of the API server could not be verified ({chain}). Paste the \
                 cluster CA in Cluster CA certificate, or tick Accept an unverifiable certificate."
            ));
        }
        ProbeError::Unreachable(format!("{path}: {chain}"))
    }
}

/// L'erreur et ses causes, sur une ligne : `reqwest` ne dit « certificat »
/// que dans la cause de la cause.
fn error_chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(cause) = source {
        parts.push(cause.to_string());
        source = cause.source();
    }
    parts.join(": ")
}

/// Un code d'erreur de l'API, traduit. Le corps d'un refus (`Status`) nomme
/// précisément la ressource et le verbe manquants : on le reprend.
fn status_error(status: StatusCode, body: &str, path: &str, base_url: &str) -> ProbeError {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_string))
        .unwrap_or_else(|| body.chars().take(MAX_ERROR_BODY).collect());
    let message = message.trim();
    match status {
        StatusCode::UNAUTHORIZED => ProbeError::Auth(
            "The API server refused the token (401). Copy the token of the dumbmonit service \
             account again: a token from kubectl create token expires."
                .to_string(),
        ),
        StatusCode::FORBIDDEN => ProbeError::Auth(format!(
            "The service account may not read {path} (403): {message}. Apply the ClusterRole \
             from the setup notice and its binding."
        )),
        StatusCode::NOT_FOUND => ProbeError::Protocol(format!(
            "{base_url} has no {path}: is this the address of a Kubernetes API server (port 6443)?"
        )),
        status if status.is_server_error() => ProbeError::Unreachable(format!(
            "The API server answered {status} on {path}: {message}"
        )),
        status => {
            ProbeError::Protocol(format!("The API server answered {status} on {path}: {message}"))
        }
    }
}

/// Encodage minimal d'un paramètre de requête : le jeton `continue` est du
/// base64, dont `+`, `/` et `=` doivent voyager encodés.
fn urlencoding(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// L'autorité du cluster, en PEM. `kubectl` l'imprime en base64 (le champ
/// `ca.crt` du secret) : c'est la forme la plus simple à copier. Un PEM collé
/// tel quel est accepté aussi, même aplati sur une ligne par le formulaire.
pub fn decode_ca(raw: &str) -> Result<Vec<u8>, ProbeError> {
    let raw = raw.trim();
    let pem = if raw.contains("-----BEGIN") {
        repair_pem(raw)
    } else {
        let compact: String = raw.split_whitespace().collect();
        let bytes = base64::engine::general_purpose::STANDARD.decode(compact).map_err(|_| {
            ProbeError::Config(
                "Cluster CA certificate: expected the base64 value of ca.crt, or a PEM certificate"
                    .to_string(),
            )
        })?;
        String::from_utf8_lossy(&bytes).into_owned()
    };
    if !pem.contains("-----BEGIN CERTIFICATE-----") {
        return Err(ProbeError::Config(
            "Cluster CA certificate: no PEM certificate found in the value".to_string(),
        ));
    }
    Ok(pem.into_bytes())
}

/// Remet les sauts de ligne d'un PEM collé sur une seule ligne.
fn repair_pem(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find("-----BEGIN CERTIFICATE-----") {
        let after = &rest[start + "-----BEGIN CERTIFICATE-----".len()..];
        let Some(end) = after.find("-----END CERTIFICATE-----") else { break };
        let body: String = after[..end].split_whitespace().collect();
        out.push_str("-----BEGIN CERTIFICATE-----\n");
        for chunk in body.as_bytes().chunks(64) {
            out.push_str(&String::from_utf8_lossy(chunk));
            out.push('\n');
        }
        out.push_str("-----END CERTIFICATE-----\n");
        rest = &after[end + "-----END CERTIFICATE-----".len()..];
    }
    out
}

/// Un client qui ne fait confiance qu'à l'autorité du cluster, gardé d'une
/// interrogation à l'autre : une poignée de main TLS par minute suffit.
fn client_trusting(pem: &[u8]) -> Result<reqwest::Client, ProbeError> {
    static CLIENTS: OnceLock<Mutex<HashMap<u64, reqwest::Client>>> = OnceLock::new();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pem.hash(&mut hasher);
    let key = hasher.finish();
    let clients = CLIENTS.get_or_init(Default::default);
    if let Some(client) = clients.lock().ok().and_then(|map| map.get(&key).cloned()) {
        return Ok(client);
    }
    let certificates = reqwest::Certificate::from_pem_bundle(pem).map_err(|error| {
        ProbeError::Config(format!("Cluster CA certificate is not a valid certificate: {error}"))
    })?;
    if certificates.is_empty() {
        return Err(ProbeError::Config("Cluster CA certificate is empty".to_string()));
    }
    let client = reqwest::Client::builder()
        .tls_certs_only(certificates)
        .connect_timeout(crate::http::CONNECT_TIMEOUT)
        .pool_max_idle_per_host(2)
        .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| ProbeError::Config(format!("HTTP client unavailable: {error}")))?;
    if let Ok(mut map) = clients.lock() {
        map.insert(key, client.clone());
    }
    Ok(client)
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::extract::RawQuery;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;
    use chrono::{DateTime, Utc};

    use super::metrics::{Cluster, parse_list, samples};
    use super::*;
    use crate::api_options::test_support::target;

    // Réponses réelles d'un k3s v1.33.4 (un nœud, un déploiement sain, un
    // déploiement dont le conteneur sort en erreur, un pod impossible à placer,
    // un volume jamais provisionné), lues avec le jeton du compte `dumbmonit`
    // et le rôle de la notice, puis anonymisées.
    const VERSION: &str = include_str!("testdata/version.json");
    const NODES: &str = include_str!("testdata/nodes.json");
    const PODS: &str = include_str!("testdata/pods.json");
    const DEPLOYMENTS: &str = include_str!("testdata/deployments.json");
    const STATEFULSETS: &str = include_str!("testdata/statefulsets.json");
    const DAEMONSETS: &str = include_str!("testdata/daemonsets.json");
    const PVCS: &str = include_str!("testdata/pvcs.json");
    const EVENTS: &str = include_str!("testdata/events_warning.json");
    const FORBIDDEN: &str = include_str!("testdata/forbidden.json");
    const UNAUTHORIZED: &str = include_str!("testdata/unauth.json");

    /// Le moment de la capture, pour que la fenêtre d'une heure des
    /// événements soit reproductible.
    fn captured_at() -> DateTime<Utc> {
        "2026-09-30T22:47:00Z".parse().unwrap()
    }

    fn cluster() -> Cluster {
        Cluster {
            version: serde_json::from_str(VERSION).unwrap(),
            nodes: parse_list(NODES).unwrap().items,
            pods: parse_list(PODS).unwrap().items,
            deployments: parse_list(DEPLOYMENTS).unwrap().items,
            statefulsets: parse_list(STATEFULSETS).unwrap().items,
            daemonsets: parse_list(DAEMONSETS).unwrap().items,
            claims: parse_list(PVCS).unwrap().items,
            warnings: parse_list(EVENTS).unwrap().items,
        }
    }

    fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == name
                && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(*v))
        })
    }

    fn value(samples: &[Sample], name: &str, labels: &[(&str, &str)]) -> f64 {
        find(samples, name, labels).unwrap_or_else(|| panic!("{name} {labels:?} absent")).value
    }

    #[test]
    fn le_cluster_de_reference_se_lit_entierement() {
        let s = samples(&cluster(), captured_at());
        assert_eq!(value(&s, "k8s_version_info", &[("version", "1.33.4+k3s1")]), 1.0);
        assert_eq!(value(&s, "k8s_nodes", &[]), 1.0);
        assert_eq!(value(&s, "k8s_nodes_ready", &[]), 1.0);
        assert_eq!(value(&s, "k8s_node_ready", &[("node", "node-1")]), 1.0);
        assert_eq!(
            value(&s, "k8s_node_pressure", &[("node", "node-1"), ("condition", "disk")]),
            0.0
        );
        assert!(
            find(&s, "k8s_node_pressure", &[("condition", "network")]).is_none(),
            "k3s ne déclare pas NetworkUnavailable : pas de série inventée"
        );
        assert_eq!(value(&s, "k8s_node_unschedulable", &[("node", "node-1")]), 0.0);
        assert_eq!(value(&s, "k8s_pods", &[]), 8.0);
        assert_eq!(value(&s, "k8s_pods_pending", &[]), 1.0);
        assert_eq!(value(&s, "k8s_pods_crashlooping", &[]), 1.0);
    }

    #[test]
    fn le_pod_en_boucle_de_redemarrage_est_rattache_a_son_deploiement() {
        let s = samples(&cluster(), captured_at());
        let api = [("namespace", "demo"), ("pod", "api-7c9696d669-8tpp6")];
        assert_eq!(value(&s, "k8s_pod_crashlooping", &api), 1.0);
        assert_eq!(value(&s, "k8s_pod_ready", &api), 0.0);
        let restarts = find(&s, "k8s_pod_restarts", &api).unwrap();
        assert_eq!(restarts.value, 5.0);
        assert_eq!(restarts.kind, dumbmonit_proto::MetricKind::Counter);
        assert_eq!(restarts.labels["workload"], "api");
        let web = [("pod", "web-5bd6c6ddd5-jc7wv")];
        assert_eq!(value(&s, "k8s_pod_crashlooping", &web), 0.0);
        assert_eq!(find(&s, "k8s_pod_ready", &web).unwrap().labels["workload"], "web");
        assert_eq!(value(&s, "k8s_pod_pending", &[("pod", "big")]), 1.0);
        assert_eq!(find(&s, "k8s_pod_pending", &[("pod", "big")]).unwrap().labels["workload"], "");
        assert_eq!(find(&s, "k8s_pod_ready", &[("pod", "db-0")]).unwrap().labels["workload"], "db");
    }

    #[test]
    fn les_charges_de_travail_disent_ce_qui_leur_manque() {
        let s = samples(&cluster(), captured_at());
        let api = [("kind", "Deployment"), ("workload", "api")];
        assert_eq!(value(&s, "k8s_workload_desired", &api), 1.0);
        assert_eq!(value(&s, "k8s_workload_ready", &api), 0.0, "readyReplicas omis vaut zéro");
        assert_eq!(value(&s, "k8s_workload_unavailable", &api), 1.0);
        assert_eq!(value(&s, "k8s_workload_unavailable", &[("workload", "web")]), 0.0);
        assert_eq!(value(&s, "k8s_workload_ready", &[("workload", "web")]), 2.0);
        let db = [("kind", "StatefulSet"), ("workload", "db")];
        assert_eq!(value(&s, "k8s_workload_unavailable", &db), 0.0);
        let agent = [("kind", "DaemonSet"), ("workload", "agent")];
        assert_eq!(value(&s, "k8s_workload_desired", &agent), 1.0);
        assert_eq!(value(&s, "k8s_workload_unavailable", &agent), 0.0);
    }

    #[test]
    fn un_volume_en_attente_compte_quand_un_avertissement_le_concerne() {
        let s = samples(&cluster(), captured_at());
        assert_eq!(value(&s, "k8s_pvc_pending", &[("namespace", "demo"), ("pvc", "data")]), 1.0);
        assert_eq!(value(&s, "k8s_pvcs_pending", &[]), 1.0);

        // Sans l'avertissement ni pod qui le réclame, c'est une classe
        // WaitForFirstConsumer qui attend son premier pod : rien d'anormal.
        let mut quiet = cluster();
        quiet.warnings.retain(|e| e.involved_object.kind != "PersistentVolumeClaim");
        let s = samples(&quiet, captured_at());
        assert_eq!(value(&s, "k8s_pvc_pending", &[("pvc", "data")]), 0.0);
    }

    #[test]
    fn les_avertissements_de_la_derniere_heure_sont_comptes_par_raison() {
        let s = samples(&cluster(), captured_at());
        assert_eq!(value(&s, "k8s_warning_events", &[]), 9.0);
        assert_eq!(value(&s, "k8s_warning_events_by_reason", &[("reason", "SystemOOM")]), 5.0);
        assert_eq!(
            value(&s, "k8s_warning_events_by_reason", &[("reason", "FailedScheduling")]),
            1.0,
            "l'ordonnanceur date par eventTime, sans lastTimestamp"
        );
        let later = captured_at() + chrono::Duration::hours(2);
        assert_eq!(value(&samples(&cluster(), later), "k8s_warning_events", &[]), 0.0);
    }

    #[test]
    fn les_espaces_de_noms_exclus_disparaissent_des_series() {
        let s = samples(&cluster().without_namespaces(&["demo".into()]), captured_at());
        assert_eq!(value(&s, "k8s_pods", &[]), 2.0);
        assert_eq!(value(&s, "k8s_pods_crashlooping", &[]), 0.0);
        assert!(find(&s, "k8s_pvc_pending", &[]).is_none());
        assert_eq!(value(&s, "k8s_warning_events", &[]), 6.0, "les événements de nœud restent");
    }

    #[test]
    fn l_autorite_se_colle_en_base64_ou_en_pem() {
        let pem = "-----BEGIN CERTIFICATE-----\nMIIBdzCCAR2gAwIBAgIBADAKBggqhkjOPQQDAjAjMSEwHwYDVQQDDBhrM3Mtc2Vy\ndmVyLWNhQDE3NTkyNzI1OTg=\n-----END CERTIFICATE-----\n";
        let b64 = base64::engine::general_purpose::STANDARD.encode(pem);
        assert_eq!(decode_ca(&b64).unwrap(), pem.as_bytes());
        let flattened = pem.replace('\n', " ");
        assert_eq!(decode_ca(&flattened).unwrap(), pem.as_bytes());
        assert!(matches!(decode_ca("pas du base64 !"), Err(ProbeError::Config(_))));
        let not_pem = base64::engine::general_purpose::STANDARD.encode("hello");
        assert!(matches!(decode_ca(&not_pem), Err(ProbeError::Config(_))));
    }

    #[test]
    fn les_refus_de_l_api_ne_passent_pas_pour_une_panne() {
        let forbidden = status_error(StatusCode::FORBIDDEN, FORBIDDEN, "/api/v1/secrets", "x");
        assert!(matches!(&forbidden, ProbeError::Auth(m) if m.contains("cannot list resource")));
        assert!(!forbidden.means_down());
        let refused = status_error(StatusCode::UNAUTHORIZED, UNAUTHORIZED, "/api/v1/nodes", "x");
        assert!(matches!(refused, ProbeError::Auth(_)));
        assert!(status_error(StatusCode::SERVICE_UNAVAILABLE, "", "/", "x").means_down());
        assert!(matches!(
            status_error(StatusCode::NOT_FOUND, "", "/", "x"),
            ProbeError::Protocol(_)
        ));
    }

    #[tokio::test]
    async fn un_jeton_est_obligatoire() {
        let t = target("kubernetes", "k3s.lan", &[], Credential::None);
        let error = KubernetesCollector::new().probe(&t).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn reply(headers: &HeaderMap, body: &'static str) -> (StatusCode, &'static str) {
        if headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer jeton") {
            (StatusCode::OK, body)
        } else {
            (StatusCode::UNAUTHORIZED, UNAUTHORIZED)
        }
    }

    #[tokio::test]
    async fn interrogation_complete_contre_une_fausse_api() {
        // Les pods arrivent en deux pages, comme avec `limit` sur un vrai
        // serveur : la seconde n'est servie qu'avec le bon jeton `continue`.
        let first_page = {
            let mut list: serde_json::Value = serde_json::from_str(PODS).unwrap();
            let items = list["items"].as_array().unwrap().clone();
            list["items"] = serde_json::Value::Array(items[..3].to_vec());
            list["metadata"]["continue"] = "eyJ2IjoibWV0YS5rOHMuaW8vdjEifQ==".into();
            Box::leak(list.to_string().into_boxed_str()) as &'static str
        };
        let second_page = {
            let mut list: serde_json::Value = serde_json::from_str(PODS).unwrap();
            let items = list["items"].as_array().unwrap().clone();
            list["items"] = serde_json::Value::Array(items[3..].to_vec());
            Box::leak(list.to_string().into_boxed_str()) as &'static str
        };
        let app = Router::new()
            .route("/version", get(|h: HeaderMap| async move { reply(&h, VERSION) }))
            .route("/api/v1/nodes", get(|h: HeaderMap| async move { reply(&h, NODES) }))
            .route(
                "/api/v1/pods",
                get(move |h: HeaderMap, RawQuery(q): RawQuery| async move {
                    let q = q.unwrap_or_default();
                    assert!(q.contains("limit=500"), "{q}");
                    if q.contains("continue=eyJ2IjoibWV0YS5rOHMuaW8vdjEifQ%3D%3D") {
                        reply(&h, second_page)
                    } else {
                        reply(&h, first_page)
                    }
                }),
            )
            .route(
                "/apis/apps/v1/deployments",
                get(|h: HeaderMap| async move { reply(&h, DEPLOYMENTS) }),
            )
            .route(
                "/apis/apps/v1/statefulsets",
                get(|h: HeaderMap| async move { reply(&h, STATEFULSETS) }),
            )
            .route(
                "/apis/apps/v1/daemonsets",
                get(|h: HeaderMap| async move { reply(&h, DAEMONSETS) }),
            )
            .route(
                "/api/v1/persistentvolumeclaims",
                get(|h: HeaderMap| async move { reply(&h, PVCS) }),
            )
            .route(
                "/api/v1/events",
                get(|h: HeaderMap, RawQuery(q): RawQuery| async move {
                    assert!(q.unwrap_or_default().contains("fieldSelector=type%3DWarning"));
                    reply(&h, EVENTS)
                }),
            );
        let base = serve(app).await;
        let token = Credential::ApiToken { token: "jeton".into() };
        let t = target("kubernetes", &base, &[], token);
        let s = KubernetesCollector::new().probe(&t).await.unwrap();
        assert_eq!(value(&s, "k8s_pods", &[]), 8.0, "les deux pages sont lues");
        assert_eq!(value(&s, "k8s_workload_unavailable", &[("workload", "api")]), 1.0);
        assert_eq!(
            KubernetesCollector::new().discover(&t).await.unwrap().as_deref(),
            Some("kubernetes")
        );

        let wrong = target("kubernetes", &base, &[], Credential::ApiToken { token: "faux".into() });
        let error = KubernetesCollector::new().probe(&wrong).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn une_api_eteinte_est_une_panne() {
        let t = target(
            "kubernetes",
            "http://127.0.0.1:9",
            &[],
            Credential::ApiToken { token: "jeton".into() },
        );
        let error = KubernetesCollector::new().probe(&t).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    /// Contre un vrai cluster : `DUMBMONIT_TEST_K8S=https://hôte:6443`,
    /// `DUMBMONIT_TEST_K8S_TOKEN` et `DUMBMONIT_TEST_K8S_CA` (base64 de ca.crt).
    #[tokio::test]
    #[ignore = "demande un cluster Kubernetes joignable"]
    async fn cluster_reel_joignable() {
        let address = std::env::var("DUMBMONIT_TEST_K8S").unwrap();
        let token = std::env::var("DUMBMONIT_TEST_K8S_TOKEN").unwrap();
        let ca = std::env::var("DUMBMONIT_TEST_K8S_CA").unwrap_or_default();
        let tags: Vec<(&str, &str)> = if ca.is_empty() {
            vec![("insecure_tls", "true")]
        } else {
            vec![("ca_cert", ca.as_str())]
        };
        let t = target("kubernetes", &address, &tags, Credential::ApiToken { token });
        let collector = KubernetesCollector::new();
        assert_eq!(collector.discover(&t).await.unwrap().as_deref(), Some("kubernetes"));
        for sample in collector.probe(&t).await.unwrap() {
            println!("{} {:?} {}", sample.metric, sample.labels, sample.value);
        }
    }
}
