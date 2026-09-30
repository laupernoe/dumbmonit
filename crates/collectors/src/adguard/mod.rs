//! AdGuard Home, par son API REST (`/control`).
//!
//! Un filtre DNS en panne ne se remarque pas : les pages s'ouvrent, avec leurs
//! publicités. Ce module lit ce qu'AdGuard Home dit de lui-même :
//!
//! * la protection active, ou mise en pause (« désactiver pour une heure »)
//!   et jamais rétablie ;
//! * le serveur DNS en marche ;
//! * les requêtes et les blocages sur sa fenêtre de statistiques ;
//! * les listes de filtrage : combien, combien de règles, l'âge de la plus
//!   ancienne mise à jour — une liste qui ne se télécharge plus bloque de
//!   moins en moins ;
//! * la version, et la mise à jour disponible quand le contrôle de version
//!   n'est pas désactivé (il l'est dans l'image Docker officielle) ;
//! * chaque serveur DNS amont, testé par AdGuard Home lui-même (le bouton
//!   « Tester les serveurs amont ») : les statistiques ne comptent que les
//!   réponses, un amont muet n'y laisse aucune trace.
//!
//! # Appels
//!
//! Des `GET`, plus deux `POST` qui ne modifient rien : `version.json` avec
//! `recheck_now: false` (ce que l'interface appelle à chaque ouverture) et
//! `test_upstream_dns` (une requête DNS de test vers l'amont donné). Aucune
//! entrée du journal des requêtes n'est lue.
//!
//! AdGuard Home n'a pas de compte en lecture seule : tout utilisateur de
//! `AdGuardHome.yaml` peut tout faire. La documentation fait créer un
//! utilisateur dédié, pour pouvoir le retirer sans toucher au sien.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | `https` si l'interface a un certificat. |
//! | `port` | `80` | Port de l'interface web. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `5` | Délai par requête HTTP. |
//! | `upstream_check` | `true` | Fait tester chaque serveur amont. |

mod client;
pub mod metrics;
mod options;

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target, TargetId};
use serde_json::json;
use tracing::warn;

use client::{Client, Login};
use metrics::{DnsInfo, FilteringStatus, Stats, StatsConfig, Status, VersionCheck};
use options::Options;

pub use options::{DEFAULT_PORT, DEFAULT_REQUEST_TIMEOUT};

/// Délai laissé à AdGuard Home pour tester un amont. Un amont sain répond en
/// quelques dizaines de millisecondes ; un amont muet ferait attendre AdGuard
/// Home deux fois son `upstream_timeout` (20 s par défaut), bien au-delà du
/// délai de l'interrogation entière : passé ce délai, l'amont est compté
/// comme ne répondant pas.
pub const UPSTREAM_TEST_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Default)]
pub struct AdguardCollector;

impl AdguardCollector {
    pub fn new() -> Self {
        Self
    }
}

fn connect(target: &Target) -> Result<(Client, Options), ProbeError> {
    let options = Options::from_target(target)?;
    let login = Login::from_credential(&target.credential)?;
    let client = Client::new(
        crate::http::client(options.insecure_tls)?,
        options.base_url.clone(),
        login,
        options.request_timeout,
    );
    Ok((client, options))
}

fn settle<T>(
    result: Result<T, ProbeError>,
    errors: &mut u32,
    target_id: TargetId,
    what: &str,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            *errors += 1;
            warn!(target_id, %error, what, "lecture AdGuard Home en échec");
            None
        }
    }
}

/// Teste chaque amont séparément : un amont muet ne retarde pas les autres,
/// et le verdict se rattache sans ambiguïté à la ligne configurée.
async fn test_upstreams(
    client: &Client,
    info: &DnsInfo,
    errors: &mut u32,
    target_id: TargetId,
    ts_ms: i64,
) -> Vec<Sample> {
    let bootstrap = info.bootstrap_dns.clone().unwrap_or_default();
    let upstreams = info.upstreams();
    let tests = upstreams.iter().map(|upstream| {
        let body = json!({ "upstream_dns": [upstream], "bootstrap_dns": bootstrap });
        async move {
            client
                .post::<BTreeMap<String, String>, _>(
                    "/control/test_upstream_dns",
                    &body,
                    UPSTREAM_TEST_TIMEOUT,
                )
                .await
        }
    });
    let results = futures::future::join_all(tests).await;
    let mut out = Vec::new();
    for (upstream, result) in upstreams.iter().zip(results) {
        match result {
            // La clé est la forme normalisée de l'amont (`https://h:443/dns-query`,
            // `192.0.2.1:53`), celle que les statistiques emploient : elle sert
            // d'étiquette, pour que test et statistiques tombent sur la même ligne.
            Ok(verdicts) => match verdicts.iter().next() {
                Some((normalized, verdict)) => {
                    out.push(metrics::upstream_sample(normalized, Some(verdict), ts_ms));
                }
                None => out.push(metrics::upstream_sample(upstream, None, ts_ms)),
            },
            Err(ProbeError::Timeout(_)) => {
                out.push(metrics::upstream_sample(upstream, None, ts_ms));
            }
            Err(error) => {
                *errors += 1;
                warn!(target_id, %error, upstream, "test d'amont AdGuard Home en échec");
            }
        }
    }
    out
}

pub async fn probe(target: &Target, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    let (client, options) = connect(target)?;
    // La preuve de vie et d'authentification : le seul appel qui condamne.
    let status: Status = client.get("/control/status").await?;
    let mut out = metrics::status_samples(&status, ts_ms);
    let mut errors = 0u32;

    let no_recheck = json!({ "recheck_now": false });
    let upstream_phase = async {
        if !options.upstream_check {
            return (Ok(()), Vec::new(), 0);
        }
        let mut errors = 0u32;
        match client.get::<DnsInfo>("/control/dns_info").await {
            Ok(info) => {
                let samples = test_upstreams(&client, &info, &mut errors, target.id, ts_ms).await;
                (Ok(()), samples, errors)
            }
            Err(error) => (Err(error), Vec::new(), errors),
        }
    };
    let (stats, stats_config, filtering, version, (dns_info, upstreams, upstream_errors)) = futures::join!(
        client.get::<Stats>("/control/stats"),
        client.get::<StatsConfig>("/control/stats/config"),
        client.get::<FilteringStatus>("/control/filtering/status"),
        client.post::<VersionCheck, _>(
            "/control/version.json",
            &no_recheck,
            options.request_timeout
        ),
        upstream_phase,
    );

    // `stats/config` n'existe qu'à partir de 0.107.30 : son absence ne compte
    // pas comme une erreur.
    let stats_config = stats_config.ok();
    if let Some(stats) = settle(stats, &mut errors, target.id, "stats") {
        out.extend(metrics::stats_samples(&stats, stats_config.as_ref(), ts_ms));
    }
    if let Some(filtering) = settle(filtering, &mut errors, target.id, "filtering") {
        out.extend(metrics::filtering_samples(&filtering, chrono::Utc::now(), ts_ms));
    }
    if let Some(version) = settle(version, &mut errors, target.id, "version") {
        out.extend(metrics::version_samples(&version, &status.version, ts_ms));
    }
    settle(dns_info, &mut errors, target.id, "dns_info");
    errors += upstream_errors;
    out.extend(upstreams);

    out.push(Sample::new("adguard_scrape_errors", f64::from(errors), MetricKind::Gauge, ts_ms));
    Ok(out)
}

#[async_trait]
impl Collector for AdguardCollector {
    fn kind(&self) -> &'static str {
        "adguard"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(target, chrono::Utc::now().timestamp_millis()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let (client, _) = connect(target)?;
        let _: Status = client.get("/control/status").await?;
        Ok(Some("adguard".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::{get, post};
    use dumbmonit_proto::Credential;

    use super::*;

    fn target(address: String, credential: Credential, tags: &[(&str, &str)]) -> Target {
        Target {
            id: 9,
            name: "adguard".into(),
            address,
            kind: "adguard".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            credential,
        }
    }

    fn login(password: &str) -> Credential {
        Credential::UsernamePassword { username: "dumbmonit".into(), password: password.into() }
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    /// `dumbmonit:secret` en basic.
    fn authorized(headers: &HeaderMap) -> bool {
        headers.get("authorization").and_then(|v| v.to_str().ok())
            == Some("Basic ZHVtYm1vbml0OnNlY3JldA==")
    }

    fn fixture(name: &str) -> &'static str {
        match name {
            "status" => include_str!("testdata/v0.107.79/status.json"),
            "stats" => include_str!("testdata/v0.107.79/stats.json"),
            "filtering" => include_str!("testdata/v0.107.79/filtering_status.json"),
            "version" => include_str!("testdata/v0.107.79/version_update_from_v0.107.78.json"),
            "dns_info" => include_str!("testdata/v0.107.79/dns_info.json"),
            "upstream_ok" => include_str!("testdata/v0.107.79/test_upstream_ok.json"),
            "upstream_failed" => include_str!("testdata/v0.107.79/test_upstream_failed.json"),
            other => panic!("pas de fichier « {other} »"),
        }
    }

    fn app() -> Router {
        let json = |name: &'static str| {
            move |headers: HeaderMap| async move {
                if !authorized(&headers) {
                    return (StatusCode::UNAUTHORIZED, String::new());
                }
                (StatusCode::OK, fixture(name).to_string())
            }
        };
        Router::new()
            .route("/control/status", get(json("status")))
            .route("/control/stats", get(json("stats")))
            // Une version antérieure à 0.107.30 : pas de `stats/config`.
            .route("/control/filtering/status", get(json("filtering")))
            .route("/control/dns_info", get(|| async {
                r#"{"upstream_dns":["https://dns10.quad9.net/dns-query","192.0.2.1","10.0.0.9"],"bootstrap_dns":["9.9.9.10"]}"#
            }))
            .route(
                "/control/version.json",
                post(|body: String| async move {
                    assert!(body.contains("\"recheck_now\":false"), "jamais de nouveau contrôle");
                    fixture("version")
                }),
            )
            .route(
                "/control/test_upstream_dns",
                post(|body: String| async move {
                    if body.contains("192.0.2.1") {
                        fixture("upstream_failed").to_string()
                    } else if body.contains("10.0.0.9") {
                        // Un amont muet : AdGuard Home attendrait 20 s.
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        String::new()
                    } else {
                        assert!(body.contains("9.9.9.10"), "les amorces sont transmises");
                        fixture("upstream_ok").to_string()
                    }
                }),
            )
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let base = serve(app()).await;
        let samples =
            probe(&target(format!("{base}/control/"), login("secret"), &[]), 0).await.unwrap();
        assert_eq!(value(&samples, "adguard_protection_enabled"), Some(1.0));
        assert_eq!(value(&samples, "adguard_dns_queries"), Some(18.0));
        assert_eq!(value(&samples, "adguard_filters_enabled"), Some(1.0));
        assert!(value(&samples, "adguard_stats_window_seconds").is_none());
        // Le fichier de version vient d'une 0.107.78 ; le statut dit 0.107.79.
        assert_eq!(value(&samples, "adguard_update_available"), Some(0.0));
        let up = |name: &str| {
            samples
                .iter()
                .find(|s| s.metric == "adguard_upstream_up" && s.labels["upstream"] == name)
                .map(|s| s.value)
        };
        assert_eq!(up("https://dns10.quad9.net:443/dns-query"), Some(1.0));
        assert_eq!(up("192.0.2.1:53"), Some(0.0));
        assert_eq!(up("10.0.0.9"), Some(0.0), "un amont muet compte comme en panne");
        assert_eq!(value(&samples, "adguard_scrape_errors"), Some(0.0));
    }

    #[tokio::test]
    async fn sans_test_des_amonts() {
        let base = serve(app()).await;
        let samples =
            probe(&target(base, login("secret"), &[("upstream_check", "false")]), 0).await.unwrap();
        assert!(value(&samples, "adguard_upstream_up").is_none());
    }

    #[tokio::test]
    async fn un_mot_de_passe_refuse_n_est_pas_une_panne() {
        let base = serve(app()).await;
        let error = probe(&target(base.clone(), login("faux"), &[]), 0).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
        let error = AdguardCollector::new()
            .discover(&target(base, Credential::None, &[]))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }

    #[tokio::test]
    async fn un_serveur_eteint_est_injoignable() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let error = probe(&target(format!("http://{address}"), Credential::None, &[]), 0)
            .await
            .unwrap_err();
        assert!(error.means_down(), "{error:?}");
    }

    #[test]
    fn le_type_est_annonce() {
        assert_eq!(AdguardCollector::new().kind(), "adguard");
    }
}
