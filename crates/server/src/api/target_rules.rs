//! Règles d'alerte qui surveillent un équipement, pour le formulaire d'édition.
//!
//! Une règle ne dit pas à quel type d'équipement elle s'adresse : elle vaut
//! pour toute série que sa requête ramène. Pour savoir ce qui surveille un
//! équipement, on croise donc les sélecteurs de chaque règle (nom de métrique
//! et filtres d'étiquettes, comme `probe="push"`) avec les séries que
//! l'équipement a réellement écrites. Un agent sans module Docker ne voit
//! pas les règles des conteneurs ; une règle de pack ou écrite à la main est
//! retenue comme les autres, sans liste à tenir.
//!
//! Couper une règle ici pose la même surcharge `enabled: false` que « Ne plus
//! m'alerter » : la route ne fait que lire.

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::Json;
use axum::extract::{Path, State};
use chrono::{TimeDelta, Utc};
use dumbmonit_proto::TargetId;
use serde::Serialize;

use crate::alerting::machine::Phase;
use crate::alerting::model::Severity;
use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::notify::policy_store;
use crate::state::AppState;
use crate::tsdb::METRIC_PREFIX;

/// Fenêtre de recherche des métriques écrites : assez longue pour une sonde
/// lente (une vérification par heure), assez courte pour oublier un module
/// retiré de l'agent.
const SEEN_WITHIN: TimeDelta = TimeDelta::hours(24);

#[derive(Debug, Serialize)]
pub struct TargetRules {
    /// `ok`, `no_data` (rien d'écrit sur la fenêtre : équipement neuf ou en
    /// pause) ou `unavailable` (VictoriaMetrics n'a pas répondu).
    pub status: &'static str,
    pub rules: Vec<TargetRule>,
}

#[derive(Debug, Serialize)]
pub struct TargetRule {
    pub id: i64,
    pub uid: String,
    pub name: String,
    pub description: String,
    pub severity: Severity,
    /// Activée pour l'ensemble des équipements.
    pub enabled: bool,
    /// Coupée pour cet équipement.
    pub ignored: bool,
    /// Un administrateur a surchargé le seuil pour cet équipement.
    pub custom_threshold: bool,
    /// En alerte sur cet équipement en ce moment.
    pub firing: bool,
}

pub async fn list(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<TargetRules>> {
    let nodes = db::alerts::list_target_nodes(&state.pool).await?;
    let Some(node) = nodes.into_iter().find(|node| node.id == id) else {
        return Err(ApiError::NotFound(format!("Monitor {id} not found.")));
    };

    let since = (Utc::now() - SEEN_WITHIN).timestamp();
    let written = match state.victoria.target_series(id, since).await {
        Ok(series) => series,
        Err(error) => {
            tracing::warn!(%error, target = id, "monitored rules: series unavailable");
            return Ok(Json(TargetRules { status: "unavailable", rules: Vec::new() }));
        }
    };
    if written.is_empty() {
        return Ok(Json(TargetRules { status: "no_data", rules: Vec::new() }));
    }

    let index = SeriesIndex::new(&written);
    let overrides = policy_store::list_overrides(&state.pool, None, Some(id)).await?;
    let firing: HashSet<String> = db::alerts::list_active(&state.pool)
        .await?
        .into_iter()
        .filter(|alert| alert.target_id == Some(id) && alert.state.phase == Phase::Firing)
        .map(|alert| alert.rule_uid)
        .collect();

    let mut rules: Vec<TargetRule> = db::alerts::list_rules(&state.pool)
        .await?
        .into_iter()
        .filter(|rule| rule.selector.matches(Some(&node)))
        .filter(|rule| reads_any(&rule.query, &index))
        .map(|rule| {
            let over = overrides.iter().find(|o| o.rule_uid == rule.uid);
            TargetRule {
                id: rule.id,
                firing: firing.contains(&rule.uid),
                ignored: over.is_some_and(|o| o.enabled == Some(false)),
                custom_threshold: over
                    .is_some_and(|o| o.threshold.is_some() || o.clear_threshold.is_some()),
                uid: rule.uid,
                name: rule.name,
                description: rule.description,
                severity: rule.severity,
                enabled: rule.enabled,
            }
        })
        .collect();
    rules.sort_by_key(|rule| rule.name.to_lowercase());
    Ok(Json(TargetRules { status: "ok", rules }))
}

/// La requête lit-elle au moins une des séries `written` ?
fn reads_any(query: &str, written: &SeriesIndex<'_>) -> bool {
    query_selectors(query)
        .iter()
        .any(|selector| written.named(&selector.name).any(|series| selector.matches(series)))
}

/// Séries d'un équipement rangées par nom de métrique : un agent en écrit des
/// milliers, et chaque règle n'en regarde que quelques noms.
struct SeriesIndex<'a> {
    by_name: HashMap<&'a str, Vec<&'a BTreeMap<String, String>>>,
}

impl<'a> SeriesIndex<'a> {
    fn new(written: &'a [BTreeMap<String, String>]) -> Self {
        let mut by_name: HashMap<&str, Vec<_>> = HashMap::new();
        for series in written {
            let name = series.get("__name__").map_or("", String::as_str);
            by_name.entry(name).or_default().push(series);
        }
        Self { by_name }
    }

    /// Les séries dont le nom convient.
    fn named(&self, wanted: &Wanted<'_>) -> impl Iterator<Item = &'a BTreeMap<String, String>> {
        let found: Vec<&'a BTreeMap<String, String>> = match *wanted {
            Wanted::Exact(name) => self.by_name.get(name).cloned().unwrap_or_default(),
            Wanted::Prefix(prefix) => self
                .by_name
                .iter()
                .filter(|(name, _)| name.starts_with(prefix))
                .flat_map(|(_, series)| series.iter().copied())
                .collect(),
        };
        found.into_iter()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Wanted<'a> {
    Exact(&'a str),
    /// Fragment d'une expression régulière sur `__name__`, comme
    /// `"dumbmonit_disk_.*"` : toute métrique qui commence ainsi.
    Prefix(&'a str),
}

#[derive(Debug, PartialEq, Eq)]
enum Op {
    Eq,
    Ne,
    Re,
    NotRe,
}

#[derive(Debug)]
struct Matcher {
    label: String,
    op: Op,
    value: String,
    /// L'expression de `=~` et `!~`, compilée une fois ; `None` si elle ne se
    /// lit pas.
    regex: Option<regex::Regex>,
}

impl Matcher {
    fn new(label: String, op: Op, value: String) -> Self {
        let regex = matches!(op, Op::Re | Op::NotRe)
            .then(|| regex::Regex::new(&format!("^(?:{value})$")).ok())
            .flatten();
        Self { label, op, value, regex }
    }

    /// Une étiquette absente vaut la chaîne vide, comme dans PromQL. Une
    /// expression illisible ne doit pas cacher la règle : elle laisse passer.
    fn holds(&self, series: &BTreeMap<String, String>) -> bool {
        let actual = series.get(&self.label).map_or("", String::as_str);
        match self.op {
            Op::Eq => actual == self.value,
            Op::Ne => actual != self.value,
            Op::Re => self.regex.as_ref().is_none_or(|re| re.is_match(actual)),
            Op::NotRe => self.regex.as_ref().is_none_or(|re| !re.is_match(actual)),
        }
    }
}

/// Un sélecteur de série d'une requête : le nom de la métrique et les filtres
/// posés juste après, entre accolades.
#[derive(Debug)]
struct SeriesSelector<'a> {
    name: Wanted<'a>,
    matchers: Vec<Matcher>,
}

impl SeriesSelector<'_> {
    fn matches(&self, series: &BTreeMap<String, String>) -> bool {
        let name = series.get("__name__").map_or("", String::as_str);
        let named = match self.name {
            Wanted::Exact(wanted) => name == wanted,
            Wanted::Prefix(prefix) => name.starts_with(prefix),
        };
        named && self.matchers.iter().all(|matcher| matcher.holds(series))
    }
}

/// Sélecteurs de séries qu'une requête MetricsQL lit : chaque mot qui commence
/// par le préfixe des métriques de DumbMonit, avec les filtres qui le suivent.
fn query_selectors(query: &str) -> Vec<SeriesSelector<'_>> {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == ':';
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = query[from..].find(METRIC_PREFIX) {
        let start = from + at;
        let end = query[start..].find(|c: char| !word(c)).map_or(query.len(), |n| start + n);
        from = end;
        if query[..start].chars().next_back().is_some_and(word) {
            continue;
        }
        let name = &query[start..end];
        if name.len() == METRIC_PREFIX.len() {
            continue;
        }
        let name = if name.ends_with('_') { Wanted::Prefix(name) } else { Wanted::Exact(name) };
        let matchers = query[end..]
            .strip_prefix('{')
            .and_then(|rest| rest.find('}').map(|close| parse_matchers(&rest[..close])))
            .unwrap_or_default();
        found.push(SeriesSelector { name, matchers });
    }
    found
}

/// `probe!="push", job=~"a|b"` → filtres. Ce qui ne se lit pas est laissé de
/// côté : mieux vaut lister une règle de trop que d'en cacher une.
fn parse_matchers(inner: &str) -> Vec<Matcher> {
    let mut matchers = Vec::new();
    let mut rest = inner.trim();
    while !rest.is_empty() {
        let Some(op_at) = rest.find(['=', '!']) else { break };
        let label = rest[..op_at].trim().to_string();
        let after = &rest[op_at..];
        let (op, skip) = if after.starts_with("=~") {
            (Op::Re, 2)
        } else if after.starts_with("!~") {
            (Op::NotRe, 2)
        } else if after.starts_with("!=") {
            (Op::Ne, 2)
        } else {
            (Op::Eq, 1)
        };
        let value_part = after[skip..].trim_start();
        let Some(quote) = value_part.chars().next().filter(|c| matches!(c, '"' | '\'' | '`'))
        else {
            break;
        };
        let Some(close) = value_part[1..].find(quote) else { break };
        let value = value_part[1..1 + close].replace("\\\\", "\\");
        if !label.is_empty() && label != "__name__" {
            matchers.push(Matcher::new(label, op, value));
        }
        rest = value_part[close + 2..].trim_start().trim_start_matches(',').trim_start();
    }
    matchers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(query: &str) -> Vec<String> {
        query_selectors(query)
            .into_iter()
            .map(|selector| match selector.name {
                Wanted::Exact(n) => n.to_string(),
                Wanted::Prefix(p) => format!("{p}*"),
            })
            .collect()
    }

    fn series(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn les_metriques_dune_requete_sont_relevees() {
        assert_eq!(
            names("dumbmonit_container_update_available == 1"),
            ["dumbmonit_container_update_available"]
        );
        assert_eq!(
            names(
                "max by (target) (dumbmonit_up{x=\"1\"}) or on(target) absent(dumbmonit_probe_success)"
            ),
            ["dumbmonit_up", "dumbmonit_probe_success"]
        );
        assert_eq!(names("{__name__=~\"dumbmonit_disk_.*\"} > 90"), ["dumbmonit_disk_*"]);
        assert!(names("{__name__=~\"dumbmonit_.*\"}").is_empty(), "tout le préfixe ne dit rien");
        assert!(names("my_dumbmonit_up").is_empty(), "le préfixe doit commencer un mot");
        assert!(names("vector(1)").is_empty());
    }

    #[test]
    fn les_filtres_detiquettes_sont_lus() {
        let selectors = query_selectors(
            "dumbmonit_probe_success{probe!=\"push\", job=~'a|b' ,host!~\"x.*\",env=\"prod\"} == 0",
        );
        assert_eq!(selectors[0].matchers[0].label, "probe");
        assert_eq!(selectors[0].matchers[0].op, Op::Ne);
        assert_eq!(selectors[0].matchers[0].value, "push");
        assert_eq!(selectors[0].matchers[1].op, Op::Re);
        assert_eq!(selectors[0].matchers[1].value, "a|b");
        assert_eq!(selectors[0].matchers[2].op, Op::NotRe);
        assert_eq!(selectors[0].matchers[3].label, "env");
        assert_eq!(selectors[0].matchers[3].value, "prod");
    }

    #[test]
    fn une_regle_est_retenue_si_un_de_ses_selecteurs_trouve_une_serie() {
        let all = vec![
            series(&[("__name__", "dumbmonit_probe_success"), ("probe", "http")]),
            series(&[("__name__", "dumbmonit_disk_used_ratio"), ("mount", "/")]),
        ];
        let written = SeriesIndex::new(&all);
        assert!(reads_any("dumbmonit_probe_success{probe!=\"push\"} == bool 0", &written));
        assert!(!reads_any("dumbmonit_probe_success{probe=\"push\"} == bool 0", &written));
        assert!(reads_any("dumbmonit_probe_success{probe=~\"http|tcp\"} == 0", &written));
        assert!(!reads_any("dumbmonit_probe_success{probe!~\"ht.*\"} == 0", &written));
        assert!(reads_any("{__name__=~\"dumbmonit_disk_.*\"} > 0.9", &written));
        assert!(!reads_any("dumbmonit_container_up == 0", &written));
        assert!(!reads_any("dumbmonit_upgrade_pending > 0", &written));
        // Une étiquette absente vaut la chaîne vide.
        assert!(reads_any("dumbmonit_disk_used_ratio{kloset=\"\"} > 0.9", &written));
        assert!(!reads_any("dumbmonit_disk_used_ratio{kloset!=\"\"} > 0.9", &written));
    }
}
