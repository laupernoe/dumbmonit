//! Règles livrées de Plex Media Server au-delà de la mise à jour, qui reste
//! dans `rules.rs`.

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Métriques visées par ces règles et absentes de `PRODUITES`.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_plex_transcode_speed_min",
    "dumbmonit_plex_transcode_errors",
    "dumbmonit_plex_remote_access_failed",
    "dumbmonit_plex_signin_ok",
    "dumbmonit_plex_database_backup_enabled",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // Sous 1, le transcodeur produit moins vite que la lecture ne consomme :
        // la lecture finit par se figer. La mesure ne porte que sur les vidéos
        // en cours de lecture, hors transcodages freinés ou en pause ; elle
        // n'existe pas quand rien ne transcode.
        Rule {
            description: "Plex's transcoder is slower than real time (below 0.95x) for a video \
                          being played: playback will buffer. The server CPU or GPU is too weak \
                          for this stream."
                .to_string(),
            operator: Operator::Lt,
            threshold: 0.95,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "plex_transcoder_too_slow",
                "Plex transcoder too slow",
                RuleKind::Threshold,
                "dumbmonit_plex_transcode_speed_min",
            )
        },
        Rule {
            description: "A Plex transcode session reports an error: the stream is stuck or \
                          has already failed on the viewer's side."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "plex_transcode_error",
                "Plex transcode error",
                RuleKind::Threshold,
                "dumbmonit_plex_transcode_errors",
            )
        },
        // Seul l'état « failed » compte : un accès distant que l'on n'a jamais
        // activé n'est pas une panne.
        Rule {
            description: "Plex cannot open its remote access (port mapping failed): viewers \
                          outside your network cannot reach the server directly."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "plex_remote_access_failed",
                "Plex remote access failed",
                RuleKind::Threshold,
                "dumbmonit_plex_remote_access_failed",
            )
        },
        // Mesurée seulement sur un serveur revendiqué.
        Rule {
            description: "The Plex server is no longer signed in to its plex.tv account: sharing, \
                          remote access and update checks stop until it is signed in again."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(60 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "plex_signin_lost",
                "Plex signed out of plex.tv",
                RuleKind::Threshold,
                "dumbmonit_plex_signin_ok",
            )
        },
        // Posture, pas panne : la sauvegarde automatique de la base est coupée
        // dans les tâches de maintenance. Discret, et à n'écarter que si une
        // autre sauvegarde couvre la base.
        Rule {
            description: "Plex's scheduled database backup is turned off in its maintenance \
                          tasks: a corrupt database could not be restored from Plex itself."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(24 * 3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(30 * 24 * 3600)),
            ..base(
                "plex_database_backup_off",
                "Plex database backup off",
                RuleKind::Threshold,
                "dumbmonit_plex_database_backup_enabled",
            )
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_regles_plex_visent_des_metriques_declarees() {
        let rules = rules();
        let uids: Vec<&str> = rules.iter().map(|r| r.uid.as_str()).collect();
        assert_eq!(
            uids,
            [
                "plex_transcoder_too_slow",
                "plex_transcode_error",
                "plex_remote_access_failed",
                "plex_signin_lost",
                "plex_database_backup_off",
            ]
        );
        for rule in &rules {
            assert!(rule.builtin && rule.enabled);
            for mot in rule.query.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if mot.starts_with("dumbmonit_") {
                    assert!(PRODUCED.contains(&mot), "{}: {mot}", rule.uid);
                }
            }
        }
    }
}
