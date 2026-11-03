//! Règles livrées de Pi-hole, en plus de celles de `builtin_rules`
//! (blocage coupé, gravity périmé, mise à jour, diagnostic) : ce que seule la
//! lecture des résolveurs amont et des listes révèle.

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Les métriques (préfixe `dumbmonit_` compris) que ces règles visent.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_pihole_replies_servfail_24h",
    "dumbmonit_pihole_queries_24h",
    "dumbmonit_pihole_upstream_response_seconds",
    "dumbmonit_pihole_lists_download_failed",
];

/// Part de réponses SERVFAIL sur les dernières 24 h, quand il y a assez de
/// requêtes pour que le pourcentage dise quelque chose.
const SERVFAIL_QUERY: &str = "100 * dumbmonit_pihole_replies_servfail_24h \
     / dumbmonit_pihole_queries_24h \
     and on (target) dumbmonit_pihole_queries_24h >= 200";

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // Un résolveur amont qui tombe fait monter les SERVFAIL pendant que le
        // blocage, lui, continue de répondre. La fenêtre de 24 h lisse le
        // bruit : cinq pour cent y est déjà une panne de plusieurs heures.
        Rule {
            description: "More than 5% of the DNS replies of the last 24 hours are SERVFAIL: an \
                          upstream resolver is failing or unreachable."
                .to_string(),
            operator: Operator::Gt,
            threshold: 5.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pihole_servfail_high",
                "Pi-hole SERVFAIL replies",
                RuleKind::Threshold,
                SERVFAIL_QUERY,
            )
        },
        // Un résolveur récursif local (Unbound) répond en dizaines de
        // millisecondes, un amont public en moins de 100 ms : une seconde en
        // moyenne, c'est un lien ou un résolveur en difficulté.
        Rule {
            description: "An upstream resolver of Pi-hole answers in more than one second on \
                          average: every uncached lookup waits for it."
                .to_string(),
            operator: Operator::Gt,
            threshold: 1.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pihole_upstream_slow",
                "Pi-hole upstream resolver slow",
                RuleKind::Threshold,
                "dumbmonit_pihole_upstream_response_seconds",
            )
        },
        // Statut 3 : liste injoignable, la copie locale sert encore ; statut 4 :
        // aucune copie, la liste ne bloque rien. Une heure laisse passer la
        // reconstruction en cours.
        Rule {
            description: "At least one enabled blocklist of Pi-hole failed to download on the \
                          last gravity update: it is stale or empty."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pihole_lists_download_failed",
                "Pi-hole blocklist download failing",
                RuleKind::Threshold,
                "dumbmonit_pihole_lists_download_failed",
            )
        },
        // Cumul sur 24 h : zéro requête, c'est que plus aucun client ne
        // l'utilise (DNS d'un routeur changé, DHCP qui distribue un autre
        // résolveur) ou que FTL est figé.
        Rule {
            description: "Pi-hole has answered no DNS query for 24 hours: no client uses it any \
                          more, or the DNS service is stuck."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pihole_no_queries",
                "Pi-hole receives no queries",
                RuleKind::Threshold,
                "dumbmonit_pihole_queries_24h",
            )
        },
    ]
}
