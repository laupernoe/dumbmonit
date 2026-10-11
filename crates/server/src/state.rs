use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sqlx::SqlitePool;

use crate::collectors::Registry;
use crate::collectors::relay::RelayHub;
use crate::config::Config;
use crate::crypto::Cipher;
use crate::tsdb::{SampleSink, Victoria};

/// État partagé par tous les gestionnaires HTTP et par le planificateur.
#[derive(Clone)]
pub struct AppState(Arc<Inner>, Arc<RelayHub>, Arc<Cooldowns>);

pub struct Inner {
    pub config: Config,
    pub pool: SqlitePool,
    pub cipher: Cipher,
    pub victoria: Victoria,
    pub sink: SampleSink,
    pub collectors: Registry,
}

impl AppState {
    pub fn new(inner: Inner) -> Self {
        Self(Arc::new(inner), Arc::new(RelayHub::new()), Arc::new(Cooldowns::default()))
    }

    /// Sondes déléguées aux agents relais. Construite avec l'état plutôt que
    /// fournie par l'appelant : elle n'a aucune dépendance, et chaque instance
    /// (serveur ou test) doit avoir la sienne.
    pub fn relay(&self) -> &RelayHub {
        &self.1
    }

    /// Délais minimaux entre deux gestes coûteux d'un même compte (envoi de
    /// test Web Push…). Propres à l'instance, comme le relais.
    pub fn cooldowns(&self) -> &Cooldowns {
        &self.2
    }
}

impl std::ops::Deref for AppState {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Dernier usage de chaque geste limité, par clé (nom du geste, compte).
///
/// En mémoire : un redémarrage remet les compteurs à zéro, ce qui est sans
/// conséquence pour un délai de quelques dizaines de secondes.
#[derive(Default)]
pub struct Cooldowns(Mutex<HashMap<(&'static str, i64), Instant>>);

impl Cooldowns {
    /// Autorise le geste et le note, ou renvoie les secondes à attendre.
    pub fn try_take(&self, action: &'static str, who: i64, every: Duration) -> Result<(), u64> {
        let now = Instant::now();
        let mut map = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(last) = map.get(&(action, who)) {
            let elapsed = now.duration_since(*last);
            if elapsed < every {
                return Err((every - elapsed).as_secs().max(1));
            }
        }
        // Purge des entrées périmées : la table reste à la taille des comptes actifs.
        map.retain(|_, last| now.duration_since(*last) < every.max(Duration::from_secs(3600)));
        map.insert((action, who), now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_geste_limite_attend_son_delai() {
        let cooldowns = Cooldowns::default();
        let every = Duration::from_secs(30);
        assert!(cooldowns.try_take("test", 1, every).is_ok());
        let wait = cooldowns.try_take("test", 1, every).unwrap_err();
        assert!((1..=30).contains(&wait), "{wait}");
        // Un autre compte, ou un autre geste, n'est pas concerné.
        assert!(cooldowns.try_take("test", 2, every).is_ok());
        assert!(cooldowns.try_take("autre", 1, every).is_ok());
    }
}
