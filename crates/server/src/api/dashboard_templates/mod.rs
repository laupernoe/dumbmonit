//! Modèles de tableaux de bord prêts à l'emploi.
//!
//! - `GET /api/dashboards/templates` : les modèles, avec pour chacun les types
//!   d'équipements présents et s'il est utilisable, et les filtres proposés
//!   (dossiers, sites, clients) d'après les équipements que l'appelant voit ;
//! - `GET /api/dashboards/templates/{id}/preview?kind=&folder=&site=&client=` :
//!   le tableau que donnerait le modèle pour ce filtre, prêt à être affiché,
//!   puis créé par `POST /api/dashboards` (aucune route d'écriture ici).
//!
//! Les modèles sont des données ([`catalog`]). La génération remplace le jeton
//! `$T` de chaque requête par `target=~"1|2|3"` : les équipements actifs que
//! l'appelant voit, qui passent le filtre et sont du type de ce widget. C'est
//! un instantané : un équipement ajouté plus tard n'entre pas dans un tableau
//! déjà créé. Un widget sans équipement de son type est omis ; un modèle sans
//! aucun équipement n'est pas utilisable.

pub mod catalog;

use std::collections::{BTreeMap, BTreeSet, HashSet};

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::dashboards::{GRID_COLUMNS, Widget, clean_widgets};
use crate::api::{ApiError, ApiResult};
use crate::auth::clients;
use crate::auth::teams::{self, Access, TeamAccess};
use crate::state::AppState;
use catalog::{SELECTOR, Shape, Spec, Template};

/// Identifiants d'équipements au plus dans un sélecteur : la requête garde
/// ainsi bien moins de 4 Ko.
const MAX_IDS: usize = 300;
/// Widgets « dossier » au plus par modèle.
const MAX_FOLDERS: usize = 6;
/// Équipements nommés dans l'aperçu.
const MAX_LISTED: usize = 12;
const MAX_NAME_CHARS: usize = 80;
const ALERTS_LIMIT: u32 = 8;

type TargetRow = (i64, String, String, String, Option<i64>, i64);

/// Un équipement actif que l'appelant voit.
#[derive(Debug, Clone)]
struct Row {
    id: i64,
    name: String,
    kind: String,
    folder: String,
    site: Option<i64>,
    client: i64,
}

async fn visible_rows(state: &AppState, access: &Access) -> ApiResult<Vec<Row>> {
    let visible = access.visible_targets(&state.pool).await?;
    let rows: Vec<TargetRow> = sqlx::query_as(
        "SELECT id, name, kind, group_name, location_id, client_id
         FROM targets WHERE enabled = 1 ORDER BY name COLLATE NOCASE, id",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(anyhow::Error::from)?;
    Ok(rows
        .into_iter()
        .filter(|(id, ..)| visible.as_ref().is_none_or(|set| set.contains(id)))
        .map(|(id, name, kind, folder, site, client)| Row { id, name, kind, folder, site, client })
        .collect())
}

/// Les équipements de `rows` qui publient `metric`, d'après VictoriaMetrics ;
/// `None` quand la base ne répond pas (le type seul décide alors).
async fn publishing(state: &AppState, metric: &str) -> Option<HashSet<i64>> {
    match state.victoria.query(&format!("count by (target) ({metric})")).await {
        Ok(series) => Some(
            series.iter().filter_map(|s| s.metric.get("target")?.parse::<i64>().ok()).collect(),
        ),
        Err(error) => {
            tracing::warn!(%error, metric, "dashboard templates: series lookup failed");
            None
        }
    }
}

/// Les équipements qui peuvent alimenter le modèle, avant tout filtre.
async fn candidates<'a>(
    state: &AppState,
    template: &Template,
    rows: &'a [Row],
) -> Vec<&'a Row> {
    let of_kind: Vec<&Row> = rows
        .iter()
        .filter(|row| template.kinds.is_empty() || template.kinds.contains(&row.kind.as_str()))
        .collect();
    let Some(metric) = template.needs_metric else { return of_kind };
    if of_kind.is_empty() {
        return of_kind;
    }
    match publishing(state, metric).await {
        Some(ids) => of_kind.into_iter().filter(|row| ids.contains(&row.id)).collect(),
        None => of_kind,
    }
}

// ------------------------------------------------------------------ liste

#[derive(Debug, Serialize)]
struct Named {
    id: i64,
    name: String,
}

#[derive(Debug, Serialize)]
struct KindCount {
    kind: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct FilterOptions {
    /// Dossiers des équipements, et les dossiers qui les contiennent.
    folders: Vec<String>,
    /// Emplacements (carte du monde) où un équipement est rangé.
    sites: Vec<Named>,
    /// Clients visibles ; vide hors mode MSP.
    clients: Vec<Named>,
}

#[derive(Debug, Serialize)]
struct TemplateView {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    /// Nom d'icône lucide.
    icon: &'static str,
    /// Types d'équipements qui l'alimentent ; vide : tous.
    requires: &'static [&'static str],
    /// Série qu'un équipement doit publier pour compter, le cas échéant.
    needs_metric: Option<&'static str>,
    /// Les types présents parmi ceux qui l'alimentent, du plus nombreux au moins.
    present_kinds: Vec<KindCount>,
    target_count: usize,
    widget_count: usize,
    available: bool,
    /// Pourquoi il n'est pas utilisable (anglais, comme le catalogue).
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TemplateList {
    filters: FilterOptions,
    templates: Vec<TemplateView>,
}

fn unavailable_reason(template: &Template) -> String {
    if let Some(metric) = template.needs_metric {
        return format!("No device publishes {metric} yet.");
    }
    if template.kinds.is_empty() {
        return "Add a device first: this template reads the devices you monitor.".into();
    }
    format!("Needs at least one device of type {}.", template.kinds.join(", "))
}

/// Les dossiers, plus leurs parents (`Paris/Rack A` apporte `Paris`).
fn folder_options(rows: &[Row]) -> Vec<String> {
    let mut folders = BTreeSet::new();
    for row in rows {
        let folder = teams::normalize_folder(&row.folder);
        let mut path = String::new();
        for part in folder.split('/').filter(|part| !part.is_empty()) {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
            folders.insert(path.clone());
        }
    }
    folders.into_iter().collect()
}

async fn filter_options(
    state: &AppState,
    access: &Access,
    rows: &[Row],
) -> ApiResult<FilterOptions> {
    let used_sites: HashSet<i64> = rows.iter().filter_map(|row| row.site).collect();
    let sites: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, name FROM locations ORDER BY name COLLATE NOCASE")
            .fetch_all(&state.pool)
            .await
            .map_err(anyhow::Error::from)?;
    let sites = sites
        .into_iter()
        .filter(|(id, _)| used_sites.contains(id))
        .map(|(id, name)| Named { id, name })
        .collect();
    let clients = if clients::enabled(&state.pool).await? {
        let used: HashSet<i64> = rows.iter().map(|row| row.client).collect();
        clients::refs(&state.pool, access.clients())
            .await?
            .into_iter()
            .filter(|client| used.contains(&client.id))
            .map(|client| Named { id: client.id, name: client.name })
            .collect()
    } else {
        Vec::new()
    };
    Ok(FilterOptions { folders: folder_options(rows), sites, clients })
}

/// `GET /api/dashboards/templates`
pub async fn list(
    State(state): State<AppState>,
    TeamAccess(access): TeamAccess,
) -> ApiResult<Json<TemplateList>> {
    let rows = visible_rows(&state, &access).await?;
    let filters = filter_options(&state, &access, &rows).await?;
    let mut templates = Vec::with_capacity(catalog::TEMPLATES.len());
    for template in catalog::TEMPLATES {
        let found = candidates(&state, template, &rows).await;
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for row in &found {
            *counts.entry(row.kind.as_str()).or_default() += 1;
        }
        let mut present_kinds: Vec<KindCount> = counts
            .into_iter()
            .map(|(kind, count)| KindCount { kind: kind.to_string(), count })
            .collect();
        present_kinds.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.kind.cmp(&b.kind)));
        let available = !found.is_empty();
        templates.push(TemplateView {
            id: template.id,
            title: template.title,
            description: template.description,
            icon: template.icon,
            requires: template.kinds,
            needs_metric: template.needs_metric,
            present_kinds,
            target_count: found.len(),
            widget_count: template.widgets.len(),
            available,
            reason: (!available).then(|| unavailable_reason(template)),
        });
    }
    Ok(Json(TemplateList { filters, templates }))
}

// ---------------------------------------------------------------- aperçu

#[derive(Debug, Default, Deserialize)]
pub struct Filter {
    /// Un seul type d'équipement.
    kind: Option<String>,
    /// Un dossier, sous-dossiers compris.
    folder: Option<String>,
    /// Un emplacement.
    site: Option<i64>,
    /// Un client (mode MSP).
    client: Option<i64>,
}

impl Filter {
    fn folder(&self) -> Option<String> {
        let folder = teams::normalize_folder(self.folder.as_deref().unwrap_or(""));
        (!folder.is_empty()).then_some(folder)
    }

    fn kind(&self) -> Option<&str> {
        self.kind.as_deref().map(str::trim).filter(|kind| !kind.is_empty())
    }

    fn accepts(&self, row: &Row) -> bool {
        self.kind().is_none_or(|kind| row.kind == kind)
            && self.folder().is_none_or(|folder| teams::folder_covers(&folder, &row.folder))
            && self.site.is_none_or(|site| row.site == Some(site))
            && self.client.is_none_or(|client| row.client == client)
    }
}

#[derive(Debug, Serialize)]
struct Matched {
    id: i64,
    name: String,
    kind: String,
}

#[derive(Debug, Serialize)]
pub struct Preview {
    id: &'static str,
    /// Nom proposé pour le tableau.
    name: String,
    range_secs: i64,
    refresh_secs: i64,
    matched_count: usize,
    /// Les premiers équipements retenus.
    matched: Vec<Matched>,
    /// Plus de [`MAX_IDS`] équipements : les requêtes n'en portent que les
    /// premiers.
    truncated: bool,
    widgets: Vec<Widget>,
}

fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let kept: String = text.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", kept.trim_end())
    }
}

fn selector(ids: &[i64]) -> String {
    let alternatives = ids.iter().map(i64::to_string).collect::<Vec<_>>().join("|");
    format!("target=~\"{alternatives}\"")
}

fn widget(spec: &Spec, id: String, title: &str, kind: &str, config: serde_json::Value) -> Widget {
    Widget {
        id,
        kind: kind.to_string(),
        title: shorten(title, MAX_NAME_CHARS),
        x: 0,
        y: 0,
        w: spec.w,
        h: spec.h,
        config,
    }
}

/// Range les widgets en lignes, de gauche à droite, sur la grille de 12 colonnes.
fn place(widgets: &mut [Widget]) {
    let (mut x, mut y, mut row_height) = (0, 0, 0);
    for item in widgets {
        if x + item.w > GRID_COLUMNS {
            y += row_height;
            x = 0;
            row_height = 0;
        }
        item.x = x;
        item.y = y;
        x += item.w;
        row_height = row_height.max(item.h);
    }
}

/// Les widgets du modèle pour ces équipements, et si la liste d'identifiants
/// a dû être coupée.
fn build(template: &Template, targets: &[&Row]) -> (Vec<Widget>, bool) {
    let mut truncated = false;
    let mut widgets = Vec::new();
    for spec in template.widgets {
        let applies =
            |row: &&&Row| spec.kinds.is_empty() || spec.kinds.contains(&row.kind.as_str());
        match spec.shape {
            Shape::Alerts => widgets.push(widget(
                spec,
                spec.id.to_string(),
                spec.title,
                "alerts",
                json!({ "limit": ALERTS_LIMIT }),
            )),
            Shape::Text => widgets.push(widget(
                spec,
                spec.id.to_string(),
                spec.title,
                "text",
                json!({ "markdown": spec.body }),
            )),
            Shape::Folders => {
                let folders: BTreeSet<&str> =
                    targets.iter().filter(applies).map(|row| row.folder.as_str()).collect();
                for (n, folder) in folders.into_iter().take(MAX_FOLDERS).enumerate() {
                    let title = if folder.is_empty() { "No folder" } else { folder };
                    widgets.push(widget(
                        spec,
                        format!("{}-{}", spec.id, n + 1),
                        title,
                        "group",
                        json!({ "folder": folder }),
                    ));
                }
            }
            Shape::Chart | Shape::Stat => {
                let mut ids: Vec<i64> =
                    targets.iter().filter(applies).map(|row| row.id).collect();
                if ids.is_empty() {
                    continue;
                }
                ids.sort_unstable();
                if ids.len() > MAX_IDS {
                    ids.truncate(MAX_IDS);
                    truncated = true;
                }
                let query = spec.body.replace(SELECTOR, &selector(&ids));
                let (kind, config) = if spec.shape == Shape::Chart {
                    ("chart", json!({ "query": query, "unit": spec.unit }))
                } else {
                    let mut config = json!({
                        "query": query,
                        "unit": spec.unit,
                        "direction": if spec.below { "below" } else { "above" },
                    });
                    if let Some(warn) = spec.warn {
                        config["warn"] = json!(warn);
                    }
                    if let Some(crit) = spec.crit {
                        config["crit"] = json!(crit);
                    }
                    ("stat", config)
                };
                widgets.push(widget(spec, spec.id.to_string(), spec.title, kind, config));
            }
        }
    }
    place(&mut widgets);
    (widgets, truncated)
}

async fn suggested_name(
    state: &AppState,
    access: &Access,
    template: &Template,
    filter: &Filter,
) -> ApiResult<String> {
    let mut parts = Vec::new();
    if let Some(folder) = filter.folder() {
        parts.push(folder);
    }
    if let Some(site) = filter.site {
        let name: Option<String> = sqlx::query_scalar("SELECT name FROM locations WHERE id = ?")
            .bind(site)
            .fetch_optional(&state.pool)
            .await
            .map_err(anyhow::Error::from)?;
        parts.extend(name);
    }
    if let Some(client) = filter.client.filter(|client| access.sees_client(*client)) {
        let name: Option<String> = sqlx::query_scalar("SELECT name FROM clients WHERE id = ?")
            .bind(client)
            .fetch_optional(&state.pool)
            .await
            .map_err(anyhow::Error::from)?;
        parts.extend(name);
    }
    let name = if parts.is_empty() {
        template.title.to_string()
    } else {
        format!("{} - {}", template.title, parts.join(", "))
    };
    Ok(shorten(&name, MAX_NAME_CHARS))
}

/// `GET /api/dashboards/templates/{id}/preview`
pub async fn preview(
    State(state): State<AppState>,
    TeamAccess(access): TeamAccess,
    Path(id): Path<String>,
    Query(filter): Query<Filter>,
) -> ApiResult<Json<Preview>> {
    let template = catalog::find(&id)
        .ok_or_else(|| ApiError::NotFound(format!("Dashboard template \"{id}\" not found.")))?;
    if let Some(kind) = filter.kind()
        && !template.kinds.is_empty()
        && !template.kinds.contains(&kind)
    {
        return Err(ApiError::BadRequest(format!(
            "The \"{}\" template does not apply to devices of type \"{kind}\".",
            template.title
        )));
    }
    let rows = visible_rows(&state, &access).await?;
    let found = candidates(&state, template, &rows).await;
    let targets: Vec<&Row> = found.into_iter().filter(|row| filter.accepts(row)).collect();
    let (widgets, truncated) = build(template, &targets);
    let widgets = clean_widgets(widgets)?;
    Ok(Json(Preview {
        id: template.id,
        name: suggested_name(&state, &access, template, &filter).await?,
        range_secs: template.range_secs,
        refresh_secs: template.refresh_secs,
        matched_count: targets.len(),
        matched: targets
            .iter()
            .take(MAX_LISTED)
            .map(|row| Matched { id: row.id, name: row.name.clone(), kind: row.kind.clone() })
            .collect(),
        truncated,
        widgets,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_of_every_kind(per_kind: i64) -> Vec<Row> {
        let mut kinds: BTreeSet<&str> = BTreeSet::new();
        for template in catalog::TEMPLATES {
            kinds.extend(template.kinds.iter().copied());
            for spec in template.widgets {
                kinds.extend(spec.kinds.iter().copied());
            }
        }
        let mut rows = Vec::new();
        let mut id = 0;
        for kind in kinds {
            for n in 0..per_kind {
                id += 1;
                rows.push(Row {
                    id,
                    name: format!("{kind}-{n}"),
                    kind: kind.to_string(),
                    folder: if n % 2 == 0 { "Paris/Rack A".into() } else { String::new() },
                    site: None,
                    client: 1,
                });
            }
        }
        rows
    }

    fn matching<'a>(template: &Template, rows: &'a [Row]) -> Vec<&'a Row> {
        rows.iter()
            .filter(|row| template.kinds.is_empty() || template.kinds.contains(&row.kind.as_str()))
            .collect()
    }

    #[test]
    fn every_template_builds_widgets_the_dashboard_validator_accepts() {
        let rows = rows_of_every_kind(3);
        for template in catalog::TEMPLATES {
            let targets = matching(template, &rows);
            let (widgets, truncated) = build(template, &targets);
            assert!(!truncated, "{}", template.id);
            assert!(!widgets.is_empty(), "{}: aucun widget", template.id);
            let count = widgets.len();
            let cleaned = clean_widgets(widgets)
                .unwrap_or_else(|error| panic!("{} : {}", template.id, error.into_parts().1));
            assert_eq!(cleaned.len(), count);
        }
    }

    #[test]
    fn every_widget_spec_is_well_formed() {
        for template in catalog::TEMPLATES {
            assert!(template.title.chars().count() <= MAX_NAME_CHARS, "{}", template.id);
            assert!(!template.description.is_empty(), "{}", template.id);
            let mut seen = HashSet::new();
            for spec in template.widgets {
                assert!(seen.insert(spec.id), "{} : id en double {}", template.id, spec.id);
                assert!(spec.title.chars().count() <= MAX_NAME_CHARS, "{}", spec.id);
                assert!(spec.unit.chars().count() <= 16, "{}", spec.id);
                for kind in spec.kinds {
                    assert!(
                        template.kinds.is_empty() || template.kinds.contains(kind),
                        "{} : le widget {} vise {kind}, que le modèle n'accepte pas",
                        template.id,
                        spec.id
                    );
                }
                if matches!(spec.shape, Shape::Chart | Shape::Stat) {
                    assert!(spec.body.contains(SELECTOR), "{} : pas de {SELECTOR}", spec.id);
                    assert_eq!(
                        spec.body.matches('(').count(),
                        spec.body.matches(')').count(),
                        "{} : parenthèses",
                        spec.id
                    );
                    assert!(spec.body.contains("dumbmonit_"), "{}", spec.id);
                }
            }
        }
    }

    #[test]
    fn template_ids_are_unique_and_findable() {
        let mut seen = HashSet::new();
        for template in catalog::TEMPLATES {
            assert!(seen.insert(template.id));
            assert!(catalog::find(template.id).is_some());
        }
        assert!(catalog::find("nope").is_none());
    }

    #[test]
    fn a_widget_without_a_device_of_its_kind_is_left_out() {
        let template = catalog::find("network").expect("modèle réseau");
        let rows = vec![Row {
            id: 7,
            name: "fw".into(),
            kind: "fortigate".into(),
            folder: String::new(),
            site: None,
            client: 1,
        }];
        let targets: Vec<&Row> = rows.iter().collect();
        let (widgets, _) = build(template, &targets);
        let queries: Vec<String> = widgets
            .iter()
            .filter_map(|w| w.config.get("query")?.as_str().map(str::to_string))
            .collect();
        assert!(!queries.is_empty());
        assert!(queries.iter().any(|q| q.contains("dumbmonit_fortigate_cpu_usage_percent")));
        assert!(queries.iter().all(|q| !q.contains("dumbmonit_if_octets_in")));
        assert!(queries.iter().all(|q| q.contains("target=~\"7\"") && !q.contains(SELECTOR)));
    }

    #[test]
    fn the_selector_lists_the_ids_and_is_capped() {
        assert_eq!(selector(&[1, 20, 3]), "target=~\"1|20|3\"");
        let template = catalog::find("servers").expect("modèle serveurs");
        let rows: Vec<Row> = (1..=(MAX_IDS as i64 + 20))
            .map(|id| Row {
                id,
                name: format!("h{id}"),
                kind: "agent".into(),
                folder: String::new(),
                site: None,
                client: 1,
            })
            .collect();
        let targets: Vec<&Row> = rows.iter().collect();
        let (widgets, truncated) = build(template, &targets);
        assert!(truncated);
        for w in &widgets {
            let query = w.config["query"].as_str().expect("requête");
            assert!(query.len() < 4096, "{} : {} octets", w.id, query.len());
        }
        assert!(clean_widgets(widgets).is_ok());
    }

    #[test]
    fn folders_become_one_group_widget_each_and_the_grid_is_respected() {
        let template = catalog::find("homelab-overview").expect("modèle d'ensemble");
        let rows = rows_of_every_kind(2);
        let targets: Vec<&Row> = rows.iter().collect();
        let (widgets, _) = build(template, &targets);
        let groups: Vec<&Widget> = widgets.iter().filter(|w| w.kind == "group").collect();
        assert_eq!(groups.len(), 2);
        assert!(groups.iter().any(|w| w.config["folder"] == "Paris/Rack A"));
        assert!(groups.iter().any(|w| w.config["folder"] == "" && w.title == "No folder"));
        for w in &widgets {
            assert!(w.x + w.w <= GRID_COLUMNS, "{}", w.id);
        }
        let mut cells = HashSet::new();
        for w in &widgets {
            for x in w.x..w.x + w.w {
                for y in w.y..w.y + w.h {
                    assert!(cells.insert((x, y)), "{} chevauche un autre widget", w.id);
                }
            }
        }
    }

    #[test]
    fn the_filter_narrows_by_kind_folder_site_and_client() {
        let row = Row {
            id: 1,
            name: "a".into(),
            kind: "agent".into(),
            folder: "Paris/Rack A".into(),
            site: Some(4),
            client: 2,
        };
        assert!(Filter::default().accepts(&row));
        let parse = |kind: Option<&str>, folder: Option<&str>, site, client| Filter {
            kind: kind.map(str::to_string),
            folder: folder.map(str::to_string),
            site,
            client,
        };
        assert!(parse(Some("agent"), Some("Paris"), Some(4), Some(2)).accepts(&row));
        assert!(parse(Some("agent"), Some("/Paris/Rack A/"), None, None).accepts(&row));
        assert!(!parse(Some("snmp"), None, None, None).accepts(&row));
        assert!(!parse(None, Some("Paris-Nord"), None, None).accepts(&row));
        assert!(!parse(None, None, Some(5), None).accepts(&row));
        assert!(!parse(None, None, None, Some(1)).accepts(&row));
        // Un dossier vide ne filtre rien.
        assert!(parse(None, Some(" "), None, None).accepts(&row));
    }

    #[test]
    fn folder_options_include_parent_folders() {
        let rows = rows_of_every_kind(1);
        let folders = folder_options(&rows);
        assert!(folders.contains(&"Paris".to_string()));
        assert!(folders.contains(&"Paris/Rack A".to_string()));
        assert!(!folders.iter().any(String::is_empty));
    }

    #[test]
    fn long_names_are_shortened_on_a_character_boundary() {
        assert_eq!(shorten("abc", 5), "abc");
        let long = "é".repeat(100);
        let short = shorten(&long, 80);
        assert_eq!(short.chars().count(), 80);
        assert!(short.ends_with('…'));
    }
}
