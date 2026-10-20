//! Expédition vers les destinations : une file bornée et une tâche par
//! destination.
//!
//! La couche `tracing` dépose chaque enregistrement dans la file des
//! destinations dont il passe le seuil, par `try_send` : une file pleine perd
//! l'enregistrement et le compte, sans jamais faire attendre l'appelant. La
//! tâche regroupe ce qui arrive en lots (au plus [`BATCH`], au plus une seconde
//! d'attente) et les remet à l'expéditeur ; un lot refusé est perdu, compté, et
//! l'erreur gardée pour l'interface. Un seul avertissement est écrit quand une
//! destination tombe, un autre quand elle revient.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

use super::sinks::{SEND_TIMEOUT, Sink};
use super::{Level, Record, catalog, hub, store};
use crate::crypto::Cipher;

/// Capacité de la file d'une destination.
pub const QUEUE: usize = 10_000;
/// Taille maximale d'un lot.
pub const BATCH: usize = 500;
/// Attente maximale avant d'envoyer un lot incomplet.
pub const FLUSH_EVERY: Duration = Duration::from_secs(1);

/// Compteurs d'une destination, gardés d'un rechargement à l'autre.
#[derive(Default)]
pub struct Stats {
    sent: AtomicU64,
    dropped: AtomicU64,
    failed_batches: AtomicU64,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    last_error: Option<(String, DateTime<Utc>)>,
    last_success: Option<DateTime<Utc>>,
    failing: bool,
}

/// Ce que l'interface affiche d'une destination.
#[derive(Debug, Clone, Default, Serialize)]
pub struct StatsView {
    pub sent: u64,
    pub dropped: u64,
    pub failed_batches: u64,
    pub queued: u64,
    pub last_error: Option<String>,
    pub last_error_at: Option<String>,
    pub last_success_at: Option<String>,
}

/// Horodatage à la manière du reste de l'API : UTC, sans suffixe.
fn server_time(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%d %H:%M:%S").to_string()
}

impl Stats {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Lot remis : vrai si la destination revenait d'une panne.
    fn success(&self, count: usize) -> bool {
        self.sent.fetch_add(count as u64, Ordering::Relaxed);
        let mut state = self.lock();
        state.last_success = Some(Utc::now());
        std::mem::replace(&mut state.failing, false)
    }

    /// Lot perdu : vrai si c'est le premier échec d'une série.
    fn failure(&self, count: usize, error: String) -> bool {
        self.dropped.fetch_add(count as u64, Ordering::Relaxed);
        self.failed_batches.fetch_add(1, Ordering::Relaxed);
        let mut state = self.lock();
        state.last_error = Some((error, Utc::now()));
        !std::mem::replace(&mut state.failing, true)
    }

    /// Configuration inutilisable : l'erreur est montrée, rien n'est envoyé.
    fn misconfigured(&self, error: String) {
        let mut state = self.lock();
        state.last_error = Some((error, Utc::now()));
        state.failing = true;
    }

    pub fn view(&self, queued: u64) -> StatsView {
        let state = self.lock();
        StatsView {
            sent: self.sent.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            failed_batches: self.failed_batches.load(Ordering::Relaxed),
            queued,
            last_error: state.last_error.as_ref().map(|(error, _)| error.clone()),
            last_error_at: state.last_error.as_ref().map(|(_, at)| server_time(*at)),
            last_success_at: state.last_success.map(server_time),
        }
    }
}

/// Une destination active : son seuil et sa file.
pub struct Route {
    pub id: i64,
    pub min: Level,
    tx: mpsc::Sender<Arc<Record>>,
    stats: Arc<Stats>,
}

/// Les destinations actives, remplacées d'un bloc à chaque rechargement.
#[derive(Default)]
pub struct Routes {
    current: RwLock<Arc<Vec<Route>>>,
    stats: Mutex<HashMap<i64, Arc<Stats>>>,
}

impl Routes {
    /// Dépose un enregistrement dans chaque file concernée, sans attendre.
    pub fn offer(&self, record: &Arc<Record>) {
        let routes = self.current.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
        for route in routes.iter() {
            if !record.level.passes(route.min) {
                continue;
            }
            if route.tx.try_send(record.clone()).is_err() {
                route.stats.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn replace(&self, routes: Vec<Route>) {
        *self.current.write().unwrap_or_else(std::sync::PoisonError::into_inner) = Arc::new(routes);
    }

    /// Compteurs d'une destination, créés au besoin.
    pub fn stats(&self, id: i64) -> Arc<Stats> {
        self.stats
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(id)
            .or_default()
            .clone()
    }

    /// Ce que l'interface affiche d'une destination.
    pub fn view(&self, id: i64) -> StatsView {
        let routes = self.current.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
        let queued = routes
            .iter()
            .find(|route| route.id == id)
            .map_or(0, |route| (route.tx.max_capacity() - route.tx.capacity()) as u64);
        self.stats(id).view(queued)
    }

    fn forget(&self, keep: &[i64]) {
        self.stats
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|id, _| keep.contains(id));
    }
}

/// Relit les destinations en base et remplace les tâches d'expédition.
///
/// Les anciennes tâches s'arrêtent d'elles-mêmes : leur file, dont la couche
/// ne tient plus l'émetteur, se vide puis se ferme.
pub async fn reload(pool: &SqlitePool, cipher: &Cipher) -> anyhow::Result<()> {
    let routes = &hub().routes;
    let destinations = store::list(pool).await?;
    let mut active = Vec::new();
    for destination in destinations.iter().filter(|destination| destination.enabled) {
        let stats = routes.stats(destination.id);
        let config = match store::config(pool, cipher, destination).await {
            Ok(config) => config,
            Err(error) => {
                stats.misconfigured(format!("{error:#}"));
                continue;
            }
        };
        let sink = match catalog::build(&destination.kind, &config) {
            Ok(sink) => sink,
            Err(error) => {
                stats.misconfigured(error);
                continue;
            }
        };
        let (tx, rx) = mpsc::channel(QUEUE);
        tokio::spawn(worker(destination.id, rx, sink, stats.clone()));
        active.push(Route {
            id: destination.id,
            min: Level::parse(&destination.min_level).unwrap_or(Level::Info),
            tx,
            stats,
        });
    }
    routes.forget(&destinations.iter().map(|destination| destination.id).collect::<Vec<_>>());
    routes.replace(active);
    Ok(())
}

async fn worker(
    id: i64,
    mut rx: mpsc::Receiver<Arc<Record>>,
    sink: Arc<dyn Sink>,
    stats: Arc<Stats>,
) {
    let mut batch = Vec::with_capacity(BATCH);
    while let Some(first) = rx.recv().await {
        batch.push(first);
        let deadline = tokio::time::Instant::now() + FLUSH_EVERY;
        while batch.len() < BATCH {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Some(record)) => batch.push(record),
                Ok(None) | Err(_) => break,
            }
        }
        let outcome = match tokio::time::timeout(SEND_TIMEOUT, sink.send(&batch)).await {
            Ok(outcome) => outcome,
            Err(_) => Err(format!("no answer within {} s", SEND_TIMEOUT.as_secs())),
        };
        match outcome {
            Ok(()) => {
                if stats.success(batch.len()) {
                    tracing::info!(destination = id, "log destination reachable again");
                }
            }
            Err(error) => {
                if stats.failure(batch.len(), error.clone()) {
                    tracing::warn!(
                        destination = id,
                        %error,
                        "log destination unreachable, records are dropped until it answers"
                    );
                }
            }
        }
        batch.clear();
    }
}

/// Envoie un enregistrement de test, tout de suite, et rend le verdict.
pub async fn test(sink: Arc<dyn Sink>) -> Result<(), String> {
    let mut record = Record::new(Level::Info, "DumbMonit test message: log shipping works");
    record.fields.insert("test".into(), serde_json::Value::Bool(true));
    match tokio::time::timeout(SEND_TIMEOUT, sink.send(&[Arc::new(record)])).await {
        Ok(outcome) => outcome,
        Err(_) => Err(format!("no answer within {} s", SEND_TIMEOUT.as_secs())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct Failing;

    #[async_trait]
    impl Sink for Failing {
        async fn send(&self, _batch: &[Arc<Record>]) -> Result<(), String> {
            Err("connection refused".into())
        }
    }

    #[tokio::test]
    async fn une_destination_en_panne_compte_ses_pertes_sans_bloquer() {
        let routes = Routes::default();
        let stats = routes.stats(7);
        let (tx, rx) = mpsc::channel(4);
        tokio::spawn(worker(7, rx, Arc::new(Failing), stats.clone()));
        routes.replace(vec![Route { id: 7, min: Level::Info, tx, stats: stats.clone() }]);

        // Bien plus que la file n'en tient : rien n'attend, l'excédent est perdu.
        for i in 0..50 {
            routes.offer(&Arc::new(Record::new(Level::Info, format!("m{i}"))));
        }
        // Sous le seuil : ni envoyé ni compté.
        routes.offer(&Arc::new(Record::new(Level::Debug, "bavard")));
        tokio::time::sleep(Duration::from_millis(1500)).await;
        let view = routes.view(7);
        assert_eq!(view.dropped, 50, "{view:?}");
        assert_eq!(view.sent, 0);
        assert!(view.failed_batches >= 1);
        assert_eq!(view.last_error.as_deref(), Some("connection refused"));
    }
}
