//! Séries `ad_*` tirées de la vue.
//!
//! La cardinalité est bornée : un contrôleur, un rôle, un groupe ou un constat
//! par série, plafonnés à soixante-quatre par famille.

use dumbmonit_proto::{MetricKind, Sample};

use super::analysis::{Observed, group_fingerprint};
use super::view::ProbeView;

/// Plafond de séries par famille étiquetée (contrôleurs, voisins).
pub const MAX_SERIES_PER_FAMILY: usize = 64;

struct Out {
    samples: Vec<Sample>,
    ts_ms: i64,
}

impl Out {
    fn gauge(&mut self, metric: &str, value: f64) -> &mut Sample {
        self.samples.push(Sample::new(metric, value, MetricKind::Gauge, self.ts_ms));
        self.samples.last_mut().expect("échantillon tout juste ajouté")
    }

    fn opt(&mut self, metric: &str, value: Option<f64>) {
        if let Some(value) = value {
            self.gauge(metric, value);
        }
    }

    fn labelled(&mut self, metric: &str, value: f64, labels: &[(&str, &str)]) {
        let sample = self.gauge(metric, value);
        for (key, label) in labels {
            sample.labels.insert((*key).to_string(), (*label).to_string());
        }
    }
}

fn flag(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

/// Échantillons d'une liaison refusée : le contrôleur répond, le compte non.
pub fn bind_failed_samples(view: &ProbeView, ts_ms: i64) -> Vec<Sample> {
    let mut out = Out { samples: Vec::new(), ts_ms };
    out.gauge("ad_bind_ok", 0.0);
    out.opt("ad_connect_seconds", view.connection.connect_seconds);
    out.labelled(
        "ad_dc_reachable",
        1.0,
        &[("dc", view.connection.host.to_ascii_lowercase().as_str())],
    );
    out.samples
}

pub fn samples(view: &ProbeView, observed: &Observed, ts_ms: i64) -> Vec<Sample> {
    let mut out = Out { samples: Vec::new(), ts_ms };
    out.gauge("ad_bind_ok", 1.0);
    out.opt("ad_connect_seconds", view.connection.connect_seconds);
    out.opt("ad_bind_seconds", view.connection.bind_seconds);
    out.gauge("ad_tls_verified", flag(view.connection.certificate_verified));

    if let Some(domain) = &view.domain {
        out.labelled(
            "ad_domain_info",
            1.0,
            &[
                ("domain", domain.dns_name.as_str()),
                ("forest", domain.forest_dns_name.as_str()),
                ("dc", domain.dc_host_name.as_deref().unwrap_or_default()),
            ],
        );
        out.opt("ad_domain_functional_level", domain.domain_level.map(|v| v as f64));
        out.opt("ad_forest_functional_level", domain.forest_level.map(|v| v as f64));
        out.opt("ad_dc_synchronized", domain.dc_synchronized.map(flag));
        out.opt("ad_dc_global_catalog_ready", domain.dc_global_catalog_ready.map(flag));
        out.opt("ad_clock_skew_seconds", domain.clock_skew_seconds.map(|v| v as f64));
        out.opt("ad_recycle_bin_enabled", domain.recycle_bin_enabled.map(flag));
    }

    if let Some(policy) = &view.policy {
        out.opt("ad_password_min_length", policy.min_password_length.map(|v| v as f64));
        out.opt("ad_password_history_length", policy.password_history_length.map(|v| v as f64));
        // 0 : les mots de passe n'expirent jamais.
        out.gauge(
            "ad_password_max_age_seconds",
            policy.max_password_age_seconds.unwrap_or(0) as f64,
        );
        out.opt("ad_password_complexity", policy.complexity_required.map(flag));
        out.opt("ad_lockout_threshold", policy.lockout_threshold.map(|v| v as f64));
        out.opt("ad_machine_account_quota", policy.machine_account_quota.map(|v| v as f64));
    }

    out.gauge("ad_dc_count", view.dcs.len() as f64);
    for dc in view.dcs.iter().take(MAX_SERIES_PER_FAMILY) {
        let name = dc.host_name.as_deref().unwrap_or(&dc.name).to_ascii_lowercase();
        out.labelled(
            "ad_dc_info",
            1.0,
            &[
                ("dc", name.as_str()),
                ("site", dc.site.as_str()),
                ("gc", if dc.global_catalog { "true" } else { "false" }),
                ("rodc", if dc.read_only { "true" } else { "false" }),
                ("os", dc.operating_system.as_deref().unwrap_or_default()),
            ],
        );
        // Un nom qui ne se résout pas ne dit rien de l'état du contrôleur :
        // pas de série, plutôt qu'une fausse alerte.
        match dc.reachability.as_str() {
            "reachable" => out.labelled("ad_dc_reachable", 1.0, &[("dc", name.as_str())]),
            "unreachable" => out.labelled("ad_dc_reachable", 0.0, &[("dc", name.as_str())]),
            _ => {}
        }
    }
    for role in &view.fsmo {
        if let Some(holder) = &role.holder {
            out.labelled("ad_fsmo_role", 1.0, &[("role", role.role.as_str()), ("holder", holder)]);
        }
    }

    out.gauge("ad_replication_readable", flag(view.replication.status == "readable"));
    for neighbor in view.replication.neighbors.iter().take(MAX_SERIES_PER_FAMILY) {
        let nc = neighbor.naming_context.as_str();
        let labels = [("source", neighbor.source.as_str()), ("naming_context", nc)];
        out.labelled(
            "ad_replication_consecutive_failures",
            f64::from(neighbor.consecutive_failures),
            &labels,
        );
        if let Some(success) = neighbor.last_success {
            out.labelled(
                "ad_replication_last_success_age_seconds",
                (ts_ms / 1_000 - success).max(0) as f64,
                &labels,
            );
        }
    }

    for group in &view.privileged_groups {
        if !group.found {
            continue;
        }
        let labels = [("group", group.name.as_str())];
        out.labelled("ad_privileged_group_members", group.member_count as f64, &labels);
        out.labelled(
            "ad_privileged_group_enabled_members",
            group.enabled_member_count as f64,
            &labels,
        );
    }
    for group in &observed.groups {
        out.labelled(
            "ad_privileged_group_fingerprint",
            f64::from(group_fingerprint(group)),
            &[("group", group.def.name)],
        );
    }

    if let Some(inv) = &view.inventory {
        out.gauge("ad_users_total", inv.users_total as f64);
        out.gauge("ad_users_enabled", inv.users_enabled as f64);
        out.gauge("ad_users_disabled", inv.users_disabled as f64);
        out.gauge("ad_users_locked_out", inv.users_locked_out as f64);
        out.gauge("ad_users_password_never_expires", inv.users_password_never_expires as f64);
        out.gauge("ad_users_stale", inv.users_stale as f64);
        out.gauge("ad_computers_total", inv.computers_total as f64);
        out.gauge("ad_computers_enabled", inv.computers_enabled as f64);
        out.gauge("ad_computers_stale", inv.computers_stale as f64);
        out.gauge("ad_groups_total", inv.groups_total as f64);
        out.gauge("ad_admincount_accounts", inv.admin_count_accounts as f64);
        out.opt(
            "ad_krbtgt_password_age_seconds",
            inv.krbtgt_password_age_seconds.map(|v| v as f64),
        );
        out.gauge("ad_laps_schema", flag(inv.laps_schema));
        out.gauge("ad_laps_computers", inv.laps_computers as f64);
        out.gauge("ad_laps_eligible_computers", inv.laps_eligible_computers as f64);
        out.gauge("ad_inventory_age_seconds", (ts_ms / 1_000 - inv.refreshed_at).max(0) as f64);
    }

    for finding in &view.findings {
        out.labelled(
            "ad_finding_count",
            finding.count as f64,
            &[
                ("finding", finding.id.as_str()),
                ("severity", finding.severity.as_str()),
                ("title", finding.title.as_str()),
            ],
        );
    }
    out.gauge("ad_scrape_errors", view.errors.len() as f64);
    out.samples
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activedirectory::view::{ConnectionView, DcView, Finding, FindingSeverity};

    fn value(samples: &[Sample], metric: &str, label: Option<(&str, &str)>) -> Option<f64> {
        samples
            .iter()
            .find(|s| {
                s.metric == metric
                    && label.is_none_or(|(k, v)| s.labels.get(k).map(String::as_str) == Some(v))
            })
            .map(|s| s.value)
    }

    #[test]
    fn un_nom_non_resolu_ne_produit_pas_de_serie_de_joignabilite() {
        let view = ProbeView {
            dcs: vec![
                DcView {
                    name: "DC1".into(),
                    reachability: "reachable".into(),
                    ..DcView::default()
                },
                DcView {
                    name: "DC2".into(),
                    host_name: Some("DC2.corp.lan".into()),
                    reachability: "unreachable".into(),
                    ..DcView::default()
                },
                DcView {
                    name: "DC3".into(),
                    reachability: "unresolved".into(),
                    ..DcView::default()
                },
            ],
            findings: vec![Finding {
                id: "asrep_roastable_users".into(),
                severity: FindingSeverity::High,
                category: "kerberos".into(),
                title: String::new(),
                detail: String::new(),
                count: 3,
                samples: Vec::new(),
            }],
            ..ProbeView::default()
        };
        let samples = samples(&view, &Observed::default(), 1_000_000);
        assert_eq!(value(&samples, "ad_dc_reachable", Some(("dc", "dc1"))), Some(1.0));
        assert_eq!(value(&samples, "ad_dc_reachable", Some(("dc", "dc2.corp.lan"))), Some(0.0));
        assert_eq!(value(&samples, "ad_dc_reachable", Some(("dc", "dc3"))), None);
        assert_eq!(value(&samples, "ad_dc_count", None), Some(3.0));
        assert_eq!(value(&samples, "ad_finding_count", Some(("finding", "asrep_roastable_users"))), Some(3.0));
        assert_eq!(value(&samples, "ad_bind_ok", None), Some(1.0));
    }

    #[test]
    fn une_liaison_refusee_garde_le_controleur_joignable() {
        let view = ProbeView {
            connection: ConnectionView {
                host: "DC1.corp.lan".into(),
                connect_seconds: Some(0.01),
                ..ConnectionView::default()
            },
            ..ProbeView::default()
        };
        let samples = bind_failed_samples(&view, 0);
        assert_eq!(value(&samples, "ad_bind_ok", None), Some(0.0));
        assert_eq!(value(&samples, "ad_dc_reachable", Some(("dc", "dc1.corp.lan"))), Some(1.0));
    }
}
