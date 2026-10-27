//! Tableaux de bord personnalisés.
//!
//! - `GET /api/dashboards` : les tableaux partagés et ceux du compte ;
//! - `POST /api/dashboards`, `GET|PUT|DELETE /api/dashboards/{id}` ;
//!
//! Un tableau est une grille de 12 colonnes de widgets (graphe, valeur
//! unique, état d'un dossier, alertes, texte). Les droits suivent les rôles :
//!
//! - un tableau **personnel** n'est vu et modifié que par son propriétaire,
//!   quel que soit son rôle (les écritures sont ouvertes à tous les comptes
//!   dans `auth::middleware`, comme le mot de passe) ; un jeton d'API n'en a
//!   pas ;
//! - un tableau **partagé** est lu par tous et modifié par les
//!   administrateurs (ou un jeton `write`), seuls à pouvoir aussi en créer ou
//!   en retirer le partage.
//!
//! La grille est stockée en JSON ; le serveur en valide la forme ([`validate`])
//! et ne garde que les clés connues de chaque type de widget.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{Row, SqlitePool};

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::{Identity, Principal};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dashboards", get(list).post(create))
        .route("/dashboards/{id}", get(get_one).put(update).delete(remove))
}

pub type DashboardId = i64;

/// Colonnes de la grille.
pub const GRID_COLUMNS: u32 = 12;
const MAX_NAME_CHARS: usize = 80;
const MAX_TITLE_CHARS: usize = 80;
const MAX_WIDGETS: usize = 60;
const MAX_ROWS: u32 = 2000;
const MAX_HEIGHT: u32 = 24;
const MAX_QUERY_CHARS: usize = 4096;
const MAX_TEXT_CHARS: usize = 4000;
const MAX_FOLDER_CHARS: usize = 120;
const MAX_UNIT_CHARS: usize = 16;
/// Tableaux qu'un compte peut posséder, et tableaux partagés au total.
const MAX_DASHBOARDS: i64 = 200;
const WIDGET_KINDS: [&str; 5] = ["chart", "stat", "group", "alerts", "text"];
const MIN_RANGE_SECS: i64 = 60;
/// Un peu plus d'un an, comme la plage maximale des requêtes de métriques.
const MAX_RANGE_SECS: i64 = 400 * 24 * 3600;

/// Un widget placé sur la grille.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub title: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    #[serde(default)]
    pub config: Value,
}

#[derive(Debug, Serialize)]
pub struct DashboardView {
    pub id: DashboardId,
    pub name: String,
    pub shared: bool,
    /// Compte propriétaire ; `None` : créé par un jeton.
    pub owner: Option<String>,
    /// Le compte courant peut modifier ce tableau.
    pub can_edit: bool,
    pub range_secs: i64,
    /// 0 : pas de rafraîchissement automatique.
    pub refresh_secs: i64,
    pub widgets: Vec<Widget>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct DashboardPayload {
    pub name: String,
    #[serde(default)]
    pub shared: bool,
    #[serde(default = "default_range")]
    pub range_secs: i64,
    #[serde(default = "default_refresh")]
    pub refresh_secs: i64,
    #[serde(default)]
    pub widgets: Vec<Widget>,
}

fn default_range() -> i64 {
    3600
}

fn default_refresh() -> i64 {
    30
}

/// Un tableau validé, prêt à être écrit.
#[derive(Debug, PartialEq)]
pub struct DashboardInput {
    pub name: String,
    pub shared: bool,
    pub range_secs: i64,
    pub refresh_secs: i64,
    pub widgets: Vec<Widget>,
}

fn bad(message: impl Into<String>) -> ApiError {
    ApiError::BadRequest(message.into())
}

fn text(config: &Map<String, Value>, key: &str, max: usize) -> ApiResult<Option<String>> {
    match config.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.chars().count() <= max => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(bad(format!("\"{key}\" must be at most {max} characters."))),
        Some(_) => Err(bad(format!("\"{key}\" must be a string."))),
    }
}

fn number(config: &Map<String, Value>, key: &str) -> ApiResult<Option<f64>> {
    match config.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_f64()
            .filter(|v| v.is_finite())
            .map(Some)
            .ok_or_else(|| bad(format!("\"{key}\" must be a finite number."))),
        Some(_) => Err(bad(format!("\"{key}\" must be a number."))),
    }
}

fn required_query(config: &Map<String, Value>) -> ApiResult<String> {
    let query = text(config, "query", MAX_QUERY_CHARS)?.unwrap_or_default();
    if query.trim().is_empty() {
        return Err(bad("A chart or value widget needs a query."));
    }
    Ok(query)
}

/// Ne garde de la configuration d'un widget que les clés de son type, du bon type.
fn clean_config(kind: &str, raw: &Value) -> ApiResult<Value> {
    let empty = Map::new();
    let config = match raw {
        Value::Null => &empty,
        Value::Object(map) => map,
        _ => return Err(bad("A widget's config must be an object.")),
    };
    let mut out = Map::new();
    let mut put = |key: &str, value: Value| {
        out.insert(key.to_string(), value);
    };
    match kind {
        "chart" => {
            put("query", required_query(config)?.into());
            put("unit", text(config, "unit", MAX_UNIT_CHARS)?.unwrap_or_default().into());
        }
        "stat" => {
            put("query", required_query(config)?.into());
            put("unit", text(config, "unit", MAX_UNIT_CHARS)?.unwrap_or_default().into());
            for key in ["warn", "crit"] {
                if let Some(v) = number(config, key)? {
                    put(key, serde_json::json!(v));
                }
            }
            let direction = text(config, "direction", 8)?.unwrap_or_else(|| "above".into());
            if direction != "above" && direction != "below" {
                return Err(bad("\"direction\" must be \"above\" or \"below\"."));
            }
            put("direction", direction.into());
        }
        "group" => {
            put("folder", text(config, "folder", MAX_FOLDER_CHARS)?.unwrap_or_default().into());
        }
        "alerts" => {
            let limit = number(config, "limit")?.unwrap_or(10.0);
            if !(1.0..=50.0).contains(&limit) {
                return Err(bad("\"limit\" must be between 1 and 50."));
            }
            put("limit", serde_json::json!(limit.round() as u32));
        }
        _ => {
            put("markdown", text(config, "markdown", MAX_TEXT_CHARS)?.unwrap_or_default().into());
        }
    }
    Ok(Value::Object(out))
}

/// Valide la grille : types connus, identifiants uniques, cases dans la grille.
pub fn clean_widgets(widgets: Vec<Widget>) -> ApiResult<Vec<Widget>> {
    if widgets.len() > MAX_WIDGETS {
        return Err(bad(format!("At most {MAX_WIDGETS} widgets per dashboard.")));
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(widgets.len());
    for mut widget in widgets {
        let id_ok = !widget.id.is_empty()
            && widget.id.len() <= 40
            && widget.id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        if !id_ok {
            return Err(bad("A widget id is 1 to 40 letters, digits, '-' or '_'."));
        }
        if !seen.insert(widget.id.clone()) {
            return Err(bad(format!("Duplicate widget id \"{}\".", widget.id)));
        }
        if !WIDGET_KINDS.contains(&widget.kind.as_str()) {
            return Err(bad(format!("Unknown widget type \"{}\".", widget.kind)));
        }
        widget.title = widget.title.trim().to_string();
        if widget.title.chars().count() > MAX_TITLE_CHARS {
            return Err(bad(format!("A widget title is at most {MAX_TITLE_CHARS} characters.")));
        }
        if widget.w == 0
            || widget.w > GRID_COLUMNS
            || widget.x >= GRID_COLUMNS
            || widget.x + widget.w > GRID_COLUMNS
            || widget.h == 0
            || widget.h > MAX_HEIGHT
            || widget.y > MAX_ROWS
        {
            return Err(bad("A widget does not fit the 12-column grid."));
        }
        widget.config = clean_config(&widget.kind, &widget.config)?;
        out.push(widget);
    }
    Ok(out)
}

pub fn validate(payload: DashboardPayload) -> ApiResult<DashboardInput> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(bad("Name is required."));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(bad(format!("The name must be at most {MAX_NAME_CHARS} characters.")));
    }
    if !(MIN_RANGE_SECS..=MAX_RANGE_SECS).contains(&payload.range_secs) {
        return Err(bad("The time range must be between 1 minute and 400 days."));
    }
    if payload.refresh_secs != 0 && !(5..=3600).contains(&payload.refresh_secs) {
        return Err(bad("The refresh interval is 0 (off) or 5 to 3600 seconds."));
    }
    Ok(DashboardInput {
        name,
        shared: payload.shared,
        range_secs: payload.range_secs,
        refresh_secs: payload.refresh_secs,
        widgets: clean_widgets(payload.widgets)?,
    })
}

// ------------------------------------------------------------------ droits

/// Qui demande : l'identifiant du compte (`None` pour un jeton) et ses droits.
struct Who {
    user_id: Option<i64>,
    admin: bool,
}

fn who(principal: &Principal) -> Who {
    match principal {
        Principal::User(user) => Who { user_id: Some(user.id), admin: user.role.is_admin() },
        Principal::Token(_) => Who { user_id: None, admin: principal.is_admin() },
    }
}

struct Record {
    id: DashboardId,
    name: String,
    shared: bool,
    owner_id: Option<i64>,
    owner: Option<String>,
    range_secs: i64,
    refresh_secs: i64,
    widgets: String,
    created_at: String,
    updated_at: String,
}

impl Record {
    fn visible_to(&self, who: &Who) -> bool {
        self.shared || (who.user_id.is_some() && self.owner_id == who.user_id)
    }

    fn editable_by(&self, who: &Who) -> bool {
        if self.shared { who.admin } else { who.user_id.is_some() && self.owner_id == who.user_id }
    }

    fn view(self, who: &Who) -> DashboardView {
        let can_edit = self.editable_by(who);
        DashboardView {
            id: self.id,
            name: self.name,
            shared: self.shared,
            owner: self.owner,
            can_edit,
            range_secs: self.range_secs,
            refresh_secs: self.refresh_secs,
            widgets: serde_json::from_str(&self.widgets).unwrap_or_default(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// La lecture d'un tableau, suivie de `$tail` : un littéral, comme sqlx l'exige.
macro_rules! select {
    ($tail:literal) => {
        concat!(
            "SELECT d.id, d.name, d.shared, d.owner_id, u.username AS owner,
                d.range_secs, d.refresh_secs, d.widgets, d.created_at, d.updated_at
             FROM dashboards d LEFT JOIN users u ON u.id = d.owner_id ",
            $tail
        )
    };
}

fn row_of(row: &sqlx::sqlite::SqliteRow) -> Result<Record, sqlx::Error> {
    Ok(Record {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        shared: row.try_get::<i64, _>("shared")? != 0,
        owner_id: row.try_get("owner_id")?,
        owner: row.try_get("owner")?,
        range_secs: row.try_get("range_secs")?,
        refresh_secs: row.try_get("refresh_secs")?,
        widgets: row.try_get("widgets")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

async fn load(pool: &SqlitePool, id: DashboardId) -> ApiResult<Option<Record>> {
    let found = sqlx::query(select!("WHERE d.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(anyhow::Error::from)?;
    Ok(found.as_ref().map(row_of).transpose().map_err(anyhow::Error::from)?)
}

fn not_found(id: DashboardId) -> ApiError {
    ApiError::NotFound(format!("Dashboard {id} not found."))
}

/// Le tableau, s'il existe et que ce porteur le voit (sinon 404, sans révéler
/// l'existence d'un tableau personnel d'autrui).
async fn visible(pool: &SqlitePool, id: DashboardId, who: &Who) -> ApiResult<Record> {
    load(pool, id).await?.filter(|row| row.visible_to(who)).ok_or_else(|| not_found(id))
}

async fn editable(pool: &SqlitePool, id: DashboardId, who: &Who) -> ApiResult<Record> {
    let row = visible(pool, id, who).await?;
    if !row.editable_by(who) {
        return Err(ApiError::Forbidden(
            "Only administrators can change a shared dashboard.".into(),
        ));
    }
    Ok(row)
}

// ---------------------------------------------------------------- gestes

/// `GET /api/dashboards`
async fn list(
    State(state): State<AppState>,
    Identity(principal): Identity,
) -> ApiResult<Json<Vec<DashboardView>>> {
    let who = who(&principal);
    let rows = sqlx::query(select!(
        "WHERE d.shared = 1 OR (? IS NOT NULL AND d.owner_id = ?)
         ORDER BY d.shared DESC, d.name COLLATE NOCASE, d.id"
    ))
    .bind(who.user_id)
    .bind(who.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(anyhow::Error::from)?;
    let views = rows
        .iter()
        .map(|r| row_of(r).map(|row| row.view(&who)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(anyhow::Error::from)?;
    Ok(Json(views))
}

/// `GET /api/dashboards/{id}`
async fn get_one(
    State(state): State<AppState>,
    Identity(principal): Identity,
    Path(id): Path<DashboardId>,
) -> ApiResult<Json<DashboardView>> {
    let who = who(&principal);
    Ok(Json(visible(&state.pool, id, &who).await?.view(&who)))
}

fn check_may_write(who: &Who, shared: bool) -> ApiResult<()> {
    if shared && !who.admin {
        return Err(ApiError::Forbidden(
            "Only administrators can share a dashboard: save it as a personal one.".into(),
        ));
    }
    if !shared && who.user_id.is_none() {
        return Err(ApiError::Forbidden(
            "Personal dashboards belong to accounts: share it, or sign in.".into(),
        ));
    }
    Ok(())
}

fn conflict(error: sqlx::Error, name: &str) -> ApiError {
    if error.as_database_error().is_some_and(|e| e.is_unique_violation()) {
        ApiError::Conflict(format!("A dashboard named \"{name}\" already exists."))
    } else {
        anyhow::Error::from(error).into()
    }
}

/// `POST /api/dashboards`
async fn create(
    State(state): State<AppState>,
    Identity(principal): Identity,
    Json(payload): Json<DashboardPayload>,
) -> ApiResult<(StatusCode, Json<DashboardView>)> {
    let who = who(&principal);
    let input = validate(payload)?;
    check_may_write(&who, input.shared)?;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dashboards WHERE shared = ? AND (? = 1 OR owner_id = ?)",
    )
    .bind(input.shared)
    .bind(input.shared)
    .bind(who.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(anyhow::Error::from)?;
    if count >= MAX_DASHBOARDS {
        return Err(bad(format!("At most {MAX_DASHBOARDS} dashboards.")));
    }
    let widgets = serde_json::to_string(&input.widgets).map_err(anyhow::Error::from)?;
    let id = sqlx::query(
        "INSERT INTO dashboards (name, shared, owner_id, range_secs, refresh_secs, widgets)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&input.name)
    .bind(input.shared)
    .bind(who.user_id)
    .bind(input.range_secs)
    .bind(input.refresh_secs)
    .bind(widgets)
    .execute(&state.pool)
    .await
    .map_err(|e| conflict(e, &input.name))?
    .last_insert_rowid();
    Ok((StatusCode::CREATED, Json(visible(&state.pool, id, &who).await?.view(&who))))
}

/// `PUT /api/dashboards/{id}` — remplace le tableau. Partager ou retirer le
/// partage est réservé aux administrateurs ; un tableau garde son propriétaire.
async fn update(
    State(state): State<AppState>,
    Identity(principal): Identity,
    Path(id): Path<DashboardId>,
    Json(payload): Json<DashboardPayload>,
) -> ApiResult<Json<DashboardView>> {
    let who = who(&principal);
    let input = validate(payload)?;
    let current = editable(&state.pool, id, &who).await?;
    if input.shared != current.shared {
        if !who.admin {
            return Err(ApiError::Forbidden("Only administrators can change sharing.".into()));
        }
        if !input.shared && current.owner_id.is_none() {
            return Err(bad("A dashboard made by a token cannot become personal."));
        }
    }
    let widgets = serde_json::to_string(&input.widgets).map_err(anyhow::Error::from)?;
    sqlx::query(
        "UPDATE dashboards SET name = ?, shared = ?, range_secs = ?, refresh_secs = ?,
             widgets = ?, updated_at = strftime('%Y-%m-%d %H:%M:%S', 'now')
         WHERE id = ?",
    )
    .bind(&input.name)
    .bind(input.shared)
    .bind(input.range_secs)
    .bind(input.refresh_secs)
    .bind(widgets)
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|e| conflict(e, &input.name))?;
    Ok(Json(visible(&state.pool, id, &who).await?.view(&who)))
}

/// `DELETE /api/dashboards/{id}`
async fn remove(
    State(state): State<AppState>,
    Identity(principal): Identity,
    Path(id): Path<DashboardId>,
) -> ApiResult<StatusCode> {
    let who = who(&principal);
    editable(&state.pool, id, &who).await?;
    sqlx::query("DELETE FROM dashboards WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(anyhow::Error::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn widget(id: &str, kind: &str, x: u32, w: u32, config: Value) -> Widget {
        Widget { id: id.into(), kind: kind.into(), title: " t ".into(), x, y: 0, w, h: 4, config }
    }

    fn payload(widgets: Vec<Widget>) -> DashboardPayload {
        DashboardPayload {
            name: " Home ".into(),
            shared: false,
            range_secs: 3600,
            refresh_secs: 30,
            widgets,
        }
    }

    #[test]
    fn a_valid_dashboard_is_trimmed_and_unknown_config_keys_are_dropped() {
        let grid = vec![widget("a", "chart", 0, 6, json!({"query": "up", "unit": "%", "evil": 1}))];
        let Ok(input) = validate(payload(grid)) else { panic!("tableau valide") };
        assert_eq!(input.name, "Home");
        assert_eq!(input.widgets[0].title, "t");
        assert_eq!(input.widgets[0].config, json!({"query": "up", "unit": "%"}));
    }

    #[test]
    fn widgets_must_fit_the_grid_and_be_known() {
        assert!(validate(payload(vec![widget("a", "text", 8, 6, json!({}))])).is_err());
        assert!(validate(payload(vec![widget("a", "text", 0, 0, json!({}))])).is_err());
        assert!(validate(payload(vec![widget("a", "iframe", 0, 3, json!({}))])).is_err());
        assert!(validate(payload(vec![widget("a b", "text", 0, 3, json!({}))])).is_err());
        let twice =
            vec![widget("a", "text", 0, 3, json!({})), widget("a", "text", 3, 3, json!({}))];
        assert!(validate(payload(twice)).is_err());
    }

    #[test]
    fn queries_and_thresholds_are_checked() {
        assert!(validate(payload(vec![widget("a", "stat", 0, 3, json!({}))])).is_err());
        let ok = widget("a", "stat", 0, 3, json!({"query": "up", "warn": 80, "crit": 95}));
        assert!(validate(payload(vec![ok])).is_ok());
        let bad_dir = widget("a", "stat", 0, 3, json!({"query": "up", "direction": "sideways"}));
        assert!(validate(payload(vec![bad_dir])).is_err());
        let stringy = widget("a", "stat", 0, 3, json!({"query": "up", "warn": "high"}));
        assert!(validate(payload(vec![stringy])).is_err());
    }

    #[test]
    fn range_and_refresh_are_bounded() {
        let mut p = payload(vec![]);
        p.range_secs = 10;
        assert!(validate(p).is_err());
        let mut p = payload(vec![]);
        p.refresh_secs = 2;
        assert!(validate(p).is_err());
        let mut p = payload(vec![]);
        p.refresh_secs = 0;
        assert!(validate(p).is_ok());
    }
}
