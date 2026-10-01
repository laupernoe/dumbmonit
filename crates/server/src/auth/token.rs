//! Jetons d'API : l'authentification des clients qui ne sont pas un navigateur.
//!
//! Un script, un tableau de bord ou un assistant (Claude, ChatGPT, Cursor…)
//! n'ouvre pas de session : il présente un jeton dans
//! `Authorization: Bearer dmt_…`, comme un agent présente son jeton
//! d'enregistrement. Le jeton est un secret porteur, traité comme un mot de
//! passe : haché en base, jamais réaffiché, comparé en temps constant.
//!
//! Deux portées seulement, `read` et `write`. Un jeton `read` ne peut rien
//! changer — c'est la promesse faite dans l'interface, et elle est tenue ici plutôt
//! que dans chaque outil : un outil d'écriture demande la portée avant d'agir.
//! Il n'y a pas de portée `admin` : ce qu'elle ouvrirait (comptes, sessions,
//! second facteur, SSO, jetons, sauvegardes) reste fermé à tout jeton, si bien
//! qu'un jeton ne peut jamais en fabriquer un plus puissant que lui.
//!
//! Chaque jeton peut en outre être borné :
//!
//! - dans le temps (`expires_at`) : passé cette date, il est refusé (401) ;
//! - dans l'espace (`allowed_networks`) : hors de ces réseaux, il est refusé
//!   (403). L'adresse est celle de [`client_ip`], qui ne croit `X-Forwarded-For`
//!   que d'un mandataire déclaré — et refuse quand elle ne sait pas ;
//! - par son propriétaire : un jeton vaut ce que vaut le compte qui l'a créé.
//!   Compte désactivé : jeton refusé ; compte redevenu simple lecteur : un jeton
//!   `write` n'agit plus qu'en lecture ; compte supprimé : jetons révoqués (par
//!   un déclencheur SQL, voir la migration 0034).
//!
//! Le garde [`require_token`] ne s'applique qu'aux routes qui le déclarent
//! explicitement (le point d'entrée MCP). Le garde de session
//! (`middleware::require_session`) accepte aussi ces jetons, via [`check`], sur
//! toute l'API REST — sauf la gestion des comptes et des jetons, qui reste
//! réservée à une session de navigateur.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use axum::Json;
use axum::extract::{Request, State};
use axum::http::{Extensions, HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, FromFnLayer, Next};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, SecondsFormat, Utc};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use subtle::ConstantTimeEq;

use crate::auth::AuthState;
use crate::auth::client_ip;
use crate::state::AppState;

/// Préfixe de tous les jetons d'API : « DumbMonit token ». Il rend un jeton
/// reconnaissable dans un fichier de configuration, et permet à un scanner de
/// secrets de le repérer.
pub const TOKEN_PREFIX: &str = "dmt_";

/// Octets d'aléa. 128 bits : hors de portée d'une recherche exhaustive, et le
/// jeton reste court à coller dans un fichier de configuration.
const TOKEN_BYTES: usize = 16;

/// Caractères conservés pour l'affichage et la recherche, préfixe compris.
const DISPLAY_LEN: usize = TOKEN_PREFIX.len() + 8;

/// Délai minimal entre deux mises à jour de `last_used_at` pour un même jeton.
/// Un assistant enchaîne les appels : écrire à chaque fois serait du bruit.
const TOUCH_INTERVAL: Duration = Duration::from_secs(60);

/// Plafond d'appels par jeton et par minute.
///
/// Un assistant qui boucle — ce qui arrive — ne doit pas pouvoir occuper le
/// serveur ni VictoriaMetrics. Cent vingt appels par minute laissent de la marge à
/// n'importe quelle conversation ; au-delà, c'est une boucle.
pub const RATE_LIMIT_PER_MINUTE: u32 = 120;

/// Fenêtre du plafond d'appels.
const RATE_WINDOW: Duration = Duration::from_secs(60);

/// Nombre maximal de réseaux autorisés pour un jeton : au-delà, c'est une
/// liste de pare-feu, pas une restriction de jeton.
pub const MAX_NETWORKS: usize = 32;

/// Durée de validité maximale à la création : dix ans, ce qui revient à « jamais »
/// sans en être un.
pub const MAX_EXPIRY_DAYS: i64 = 3650;

/// Ce qu'un jeton a le droit de faire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Read,
    Write,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            _ => None,
        }
    }

    /// Vrai si cette portée couvre `wanted` : `write` inclut `read`.
    pub fn allows(self, wanted: Scope) -> bool {
        self >= wanted
    }
}

/// Jeton authentifié, déposé dans la requête par [`require_token`] et par le
/// garde de session.
///
/// Le secret n'y figure pas : une fois vérifié, il n'a plus aucune raison de
/// circuler.
#[derive(Debug, Clone)]
pub struct ApiToken {
    pub id: i64,
    pub name: String,
    /// Portée effective : celle du jeton, bornée par le rôle de son propriétaire.
    pub scope: Scope,
    /// Compte qui l'a créé, quand il est connu.
    pub owner: Option<String>,
    /// Vrai quand la portée a été réduite parce que le propriétaire n'est plus
    /// administrateur : le refus doit le dire, sinon il est incompréhensible.
    pub limited_by_owner: bool,
}

impl ApiToken {
    /// Vérifie que le jeton couvre la portée demandée, ou explique le refus dans
    /// des termes que l'utilisateur — pas seulement l'assistant — comprend.
    pub fn require(&self, wanted: Scope) -> Result<(), String> {
        if self.scope.allows(wanted) {
            return Ok(());
        }
        if self.limited_by_owner {
            return Err(format!(
                "The token \"{}\" was created by {}, who is no longer an administrator: it can \
                 only read now. Create a new write token from an administrator account.",
                self.name,
                self.owner.as_deref().unwrap_or("an account")
            ));
        }
        Err(format!(
            "This action needs a token with the \"{}\" scope; the token \"{}\" is \"{}\" only. \
             Create a write token in Settings → API & assistants.",
            wanted.as_str(),
            self.name,
            self.scope.as_str()
        ))
    }
}

/// Un jeton tel qu'il peut être montré : jamais le secret.
#[derive(Debug, Clone, Serialize)]
pub struct TokenRecord {
    pub id: i64,
    pub name: String,
    pub prefix: String,
    pub scope: Scope,
    pub created_at: String,
    /// Identifiant du compte qui l'a créé ; `null` pour un jeton antérieur aux
    /// propriétaires, ou dont le compte a disparu.
    pub created_by: Option<String>,
    /// Fin de validité, RFC 3339 UTC ; `null` : jamais.
    pub expires_at: Option<String>,
    /// Vrai une fois `expires_at` passée : l'interface n'a pas à comparer des
    /// dates pour allumer la plaque.
    pub expired: bool,
    /// Réseaux d'où le jeton est accepté ; vide : partout.
    pub allowed_networks: Vec<String>,
    pub last_used_at: Option<String>,
    pub last_used_ip: Option<String>,
    pub revoked_at: Option<String>,
}

/// Ce qu'il faut pour créer un jeton.
#[derive(Debug, Clone)]
pub struct NewToken {
    pub name: String,
    pub scope: Scope,
    /// Compte créateur.
    pub user_id: Option<i64>,
    pub expires_at: Option<DateTime<Utc>>,
    pub allowed_networks: Vec<IpNet>,
}

// --------------------------------------------------------------------------
// Fabrication et empreinte
// --------------------------------------------------------------------------

pub fn generate() -> String {
    let bytes: [u8; TOKEN_BYTES] = rand::random();
    format!("{TOKEN_PREFIX}{}", hex::encode(bytes))
}

/// Empreinte stockée en base : SHA-256 brut du jeton complet.
///
/// SHA-256 et non Argon2 : le jeton est un aléa de 128 bits, pas un mot de passe
/// humain, et l'empreinte est recalculée à chaque appel.
pub fn fingerprint(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub fn display_prefix(token: &str) -> String {
    token.chars().take(DISPLAY_LEN).collect()
}

/// Extrait le jeton d'un en-tête `Authorization`. Strict sur le schéma : un mot de
/// passe « Basic » accepté ici serait une porte dérobée.
pub fn extract_bearer(headers: &HeaderMap) -> Option<&str> {
    let header = headers.get(header::AUTHORIZATION)?.to_str().ok()?.trim();
    let (scheme, value) = header.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = value.trim();
    if token.starts_with(TOKEN_PREFIX) { Some(token) } else { None }
}

/// Lit une liste de réseaux saisie par un humain : CIDR (`192.168.1.0/24`) ou
/// adresse seule (`10.0.0.5`, qui devient `10.0.0.5/32`). Chaque réseau est
/// normalisé (`192.168.1.7/24` → `192.168.1.0/24`) et les doublons retirés,
/// pour que la liste affichée soit exactement celle qui s'applique.
pub fn parse_networks(raw: &[String]) -> Result<Vec<IpNet>, String> {
    let mut out: Vec<IpNet> = Vec::new();
    for entry in raw.iter().map(|e| e.trim()).filter(|e| !e.is_empty()) {
        let net = entry
            .parse::<IpNet>()
            .or_else(|_| entry.parse::<IpAddr>().map(|ip| IpNet::from(ip.to_canonical())))
            .map_err(|_| {
                format!(
                    "Invalid network \"{entry}\": use CIDR notation (192.168.1.0/24) or a single \
                     address (10.0.0.5)."
                )
            })?
            .trunc();
        if !out.contains(&net) {
            out.push(net);
        }
    }
    if out.len() > MAX_NETWORKS {
        return Err(format!("A token accepts at most {MAX_NETWORKS} networks."));
    }
    Ok(out)
}

/// Vrai si l'adresse appartient à l'un des réseaux. Une adresse IPv4 vue à
/// travers une socket double pile (`::ffff:192.168.1.5`) est ramenée à sa forme
/// IPv4 avant la comparaison, sans quoi aucun réseau IPv4 ne la contiendrait.
pub fn network_allows(networks: &[IpNet], ip: Option<IpAddr>) -> bool {
    if networks.is_empty() {
        return true;
    }
    let Some(ip) = ip.map(|ip| ip.to_canonical()) else { return false };
    networks.iter().any(|net| net.contains(&ip))
}

fn rfc3339(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_time(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw).ok().map(|at| at.with_timezone(&Utc))
}

fn decode_networks(raw: Option<String>) -> Vec<IpNet> {
    raw.as_deref()
        .and_then(|text| serde_json::from_str::<Vec<String>>(text).ok())
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| entry.parse::<IpNet>().ok())
        .collect()
}

// --------------------------------------------------------------------------
// Persistance
// --------------------------------------------------------------------------

/// Requête de lecture d'un jeton tel qu'on le montre, complétée d'une fin de
/// requête littérale : sqlx n'accepte que des chaînes statiques.
macro_rules! select_record {
    ($tail:literal) => {
        concat!(
            "SELECT t.id, t.name, t.prefix, t.scope, t.created_at, t.last_used_at, ",
            "t.last_used_ip, t.revoked_at, t.expires_at, t.allowed_networks, ",
            "u.username AS created_by ",
            "FROM api_tokens t LEFT JOIN users u ON u.id = t.user_id ",
            $tail
        )
    };
}

/// Crée un jeton et renvoie sa forme en clair — la seule et unique fois.
pub async fn create(pool: &SqlitePool, new: &NewToken) -> Result<(TokenRecord, String)> {
    let clear = generate();
    let networks: Option<String> = if new.allowed_networks.is_empty() {
        None
    } else {
        Some(serde_json::to_string(
            &new.allowed_networks.iter().map(ToString::to_string).collect::<Vec<_>>(),
        )?)
    };
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO api_tokens
             (name, prefix, token_hash, scope, user_id, expires_at, allowed_networks)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(&new.name)
    .bind(display_prefix(&clear))
    .bind(fingerprint(&clear))
    .bind(new.scope.as_str())
    .bind(new.user_id)
    .bind(new.expires_at.map(rfc3339))
    .bind(networks)
    .fetch_one(pool)
    .await
    .context("creating the API token")?;
    let record = get(pool, id).await?.context("reading back the API token")?;
    Ok((record, clear))
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<TokenRecord>> {
    let row = sqlx::query(select_record!("WHERE t.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("reading the API token")?;
    row.as_ref().map(row_to_record).transpose()
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<TokenRecord>> {
    let rows = sqlx::query(select_record!("ORDER BY t.id DESC"))
        .fetch_all(pool)
        .await
        .context("listing API tokens")?;
    rows.iter().map(row_to_record).collect()
}

/// Révoque un jeton. Seule la première révocation renvoie « vrai ».
pub async fn revoke(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE api_tokens SET revoked_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ? AND revoked_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await
    .context("revoking the API token")?;
    Ok(result.rows_affected() > 0)
}

/// Ce que donne la présentation d'un jeton.
#[derive(Debug)]
pub enum Outcome {
    Valid(ApiToken),
    /// Inconnu, révoqué, ou propriétaire disparu : de l'extérieur, « pas
    /// authentifié ».
    Unknown,
    /// Le jeton est bon mais sa date est passée.
    Expired {
        name: String,
        at: String,
    },
    /// Le compte qui l'a créé est désactivé.
    OwnerDisabled {
        name: String,
    },
    /// Présenté hors des réseaux autorisés.
    NetworkDenied {
        name: String,
        ip: Option<IpAddr>,
    },
}

/// Vérifie un jeton en clair, depuis l'adresse `ip`, et dit ce qu'il autorise.
///
/// La recherche se fait sur le préfixe — la partie publique — puis l'empreinte est
/// comparée en temps constant, pour qu'aucune mesure de durée ne permette de la
/// reconstituer. Un jeton révoqué est refusé au même titre qu'un jeton inconnu.
/// Les autres refus (expiré, propriétaire désactivé, réseau) ne sont dits qu'à
/// qui détient déjà le jeton : ils n'apprennent rien à celui qui le devine.
pub async fn authenticate(pool: &SqlitePool, clear: &str, ip: Option<IpAddr>) -> Result<Outcome> {
    let rows = sqlx::query(
        "SELECT t.id, t.name, t.scope, t.token_hash, t.last_used_at, t.last_used_ip,
                t.expires_at, t.allowed_networks, t.user_id,
                u.username AS owner_name, u.role AS owner_role, u.disabled AS owner_disabled
         FROM api_tokens t LEFT JOIN users u ON u.id = t.user_id
         WHERE t.prefix = ? AND t.revoked_at IS NULL",
    )
    .bind(display_prefix(clear))
    .fetch_all(pool)
    .await
    .context("checking the API token")?;

    let presented = fingerprint(clear);
    for row in &rows {
        let stored: Vec<u8> = row.try_get("token_hash")?;
        let matches: bool = stored.ct_eq(&presented).into();
        if !matches {
            continue;
        }
        let id: i64 = row.try_get("id")?;
        let name: String = row.try_get("name")?;

        // Propriétaire : un compte disparu sans déclencheur (base restaurée à
        // la main, par exemple) ne laisse pas un jeton orphelin tout-puissant.
        let user_id: Option<i64> = row.try_get("user_id")?;
        let owner: Option<String> = row.try_get("owner_name")?;
        let owner_role: Option<String> = row.try_get("owner_role")?;
        let owner_disabled: Option<bool> = row.try_get("owner_disabled")?;
        if user_id.is_some() && owner.is_none() {
            return Ok(Outcome::Unknown);
        }
        if owner_disabled == Some(true) {
            return Ok(Outcome::OwnerDisabled { name });
        }

        let expires_at: Option<String> = row.try_get("expires_at")?;
        if let Some(at) = expires_at.as_deref()
            && parse_time(at).is_none_or(|at| at <= Utc::now())
        {
            return Ok(Outcome::Expired { name, at: at.to_string() });
        }

        let networks = decode_networks(row.try_get("allowed_networks")?);
        if !network_allows(&networks, ip) {
            return Ok(Outcome::NetworkDenied { name, ip });
        }

        let granted = Scope::parse(&row.try_get::<String, _>("scope")?).unwrap_or(Scope::Read);
        let owner_is_admin = owner_role.as_deref().is_none_or(|role| role == "admin");
        let scope = if owner_is_admin { granted } else { granted.min(Scope::Read) };

        let last_used_at: Option<String> = row.try_get("last_used_at")?;
        let last_used_ip: Option<String> = row.try_get("last_used_ip")?;
        touch_if_stale(pool, id, last_used_at.as_deref(), last_used_ip.as_deref(), ip).await;
        return Ok(Outcome::Valid(ApiToken {
            id,
            name,
            scope,
            owner,
            limited_by_owner: scope != granted,
        }));
    }
    Ok(Outcome::Unknown)
}

/// Met à jour `last_used_at` et `last_used_ip`, au plus une fois par minute —
/// sauf si l'adresse change : c'est justement ce qu'on veut voir.
///
/// Un échec ici n'est pas une raison de refuser l'appel : c'est un indice
/// d'affichage, pas une donnée de sécurité.
async fn touch_if_stale(
    pool: &SqlitePool,
    id: i64,
    last_used_at: Option<&str>,
    last_used_ip: Option<&str>,
    ip: Option<IpAddr>,
) {
    let ip = ip.map(|ip| ip.to_canonical().to_string());
    let fresh = last_used_at
        .and_then(parse_time)
        .is_some_and(|at| (Utc::now() - at).num_seconds() < TOUCH_INTERVAL.as_secs() as i64);
    if fresh && (ip.is_none() || ip.as_deref() == last_used_ip) {
        return;
    }
    let result = sqlx::query(
        "UPDATE api_tokens SET last_used_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                               last_used_ip = COALESCE(?, last_used_ip)
         WHERE id = ?",
    )
    .bind(ip)
    .bind(id)
    .execute(pool)
    .await;
    if let Err(error) = result {
        tracing::warn!(?error, token = id, "cannot record the API token usage");
    }
}

fn row_to_record(row: &sqlx::sqlite::SqliteRow) -> Result<TokenRecord> {
    let scope: String = row.try_get("scope")?;
    let expires_at: Option<String> = row.try_get("expires_at")?;
    let expired =
        expires_at.as_deref().is_some_and(|at| parse_time(at).is_none_or(|at| at <= Utc::now()));
    Ok(TokenRecord {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        prefix: row.try_get("prefix")?,
        scope: Scope::parse(&scope).unwrap_or(Scope::Read),
        created_at: row.try_get("created_at")?,
        created_by: row.try_get("created_by")?,
        expires_at,
        expired,
        allowed_networks: decode_networks(row.try_get("allowed_networks")?)
            .iter()
            .map(ToString::to_string)
            .collect(),
        last_used_at: row.try_get("last_used_at")?,
        last_used_ip: row.try_get("last_used_ip")?,
        revoked_at: row.try_get("revoked_at")?,
    })
}

// --------------------------------------------------------------------------
// Limitation par jeton
// --------------------------------------------------------------------------

/// Où en est un jeton dans sa fenêtre, pour les en-têtes `RateLimit-*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateStatus {
    pub limit: u32,
    pub remaining: u32,
    /// Secondes avant que la fenêtre ne reparte de zéro.
    pub reset_secs: u64,
}

impl RateStatus {
    /// Pose `RateLimit-Limit`, `RateLimit-Remaining` et `RateLimit-Reset` : un
    /// client sait ralentir avant de recevoir un 429.
    pub fn apply(&self, headers: &mut HeaderMap) {
        headers.insert("ratelimit-limit", HeaderValue::from(self.limit));
        headers.insert("ratelimit-remaining", HeaderValue::from(self.remaining));
        headers.insert("ratelimit-reset", HeaderValue::from(self.reset_secs));
    }
}

/// Compteur à fenêtre fixe, par jeton, en mémoire.
///
/// Le limiteur des connexions (`rate_limit.rs`) est global et pénalise les échecs ;
/// ici il s'agit de plafonner des appels *réussis* par identité, ce qui est un
/// autre problème. Une instance est un processus unique : la mémoire suffit.
pub struct TokenRateLimiter {
    windows: HashMap<i64, (Instant, u32)>,
}

impl TokenRateLimiter {
    pub fn new() -> Self {
        Self { windows: HashMap::new() }
    }

    /// Compte un appel. En cas de refus, renvoie les secondes à attendre.
    pub fn check(&mut self, token_id: i64, now: Instant) -> Result<RateStatus, u64> {
        let entry = self.windows.entry(token_id).or_insert((now, 0));
        if now.duration_since(entry.0) >= RATE_WINDOW {
            *entry = (now, 0);
        }
        let reset = RATE_WINDOW.saturating_sub(now.duration_since(entry.0)).as_secs() + 1;
        if entry.1 >= RATE_LIMIT_PER_MINUTE {
            return Err(reset);
        }
        entry.1 += 1;
        let status = RateStatus {
            limit: RATE_LIMIT_PER_MINUTE,
            remaining: RATE_LIMIT_PER_MINUTE - entry.1,
            reset_secs: reset,
        };
        // Les jetons oubliés ne doivent pas s'accumuler ; ils sont rares, on balaie
        // quand la table grossit.
        if self.windows.len() > 256 {
            self.windows.retain(|_, (start, _)| now.duration_since(*start) < RATE_WINDOW);
        }
        Ok(status)
    }
}

impl Default for TokenRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

static LIMITER: LazyLock<Mutex<TokenRateLimiter>> =
    LazyLock::new(|| Mutex::new(TokenRateLimiter::new()));

// --------------------------------------------------------------------------
// Garde de route
// --------------------------------------------------------------------------

/// Erreur d'authentification par jeton, même corps `{"error": …}` que le reste.
pub enum TokenError {
    Unauthorized,
    Expired(String),
    OwnerDisabled(String),
    Forbidden(String),
    TooManyRequests(u64),
    Internal(anyhow::Error),
}

impl IntoResponse for TokenError {
    fn into_response(self) -> Response {
        let (status, message, challenge) = match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "A valid API token is required.".to_string(),
                "Bearer".to_string(),
            ),
            Self::Expired(message) => (
                StatusCode::UNAUTHORIZED,
                message,
                r#"Bearer error="invalid_token", error_description="The token expired""#
                    .to_string(),
            ),
            Self::OwnerDisabled(message) => (
                StatusCode::UNAUTHORIZED,
                message,
                r#"Bearer error="invalid_token", error_description="The token owner is disabled""#
                    .to_string(),
            ),
            Self::Forbidden(message) => (StatusCode::FORBIDDEN, message, String::new()),
            Self::TooManyRequests(retry_after) => {
                let mut response = (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({
                        "error": format!(
                            "Rate limit reached ({RATE_LIMIT_PER_MINUTE} calls per minute per \
                             token). Retry in {retry_after} s."
                        )
                    })),
                )
                    .into_response();
                let headers = response.headers_mut();
                headers.insert(header::RETRY_AFTER, HeaderValue::from(retry_after));
                RateStatus { limit: RATE_LIMIT_PER_MINUTE, remaining: 0, reset_secs: retry_after }
                    .apply(headers);
                return response;
            }
            Self::Internal(error) => {
                tracing::error!(?error, "internal error while checking an API token");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error.".to_string(),
                    String::new(),
                )
            }
        };
        let mut response = (status, Json(json!({ "error": message }))).into_response();
        // Indique le schéma attendu, comme le veut HTTP ; les clients MCP s'en
        // servent pour distinguer « pas de jeton » de « refusé ».
        if !challenge.is_empty()
            && let Ok(value) = HeaderValue::from_str(&challenge)
        {
            response.headers_mut().insert(header::WWW_AUTHENTICATE, value);
        }
        response
    }
}

/// Jeton vérifié et compté.
#[derive(Debug, Clone)]
pub struct Checked {
    pub token: ApiToken,
    pub rate: RateStatus,
}

/// Adresse du client d'une requête, selon les mandataires déclarés de confiance
/// (voir [`client_ip`]).
pub fn request_ip(extensions: &Extensions, headers: &HeaderMap) -> Option<IpAddr> {
    let trusted = extensions
        .get::<AuthState>()
        .map(|auth| auth.trusted_proxies().to_vec())
        .unwrap_or_default();
    client_ip::resolve(extensions, headers, &trusted)
}

/// Garde à poser en `route_layer` : exige un jeton valide couvrant `scope`, et
/// dépose l'[`ApiToken`] dans la requête.
///
/// Un jeton absent, mal formé, inconnu, révoqué ou expiré donne 401 ; une
/// portée insuffisante ou un réseau non autorisé 403 ; un jeton qui boucle 429.
pub fn require_token(state: AppState, scope: Scope) -> TokenLayer {
    middleware::from_fn_with_state((state, scope), guard as GuardFn)
}

/// Type concret de la couche, pour que `require_token` puisse le nommer.
pub type TokenLayer = FromFnLayer<GuardFn, (AppState, Scope), (State<(AppState, Scope)>, Request)>;

/// Type de la fonction de garde, pour nommer ce que renvoie [`require_token`].
pub type GuardFn = fn(
    State<(AppState, Scope)>,
    Request,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>;

fn guard(
    State((state, scope)): State<(AppState, Scope)>,
    mut request: Request,
    next: Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> {
    Box::pin(async move {
        let ip = request_ip(request.extensions(), request.headers());
        let checked = match check(&state.pool, request.headers(), ip, scope).await {
            Ok(checked) => checked,
            Err(error) => return error.into_response(),
        };
        request.extensions_mut().insert(checked.token);
        let mut response = next.run(request).await;
        checked.rate.apply(response.headers_mut());
        response
    })
}

/// Vérifie l'en-tête, la validité, le réseau, la portée et le plafond d'appels.
pub async fn check(
    pool: &SqlitePool,
    headers: &HeaderMap,
    ip: Option<IpAddr>,
    scope: Scope,
) -> Result<Checked, TokenError> {
    let Some(clear) = extract_bearer(headers) else { return Err(TokenError::Unauthorized) };
    let token = match authenticate(pool, clear, ip).await.map_err(TokenError::Internal)? {
        Outcome::Valid(token) => token,
        Outcome::Unknown => return Err(TokenError::Unauthorized),
        Outcome::Expired { name, at } => {
            return Err(TokenError::Expired(format!(
                "The API token \"{name}\" expired on {at}. Create a new one in Settings → API \
                 & assistants."
            )));
        }
        Outcome::OwnerDisabled { name } => {
            return Err(TokenError::OwnerDisabled(format!(
                "The API token \"{name}\" belongs to a disabled account."
            )));
        }
        Outcome::NetworkDenied { name, ip } => {
            tracing::warn!(token = %name, ip = ?ip, "API token used outside its allowed networks");
            let from = ip.map_or_else(|| "an unknown address".to_string(), |ip| ip.to_string());
            return Err(TokenError::Forbidden(format!(
                "The API token \"{name}\" cannot be used from {from}."
            )));
        }
    };
    token.require(scope).map_err(TokenError::Forbidden)?;

    let verdict = LIMITER.lock().unwrap_or_else(|e| e.into_inner()).check(token.id, Instant::now());
    match verdict {
        Ok(rate) => Ok(Checked { token, rate }),
        Err(retry_after) => {
            tracing::warn!(token = %token.name, "API token rate limit reached");
            Err(TokenError::TooManyRequests(retry_after))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_token_is_recognisable_and_unique() {
        let first = generate();
        let second = generate();
        assert!(first.starts_with(TOKEN_PREFIX));
        assert_eq!(first.len(), TOKEN_PREFIX.len() + TOKEN_BYTES * 2);
        assert_ne!(first, second);
    }

    #[test]
    fn the_display_prefix_reveals_only_a_handful_of_characters() {
        let token = generate();
        let prefix = display_prefix(&token);
        assert_eq!(prefix.len(), DISPLAY_LEN);
        assert!(token.starts_with(&prefix));
        assert!(prefix.len() < token.len() / 2);
    }

    #[test]
    fn the_fingerprint_never_contains_the_token() {
        let token = generate();
        let digest = hex::encode(fingerprint(&token));
        assert!(!digest.contains(&token[TOKEN_PREFIX.len()..]));
        assert_eq!(fingerprint(&token), fingerprint(&token));
    }

    #[test]
    fn write_covers_read_but_not_the_reverse() {
        assert!(Scope::Write.allows(Scope::Read));
        assert!(Scope::Write.allows(Scope::Write));
        assert!(Scope::Read.allows(Scope::Read));
        assert!(!Scope::Read.allows(Scope::Write));
    }

    #[test]
    fn only_a_bearer_with_our_prefix_is_accepted() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_bearer(&headers), None);

        headers.insert(header::AUTHORIZATION, "Bearer dmt_abc".parse().unwrap());
        assert_eq!(extract_bearer(&headers), Some("dmt_abc"));

        headers.insert(header::AUTHORIZATION, "bearer   dmt_abc  ".parse().unwrap());
        assert_eq!(extract_bearer(&headers), Some("dmt_abc"));

        // Un jeton d'agent ou un mot de passe Basic ne sont pas des jetons d'API.
        headers.insert(header::AUTHORIZATION, "Bearer dmon_abc".parse().unwrap());
        assert_eq!(extract_bearer(&headers), None);
        headers.insert(header::AUTHORIZATION, "Basic dXNlcjpwYXNz".parse().unwrap());
        assert_eq!(extract_bearer(&headers), None);
    }

    #[test]
    fn the_rate_limit_resets_with_the_window() {
        let mut limiter = TokenRateLimiter::new();
        let now = Instant::now();
        let first = limiter.check(7, now).expect("first call");
        assert_eq!(first.limit, RATE_LIMIT_PER_MINUTE);
        assert_eq!(first.remaining, RATE_LIMIT_PER_MINUTE - 1);
        assert!(first.reset_secs > 0 && first.reset_secs <= 61);
        for _ in 1..RATE_LIMIT_PER_MINUTE {
            assert!(limiter.check(7, now).is_ok());
        }
        let wait = limiter.check(7, now).expect_err("the next call is refused");
        assert!(wait > 0 && wait <= 61, "{wait}");
        // Un autre jeton n'est pas pénalisé.
        assert!(limiter.check(8, now).is_ok());
        // La fenêtre suivante repart de zéro.
        assert!(limiter.check(7, now + Duration::from_secs(61)).is_ok());
    }

    #[test]
    fn networks_are_normalised_and_checked() {
        let parsed = parse_networks(&[
            " 192.168.1.7/24 ".into(),
            "10.0.0.5".into(),
            "".into(),
            "192.168.1.0/24".into(),
            "fd00::/8".into(),
        ])
        .expect("valid list");
        let shown: Vec<String> = parsed.iter().map(ToString::to_string).collect();
        assert_eq!(shown, ["192.168.1.0/24", "10.0.0.5/32", "fd00::/8"]);

        assert!(parse_networks(&["192.168.1.0/33".into()]).is_err());
        assert!(parse_networks(&["nas.lan".into()]).unwrap_err().contains("nas.lan"));
        let too_many: Vec<String> = (0..=MAX_NETWORKS).map(|i| format!("10.0.{i}.0/24")).collect();
        assert!(parse_networks(&too_many).is_err());

        let ip = |raw: &str| Some(raw.parse::<IpAddr>().unwrap());
        assert!(network_allows(&[], None), "sans restriction, tout passe");
        assert!(network_allows(&parsed, ip("192.168.1.200")));
        assert!(network_allows(&parsed, ip("::ffff:192.168.1.9")), "IPv4 vue en IPv6");
        assert!(network_allows(&parsed, ip("fd12::1")));
        assert!(!network_allows(&parsed, ip("192.168.2.1")));
        assert!(!network_allows(&parsed, None), "adresse inconnue : refus");
    }

    #[test]
    fn a_token_limited_by_its_owner_says_why() {
        let token = ApiToken {
            id: 1,
            name: "CI".into(),
            scope: Scope::Read,
            owner: Some("alice".into()),
            limited_by_owner: true,
        };
        let message = token.require(Scope::Write).unwrap_err();
        assert!(message.contains("alice") && message.contains("no longer an administrator"));
        assert!(token.require(Scope::Read).is_ok());
    }
}
