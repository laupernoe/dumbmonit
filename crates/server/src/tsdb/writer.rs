//! Tampon d'écriture vers VictoriaMetrics.
//!
//! Les collecteurs produisent par à-coups ; regrouper leurs échantillons en lots
//! transforme des centaines de petites requêtes HTTP par minute en quelques-unes.

use std::time::Duration;

use dumbmonit_proto::Sample;
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

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
                _ = ticker.tick() => flush(&victoria, &mut buffer).await,
            }
        }
    });

    SampleSink { tx }
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
        let sink = SampleSink { tx };

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
        let sink = SampleSink { tx };
        let before = stats().snapshot().samples_dropped;

        let result = tokio::time::timeout(Duration::from_millis(200), sink.send(batch())).await;
        assert!(result.is_ok(), "send() a attendu alors que le canal était fermé");
        assert_eq!(stats().snapshot().samples_dropped, before + 1);
    }
}
