//! Carte des menaces de la page Sécurité : d'où viennent les attaques
//! arrêtées, réduites à leur pays, toutes sources réunies.
//!
//! * `GET /security/threats` — les pays, du plus actif au plus calme, avec la
//!   part de chaque source ; `?target=<id>` limite à un équipement (le panneau
//!   CrowdSec ne montre que les siennes).
//!
//! Les sources, toutes déjà collectées ailleurs :
//!
//! * **CrowdSec** : alertes locales des dernières 24 heures par pays
//!   (`source.cn`), série `dumbmonit_crowdsec_alerts_by_country`, présente
//!   quand la cible a un compte de machine (`collectors/crowdsec`) ;
//! * **pare-feu** : connexions entrantes bloquées de la carte en direct
//!   OPNsense (fenêtre de quinze minutes en mémoire, seulement tant que
//!   quelqu'un la regarde, voir `opnsense_live.rs`) ;
//! * **Cloudflare** : menaces arrêtées par pays (`countryMap` du trafic),
//!   série `dumbmonit_cloudflare_country_threats`.
//!
//! Rien que des pays : aucune adresse n'est lue ici, ni stockée, ni rendue.
//! Un compte restreint par ses équipes ne voit que les sources de ses
//! équipements.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use dumbmonit_proto::TargetId;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::api::{ApiError, ApiResult};
use crate::auth::teams::TeamAccess;
use crate::state::AppState;
use crate::tsdb::InstantSeries;

pub fn routes() -> Router<AppState> {
    Router::new().route("/security/threats", get(threats))
}

/// Dernière valeur des séries, sur une fenêtre un peu plus longue que les
/// intervalles de sonde habituels.
const LOOKBACK: &str = "15m";
/// Fenêtre des alertes CrowdSec comptées (`ALERTS_SINCE` du collecteur).
const CROWDSEC_WINDOW_SECS: i64 = 24 * 3600;
/// Fenêtre du trafic Cloudflare lu : l'heure précédente et l'heure en cours.
const CLOUDFLARE_WINDOW_SECS: i64 = 2 * 3600;

/// Séries lues : les comptes par pays, et de quoi savoir qu'une source est
/// branchée même quand elle n'a rien vu.
const SERIES: &str = "dumbmonit_(crowdsec_alerts_recent|crowdsec_alerts_by_country|\
                      cloudflare_analytics_readable|cloudflare_country_threats)";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Crowdsec,
    Firewall,
    Cloudflare,
}

impl Source {
    fn window_secs(self) -> i64 {
        match self {
            Source::Crowdsec => CROWDSEC_WINDOW_SECS,
            Source::Firewall => super::opnsense_live::WINDOW_SECS,
            Source::Cloudflare => CLOUDFLARE_WINDOW_SECS,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct ThreatCountry {
    /// ISO 3166-1 alpha-2 (`CN`).
    pub code: String,
    pub total: u64,
    pub crowdsec: u64,
    pub firewall: u64,
    pub cloudflare: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThreatSource {
    pub source: Source,
    /// Équipements qui alimentent cette source (pour un lien vers leur fiche).
    pub targets: Vec<TargetId>,
    pub total: u64,
    /// Durée couverte par les comptes de cette source.
    pub window_secs: i64,
}

#[derive(Debug, Serialize)]
pub struct ThreatMap {
    pub countries: Vec<ThreatCountry>,
    /// Les sources branchées, même sans rien vu ; vide : aucune source.
    pub sources: Vec<ThreatSource>,
    /// Vrai quand VictoriaMetrics n'a pas répondu : seules les sources en
    /// mémoire (pare-feu) sont comptées.
    pub partial: bool,
    pub evaluated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ThreatQuery {
    #[serde(default)]
    pub target: Option<TargetId>,
}

/// Un code pays utilisable sur la carte : deux lettres majuscules.
fn is_country(code: &str) -> bool {
    code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase())
}

/// Un compte de VictoriaMetrics en entier positif (les valeurs sont des
/// comptes, rendus en texte).
fn count_of(series: &InstantSeries) -> u64 {
    // Un compte : arrondi, jamais négatif.
    let value = series.value.1.parse::<f64>().unwrap_or(0.0);
    if value.is_finite() && value > 0.0 { value.round() as u64 } else { 0 }
}

/// Réunit les séries lues et les blocages des pare-feu en une carte par pays.
/// `keep` dit quels équipements comptent (équipes, filtre `target`).
pub fn aggregate(
    series: &[InstantSeries],
    firewalls: &[(TargetId, BTreeMap<String, u64>)],
    keep: impl Fn(TargetId) -> bool,
) -> (Vec<ThreatCountry>, Vec<ThreatSource>) {
    let mut countries: BTreeMap<String, ThreatCountry> = BTreeMap::new();
    let mut sources: BTreeMap<Source, (BTreeSet<TargetId>, u64)> = BTreeMap::new();
    let mut add = |source: Source, target: TargetId, code: Option<&str>, count: u64| {
        let slot = sources.entry(source).or_default();
        slot.0.insert(target);
        let Some(code) = code.filter(|c| is_country(c)) else { return };
        if count == 0 {
            return;
        }
        slot.1 += count;
        let row = countries
            .entry(code.to_string())
            .or_insert_with(|| ThreatCountry { code: code.to_string(), ..Default::default() });
        row.total += count;
        match source {
            Source::Crowdsec => row.crowdsec += count,
            Source::Firewall => row.firewall += count,
            Source::Cloudflare => row.cloudflare += count,
        }
    };
    for one in series {
        let Some(target) = one.metric.get("target").and_then(|t| t.parse::<TargetId>().ok())
        else {
            continue;
        };
        if !keep(target) {
            continue;
        }
        let name = one.metric.get("__name__").map(String::as_str).unwrap_or_default();
        let country = one.metric.get("country").map(String::as_str);
        match name {
            "dumbmonit_crowdsec_alerts_recent" => add(Source::Crowdsec, target, None, 0),
            "dumbmonit_crowdsec_alerts_by_country" => {
                add(Source::Crowdsec, target, country, count_of(one));
            }
            // Un jeton sans le droit d'analyse ne branche pas la source.
            "dumbmonit_cloudflare_analytics_readable" if count_of(one) > 0 => {
                add(Source::Cloudflare, target, None, 0);
            }
            "dumbmonit_cloudflare_country_threats" => {
                add(Source::Cloudflare, target, country, count_of(one));
            }
            _ => {}
        }
    }
    for (target, blocked) in firewalls {
        if !keep(*target) {
            continue;
        }
        add(Source::Firewall, *target, None, 0);
        for (code, count) in blocked {
            add(Source::Firewall, *target, Some(code.as_str()), *count);
        }
    }
    let mut countries: Vec<ThreatCountry> = countries.into_values().collect();
    countries.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.code.cmp(&b.code)));
    let sources = sources
        .into_iter()
        .map(|(source, (targets, total))| ThreatSource {
            source,
            targets: targets.into_iter().collect(),
            total,
            window_secs: source.window_secs(),
        })
        .collect();
    (countries, sources)
}

/// `GET /api/security/threats` — lecture, ouverte à tous les rôles.
async fn threats(
    TeamAccess(access): TeamAccess,
    State(state): State<AppState>,
    Query(query): Query<ThreatQuery>,
) -> ApiResult<Json<ThreatMap>> {
    let visible: Option<HashSet<TargetId>> = access.visible_targets(&state.pool).await?;
    if let Some(id) = query.target {
        let exists = sqlx::query_scalar::<_, i64>("SELECT id FROM targets WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .is_some();
        // Hors des équipes du compte : comme s'il n'existait pas.
        if !exists || visible.as_ref().is_some_and(|v| !v.contains(&id)) {
            return Err(ApiError::NotFound(format!("Monitor {id} not found.")));
        }
    }
    let keep = |id: TargetId| {
        query.target.is_none_or(|only| only == id)
            && visible.as_ref().is_none_or(|v| v.contains(&id))
    };
    let selector = match query.target {
        Some(id) => format!(r#"{{__name__=~"{SERIES}", target="{id}"}}"#),
        None => format!(r#"{{__name__=~"{SERIES}"}}"#),
    };
    let promql = format!("last_over_time({selector}[{LOOKBACK}]) keep_metric_names");
    let nothing_visible = visible.as_ref().is_some_and(HashSet::is_empty);
    let (series, partial) = if nothing_visible {
        (Vec::new(), false)
    } else {
        match state.victoria.query(&promql).await {
            Ok(series) => (series, false),
            Err(error) => {
                warn!("carte des menaces : VictoriaMetrics ne répond pas : {error:#}");
                (Vec::new(), true)
            }
        }
    };
    let now = chrono::Utc::now();
    let firewalls = super::opnsense_live::blocked_by_country(now.timestamp());
    let (countries, sources) = aggregate(&series, &firewalls, keep);
    Ok(Json(ThreatMap {
        countries,
        sources,
        partial,
        evaluated_at: now.format("%Y-%m-%d %H:%M:%S").to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serie(name: &str, target: &str, country: Option<&str>, value: &str) -> InstantSeries {
        let mut metric = BTreeMap::new();
        metric.insert("__name__".to_string(), format!("dumbmonit_{name}"));
        metric.insert("target".to_string(), target.to_string());
        if let Some(country) = country {
            metric.insert("country".to_string(), country.to_string());
        }
        InstantSeries { metric, value: (0.0, value.to_string()) }
    }

    #[test]
    fn les_sources_s_additionnent_par_pays() {
        let series = vec![
            serie("crowdsec_alerts_recent", "1", None, "7"),
            serie("crowdsec_alerts_by_country", "1", Some("CN"), "5"),
            serie("crowdsec_alerts_by_country", "1", Some("FR"), "2"),
            serie("cloudflare_analytics_readable", "2", None, "1"),
            serie("cloudflare_country_threats", "2", Some("CN"), "10"),
            serie("cloudflare_country_threats", "2", Some("US"), "3"),
        ];
        let firewalls = vec![(3, BTreeMap::from([("RU".to_string(), 4), ("CN".to_string(), 1)]))];
        let (countries, sources) = aggregate(&series, &firewalls, |_| true);
        let codes: Vec<&str> = countries.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["CN", "RU", "US", "FR"], "du plus actif au plus calme");
        let china = &countries[0];
        assert_eq!((china.total, china.crowdsec, china.cloudflare, china.firewall), (16, 5, 10, 1));
        let kinds: Vec<Source> = sources.iter().map(|s| s.source).collect();
        assert_eq!(kinds, [Source::Crowdsec, Source::Firewall, Source::Cloudflare]);
        assert_eq!(sources[0].total, 7);
        assert_eq!(sources[0].targets, [1]);
        assert_eq!(sources[0].window_secs, 24 * 3600);
        assert_eq!(sources[1].total, 5);
        assert_eq!(sources[2].total, 13);
    }

    #[test]
    fn une_source_branchee_sans_menace_reste_listee() {
        let series = vec![serie("crowdsec_alerts_recent", "4", None, "0")];
        let (countries, sources) = aggregate(&series, &[], |_| true);
        assert!(countries.is_empty());
        assert_eq!(sources.len(), 1);
        assert_eq!((sources[0].source, sources[0].total), (Source::Crowdsec, 0));
    }

    #[test]
    fn les_equipements_hors_champ_et_les_codes_douteux_sont_ignores() {
        let series = vec![
            serie("crowdsec_alerts_by_country", "1", Some("CN"), "5"),
            serie("crowdsec_alerts_by_country", "2", Some("DE"), "3"),
            serie("crowdsec_alerts_by_country", "2", Some("192.0.2.1"), "9"),
            serie("crowdsec_alerts_by_country", "2", Some("de"), "9"),
            serie("crowdsec_alerts_by_country", "2", Some("NL"), "NaN"),
        ];
        let firewalls = vec![(1, BTreeMap::from([("RU".to_string(), 4)]))];
        let (countries, sources) = aggregate(&series, &firewalls, |id| id == 2);
        let codes: Vec<&str> = countries.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["DE"]);
        assert_eq!(sources.len(), 1, "le pare-feu 1 est hors champ");
        assert_eq!(sources[0].targets, [2]);
    }
}
