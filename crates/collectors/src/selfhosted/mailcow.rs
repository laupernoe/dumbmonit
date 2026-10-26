//! mailcow: dockerized.
//!
//! Une clé d'API en **lecture seule** (`X-API-Key`, créée dans System →
//! Configuration → Access → API, avec la liste d'adresses autorisées qui va
//! avec) et cinq routes `GET` :
//! `/api/v1/get/status/containers`, `/status/version`, `/status/vmail`,
//! `/mailq/all` et `/mailbox/all`.
//!
//! Les pannes que ce module existe pour voir : **un conteneur arrêté** (rspamd
//! tombé : plus de filtre antispam, ou Dovecot : plus de lecture), **une file
//! Postfix qui se remplit** (un relais refuse tout, le courrier s'accumule), le
//! **volume des courriels plein** (`vmail`), et **des boîtes proches de leur
//! quota** (le courrier entrant leur est alors refusé sans bruit).
//!
//! Seuls des compteurs et les noms des conteneurs et des boîtes proches du
//! quota sortent ; aucun courriel, expéditeur ou destinataire n'est publié.

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use serde_json::Value;

use super::client::{Auth, HttpClient};
use super::{
    MAX_NAMED, flag, gauge, http_client, metric, now_ms, number, optional_json, settle, token,
};

pub const DEFAULT_PORT: u16 = 443;
const PRODUCT: &str = "mailcow";
const P: &str = "mailcow";

/// Une boîte est « proche du quota » à partir de ce pourcentage.
pub const QUOTA_WARN_PERCENT: f64 = 90.0;
/// Les boîtes à partir de ce pourcentage sont nommées (au plus [`MAX_NAMED`]).
pub const QUOTA_NAMED_PERCENT: f64 = 80.0;

// ------------------------------------------------------------ traduction

/// `GET /api/v1/get/status/containers` : `{"<nom>": {"state": "running", …}}`.
pub fn container_samples(body: &Value, ts_ms: i64) -> Vec<Sample> {
    let Some(containers) = body.as_object() else { return Vec::new() };
    let running = |c: &Value| c.get("state").and_then(Value::as_str) == Some("running");
    let mut out = vec![
        gauge(&metric(P, "containers_total"), containers.len() as f64, ts_ms),
        gauge(
            &metric(P, "containers_running"),
            containers.values().filter(|c| running(c)).count() as f64,
            ts_ms,
        ),
    ];
    for (name, container) in containers.iter().take(MAX_NAMED) {
        out.push(
            flag(&metric(P, "container_up"), running(container), ts_ms)
                .with_label("container", name),
        );
    }
    out
}

/// `GET /api/v1/get/status/version` : `{"version": "2025-08a"}`.
pub fn version_samples(body: &Value, ts_ms: i64) -> Vec<Sample> {
    match body.get("version").and_then(Value::as_str) {
        Some(version) => {
            vec![gauge(&metric(P, "version_info"), 1.0, ts_ms).with_label("version", version)]
        }
        None => Vec::new(),
    }
}

/// `GET /api/v1/get/status/vmail` : `{"used_percent": "85%"}` (du texte).
pub fn vmail_samples(body: &Value, ts_ms: i64) -> Vec<Sample> {
    let percent = body
        .get("used_percent")
        .and_then(Value::as_str)
        .and_then(|text| text.trim().trim_end_matches('%').trim().parse::<f64>().ok());
    percent.map(|p| gauge(&metric(P, "vmail_used_percent"), p, ts_ms)).into_iter().collect()
}

/// `GET /api/v1/get/mailq/all` : un élément par message en file. Une file
/// vide est rendue tantôt `[]`, tantôt `{}`.
pub fn queue_samples(body: &Value, now_s: i64, ts_ms: i64) -> Vec<Sample> {
    let messages = body.as_array().map(Vec::as_slice).unwrap_or(&[]);
    let oldest = messages
        .iter()
        .filter_map(|m| number(m.get("arrival_time")))
        .map(|arrival| arrival as i64)
        .min();
    let age = oldest.map(|arrival| (now_s - arrival).max(0)).unwrap_or(0);
    vec![
        gauge(&metric(P, "queue_messages"), messages.len() as f64, ts_ms),
        gauge(&metric(P, "queue_oldest_seconds"), age as f64, ts_ms),
    ]
}

/// Le taux d'occupation d'une boîte, ou `None` sans quota (0 = illimité).
fn quota_percent(mailbox: &Value) -> Option<f64> {
    let limit = number(mailbox.get("quota")).filter(|limit| *limit > 0.0)?;
    match number(mailbox.get("percent_in_use")) {
        Some(percent) => Some(percent),
        None => Some(100.0 * number(mailbox.get("quota_used")).unwrap_or(0.0) / limit),
    }
}

/// `GET /api/v1/get/mailbox/all`.
pub fn mailbox_samples(body: &Value, ts_ms: i64) -> Vec<Sample> {
    let Some(mailboxes) = body.as_array() else { return Vec::new() };
    let inactive =
        mailboxes.iter().filter(|m| number(m.get("active")).is_some_and(|a| a == 0.0)).count();
    let mut near = 0usize;
    let mut named = Vec::new();
    for mailbox in mailboxes {
        let Some(percent) = quota_percent(mailbox) else { continue };
        if percent >= QUOTA_WARN_PERCENT {
            near += 1;
        }
        if percent >= QUOTA_NAMED_PERCENT && named.len() < MAX_NAMED {
            let name = mailbox.get("username").and_then(Value::as_str).unwrap_or("unknown");
            named.push(
                gauge(&metric(P, "mailbox_quota_percent"), percent, ts_ms)
                    .with_label("mailbox", name),
            );
        }
    }
    let mut out = vec![
        gauge(&metric(P, "mailboxes_total"), mailboxes.len() as f64, ts_ms),
        gauge(&metric(P, "mailboxes_inactive"), inactive as f64, ts_ms),
        gauge(&metric(P, "mailboxes_near_quota"), near as f64, ts_ms),
    ];
    out.extend(named);
    out
}

// ------------------------------------------------------------------ sonde

pub async fn probe(client: &HttpClient, target: &Target, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    // La preuve de vie et d'authentification : le seul appel qui condamne.
    let containers: Value = client.get_json("/api/v1/get/status/containers").await?;
    let mut out = container_samples(&containers, ts_ms);
    let mut errors = 0u32;
    let (version, vmail, queue, mailboxes) = futures::join!(
        optional_json(client, "/api/v1/get/status/version"),
        optional_json(client, "/api/v1/get/status/vmail"),
        optional_json(client, "/api/v1/get/mailq/all"),
        optional_json(client, "/api/v1/get/mailbox/all"),
    );
    if let Some(Some(body)) = settle(version, &mut errors, target.id, "status/version") {
        out.extend(version_samples(&body, ts_ms));
    }
    if let Some(Some(body)) = settle(vmail, &mut errors, target.id, "status/vmail") {
        out.extend(vmail_samples(&body, ts_ms));
    }
    if let Some(Some(body)) = settle(queue, &mut errors, target.id, "mailq/all") {
        out.extend(queue_samples(&body, ts_ms / 1000, ts_ms));
    }
    if let Some(Some(body)) = settle(mailboxes, &mut errors, target.id, "mailbox/all") {
        out.extend(mailbox_samples(&body, ts_ms));
    }
    out.push(gauge(&metric(P, "scrape_errors"), f64::from(errors), ts_ms));
    Ok(out)
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct MailcowCollector;

impl MailcowCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        let Some(key) = token(&target.credential) else {
            return Err(ProbeError::Config(format!(
                "mailcow expects an API key, configured: {}",
                target.credential
            )));
        };
        http_client(target, "https", DEFAULT_PORT, Auth::header("X-API-Key", key), PRODUCT)
    }
}

#[async_trait]
impl Collector for MailcowCollector {
    fn kind(&self) -> &str {
        "mailcow"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(&Self::client(target)?, target, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let _: Value = Self::client(target)?.get_json("/api/v1/get/status/containers").await?;
        Ok(Some("mailcow".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;
    use dumbmonit_proto::Credential;

    use super::*;

    // Réponses d'un mailcow 2025-08a : six conteneurs dont rspamd arrêté, un
    // volume des courriels à 85 %, trois messages en file (le plus ancien
    // date de 9 500 s) et cinq boîtes dont une à 95 % du quota, une à 85 %,
    // une désactivée et une sans quota.
    const CONTAINERS: &str = include_str!("testdata/mailcow_2025-08/containers.json");
    const VERSION: &str = include_str!("testdata/mailcow_2025-08/version.json");
    const VMAIL: &str = include_str!("testdata/mailcow_2025-08/vmail.json");
    const MAILQ: &str = include_str!("testdata/mailcow_2025-08/mailq.json");
    const MAILBOXES: &str = include_str!("testdata/mailcow_2025-08/mailbox_all.json");

    // 2026-09-30T20:18:20Z : 9 500 s après le plus ancien message de la file.
    const NOW: i64 = 1_790_799_500;

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn named(samples: &[Sample], name: &str, key: &str, wanted: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| s.metric == name && s.labels.get(key).map(String::as_str) == Some(wanted))
            .map(|s| s.value)
    }

    #[test]
    fn un_conteneur_arrete_se_voit_par_nom() {
        let samples = container_samples(&json(CONTAINERS), 0);
        assert_eq!(value(&samples, "mailcow_containers_total"), Some(6.0));
        assert_eq!(value(&samples, "mailcow_containers_running"), Some(5.0));
        assert_eq!(named(&samples, "mailcow_container_up", "container", "rspamd-mailcow"), Some(0.0));
        assert_eq!(named(&samples, "mailcow_container_up", "container", "postfix-mailcow"), Some(1.0));
    }

    #[test]
    fn la_file_postfix_donne_sa_taille_et_l_age_du_plus_ancien() {
        let samples = queue_samples(&json(MAILQ), NOW, 0);
        assert_eq!(value(&samples, "mailcow_queue_messages"), Some(3.0));
        assert_eq!(value(&samples, "mailcow_queue_oldest_seconds"), Some(9_500.0));
        // Une file vide arrive tantôt en tableau, tantôt en objet.
        for empty in ["[]", "{}"] {
            let samples = queue_samples(&json(empty), NOW, 0);
            assert_eq!(value(&samples, "mailcow_queue_messages"), Some(0.0));
            assert_eq!(value(&samples, "mailcow_queue_oldest_seconds"), Some(0.0));
        }
    }

    #[test]
    fn le_volume_des_courriels_arrive_en_texte() {
        assert_eq!(value(&vmail_samples(&json(VMAIL), 0), "mailcow_vmail_used_percent"), Some(85.0));
        assert!(vmail_samples(&json("{}"), 0).is_empty());
        let samples = version_samples(&json(VERSION), 0);
        assert_eq!(samples[0].labels["version"], "2025-08a");
    }

    #[test]
    fn les_boites_proches_du_quota_se_nomment_et_les_boites_sans_quota_sont_ignorees() {
        let samples = mailbox_samples(&json(MAILBOXES), 0);
        assert_eq!(value(&samples, "mailcow_mailboxes_total"), Some(5.0));
        assert_eq!(value(&samples, "mailcow_mailboxes_inactive"), Some(1.0));
        assert_eq!(value(&samples, "mailcow_mailboxes_near_quota"), Some(1.0));
        assert_eq!(
            named(&samples, "mailcow_mailbox_quota_percent", "mailbox", "bob@example.com"),
            Some(95.0)
        );
        // À 85 %, nommée mais pas encore « proche ».
        assert_eq!(
            named(&samples, "mailcow_mailbox_quota_percent", "mailbox", "dave@example.com"),
            Some(85.0)
        );
        assert!(
            named(&samples, "mailcow_mailbox_quota_percent", "mailbox", "shared@example.com")
                .is_none()
        );
        assert!(
            named(&samples, "mailcow_mailbox_quota_percent", "mailbox", "alice@example.com")
                .is_none()
        );
    }

    #[test]
    fn le_pourcentage_manquant_se_calcule() {
        let mailbox = json(r#"{"quota": 1000, "quota_used": 950, "percent_in_use": null}"#);
        assert_eq!(quota_percent(&mailbox), Some(95.0));
        assert_eq!(quota_percent(&json(r#"{"quota": 0, "quota_used": 5}"#)), None);
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(address: String, key: &str) -> Target {
        Target {
            id: 14,
            name: "mail".into(),
            address,
            kind: "mailcow".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential: Credential::ApiToken { token: key.into() },
            group_name: String::new(),
            position: 0,
        }
    }

    fn app() -> Router {
        Router::new()
            .route(
                "/api/v1/get/status/containers",
                get(|headers: HeaderMap| async move {
                    let ok = headers.get("x-api-key").and_then(|v| v.to_str().ok()) == Some("cle");
                    if ok {
                        (StatusCode::OK, CONTAINERS)
                    } else {
                        (StatusCode::UNAUTHORIZED, r#"{"type":"error","msg":"authentication failed"}"#)
                    }
                }),
            )
            .route("/api/v1/get/status/version", get(|| async { VERSION }))
            .route("/api/v1/get/status/vmail", get(|| async { VMAIL }))
            .route("/api/v1/get/mailq/all", get(|| async { MAILQ }))
            .route("/api/v1/get/mailbox/all", get(|| async { MAILBOXES }))
    }

    #[tokio::test]
    async fn sonde_complete() {
        let base = serve(app()).await;
        let samples = MailcowCollector::new().probe(&target(base, "cle")).await.unwrap();
        assert_eq!(value(&samples, "mailcow_containers_running"), Some(5.0));
        assert_eq!(value(&samples, "mailcow_queue_messages"), Some(3.0));
        assert_eq!(value(&samples, "mailcow_mailboxes_near_quota"), Some(1.0));
        assert_eq!(value(&samples, "mailcow_scrape_errors"), Some(0.0));
    }

    #[tokio::test]
    async fn une_cle_refusee_n_est_pas_une_panne_et_un_serveur_eteint_en_est_une() {
        let base = serve(app()).await;
        let error = MailcowCollector::new().probe(&target(base, "faux")).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
        let error = MailcowCollector::new()
            .probe(&target("http://127.0.0.1:1".into(), "cle"))
            .await
            .unwrap_err();
        assert!(error.means_down());
    }

    #[tokio::test]
    async fn les_pages_absentes_sont_sautees_sans_erreur() {
        let app = Router::new()
            .route("/api/v1/get/status/containers", get(|| async { CONTAINERS }));
        let base = serve(app).await;
        let samples = MailcowCollector::new().probe(&target(base, "cle")).await.unwrap();
        assert_eq!(value(&samples, "mailcow_scrape_errors"), Some(0.0));
        assert!(value(&samples, "mailcow_queue_messages").is_none());
    }
}
