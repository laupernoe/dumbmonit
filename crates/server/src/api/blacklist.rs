//! Le panneau des listes noires DNS (`collectors/blacklist`) : réputation d'un
//! serveur mail et liste noire d'une plage d'adresses.
//!
//! `GET /targets/{id}/blacklist` reconstruit le dernier passage à partir des
//! séries `dumbmonit_blacklist_*` de la cible, sans jamais réinterroger les
//! listes. Les liens de consultation et de suppression viennent du catalogue
//! du collecteur (`blacklist/lists.rs`), pas des séries.
//!
//! Un passage dont les étiquettes changent (un motif, un code) laisse ses
//! anciennes séries dans la fenêtre de lecture : seules sont retenues celles de
//! la dernière sonde, repérées par l'horodatage de leur dernier point.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use dumbmonit_collectors::blacklist::lists::{self, Subject};
use dumbmonit_proto::TargetId;
use serde::Serialize;

use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;
use crate::tsdb::InstantSeries;

pub const KINDS: [&str; 2] = ["mail_blacklist", "ip_blacklist"];
const PREFIX: &str = "dumbmonit_blacklist_";
/// Écart toléré entre les points d'une même sonde.
const SAME_PROBE_SECONDS: f64 = 5.0;

pub fn routes() -> Router<AppState> {
    Router::new().route("/targets/{id}/blacklist", get(overview))
}

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct BlacklistView {
    pub kind: String,
    /// Secondes Unix du dernier point ; `None` avant la première sonde.
    pub sampled_at: Option<f64>,
    pub scan_running: bool,
    pub scan_age_seconds: Option<f64>,
    pub scan_duration_seconds: Option<f64>,
    pub addresses_checked: Option<f64>,
    pub addresses_listed: Option<f64>,
    pub listed_lists: Option<f64>,
    pub listed_major_lists: Option<f64>,
    pub unknown_checks: Option<f64>,
    pub domain_listed: Option<bool>,
    pub lists: Vec<ListRow>,
    /// Inscriptions et vérifications sans réponse en tête.
    pub checks: Vec<CheckRow>,
    pub addresses: Vec<AddressRow>,
    pub auth: Option<AuthRow>,
    /// Les messageries sans liste publique ; vide pour une plage d'adresses.
    pub providers: Vec<ProviderRow>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ListRow {
    pub id: String,
    pub name: String,
    pub zone: String,
    /// `ip` ou `domain`.
    pub subject: &'static str,
    pub major: bool,
    /// Ajoutée par l'utilisateur (absente du catalogue).
    pub custom: bool,
    pub checked: f64,
    pub listed: f64,
    pub unknown: f64,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CheckRow {
    pub list: String,
    pub list_name: String,
    pub major: bool,
    pub subject: String,
    /// `listed`, `clean`, `unknown`, `refused`, `not_applicable`.
    pub state: String,
    pub codes: Vec<String>,
    pub meaning: Option<String>,
    /// Motif `TXT` d'une inscription, ou cause d'une réponse manquante.
    pub reason: Option<String>,
    /// Spamhaus refuse ce résolveur (public ou qui y transfère).
    pub public_resolver_refused: bool,
    pub lookup_url: Option<String>,
    pub delist_url: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct AddressRow {
    pub address: String,
    /// `input`, `mx`, `domain`, `extra`.
    pub source: String,
    pub mx: Option<String>,
    pub ptr: Option<String>,
    /// Le DNS inverse revient vers l'adresse ; `None` sans réponse.
    pub fcrdns: Option<bool>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct AuthRow {
    pub mx_hosts: Option<f64>,
    pub spf_records: Option<f64>,
    pub dmarc_records: Option<f64>,
    pub dmarc_policy: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ProviderRow {
    pub id: &'static str,
    pub name: &'static str,
    pub links: Vec<LinkRow>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct LinkRow {
    /// Nature du lien, traduite par l'interface (`postmaster`, `snds`…).
    pub kind: &'static str,
    pub url: &'static str,
}

async fn overview(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<BlacklistView>> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Monitor {id} not found.")))?;
    if !KINDS.contains(&target.kind.as_str()) {
        return Err(ApiError::NotFound(format!("Monitor {id} is not a blocklist check.")));
    }
    let lookback = (target.interval.as_secs() * 2).max(3600);
    let selector = format!(r#"{{__name__=~"{PREFIX}.*", target="{id}"}}"#);
    let last = format!("last_over_time({selector}[{lookback}s]) keep_metric_names");
    let at = format!("tlast_over_time({selector}[{lookback}s]) keep_metric_names");
    let (last, at) = tokio::try_join!(state.victoria.query(&last), state.victoria.query(&at))?;
    Ok(Json(build_view(&target.kind, &latest(&last, &at))))
}

/// Une série retenue : ses étiquettes (nom compris) et sa valeur.
type Point = (BTreeMap<String, String>, f64);

/// Les séries de la dernière sonde seulement, noms raccourcis
/// (`blacklist_check` → `check`), et l'heure de cette sonde.
pub fn latest(last: &[InstantSeries], at: &[InstantSeries]) -> (Vec<Point>, Option<f64>) {
    let when: Vec<(&BTreeMap<String, String>, f64)> =
        at.iter().filter_map(|s| s.value.1.parse().ok().map(|t| (&s.metric, t))).collect();
    let Some(newest) = when.iter().map(|(_, t)| *t).reduce(f64::max) else {
        return (Vec::new(), None);
    };
    let points = last
        .iter()
        .filter(|s| when.iter().any(|(m, t)| **m == s.metric && *t >= newest - SAME_PROBE_SECONDS))
        .filter_map(|s| {
            let value: f64 = s.value.1.parse().ok()?;
            let mut labels = s.metric.clone();
            if let Some(name) = labels.get_mut("__name__")
                && let Some(short) = name.strip_prefix(PREFIX)
            {
                *name = short.to_string();
            }
            Some((labels, value))
        })
        .collect();
    (points, Some(newest))
}

fn named<'a>(points: &'a [Point], name: &'a str) -> impl Iterator<Item = &'a Point> + 'a {
    points
        .iter()
        .filter(move |(labels, _)| labels.get("__name__").map(String::as_str) == Some(name))
}

fn one(points: &[Point], name: &str) -> Option<f64> {
    named(points, name).next().map(|(_, v)| *v)
}

fn label(labels: &BTreeMap<String, String>, key: &str) -> Option<String> {
    labels.get(key).filter(|v| !v.is_empty()).cloned()
}

fn per_list(points: &[Point], name: &str, list: &str) -> Option<f64> {
    named(points, name).find(|(l, _)| l.get("list").map(String::as_str) == Some(list)).map(|p| p.1)
}

/// Rang d'une liste : l'ordre du catalogue, puis les listes personnalisées.
fn list_rank(id: &str) -> usize {
    lists::LISTS.iter().position(|l| l.id == id).unwrap_or(usize::MAX)
}

pub fn build_view(kind: &str, (points, sampled_at): &(Vec<Point>, Option<f64>)) -> BlacklistView {
    let mut view = BlacklistView { kind: kind.to_string(), ..Default::default() };
    if points.is_empty() {
        return view;
    }
    let points = points.as_slice();
    view.sampled_at = *sampled_at;
    view.scan_running = one(points, "scan_running") == Some(1.0);
    view.scan_age_seconds = one(points, "scan_age_seconds");
    view.scan_duration_seconds = one(points, "scan_duration_seconds");
    view.addresses_checked = one(points, "addresses_checked");
    view.addresses_listed = one(points, "addresses_listed");
    view.listed_lists = one(points, "listed_lists");
    view.listed_major_lists = one(points, "listed_major_lists");
    view.unknown_checks = one(points, "unknown_checks");
    view.domain_listed = one(points, "domain_listed").map(|v| v >= 1.0);

    // Les listes réellement interrogées : celles du catalogue, et les listes
    // personnalisées que seules les séries `list_info` décrivent.
    for (labels, checked) in named(points, "list_checked") {
        let Some(id) = label(labels, "list") else { continue };
        let known = lists::by_id(&id);
        let info =
            named(points, "list_info").find(|(l, _)| label(l, "list").as_deref() == Some(&id));
        let from_info = |key: &str| info.and_then(|(l, _)| label(l, key));
        view.lists.push(ListRow {
            name: known
                .map(|l| l.name.to_string())
                .or_else(|| from_info("name"))
                .unwrap_or_else(|| id.clone()),
            zone: known
                .map(|l| l.zone.to_string())
                .or_else(|| from_info("zone"))
                .unwrap_or_default(),
            subject: match known.map(|l| l.subject) {
                Some(Subject::Domain) => "domain",
                Some(Subject::Ip) => "ip",
                None if from_info("subject").as_deref() == Some("domain") => "domain",
                None => "ip",
            },
            major: known.map_or_else(|| from_info("major").as_deref() == Some("true"), |l| l.major),
            custom: known.is_none(),
            checked: *checked,
            listed: per_list(points, "list_listed", &id).unwrap_or(0.0),
            unknown: per_list(points, "list_unknown", &id).unwrap_or(0.0),
            id,
        });
    }
    view.lists.sort_by_key(|row| (list_rank(&row.id), row.id.clone()));

    for (labels, _) in named(points, "check") {
        let (Some(id), Some(subject)) = (label(labels, "list"), label(labels, "subject")) else {
            continue;
        };
        let list = lists::by_id(&id);
        let state = label(labels, "state").unwrap_or_else(|| "unknown".to_string());
        let links = state == "listed";
        view.checks.push(CheckRow {
            list_name: list.map_or_else(|| id.clone(), |l| l.name.to_string()),
            major: list.is_some_and(|l| l.major),
            lookup_url: list.filter(|_| links).map(|l| l.lookup_for(&subject)),
            delist_url: list.filter(|_| links).map(|l| l.delist_for(&subject)),
            codes: label(labels, "code")
                .map(|c| c.split(',').map(str::to_string).collect())
                .unwrap_or_default(),
            meaning: label(labels, "meaning"),
            public_resolver_refused: state == "refused"
                && label(labels, "reason").as_deref() == Some(lists::PUBLIC_RESOLVER_REFUSAL),
            reason: label(labels, "reason"),
            list: id,
            subject,
            state,
        });
    }
    let rank = |state: &str| match state {
        "listed" => 0,
        "refused" | "unknown" => 1,
        "clean" => 2,
        _ => 3,
    };
    let order = list_rank;
    view.checks.sort_by(|a, b| {
        rank(&a.state)
            .cmp(&rank(&b.state))
            .then_with(|| order(&a.list).cmp(&order(&b.list)))
            .then_with(|| a.subject.cmp(&b.subject))
    });

    for (labels, _) in named(points, "address_info") {
        let Some(address) = label(labels, "address") else { continue };
        let ptr = named(points, "ptr_ok")
            .find(|(l, _)| l.get("address") == Some(&address))
            .map(|(l, v)| (label(l, "ptr"), *v >= 1.0));
        view.addresses.push(AddressRow {
            source: label(labels, "source").unwrap_or_default(),
            mx: label(labels, "mx"),
            ptr: ptr.as_ref().and_then(|(name, _)| name.clone()),
            fcrdns: ptr.map(|(_, ok)| ok),
            address,
        });
    }

    let mx_hosts = one(points, "mx_hosts");
    if mx_hosts.is_some() {
        view.auth = Some(AuthRow {
            mx_hosts,
            spf_records: one(points, "spf_records"),
            dmarc_records: one(points, "dmarc_records"),
            dmarc_policy: named(points, "dmarc_policy_info").find_map(|(l, _)| label(l, "policy")),
        });
    }
    if kind == "mail_blacklist" {
        view.providers = lists::PROVIDERS
            .iter()
            .map(|p| ProviderRow {
                id: p.id,
                name: p.name,
                links: p.links.iter().map(|&(kind, url)| LinkRow { kind, url }).collect(),
            })
            .collect();
    }
    view
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serie(
        name: &str,
        value: f64,
        at: f64,
        labels: &[(&str, &str)],
    ) -> (InstantSeries, InstantSeries) {
        let mut metric: BTreeMap<String, String> =
            labels.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        metric.insert("__name__".into(), format!("{PREFIX}{name}"));
        metric.insert("target".into(), "9".into());
        (
            InstantSeries { metric: metric.clone(), value: (at, value.to_string()) },
            InstantSeries { metric, value: (at, at.to_string()) },
        )
    }

    #[test]
    fn sans_mesure_la_vue_est_vide() {
        let view = build_view("mail_blacklist", &(Vec::new(), None));
        assert!(view.sampled_at.is_none());
        assert!(view.providers.is_empty());
    }

    #[test]
    fn les_series_perimees_sont_ecartees_et_les_liens_ajoutes() {
        let now = 1_790_812_800.0;
        let series = [
            serie("listed_major_lists", 1.0, now, &[]),
            serie("list_checked", 2.0, now, &[("list", "spamcop")]),
            serie("list_listed", 1.0, now, &[("list", "spamcop")]),
            serie(
                "check",
                0.0,
                now,
                &[("list", "spamhaus_zen"), ("subject", "8.8.8.1"), ("state", "clean")],
            ),
            serie(
                "check",
                1.0,
                now,
                &[
                    ("list", "spamcop"),
                    ("subject", "8.8.8.1"),
                    ("state", "listed"),
                    ("code", "127.0.0.2"),
                ],
            ),
            // Le passage précédent : un autre motif, une heure plus tôt.
            serie(
                "check",
                1.0,
                now - 3600.0,
                &[("list", "barracuda"), ("subject", "8.8.8.1"), ("state", "listed")],
            ),
            serie(
                "address_info",
                1.0,
                now,
                &[("address", "8.8.8.1"), ("source", "mx"), ("mx", "mx1.example.net")],
            ),
            serie("ptr_ok", 0.0, now, &[("address", "8.8.8.1"), ("ptr", "")]),
            serie("mx_hosts", 1.0, now, &[]),
            serie("spf_records", 0.0, now, &[]),
        ];
        let (last, at): (Vec<InstantSeries>, Vec<InstantSeries>) = series.into_iter().unzip();
        let view = build_view("mail_blacklist", &latest(&last, &at));

        assert_eq!(view.listed_major_lists, Some(1.0));
        assert_eq!(view.checks.len(), 2, "la série d'une heure n'est plus lue");
        let listed = &view.checks[0];
        assert_eq!(listed.list, "spamcop");
        assert_eq!(listed.codes, ["127.0.0.2"]);
        assert_eq!(listed.delist_url.as_deref(), Some("https://www.spamcop.net/bl.shtml?8.8.8.1"));
        assert_eq!(view.checks[1].delist_url, None, "pas de lien pour une adresse propre");
        assert_eq!(view.lists[0].id, "spamcop");
        assert_eq!(view.lists[0].listed, 1.0);
        assert_eq!(view.addresses[0].fcrdns, Some(false));
        assert_eq!(view.addresses[0].ptr, None);
        assert_eq!(view.addresses[0].mx.as_deref(), Some("mx1.example.net"));
        assert_eq!(view.auth.as_ref().unwrap().spf_records, Some(0.0));
        assert_eq!(view.providers.len(), 3);
        assert!(build_view("ip_blacklist", &latest(&last, &at)).providers.is_empty());
    }

    #[test]
    fn les_listes_personnalisees_et_le_refus_de_spamhaus_sont_montres() {
        let now = 1_790_812_800.0;
        let custom = "custom:dnsbl.example.org";
        let series = [
            serie("list_checked", 1.0, now, &[("list", custom)]),
            serie(
                "list_info",
                1.0,
                now,
                &[
                    ("list", custom),
                    ("name", "dnsbl.example.org"),
                    ("zone", "dnsbl.example.org"),
                    ("subject", "ip"),
                    ("major", "true"),
                ],
            ),
            serie("list_listed", 1.0, now, &[("list", custom)]),
            serie("list_checked", 1.0, now, &[("list", "spamhaus_zen")]),
            serie("list_unknown", 1.0, now, &[("list", "spamhaus_zen")]),
            serie(
                "check",
                -2.0,
                now,
                &[
                    ("list", "spamhaus_zen"),
                    ("subject", "8.8.8.1"),
                    ("state", "refused"),
                    ("reason", lists::PUBLIC_RESOLVER_REFUSAL),
                ],
            ),
            serie(
                "check",
                1.0,
                now,
                &[("list", custom), ("subject", "8.8.8.1"), ("state", "listed")],
            ),
        ];
        let (last, at): (Vec<InstantSeries>, Vec<InstantSeries>) = series.into_iter().unzip();
        let view = build_view("ip_blacklist", &latest(&last, &at));

        // Le catalogue d'abord, puis la liste personnalisée décrite par ses séries.
        let ids: Vec<&str> = view.lists.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(ids, ["spamhaus_zen", custom]);
        let row = &view.lists[1];
        assert!(row.custom && row.major && row.subject == "ip");
        assert_eq!(
            (row.name.as_str(), row.zone.as_str()),
            ("dnsbl.example.org", "dnsbl.example.org")
        );
        assert!(!view.lists[0].custom);

        let refused = view.checks.iter().find(|c| c.list == "spamhaus_zen").unwrap();
        assert!(refused.public_resolver_refused);
        let listed = view.checks.iter().find(|c| c.list == custom).unwrap();
        assert!(!listed.public_resolver_refused);
        assert_eq!(listed.lookup_url, None, "pas de page connue pour une liste inconnue");
    }
}
