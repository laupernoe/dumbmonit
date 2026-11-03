//! Abonnement du serveur : `GET /nodes/localhost/subscription`.
//!
//! Un abonnement expiré ou invalide ne coupe rien, mais le dépôt d'entreprise
//! cesse de répondre : les mises à jour de sécurité n'arrivent plus, sans autre
//! symptôme. Un serveur sans abonnement (`notfound`) est le cas ordinaire d'un
//! homelab, et n'est pas un défaut.

use dumbmonit_proto::Sample;
use serde::Deserialize;

use super::metrics::gauge;

/// Réponse de `GET /nodes/localhost/subscription`.
#[derive(Debug, Default, Deserialize)]
pub struct Subscription {
    /// `active`, `notfound`, `new`, `invalid`, `expired`, `suspended`.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub nextduedate: Option<String>,
}

/// `node_subscription_active` vaut 1 pour un abonnement en cours ; l'état lui-même
/// voyage en étiquette d'une série `_info`, que les règles filtrent par valeur.
pub fn subscription_samples(subscription: &Subscription, ts_ms: i64) -> Vec<Sample> {
    let status = subscription.status.clone().unwrap_or_else(|| "unknown".to_string());
    let active = status.eq_ignore_ascii_case("active");
    vec![
        gauge("node_subscription_active", if active { 1.0 } else { 0.0 }, ts_ms),
        gauge("node_subscription_info", 1.0, ts_ms)
            .with_label("status", status.to_ascii_lowercase())
            .with_label("level", subscription.level.clone().unwrap_or_default())
            .with_label("next_due", subscription.nextduedate.clone().unwrap_or_default()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_abonnement_en_cours_est_actif() {
        let subscription: Subscription = serde_json::from_str(
            r#"{"status":"active","level":"c","nextduedate":"2027-03-01","productname":"PBS"}"#,
        )
        .unwrap();
        let samples = subscription_samples(&subscription, 1000);
        assert_eq!(samples[0].metric, "pbs_node_subscription_active");
        assert_eq!(samples[0].value, 1.0);
        assert_eq!(samples[1].labels["status"], "active");
        assert_eq!(samples[1].labels["next_due"], "2027-03-01");
    }

    #[test]
    fn un_serveur_sans_abonnement_n_est_pas_actif() {
        let subscription: Subscription = serde_json::from_str(r#"{"status":"NotFound"}"#).unwrap();
        let samples = subscription_samples(&subscription, 1000);
        assert_eq!(samples[0].value, 0.0);
        assert_eq!(samples[1].labels["status"], "notfound");
    }

    #[test]
    fn une_reponse_vide_donne_un_etat_inconnu() {
        let samples = subscription_samples(&Subscription::default(), 1000);
        assert_eq!(samples[1].labels["status"], "unknown");
    }
}
