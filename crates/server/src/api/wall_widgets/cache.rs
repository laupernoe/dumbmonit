//! Cache d'une source des widgets : une seule entrée (un réglage à la fois),
//! fraîche pendant sa durée de vie, gardée au-delà quand la source tombe.
//!
//! - Frais : servi tel quel, sans appel.
//! - Périmé : relu une fois ; le verrou reste pris pendant la lecture, si bien
//!   que cent écrans qui se réveillent ensemble ne font qu'un appel.
//! - La source échoue : la dernière valeur reste servie (avec l'erreur et sa
//!   date) jusqu'à [`MAX_STALE`], et la source n'est pas relancée avant
//!   [`RETRY_AFTER`] — on ne martèle pas une API qui nous limite.

use std::future::Future;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

/// Délai avant de relancer une source qui vient d'échouer.
pub const RETRY_AFTER: Duration = Duration::from_secs(60);
/// Au-delà, une ancienne valeur ne s'affiche plus : mieux vaut « indisponible »
/// qu'un cours de la veille présenté comme actuel.
pub const MAX_STALE: Duration = Duration::from_secs(6 * 3600);

struct Entry<T> {
    key: String,
    value: Option<(T, Instant, DateTime<Utc>)>,
    failure: Option<(Instant, String)>,
}

/// Ce que le cache rend : la valeur (fraîche ou ancienne), sa date de lecture,
/// et l'erreur de la dernière tentative si elle a échoué.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome<T> {
    pub value: Option<T>,
    pub fetched: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

pub struct Cache<T> {
    slot: Mutex<Option<Entry<T>>>,
}

impl<T: Clone> Default for Cache<T> {
    fn default() -> Self {
        Self { slot: Mutex::new(None) }
    }
}

impl<T: Clone> Cache<T> {
    pub async fn get<F, Fut>(&self, key: &str, ttl: Duration, fetch: F) -> Outcome<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, String>>,
    {
        self.get_at(Instant::now(), key, ttl, fetch).await
    }

    /// [`Cache::get`] à un instant donné (les tests avancent l'horloge).
    pub async fn get_at<F, Fut>(
        &self,
        now: Instant,
        key: &str,
        ttl: Duration,
        fetch: F,
    ) -> Outcome<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, String>>,
    {
        let mut slot = self.slot.lock().await;
        if slot.as_ref().is_some_and(|entry| entry.key != key) {
            *slot = None;
        }
        let entry =
            slot.get_or_insert_with(|| Entry { key: key.to_string(), value: None, failure: None });
        if let Some((value, at, when)) = &entry.value
            && now.saturating_duration_since(*at) < ttl
        {
            return Outcome { value: Some(value.clone()), fetched: Some(*when), error: None };
        }
        if let Some((failed, message)) = &entry.failure
            && now.saturating_duration_since(*failed) < RETRY_AFTER
        {
            let message = message.clone();
            return stale(entry, now, message);
        }
        match fetch().await {
            Ok(value) => {
                let when = Utc::now();
                entry.value = Some((value.clone(), now, when));
                entry.failure = None;
                Outcome { value: Some(value), fetched: Some(when), error: None }
            }
            Err(message) => {
                entry.failure = Some((now, message.clone()));
                stale(entry, now, message)
            }
        }
    }
}

fn stale<T: Clone>(entry: &Entry<T>, now: Instant, error: String) -> Outcome<T> {
    match &entry.value {
        Some((value, at, when)) if now.saturating_duration_since(*at) < MAX_STALE => {
            Outcome { value: Some(value.clone()), fetched: Some(*when), error: Some(error) }
        }
        _ => Outcome { value: None, fetched: None, error: Some(error) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const TTL: Duration = Duration::from_secs(60);

    #[tokio::test]
    async fn fresh_values_are_served_without_calling_again() {
        let cache = Cache::<u32>::default();
        let calls = AtomicUsize::new(0);
        let start = Instant::now();
        let fetch = || async {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(7)
        };
        let first = cache.get_at(start, "k", TTL, fetch).await;
        assert_eq!(first.value, Some(7));
        assert!(first.error.is_none());
        let again = cache
            .get_at(start + Duration::from_secs(30), "k", TTL, || async {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(8)
            })
            .await;
        assert_eq!(again.value, Some(7));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // Périmé : relu.
        let later =
            cache.get_at(start + Duration::from_secs(61), "k", TTL, || async { Ok(9) }).await;
        assert_eq!(later.value, Some(9));
    }

    #[tokio::test]
    async fn a_failure_keeps_the_last_value_and_waits_before_retrying() {
        let cache = Cache::<u32>::default();
        let start = Instant::now();
        cache.get_at(start, "k", TTL, || async { Ok(1) }).await;
        let failed = cache
            .get_at(start + Duration::from_secs(70), "k", TTL, || async {
                Err("answered 429".to_string())
            })
            .await;
        assert_eq!(failed.value, Some(1));
        assert_eq!(failed.error.as_deref(), Some("answered 429"));
        // Pas de nouvel appel pendant RETRY_AFTER.
        let calls = AtomicUsize::new(0);
        let held = cache
            .get_at(start + Duration::from_secs(80), "k", TTL, || async {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(2)
            })
            .await;
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(held.value, Some(1));
        assert!(held.error.is_some());
        // Puis la source est relancée, et l'erreur oubliée.
        let back =
            cache.get_at(start + Duration::from_secs(140), "k", TTL, || async { Ok(3) }).await;
        assert_eq!(back, Outcome { value: Some(3), fetched: back.fetched, error: None });
    }

    #[tokio::test]
    async fn too_old_values_are_not_shown_and_a_new_key_starts_over() {
        let cache = Cache::<u32>::default();
        let start = Instant::now();
        cache.get_at(start, "k", TTL, || async { Ok(1) }).await;
        let gone = cache
            .get_at(start + MAX_STALE + TTL, "k", TTL, || async { Err("down".to_string()) })
            .await;
        assert_eq!(gone.value, None);
        assert_eq!(gone.error.as_deref(), Some("down"));

        let other = cache.get_at(start, "autre", TTL, || async { Err("down".to_string()) }).await;
        assert_eq!(other.value, None, "un autre réglage ne reprend pas l'ancienne valeur");
    }
}
