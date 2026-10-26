//! Collecteur « otlp » : la cible qui reçoit les métriques OpenTelemetry.
//!
//! Comme le moniteur « push », `probe` n'interroge rien : les points arrivent par
//! `POST /api/otlp/v1/metrics` (`api/otlp.rs`) et sont écrits sous l'identité de la
//! cible. Ce collecteur ne fait que publier l'activité de la réception, pour
//! qu'une règle de seuil sur l'âge du dernier envoi signale un exportateur muet.
//!
//! Les compteurs sont en mémoire : après un redémarrage du serveur, rien n'est
//! publié jusqu'au premier envoi, et la sonde reste en succès.
//!
//! | Métrique | Sens |
//! |---|---|
//! | `otlp_last_receive_seconds` | Âge du dernier envoi accepté. |
//! | `otlp_points_received_total` | Points de données reçus depuis le démarrage. |
//! | `otlp_points_rejected_total` | Points refusés (temporalité delta, horodatage…). |

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target, TargetId};

/// Type de cible, tel qu'enregistré dans `Target::kind`.
pub const KIND: &str = "otlp";

struct Activity {
    last: Instant,
    received: u64,
    rejected: u64,
}

static ACTIVITY: LazyLock<Mutex<HashMap<TargetId, Activity>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Note un envoi accepté pour la cible.
pub fn record(target: TargetId, received: usize, rejected: usize) {
    let mut map = ACTIVITY.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let entry =
        map.entry(target).or_insert(Activity { last: Instant::now(), received: 0, rejected: 0 });
    entry.last = Instant::now();
    entry.received += received as u64;
    entry.rejected += rejected as u64;
}

#[derive(Default)]
pub struct OtlpCollector;

#[async_trait]
impl Collector for OtlpCollector {
    fn kind(&self) -> &'static str {
        KIND
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let map = ACTIVITY.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(activity) = map.get(&target.id) else { return Ok(Vec::new()) };
        let now_ms = chrono::Utc::now().timestamp_millis();
        Ok(vec![
            Sample::new(
                "otlp_last_receive_seconds",
                activity.last.elapsed().as_secs_f64(),
                MetricKind::Gauge,
                now_ms,
            ),
            Sample::new(
                "otlp_points_received_total",
                activity.received as f64,
                MetricKind::Counter,
                now_ms,
            ),
            Sample::new(
                "otlp_points_rejected_total",
                activity.rejected as f64,
                MetricKind::Counter,
                now_ms,
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(id: TargetId) -> Target {
        Target {
            id,
            name: "otel".into(),
            address: "otlp".into(),
            kind: KIND.into(),
            profile_id: None,
            parent_id: None,
            interval: std::time::Duration::from_secs(60),
            enabled: true,
            tags: Default::default(),
            credential: dumbmonit_proto::Credential::None,
            group_name: String::new(),
            position: 0,
        }
    }

    #[tokio::test]
    async fn rien_n_est_publie_avant_le_premier_envoi() {
        let samples = OtlpCollector.probe(&target(987_001)).await.unwrap();
        assert!(samples.is_empty());
    }

    #[tokio::test]
    async fn l_activite_est_cumulee_par_cible() {
        record(987_002, 10, 1);
        record(987_002, 5, 0);
        let samples = OtlpCollector.probe(&target(987_002)).await.unwrap();
        let value = |name: &str| samples.iter().find(|s| s.metric == name).unwrap().value;
        assert_eq!(value("otlp_points_received_total"), 15.0);
        assert_eq!(value("otlp_points_rejected_total"), 1.0);
        assert!(value("otlp_last_receive_seconds") < 5.0);
    }
}
