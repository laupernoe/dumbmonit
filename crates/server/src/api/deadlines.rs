//! La frise des échéances de l'instance : domaines, certificats, secrets
//! d'application Entra, produits Infomaniak, clés Tailscale, licences.
//!
//! `GET /api/deadlines` lit le dernier point de chaque famille `*_expiry_*`
//! déjà collectée (voir [`FAMILIES`]) et rend une liste triée de la plus proche
//! à la plus lointaine, les périmés de moins de trente jours en tête. Rien n'est
//! réinterrogé : la date d'expiration est l'horodatage du dernier point plus le
//! temps restant qu'il portait, donc elle reste juste même si la sonde n'a pas
//! tourné depuis. Les alertes à 30, 14 et 7 jours restent celles de chaque
//! famille (`alerting/rules.rs`) ; l'interface colorie les mêmes paliers.
//!
//! Paramètres : `target` (une seule cible, pour sa page de détail) et
//! `max_days` (horizon, 366 par défaut).

use std::collections::{BTreeMap, HashMap};

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::ApiResult;
use crate::auth::teams::TeamAccess;
use crate::db;
use crate::state::AppState;
use crate::tsdb::InstantSeries;

const DAY: f64 = 86_400.0;
/// Fenêtre de lecture : une sonde quotidienne ou deux jours de panne passent.
const LOOKBACK_SECONDS: u64 = 3 * 86_400;
/// Écart toléré entre les séries d'une même sonde d'une même famille ; au-delà,
/// la série appartient à une sonde plus ancienne (certificat remplacé).
const SAME_PROBE_SECONDS: f64 = 120.0;
/// Un périmé plus ancien que cela n'est plus une échéance, c'est du ménage.
const EXPIRED_KEEP_DAYS: f64 = -30.0;
const DEFAULT_MAX_DAYS: f64 = 366.0;
const MAX_MAX_DAYS: f64 = 3650.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Unit {
    Days,
    Seconds,
}

struct Family {
    /// Nom de la série, sans le préfixe `dumbmonit_`.
    metric: &'static str,
    unit: Unit,
    /// `domain`, `certificate`, `credential`, `product`, `key` ou `license`.
    kind: &'static str,
    /// Étiquettes qui nomment l'objet, jointes par « · ». Aucune : le nom de la
    /// cible.
    what: &'static [&'static str],
}

/// Les familles d'échéances que les collecteurs produisent aujourd'hui.
const FAMILIES: &[Family] = &[
    Family { metric: "domain_expiry_days", unit: Unit::Days, kind: "domain", what: &[] },
    Family {
        metric: "probe_ssl_cert_expiry_days",
        unit: Unit::Days,
        kind: "certificate",
        what: &[],
    },
    Family {
        metric: "npm_cert_expiry_days",
        unit: Unit::Days,
        kind: "certificate",
        what: &["proxy_host"],
    },
    Family {
        metric: "traefik_cert_expiry_days",
        unit: Unit::Days,
        kind: "certificate",
        what: &["cert"],
    },
    Family {
        metric: "cloudflare_certificate_expiry_seconds",
        unit: Unit::Seconds,
        kind: "certificate",
        what: &["host", "zone"],
    },
    Family {
        metric: "proxmox_node_certificate_expiry_days",
        unit: Unit::Days,
        kind: "certificate",
        what: &["node", "filename"],
    },
    Family {
        metric: "pdm_node_certificate_expiry_days",
        unit: Unit::Days,
        kind: "certificate",
        what: &["file"],
    },
    Family {
        metric: "pbs_node_certificate_expires_seconds",
        unit: Unit::Seconds,
        kind: "certificate",
        what: &["certificate"],
    },
    Family {
        metric: "pmg_certificate_expires_in_seconds",
        unit: Unit::Seconds,
        kind: "certificate",
        what: &["node", "certificate"],
    },
    Family {
        metric: "entra_credential_expiry_seconds",
        unit: Unit::Seconds,
        kind: "credential",
        what: &["app", "kind"],
    },
    Family {
        metric: "infomaniak_product_expiry_seconds",
        unit: Unit::Seconds,
        kind: "product",
        what: &["product"],
    },
    Family {
        metric: "ovh_service_expiry_seconds",
        unit: Unit::Seconds,
        kind: "product",
        what: &["service"],
    },
    Family {
        metric: "tailscale_device_key_expiry_seconds",
        unit: Unit::Seconds,
        kind: "key",
        what: &["device"],
    },
    Family {
        metric: "fortigate_license_expiry_seconds",
        unit: Unit::Seconds,
        kind: "license",
        what: &["license"],
    },
    Family {
        metric: "meraki_license_expiry_seconds",
        unit: Unit::Seconds,
        kind: "license",
        what: &[],
    },
    Family {
        metric: "paloalto_license_expiry_seconds",
        unit: Unit::Seconds,
        kind: "license",
        what: &["license"],
    },
    Family {
        metric: "veeam_license_expiry_seconds",
        unit: Unit::Seconds,
        kind: "license",
        what: &[],
    },
    Family {
        metric: "gitlab_license_expiry_seconds",
        unit: Unit::Seconds,
        kind: "license",
        what: &[],
    },
];

pub fn routes() -> Router<AppState> {
    Router::new().route("/deadlines", get(list))
}

#[derive(Debug, Deserialize)]
pub struct DeadlinesQuery {
    pub target: Option<i64>,
    pub max_days: Option<f64>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Deadline {
    pub target: i64,
    pub target_name: String,
    pub target_kind: String,
    /// `domain`, `certificate`, `credential`, `product`, `key`, `license`.
    pub kind: String,
    /// L'objet qui expire : domaine, nom d'hôte, application, appareil…
    pub what: String,
    /// UTC, sans suffixe.
    pub expires_at: String,
    /// Négatif : déjà expiré.
    pub days_left: f64,
}

/// Une cible telle que la route en a besoin pour nommer ses échéances.
pub struct TargetInfo {
    pub name: String,
    pub kind: String,
}

async fn list(
    TeamAccess(access): TeamAccess,
    State(state): State<AppState>,
    Query(query): Query<DeadlinesQuery>,
) -> ApiResult<Json<Vec<Deadline>>> {
    let visible = access.visible_targets(&state.pool).await?;
    let targets: HashMap<i64, TargetInfo> = db::targets::list(&state.pool, &state.cipher)
        .await?
        .into_iter()
        .filter(|t| visible.as_ref().is_none_or(|v| v.contains(&t.id)))
        .filter(|t| query.target.is_none_or(|only| only == t.id))
        .map(|t| (t.id, TargetInfo { name: t.name, kind: t.kind }))
        .collect();
    if targets.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let names: Vec<String> =
        FAMILIES.iter().map(|f| format!("{}{}", crate::tsdb::METRIC_PREFIX, f.metric)).collect();
    let mut selector = format!(r#"{{__name__=~"{}""#, names.join("|"));
    if let Some(only) = query.target {
        selector.push_str(&format!(r#", target="{only}""#));
    }
    selector.push('}');
    let last = format!("last_over_time({selector}[{LOOKBACK_SECONDS}s]) keep_metric_names");
    let at = format!("tlast_over_time({selector}[{LOOKBACK_SECONDS}s]) keep_metric_names");
    let (last, at) = tokio::try_join!(state.victoria.query(&last), state.victoria.query(&at))?;
    let max_days = query.max_days.filter(|d| d.is_finite()).unwrap_or(DEFAULT_MAX_DAYS);
    let now = chrono::Utc::now().timestamp() as f64;
    Ok(Json(build(&last, &at, &targets, now, max_days.clamp(1.0, MAX_MAX_DAYS))))
}

/// Identité d'une série : son nom et toutes ses étiquettes.
fn key(metric: &BTreeMap<String, String>) -> Vec<(&str, &str)> {
    metric.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
}

/// Agrège les séries en échéances triées. `last` porte les valeurs, `at` les
/// horodatages de leurs derniers points ; `now` est en secondes Unix.
pub fn build(
    last: &[InstantSeries],
    at: &[InstantSeries],
    targets: &HashMap<i64, TargetInfo>,
    now: f64,
    max_days: f64,
) -> Vec<Deadline> {
    let stamps: HashMap<Vec<(&str, &str)>, f64> = at
        .iter()
        .filter_map(|s| s.value.1.parse::<f64>().ok().map(|t| (key(&s.metric), t)))
        .collect();

    struct Row<'a> {
        family: &'static Family,
        target: i64,
        labels: &'a BTreeMap<String, String>,
        value: f64,
        sampled: f64,
    }
    let mut rows: Vec<Row> = Vec::new();
    for series in last {
        let Some(name) = series.metric.get("__name__") else { continue };
        let Some(family) = FAMILIES
            .iter()
            .find(|f| name.strip_prefix(crate::tsdb::METRIC_PREFIX) == Some(f.metric))
        else {
            continue;
        };
        let Some(target) = series.metric.get("target").and_then(|t| t.parse::<i64>().ok()) else {
            continue;
        };
        let Ok(value) = series.value.1.parse::<f64>() else { continue };
        if !value.is_finite() {
            continue;
        }
        let Some(&sampled) = stamps.get(&key(&series.metric)) else { continue };
        rows.push(Row { family, target, labels: &series.metric, value, sampled });
    }

    // Par cible et famille, seule la dernière sonde compte.
    let mut newest: HashMap<(i64, &str), f64> = HashMap::new();
    for row in &rows {
        let slot = newest.entry((row.target, row.family.metric)).or_insert(f64::MIN);
        *slot = slot.max(row.sampled);
    }

    let mut out: Vec<Deadline> = Vec::new();
    for row in rows {
        let Some(target) = targets.get(&row.target) else { continue };
        if row.sampled < newest[&(row.target, row.family.metric)] - SAME_PROBE_SECONDS {
            continue;
        }
        let left = match row.family.unit {
            Unit::Days => row.value * DAY,
            Unit::Seconds => row.value,
        };
        let expires = row.sampled + left;
        let days_left = (expires - now) / DAY;
        if days_left > max_days || days_left < EXPIRED_KEEP_DAYS {
            continue;
        }
        let parts: Vec<&str> = row
            .family
            .what
            .iter()
            .filter_map(|l| row.labels.get(*l).map(String::as_str))
            .filter(|v| !v.is_empty())
            .collect();
        let what = if parts.is_empty() { target.name.clone() } else { parts.join(" · ") };
        let Some(date) = chrono::DateTime::from_timestamp(expires as i64, 0) else { continue };
        out.push(Deadline {
            target: row.target,
            target_name: target.name.clone(),
            target_kind: target.kind.clone(),
            kind: row.family.kind.to_string(),
            what,
            expires_at: date.format("%Y-%m-%dT%H:%M:%S").to_string(),
            days_left: (days_left * 100.0).round() / 100.0,
        });
    }
    out.sort_by(|a, b| {
        a.days_left
            .total_cmp(&b.days_left)
            .then_with(|| a.target.cmp(&b.target))
            .then_with(|| a.what.cmp(&b.what))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: f64 = 1_790_000_000.0;
    const INTRANET: [(&str, &str); 2] = [("app", "Intranet"), ("kind", "secret")];
    const VIEUX: [(&str, &str); 2] = [("app", "Vieux"), ("kind", "secret")];

    fn series(metric: &str, target: i64, extra: &[(&str, &str)], value: f64) -> InstantSeries {
        let mut labels = BTreeMap::new();
        labels.insert("__name__".to_string(), format!("dumbmonit_{metric}"));
        labels.insert("target".to_string(), target.to_string());
        for (k, v) in extra {
            labels.insert((*k).to_string(), (*v).to_string());
        }
        InstantSeries { metric: labels, value: (NOW, value.to_string()) }
    }

    fn targets() -> HashMap<i64, TargetInfo> {
        HashMap::from([
            (1, TargetInfo { name: "example.com".into(), kind: "domain".into() }),
            (2, TargetInfo { name: "Entra".into(), kind: "entra".into() }),
        ])
    }

    #[test]
    fn trie_du_plus_proche_et_date_depuis_le_dernier_point() {
        // Le point date d'un jour : 10 j restants alors valent 9 j aujourd'hui.
        let sampled = NOW - DAY;
        let last = vec![
            series("domain_expiry_days", 1, &[], 10.0),
            series("entra_credential_expiry_seconds", 2, &INTRANET, 3.0 * DAY),
            series("entra_credential_expiry_seconds", 2, &VIEUX, -40.0 * DAY),
        ];
        let at: Vec<InstantSeries> = last
            .iter()
            .map(|s| InstantSeries { metric: s.metric.clone(), value: (NOW, sampled.to_string()) })
            .collect();
        let out = build(&last, &at, &targets(), NOW, 366.0);
        assert_eq!(out.len(), 2, "le secret périmé depuis 40 jours est écarté");
        assert_eq!(out[0].what, "Intranet · secret");
        assert_eq!(out[0].kind, "credential");
        assert!((out[0].days_left - 2.0).abs() < 0.01, "{}", out[0].days_left);
        assert_eq!(out[1].what, "example.com");
        assert_eq!(out[1].kind, "domain");
        assert!((out[1].days_left - 9.0).abs() < 0.01);
        assert_eq!(out[1].expires_at.len(), 19);
        assert!(!out[1].expires_at.ends_with('Z'));
    }

    #[test]
    fn une_ancienne_sonde_et_les_cibles_inconnues_sont_ignorees() {
        let fresh = series("npm_cert_expiry_days", 1, &[("proxy_host", "a.example")], 20.0);
        let stale = series("npm_cert_expiry_days", 1, &[("proxy_host", "old.example")], 5.0);
        let other = series("domain_expiry_days", 99, &[], 5.0);
        let last = vec![fresh, stale, other];
        let stamps = [NOW, NOW - 3600.0, NOW];
        let at: Vec<InstantSeries> = last
            .iter()
            .zip(stamps)
            .map(|(s, t)| InstantSeries { metric: s.metric.clone(), value: (NOW, t.to_string()) })
            .collect();
        let out = build(&last, &at, &targets(), NOW, 366.0);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].what, "a.example");
    }

    #[test]
    fn les_services_ovh_sont_des_echeances() {
        let last = vec![series(
            "ovh_service_expiry_seconds",
            2,
            &[("service", "vps-aaa111.vps.ovh.net"), ("family", "vps")],
            5.0 * DAY,
        )];
        let metric = last[0].metric.clone();
        let at = vec![InstantSeries { metric, value: (NOW, NOW.to_string()) }];
        let out = build(&last, &at, &targets(), NOW, 366.0);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, "product");
        assert_eq!(out[0].what, "vps-aaa111.vps.ovh.net");
        assert!((out[0].days_left - 5.0).abs() < 0.01);
    }

    #[test]
    fn l_horizon_coupe_les_echeances_lointaines() {
        let last = vec![series("domain_expiry_days", 1, &[], 200.0)];
        let metric = last[0].metric.clone();
        let at = vec![InstantSeries { metric, value: (NOW, NOW.to_string()) }];
        assert!(build(&last, &at, &targets(), NOW, 90.0).is_empty());
        assert_eq!(build(&last, &at, &targets(), NOW, 366.0).len(), 1);
    }
}
