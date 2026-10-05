//! MikroTik RouterOS, par son API REST (`/rest`, RouterOS 7.1 et suivants).
//!
//! Un routeur MikroTik tombe rarement ; il vieillit. Une version qui n'est plus
//! la dernière de sa branche, un firmware RouterBOOT resté derrière le système
//! après une mise à jour (il ne suit qu'au redémarrage suivant, et seulement
//! si on le lui demande), une alimentation redondante morte que personne n'a
//! vue, un port qui accumule les erreurs ou qui tombe et remonte. C'est ce que
//! ce module lit, avec la charge, la mémoire et le stockage.
//!
//! # Appels
//!
//! Uniquement des `GET` : `/system/resource` (le seul qui condamne
//! l'interrogation), `/system/identity`, `/system/package/update`,
//! `/system/health`, `/system/routerboard` et `/interface`. Jamais
//! `check-for-updates` : un compte en lecture n'a pas le droit de la lancer
//! (vérifié sur 7.23.7, « not enough permissions »), et la version la plus
//! récente n'est donc connue que si le routeur vérifie lui-même.
//!
//! # Compte
//!
//! Un utilisateur dans un groupe aux politiques `read,api,rest-api` et rien
//! d'autre : `rest-api` ouvre l'API REST, `api` et `read` sont exigés en plus
//! pour chaque lecture ; sans `write`, toute modification est refusée.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `https` | `http` pour le service `www`, en clair. |
//! | `port` | `443` (`80` en `http`) | Port du service web. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |
//! | `interfaces` | `true` | Lit l'état et les compteurs des interfaces. |
//! | `max_interfaces` | `64` | Interfaces lues au plus. |

mod client;
pub mod metrics;
mod options;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target, TargetId};
use serde_json::Value;
use tracing::warn;

use client::{Client, Failure, Login};
use metrics::Record;
use options::Options;

pub use options::{DEFAULT_MAX_INTERFACES, DEFAULT_REQUEST_TIMEOUT};

#[derive(Default)]
pub struct MikrotikCollector;

impl MikrotikCollector {
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

/// Une lecture secondaire : son échec est journalisé et compté, il ne fait
/// pas échouer l'interrogation.
fn settle<T>(
    result: Result<T, Failure>,
    errors: &mut u32,
    target_id: TargetId,
    path: &str,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(failure) => {
            *errors += 1;
            let error = failure.into_error(path);
            warn!(target_id, %error, path, "lecture RouterOS en échec");
            None
        }
    }
}

pub async fn probe(target: &Target, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    let (client, options) = connect(target)?;
    let resource: Record =
        client.get("/system/resource").await.map_err(|f| f.into_error("/system/resource"))?;
    let mut errors = 0u32;

    let interface_path = format!("/interface?.proplist={}", metrics::INTERFACE_FIELDS);
    let interfaces = async {
        if options.interfaces {
            Some(client.get::<Vec<Record>>(&interface_path).await)
        } else {
            None
        }
    };
    let (identity, update, health, board, interfaces) = futures::join!(
        client.get::<Record>("/system/identity"),
        client.get::<Record>("/system/package/update"),
        client.get::<Value>("/system/health"),
        client.get::<Record>("/system/routerboard"),
        interfaces,
    );

    let identity = settle(identity, &mut errors, target.id, "/system/identity");
    let name = identity.as_ref().and_then(|i| metrics::text(i, "name"));
    let mut out = metrics::resource_samples(&resource, name, ts_ms);
    if let Some(update) = settle(update, &mut errors, target.id, "/system/package/update") {
        out.extend(metrics::update_samples(&update, ts_ms));
    }
    match health {
        Ok(health) => out.extend(metrics::health_samples(&health, ts_ms)),
        // Un menu absent n'a rien d'anormal : une machine x86 n'a pas de capteur.
        Err(Failure::Missing) => {}
        Err(failure) => {
            settle::<()>(Err(failure), &mut errors, target.id, "/system/health");
        }
    }
    match board {
        Ok(board) => out.extend(metrics::routerboard_samples(Some(&board), ts_ms)),
        // Un CHR ou une machine x86 : « no such command or directory ».
        Err(Failure::Missing) => out.extend(metrics::routerboard_samples(None, ts_ms)),
        Err(failure) => {
            settle::<()>(Err(failure), &mut errors, target.id, "/system/routerboard");
        }
    }
    if let Some(interfaces) = interfaces
        && let Some(list) = settle(interfaces, &mut errors, target.id, "/interface")
    {
        out.extend(metrics::interface_samples(&list, options.max_interfaces, ts_ms));
    }
    out.push(Sample::new("mikrotik_scrape_errors", f64::from(errors), MetricKind::Gauge, ts_ms));
    Ok(out)
}

#[async_trait]
impl Collector for MikrotikCollector {
    fn kind(&self) -> &'static str {
        "mikrotik"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(target, chrono::Utc::now().timestamp_millis()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let (client, _) = connect(target)?;
        let _: Record =
            client.get("/system/resource").await.map_err(|f| f.into_error("/system/resource"))?;
        Ok(Some("mikrotik".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use axum::Router;
    use axum::extract::RawQuery;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;
    use dumbmonit_proto::Credential;

    use super::*;

    /// `dumbmonit:Str0ng-read-only` en basic.
    const AUTH: &str = "Basic ZHVtYm1vbml0OlN0cjBuZy1yZWFkLW9ubHk=";

    fn target(address: String, password: &str) -> Target {
        Target {
            id: 9,
            name: "edge-router".into(),
            address,
            kind: "mikrotik".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential: Credential::UsernamePassword {
                username: "dumbmonit".into(),
                password: password.into(),
            },
            group_name: String::new(),
            position: 0,
        }
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn authorized(headers: &HeaderMap) -> bool {
        headers.get("authorization").and_then(|v| v.to_str().ok()) == Some(AUTH)
    }

    /// Un CHR 7.23.7 tel que capturé, qui refuse tout autre mot de passe comme
    /// le fait RouterOS (401, corps JSON).
    fn chr(update_down: bool) -> Router {
        fn reply(
            headers: &HeaderMap,
            body: &'static str,
        ) -> (StatusCode, [(&'static str, &'static str); 1], &'static str) {
            let json = [("content-type", "application/json")];
            if !authorized(headers) {
                return (
                    StatusCode::UNAUTHORIZED,
                    json,
                    r#"{"error":401,"message":"Unauthorized"}"#,
                );
            }
            (StatusCode::OK, json, body)
        }
        Router::new()
            .route(
                "/rest/system/resource",
                get(|h: HeaderMap| async move {
                    reply(&h, include_str!("testdata/chr_7.23.7/system_resource.json"))
                }),
            )
            .route(
                "/rest/system/identity",
                get(|h: HeaderMap| async move {
                    reply(&h, include_str!("testdata/chr_7.23.7/system_identity.json"))
                }),
            )
            .route(
                "/rest/system/package/update",
                get(move |h: HeaderMap| async move {
                    if update_down {
                        return (
                            StatusCode::SERVICE_UNAVAILABLE,
                            [("content-type", "text/plain")],
                            "",
                        );
                    }
                    reply(&h, include_str!("testdata/chr_7.23.7/update_available.json"))
                }),
            )
            .route(
                "/rest/system/health",
                get(|h: HeaderMap| async move {
                    reply(&h, include_str!("testdata/chr_7.23.7/system_health.json"))
                }),
            )
            .route(
                "/rest/system/routerboard",
                get(|| async {
                    (
                        StatusCode::BAD_REQUEST,
                        include_str!("testdata/chr_7.23.7/system_routerboard_absent.json"),
                    )
                }),
            )
            .route(
                "/rest/interface",
                get(|h: HeaderMap, RawQuery(query): RawQuery| async move {
                    // Seuls les champs utiles sont demandés.
                    assert!(query.unwrap_or_default().starts_with(".proplist=name,type,running"));
                    reply(&h, include_str!("testdata/chr_7.23.7/interface.json"))
                }),
            )
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[tokio::test]
    async fn interrogation_complete_d_un_chr() {
        let base = serve(chr(false)).await;
        let samples = MikrotikCollector::new()
            .probe(&target(base.clone(), "Str0ng-read-only"))
            .await
            .unwrap();
        assert_eq!(value(&samples, "mikrotik_scrape_errors"), Some(0.0));
        assert_eq!(value(&samples, "mikrotik_routerboard"), Some(0.0));
        assert_eq!(value(&samples, "mikrotik_update_available"), Some(1.0));
        assert!(value(&samples, "mikrotik_cpu_load_percent").is_some());
        assert!(samples.iter().any(|s| s.metric == "mikrotik_interface_rx_bytes_total"));
        let info = samples.iter().find(|s| s.metric == "mikrotik_info").unwrap();
        assert_eq!(info.labels["identity"], "edge-router");
        // Le socle ajoute `up` et les étiquettes : pas le collecteur.
        assert!(value(&samples, "up").is_none());

        let found = MikrotikCollector::new().discover(&target(base, "Str0ng-read-only")).await;
        assert_eq!(found.unwrap().as_deref(), Some("mikrotik"));
    }

    #[tokio::test]
    async fn un_mot_de_passe_refuse_n_est_pas_une_panne() {
        let base = serve(chr(false)).await;
        let error = MikrotikCollector::new().probe(&target(base, "faux")).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(ref m) if m.contains("rest-api")));
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn un_groupe_sans_la_politique_read_est_explique() {
        let app = Router::new().route(
            "/rest/system/resource",
            get(|| async {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    r#"{"detail":"not enough permissions (9)","error":500,"message":"Internal Server Error"}"#,
                )
            }),
        );
        let base = serve(app).await;
        let error =
            MikrotikCollector::new().probe(&target(base, "Str0ng-read-only")).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(ref m) if m.contains("read,api,rest-api")));
    }

    #[tokio::test]
    async fn un_menu_secondaire_en_echec_est_compte_sans_faire_tomber_la_collecte() {
        let base = serve(chr(true)).await;
        let samples =
            MikrotikCollector::new().probe(&target(base, "Str0ng-read-only")).await.unwrap();
        assert_eq!(value(&samples, "mikrotik_scrape_errors"), Some(1.0));
        assert!(value(&samples, "mikrotik_update_checked").is_none());
    }

    #[tokio::test]
    async fn un_routeur_eteint_est_injoignable() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let error = MikrotikCollector::new()
            .probe(&target(format!("http://{address}"), "x"))
            .await
            .unwrap_err();
        assert!(error.means_down(), "{error:?}");
    }

    /// Contre un vrai routeur, à la demande :
    /// `DUMBMONIT_MIKROTIK_URL=https://… DUMBMONIT_MIKROTIK_USER=… DUMBMONIT_MIKROTIK_PASSWORD=…
    /// cargo test -p dumbmonit-collectors mikrotik -- --ignored`.
    #[tokio::test]
    #[ignore = "demande un routeur RouterOS joignable"]
    async fn sonde_d_un_vrai_routeur() {
        let (Ok(url), Ok(user), Ok(password)) = (
            std::env::var("DUMBMONIT_MIKROTIK_URL"),
            std::env::var("DUMBMONIT_MIKROTIK_USER"),
            std::env::var("DUMBMONIT_MIKROTIK_PASSWORD"),
        ) else {
            panic!("DUMBMONIT_MIKROTIK_URL, _USER et _PASSWORD sont requises");
        };
        let mut t = target(url, &password);
        t.credential = Credential::UsernamePassword { username: user, password };
        t.tags.insert("insecure_tls".into(), "true".into());
        let samples = MikrotikCollector::new().probe(&t).await.unwrap();
        for s in &samples {
            println!("{} {:?} = {}", s.metric, s.labels, s.value);
        }
        assert_eq!(value(&samples, "mikrotik_scrape_errors"), Some(0.0));
    }

    #[test]
    fn sans_identifiant_la_cible_est_mal_configuree() {
        let mut t = target("router.lan".into(), "x");
        t.credential = Credential::None;
        assert!(matches!(connect(&t), Err(ProbeError::Config(_))));
    }
}
