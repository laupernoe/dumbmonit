//! Règles livrées d'AdGuard Home, en plus de celles de `builtin_rules`
//! (protection coupée, DNS arrêté, amont en échec, listes périmées, mise à
//! jour) : latence, certificat du DNS chiffré, plage DHCP, trafic disparu.

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Les métriques (préfixe `dumbmonit_` compris) que ces règles visent.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_adguard_avg_processing_seconds",
    "dumbmonit_adguard_upstream_avg_seconds",
    "dumbmonit_adguard_tls_valid",
    "dumbmonit_adguard_tls_expiry_seconds",
    "dumbmonit_adguard_dhcp_pool_used_percent",
    "dumbmonit_adguard_dns_queries",
    "dumbmonit_adguard_stats_enabled",
    "dumbmonit_adguard_stats_window_seconds",
];

/// Aucune requête sur la fenêtre de statistiques, que si les statistiques sont
/// allumées et couvrent au moins un jour : sinon zéro veut dire « rien
/// d'enregistré », pas « personne n'interroge ».
const NO_QUERIES_QUERY: &str = "dumbmonit_adguard_dns_queries \
     and on (target) dumbmonit_adguard_stats_enabled == 1 \
     and on (target) dumbmonit_adguard_stats_window_seconds >= 86400";

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // Le temps moyen de traitement (filtrage compris) tient en quelques
        // millisecondes ; une demi-seconde, c'est une machine saturée ou un
        // amont qui retient chaque réponse.
        Rule {
            description: "AdGuard Home takes more than half a second on average to process a DNS \
                          query: the host is overloaded or an upstream is slow."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.5,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "adguard_processing_slow",
                "AdGuard Home slow to answer",
                RuleKind::Threshold,
                "dumbmonit_adguard_avg_processing_seconds",
            )
        },
        // Moyenne par serveur amont sur la fenêtre de statistiques : un amont
        // public répond en dizaines de millisecondes. Une seconde sur la
        // fenêtre entière est une dégradation durable.
        Rule {
            description: "An upstream DNS server of AdGuard Home answers in more than one second \
                          on average over the statistics window."
                .to_string(),
            operator: Operator::Gt,
            threshold: 1.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "adguard_upstream_slow",
                "AdGuard Home upstream slow",
                RuleKind::Threshold,
                "dumbmonit_adguard_upstream_avg_seconds",
            )
        },
        // Certificat illisible, clé qui ne va pas avec lui : les clients DoH et
        // DoT échouent. Présent seulement quand le DNS chiffré est activé.
        Rule {
            description: "The certificate or private key of AdGuard Home's encrypted DNS (DoH, \
                          DoT, DoQ) is invalid: encrypted clients cannot connect."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "adguard_tls_invalid",
                "AdGuard Home certificate invalid",
                RuleKind::Threshold,
                "dumbmonit_adguard_tls_valid",
            )
        },
        // Let's Encrypt renouvelle à 30 jours de l'échéance : quatorze jours
        // sans renouvellement, c'est que l'automatisation est cassée.
        Rule {
            description: "The certificate of AdGuard Home's encrypted DNS expires in less than \
                          14 days."
                .to_string(),
            operator: Operator::Lt,
            threshold: 14.0 * 86_400.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "adguard_tls_expiring",
                "AdGuard Home certificate expiring",
                RuleKind::Threshold,
                "dumbmonit_adguard_tls_expiry_seconds",
            )
        },
        // Au-delà de 90 % de la plage, un appareil de plus risque de ne plus
        // obtenir d'adresse ; les baux expirent, le pic d'une soirée passe.
        Rule {
            description: "More than 90% of AdGuard Home's DHCP address range is leased: new \
                          devices may get no address."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "adguard_dhcp_pool_full",
                "AdGuard Home DHCP range almost full",
                RuleKind::Threshold,
                "dumbmonit_adguard_dhcp_pool_used_percent",
            )
        },
        Rule {
            description: "AdGuard Home has answered no DNS query over its whole statistics \
                          window (a day or more): no client uses it any more."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "adguard_no_queries",
                "AdGuard Home receives no queries",
                RuleKind::Threshold,
                NO_QUERIES_QUERY,
            )
        },
    ]
}
