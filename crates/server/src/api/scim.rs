//! `/api/scim/v2` : provisionnement SCIM 2.0 (RFC 7643, RFC 7644) des comptes
//! et des équipes par un fournisseur d'identité — Entra ID, Okta.
//!
//! Ces routes sont hors du garde de session : le fournisseur présente son jeton
//! dédié (`Bearer dmscim_…`, voir [`crate::auth::scim`]), vérifié par
//! [`require_scim_token`] avant toute lecture du corps. SCIM désactivé : 404.
//!
//! Correspondances :
//!
//! - un `User` SCIM est un compte (`users`) dont `scim_id` est renseigné ;
//!   `userName` → identifiant, `displayName` (ou `name`) → nom affiché,
//!   `active: false` → compte désactivé et sessions fermées, `DELETE` → compte
//!   supprimé (Entra ID et Okta envoient d'abord `active: false` ; `DELETE`
//!   est la suppression définitive, côté fournisseur aussi) ;
//! - un `Group` SCIM est une équipe (`teams`), ses membres y sont inscrits avec
//!   la source `scim`.
//!
//! Garde-fous, vérifiés ici et repris dans chaque écriture SQL (un
//! administrateur peut promouvoir un compte pendant qu'une requête SCIM est en
//! vol) :
//!
//! - un administrateur n'est jamais créé, modifié, désactivé ni supprimé : le
//!   rôle des comptes créés est le réglage `default_role` (lecteur ou
//!   opérateur), un compte existant de même identifiant n'est rattaché que s'il
//!   n'est pas administrateur, et un administrateur n'est jamais inscrit dans
//!   une équipe par SCIM ;
//! - aucun mot de passe n'est lu : un compte provisionné se connecte par le SSO
//!   (OIDC ou SAML), qui le retrouve par son identifiant ou son courriel ;
//! - un membre inscrit à la main (`manual`) n'est jamais retiré, et les droits
//!   des équipes sur les dossiers restent l'affaire de l'administrateur ;
//! - un compte désactivé par un administrateur n'est pas réactivé par SCIM.
//!
//! Le fournisseur ne voit que ce qu'il a provisionné ou rattaché.

use std::time::Instant;

use anyhow::Context;
use axum::body::Bytes;
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, Sqlite, SqlitePool, Transaction};

use super::oidc::Origin;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::AdminUser;
use crate::auth::rate_limit::Key;
use crate::auth::scim::filter::{self, Comparison, Literal};
use crate::auth::scim::{self, DefaultRole};
use crate::auth::users::{self, Role};
use crate::auth::{AuthError, AuthResult, AuthState, audit, teams, token};
use crate::state::AppState;

const USER_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
const GROUP_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:Group";
const LIST_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:ListResponse";
const ERROR_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:Error";
const SCHEMA_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:Schema";
const RESOURCE_TYPE_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:ResourceType";
const CONFIG_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig";

/// Type de contenu des réponses (RFC 7644 §3.1).
const SCIM_CONTENT_TYPE: &str = "application/scim+json";

/// Résultats par page, au plus.
const MAX_RESULTS: usize = 200;
/// Opérations par `PATCH`, au plus.
const MAX_OPERATIONS: usize = 1_000;
/// Membres désignés par une requête, au plus.
const MAX_MEMBERS: usize = 10_000;
const MAX_DISPLAY_CHARS: usize = 200;
const MAX_EXTERNAL_ID_CHARS: usize = 256;
const MAX_EMAIL_CHARS: usize = 254;

/// Auteur des entrées d'audit écrites par le fournisseur.
const ACTOR: &str = "scim";
/// Seau du limiteur des échecs : un identifiant qu'aucun compte ne peut porter
/// (caractère de contrôle), pour ne jamais partager le seau d'une connexion.
const LIMITER_KEY: &str = "\u{0}scim";

/// Routes SCIM, sous leur propre garde.
pub fn routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/scim/v2/ServiceProviderConfig", get(service_provider_config))
        .route("/scim/v2/ResourceTypes", get(resource_types))
        .route("/scim/v2/Schemas", get(schemas))
        .route("/scim/v2/Users", get(list_users).post(create_user))
        .route(
            "/scim/v2/Users/{id}",
            get(get_user).put(replace_user).patch(patch_user).delete(delete_user),
        )
        .route("/scim/v2/Groups", get(list_groups).post(create_group))
        .route(
            "/scim/v2/Groups/{id}",
            get(get_group).put(replace_group).patch(patch_group).delete(delete_group),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_scim_token))
}

// ---------------------------------------------------------------------------
// Erreurs et réponses
// ---------------------------------------------------------------------------

/// Erreur au format SCIM (`urn:ietf:params:scim:api:messages:2.0:Error`).
#[derive(Debug)]
pub struct ScimError {
    status: StatusCode,
    scim_type: Option<&'static str>,
    detail: String,
    retry_after: Option<u64>,
}

type ScimResult<T> = Result<T, ScimError>;

impl ScimError {
    fn new(status: StatusCode, scim_type: Option<&'static str>, detail: impl Into<String>) -> Self {
        Self { status, scim_type, detail: detail.into(), retry_after: None }
    }

    fn invalid_value(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, Some("invalidValue"), detail)
    }

    fn invalid_syntax(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, Some("invalidSyntax"), detail)
    }

    fn invalid_filter(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, Some("invalidFilter"), detail)
    }

    fn mutability(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, Some("mutability"), detail)
    }

    fn uniqueness(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, Some("uniqueness"), detail)
    }

    fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, None, "Resource not found.")
    }

    /// Le refus opposé à toute écriture sur un administrateur.
    fn admin() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            None,
            "Administrator accounts cannot be provisioned, changed or deleted through SCIM.",
        )
    }

    fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, None, "Missing or invalid SCIM bearer token.")
    }

    fn too_many(retry_after: u64) -> Self {
        let mut error = Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            None,
            format!("Too many failed attempts. Try again in {retry_after} seconds."),
        );
        error.retry_after = Some(retry_after);
        error
    }
}

impl IntoResponse for ScimError {
    fn into_response(self) -> Response {
        let mut body = json!({
            "schemas": [ERROR_SCHEMA],
            "status": self.status.as_u16().to_string(),
            "detail": self.detail,
        });
        if let Some(scim_type) = self.scim_type {
            body["scimType"] = json!(scim_type);
        }
        let mut response = scim_response(self.status, &body);
        if self.status == StatusCode::UNAUTHORIZED {
            response.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                HeaderValue::from_static("Bearer realm=\"DumbMonit SCIM\""),
            );
        }
        if let Some(seconds) = self.retry_after
            && let Ok(value) = HeaderValue::from_str(&seconds.to_string())
        {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        response
    }
}

/// Toute autre erreur : 500, le détail au journal seulement.
impl<E: Into<anyhow::Error>> From<E> for ScimError {
    fn from(error: E) -> Self {
        let error: anyhow::Error = error.into();
        tracing::error!(error = %format!("{error:#}"), "erreur interne SCIM");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, None, "Internal server error.")
    }
}

fn scim_response(status: StatusCode, body: &Value) -> Response {
    let mut response = (status, body.to_string()).into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(SCIM_CONTENT_TYPE));
    response
}

/// Réponse `201 Created` avec son en-tête `Location`.
fn created(resource: &Value) -> Response {
    let mut response = scim_response(StatusCode::CREATED, resource);
    if let Some(location) = resource["meta"]["location"].as_str()
        && let Ok(value) = HeaderValue::from_str(location)
    {
        response.headers_mut().insert(header::LOCATION, value);
    }
    response
}

fn base_url(origin: &Origin) -> String {
    format!("{}/api/scim/v2", origin.as_str())
}

/// Date de la base en RFC 3339 : `2026-10-26 07:30:47` → `2026-10-26T07:30:47Z`.
fn timestamp(raw: &str) -> String {
    let at = raw.trim().replace(' ', "T");
    if at.ends_with('Z') || at.contains('+') { at } else { format!("{at}Z") }
}

/// Identifiant SCIM : un UUID v4, sans lien avec l'identifiant interne.
fn new_scim_id() -> String {
    let mut bytes: [u8; 16] = rand::random();
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = hex::encode(bytes);
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

// ---------------------------------------------------------------------------
// Garde
// ---------------------------------------------------------------------------

/// Exige le jeton SCIM. Désactivé : 404 ; jeton absent ou faux : 401 ; trop
/// d'échecs depuis la même adresse : 429.
async fn require_scim_token(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    request: Request,
    next: Next,
) -> Response {
    let ip = token::request_ip(request.extensions(), request.headers());
    let presented = scim::extract_bearer(request.headers()).map(str::to_string);
    match authorize(&state, &auth, ip, presented.as_deref()).await {
        Ok(()) => next.run(request).await,
        Err(error) => error.into_response(),
    }
}

async fn authorize(
    state: &AppState,
    auth: &AuthState,
    ip: Option<std::net::IpAddr>,
    presented: Option<&str>,
) -> ScimResult<()> {
    if !scim::config(&state.pool).await?.enabled {
        // Désactivé : rien ici, jeton ou pas.
        return Err(ScimError::not_found());
    }
    let keys = [Key::User(LIMITER_KEY.to_string(), ip)];
    if let Err(retry_after) = auth.limiter().lock().await.check(&keys, Instant::now()) {
        return Err(ScimError::too_many(retry_after));
    }
    let stored = scim::token(&state.pool).await?;
    let valid = match (stored.as_ref(), presented) {
        (Some(stored), Some(presented)) => scim::token_matches(stored, presented),
        _ => false,
    };
    let mut limiter = auth.limiter().lock().await;
    if valid {
        limiter.record_success(&keys);
        return Ok(());
    }
    limiter.record_failure(&keys, Instant::now());
    drop(limiter);
    tracing::warn!(ip = ?ip, "SCIM request refused: missing or wrong token");
    Err(ScimError::unauthorized())
}

// ---------------------------------------------------------------------------
// Lecture des corps
// ---------------------------------------------------------------------------

fn json_body(bytes: &Bytes) -> ScimResult<Value> {
    serde_json::from_slice(bytes)
        .map_err(|_| ScimError::invalid_syntax("The body is not valid JSON."))
}

fn object(value: &Value) -> ScimResult<&Map<String, Value>> {
    value.as_object().ok_or_else(|| ScimError::invalid_syntax("The body must be a JSON object."))
}

/// Un attribut, nom insensible à la casse (RFC 7643 §2.1) ; `null` vaut absent.
fn field<'a>(object: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    object
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value)
        .filter(|value| !value.is_null())
}

fn text(value: &Value, what: &str) -> ScimResult<String> {
    value
        .as_str()
        .map(|text| text.trim().to_string())
        .ok_or_else(|| ScimError::invalid_value(format!("\"{what}\" must be a string.")))
}

/// Nom affiché : sans caractère de contrôle, borné.
fn clean_display(raw: &str) -> String {
    let clean: String = raw.chars().filter(|c| !c.is_control()).collect();
    clean.trim().chars().take(MAX_DISPLAY_CHARS).collect()
}

/// `name.formatted`, à défaut prénom et nom.
fn name_display(name: Option<&Value>) -> String {
    let Some(name) = name.and_then(Value::as_object) else { return String::new() };
    if let Some(formatted) = field(name, "formatted").and_then(Value::as_str) {
        let formatted = clean_display(formatted);
        if !formatted.is_empty() {
            return formatted;
        }
    }
    let parts: Vec<String> = ["givenName", "familyName"]
        .iter()
        .filter_map(|key| field(name, key).and_then(Value::as_str))
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect();
    clean_display(&parts.join(" "))
}

/// `active` : booléen, ou chaîne `"True"`/`"False"` comme l'envoie Entra ID.
fn parse_active(value: &Value) -> ScimResult<bool> {
    match value {
        Value::Bool(active) => Ok(*active),
        Value::String(text) if text.trim().eq_ignore_ascii_case("true") => Ok(true),
        Value::String(text) if text.trim().eq_ignore_ascii_case("false") => Ok(false),
        _ => Err(ScimError::invalid_value("\"active\" must be true or false.")),
    }
}

fn parse_external_id(value: &Value) -> ScimResult<Option<String>> {
    let id = text(value, "externalId")?;
    if id.chars().count() > MAX_EXTERNAL_ID_CHARS || id.chars().any(char::is_control) {
        return Err(ScimError::invalid_value("\"externalId\" is invalid or too long."));
    }
    Ok((!id.is_empty()).then_some(id))
}

fn clean_email(raw: &str) -> ScimResult<Option<String>> {
    let email = raw.trim();
    if email.is_empty() {
        return Ok(None);
    }
    if email.chars().count() > MAX_EMAIL_CHARS
        || email.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(ScimError::invalid_value("Invalid email address."));
    }
    Ok(Some(email.to_string()))
}

/// Le courriel retenu d'une liste `emails` : le principal, sinon celui de type
/// `work`, sinon le premier.
fn pick_email(value: &Value) -> ScimResult<Option<String>> {
    let entries: Vec<&Map<String, Value>> = match value {
        Value::Array(items) => items.iter().filter_map(Value::as_object).collect(),
        Value::Object(entry) => vec![entry],
        Value::String(email) => return clean_email(email),
        _ => return Err(ScimError::invalid_value("\"emails\" must be a list.")),
    };
    let is_true = |value: Option<&Value>| match value {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::String(text)) => text.eq_ignore_ascii_case("true"),
        _ => false,
    };
    let is_work = |entry: &&&Map<String, Value>| {
        field(entry, "type").and_then(Value::as_str).is_some_and(|t| t.eq_ignore_ascii_case("work"))
    };
    let chosen = entries
        .iter()
        .find(|entry| is_true(field(entry, "primary")))
        .or_else(|| entries.iter().find(is_work))
        .or_else(|| entries.first());
    match chosen.and_then(|entry| field(entry, "value")).and_then(Value::as_str) {
        Some(email) => clean_email(email),
        None => Ok(None),
    }
}

/// Une opération de `PATCH` (RFC 7644 §3.5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpKind {
    Add,
    Replace,
    Remove,
}

struct Operation {
    kind: OpKind,
    path: Option<String>,
    value: Option<Value>,
}

/// Lit les opérations d'un `PATCH`. L'opération est insensible à la casse
/// (Entra ID envoie `Replace`).
fn parse_patch(body: &Value) -> ScimResult<Vec<Operation>> {
    let operations = field(object(body)?, "Operations")
        .and_then(Value::as_array)
        .ok_or_else(|| ScimError::invalid_syntax("\"Operations\" must be a list."))?;
    if operations.is_empty() || operations.len() > MAX_OPERATIONS {
        return Err(ScimError::invalid_syntax(format!(
            "A PATCH request carries from 1 to {MAX_OPERATIONS} operations."
        )));
    }
    operations
        .iter()
        .map(|operation| {
            let operation = operation
                .as_object()
                .ok_or_else(|| ScimError::invalid_syntax("Each operation must be an object."))?;
            let op = field(operation, "op").and_then(Value::as_str).map(str::trim);
            let kind = match op.map(str::to_ascii_lowercase).as_deref() {
                Some("add") => OpKind::Add,
                Some("replace") => OpKind::Replace,
                Some("remove") => OpKind::Remove,
                _ => {
                    return Err(ScimError::invalid_syntax(
                        "Each operation needs an \"op\": add, replace or remove.",
                    ));
                }
            };
            let path = field(operation, "path")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(str::to_string);
            if kind == OpKind::Remove && path.is_none() {
                return Err(ScimError::new(
                    StatusCode::BAD_REQUEST,
                    Some("noTarget"),
                    "A remove operation needs a \"path\".",
                ));
            }
            Ok(Operation { kind, path, value: field(operation, "value").cloned() })
        })
        .collect()
}

/// L'objet d'une opération sans `path` : ses clés sont des attributs.
fn operation_object(operation: &Operation) -> ScimResult<&Map<String, Value>> {
    operation.value.as_ref().and_then(Value::as_object).ok_or_else(|| {
        ScimError::invalid_syntax("An operation without \"path\" needs an object \"value\".")
    })
}

/// La valeur exigée par un ajout ou un remplacement.
fn needed<'a>(value: Option<&'a Value>, path: &str) -> ScimResult<&'a Value> {
    value.filter(|value| !value.is_null()).ok_or_else(|| {
        ScimError::invalid_value(format!("The operation on \"{path}\" needs a value."))
    })
}

// ---------------------------------------------------------------------------
// Pagination et filtres
// ---------------------------------------------------------------------------

struct ListQuery {
    filter: Option<String>,
    start_index: usize,
    count: usize,
    excluded: Vec<String>,
}

fn list_query(pairs: &[(String, String)]) -> ScimResult<ListQuery> {
    let mut query =
        ListQuery { filter: None, start_index: 1, count: MAX_RESULTS, excluded: Vec::new() };
    for (key, value) in pairs {
        match key.to_ascii_lowercase().as_str() {
            "filter" => query.filter = Some(value.clone()),
            // RFC 7644 §3.4.2.4 : en dessous de 1, `startIndex` vaut 1 ; un
            // `count` négatif vaut 0.
            "startindex" => query.start_index = parse_index(value)?.max(1),
            "count" => query.count = parse_index(value)?.min(MAX_RESULTS),
            "excludedattributes" => {
                query.excluded = value
                    .split(',')
                    .map(filter::normalize_attr)
                    .filter(|attr| !attr.is_empty())
                    .collect();
            }
            _ => {}
        }
    }
    Ok(query)
}

fn parse_index(raw: &str) -> ScimResult<usize> {
    let value: i64 = raw
        .trim()
        .parse()
        .map_err(|_| ScimError::invalid_value("startIndex and count must be integers."))?;
    Ok(usize::try_from(value.max(0)).unwrap_or(0))
}

/// Retire les attributs exclus, jamais `id` ni `schemas`.
fn exclude(mut resource: Value, excluded: &[String]) -> Value {
    if let Some(object) = resource.as_object_mut() {
        object.retain(|key, _| {
            let key = key.to_ascii_lowercase();
            key == "id" || key == "schemas" || !excluded.contains(&key)
        });
    }
    resource
}

fn list_response(total: usize, start_index: usize, resources: Vec<Value>) -> Response {
    scim_response(
        StatusCode::OK,
        &json!({
            "schemas": [LIST_SCHEMA],
            "totalResults": total,
            "startIndex": start_index,
            "itemsPerPage": resources.len(),
            "Resources": resources,
        }),
    )
}

fn page<T>(items: Vec<T>, query: &ListQuery) -> (usize, Vec<T>) {
    let total = items.len();
    let page = items.into_iter().skip(query.start_index - 1).take(query.count).collect();
    (total, page)
}

/// Comparaison insensible à la casse.
fn same_text(actual: &str, wanted: &Literal) -> bool {
    wanted.as_str().is_some_and(|wanted| actual.to_lowercase() == wanted.to_lowercase())
}

// ---------------------------------------------------------------------------
// Comptes
// ---------------------------------------------------------------------------

macro_rules! select_user {
    ($tail:literal) => {
        concat!(
            "SELECT id, scim_id, username, display_name, role, disabled, scim_external_id, ",
            "scim_email, created_at, scim_updated_at FROM users ",
            $tail
        )
    };
}

#[derive(Debug, Clone)]
struct ScimUser {
    id: i64,
    scim_id: String,
    username: String,
    display_name: String,
    role: Role,
    disabled: bool,
    external_id: Option<String>,
    email: Option<String>,
    created_at: String,
    updated_at: Option<String>,
}

fn read_user(row: &SqliteRow) -> anyhow::Result<ScimUser> {
    let role: String = row.try_get("role")?;
    Ok(ScimUser {
        id: row.try_get("id")?,
        scim_id: row.try_get("scim_id")?,
        username: row.try_get("username")?,
        display_name: row.try_get("display_name")?,
        // Un rôle illisible est traité comme administrateur : intouchable.
        role: Role::parse(&role).unwrap_or(Role::Admin),
        disabled: row.try_get::<i64, _>("disabled")? != 0,
        external_id: row.try_get("scim_external_id")?,
        email: row.try_get("scim_email")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("scim_updated_at")?,
    })
}

async fn scim_users(pool: &SqlitePool) -> anyhow::Result<Vec<ScimUser>> {
    let rows = sqlx::query(select_user!("WHERE scim_id IS NOT NULL ORDER BY id"))
        .fetch_all(pool)
        .await
        .context("liste des comptes SCIM")?;
    rows.iter().map(read_user).collect()
}

async fn load_user(pool: &SqlitePool, scim_id: &str) -> ScimResult<ScimUser> {
    let row = sqlx::query(select_user!("WHERE scim_id = ?"))
        .bind(scim_id)
        .fetch_optional(pool)
        .await
        .context("lecture d'un compte SCIM")?;
    match row {
        Some(row) => Ok(read_user(&row)?),
        None => Err(ScimError::not_found()),
    }
}

async fn load_user_by_id(pool: &SqlitePool, id: i64) -> ScimResult<ScimUser> {
    let row = sqlx::query(select_user!("WHERE id = ? AND scim_id IS NOT NULL"))
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("lecture d'un compte SCIM")?;
    match row {
        Some(row) => Ok(read_user(&row)?),
        None => Err(ScimError::not_found()),
    }
}

/// Refuse toute écriture sur un administrateur.
fn writable(user: &ScimUser) -> ScimResult<()> {
    if user.role.is_admin() { Err(ScimError::admin()) } else { Ok(()) }
}

fn user_resource(user: &ScimUser, base: &str) -> Value {
    let location = format!("{base}/Users/{}", user.scim_id);
    let created = timestamp(&user.created_at);
    let modified = user.updated_at.as_deref().map_or_else(|| created.clone(), timestamp);
    let mut resource = json!({
        "schemas": [USER_SCHEMA],
        "id": user.scim_id,
        "userName": user.username,
        "displayName": user.display_name,
        "name": { "formatted": user.display_name },
        "active": !user.disabled,
        "meta": {
            "resourceType": "User",
            "created": created,
            "lastModified": modified,
            "location": location,
        },
    });
    if let Some(external_id) = &user.external_id {
        resource["externalId"] = json!(external_id);
    }
    if let Some(email) = &user.email {
        resource["emails"] = json!([{ "value": email, "type": "work", "primary": true }]);
    }
    resource
}

/// Ce que décrit un `User` en entrée (`POST`, `PUT`).
struct UserInput {
    username: String,
    display_name: String,
    external_id: Option<String>,
    email: Option<String>,
    active: Option<bool>,
}

/// Lit un `User`. `password`, s'il y en a un, n'est jamais lu.
fn parse_user(body: &Value) -> ScimResult<UserInput> {
    let body = object(body)?;
    let username = field(body, "userName")
        .ok_or_else(|| ScimError::invalid_value("\"userName\" is required."))?;
    let username = text(username, "userName")?;
    users::validate_username(&username).map_err(ScimError::invalid_value)?;
    let display_name = match field(body, "displayName") {
        Some(value) => clean_display(&text(value, "displayName")?),
        None => String::new(),
    };
    let display_name =
        if display_name.is_empty() { name_display(field(body, "name")) } else { display_name };
    let external_id = field(body, "externalId").map(parse_external_id).transpose()?.flatten();
    let email = field(body, "emails").map(pick_email).transpose()?.flatten();
    let active = field(body, "active").map(parse_active).transpose()?;
    Ok(UserInput { username, display_name, external_id, email, active })
}

/// Changements d'un compte, appliqués d'un bloc.
#[derive(Default)]
struct UserChanges {
    username: Option<String>,
    display_name: Option<String>,
    external_id: Option<Option<String>>,
    email: Option<Option<String>>,
    active: Option<bool>,
}

/// Active ou désactive un compte non administrateur, dans la transaction.
///
/// Désactiver ferme ses sessions. Réactiver ne vaut que pour un compte que SCIM
/// a lui-même désactivé : celui qu'un administrateur a désactivé le reste.
async fn set_active(tx: &mut Transaction<'_, Sqlite>, id: i64, active: bool) -> anyhow::Result<()> {
    if active {
        sqlx::query(
            "UPDATE users SET disabled = 0, disabled_by_scim = 0
             WHERE id = ? AND disabled_by_scim = 1 AND role <> 'admin'",
        )
        .bind(id)
        .execute(&mut **tx)
        .await
        .context("réactivation d'un compte SCIM")?;
        return Ok(());
    }
    // Un compte actif, ou désactivé seulement par l'annuaire LDAP, passe sous la
    // main de SCIM ; un compte désactivé par un administrateur reste à lui.
    sqlx::query(
        "UPDATE users SET disabled = 1,
                disabled_by_scim = CASE WHEN disabled = 0 OR disabled_by_ldap = 1
                                        THEN 1 ELSE disabled_by_scim END,
                disabled_by_ldap = 0
         WHERE id = ? AND role <> 'admin'",
    )
    .bind(id)
    .execute(&mut **tx)
    .await
    .context("désactivation d'un compte SCIM")?;
    // Même garde que l'`UPDATE` : un compte promu administrateur entre-temps
    // garde ses sessions.
    sqlx::query(
        "DELETE FROM auth_sessions WHERE user_id = ?
         AND EXISTS (SELECT 1 FROM users WHERE id = ? AND role <> 'admin')",
    )
    .bind(id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .context("fermeture des sessions")?;
    Ok(())
}

/// Applique des changements à un compte SCIM non administrateur.
async fn apply_user_changes(
    pool: &SqlitePool,
    user: &ScimUser,
    changes: &UserChanges,
) -> ScimResult<()> {
    writable(user)?;
    if let Some(username) = &changes.username {
        users::validate_username(username).map_err(ScimError::invalid_value)?;
        let taken: Option<i64> =
            sqlx::query_scalar("SELECT id FROM users WHERE username = ? AND id <> ?")
                .bind(username)
                .bind(user.id)
                .fetch_optional(pool)
                .await?;
        if taken.is_some() {
            return Err(ScimError::uniqueness("This userName is already taken."));
        }
    }

    let mut tx = pool.begin().await?;
    // Première instruction : une écriture, qui reprend la garde et prend le
    // verrou d'écriture. Personne ne peut promouvoir le compte avant la fin.
    let guard = sqlx::query(
        "UPDATE users SET scim_updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ? AND scim_id IS NOT NULL AND role <> 'admin'",
    )
    .bind(user.id)
    .execute(&mut *tx)
    .await?;
    if guard.rows_affected() == 0 {
        return Err(ScimError::admin());
    }
    if let Some(username) = &changes.username {
        sqlx::query("UPDATE users SET username = ? WHERE id = ?")
            .bind(username)
            .bind(user.id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(display_name) = &changes.display_name {
        sqlx::query("UPDATE users SET display_name = ? WHERE id = ?")
            .bind(display_name)
            .bind(user.id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(external_id) = &changes.external_id {
        sqlx::query("UPDATE users SET scim_external_id = ? WHERE id = ?")
            .bind(external_id)
            .bind(user.id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(email) = &changes.email {
        sqlx::query("UPDATE users SET scim_email = ? WHERE id = ?")
            .bind(email)
            .bind(user.id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(active) = changes.active {
        set_active(&mut tx, user.id, active).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Journalise une écriture sur un compte, et son changement d'état s'il y en a un.
async fn audit_user(
    state: &AppState,
    action: &str,
    before: Option<&ScimUser>,
    after: &ScimUser,
    ip: Option<std::net::IpAddr>,
) {
    audit::record(&state.pool, Some(ACTOR), action, Some(&after.username), ip).await;
    let was_disabled = before.is_some_and(|user| user.disabled);
    if after.disabled != was_disabled {
        let change = if after.disabled { "scim.user.disabled" } else { "scim.user.enabled" };
        audit::record(&state.pool, Some(ACTOR), change, Some(&after.username), ip).await;
    }
}

fn user_filter(raw: &str) -> ScimResult<Vec<Comparison>> {
    let terms = filter::parse(raw).map_err(ScimError::invalid_filter)?;
    for term in &terms {
        let known = match term.attr.as_str() {
            "id" | "username" | "externalid" | "displayname" => term.selector.is_none(),
            "emails" | "emails.value" => true,
            "active" => term.selector.is_none() && matches!(term.value, Literal::Bool(_)),
            _ => false,
        };
        let typed = term.attr == "active" || matches!(term.value, Literal::Str(_));
        if !known || !typed {
            return Err(ScimError::invalid_filter(format!(
                "Filtering users on \"{}\" is not supported.",
                term.attr
            )));
        }
    }
    Ok(terms)
}

fn user_matches(user: &ScimUser, term: &Comparison) -> bool {
    match term.attr.as_str() {
        "id" => term.value.as_str() == Some(user.scim_id.as_str()),
        "username" => same_text(&user.username, &term.value),
        "externalid" => {
            user.external_id.as_deref().is_some_and(|id| term.value.as_str() == Some(id))
        }
        "displayname" => same_text(&user.display_name, &term.value),
        "emails" | "emails.value" => {
            // Le courriel gardé est présenté comme le principal, de type `work`.
            let selected = match &term.selector {
                None => true,
                Some((sub, Literal::Str(wanted))) if sub == "type" => {
                    wanted.eq_ignore_ascii_case("work")
                }
                Some((sub, Literal::Bool(primary))) if sub == "primary" => *primary,
                Some(_) => false,
            };
            selected && user.email.as_deref().is_some_and(|email| same_text(email, &term.value))
        }
        "active" => term.value == Literal::Bool(!user.disabled),
        _ => false,
    }
}

/// `GET /api/scim/v2/Users`
async fn list_users(
    State(state): State<AppState>,
    origin: Origin,
    Query(pairs): Query<Vec<(String, String)>>,
) -> ScimResult<Response> {
    let query = list_query(&pairs)?;
    let terms = match &query.filter {
        Some(raw) => user_filter(raw)?,
        None => Vec::new(),
    };
    let matched: Vec<ScimUser> = scim_users(&state.pool)
        .await?
        .into_iter()
        .filter(|user| terms.iter().all(|term| user_matches(user, term)))
        .collect();
    let (total, page) = page(matched, &query);
    let base = base_url(&origin);
    let resources =
        page.iter().map(|user| exclude(user_resource(user, &base), &query.excluded)).collect();
    Ok(list_response(total, query.start_index, resources))
}

/// `GET /api/scim/v2/Users/{id}`
async fn get_user(
    State(state): State<AppState>,
    origin: Origin,
    Path(id): Path<String>,
    Query(pairs): Query<Vec<(String, String)>>,
) -> ScimResult<Response> {
    let query = list_query(&pairs)?;
    let user = load_user(&state.pool, &id).await?;
    let resource = exclude(user_resource(&user, &base_url(&origin)), &query.excluded);
    Ok(scim_response(StatusCode::OK, &resource))
}

/// `POST /api/scim/v2/Users` — crée le compte, ou rattache un compte existant
/// de même identifiant s'il n'est ni administrateur ni déjà géré par SCIM.
async fn create_user(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    origin: Origin,
    body: Bytes,
) -> ScimResult<Response> {
    let input = parse_user(&json_body(&body)?)?;
    // Le rôle vient du réglage, jamais de la requête : lecteur ou opérateur.
    let role = scim::config(&state.pool).await?.default_role.role();
    let inactive = i64::from(input.active == Some(false));
    let inserted: Option<i64> = sqlx::query_scalar(
        "INSERT INTO users (username, display_name, role, scim_id, scim_external_id, scim_email,
                            disabled, disabled_by_scim, scim_updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
         ON CONFLICT(username) DO NOTHING
         RETURNING id",
    )
    .bind(&input.username)
    .bind(&input.display_name)
    .bind(role.as_str())
    .bind(new_scim_id())
    .bind(&input.external_id)
    .bind(&input.email)
    .bind(inactive)
    .bind(inactive)
    .fetch_optional(&state.pool)
    .await
    .context("création d'un compte SCIM")?;

    let (id, action) = match inserted {
        Some(id) => (id, "scim.user.created"),
        None => (link_user(&state.pool, &input).await?, "scim.user.linked"),
    };
    let user = load_user_by_id(&state.pool, id).await?;
    audit_user(&state, action, None, &user, ip).await;
    tracing::info!(user = %user.username, action, "SCIM provisioning");
    Ok(created(&user_resource(&user, &base_url(&origin))))
}

/// Rattache le compte existant de même identifiant, s'il n'est pas
/// administrateur et qu'aucun fournisseur ne le gère encore.
async fn link_user(pool: &SqlitePool, input: &UserInput) -> ScimResult<i64> {
    let mut tx = pool.begin().await?;
    let linked: Option<i64> = sqlx::query_scalar(
        "UPDATE users SET scim_id = ?, scim_external_id = ?, scim_email = ?,
                display_name = CASE WHEN ? = '' THEN display_name ELSE ? END,
                scim_updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE username = ? AND scim_id IS NULL AND role <> 'admin'
         RETURNING id",
    )
    .bind(new_scim_id())
    .bind(&input.external_id)
    .bind(&input.email)
    .bind(&input.display_name)
    .bind(&input.display_name)
    .bind(&input.username)
    .fetch_optional(&mut *tx)
    .await
    .context("rattachement d'un compte existant")?;
    let Some(id) = linked else {
        return Err(ScimError::uniqueness("This userName is already taken."));
    };
    if let Some(active) = input.active {
        set_active(&mut tx, id, active).await?;
    }
    tx.commit().await?;
    Ok(id)
}

/// `PUT /api/scim/v2/Users/{id}` — remplace le compte (Okta).
async fn replace_user(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    origin: Origin,
    Path(id): Path<String>,
    body: Bytes,
) -> ScimResult<Response> {
    let user = load_user(&state.pool, &id).await?;
    writable(&user)?;
    let input = parse_user(&json_body(&body)?)?;
    let changes = UserChanges {
        username: (input.username != user.username).then(|| input.username.clone()),
        display_name: Some(input.display_name),
        external_id: Some(input.external_id),
        email: Some(input.email),
        active: input.active,
    };
    apply_user_changes(&state.pool, &user, &changes).await?;
    let after = load_user_by_id(&state.pool, user.id).await?;
    audit_user(&state, "scim.user.updated", Some(&user), &after, ip).await;
    Ok(scim_response(StatusCode::OK, &user_resource(&after, &base_url(&origin))))
}

/// `PATCH /api/scim/v2/Users/{id}`
async fn patch_user(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    origin: Origin,
    Path(id): Path<String>,
    body: Bytes,
) -> ScimResult<Response> {
    let user = load_user(&state.pool, &id).await?;
    writable(&user)?;
    let operations = parse_patch(&json_body(&body)?)?;
    let mut changes = UserChanges::default();
    let mut formatted = None;
    for operation in &operations {
        match &operation.path {
            Some(path) => patch_user_attr(
                &mut changes,
                &mut formatted,
                operation.kind,
                path,
                operation.value.as_ref(),
            )?,
            None => {
                for (key, value) in operation_object(operation)? {
                    let kind = operation.kind;
                    patch_user_attr(&mut changes, &mut formatted, kind, key, Some(value))?;
                }
            }
        }
    }
    // `name.formatted` ne sert que si `displayName` n'est pas lui-même changé.
    if changes.display_name.is_none() {
        changes.display_name = formatted.filter(|name: &String| !name.is_empty());
    }
    apply_user_changes(&state.pool, &user, &changes).await?;
    let after = load_user_by_id(&state.pool, user.id).await?;
    audit_user(&state, "scim.user.updated", Some(&user), &after, ip).await;
    Ok(scim_response(StatusCode::OK, &user_resource(&after, &base_url(&origin))))
}

/// Une opération de `PATCH` sur un attribut de compte. Les attributs que
/// DumbMonit ne garde pas (titre, téléphone, prénom, extension entreprise… et
/// le mot de passe, jamais lu) sont acceptés et ignorés : Entra ID les envoie.
fn patch_user_attr(
    changes: &mut UserChanges,
    formatted: &mut Option<String>,
    kind: OpKind,
    path: &str,
    value: Option<&Value>,
) -> ScimResult<()> {
    let remove = kind == OpKind::Remove;
    let attr = filter::normalize_attr(path);
    match attr.as_str() {
        "username" => {
            if remove {
                return Err(ScimError::mutability("\"userName\" cannot be removed."));
            }
            changes.username = Some(text(needed(value, path)?, "userName")?);
        }
        "displayname" => {
            let name = if remove {
                String::new()
            } else {
                clean_display(&text(needed(value, path)?, path)?)
            };
            changes.display_name = Some(name);
        }
        "name" => {
            if !remove {
                *formatted = Some(name_display(value));
            }
        }
        "name.formatted" => {
            if !remove {
                *formatted = Some(clean_display(&text(needed(value, path)?, path)?));
            }
        }
        "externalid" => {
            let id = if remove { None } else { parse_external_id(needed(value, path)?)? };
            changes.external_id = Some(id);
        }
        "active" => {
            if !remove {
                changes.active = Some(parse_active(needed(value, path)?)?);
            }
        }
        attr if attr == "emails" || attr == "emails.value" || attr.starts_with("emails[") => {
            let email = if remove { None } else { pick_email(needed(value, path)?)? };
            changes.email = Some(email);
        }
        _ => {}
    }
    Ok(())
}

/// `DELETE /api/scim/v2/Users/{id}` — supprime le compte (sessions et jetons
/// suivent), jamais un administrateur.
async fn delete_user(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(id): Path<String>,
) -> ScimResult<Response> {
    let user = load_user(&state.pool, &id).await?;
    writable(&user)?;
    let done =
        sqlx::query("DELETE FROM users WHERE id = ? AND scim_id IS NOT NULL AND role <> 'admin'")
            .bind(user.id)
            .execute(&state.pool)
            .await
            .context("suppression d'un compte SCIM")?;
    if done.rows_affected() == 0 {
        return Err(ScimError::admin());
    }
    audit::record(&state.pool, Some(ACTOR), "scim.user.deleted", Some(&user.username), ip).await;
    tracing::info!(user = %user.username, "SCIM provisioning: account deleted");
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---------------------------------------------------------------------------
// Équipes
// ---------------------------------------------------------------------------

macro_rules! select_team {
    ($tail:literal) => {
        concat!(
            "SELECT id, scim_id, name, scim_external_id, scim_created, created_at, ",
            "scim_updated_at FROM teams ",
            $tail
        )
    };
}

#[derive(Debug, Clone)]
struct ScimTeam {
    id: i64,
    scim_id: String,
    name: String,
    external_id: Option<String>,
    /// Créée par SCIM (et non rattachée) : sa suppression SCIM la supprime.
    created_by_scim: bool,
    created_at: String,
    updated_at: Option<String>,
}

/// Un membre tel que le fournisseur le connaît.
#[derive(Debug, Clone)]
struct ScimMember {
    scim_id: String,
    username: String,
}

fn read_team(row: &SqliteRow) -> anyhow::Result<ScimTeam> {
    Ok(ScimTeam {
        id: row.try_get("id")?,
        scim_id: row.try_get("scim_id")?,
        name: row.try_get("name")?,
        external_id: row.try_get("scim_external_id")?,
        created_by_scim: row.try_get::<i64, _>("scim_created")? != 0,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("scim_updated_at")?,
    })
}

async fn scim_teams(pool: &SqlitePool) -> anyhow::Result<Vec<ScimTeam>> {
    let rows = sqlx::query(select_team!("WHERE scim_id IS NOT NULL ORDER BY id"))
        .fetch_all(pool)
        .await
        .context("liste des équipes SCIM")?;
    rows.iter().map(read_team).collect()
}

async fn load_team(pool: &SqlitePool, scim_id: &str) -> ScimResult<ScimTeam> {
    let row = sqlx::query(select_team!("WHERE scim_id = ?"))
        .bind(scim_id)
        .fetch_optional(pool)
        .await
        .context("lecture d'une équipe SCIM")?;
    match row {
        Some(row) => Ok(read_team(&row)?),
        None => Err(ScimError::not_found()),
    }
}

async fn load_team_by_id(pool: &SqlitePool, id: i64) -> ScimResult<ScimTeam> {
    let row = sqlx::query(select_team!("WHERE id = ? AND scim_id IS NOT NULL"))
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("lecture d'une équipe SCIM")?;
    match row {
        Some(row) => Ok(read_team(&row)?),
        None => Err(ScimError::not_found()),
    }
}

/// Les membres que le fournisseur connaît : les comptes SCIM de l'équipe,
/// quelle que soit la source de leur inscription.
async fn team_members(pool: &SqlitePool, team_id: i64) -> anyhow::Result<Vec<ScimMember>> {
    let rows = sqlx::query(
        "SELECT u.scim_id AS scim_id, u.username AS username
         FROM team_members m JOIN users u ON u.id = m.user_id
         WHERE m.team_id = ? AND u.scim_id IS NOT NULL
         ORDER BY u.id",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await
    .context("membres d'une équipe SCIM")?;
    rows.iter()
        .map(|row| {
            Ok(ScimMember { scim_id: row.try_get("scim_id")?, username: row.try_get("username")? })
        })
        .collect()
}

fn group_resource(team: &ScimTeam, members: Option<&[ScimMember]>, base: &str) -> Value {
    let created = timestamp(&team.created_at);
    let modified = team.updated_at.as_deref().map_or_else(|| created.clone(), timestamp);
    let mut resource = json!({
        "schemas": [GROUP_SCHEMA],
        "id": team.scim_id,
        "displayName": team.name,
        "meta": {
            "resourceType": "Group",
            "created": created,
            "lastModified": modified,
            "location": format!("{base}/Groups/{}", team.scim_id),
        },
    });
    if let Some(external_id) = &team.external_id {
        resource["externalId"] = json!(external_id);
    }
    if let Some(members) = members {
        let members: Vec<Value> = members
            .iter()
            .map(|member| {
                json!({
                    "value": member.scim_id,
                    "display": member.username,
                    "type": "User",
                    "$ref": format!("{base}/Users/{}", member.scim_id),
                })
            })
            .collect();
        resource["members"] = json!(members);
    }
    resource
}

/// Ce que décrit un `Group` en entrée (`POST`, `PUT`).
struct GroupInput {
    display_name: String,
    external_id: Option<String>,
    members: Option<Vec<String>>,
}

fn parse_group(body: &Value) -> ScimResult<GroupInput> {
    let body = object(body)?;
    let name = field(body, "displayName")
        .ok_or_else(|| ScimError::invalid_value("\"displayName\" is required."))?;
    let display_name =
        teams::validate_name(&text(name, "displayName")?).map_err(ScimError::invalid_value)?;
    let external_id = field(body, "externalId").map(parse_external_id).transpose()?.flatten();
    let members = field(body, "members").map(member_values).transpose()?;
    Ok(GroupInput { display_name, external_id, members })
}

/// Les identifiants SCIM d'une liste de membres (`[{"value": "…"}]`).
fn member_values(value: &Value) -> ScimResult<Vec<String>> {
    let items = match value {
        Value::Array(items) => items.as_slice(),
        Value::Object(_) => std::slice::from_ref(value),
        _ => return Err(ScimError::invalid_value("\"members\" must be a list.")),
    };
    if items.len() > MAX_MEMBERS {
        return Err(ScimError::invalid_value(format!(
            "A request names at most {MAX_MEMBERS} members."
        )));
    }
    items
        .iter()
        .map(|item| {
            item.as_object()
                .and_then(|member| field(member, "value"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .ok_or_else(|| ScimError::invalid_value("Each member needs a \"value\"."))
        })
        .collect()
}

/// Les comptes désignés par leurs identifiants SCIM. Un administrateur est
/// écarté sans erreur : SCIM ne l'inscrit nulle part. Un identifiant inconnu est
/// refusé, sauf pour un retrait (`lenient`), où il n'y a rien à faire.
async fn resolve_members(
    tx: &mut Transaction<'_, Sqlite>,
    values: &[String],
    lenient: bool,
) -> ScimResult<Vec<i64>> {
    let mut out: Vec<i64> = Vec::new();
    for value in values {
        let row: Option<(i64, String)> =
            sqlx::query_as("SELECT id, role FROM users WHERE scim_id = ?")
                .bind(value)
                .fetch_optional(&mut **tx)
                .await
                .context("recherche d'un membre")?;
        match row {
            Some((id, role)) => {
                let provisionable =
                    matches!(Role::parse(&role), Some(Role::Viewer | Role::Operator));
                if provisionable && !out.contains(&id) {
                    out.push(id);
                }
            }
            None if lenient => {}
            None => {
                return Err(ScimError::invalid_value(format!("Unknown member \"{value}\".")));
            }
        }
    }
    Ok(out)
}

/// Inscrit des comptes (source `scim`). Une inscription manuelle reste
/// manuelle ; une inscription par les groupes de connexion passe sous SCIM.
async fn add_members(
    tx: &mut Transaction<'_, Sqlite>,
    team_id: i64,
    ids: &[i64],
) -> anyhow::Result<()> {
    for id in ids {
        sqlx::query(
            "INSERT INTO team_members (team_id, user_id, source) VALUES (?, ?, 'scim')
             ON CONFLICT(team_id, user_id) DO UPDATE SET source = 'scim'
             WHERE team_members.source = 'idp'",
        )
        .bind(team_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .context("inscription d'un membre SCIM")?;
    }
    Ok(())
}

/// Retire des comptes inscrits par SCIM ; les autres inscriptions restent.
async fn remove_members(
    tx: &mut Transaction<'_, Sqlite>,
    team_id: i64,
    ids: &[i64],
) -> anyhow::Result<()> {
    for id in ids {
        sqlx::query(
            "DELETE FROM team_members WHERE team_id = ? AND user_id = ? AND source = 'scim'",
        )
        .bind(team_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .context("retrait d'un membre SCIM")?;
    }
    Ok(())
}

/// Remplace les inscriptions SCIM par `ids`.
async fn replace_members(
    tx: &mut Transaction<'_, Sqlite>,
    team_id: i64,
    ids: &[i64],
) -> anyhow::Result<()> {
    let current: Vec<i64> = sqlx::query_scalar(
        "SELECT user_id FROM team_members WHERE team_id = ? AND source = 'scim'",
    )
    .bind(team_id)
    .fetch_all(&mut **tx)
    .await
    .context("membres SCIM d'une équipe")?;
    let gone: Vec<i64> = current.into_iter().filter(|id| !ids.contains(id)).collect();
    remove_members(tx, team_id, &gone).await?;
    add_members(tx, team_id, ids).await
}

/// Première écriture de toute modification d'équipe : l'équipe est toujours
/// gérée par SCIM, et le verrou d'écriture est pris.
async fn lock_team(tx: &mut Transaction<'_, Sqlite>, team: &ScimTeam) -> ScimResult<()> {
    let done = sqlx::query(
        "UPDATE teams SET scim_updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ? AND scim_id = ?",
    )
    .bind(team.id)
    .bind(&team.scim_id)
    .execute(&mut **tx)
    .await
    .context("verrou d'une équipe SCIM")?;
    if done.rows_affected() == 0 {
        return Err(ScimError::not_found());
    }
    Ok(())
}

/// Renomme une équipe créée par SCIM. Une équipe créée par un administrateur,
/// puis rattachée, garde le nom qu'il lui a donné.
async fn rename_team(
    tx: &mut Transaction<'_, Sqlite>,
    team: &ScimTeam,
    name: &str,
) -> ScimResult<()> {
    let name = teams::validate_name(name).map_err(ScimError::invalid_value)?;
    if !team.created_by_scim || name == team.name {
        return Ok(());
    }
    let taken: Option<i64> = sqlx::query_scalar("SELECT id FROM teams WHERE name = ? AND id <> ?")
        .bind(&name)
        .bind(team.id)
        .fetch_optional(&mut **tx)
        .await?;
    if taken.is_some() {
        return Err(ScimError::uniqueness("A group with this displayName already exists."));
    }
    sqlx::query("UPDATE teams SET name = ? WHERE id = ?")
        .bind(&name)
        .bind(team.id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn set_team_external_id(
    tx: &mut Transaction<'_, Sqlite>,
    team_id: i64,
    external_id: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE teams SET scim_external_id = ? WHERE id = ?")
        .bind(external_id)
        .bind(team_id)
        .execute(&mut **tx)
        .await
        .context("identifiant externe d'une équipe")?;
    Ok(())
}

fn group_filter(raw: &str) -> ScimResult<Vec<Comparison>> {
    let terms = filter::parse(raw).map_err(ScimError::invalid_filter)?;
    for term in &terms {
        let known = matches!(
            term.attr.as_str(),
            "id" | "displayname" | "externalid" | "members" | "members.value"
        );
        if !known || term.selector.is_some() || term.value.as_str().is_none() {
            return Err(ScimError::invalid_filter(format!(
                "Filtering groups on \"{}\" is not supported.",
                term.attr
            )));
        }
    }
    Ok(terms)
}

fn group_matches(team: &ScimTeam, members: Option<&[ScimMember]>, term: &Comparison) -> bool {
    match term.attr.as_str() {
        "id" => term.value.as_str() == Some(team.scim_id.as_str()),
        "displayname" => same_text(&team.name, &term.value),
        "externalid" => {
            team.external_id.as_deref().is_some_and(|id| term.value.as_str() == Some(id))
        }
        "members" | "members.value" => members.is_some_and(|members| {
            members.iter().any(|member| term.value.as_str() == Some(member.scim_id.as_str()))
        }),
        _ => false,
    }
}

/// `GET /api/scim/v2/Groups`
async fn list_groups(
    State(state): State<AppState>,
    origin: Origin,
    Query(pairs): Query<Vec<(String, String)>>,
) -> ScimResult<Response> {
    let query = list_query(&pairs)?;
    let terms = match &query.filter {
        Some(raw) => group_filter(raw)?,
        None => Vec::new(),
    };
    let want_members = !query.excluded.iter().any(|attr| attr == "members")
        || terms.iter().any(|term| term.attr.starts_with("members"));
    let mut matched = Vec::new();
    for team in scim_teams(&state.pool).await? {
        let members =
            if want_members { Some(team_members(&state.pool, team.id).await?) } else { None };
        if terms.iter().all(|term| group_matches(&team, members.as_deref(), term)) {
            matched.push((team, members));
        }
    }
    let (total, page) = page(matched, &query);
    let base = base_url(&origin);
    let resources = page
        .iter()
        .map(|(team, members)| {
            exclude(group_resource(team, members.as_deref(), &base), &query.excluded)
        })
        .collect();
    Ok(list_response(total, query.start_index, resources))
}

/// `GET /api/scim/v2/Groups/{id}`
async fn get_group(
    State(state): State<AppState>,
    origin: Origin,
    Path(id): Path<String>,
    Query(pairs): Query<Vec<(String, String)>>,
) -> ScimResult<Response> {
    let query = list_query(&pairs)?;
    let team = load_team(&state.pool, &id).await?;
    let members = if query.excluded.iter().any(|attr| attr == "members") {
        None
    } else {
        Some(team_members(&state.pool, team.id).await?)
    };
    let resource =
        exclude(group_resource(&team, members.as_deref(), &base_url(&origin)), &query.excluded);
    Ok(scim_response(StatusCode::OK, &resource))
}

/// `POST /api/scim/v2/Groups` — crée l'équipe, ou rattache l'équipe existante
/// de même nom si aucun fournisseur ne la gère encore. Ses droits sur les
/// dossiers restent ceux que l'administrateur lui donne.
async fn create_group(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    origin: Origin,
    body: Bytes,
) -> ScimResult<Response> {
    let input = parse_group(&json_body(&body)?)?;
    let mut tx = state.pool.begin().await?;
    let inserted: Option<i64> = sqlx::query_scalar(
        "INSERT INTO teams (name, scim_id, scim_external_id, scim_created, scim_updated_at)
         VALUES (?, ?, ?, 1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
         ON CONFLICT(name) DO NOTHING
         RETURNING id",
    )
    .bind(&input.display_name)
    .bind(new_scim_id())
    .bind(&input.external_id)
    .fetch_optional(&mut *tx)
    .await
    .context("création d'une équipe SCIM")?;
    let (team_id, action) = match inserted {
        Some(id) => (id, "scim.group.created"),
        None => {
            let linked: Option<i64> = sqlx::query_scalar(
                "UPDATE teams SET scim_id = ?, scim_external_id = ?, scim_created = 0,
                        scim_updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE name = ? AND scim_id IS NULL
                 RETURNING id",
            )
            .bind(new_scim_id())
            .bind(&input.external_id)
            .bind(&input.display_name)
            .fetch_optional(&mut *tx)
            .await
            .context("rattachement d'une équipe existante")?;
            let Some(id) = linked else {
                return Err(ScimError::uniqueness("A group with this displayName already exists."));
            };
            (id, "scim.group.linked")
        }
    };
    if let Some(values) = &input.members {
        let ids = resolve_members(&mut tx, values, false).await?;
        add_members(&mut tx, team_id, &ids).await?;
    }
    tx.commit().await?;

    let team = load_team_by_id(&state.pool, team_id).await?;
    let members = team_members(&state.pool, team.id).await?;
    audit::record(&state.pool, Some(ACTOR), action, Some(&team.name), ip).await;
    tracing::info!(team = %team.name, action, "SCIM provisioning");
    Ok(created(&group_resource(&team, Some(&members), &base_url(&origin))))
}

/// `PUT /api/scim/v2/Groups/{id}` — remplace l'équipe (Okta). Sans `members`,
/// les membres restent ; avec, les inscriptions SCIM suivent la liste.
async fn replace_group(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    origin: Origin,
    Path(id): Path<String>,
    body: Bytes,
) -> ScimResult<Response> {
    let team = load_team(&state.pool, &id).await?;
    let input = parse_group(&json_body(&body)?)?;
    let mut tx = state.pool.begin().await?;
    lock_team(&mut tx, &team).await?;
    rename_team(&mut tx, &team, &input.display_name).await?;
    set_team_external_id(&mut tx, team.id, input.external_id.as_deref()).await?;
    if let Some(values) = &input.members {
        let ids = resolve_members(&mut tx, values, false).await?;
        replace_members(&mut tx, team.id, &ids).await?;
    }
    tx.commit().await?;

    let after = load_team_by_id(&state.pool, team.id).await?;
    let members = team_members(&state.pool, team.id).await?;
    audit::record(&state.pool, Some(ACTOR), "scim.group.updated", Some(&after.name), ip).await;
    Ok(scim_response(StatusCode::OK, &group_resource(&after, Some(&members), &base_url(&origin))))
}

/// `PATCH /api/scim/v2/Groups/{id}` — `204` sans corps.
async fn patch_group(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(id): Path<String>,
    body: Bytes,
) -> ScimResult<Response> {
    let team = load_team(&state.pool, &id).await?;
    let operations = parse_patch(&json_body(&body)?)?;
    let mut tx = state.pool.begin().await?;
    lock_team(&mut tx, &team).await?;
    for operation in &operations {
        match &operation.path {
            Some(path) => {
                patch_group_attr(&mut tx, &team, operation.kind, path, operation.value.as_ref())
                    .await?;
            }
            None => {
                for (key, value) in operation_object(operation)? {
                    patch_group_attr(&mut tx, &team, operation.kind, key, Some(value)).await?;
                }
            }
        }
    }
    tx.commit().await?;
    audit::record(&state.pool, Some(ACTOR), "scim.group.updated", Some(&team.name), ip).await;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Une opération de `PATCH` sur un attribut d'équipe. `id`, `schemas`, `meta`
/// et ce que DumbMonit ne garde pas sont ignorés.
async fn patch_group_attr(
    tx: &mut Transaction<'_, Sqlite>,
    team: &ScimTeam,
    kind: OpKind,
    path: &str,
    value: Option<&Value>,
) -> ScimResult<()> {
    let value = value.filter(|value| !value.is_null());
    // `members[value eq "…"]` : un membre précis (Entra ID, pour un retrait).
    if path.contains('[') {
        let Some(term) = filter::parse_value_path(path) else {
            return Err(ScimError::new(
                StatusCode::BAD_REQUEST,
                Some("invalidPath"),
                format!("Unsupported path \"{path}\"."),
            ));
        };
        if term.attr != "members.value" || kind != OpKind::Remove {
            return Err(ScimError::new(
                StatusCode::BAD_REQUEST,
                Some("invalidPath"),
                format!("Unsupported path \"{path}\"."),
            ));
        }
        let wanted = term.value.as_str().map(str::to_string).into_iter().collect::<Vec<_>>();
        let ids = resolve_members(tx, &wanted, true).await?;
        remove_members(tx, team.id, &ids).await?;
        return Ok(());
    }
    match filter::normalize_attr(path).as_str() {
        "displayname" => {
            if kind == OpKind::Remove {
                return Err(ScimError::mutability("\"displayName\" cannot be removed."));
            }
            let name = text(needed(value, path)?, "displayName")?;
            rename_team(tx, team, &name).await?;
        }
        "externalid" => {
            let id = match kind {
                OpKind::Remove => None,
                _ => parse_external_id(needed(value, path)?)?,
            };
            set_team_external_id(tx, team.id, id.as_deref()).await?;
        }
        "members" => match kind {
            OpKind::Add => {
                let values = member_values(needed(value, path)?)?;
                let ids = resolve_members(tx, &values, false).await?;
                add_members(tx, team.id, &ids).await?;
            }
            OpKind::Replace => {
                let values = value.map(member_values).transpose()?.unwrap_or_default();
                let ids = resolve_members(tx, &values, false).await?;
                replace_members(tx, team.id, &ids).await?;
            }
            OpKind::Remove => match value {
                Some(value) => {
                    let ids = resolve_members(tx, &member_values(value)?, true).await?;
                    remove_members(tx, team.id, &ids).await?;
                }
                None => replace_members(tx, team.id, &[]).await?,
            },
        },
        _ => {}
    }
    Ok(())
}

/// `DELETE /api/scim/v2/Groups/{id}` — supprime l'équipe si SCIM l'a créée ;
/// sinon la détache, en n'en retirant que les membres inscrits par SCIM.
async fn delete_group(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(id): Path<String>,
) -> ScimResult<Response> {
    let team = load_team(&state.pool, &id).await?;
    let mut tx = state.pool.begin().await?;
    let action = if team.created_by_scim {
        sqlx::query("DELETE FROM teams WHERE id = ? AND scim_id = ?")
            .bind(team.id)
            .bind(&team.scim_id)
            .execute(&mut *tx)
            .await
            .context("suppression d'une équipe SCIM")?;
        "scim.group.deleted"
    } else {
        sqlx::query("DELETE FROM team_members WHERE team_id = ? AND source = 'scim'")
            .bind(team.id)
            .execute(&mut *tx)
            .await
            .context("retrait des membres SCIM")?;
        sqlx::query(
            "UPDATE teams SET scim_id = NULL, scim_external_id = NULL, scim_updated_at = NULL
             WHERE id = ?",
        )
        .bind(team.id)
        .execute(&mut *tx)
        .await
        .context("détachement d'une équipe")?;
        "scim.group.unlinked"
    };
    tx.commit().await?;
    audit::record(&state.pool, Some(ACTOR), action, Some(&team.name), ip).await;
    tracing::info!(team = %team.name, action, "SCIM provisioning");
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---------------------------------------------------------------------------
// Découverte (RFC 7644 §4)
// ---------------------------------------------------------------------------

/// `GET /api/scim/v2/ServiceProviderConfig`
async fn service_provider_config(origin: Origin) -> Response {
    let base = base_url(&origin);
    scim_response(
        StatusCode::OK,
        &json!({
            "schemas": [CONFIG_SCHEMA],
            "patch": { "supported": true },
            "bulk": { "supported": false, "maxOperations": 0, "maxPayloadSize": 0 },
            "filter": { "supported": true, "maxResults": MAX_RESULTS },
            "changePassword": { "supported": false },
            "sort": { "supported": false },
            "etag": { "supported": false },
            "authenticationSchemes": [{
                "type": "oauthbearertoken",
                "name": "Bearer token",
                "description": "The SCIM token generated in DumbMonit (Settings → Single sign-on), sent as Authorization: Bearer dmscim_…",
                "primary": true,
            }],
            "meta": {
                "resourceType": "ServiceProviderConfig",
                "location": format!("{base}/ServiceProviderConfig"),
            },
        }),
    )
}

/// `GET /api/scim/v2/ResourceTypes`
async fn resource_types(origin: Origin) -> Response {
    let base = base_url(&origin);
    let kind = |name: &str, endpoint: &str, schema: &str| {
        json!({
            "schemas": [RESOURCE_TYPE_SCHEMA],
            "id": name,
            "name": name,
            "endpoint": endpoint,
            "schema": schema,
            "meta": {
                "resourceType": "ResourceType",
                "location": format!("{base}/ResourceTypes/{name}"),
            },
        })
    };
    let resources =
        vec![kind("User", "/Users", USER_SCHEMA), kind("Group", "/Groups", GROUP_SCHEMA)];
    list_response(resources.len(), 1, resources)
}

/// Un attribut simple d'un schéma.
fn attribute(name: &str, kind: &str, required: bool, mutability: &str, case_exact: bool) -> Value {
    let unique = required && matches!(name, "userName" | "displayName");
    let uniqueness = if unique { "server" } else { "none" };
    json!({
        "name": name,
        "type": kind,
        "multiValued": false,
        "required": required,
        "caseExact": case_exact,
        "mutability": mutability,
        "returned": "default",
        "uniqueness": uniqueness,
    })
}

/// Un attribut complexe d'un schéma.
fn complex(name: &str, multi_valued: bool, sub_attributes: Vec<Value>) -> Value {
    json!({
        "name": name,
        "type": "complex",
        "multiValued": multi_valued,
        "required": false,
        "mutability": "readWrite",
        "returned": "default",
        "subAttributes": sub_attributes,
    })
}

/// `GET /api/scim/v2/Schemas`
async fn schemas(origin: Origin) -> Response {
    let base = base_url(&origin);
    let user = json!({
        "schemas": [SCHEMA_SCHEMA],
        "id": USER_SCHEMA,
        "name": "User",
        "description": "DumbMonit account",
        "attributes": [
            attribute("userName", "string", true, "readWrite", false),
            attribute("displayName", "string", false, "readWrite", false),
            complex("name", false, vec![
                attribute("formatted", "string", false, "readWrite", false),
                attribute("givenName", "string", false, "writeOnly", false),
                attribute("familyName", "string", false, "writeOnly", false),
            ]),
            attribute("active", "boolean", false, "readWrite", false),
            complex("emails", true, vec![
                attribute("value", "string", false, "readWrite", false),
                attribute("type", "string", false, "readWrite", false),
                attribute("primary", "boolean", false, "readWrite", false),
            ]),
        ],
        "meta": { "resourceType": "Schema", "location": format!("{base}/Schemas/{USER_SCHEMA}") },
    });
    let group = json!({
        "schemas": [SCHEMA_SCHEMA],
        "id": GROUP_SCHEMA,
        "name": "Group",
        "description": "DumbMonit team",
        "attributes": [
            attribute("displayName", "string", true, "readWrite", false),
            complex("members", true, vec![
                attribute("value", "string", false, "immutable", true),
                attribute("display", "string", false, "readOnly", false),
                attribute("$ref", "reference", false, "immutable", true),
            ]),
        ],
        "meta": { "resourceType": "Schema", "location": format!("{base}/Schemas/{GROUP_SCHEMA}") },
    });
    let resources = vec![user, group];
    list_response(resources.len(), 1, resources)
}

// ---------------------------------------------------------------------------
// Réglages, réservés à une session d'administrateur (`/api/auth/scim`)
// ---------------------------------------------------------------------------

/// Les réglages tels que l'écran les montre. Le jeton n'y figure qu'à sa
/// création, une seule fois ; pas de `Debug`, pour qu'il ne finisse jamais
/// dans un journal.
#[derive(Serialize)]
pub struct ScimSettingsView {
    enabled: bool,
    base_url: String,
    token_set: bool,
    token_created_at: Option<String>,
    default_role: DefaultRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

async fn settings_view(state: &AppState, origin: &Origin) -> anyhow::Result<ScimSettingsView> {
    let config = scim::config(&state.pool).await?;
    let token = scim::token(&state.pool).await?;
    Ok(ScimSettingsView {
        enabled: config.enabled,
        base_url: base_url(origin),
        token_set: token.is_some(),
        token_created_at: token.map(|token| token.created_at),
        default_role: config.default_role,
        token: None,
    })
}

/// `GET /api/auth/scim`
pub async fn get_settings(
    State(state): State<AppState>,
    _: AdminUser,
    origin: Origin,
) -> AuthResult<Json<ScimSettingsView>> {
    Ok(Json(settings_view(&state, &origin).await?))
}

/// Corps de `PUT /api/auth/scim`.
#[derive(Debug, Deserialize)]
pub struct SettingsPayload {
    enabled: bool,
    #[serde(default)]
    default_role: Option<String>,
}

/// `PUT /api/auth/scim`
pub async fn put_settings(
    State(state): State<AppState>,
    AdminUser(me): AdminUser,
    ClientIp(ip): ClientIp,
    origin: Origin,
    Json(payload): Json<SettingsPayload>,
) -> AuthResult<Json<ScimSettingsView>> {
    let mut config = scim::config(&state.pool).await?;
    if let Some(role) = payload.default_role.as_deref() {
        config.default_role = DefaultRole::parse(role).ok_or_else(|| {
            AuthError::Invalid("The default role must be \"viewer\" or \"operator\".".into())
        })?;
    }
    let action = match (config.enabled, payload.enabled) {
        (false, true) => "scim.enabled",
        (true, false) => "scim.disabled",
        _ => "scim.settings.updated",
    };
    config.enabled = payload.enabled;
    scim::save_config(&state.pool, &config).await?;
    audit::record(&state.pool, Some(&me.username), action, Some(config.default_role.as_str()), ip)
        .await;
    Ok(Json(settings_view(&state, &origin).await?))
}

/// `POST /api/auth/scim/token` — nouveau jeton, montré une fois ; l'ancien
/// cesse aussitôt de valoir.
pub async fn create_token(
    State(state): State<AppState>,
    AdminUser(me): AdminUser,
    ClientIp(ip): ClientIp,
    origin: Origin,
) -> AuthResult<Response> {
    let (_, clear) = scim::rotate_token(&state.pool).await?;
    audit::record(&state.pool, Some(&me.username), "scim.token.created", None, ip).await;
    tracing::info!(by = %me.username, "SCIM token generated");
    let mut view = settings_view(&state, &origin).await?;
    view.token = Some(clear);
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((headers, Json(view)).into_response())
}

/// `DELETE /api/auth/scim/token`
pub async fn delete_token(
    State(state): State<AppState>,
    AdminUser(me): AdminUser,
    ClientIp(ip): ClientIp,
) -> AuthResult<StatusCode> {
    scim::revoke_token(&state.pool).await?;
    audit::record(&state.pool, Some(&me.username), "scim.token.revoked", None, ip).await;
    tracing::info!(by = %me.username, "SCIM token revoked");
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scim_ids_are_version_4_uuids() {
        let id = new_scim_id();
        assert_eq!(id.len(), 36);
        assert_eq!(id.as_bytes()[14], b'4');
        assert!(matches!(id.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
        assert_ne!(id, new_scim_id());
    }

    #[test]
    fn database_dates_become_rfc3339() {
        assert_eq!(timestamp("2026-10-26 07:30:47"), "2026-10-26T07:30:47Z");
        assert_eq!(timestamp("2026-10-26T07:30:47Z"), "2026-10-26T07:30:47Z");
    }

    #[test]
    fn active_accepts_booleans_and_entra_strings() {
        assert!(!parse_active(&json!("False")).unwrap());
        assert!(parse_active(&json!("TRUE")).unwrap());
        assert!(!parse_active(&json!(false)).unwrap());
        assert!(parse_active(&json!("no")).is_err());
        assert!(parse_active(&json!(0)).is_err());
    }

    #[test]
    fn the_primary_then_work_email_is_kept() {
        let emails = json!([
            { "value": "home@x.org", "type": "home" },
            { "value": "work@x.org", "type": "work" },
        ]);
        assert_eq!(pick_email(&emails).unwrap().as_deref(), Some("work@x.org"));
        let emails = json!([
            { "value": "work@x.org", "type": "work" },
            { "value": "main@x.org", "primary": true },
        ]);
        assert_eq!(pick_email(&emails).unwrap().as_deref(), Some("main@x.org"));
        assert!(pick_email(&json!([{ "value": "a b@x.org" }])).is_err());
    }

    #[test]
    fn patch_operations_are_case_insensitive() {
        let body = json!({ "Operations": [
            { "op": "Replace", "path": "active", "value": "False" },
            { "op": "add", "value": { "displayName": "Jane" } },
        ]});
        let operations = parse_patch(&body).unwrap();
        assert_eq!(operations[0].kind, OpKind::Replace);
        assert_eq!(operations[1].kind, OpKind::Add);
        assert!(parse_patch(&json!({ "Operations": [{ "op": "remove" }] })).is_err());
        assert!(parse_patch(&json!({ "Operations": [{ "op": "move", "path": "x" }] })).is_err());
        assert!(parse_patch(&json!({ "Operations": [] })).is_err());
    }

    #[test]
    fn excluded_attributes_never_drop_the_id() {
        let resource = json!({ "id": "1", "schemas": [], "members": [], "displayName": "NOC" });
        let excluded = vec!["members".to_string(), "id".to_string()];
        let out = exclude(resource, &excluded);
        assert!(out.get("members").is_none());
        assert_eq!(out["id"], "1");
        assert_eq!(out["displayName"], "NOC");
    }
}
