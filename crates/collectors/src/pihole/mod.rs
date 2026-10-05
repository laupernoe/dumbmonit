//! Pi-hole v6, par son API REST (`/api`).
//!
//! Ce qu'un Pi-hole fait mal se voit rarement : le DNS continue de répondre.
//! Le blocage suspendu « cinq minutes » et jamais rétabli (la minuterie a été
//! annulée, ou mise à zéro), une liste gravity qui ne se reconstruit plus
//! depuis des semaines, un résolveur amont qui rend des SERVFAIL, une mise à
//! jour qui attend, et le « Pi-hole diagnosis » que personne n'ouvre : voilà
//! ce que ce module lit.
//!
//! # Session
//!
//! L'API demande une session : `POST /api/auth` avec le mot de passe
//! d'application, puis l'identifiant rendu dans `X-FTL-SID`. Pi-hole limite le
//! nombre de sessions simultanées (16 par défaut) et garde chacune 30 minutes
//! après son dernier usage : en ouvrir une par interrogation les épuiserait en
//! un quart d'heure et fermerait l'interface à son propriétaire. La session
//! est donc gardée d'une interrogation à l'autre, par cible, et rouverte
//! seulement quand Pi-hole ne la reconnaît plus (401).
//!
//! Pi-hole n'a pas de compte en lecture seule. Le mot de passe d'application
//! est ce qui s'en approche : avec `webserver.api.app_sudo` à `false` (le
//! défaut), sa session ne peut modifier aucun réglage — elle peut en revanche
//! suspendre le blocage. Ce module ne fait que des `GET`, hors l'ouverture et
//! la fermeture de session.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | `https` pour le port 443, au certificat auto-signé. |
//! | `port` | `80` | Port de l'interface web. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |

mod client;
pub mod metrics;
mod options;

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Mutex;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target, TargetId};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use tracing::warn;

use client::{Client, Reply};
use metrics::{Blocking, FtlReply, Messages, Summary, VersionReply};
use options::Options;

pub use options::{DEFAULT_PORT, DEFAULT_REQUEST_TIMEOUT};

/// Une session ouverte, et ce qui l'a ouverte : une adresse ou un mot de passe
/// changé sur la cible impose d'en ouvrir une autre.
struct Session {
    fingerprint: u64,
    /// `None` : ce Pi-hole n'a pas de mot de passe.
    sid: Option<String>,
}

#[derive(Default)]
pub struct PiholeCollector {
    sessions: Mutex<HashMap<TargetId, Session>>,
}

impl PiholeCollector {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Le mot de passe d'application, ou rien pour un Pi-hole sans mot de passe.
fn password(credential: &Credential) -> Result<Option<String>, ProbeError> {
    match credential {
        Credential::None => Ok(None),
        Credential::ApiToken { token } if !token.trim().is_empty() => {
            Ok(Some(token.trim().to_string()))
        }
        other => Err(ProbeError::Config(format!(
            "Pi-hole expects its app password, or no credential when it has no password; \
             configured: {other}"
        ))),
    }
}

fn fingerprint(base_url: &str, password: Option<&str>) -> u64 {
    let mut hasher = DefaultHasher::new();
    base_url.hash(&mut hasher);
    password.hash(&mut hasher);
    hasher.finish()
}

/// Ce qu'il faut pour parler à une cible pendant une interrogation.
struct Probe<'a> {
    collector: &'a PiholeCollector,
    client: Client,
    target_id: TargetId,
    password: Option<String>,
    fingerprint: u64,
}

impl Probe<'_> {
    fn cached_sid(&self) -> Option<Option<String>> {
        let sessions = self.collector.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions
            .get(&self.target_id)
            .filter(|session| session.fingerprint == self.fingerprint)
            .map(|session| session.sid.clone())
    }

    /// Une session ouverte avec un autre mot de passe ou une autre adresse est
    /// simplement remplacée : elle expirera d'elle-même côté Pi-hole.
    fn remember(&self, sid: Option<String>) -> Option<String> {
        let mut sessions = self.collector.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions
            .insert(self.target_id, Session { fingerprint: self.fingerprint, sid: sid.clone() });
        sid
    }

    fn forget(&self) {
        let mut sessions = self.collector.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.remove(&self.target_id);
    }

    async fn open(&self) -> Result<Option<String>, ProbeError> {
        match &self.password {
            None => Ok(self.remember(None)),
            Some(password) => {
                let sid = self.client.login(password).await?;
                Ok(self.remember(sid))
            }
        }
    }

    async fn sid(&self) -> Result<Option<String>, ProbeError> {
        match self.cached_sid() {
            Some(sid) => Ok(sid),
            None => self.open().await,
        }
    }

    /// Le premier appel, le seul qui condamne l'interrogation : il rouvre la
    /// session si Pi-hole ne la reconnaît plus, une fois.
    async fn first<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<(T, Option<String>), ProbeError> {
        let sid = self.sid().await?;
        let reply = self.client.get(path, sid.as_deref()).await?;
        if reply.status == StatusCode::UNAUTHORIZED {
            self.forget();
            if self.password.is_none() {
                return Err(self.client.status_error(reply.status, &reply.body, path));
            }
            let sid = self.open().await?;
            let reply = self.client.get(path, sid.as_deref()).await?;
            if reply.status == StatusCode::UNAUTHORIZED {
                self.forget();
            }
            return Ok((self.client.decode(&reply, path)?, sid));
        }
        Ok((self.client.decode(&reply, path)?, sid))
    }

    fn settle<T: DeserializeOwned>(
        &self,
        reply: Result<Reply, ProbeError>,
        path: &str,
        errors: &mut u32,
    ) -> Option<T> {
        match reply.and_then(|reply| self.client.decode::<T>(&reply, path)) {
            Ok(value) => Some(value),
            Err(error) => {
                *errors += 1;
                warn!(target_id = self.target_id, %error, path, "lecture Pi-hole en échec");
                None
            }
        }
    }
}

pub async fn probe(
    collector: &PiholeCollector,
    target: &Target,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    let options = Options::from_target(target)?;
    let password = password(&target.credential)?;
    let fingerprint = fingerprint(&options.base_url, password.as_deref());
    let client = Client::new(
        crate::http::client(options.insecure_tls)?,
        options.base_url,
        options.request_timeout,
    );
    let probe = Probe { collector, client, target_id: target.id, password, fingerprint };

    let (summary, sid): (Summary, _) = probe.first("/api/stats/summary").await?;
    let mut out = metrics::summary_samples(&summary, ts_ms);

    let sid = sid.as_deref();
    let (blocking, version, messages, ftl) = futures::join!(
        probe.client.get("/api/dns/blocking", sid),
        probe.client.get("/api/info/version", sid),
        probe.client.get("/api/info/messages", sid),
        probe.client.get("/api/info/ftl", sid),
    );
    let mut errors = 0u32;
    if let Some(blocking) = probe.settle::<Blocking>(blocking, "/api/dns/blocking", &mut errors) {
        out.extend(metrics::blocking_samples(&blocking, ts_ms));
    }
    if let Some(version) = probe.settle::<VersionReply>(version, "/api/info/version", &mut errors) {
        out.extend(metrics::version_samples(&version.version, ts_ms));
    }
    if let Some(messages) = probe.settle::<Messages>(messages, "/api/info/messages", &mut errors) {
        out.extend(metrics::message_samples(&messages, ts_ms));
    }
    if let Some(ftl) = probe.settle::<FtlReply>(ftl, "/api/info/ftl", &mut errors) {
        out.extend(metrics::ftl_samples(&ftl.ftl, ts_ms));
    }
    out.push(Sample::new("pihole_scrape_errors", f64::from(errors), MetricKind::Gauge, ts_ms));
    Ok(out)
}

#[async_trait]
impl Collector for PiholeCollector {
    fn kind(&self) -> &'static str {
        "pihole"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(self, target, chrono::Utc::now().timestamp_millis()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        probe(self, target, chrono::Utc::now().timestamp_millis()).await?;
        Ok(Some("pihole".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::{get, post};
    use serde_json::Value;

    use super::*;

    const SUMMARY: &str = include_str!("testdata/2026.09.0/stats_summary.json");
    const BLOCKING: &str = include_str!("testdata/2026.09.0/dns_blocking.json");
    const VERSION: &str = include_str!("testdata/2026.09.0/info_version.json");
    const MESSAGES: &str = include_str!("testdata/2026.09.0/info_messages.json");
    const FTL: &str = include_str!("testdata/2026.09.0/info_ftl.json");

    const PASSWORD: &str = "mot-de-passe-application";

    fn target(address: String, credential: Credential) -> Target {
        Target {
            id: 11,
            name: "pihole".into(),
            address,
            kind: "pihole".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential,
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

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    /// Un faux Pi-hole : une seule session valide à la fois, que le test
    /// peut faire expirer ; chaque connexion est comptée.
    #[derive(Clone, Default)]
    struct Fake {
        logins: Arc<AtomicUsize>,
        generation: Arc<AtomicUsize>,
    }

    impl Fake {
        fn sid(&self) -> String {
            format!("sid-{}", self.generation.load(Ordering::SeqCst))
        }

        fn app(&self) -> Router {
            let guard = |fake: Fake, body: &'static str| {
                move |headers: HeaderMap| {
                    let fake = fake.clone();
                    async move {
                        let sid = headers.get("x-ftl-sid").and_then(|v| v.to_str().ok());
                        if sid != Some(fake.sid().as_str()) {
                            return (
                                StatusCode::UNAUTHORIZED,
                                r#"{"error":{"key":"unauthorized","message":"Unauthorized","hint":null}}"#,
                            );
                        }
                        (StatusCode::OK, body)
                    }
                }
            };
            let login = {
                let fake = self.clone();
                move |axum::Json(body): axum::Json<Value>| {
                    let fake = fake.clone();
                    async move {
                        fake.logins.fetch_add(1, Ordering::SeqCst);
                        if body["password"] != PASSWORD {
                            return (
                                StatusCode::UNAUTHORIZED,
                                r#"{"session":{"valid":false,"totp":false,"sid":null,"validity":-1,"message":"password incorrect"}}"#
                                    .to_string(),
                            );
                        }
                        let generation = fake.generation.fetch_add(1, Ordering::SeqCst) + 1;
                        (
                            StatusCode::OK,
                            format!(
                                r#"{{"session":{{"valid":true,"totp":false,"sid":"sid-{generation}","csrf":"x","validity":1800,"message":"app-password correct"}}}}"#
                            ),
                        )
                    }
                }
            };
            Router::new()
                .route("/api/auth", post(login))
                .route("/api/stats/summary", get(guard(self.clone(), SUMMARY)))
                .route("/api/dns/blocking", get(guard(self.clone(), BLOCKING)))
                .route("/api/info/version", get(guard(self.clone(), VERSION)))
                .route("/api/info/messages", get(guard(self.clone(), MESSAGES)))
                .route("/api/info/ftl", get(guard(self.clone(), FTL)))
        }
    }

    #[test]
    fn le_type_est_annonce() {
        assert_eq!(PiholeCollector::new().kind(), "pihole");
    }

    #[tokio::test]
    async fn la_session_est_gardee_puis_rouverte_quand_elle_expire() {
        let fake = Fake::default();
        let base = serve(fake.app()).await;
        let collector = PiholeCollector::new();
        let target =
            target(format!("{base}/admin/"), Credential::ApiToken { token: PASSWORD.into() });

        let samples = collector.probe(&target).await.unwrap();
        assert_eq!(value(&samples, "pihole_queries_24h"), Some(8.0));
        assert_eq!(value(&samples, "pihole_blocking_enabled"), Some(1.0));
        assert_eq!(value(&samples, "pihole_updates_available"), Some(0.0));
        assert_eq!(value(&samples, "pihole_messages"), Some(1.0));
        assert_eq!(value(&samples, "pihole_scrape_errors"), Some(0.0));
        assert_eq!(fake.logins.load(Ordering::SeqCst), 1);

        // Deuxième interrogation : la même session, aucune nouvelle connexion.
        collector.probe(&target).await.unwrap();
        assert_eq!(fake.logins.load(Ordering::SeqCst), 1);

        // Pi-hole a oublié la session (redémarrage de FTL) : une reconnexion,
        // et l'interrogation réussit quand même.
        fake.generation.fetch_add(1, Ordering::SeqCst);
        let samples = collector.probe(&target).await.unwrap();
        assert_eq!(value(&samples, "pihole_scrape_errors"), Some(0.0));
        assert_eq!(fake.logins.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn un_mauvais_mot_de_passe_n_est_pas_une_panne() {
        let fake = Fake::default();
        let base = serve(fake.app()).await;
        let error = PiholeCollector::new()
            .probe(&target(base.clone(), Credential::ApiToken { token: "faux".into() }))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error:?}");
        assert!(!error.means_down());

        // Sans identifiant face à un Pi-hole protégé : même verdict.
        let error =
            PiholeCollector::new().probe(&target(base, Credential::None)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(ref m) if m.contains("app password")));
    }

    #[tokio::test]
    async fn un_pihole_sans_mot_de_passe_repond_sans_session() {
        let app = Router::new()
            .route("/api/stats/summary", get(|| async { SUMMARY }))
            .route("/api/dns/blocking", get(|| async { BLOCKING }))
            .route("/api/info/version", get(|| async { VERSION }))
            .route("/api/info/messages", get(|| async { MESSAGES }))
            .route("/api/info/ftl", get(|| async { (StatusCode::FORBIDDEN, "{}") }));
        let base = serve(app).await;
        let samples = PiholeCollector::new().probe(&target(base, Credential::None)).await.unwrap();
        assert_eq!(value(&samples, "pihole_queries_blocked_24h"), Some(3.0));
        // Un appel secondaire refusé est compté, il ne fait pas échouer la sonde.
        assert_eq!(value(&samples, "pihole_scrape_errors"), Some(1.0));
        assert!(value(&samples, "pihole_ftl_uptime_seconds").is_none());
    }

    #[tokio::test]
    async fn pihole_5_est_reconnu() {
        let app = Router::new().route("/admin/api.php", get(|| async { "{}" }));
        let base = serve(app).await;
        let error = PiholeCollector::new()
            .probe(&target(base, Credential::ApiToken { token: PASSWORD.into() }))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(ref m) if m.contains("Pi-hole 5")));
    }

    #[test]
    fn un_identifiant_d_un_autre_type_est_refuse() {
        let error = password(&Credential::SnmpCommunity { community: "public".into() });
        assert!(matches!(error, Err(ProbeError::Config(_))));
        assert_eq!(
            password(&Credential::ApiToken { token: " abc\n".into() }).unwrap(),
            Some("abc".into())
        );
    }

    /// Contre un vrai Pi-hole, hors de la suite normale :
    /// `PIHOLE_URL=http://… PIHOLE_PASSWORD=… cargo test -p dumbmonit-collectors
    /// pihole_reel -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "demande un Pi-hole joignable"]
    async fn pihole_reel() {
        let url = std::env::var("PIHOLE_URL").expect("PIHOLE_URL");
        let credential = match std::env::var("PIHOLE_PASSWORD") {
            Ok(token) => Credential::ApiToken { token },
            Err(_) => Credential::None,
        };
        let collector = PiholeCollector::new();
        let target = target(url, credential);
        for _ in 0..2 {
            let samples = collector.probe(&target).await.unwrap();
            for sample in &samples {
                println!("{} {:?} {}", sample.metric, sample.labels, sample.value);
            }
            assert_eq!(value(&samples, "pihole_scrape_errors"), Some(0.0));
        }
    }
}
