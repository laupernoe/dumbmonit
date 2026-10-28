//! Équipes et droits par dossier.
//!
//! Une équipe regroupe des comptes et reçoit, pour des dossiers d'équipements,
//! un niveau de droit ([`Level`]). Le rôle global du compte reste le plafond :
//! un `viewer` ne dépasse jamais la lecture, un `operator` peut aller jusqu'à la
//! gestion, un `admin` n'est jamais restreint.
//!
//! La règle de visibilité tient en trois lignes :
//!
//! - un administrateur (ou un jeton dont le propriétaire est administrateur, ou
//!   un jeton sans propriétaire connu, créé avant les comptes) voit tout ;
//! - un compte membre d'au moins une équipe ne voit que ce que ses équipes lui
//!   ouvrent ([`Access::Scoped`]) ;
//! - un compte sans équipe garde le comportement d'avant les équipes, sauf si
//!   l'administrateur a choisi « aucun accès » ([`UnassignedAccess::None`]).
//!
//! L'application est faite côté serveur, en deux endroits seulement :
//!
//! - le garde d'API ([`guard`]) refuse à un compte restreint toute route qui
//!   n'est pas dans la liste fermée [`SCOPED_ROUTES`] — une route ajoutée demain
//!   reste donc invisible aux équipes tant que personne ne l'a vérifiée — et
//!   contrôle lui-même les routes qui désignent un équipement (`/targets/{id}/…`,
//!   alertes, silences, surcharges) ;
//! - les routes de liste de [`SCOPED_ROUTES`] filtrent leur réponse avec
//!   [`Access::visible_targets`].

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::FromRequestParts;
use axum::http::Method;
use axum::http::request::Parts;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::auth::AuthError;
use crate::auth::middleware::Principal;
use crate::auth::users::{self, Role, User};

/// Niveau de droit sur un dossier. L'ordre compte : chaque niveau contient le
/// précédent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Voir les équipements, leurs métriques et leurs alertes.
    Read,
    /// Et traiter les alertes : acquitter, écarter, mettre en sourdine.
    Operate,
    /// Et modifier, ajouter ou supprimer les équipements du dossier.
    Manage,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Operate => "operate",
            Self::Manage => "manage",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read" => Some(Self::Read),
            "operate" => Some(Self::Operate),
            "manage" => Some(Self::Manage),
            _ => None,
        }
    }

    /// Le plafond qu'impose un rôle global.
    pub fn ceiling(role: Role) -> Self {
        match role {
            Role::Viewer => Self::Read,
            Role::Operator | Role::Admin => Self::Manage,
        }
    }
}

/// Dossier qui couvre tous les équipements.
pub const ALL_FOLDERS: &str = "*";

/// Longueur maximale d'un nom d'équipe, d'une description, d'un groupe.
const MAX_NAME_CHARS: usize = 80;
const MAX_DESCRIPTION_CHARS: usize = 500;
const MAX_GROUP_CHARS: usize = 512;
const MAX_GROUPS: usize = 64;
const MAX_GRANTS: usize = 256;

/// Un droit d'équipe sur un dossier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    pub folder: String,
    pub level: Level,
}

/// Normalise un nom de dossier tel qu'on le compare : sans espaces ni barres
/// obliques en bordure, segments vides retirés (`Paris//Rack` → `Paris/Rack`).
pub fn normalize_folder(folder: &str) -> String {
    let folder = folder.trim();
    if folder == ALL_FOLDERS {
        return ALL_FOLDERS.to_string();
    }
    folder.split('/').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("/")
}

/// Vrai si un droit posé sur `grant` couvre un équipement rangé dans `folder`.
///
/// `*` couvre tout ; la chaîne vide ne couvre que les équipements sans dossier ;
/// un dossier couvre lui-même et ses sous-dossiers (`Paris` couvre `Paris/Rack
/// A`, pas `Paris-Nord`).
pub fn folder_covers(grant: &str, folder: &str) -> bool {
    if grant == ALL_FOLDERS {
        return true;
    }
    let folder = normalize_folder(folder);
    if grant.is_empty() {
        return folder.is_empty();
    }
    folder == grant || folder.strip_prefix(grant).is_some_and(|rest| rest.starts_with('/'))
}

/// Ce que l'on fait d'un compte qui n'est membre d'aucune équipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnassignedAccess {
    /// Comme avant les équipes : tout, au niveau de son rôle.
    #[default]
    All,
    /// Rien : il faut une équipe pour voir quoi que ce soit.
    None,
}

/// Réglages des équipes, stockés dans `settings` sous [`SETTINGS_KEY`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamSettings {
    #[serde(default)]
    pub unassigned_access: UnassignedAccess,
}

pub const SETTINGS_KEY: &str = "teams.access";

pub async fn settings(pool: &SqlitePool) -> Result<TeamSettings> {
    Ok(crate::auth::settings::get(pool, SETTINGS_KEY).await?.unwrap_or_default())
}

pub async fn save_settings(pool: &SqlitePool, value: &TeamSettings) -> Result<()> {
    crate::auth::settings::set(pool, SETTINGS_KEY, value).await
}

/// Ce qu'une requête a le droit de voir et de faire, du point de vue des
/// équipes. Le rôle global reste vérifié à part, par le garde d'API.
#[derive(Debug, Clone)]
pub enum Access {
    /// Aucune restriction d'équipe.
    All,
    /// Restreint aux dossiers de ses équipes.
    Scoped(Arc<Scope>),
}

/// Les droits d'un compte restreint.
#[derive(Debug, Clone)]
pub struct Scope {
    /// Plafond imposé par le rôle global.
    pub ceiling: Level,
    /// Droits de toutes ses équipes, mis bout à bout.
    pub grants: Vec<Grant>,
    /// Noms de ses équipes, pour l'interface.
    pub teams: Vec<String>,
    /// Mode MSP : les clients dont il voit les équipements. `None` : aucune
    /// restriction de client (mode coupé, ou administrateur). Le niveau client
    /// passe avant le niveau dossier : un dossier ouvert par une équipe ne
    /// montre jamais les équipements d'un autre client.
    pub clients: Option<Vec<i64>>,
    /// Vue d'un administrateur limitée à un client par le sélecteur de l'en-tête
    /// : un filtre d'affichage, pas une barrière. Le garde de routes ne
    /// s'applique pas et les écritures restent celles d'un administrateur.
    pub admin_view: bool,
}

impl Scope {
    /// Niveau effectif sur un équipement rangé dans `folder` : le plus fort des
    /// droits qui le couvrent, borné par le rôle. `None` : invisible.
    pub fn level_for(&self, folder: &str) -> Option<Level> {
        self.grants
            .iter()
            .filter(|grant| folder_covers(&grant.folder, folder))
            .map(|grant| grant.level)
            .max()
            .map(|level| level.min(self.ceiling))
    }

    /// Vrai si le client `client` est visible.
    pub fn sees_client(&self, client: i64) -> bool {
        self.clients.as_ref().is_none_or(|clients| clients.contains(&client))
    }

    /// Niveau effectif sur un équipement du client `client`, rangé dans
    /// `folder` : le client d'abord, le dossier ensuite.
    pub fn level_for_target(&self, client: i64, folder: &str) -> Option<Level> {
        if !self.sees_client(client) {
            return None;
        }
        self.level_for(folder)
    }

    /// Droits qui ne laissent rien voir : la valeur par défaut quand on ne sait
    /// pas qui parle.
    pub fn nothing(ceiling: Level) -> Self {
        Self { ceiling, grants: Vec::new(), teams: Vec::new(), clients: None, admin_view: false }
    }
}

impl Access {
    /// Vrai si le compte est restreint (équipes ou clients). La vue d'un
    /// administrateur limitée à un client ne compte pas : elle filtre les
    /// listes, elle n'interdit rien.
    pub fn is_scoped(&self) -> bool {
        matches!(self, Self::Scoped(scope) if !scope.admin_view)
    }

    /// Les clients visibles, `None` sans restriction de client.
    pub fn clients(&self) -> Option<&[i64]> {
        self.scope().and_then(|scope| scope.clients.as_deref())
    }

    /// Vrai si le client `client` est visible.
    pub fn sees_client(&self, client: i64) -> bool {
        self.scope().is_none_or(|scope| scope.sees_client(client))
    }

    pub fn scope(&self) -> Option<&Scope> {
        match self {
            Self::All => None,
            Self::Scoped(scope) => Some(scope),
        }
    }

    /// Les équipements visibles, ou `None` quand tout l'est.
    pub async fn visible_targets(&self, pool: &SqlitePool) -> Result<Option<HashSet<i64>>> {
        self.targets_at(pool, Level::Read).await
    }

    /// Les équipements sur lesquels on a au moins `level`, ou `None` sans
    /// restriction d'équipe.
    pub async fn targets_at(
        &self,
        pool: &SqlitePool,
        level: Level,
    ) -> Result<Option<HashSet<i64>>> {
        let Self::Scoped(scope) = self else { return Ok(None) };
        let rows = sqlx::query("SELECT id, group_name, client_id FROM targets")
            .fetch_all(pool)
            .await
            .context("lecture des dossiers des équipements")?;
        let mut visible = HashSet::new();
        for row in rows {
            let id: i64 = row.try_get("id")?;
            let folder: String = row.try_get("group_name")?;
            let client: i64 = row.try_get("client_id")?;
            if scope.level_for_target(client, &folder).is_some_and(|have| have >= level) {
                visible.insert(id);
            }
        }
        Ok(Some(visible))
    }

    /// Vrai si l'équipement `target_id` est visible.
    pub async fn can_see(&self, pool: &SqlitePool, target_id: i64) -> Result<bool> {
        Ok(self.target_level(pool, target_id).await?.is_some())
    }

    /// Niveau sur un équipement : `Some(Manage)` sans restriction, `None` s'il
    /// est invisible ou n'existe pas.
    pub async fn target_level(&self, pool: &SqlitePool, target_id: i64) -> Result<Option<Level>> {
        let Self::Scoped(scope) = self else { return Ok(Some(Level::Manage)) };
        let row: Option<(String, i64)> =
            sqlx::query_as("SELECT group_name, client_id FROM targets WHERE id = ?")
                .bind(target_id)
                .fetch_optional(pool)
                .await
                .context("lecture du dossier d'un équipement")?;
        Ok(row.and_then(|(folder, client)| scope.level_for_target(client, &folder)))
    }

    /// Niveau sur un dossier (pour une création ou un déplacement).
    pub fn folder_level(&self, folder: &str) -> Option<Level> {
        match self {
            Self::All => Some(Level::Manage),
            Self::Scoped(scope) => scope.level_for(folder),
        }
    }

    /// Exige `level` sur l'équipement : 404 s'il est invisible (on ne révèle
    /// pas son existence), 403 si le niveau est insuffisant.
    pub async fn require_target(
        &self,
        pool: &SqlitePool,
        target_id: i64,
        level: Level,
    ) -> Result<(), AuthError> {
        match self.target_level(pool, target_id).await? {
            None => Err(AuthError::NotFound("Target not found.".into())),
            Some(have) if have >= level => Ok(()),
            Some(_) => Err(insufficient(level)),
        }
    }
}

fn insufficient(level: Level) -> AuthError {
    AuthError::Forbidden(match level {
        Level::Read => "Your teams do not give you access to this target.".into(),
        Level::Operate => "Your teams only let you view this target, not handle its alerts.".into(),
        Level::Manage => "Your teams do not let you edit targets in this folder.".into(),
    })
}

/// Droits d'un compte : ceux de ses équipes, puis le niveau client du mode MSP
/// (voir [`crate::auth::clients::restrict`]).
pub async fn access_for_user(pool: &SqlitePool, user: &User) -> Result<Access> {
    let teams = team_access_for_user(pool, user).await?;
    crate::auth::clients::restrict(pool, user, teams).await
}

/// Droits d'équipe d'un compte, sans le niveau client.
async fn team_access_for_user(pool: &SqlitePool, user: &User) -> Result<Access> {
    if user.role.is_admin() {
        return Ok(Access::All);
    }
    let rows = sqlx::query(
        "SELECT t.name AS name, g.folder AS folder, g.level AS level
         FROM team_members m
         JOIN teams t ON t.id = m.team_id
         LEFT JOIN team_folder_grants g ON g.team_id = t.id
         WHERE m.user_id = ?
         ORDER BY t.name COLLATE NOCASE",
    )
    .bind(user.id)
    .fetch_all(pool)
    .await
    .context("lecture des équipes d'un compte")?;

    if rows.is_empty() {
        return Ok(match settings(pool).await?.unassigned_access {
            UnassignedAccess::All => Access::All,
            UnassignedAccess::None => {
                Access::Scoped(Arc::new(Scope::nothing(Level::ceiling(user.role))))
            }
        });
    }

    let mut teams: Vec<String> = Vec::new();
    let mut grants = Vec::new();
    for row in rows {
        let name: String = row.try_get("name")?;
        if !teams.contains(&name) {
            teams.push(name);
        }
        let folder: Option<String> = row.try_get("folder")?;
        let level: Option<String> = row.try_get("level")?;
        if let (Some(folder), Some(level)) = (folder, level.as_deref().and_then(Level::parse)) {
            grants.push(Grant { folder, level });
        }
    }
    Ok(Access::Scoped(Arc::new(Scope {
        ceiling: Level::ceiling(user.role),
        grants,
        teams,
        clients: None,
        admin_view: false,
    })))
}

/// Droits d'équipe d'une requête, quel qu'en soit le porteur. Un jeton hérite
/// de ceux de son propriétaire ; un propriétaire disparu ne laisse rien voir.
pub async fn access_for(pool: &SqlitePool, principal: &Principal) -> Result<Access> {
    match principal {
        Principal::User(user) => access_for_user(pool, user).await,
        Principal::Token(token) => {
            let Some(owner) = token.owner.as_deref() else { return Ok(Access::All) };
            match users::by_username(pool, owner).await? {
                Some(user) if !user.disabled => access_for_user(pool, &user).await,
                _ => Ok(Access::Scoped(Arc::new(Scope::nothing(Level::Read)))),
            }
        }
    }
}

/// Droits d'équipe de la requête, déposés par le garde d'API.
#[derive(Debug, Clone)]
pub struct CurrentAccess(pub Access);

/// Extracteur : les droits d'équipe de la requête. Sans dépôt du garde (route
/// publique, test unitaire), on ne présume rien : aucun équipement visible.
pub struct TeamAccess(pub Access);

impl<S: Send + Sync> FromRequestParts<S> for TeamAccess {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(parts.extensions.get::<CurrentAccess>().map_or_else(
            || Access::Scoped(Arc::new(Scope::nothing(Level::Read))),
            |current| current.0.clone(),
        )))
    }
}

// ---------------------------------------------------------------------------
// Garde d'API
// ---------------------------------------------------------------------------

/// Ce qu'une route de [`SCOPED_ROUTES`] laisse faire à un compte restreint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteRule {
    /// Route sans donnée d'équipement (son propre compte, catalogues) ou dont
    /// le gestionnaire filtre lui-même la réponse.
    Open,
    /// Alerte désignée par son empreinte (2ᵉ segment) : `operate` sur sa cible.
    AlertFingerprint,
    /// Entrée d'historique désignée par son numéro (3ᵉ segment).
    HistoryEntry,
    /// Silence désigné par son numéro (3ᵉ segment).
    Silence,
    /// Surcharge de règle : la cible est le dernier segment.
    OverrideTarget,
}

/// Routes ouvertes aux comptes restreints par leurs équipes, en plus de
/// `/targets/{id}/…` (contrôlé à part). Liste fermée : tout le reste répond 403.
/// Gabarits sans `/api`, `*` = un segment non vide, `**` = la suite du chemin.
pub const SCOPED_ROUTES: &[(&str, &str, RouteRule)] = &[
    // Son propre compte.
    ("*", "/auth/**", RouteRule::Open),
    ("*", "/webpush/**", RouteRule::Open),
    // Catalogues, sans donnée d'équipement.
    ("GET", "/collectors", RouteRule::Open),
    ("GET", "/notify/kinds", RouteRule::Open),
    ("GET", "/geoip", RouteRule::Open),
    ("GET", "/locations", RouteRule::Open),
    ("GET", "/locations/basemap", RouteRule::Open),
    // Filtrées par le gestionnaire.
    ("GET", "/targets", RouteRule::Open),
    ("POST", "/targets", RouteRule::Open),
    ("GET", "/metrics/query", RouteRule::Open),
    ("GET", "/metrics/query_range", RouteRule::Open),
    ("GET", "/metrics/catalog", RouteRule::Open),
    // Tableaux de bord : leur définition ne porte aucune donnée d'équipement, et
    // leurs widgets passent par les requêtes et la liste d'alertes filtrées
    // ci-dessus. Le gestionnaire réserve les tableaux partagés aux administrateurs.
    ("*", "/dashboards", RouteRule::Open),
    ("*", "/dashboards/*", RouteRule::Open),
    // Aperçu d'un modèle : le gestionnaire ne lit que les équipements visibles.
    ("GET", "/dashboards/templates/*/preview", RouteRule::Open),
    ("GET", "/alerts", RouteRule::Open),
    ("GET", "/alerts/history", RouteRule::Open),
    ("GET", "/alerts/rules", RouteRule::Open),
    ("GET", "/alerts/silences", RouteRule::Open),
    ("POST", "/alerts/silences", RouteRule::Open),
    ("GET", "/topology", RouteRule::Open),
    ("GET", "/status-pages", RouteRule::Open),
    ("GET", "/security/cves", RouteRule::Open),
    ("GET", "/security/cves/summary", RouteRule::Open),
    ("GET", "/security/cves/inventory", RouteRule::Open),
    // Mode MSP : le rapport SLA et le logo de ses propres clients (le
    // gestionnaire vérifie le client demandé).
    ("GET", "/reports/sla", RouteRule::Open),
    ("GET", "/clients/*/logo", RouteRule::Open),
    // Traitement d'une alerte précise : contrôlé ici.
    ("POST", "/alerts/*/ack", RouteRule::AlertFingerprint),
    ("DELETE", "/alerts/*/ack", RouteRule::AlertFingerprint),
    ("POST", "/alerts/*/dismiss", RouteRule::AlertFingerprint),
    ("DELETE", "/alerts/*/dismiss", RouteRule::AlertFingerprint),
    ("POST", "/alerts/history/*/dismiss", RouteRule::HistoryEntry),
    ("DELETE", "/alerts/silences/*", RouteRule::Silence),
    ("PUT", "/alerts/rules/*/overrides/*", RouteRule::OverrideTarget),
    ("DELETE", "/alerts/rules/*/overrides/*", RouteRule::OverrideTarget),
];

/// Écritures sous `/targets/{id}/…` que la gestion d'équipe ouvre à un
/// opérateur (le garde de rôle les réserve sinon à l'administrateur).
pub const MANAGE_TARGET_ROUTES: &[(&str, &str)] = &[
    ("PUT", "/targets/*"),
    ("DELETE", "/targets/*"),
    ("POST", "/targets/*/probe"),
    ("POST", "/targets"),
];

const NOT_FOR_TEAMS: &str = "This page is not available to team-restricted accounts: ask an \
                             administrator.";

fn strip(path: &str) -> &str {
    let path = path.strip_prefix("/api").unwrap_or(path);
    path.strip_suffix('/').filter(|p| !p.is_empty()).unwrap_or(path)
}

/// Comparaison d'un chemin à un gabarit (`*` un segment, `**` la suite).
pub fn path_matches(pattern: &str, path: &str) -> bool {
    let mut wanted = pattern.split('/');
    let mut actual = path.split('/');
    loop {
        match (wanted.next(), actual.next()) {
            (None, None) => return true,
            (Some("**"), _) => return true,
            (Some("*"), Some(segment)) if !segment.is_empty() => {}
            (Some(w), Some(a)) if w == a => {}
            _ => return false,
        }
    }
}

fn method_matches(wanted: &str, method: &Method) -> bool {
    wanted == "*" || wanted == method.as_str() || (wanted == "GET" && *method == Method::HEAD)
}

/// Vrai si l'écriture fait partie de ce que `manage` ouvre à un opérateur.
pub fn is_manage_route(method: &Method, path: &str) -> bool {
    let path = strip(path);
    MANAGE_TARGET_ROUTES
        .iter()
        .any(|(m, pattern)| method_matches(m, method) && path_matches(pattern, path))
}

/// `/targets/{id}` ou `/targets/{id}/…` : l'identifiant, s'il est numérique.
pub fn target_in_path(path: &str) -> Option<i64> {
    let rest = strip(path).strip_prefix("/targets/")?;
    let id = rest.split('/').next()?;
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    id.parse().ok()
}

/// Le garde des équipes, appelé par `require_session` après le contrôle de rôle.
pub async fn guard(
    pool: &SqlitePool,
    access: &Access,
    method: &Method,
    path: &str,
) -> Result<(), AuthError> {
    if !access.is_scoped() {
        return Ok(());
    }
    let path = strip(path);
    let read = matches!(*method, Method::GET | Method::HEAD);

    if let Some(target_id) = target_in_path(path) {
        let level = if read {
            Level::Read
        } else if is_manage_route(method, path) {
            Level::Manage
        } else {
            // Écriture réservée à l'administrateur par le garde de rôle ; on ne
            // l'atteint pas, mais on ne présume rien.
            return Err(AuthError::admin_required());
        };
        return access.require_target(pool, target_id, level).await;
    }

    let Some((_, _, rule)) = SCOPED_ROUTES
        .iter()
        .find(|(m, pattern, _)| method_matches(m, method) && path_matches(pattern, path))
    else {
        return Err(AuthError::Forbidden(NOT_FOR_TEAMS.into()));
    };
    let segment = |n: usize| path.split('/').nth(n).unwrap_or_default().to_string();
    let target: Option<i64> = match rule {
        RouteRule::Open => return Ok(()),
        RouteRule::AlertFingerprint => sqlx::query_scalar::<_, Option<i64>>(
            "SELECT target_id FROM alert_state WHERE fingerprint = ?",
        )
        .bind(segment(2))
        .fetch_optional(pool)
        .await?
        .flatten(),
        RouteRule::HistoryEntry => {
            sqlx::query_scalar::<_, Option<i64>>("SELECT target_id FROM alert_history WHERE id = ?")
                .bind(segment(3).parse::<i64>().unwrap_or(-1))
                .fetch_optional(pool)
                .await?
                .flatten()
        }
        RouteRule::Silence => {
            sqlx::query_scalar::<_, Option<i64>>("SELECT target_id FROM silences WHERE id = ?")
                .bind(segment(3).parse::<i64>().unwrap_or(-1))
                .fetch_optional(pool)
                .await?
                .flatten()
        }
        RouteRule::OverrideTarget => path.rsplit('/').next().and_then(|id| id.parse().ok()),
    };
    // Une alerte, une entrée ou un silence sans équipement n'appartient à aucun
    // dossier : seul un compte non restreint y touche.
    let Some(target) = target else {
        return Err(AuthError::NotFound("Not found.".into()));
    };
    access.require_target(pool, target, Level::Operate).await
}

/// Filtre de séries VictoriaMetrics (`extra_filters[]`) qui limite une requête
/// aux équipements visibles. `None` : aucune restriction.
pub fn series_filter(visible: Option<&HashSet<i64>>) -> Option<String> {
    let visible = visible?;
    if visible.is_empty() {
        // Aucun identifiant réel n'est négatif : rien ne correspond.
        return Some("{target=\"-1\"}".to_string());
    }
    let mut ids: Vec<i64> = visible.iter().copied().collect();
    ids.sort_unstable();
    let alternatives = ids.iter().map(i64::to_string).collect::<Vec<_>>().join("|");
    Some(format!("{{target=~\"{alternatives}\"}}"))
}

// ---------------------------------------------------------------------------
// Données
// ---------------------------------------------------------------------------

/// Membre d'une équipe, tel que l'expose l'API.
#[derive(Debug, Clone, Serialize)]
pub struct Member {
    pub user_id: i64,
    pub username: String,
    pub display_name: String,
    pub role: Role,
    /// `manual`, `idp` ou `scim`.
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Team {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub idp_groups: Vec<String>,
    pub members: Vec<Member>,
    pub grants: Vec<Grant>,
    pub created_at: String,
}

/// Champs modifiables d'une équipe. `None` : inchangé (création : vide).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TeamInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub idp_groups: Option<Vec<String>>,
    /// Comptes inscrits à la main ; ceux venus d'un fournisseur ou de SCIM
    /// sont conservés.
    pub members: Option<Vec<i64>>,
    pub grants: Option<Vec<Grant>>,
}

/// Valide un nom d'équipe.
pub fn validate_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a team name.".into());
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!("The team name must be {MAX_NAME_CHARS} characters or fewer."));
    }
    if name.chars().any(char::is_control) {
        return Err("The team name cannot contain control characters.".into());
    }
    Ok(name.to_string())
}

fn clean_groups(groups: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for group in groups {
        let group = group.trim();
        if group.is_empty() {
            continue;
        }
        if group.chars().count() > MAX_GROUP_CHARS || group.chars().any(char::is_control) {
            return Err("An identity-provider group name is invalid or too long.".into());
        }
        if !out.iter().any(|g| g.eq_ignore_ascii_case(group)) {
            out.push(group.to_string());
        }
    }
    if out.len() > MAX_GROUPS {
        return Err(format!("A team maps at most {MAX_GROUPS} identity-provider groups."));
    }
    Ok(out)
}

fn clean_grants(grants: &[Grant]) -> Result<Vec<Grant>, String> {
    let mut merged: BTreeMap<String, Level> = BTreeMap::new();
    for grant in grants {
        let folder = normalize_folder(&grant.folder);
        if folder.chars().count() > 200 || folder.chars().any(char::is_control) {
            return Err("A folder name is invalid or too long.".into());
        }
        let level = merged.get(&folder).map_or(grant.level, |have| (*have).max(grant.level));
        merged.insert(folder, level);
    }
    if merged.len() > MAX_GRANTS {
        return Err(format!("A team has at most {MAX_GRANTS} folder rights."));
    }
    Ok(merged.into_iter().map(|(folder, level)| Grant { folder, level }).collect())
}

/// Erreur de saisie ou de conflit, pour l'API.
#[derive(Debug)]
pub enum TeamError {
    Invalid(String),
    Conflict(String),
    NotFound,
    Internal(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for TeamError {
    fn from(error: E) -> Self {
        Self::Internal(error.into())
    }
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Team>> {
    let rows = sqlx::query(
        "SELECT id, name, description, idp_groups, created_at FROM teams
         ORDER BY name COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await
    .context("liste des équipes")?;
    let mut teams = Vec::with_capacity(rows.len());
    for row in rows {
        let id: i64 = row.try_get("id")?;
        let groups: String = row.try_get("idp_groups")?;
        teams.push(Team {
            id,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            idp_groups: serde_json::from_str(&groups).unwrap_or_default(),
            members: members(pool, id).await?,
            grants: grants(pool, id).await?,
            created_at: row.try_get("created_at")?,
        });
    }
    Ok(teams)
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<Team>> {
    Ok(list(pool).await?.into_iter().find(|team| team.id == id))
}

async fn members(pool: &SqlitePool, team_id: i64) -> Result<Vec<Member>> {
    let rows = sqlx::query(
        "SELECT u.id AS id, u.username AS username, u.display_name AS display_name,
                u.role AS role, m.source AS source
         FROM team_members m JOIN users u ON u.id = m.user_id
         WHERE m.team_id = ? ORDER BY u.username COLLATE NOCASE",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await
    .context("membres d'une équipe")?;
    rows.into_iter()
        .map(|row| {
            let role: String = row.try_get("role")?;
            Ok(Member {
                user_id: row.try_get("id")?,
                username: row.try_get("username")?,
                display_name: row.try_get("display_name")?,
                role: Role::parse(&role).unwrap_or(Role::Viewer),
                source: row.try_get("source")?,
            })
        })
        .collect()
}

async fn grants(pool: &SqlitePool, team_id: i64) -> Result<Vec<Grant>> {
    let rows = sqlx::query(
        "SELECT folder, level FROM team_folder_grants WHERE team_id = ? ORDER BY folder",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await
    .context("droits d'une équipe")?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let folder: String = row.try_get("folder").ok()?;
            let level: String = row.try_get("level").ok()?;
            Some(Grant { folder, level: Level::parse(&level)? })
        })
        .collect())
}

/// Crée une équipe. Rend son identifiant.
pub async fn create(pool: &SqlitePool, input: &TeamInput) -> Result<i64, TeamError> {
    let name =
        validate_name(input.name.as_deref().unwrap_or_default()).map_err(TeamError::Invalid)?;
    let mut tx = pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO teams (name) VALUES (?) ON CONFLICT(name) DO NOTHING RETURNING id",
    )
    .bind(&name)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = inserted else {
        return Err(TeamError::Conflict(format!("A team named \"{name}\" already exists.")));
    };
    let id: i64 = row.try_get("id")?;
    apply(&mut tx, id, input).await?;
    tx.commit().await?;
    Ok(id)
}

/// Modifie une équipe.
pub async fn update(pool: &SqlitePool, id: i64, input: &TeamInput) -> Result<(), TeamError> {
    let mut tx = pool.begin().await?;
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM teams WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    if exists.is_none() {
        return Err(TeamError::NotFound);
    }
    if let Some(name) = &input.name {
        let name = validate_name(name).map_err(TeamError::Invalid)?;
        let taken: Option<i64> =
            sqlx::query_scalar("SELECT id FROM teams WHERE name = ? COLLATE NOCASE AND id <> ?")
                .bind(&name)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        if taken.is_some() {
            return Err(TeamError::Conflict(format!("A team named \"{name}\" already exists.")));
        }
        sqlx::query("UPDATE teams SET name = ? WHERE id = ?")
            .bind(&name)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    apply(&mut tx, id, input).await?;
    tx.commit().await?;
    Ok(())
}

async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: i64,
    input: &TeamInput,
) -> Result<(), TeamError> {
    if let Some(description) = &input.description {
        let description = description.trim();
        if description.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(TeamError::Invalid(format!(
                "The description must be {MAX_DESCRIPTION_CHARS} characters or fewer."
            )));
        }
        sqlx::query("UPDATE teams SET description = ? WHERE id = ?")
            .bind(description)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    if let Some(groups) = &input.idp_groups {
        let groups = clean_groups(groups).map_err(TeamError::Invalid)?;
        sqlx::query("UPDATE teams SET idp_groups = ? WHERE id = ?")
            .bind(serde_json::to_string(&groups)?)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    if let Some(grants) = &input.grants {
        let grants = clean_grants(grants).map_err(TeamError::Invalid)?;
        sqlx::query("DELETE FROM team_folder_grants WHERE team_id = ?")
            .bind(id)
            .execute(&mut **tx)
            .await?;
        for grant in grants {
            sqlx::query("INSERT INTO team_folder_grants (team_id, folder, level) VALUES (?, ?, ?)")
                .bind(id)
                .bind(&grant.folder)
                .bind(grant.level.as_str())
                .execute(&mut **tx)
                .await?;
        }
    }
    if let Some(wanted) = &input.members {
        let wanted: HashSet<i64> = wanted.iter().copied().collect();
        for user_id in &wanted {
            let known: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_optional(&mut **tx)
                .await?;
            if known.is_none() {
                return Err(TeamError::Invalid(format!("Account {user_id} does not exist.")));
            }
        }
        // Les inscriptions manuelles suivent la liste ; celles d'un
        // fournisseur ou de SCIM restent (elles se resynchronisent seules).
        let current: Vec<(i64, String)> =
            sqlx::query_as("SELECT user_id, source FROM team_members WHERE team_id = ?")
                .bind(id)
                .fetch_all(&mut **tx)
                .await?;
        for (user_id, source) in &current {
            if source == "manual" && !wanted.contains(user_id) {
                sqlx::query("DELETE FROM team_members WHERE team_id = ? AND user_id = ?")
                    .bind(id)
                    .bind(user_id)
                    .execute(&mut **tx)
                    .await?;
            }
        }
        for user_id in wanted {
            // Une inscription manuelle « fige » le membre : la synchronisation
            // ne le retirera plus.
            sqlx::query(
                "INSERT INTO team_members (team_id, user_id, source) VALUES (?, ?, 'manual')
                 ON CONFLICT(team_id, user_id) DO UPDATE SET source = 'manual'",
            )
            .bind(id)
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<bool> {
    let done = sqlx::query("DELETE FROM teams WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("suppression d'une équipe")?;
    Ok(done.rows_affected() > 0)
}

/// Les dossiers existants, pour la matrice de l'interface.
pub async fn folders(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT DISTINCT group_name FROM targets ORDER BY group_name")
            .fetch_all(pool)
            .await
            .context("liste des dossiers")?;
    let mut out: Vec<String> = Vec::new();
    for folder in rows {
        let folder = normalize_folder(&folder);
        // Les dossiers parents d'un sous-dossier sont proposés eux aussi.
        let parts: Vec<&str> = folder.split('/').collect();
        for n in 1..=parts.len() {
            let prefix = parts[..n].join("/");
            if !prefix.is_empty() && !out.contains(&prefix) {
                out.push(prefix);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Vrai si le groupe annoncé par le fournisseur `actual` correspond au groupe
/// configuré `wanted` : même nom sans casse, ou DN dont le premier RDN vaut
/// `wanted` (`cn=NOC,ou=Groups,…` ↔ `NOC`).
pub fn group_matches(wanted: &str, actual: &str) -> bool {
    let wanted = wanted.trim();
    let actual = actual.trim();
    if wanted.is_empty() {
        return false;
    }
    wanted.eq_ignore_ascii_case(actual)
        || crate::auth::ldap::roles::short_name(&actual.to_lowercase())
            .is_some_and(|name| name.eq_ignore_ascii_case(wanted))
}

/// Synchronise les équipes d'un compte avec les groupes annoncés par son
/// fournisseur d'identité, à chaque connexion OIDC, LDAP ou SAML.
///
/// Seules les équipes qui listent des groupes sont concernées. Un compte y entre
/// (source `idp`) s'il est dans l'un de leurs groupes, et en sort sinon — sauf
/// s'il y a été inscrit à la main ou par SCIM.
pub async fn sync_idp_groups(pool: &SqlitePool, user_id: i64, groups: &[String]) -> Result<()> {
    let rows = sqlx::query("SELECT id, idp_groups FROM teams WHERE idp_groups <> '[]'")
        .fetch_all(pool)
        .await
        .context("équipes associées à des groupes")?;
    let mut tx = pool.begin().await?;
    for row in rows {
        let team_id: i64 = row.try_get("id")?;
        let raw: String = row.try_get("idp_groups")?;
        let wanted: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        if wanted.is_empty() {
            continue;
        }
        let member = groups.iter().any(|g| wanted.iter().any(|w| group_matches(w, g)));
        if member {
            sqlx::query(
                "INSERT INTO team_members (team_id, user_id, source) VALUES (?, ?, 'idp')
                 ON CONFLICT(team_id, user_id) DO NOTHING",
            )
            .bind(team_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "DELETE FROM team_members WHERE team_id = ? AND user_id = ? AND source = 'idp'",
            )
            .bind(team_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await.context("synchronisation des équipes")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(ceiling: Level, grants: &[(&str, Level)]) -> Scope {
        Scope {
            ceiling,
            grants: grants
                .iter()
                .map(|(folder, level)| Grant { folder: folder.to_string(), level: *level })
                .collect(),
            teams: vec!["A".into()],
            clients: None,
            admin_view: false,
        }
    }

    #[test]
    fn a_folder_covers_itself_and_its_subfolders_only() {
        assert!(folder_covers("Paris", "Paris"));
        assert!(folder_covers("Paris", "Paris/Rack A"));
        assert!(folder_covers("Paris", " Paris / Rack A "));
        assert!(!folder_covers("Paris", "Paris-Nord"));
        assert!(!folder_covers("Paris", "Lyon/Paris"));
        assert!(!folder_covers("Paris", ""));
        assert!(folder_covers("", ""));
        assert!(!folder_covers("", "Paris"));
        assert!(folder_covers("*", ""));
        assert!(folder_covers("*", "Lyon"));
    }

    #[test]
    fn the_role_caps_the_team_level() {
        let viewer = scope(Level::Read, &[("Paris", Level::Manage)]);
        assert_eq!(viewer.level_for("Paris/Rack"), Some(Level::Read));
        let operator = scope(Level::Manage, &[("Paris", Level::Operate), ("*", Level::Read)]);
        assert_eq!(operator.level_for("Paris"), Some(Level::Operate));
        assert_eq!(operator.level_for("Lyon"), Some(Level::Read));
        let none = scope(Level::Manage, &[("Paris", Level::Manage)]);
        assert_eq!(none.level_for("Lyon"), None);
        assert_eq!(none.level_for(""), None);
    }

    #[test]
    fn folders_are_normalised() {
        assert_eq!(normalize_folder(" Paris//Rack A/ "), "Paris/Rack A");
        assert_eq!(normalize_folder("*"), "*");
        assert_eq!(normalize_folder("  "), "");
    }

    #[test]
    fn target_paths_are_recognised() {
        assert_eq!(target_in_path("/api/targets/12"), Some(12));
        assert_eq!(target_in_path("/targets/12/pbs/jobs"), Some(12));
        assert_eq!(target_in_path("/targets/import"), None);
        assert_eq!(target_in_path("/targets"), None);
        assert_eq!(target_in_path("/targets/12abc"), None);
    }

    #[test]
    fn manage_routes_are_a_closed_list() {
        assert!(is_manage_route(&Method::PUT, "/api/targets/3"));
        assert!(is_manage_route(&Method::DELETE, "/targets/3"));
        assert!(is_manage_route(&Method::POST, "/targets/3/probe"));
        assert!(is_manage_route(&Method::POST, "/targets"));
        assert!(!is_manage_route(&Method::POST, "/targets/3/commands"));
        assert!(!is_manage_route(&Method::POST, "/targets/import"));
        assert!(!is_manage_route(&Method::PUT, "/targets/3/agent/modules/x"));
    }

    #[test]
    fn patterns_match_segments() {
        assert!(path_matches("/auth/**", "/auth/me"));
        assert!(path_matches("/alerts/*/ack", "/alerts/abc/ack"));
        assert!(!path_matches("/alerts/*/ack", "/alerts//ack"));
        assert!(!path_matches("/alerts", "/alerts/rules"));
        assert!(!path_matches("/auth/**", "/authors"));
    }

    #[test]
    fn the_series_filter_lists_the_visible_targets() {
        assert_eq!(series_filter(None), None);
        assert_eq!(series_filter(Some(&HashSet::new())).unwrap(), "{target=\"-1\"}");
        let ids: HashSet<i64> = [3, 1].into_iter().collect();
        assert_eq!(series_filter(Some(&ids)).unwrap(), "{target=~\"1|3\"}");
    }

    #[test]
    fn groups_match_by_name_or_first_rdn() {
        assert!(group_matches("NOC", "noc"));
        assert!(group_matches("noc", "cn=NOC,ou=Groups,dc=example,dc=org"));
        assert!(group_matches(
            "cn=noc,ou=groups,dc=example,dc=org",
            "CN=NOC,OU=Groups,DC=example,DC=org"
        ));
        assert!(!group_matches("noc", "cn=noc-admins,ou=Groups"));
        assert!(!group_matches("", "noc"));
    }
}
