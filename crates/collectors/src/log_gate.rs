//! Filtre de journalisation des échecs répétitifs.
//!
//! Un collecteur qui échoue à chaque cycle pour la même raison (droits manquants
//! sur un endpoint, par exemple) noierait le journal. [`LogGate`] retient le dernier
//! état connu par clé : l'échec n'est journalisé en `warn` que la première fois,
//! quand l'erreur change, ou en rappel toutes les 24 h. Le retour à la normale est
//! signalé une seule fois en `info`. Les répétitions passent en `debug`.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tracing::{debug, info, warn};

/// Délai après lequel une erreur inchangée est rappelée.
pub const REMINDER: Duration = Duration::from_secs(24 * 3600);

/// Raison pour laquelle un échec doit être journalisé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    /// Première occurrence pour cette clé.
    First,
    /// L'erreur diffère de la précédente.
    Changed,
    /// Même erreur, rappel périodique.
    Reminder,
}

/// Mémoire des échecs par clé (cible + endpoint, éventuellement nœud).
#[derive(Debug, Default)]
pub struct LogGate {
    state: Mutex<HashMap<String, (String, Instant)>>,
}

impl LogGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enregistre un échec ; `Some` si l'on doit le journaliser maintenant.
    pub fn report(&self, key: &str, error: &str) -> Option<Repeat> {
        self.report_at(key, error, Instant::now())
    }

    /// Variante à instant injecté (tests).
    pub fn report_at(&self, key: &str, error: &str, now: Instant) -> Option<Repeat> {
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match state.get_mut(key) {
            None => {
                state.insert(key.to_string(), (error.to_string(), now));
                Some(Repeat::First)
            }
            Some((last, logged)) if last != error => {
                *last = error.to_string();
                *logged = now;
                Some(Repeat::Changed)
            }
            Some((_, logged)) if now.saturating_duration_since(*logged) >= REMINDER => {
                *logged = now;
                Some(Repeat::Reminder)
            }
            Some(_) => None,
        }
    }

    /// Marque la clé comme rétablie ; vrai si elle était en erreur.
    pub fn clear(&self, key: &str) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.remove(key).is_some()
    }

    /// Journalise un échec : `warn` si nécessaire, `debug` sinon.
    ///
    /// La clé est `portée/message` : la portée est typiquement l'id de la cible
    /// (ou le nom du nœud), le message identifie l'endpoint.
    pub fn fail(&self, scope: impl Display, message: &str, error: &dyn Display) {
        let key = format!("{scope}/{message}");
        let key = key.as_str();
        let text = error.to_string();
        match self.report(key, &text) {
            Some(Repeat::Reminder) => warn!(key, error = %text, "{message} (toujours en échec)"),
            Some(_) => warn!(key, error = %text, "{message}"),
            None => debug!(key, error = %text, "{message}"),
        }
    }

    /// Signale un succès : `info` une seule fois si la clé était en erreur.
    pub fn ok(&self, scope: impl Display, message: &str) {
        let key = format!("{scope}/{message}");
        if self.clear(&key) {
            let key = key.as_str();
            info!(key, "{message} : de nouveau disponible");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_then_silent_then_changed() {
        let gate = LogGate::new();
        let t0 = Instant::now();
        assert_eq!(gate.report_at("k", "boom", t0), Some(Repeat::First));
        assert_eq!(gate.report_at("k", "boom", t0 + Duration::from_secs(60)), None);
        assert_eq!(
            gate.report_at("k", "autre", t0 + Duration::from_secs(120)),
            Some(Repeat::Changed)
        );
        assert_eq!(gate.report_at("k", "autre", t0 + Duration::from_secs(180)), None);
    }

    #[test]
    fn reminder_after_24h() {
        let gate = LogGate::new();
        let t0 = Instant::now();
        gate.report_at("k", "boom", t0);
        assert_eq!(gate.report_at("k", "boom", t0 + REMINDER - Duration::from_secs(1)), None);
        assert_eq!(gate.report_at("k", "boom", t0 + REMINDER), Some(Repeat::Reminder));
        assert_eq!(gate.report_at("k", "boom", t0 + REMINDER + Duration::from_secs(1)), None);
    }

    #[test]
    fn clear_reports_once_and_rearms() {
        let gate = LogGate::new();
        let t0 = Instant::now();
        assert!(!gate.clear("k"));
        gate.report_at("k", "boom", t0);
        assert!(gate.clear("k"));
        assert!(!gate.clear("k"));
        assert_eq!(gate.report_at("k", "boom", t0), Some(Repeat::First));
    }

    #[test]
    fn keys_are_independent() {
        let gate = LogGate::new();
        let t0 = Instant::now();
        assert_eq!(gate.report_at("a", "boom", t0), Some(Repeat::First));
        assert_eq!(gate.report_at("b", "boom", t0), Some(Repeat::First));
    }
}
