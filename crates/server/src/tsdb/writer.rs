//! Tampon d'écriture vers VictoriaMetrics.
//!
//! Les collecteurs produisent par à-coups ; regrouper leurs échantillons en lots
//! transforme des centaines de petites requêtes HTTP par minute en quelques-unes.

use std::time::Duration;

use dumbmonit_proto::Sample;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

use super::Victoria;
use crate::stats::stats;

/// Au-delà, on considère que VictoriaMetrics est durablement indisponible et on
/// sacrifie les échantillons les plus anciens plutôt que la mémoire du serveur.
/// Correspond à plusieurs minutes de collecte pour une centaine d'équipements.
const MAX_BUFFERED: usize = 200_000;

/// Taille de lot déclenchant un envoi immédiat, sans attendre l'échéance, quand
/// l'appelant n'en précise pas (`DUMBMONIT_FLUSH_BATCH` côté serveur).
pub const DEFAULT_FLUSH_SIZE: usize = 5_000;

/// Capacité initiale du tampon. Il grandit à la demande jusqu'à la taille de lot
/// puis garde cette capacité : inutile de réserver d'emblée la place d'un lot
/// complet que la plupart des instances n'atteignent jamais.
const INITIAL_CAPACITY: usize = 512;

/// Nombre de lots en attente avant que `try_send` refuse et abandonne le lot.
/// Chaque lot est l'ensemble des échantillons d'une seule interrogation : à
/// `max_concurrent_probes` sondes en vol (soixante-quatre par défaut), cette
/// capacité couvre plusieurs tours complets avant de perdre quoi que ce soit.
const CHANNEL_CAPACITY: usize = 1024;

/// Point d'entrée des échantillons. Clonable, à distribuer aux collecteurs.
#[derive(Clone)]
pub struct SampleSink {
    tx: mpsc::Sender<Vec<Sample>>,
    /// Demandes de vidage immédiat, acquittées une fois l'envoi tenté. Les clones
    /// du point d'entrée vivent dans l'état partagé des tâches de fond : la
    /// fermeture du canal principal n'arrive donc jamais avant l'arrêt du runtime,
    /// et ne peut pas servir de signal d'arrêt.
    flush_tx: mpsc::Sender<oneshot::Sender<()>>,
}

impl SampleSink {
    /// Dépose un lot d'échantillons.
    ///
    /// N'échoue jamais du point de vue de l'appelant, et ne bloque jamais : si le
    /// tampon est saturé, le lot est abandonné avec une trace plutôt que d'attendre
    /// de la place. L'appelant est une tâche `probe_once` qui tient encore son jeton
    /// du sémaphore du planificateur pendant cet appel — un `send` bloquant
    /// transformerait un VictoriaMetrics lent en un planificateur bloqué pour toutes
    /// les cibles, pas seulement la plus lente.
    pub async fn send(&self, samples: Vec<Sample>) {
        if samples.is_empty() {
            return;
        }
        let count = samples.len();
        match self.tx.try_send(samples) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!(count, "write buffer full, samples dropped");
                stats().samples_dropped(count);
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                warn!(count, "write buffer closed, samples dropped");
                stats().samples_dropped(count);
            }
        }
    }

    /// Vide le tampon tout de suite et attend que l'envoi ait été tenté, au plus
    /// `timeout`. Appelée à l'arrêt du serveur, avant celui de VictoriaMetrics :
    /// sans elle, la fin du runtime annule la tâche d'écriture et les échantillons
    /// encore en tampon sont perdus. Renvoie `false` si l'acquittement n'est pas
    /// arrivé à temps (tâche disparue ou VictoriaMetrics trop lent).
    pub async fn flush(&self, timeout: Duration) -> bool {
        let (ack, done) = oneshot::channel();
        let attempt = async { self.flush_tx.send(ack).await.is_ok() && done.await.is_ok() };
        tokio::time::timeout(timeout, attempt).await.unwrap_or(false)
    }
}

/// Démarre la tâche d'écriture et renvoie le point d'entrée à distribuer.
pub fn spawn_writer(victoria: Victoria, flush_interval: Duration) -> SampleSink {
    spawn_writer_with(victoria, flush_interval, DEFAULT_FLUSH_SIZE)
}

/// Comme [`spawn_writer`], avec la taille de lot choisie par l'appelant.
pub fn spawn_writer_with(
    victoria: Victoria,
    flush_interval: Duration,
    flush_size: usize,
) -> SampleSink {
    let (tx, mut rx) = mpsc::channel::<Vec<Sample>>(CHANNEL_CAPACITY);
    let (flush_tx, mut flush_rx) = mpsc::channel::<oneshot::Sender<()>>(4);
    let flush_at = flush_size.max(1);

    tokio::spawn(async move {
        let mut buffer: Vec<Sample> = Vec::with_capacity(INITIAL_CAPACITY);
        let mut ticker = tokio::time::interval(flush_interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                received = rx.recv() => {
                    match received {
                        Some(samples) => {
                            buffer.extend(samples);
                            if buffer.len() >= flush_at {
                                flush(&victoria, &mut buffer).await;
                            }
                        }
                        // Tous les émetteurs ont disparu : on vide et on s'arrête.
                        None => {
                            flush(&victoria, &mut buffer).await;
                            debug!("write task stopped");
                            return;
                        }
                    }
                }
                Some(ack) = flush_rx.recv() => {
                    // Les lots déjà déposés font partie du vidage demandé.
                    while let Ok(samples) = rx.try_recv() {
                        buffer.extend(samples);
                    }
                    let pending = buffer.len();
                    flush(&victoria, &mut buffer).await;
                    info!(pending, left = buffer.len(), "write buffer flushed on request");
                    let _ = ack.send(());
                }
                _ = ticker.tick() => flush(&victoria, &mut buffer).await,
            }
        }
    });

    SampleSink { tx, flush_tx }
}

async fn flush(victoria: &Victoria, buffer: &mut Vec<Sample>) {
    if buffer.is_empty() {
        return;
    }

    match victoria.write(buffer).await {
        Ok(()) => {
            debug!(count = buffer.len(), "samples written");
            stats().samples_written(buffer.len(), 0);
            buffer.clear();
        }
        Err(error) => {
            // On conserve le lot pour la prochaine tentative, en bornant la mémoire.
            if buffer.len() > MAX_BUFFERED {
                let dropped = buffer.len() - MAX_BUFFERED;
                buffer.drain(..dropped);
                error!(
                    %error,
                    dropped,
                    "VictoriaMetrics unavailable for too long, oldest samples dropped"
                );
            } else {
                warn!(%error, pending = buffer.len(), "write deferred");
            }
            stats().sample_write_failed(buffer.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dumbmonit_proto::MetricKind;

    fn batch() -> Vec<Sample> {
        vec![Sample::new("probe_up", 1.0, MetricKind::Gauge, 0)]
    }

    /// `send` ne doit jamais attendre après de la place : un tampon saturé fait
    /// perdre le lot plutôt que de bloquer l'appelant, qui tient encore le jeton
    /// du sémaphore du planificateur.
    #[tokio::test]
    async fn send_drops_without_blocking_when_the_buffer_is_full() {
        let (tx, _rx) = mpsc::channel::<Vec<Sample>>(1);
        let (flush_tx, _flush_rx) = mpsc::channel(1);
        let sink = SampleSink { tx, flush_tx };

        // Premier lot : il prend l'unique place, personne ne le vide derrière.
        sink.send(batch()).await;
        let before = stats().snapshot().samples_dropped;

        // Deuxième lot : le canal est plein, `send` doit revenir aussitôt plutôt
        // que d'attendre une place, et le compteur de pertes doit bouger.
        let dropped = tokio::time::timeout(Duration::from_millis(200), sink.send(batch())).await;
        assert!(dropped.is_ok(), "send() a attendu alors que le tampon était saturé");
        assert_eq!(stats().snapshot().samples_dropped, before + 1);
    }

    /// Un canal fermé (le lecteur a disparu) ne doit pas non plus faire attendre
    /// l'appelant : le lot est simplement abandonné.
    #[tokio::test]
    async fn send_drops_without_blocking_when_the_buffer_is_closed() {
        let (tx, rx) = mpsc::channel::<Vec<Sample>>(4);
        drop(rx);
        let (flush_tx, _flush_rx) = mpsc::channel(1);
        let sink = SampleSink { tx, flush_tx };
        let before = stats().snapshot().samples_dropped;

        let result = tokio::time::timeout(Duration::from_millis(200), sink.send(batch())).await;
        assert!(result.is_ok(), "send() a attendu alors que le canal était fermé");
        assert_eq!(stats().snapshot().samples_dropped, before + 1);
    }

    /// L'arrêt du serveur ne ferme jamais le canal (des clones restent dans
    /// l'état partagé) : seul un vidage explicite garantit que le tampon atteint
    /// VictoriaMetrics avant l'arrêt du runtime. Échéance d'une heure : seul le
    /// vidage demandé peut avoir envoyé le lot.
    #[tokio::test]
    async fn flush_writes_the_buffer_while_other_clones_are_alive() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let received = Arc::new(AtomicUsize::new(0));
        let counter = received.clone();
        let app = axum::Router::new().route(
            "/api/v1/import/prometheus",
            axum::routing::post(move |body: String| {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(body.lines().count(), Ordering::SeqCst);
                    axum::http::StatusCode::NO_CONTENT
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let victoria = Victoria::new(format!("http://{address}")).unwrap();
        let sink = spawn_writer_with(victoria, Duration::from_secs(3600), DEFAULT_FLUSH_SIZE);
        // Un clone qui survit, comme celui de l'état partagé des tâches de fond.
        let _held = sink.clone();
        sink.send(batch()).await;

        assert!(sink.flush(Duration::from_secs(5)).await, "vidage non acquitté");
        assert!(received.load(Ordering::SeqCst) >= 1, "aucun échantillon reçu");
    }

    /// Sans tâche d'écriture pour répondre, le vidage rend la main à l'échéance.
    #[tokio::test]
    async fn flush_gives_up_after_the_timeout() {
        let (tx, _rx) = mpsc::channel::<Vec<Sample>>(1);
        let (flush_tx, _flush_rx) = mpsc::channel(1);
        let sink = SampleSink { tx, flush_tx };
        assert!(!sink.flush(Duration::from_millis(50)).await);
    }
}
