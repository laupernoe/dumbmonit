//! Emplacements et carte du monde.
//!
//! - `GET /api/locations` : les emplacements, les cibles que chacun réunit, les
//!   sites connus de la carte de topologie et les cibles sans emplacement ;
//! - `POST /api/locations`, `PUT|DELETE /api/locations/{id}` : les créer, les
//!   modifier, les supprimer (administrateurs) ;
//! - `POST /api/locations/assign` : placer un lot de cibles (ou les retirer) ;
//! - `GET /api/locations/geocode?q=` : recherche de lieu, par le même géocodage
//!   Open-Meteo que la météo du mur ;
//! - `GET|PUT /api/locations/basemap` : le fond de carte.
//!
//! Une cible trouve son emplacement du plus explicite au plus déduit : son
//! affectation (`targets.location_id`), puis l'emplacement qui tient lieu de son
//! site distant (le site déduit par la topologie : étiquette `site`, agent,
//! relais), puis l'emplacement « site principal » si elle n'a aucun site
//! distant. Rien n'est recopié : renommer un site ou changer de relais déplace
//! la cible sur la carte au prochain affichage.
//!
//! Le fond de carte par défaut est embarqué dans l'interface (Natural Earth,
//! domaine public) : la carte ne contacte aucun serveur. Les tuiles
//! OpenStreetMap ou celles d'un serveur choisi sont une option ; leur origine
//! est alors ajoutée à `img-src` de la politique de sécurité de l'interface
//! ([`tile_origin`]), et à rien d'autre.

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use dumbmonit_proto::TargetId;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tracing::warn;

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::auth::settings;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/locations", get(list).post(create))
        .route("/locations/assign", post(assign))
        .route("/locations/geocode", get(geocode))
        .route("/locations/basemap", get(get_basemap).put(put_basemap))
        .route("/locations/{id}", put(update).delete(remove))
}

pub type LocationId = i64;

const MAX_NAME_CHARS: usize = 80;
const MAX_ADDRESS_CHARS: usize = 200;
/// Un nom de site : celui d'un agent ou d'une étiquette, jamais très long.
const MAX_SITE_CHARS: usize = 80;
/// Au-delà, un emplacement de plus n'aide plus personne à lire la carte.
const MAX_LOCATIONS: i64 = 500;
const MAX_ASSIGN: usize = 5000;

const BASEMAP_KEY: &str = "map.basemap";
pub const OSM_TILES: &str = "https://tile.openstreetmap.org/{z}/{x}/{y}.png";
pub const OSM_ATTRIBUTION: &str = "© OpenStreetMap contributors";
const MAX_URL_CHARS: usize = 300;
const MAX_ATTRIBUTION_CHARS: usize = 200;

// --------------------------------------------------------------- lecture

#[derive(Debug, Clone, PartialEq)]
pub struct LocationRow {
    pub id: LocationId,
    pub name: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
    pub site: String,
    pub main_site: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Member {
    pub target_id: TargetId,
    /// `assigned` (affectation explicite), `site` (le site de la cible) ou
    /// `main` (aucun site distant, emplacement du site principal).
    pub source: &'static str,
}

#[derive(Debug, Serialize)]
pub struct LocationView {
    pub id: LocationId,
    pub name: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
    /// Site de la topologie dont l'emplacement tient lieu ; vide : aucun.
    pub site: String,
    pub main_site: bool,
    pub members: Vec<Member>,
}

#[derive(Debug, Serialize)]
pub struct LocationsView {
    pub locations: Vec<LocationView>,
    /// Sites distants que la topologie connaît, triés : ce qu'un emplacement
    /// peut représenter.
    pub sites: Vec<String>,
    /// Cibles qu'aucun emplacement ne réunit.
    pub unplaced: Vec<TargetId>,
}

async fn load(pool: &SqlitePool) -> ApiResult<Vec<LocationRow>> {
    let rows = sqlx::query(
        "SELECT id, name, address, latitude, longitude, site, main_site FROM locations
         ORDER BY name COLLATE NOCASE, id",
    )
    .fetch_all(pool)
    .await
    .map_err(anyhow::Error::from)?;
    rows.iter()
        .map(|row| {
            Ok(LocationRow {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                address: row.try_get("address")?,
                latitude: row.try_get("latitude")?,
                longitude: row.try_get("longitude")?,
                site: row.try_get("site")?,
                main_site: row.try_get::<i64, _>("main_site")? != 0,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(|e| anyhow::Error::from(e).into())
}

/// Affectations explicites : cible → emplacement.
pub async fn assignments(pool: &SqlitePool) -> anyhow::Result<HashMap<TargetId, LocationId>> {
    let rows = sqlx::query("SELECT id, location_id FROM targets WHERE location_id IS NOT NULL")
        .fetch_all(pool)
        .await?;
    let mut out = HashMap::with_capacity(rows.len());
    for row in &rows {
        let target: TargetId = row.try_get("id")?;
        let location: LocationId = row.try_get("location_id")?;
        out.insert(target, location);
    }
    Ok(out)
}

/// L'emplacement affecté à une cible, s'il y en a un.
pub async fn location_of(pool: &SqlitePool, id: TargetId) -> anyhow::Result<Option<LocationId>> {
    let found: Option<Option<LocationId>> =
        sqlx::query_scalar("SELECT location_id FROM targets WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    Ok(found.flatten())
}

/// Ce qu'on sait d'une cible pour la placer.
#[derive(Debug, Clone)]
pub struct Placement {
    pub target_id: TargetId,
    pub assigned: Option<LocationId>,
    /// Site distant déduit par la topologie ; `None` : site principal.
    pub site: Option<String>,
}

/// Répartit les cibles entre les emplacements : affectation, puis site, puis
/// site principal. Rend les membres de chaque emplacement et les cibles
/// restées sans emplacement, dans l'ordre reçu.
pub fn resolve(
    locations: &[LocationRow],
    targets: &[Placement],
) -> (HashMap<LocationId, Vec<Member>>, Vec<TargetId>) {
    let known: HashSet<LocationId> = locations.iter().map(|l| l.id).collect();
    // Le premier emplacement (par nom) qui revendique un site l'emporte.
    let mut by_site: HashMap<String, LocationId> = HashMap::new();
    for location in locations {
        let site = location.site.trim().to_lowercase();
        if !site.is_empty() {
            by_site.entry(site).or_insert(location.id);
        }
    }
    let main = locations.iter().find(|l| l.main_site).map(|l| l.id);
    let mut members: HashMap<LocationId, Vec<Member>> = HashMap::new();
    let mut unplaced = Vec::new();
    for target in targets {
        let found = match target.assigned.filter(|id| known.contains(id)) {
            Some(id) => Some((id, "assigned")),
            None => match &target.site {
                Some(site) => by_site.get(&site.trim().to_lowercase()).map(|id| (*id, "site")),
                None => main.map(|id| (id, "main")),
            },
        };
        match found {
            Some((id, source)) => {
                members.entry(id).or_default().push(Member { target_id: target.target_id, source })
            }
            None => unplaced.push(target.target_id),
        }
    }
    (members, unplaced)
}

/// L'emplacement où se trouve une cible (affectation, site ou site
/// principal), s'il y en a un : la carte en direct d'un pare-feu y ancre ses arcs.
pub async fn location_for(state: &AppState, id: TargetId) -> ApiResult<Option<LocationRow>> {
    let locations = load(&state.pool).await?;
    if locations.is_empty() {
        return Ok(None);
    }
    let site = super::topology::target_sites(state).await?.remove(&id).flatten();
    let placement =
        Placement { target_id: id, assigned: location_of(&state.pool, id).await?, site };
    let (members, _) = resolve(&locations, &[placement]);
    Ok(members.keys().next().and_then(|found| locations.into_iter().find(|l| l.id == *found)))
}

/// `GET /api/locations` — lecture, ouverte à tous les rôles comme la carte.
async fn list(State(state): State<AppState>) -> ApiResult<Json<LocationsView>> {
    let locations = load(&state.pool).await?;
    let sites = super::topology::target_sites(&state).await?;
    let assigned = assignments(&state.pool).await?;
    let mut ids: Vec<TargetId> = sites.keys().copied().collect();
    ids.sort_unstable();
    let placements: Vec<Placement> = ids
        .iter()
        .map(|id| Placement {
            target_id: *id,
            assigned: assigned.get(id).copied(),
            site: sites.get(id).cloned().flatten(),
        })
        .collect();
    let (mut members, unplaced) = resolve(&locations, &placements);
    let mut known_sites: Vec<String> = Vec::new();
    for site in sites.values().flatten() {
        if !known_sites.iter().any(|s| s.eq_ignore_ascii_case(site)) {
            known_sites.push(site.clone());
        }
    }
    known_sites.sort_by_key(|s| s.to_lowercase());
    let locations = locations
        .into_iter()
        .map(|l| LocationView {
            members: members.remove(&l.id).unwrap_or_default(),
            id: l.id,
            name: l.name,
            address: l.address,
            latitude: l.latitude,
            longitude: l.longitude,
            site: l.site,
            main_site: l.main_site,
        })
        .collect();
    Ok(Json(LocationsView { locations, sites: known_sites, unplaced }))
}

// --------------------------------------------------------------- écriture

#[derive(Debug, Deserialize)]
pub struct LocationPayload {
    pub name: String,
    #[serde(default)]
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub site: String,
    #[serde(default)]
    pub main_site: bool,
}

#[derive(Debug, PartialEq)]
struct LocationInput {
    name: String,
    address: String,
    latitude: f64,
    longitude: f64,
    site: String,
    main_site: bool,
}

fn validate(payload: LocationPayload) -> ApiResult<LocationInput> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::BadRequest("Name is required.".into()));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(ApiError::BadRequest(format!(
            "The name must be at most {MAX_NAME_CHARS} characters."
        )));
    }
    let address = payload.address.trim().to_string();
    if address.chars().count() > MAX_ADDRESS_CHARS {
        return Err(ApiError::BadRequest(format!(
            "The address must be at most {MAX_ADDRESS_CHARS} characters."
        )));
    }
    let site = payload.site.trim().to_string();
    if site.chars().count() > MAX_SITE_CHARS {
        return Err(ApiError::BadRequest(format!(
            "The site name must be at most {MAX_SITE_CHARS} characters."
        )));
    }
    if !(payload.latitude.is_finite() && (-90.0..=90.0).contains(&payload.latitude)) {
        return Err(ApiError::BadRequest("The latitude must be between -90 and 90.".into()));
    }
    if !(payload.longitude.is_finite() && (-180.0..=180.0).contains(&payload.longitude)) {
        return Err(ApiError::BadRequest("The longitude must be between -180 and 180.".into()));
    }
    // Six décimales : une dizaine de centimètres, bien assez pour une pastille.
    let round = |v: f64| (v * 1e6).round() / 1e6;
    Ok(LocationInput {
        name,
        address,
        latitude: round(payload.latitude),
        longitude: round(payload.longitude),
        site,
        main_site: payload.main_site,
    })
}

fn name_conflict(error: sqlx::Error) -> ApiError {
    if error.to_string().contains("UNIQUE constraint failed: locations.name") {
        ApiError::Conflict("A location with this name already exists.".into())
    } else {
        ApiError::Internal(error.into())
    }
}

/// Écrit l'emplacement (nouveau si `id` est `None`) ; un seul site principal :
/// le marquer retire la marque des autres, dans la même transaction.
async fn save(
    pool: &SqlitePool,
    id: Option<LocationId>,
    input: &LocationInput,
) -> ApiResult<LocationId> {
    let mut tx = pool.begin().await.map_err(anyhow::Error::from)?;
    if input.main_site {
        sqlx::query("UPDATE locations SET main_site = 0 WHERE main_site = 1 AND id IS NOT ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(anyhow::Error::from)?;
    }
    let id = match id {
        Some(id) => {
            let done = sqlx::query(
                "UPDATE locations SET name = ?, address = ?, latitude = ?, longitude = ?,
                     site = ?, main_site = ? WHERE id = ?",
            )
            .bind(&input.name)
            .bind(&input.address)
            .bind(input.latitude)
            .bind(input.longitude)
            .bind(&input.site)
            .bind(input.main_site)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(name_conflict)?;
            if done.rows_affected() == 0 {
                return Err(ApiError::NotFound(format!("Location {id} not found.")));
            }
            id
        }
        None => sqlx::query_scalar::<_, LocationId>(
            "INSERT INTO locations (name, address, latitude, longitude, site, main_site)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(&input.name)
        .bind(&input.address)
        .bind(input.latitude)
        .bind(input.longitude)
        .bind(&input.site)
        .bind(input.main_site)
        .fetch_one(&mut *tx)
        .await
        .map_err(name_conflict)?,
    };
    tx.commit().await.map_err(anyhow::Error::from)?;
    Ok(id)
}

async fn view(pool: &SqlitePool, id: LocationId) -> ApiResult<LocationView> {
    let row = load(pool)
        .await?
        .into_iter()
        .find(|l| l.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("Location {id} not found.")))?;
    Ok(LocationView {
        id: row.id,
        name: row.name,
        address: row.address,
        latitude: row.latitude,
        longitude: row.longitude,
        site: row.site,
        main_site: row.main_site,
        members: Vec::new(),
    })
}

/// `POST /api/locations`
async fn create(
    State(state): State<AppState>,
    Json(payload): Json<LocationPayload>,
) -> ApiResult<(StatusCode, Json<LocationView>)> {
    let input = validate(payload)?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM locations")
        .fetch_one(&state.pool)
        .await
        .map_err(anyhow::Error::from)?;
    if count >= MAX_LOCATIONS {
        return Err(ApiError::BadRequest(format!("At most {MAX_LOCATIONS} locations.")));
    }
    let id = save(&state.pool, None, &input).await?;
    Ok((StatusCode::CREATED, Json(view(&state.pool, id).await?)))
}

/// `PUT /api/locations/{id}`
async fn update(
    State(state): State<AppState>,
    Path(id): Path<LocationId>,
    Json(payload): Json<LocationPayload>,
) -> ApiResult<Json<LocationView>> {
    let input = validate(payload)?;
    save(&state.pool, Some(id), &input).await?;
    Ok(Json(view(&state.pool, id).await?))
}

/// `DELETE /api/locations/{id}` — ses cibles restent, sans emplacement.
async fn remove(
    State(state): State<AppState>,
    Path(id): Path<LocationId>,
) -> ApiResult<StatusCode> {
    let mut tx = state.pool.begin().await.map_err(anyhow::Error::from)?;
    // La clé étrangère le ferait seule ; l'écrire évite d'en dépendre.
    sqlx::query("UPDATE targets SET location_id = NULL WHERE location_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(anyhow::Error::from)?;
    let done = sqlx::query("DELETE FROM locations WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(anyhow::Error::from)?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("Location {id} not found.")));
    }
    tx.commit().await.map_err(anyhow::Error::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct AssignPayload {
    /// `null` : retirer l'affectation (la cible retombe sur son site).
    pub location_id: Option<LocationId>,
    pub target_ids: Vec<TargetId>,
}

#[derive(Debug, Serialize)]
pub struct AssignReport {
    pub updated: u64,
}

/// Vérifie qu'un emplacement existe, avant que la clé étrangère n'en fasse une
/// erreur 500.
pub async fn check_location(pool: &SqlitePool, id: Option<LocationId>) -> ApiResult<()> {
    let Some(id) = id else { return Ok(()) };
    let found: Option<i64> = sqlx::query_scalar("SELECT id FROM locations WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(anyhow::Error::from)?;
    match found {
        Some(_) => Ok(()),
        None => Err(ApiError::BadRequest(format!("Location {id} not found."))),
    }
}

/// Affecte (ou retire) l'emplacement d'une liste de cibles.
pub async fn set_location(
    pool: &SqlitePool,
    targets: &[TargetId],
    location: Option<LocationId>,
) -> anyhow::Result<u64> {
    let mut tx = pool.begin().await?;
    let mut updated = 0;
    for id in targets {
        updated += sqlx::query("UPDATE targets SET location_id = ? WHERE id = ?")
            .bind(location)
            .bind(id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
    }
    tx.commit().await?;
    Ok(updated)
}

/// `POST /api/locations/assign` — le mode Sélection de la liste des moniteurs.
async fn assign(
    State(state): State<AppState>,
    Json(payload): Json<AssignPayload>,
) -> ApiResult<Json<AssignReport>> {
    if payload.target_ids.is_empty() {
        return Err(ApiError::BadRequest("Pick at least one monitor.".into()));
    }
    if payload.target_ids.len() > MAX_ASSIGN {
        return Err(ApiError::BadRequest(format!("At most {MAX_ASSIGN} monitors at once.")));
    }
    check_location(&state.pool, payload.location_id).await?;
    let updated = set_location(&state.pool, &payload.target_ids, payload.location_id).await?;
    Ok(Json(AssignReport { updated }))
}

#[derive(Debug, Deserialize)]
struct GeocodeQuery {
    q: String,
    #[serde(default)]
    lang: Option<String>,
}

/// `GET /api/locations/geocode?q=Lausanne&lang=fr` — administrateurs : la
/// requête part chez Open-Meteo depuis le serveur, jamais depuis le navigateur.
async fn geocode(
    AdminIdentity(_): AdminIdentity,
    Query(query): Query<GeocodeQuery>,
) -> ApiResult<Json<Vec<super::wall_widgets::Place>>> {
    Ok(Json(super::wall_widgets::search_places(&query.q, query.lang.as_deref()).await?))
}

// --------------------------------------------------------------- fond de carte

/// Le fond de carte : `offline` (embarqué, par défaut), `osm` (tuiles
/// OpenStreetMap) ou `custom` (une adresse de tuiles `{z}/{x}/{y}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Basemap {
    pub mode: String,
    /// Adresse des tuiles ; imposée pour `osm`, vide pour `offline`.
    #[serde(default)]
    pub url: String,
    /// Mention à afficher sur la carte ; imposée pour `osm`.
    #[serde(default)]
    pub attribution: String,
}

impl Default for Basemap {
    fn default() -> Self {
        Self { mode: "offline".into(), url: String::new(), attribution: String::new() }
    }
}

/// L'origine des tuiles en cours, pour `img-src` : `None` hors ligne.
static TILE_ORIGIN: RwLock<Option<String>> = RwLock::new(None);

/// Origine à ajouter à la politique de sécurité de l'interface.
pub fn tile_origin() -> Option<String> {
    TILE_ORIGIN.read().ok().and_then(|o| o.clone())
}

fn remember_origin(basemap: &Basemap) {
    if let Ok(mut slot) = TILE_ORIGIN.write() {
        *slot = origin(&basemap.url).filter(|_| basemap.mode != "offline");
    }
}

/// Lit le fond de carte enregistré au démarrage, pour la politique de sécurité.
pub async fn load_basemap(pool: &SqlitePool) {
    match settings::get::<Basemap>(pool, BASEMAP_KEY).await {
        Ok(found) => remember_origin(&found.unwrap_or_default()),
        Err(error) => warn!(?error, "fond de carte illisible, carte hors ligne"),
    }
}

/// `schéma://hôte[:port]` d'une adresse de tuiles, si elle n'a rien d'autre
/// qu'un hôte ordinaire : c'est ce qui entre tel quel dans l'en-tête CSP.
pub fn origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if scheme != "https" && scheme != "http" {
        return None;
    }
    let host = rest.split('/').next()?;
    let ok = !host.is_empty()
        && host.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'))
        && !host.starts_with(['.', '-', ':']);
    ok.then(|| format!("{scheme}://{}", host.to_ascii_lowercase()))
}

fn validate_basemap(mut basemap: Basemap) -> Result<Basemap, String> {
    match basemap.mode.as_str() {
        "offline" => Ok(Basemap::default()),
        "osm" => {
            basemap.url = OSM_TILES.into();
            basemap.attribution = OSM_ATTRIBUTION.into();
            Ok(basemap)
        }
        "custom" => {
            let url = basemap.url.trim().to_string();
            if url.chars().count() > MAX_URL_CHARS {
                return Err(format!(
                    "The tile address must be at most {MAX_URL_CHARS} characters."
                ));
            }
            if url.chars().any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | ';' | ',' | '<')) {
                return Err("The tile address contains a character it cannot hold.".into());
            }
            if origin(&url).is_none() {
                return Err(
                    "The tile address must start with https:// (or http://) and a plain host name."
                        .into(),
                );
            }
            if !["{z}", "{x}", "{y}"].iter().all(|p| url.contains(p)) {
                return Err("The tile address needs {z}, {x} and {y}.".into());
            }
            let attribution = basemap.attribution.trim().to_string();
            if attribution.is_empty() {
                return Err("Say whose tiles these are: the attribution is required.".into());
            }
            if attribution.chars().count() > MAX_ATTRIBUTION_CHARS {
                return Err(format!(
                    "The attribution must be at most {MAX_ATTRIBUTION_CHARS} characters."
                ));
            }
            Ok(Basemap { mode: "custom".into(), url, attribution })
        }
        other => Err(format!("Unknown basemap \"{other}\" (offline, osm or custom).")),
    }
}

/// `GET /api/locations/basemap` — tous les rôles : la carte en a besoin.
async fn get_basemap(State(state): State<AppState>) -> ApiResult<Json<Basemap>> {
    Ok(Json(settings::get(&state.pool, BASEMAP_KEY).await?.unwrap_or_default()))
}

/// `PUT /api/locations/basemap`
async fn put_basemap(
    State(state): State<AppState>,
    Json(payload): Json<Basemap>,
) -> ApiResult<Json<Basemap>> {
    let basemap = validate_basemap(payload).map_err(ApiError::BadRequest)?;
    settings::set(&state.pool, BASEMAP_KEY, &basemap).await?;
    remember_origin(&basemap);
    Ok(Json(basemap))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location(id: LocationId, name: &str, site: &str, main: bool) -> LocationRow {
        LocationRow {
            id,
            name: name.into(),
            address: String::new(),
            latitude: 46.5,
            longitude: 6.6,
            site: site.into(),
            main_site: main,
        }
    }

    fn placed(id: TargetId, assigned: Option<LocationId>, site: Option<&str>) -> Placement {
        Placement { target_id: id, assigned, site: site.map(Into::into) }
    }

    #[test]
    fn affectation_puis_site_puis_site_principal() {
        let locations =
            [location(1, "Siège", "", true), location(2, "Annexe", "annexe-relais", false)];
        let targets = [
            placed(10, None, None),
            placed(11, None, Some("Annexe-Relais")),
            placed(12, Some(2), None),
            placed(13, None, Some("Cave")),
            placed(14, Some(99), None),
        ];
        let (members, unplaced) = resolve(&locations, &targets);
        assert_eq!(
            members[&1],
            vec![
                Member { target_id: 10, source: "main" },
                Member { target_id: 14, source: "main" }
            ],
            "une affectation vers un emplacement disparu retombe sur le site"
        );
        assert_eq!(
            members[&2],
            vec![
                Member { target_id: 11, source: "site" },
                Member { target_id: 12, source: "assigned" }
            ]
        );
        assert_eq!(unplaced, vec![13], "un site sans emplacement reste à placer");
    }

    #[test]
    fn sans_site_principal_rien_nest_place_par_defaut() {
        let (members, unplaced) =
            resolve(&[location(1, "Annexe", "annexe", false)], &[placed(1, None, None)]);
        assert!(members.is_empty());
        assert_eq!(unplaced, vec![1]);
    }

    #[test]
    fn les_coordonnees_sont_bornees_et_arrondies() {
        let payload = |lat: f64, lon: f64| LocationPayload {
            name: " Siège ".into(),
            address: String::new(),
            latitude: lat,
            longitude: lon,
            site: String::new(),
            main_site: false,
        };
        let Ok(ok) = validate(payload(46.516_012_345, 6.632_8)) else { panic!("valide") };
        assert_eq!(ok.name, "Siège");
        assert_eq!(ok.latitude, 46.516_012);
        assert!(validate(payload(91.0, 0.0)).is_err());
        assert!(validate(payload(0.0, -180.5)).is_err());
        assert!(validate(payload(f64::NAN, 0.0)).is_err());
        let mut empty = payload(0.0, 0.0);
        empty.name = "  ".into();
        assert!(validate(empty).is_err());
    }

    #[test]
    fn lorigine_des_tuiles_est_un_hote_ordinaire() {
        assert_eq!(
            origin("https://tile.openstreetmap.org/{z}/{x}/{y}.png").as_deref(),
            Some("https://tile.openstreetmap.org")
        );
        assert_eq!(
            origin("http://Tiles.lan:8080/{z}/{x}/{y}").as_deref(),
            Some("http://tiles.lan:8080")
        );
        assert_eq!(origin("https://{s}.tile.example/{z}/{x}/{y}.png"), None);
        assert_eq!(origin("https://*.example/{z}"), None);
        assert_eq!(origin("javascript://x/{z}"), None);
        assert_eq!(origin("https://a b/{z}"), None);
        assert_eq!(origin("https:///{z}"), None);
    }

    #[test]
    fn le_fond_de_carte_est_valide_ou_refuse() {
        let custom = |url: &str, attribution: &str| Basemap {
            mode: "custom".into(),
            url: url.into(),
            attribution: attribution.into(),
        };
        let osm = validate_basemap(Basemap {
            mode: "osm".into(),
            url: "https://evil.example/{z}/{x}/{y}".into(),
            attribution: String::new(),
        })
        .expect("osm");
        assert_eq!(osm.url, OSM_TILES, "l'adresse OSM est imposée");
        assert_eq!(osm.attribution, OSM_ATTRIBUTION);
        let accepted = validate_basemap(custom("https://x/{z}/{x}/{y}", "x")).map(|b| b.mode);
        assert_eq!(accepted.as_deref(), Ok("custom"));
        assert!(validate_basemap(custom("https://tiles.lan/{z}/{x}.png", "Moi")).is_err());
        assert!(validate_basemap(custom("https://tiles.lan/{z}/{x}/{y}.png", " ")).is_err());
        assert!(validate_basemap(custom("https://tiles.lan/{z}/{x}/{y}.png; x", "Moi")).is_err());
        assert!(validate_basemap(Basemap { mode: "bing".into(), ..Basemap::default() }).is_err());
        let back = validate_basemap(Basemap {
            mode: "offline".into(),
            url: "https://x/{z}/{x}/{y}".into(),
            attribution: "x".into(),
        })
        .expect("hors ligne");
        assert_eq!(back, Basemap::default(), "hors ligne n'emporte aucune adresse");
    }
}
