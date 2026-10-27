//! Clients (mode multi-clients, « MSP »).
//!
//! Un client (locataire) se place au-dessus des équipes et des dossiers. Chaque
//! équipement, page de statut et jeton d'agent appartient à un client ; les
//! canaux de notification, règles d'alerte, silences et calendriers de rapport
//! appartiennent à un client ou à l'instance (`client_id` NULL : objet partagé
//! par l'administrateur MSP).
//!
//! La règle de visibilité, quand le mode est activé (réglage [`SETTINGS_KEY`]) :
//!
//! - un administrateur est l'administrateur MSP : il voit et gère tout. Le
//!   sélecteur de l'en-tête ([`VIEW_HEADER`]) limite son affichage à un client,
//!   sans rien lui interdire ([`Scope::admin_view`]) ;
//! - tout autre compte ne voit que les clients dont il est membre, et y reste
//!   soumis à ses équipes. Il devient un compte restreint au sens des équipes :
//!   le garde à liste fermée ([`crate::auth::teams::guard`]) lui refuse toute
//!   route non vérifiée, et chaque liste ouverte filtre par
//!   [`Access::visible_targets`], qui applique le client avant le dossier ;
//! - un compte membre d'aucun client ne voit rien.
//!
//! Mode coupé, rien de tout cela ne s'applique : une instance mono-client se
//! comporte exactement comme avant.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{Row, SqlitePool};

use crate::auth::teams::{ALL_FOLDERS, Access, Grant, Level, Member, Scope, group_matches};
use crate::auth::users::{Role, User};

/// Le client qui reçoit tout l'existant à la migration.
pub const DEFAULT_CLIENT: i64 = 1;
/// Réglage `{ "enabled": bool }` dans la table `settings`. Le déclencheur
/// `users_default_client` (migration 0185) lit la même clé.
pub const SETTINGS_KEY: &str = "msp";
/// En-tête posé par l'interface quand un administrateur choisit un client.
pub const VIEW_HEADER: &str = "x-dumbmonit-client";

const MAX_NAME_CHARS: usize = 80;
const MAX_DESCRIPTION_CHARS: usize = 500;
const MAX_GROUP_CHARS: usize = 512;
const MAX_GROUPS: usize = 64;
/// Contacts d'un client : au-delà, c'est une liste de diffusion.
pub const MAX_CONTACTS: usize = 20;
/// Taille maximale d'un logo de client (comme celui d'une page de statut).
pub const MAX_LOGO_BYTES: usize = 256 * 1024;

// ---------------------------------------------------------------------------
// Réglage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct MspSettings {
    #[serde(default)]
    pub enabled: bool,
}

pub async fn settings(pool: &SqlitePool) -> Result<MspSettings> {
    Ok(crate::auth::settings::get(pool, SETTINGS_KEY).await?.unwrap_or_default())
}

pub async fn save_settings(pool: &SqlitePool, value: &MspSettings) -> Result<()> {
    crate::auth::settings::set(pool, SETTINGS_KEY, value).await
}

/// Vrai si le mode MSP est activé.
pub async fn enabled(pool: &SqlitePool) -> Result<bool> {
    Ok(settings(pool).await?.enabled)
}

// ---------------------------------------------------------------------------
// Droits
// ---------------------------------------------------------------------------

/// Les clients dont le compte est membre.
pub async fn memberships(pool: &SqlitePool, user_id: i64) -> Result<Vec<i64>> {
    sqlx::query_scalar("SELECT client_id FROM client_members WHERE user_id = ? ORDER BY client_id")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("clients d'un compte")
}

/// Ajoute le niveau client aux droits d'équipe d'un compte.
///
/// Un administrateur, ou n'importe qui mode coupé, garde ses droits d'équipe.
/// Sinon le compte devient restreint : sans équipe, il voit tous les dossiers de
/// ses clients au niveau « traiter les alertes » borné par son rôle (ce qu'un
/// compte sans équipe pouvait faire avant) ; avec des équipes, leurs dossiers,
/// dans ses clients seulement.
pub async fn restrict(pool: &SqlitePool, user: &User, teams: Access) -> Result<Access> {
    if user.role.is_admin() || !enabled(pool).await? {
        return Ok(teams);
    }
    let clients = memberships(pool, user.id).await?;
    let scope = match teams {
        Access::All => Scope {
            ceiling: Level::ceiling(user.role),
            grants: vec![Grant { folder: ALL_FOLDERS.to_string(), level: Level::Operate }],
            teams: Vec::new(),
            clients: Some(clients),
            admin_view: false,
        },
        Access::Scoped(scope) => Scope { clients: Some(clients), ..(*scope).clone() },
    };
    Ok(Access::Scoped(Arc::new(scope)))
}

/// Limite l'affichage au client choisi dans l'en-tête.
///
/// - administrateur (droits complets) : vue filtrée, sans barrière ;
/// - compte restreint membre de ce client : ses droits, sur ce client seul ;
/// - sinon (client inconnu ou étranger) : rien ne change — l'en-tête ne donne
///   jamais plus, et ne sert pas à sonder l'existence d'un client.
pub fn narrow(access: Access, client: i64, admin: bool) -> Access {
    match access {
        Access::All if admin => Access::Scoped(Arc::new(Scope {
            ceiling: Level::Manage,
            grants: vec![Grant { folder: ALL_FOLDERS.to_string(), level: Level::Manage }],
            teams: Vec::new(),
            clients: Some(vec![client]),
            admin_view: true,
        })),
        Access::Scoped(scope) if scope.clients.as_ref().is_some_and(|c| c.contains(&client)) => {
            Access::Scoped(Arc::new(Scope { clients: Some(vec![client]), ..(*scope).clone() }))
        }
        other => other,
    }
}

/// Applique l'en-tête [`VIEW_HEADER`] d'une requête, s'il y en a un et que le
/// mode MSP est activé.
pub async fn apply_view(
    pool: &SqlitePool,
    headers: &axum::http::HeaderMap,
    access: Access,
    admin: bool,
) -> Result<Access> {
    let Some(client) = headers
        .get(VIEW_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<i64>().ok())
    else {
        return Ok(access);
    };
    if !enabled(pool).await? || !exists(pool, client).await? {
        return Ok(access);
    }
    Ok(narrow(access, client, admin))
}

/// Le client dans lequel tombe un objet créé avec ces droits, quand la requête
/// n'en désigne pas : celui de la vue d'un administrateur, le seul client d'un
/// compte restreint. `None` : à l'appelant de choisir (instance, client par
/// défaut, ou refus).
pub fn creation_client(access: &Access) -> Option<i64> {
    match access.clients() {
        Some([only]) => Some(*only),
        _ => None,
    }
}

/// Le client d'un nouvel équipement : celui demandé s'il est permis, sinon
/// celui des droits ([`creation_client`]), sinon le client par défaut pour un
/// compte sans restriction. Un compte membre de plusieurs clients doit choisir.
pub async fn target_client_for(
    pool: &SqlitePool,
    access: &Access,
    wanted: Option<i64>,
) -> Result<i64, ClientError> {
    let client = match wanted {
        Some(client) => client,
        None => match creation_client(access) {
            Some(client) => client,
            None if access.clients().is_none() => DEFAULT_CLIENT,
            None => return Err(ClientError::Invalid("Choose the client of this target.".into())),
        },
    };
    if !access.sees_client(client) || !exists(pool, client).await? {
        return Err(ClientError::Invalid("This client does not exist.".into()));
    }
    Ok(client)
}

// ---------------------------------------------------------------------------
// Appartenance des objets
// ---------------------------------------------------------------------------

/// Les objets qui appartiennent à un client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Target,
    StatusPage,
    AgentToken,
    Channel,
    Rule,
    Silence,
    Report,
    /// Tableau de bord partagé (un tableau personnel reste à son propriétaire).
    Dashboard,
    /// Planning d'astreinte.
    Schedule,
    /// Politique d'escalade d'astreinte.
    EscalationPolicy,
    /// Procédure (runbook).
    Runbook,
    /// Action de remédiation.
    RemediationAction,
}

impl Kind {
    pub const ALL: [Kind; 12] = [
        Kind::Target,
        Kind::StatusPage,
        Kind::AgentToken,
        Kind::Channel,
        Kind::Rule,
        Kind::Silence,
        Kind::Report,
        Kind::Dashboard,
        Kind::Schedule,
        Kind::EscalationPolicy,
        Kind::Runbook,
        Kind::RemediationAction,
    ];

    /// Table de l'objet. Valeurs fixes : jamais d'entrée utilisateur dans le SQL.
    fn table(self) -> &'static str {
        match self {
            Self::Target => "targets",
            Self::StatusPage => "status_pages",
            Self::AgentToken => "agent_tokens",
            Self::Channel => "notification_channels",
            Self::Rule => "alert_rules",
            Self::Silence => "silences",
            Self::Report => "report_schedules",
            Self::Dashboard => "dashboards",
            Self::Schedule => "oncall_schedules",
            Self::EscalationPolicy => "oncall_policies",
            Self::Runbook => "runbooks",
            Self::RemediationAction => "remediation_actions",
        }
    }

    /// Clé du JSON de `GET /api/clients/assignments`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Target => "targets",
            Self::StatusPage => "status_pages",
            Self::AgentToken => "agent_tokens",
            Self::Channel => "channels",
            Self::Rule => "rules",
            Self::Silence => "silences",
            Self::Report => "reports",
            Self::Dashboard => "dashboards",
            Self::Schedule => "oncall_schedules",
            Self::EscalationPolicy => "oncall_policies",
            Self::Runbook => "runbooks",
            Self::RemediationAction => "remediation_actions",
        }
    }

    /// Vrai si l'objet peut appartenir à l'instance (NULL).
    pub fn may_be_shared(self) -> bool {
        !matches!(self, Self::Target | Self::AgentToken)
    }
}

/// Le client de chaque objet d'une sorte ; absent de la table : à l'instance.
pub async fn owners(pool: &SqlitePool, kind: Kind) -> Result<HashMap<i64, i64>> {
    let sql = format!("SELECT id, client_id FROM {} WHERE client_id IS NOT NULL", kind.table());
    let rows: Vec<(i64, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .fetch_all(pool)
        .await
        .context("clients des objets")?;
    Ok(rows.into_iter().collect())
}

/// Le client d'un objet : `None` s'il n'existe pas, `Some(None)` s'il est à
/// l'instance.
pub async fn owner_of(pool: &SqlitePool, kind: Kind, id: i64) -> Result<Option<Option<i64>>> {
    let sql = format!("SELECT client_id FROM {} WHERE id = ?", kind.table());
    sqlx::query_scalar::<_, Option<i64>>(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("client d'un objet")
}

/// Le client d'un agent d'enrôlement : NULL (jeton antérieur) vaut le client
/// par défaut.
pub async fn token_client(pool: &SqlitePool, token_id: i64) -> Result<i64> {
    let client: Option<Option<i64>> =
        sqlx::query_scalar("SELECT client_id FROM agent_tokens WHERE id = ?")
            .bind(token_id)
            .fetch_optional(pool)
            .await
            .context("client d'un jeton d'agent")?;
    Ok(client.flatten().unwrap_or(DEFAULT_CLIENT))
}

/// Le client d'un équipement, `None` s'il n'existe pas.
pub async fn target_client(pool: &SqlitePool, target_id: i64) -> Result<Option<i64>> {
    sqlx::query_scalar("SELECT client_id FROM targets WHERE id = ?")
        .bind(target_id)
        .fetch_optional(pool)
        .await
        .context("client d'un équipement")
}

/// Le client de chaque équipement.
pub async fn target_clients(pool: &SqlitePool) -> Result<HashMap<i64, i64>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as("SELECT id, client_id FROM targets")
        .fetch_all(pool)
        .await
        .context("clients des équipements")?;
    Ok(rows.into_iter().collect())
}

/// Les équipements d'un client.
pub async fn targets_of(pool: &SqlitePool, client: i64) -> Result<HashSet<i64>> {
    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM targets WHERE client_id = ?")
        .bind(client)
        .fetch_all(pool)
        .await
        .context("équipements d'un client")?;
    Ok(ids.into_iter().collect())
}

/// Vrai si l'objet (client `owner`, `None` = instance) se montre avec ces
/// droits : sans restriction de client, tout ; sinon les objets de ses clients
/// et ceux de l'instance si `with_shared`.
pub fn visible_owner(access: &Access, owner: Option<i64>, with_shared: bool) -> bool {
    match (access.clients(), owner) {
        (None, _) => true,
        (Some(_), None) => with_shared,
        (Some(clients), Some(owner)) => clients.contains(&owner),
    }
}

/// Garde dans `items` ce que ces droits laissent voir (voir [`visible_owner`]).
pub async fn retain_visible<T>(
    pool: &SqlitePool,
    access: &Access,
    kind: Kind,
    items: &mut Vec<T>,
    id_of: impl Fn(&T) -> i64,
    with_shared: bool,
) -> Result<()> {
    if access.clients().is_none() {
        return Ok(());
    }
    let owners = owners(pool, kind).await?;
    items.retain(|item| visible_owner(access, owners.get(&id_of(item)).copied(), with_shared));
    Ok(())
}

/// Range un objet qui vient d'être créé dans le client de ces droits
/// ([`creation_client`]), s'il y en a un. Sans effet sinon : l'objet garde la
/// valeur par défaut de sa table.
pub async fn adopt(pool: &SqlitePool, access: &Access, kind: Kind, id: i64) -> Result<()> {
    let Some(client) = creation_client(access) else { return Ok(()) };
    let sql = format!("UPDATE {} SET client_id = ? WHERE id = ?", kind.table());
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(client)
        .bind(id)
        .execute(pool)
        .await
        .context("client d'un objet")?;
    Ok(())
}

/// Erreur de saisie ou de conflit, pour l'API.
#[derive(Debug)]
pub enum ClientError {
    Invalid(String),
    Conflict(String),
    NotFound,
    Internal(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for ClientError {
    fn from(error: E) -> Self {
        Self::Internal(error.into())
    }
}

/// Rattache des objets à un client (`None` : à l'instance), après les
/// vérifications propres à chaque sorte. Réservé à l'administrateur MSP.
pub async fn assign(
    pool: &SqlitePool,
    kind: Kind,
    ids: &[i64],
    client: Option<i64>,
) -> Result<(), ClientError> {
    if ids.is_empty() {
        return Ok(());
    }
    if ids.len() > 1000 {
        return Err(ClientError::Invalid("Assign at most 1000 objects at once.".into()));
    }
    if let Some(client) = client
        && !exists(pool, client).await?
    {
        return Err(ClientError::Invalid("This client does not exist.".into()));
    }
    if client.is_none() && !kind.may_be_shared() {
        return Err(ClientError::Invalid("This object must belong to a client.".into()));
    }
    for id in ids {
        if owner_of(pool, kind, *id).await?.is_none() {
            return Err(ClientError::NotFound);
        }
    }
    match kind {
        Kind::Target => {
            let Some(client) = client else { unreachable!("vérifié plus haut") };
            return move_targets(pool, ids, client).await;
        }
        Kind::StatusPage => {
            // Une page de client ne montre que des équipements de ce client.
            if let Some(client) = client {
                for id in ids {
                    let foreign: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM status_page_items i
                           JOIN targets t ON t.id = i.target_id
                         WHERE i.page_id = ? AND t.client_id <> ?",
                    )
                    .bind(id)
                    .bind(client)
                    .fetch_one(pool)
                    .await?;
                    if foreign > 0 {
                        return Err(ClientError::Conflict(
                            "This status page shows targets of another client: remove them first."
                                .into(),
                        ));
                    }
                }
            }
        }
        Kind::Silence => {
            // Un silence posé sur un équipement suit le client de l'équipement.
            for id in ids {
                let target: Option<i64> =
                    sqlx::query_scalar("SELECT target_id FROM silences WHERE id = ?")
                        .bind(id)
                        .fetch_one(pool)
                        .await?;
                if let Some(target) = target
                    && target_client(pool, target).await? != client
                {
                    return Err(ClientError::Conflict(
                        "A maintenance window on a target belongs to the client of that target."
                            .into(),
                    ));
                }
            }
        }
        Kind::Dashboard => {
            for id in ids {
                let shared: bool =
                    sqlx::query_scalar("SELECT shared = 1 FROM dashboards WHERE id = ?")
                        .bind(id)
                        .fetch_one(pool)
                        .await?;
                if !shared {
                    return Err(ClientError::Conflict(
                        "A personal dashboard stays with its owner: share it first.".into(),
                    ));
                }
            }
        }
        Kind::RemediationAction => {
            // Une action qui vise un équipement précis suit le client de cet équipement.
            for id in ids {
                let target: Option<i64> =
                    sqlx::query_scalar("SELECT target_id FROM remediation_actions WHERE id = ?")
                        .bind(id)
                        .fetch_one(pool)
                        .await?;
                if let Some(target) = target
                    && target_client(pool, target).await? != client
                {
                    return Err(ClientError::Conflict(
                        "An action on a target belongs to the client of that target.".into(),
                    ));
                }
            }
        }
        Kind::AgentToken
        | Kind::Channel
        | Kind::Rule
        | Kind::Report
        | Kind::Schedule
        | Kind::EscalationPolicy
        | Kind::Runbook => {}
    }
    let mut tx = pool.begin().await?;
    let sql = format!("UPDATE {} SET client_id = ? WHERE id = ?", kind.table());
    for id in ids {
        sqlx::query(sqlx::AssertSqlSafe(sql.clone()))
            .bind(client)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Déplace des équipements vers un client, avec tout ce qui en dépend
/// (enfants, équipements relayés, moniteurs gérés par étiquettes) : un lien
/// entre deux clients ferait passer des données — ou des identifiants, pour un
/// relais — de l'un à l'autre.
///
/// Les liens qui franchiraient encore la frontière après le déplacement sont
/// coupés : parent, relais, gestion par étiquettes, emplacements de sonde
/// multisite et composants de pages de statut d'un autre client.
pub async fn move_targets(pool: &SqlitePool, ids: &[i64], client: i64) -> Result<(), ClientError> {
    let mut tx = pool.begin().await?;
    let mut moved: HashSet<i64> = HashSet::new();
    for id in ids {
        let rows: Vec<i64> = sqlx::query_scalar(
            "WITH RECURSIVE moved(id) AS (
                 SELECT ?
                 UNION
                 SELECT t.id FROM targets t JOIN moved m
                   ON t.parent_id = m.id OR t.via_agent = m.id OR t.managed_source = m.id
             )
             SELECT id FROM moved",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        moved.extend(rows);
    }
    let arriving: i64 = {
        let mut count = 0;
        for id in &moved {
            let current: Option<i64> =
                sqlx::query_scalar("SELECT client_id FROM targets WHERE id = ?")
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if current.is_some_and(|c| c != client) {
                count += 1;
            }
        }
        count
    };
    let quota: Option<i64> =
        sqlx::query_scalar::<_, Option<i64>>("SELECT monitor_quota FROM clients WHERE id = ?")
            .bind(client)
            .fetch_one(&mut *tx)
            .await?;
    if let Some(quota) = quota {
        let have: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM targets WHERE client_id = ?")
            .bind(client)
            .fetch_one(&mut *tx)
            .await?;
        if have + arriving > quota {
            return Err(ClientError::Conflict(format!(
                "This client is limited to {quota} targets: moving {arriving} more would exceed it."
            )));
        }
    }
    for id in &moved {
        sqlx::query("UPDATE targets SET client_id = ? WHERE id = ?")
            .bind(client)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    sever_cross_client_links(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Coupe tout lien entre équipements (ou entre page et équipement) de clients
/// différents.
pub(crate) async fn sever_cross_client_links(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<()> {
    for sql in [
        "UPDATE targets SET parent_id = NULL
         WHERE parent_id IS NOT NULL
           AND client_id <> (SELECT p.client_id FROM targets p WHERE p.id = targets.parent_id)",
        "UPDATE targets SET via_agent = NULL
         WHERE via_agent IS NOT NULL
           AND client_id <> (SELECT p.client_id FROM targets p WHERE p.id = targets.via_agent)",
        "UPDATE targets SET managed_by = NULL, managed_source = NULL, managed_key = NULL,
                            managed_missing_since = NULL
         WHERE managed_source IS NOT NULL
           AND client_id <> (SELECT p.client_id FROM targets p
                             WHERE p.id = targets.managed_source)",
        "DELETE FROM target_multisite_agents
         WHERE (SELECT client_id FROM targets WHERE id = target_multisite_agents.target_id)
            <> (SELECT client_id FROM targets WHERE id = target_multisite_agents.agent_id)",
        "DELETE FROM status_page_items
         WHERE EXISTS (SELECT 1 FROM status_pages p JOIN targets t
                         ON t.id = status_page_items.target_id
                       WHERE p.id = status_page_items.page_id
                         AND p.client_id IS NOT NULL AND p.client_id <> t.client_id)",
    ] {
        sqlx::query(sql).execute(&mut **tx).await.context("liens entre clients")?;
    }
    Ok(())
}

/// Vrai si le client existe.
pub async fn exists(pool: &SqlitePool, client: i64) -> Result<bool> {
    let found: Option<i64> = sqlx::query_scalar("SELECT id FROM clients WHERE id = ?")
        .bind(client)
        .fetch_optional(pool)
        .await
        .context("existence d'un client")?;
    Ok(found.is_some())
}

// ---------------------------------------------------------------------------
// Données
// ---------------------------------------------------------------------------

/// Un client, tel que l'expose l'API d'administration.
#[derive(Debug, Clone, Serialize)]
pub struct Client {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub contacts: Vec<String>,
    pub monitor_quota: Option<i64>,
    pub idp_groups: Vec<String>,
    pub members: Vec<Member>,
    /// Nombre d'équipements.
    pub targets: i64,
    pub has_logo: bool,
    pub sla_report: bool,
    pub sla_objective: Option<f64>,
    pub report_timezone: String,
    pub report_last_period: Option<String>,
    pub report_last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Référence courte : le sélecteur de l'en-tête, `GET /api/auth/me`.
#[derive(Debug, Clone, Serialize)]
pub struct ClientRef {
    pub id: i64,
    pub name: String,
    pub has_logo: bool,
}

fn double_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Champs modifiables d'un client. `None` : inchangé (création : défaut).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ClientInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub contacts: Option<Vec<String>>,
    /// Absent : inchangé ; `null` : sans limite.
    #[serde(default, deserialize_with = "double_option")]
    pub monitor_quota: Option<Option<i64>>,
    pub idp_groups: Option<Vec<String>>,
    /// Comptes inscrits à la main ; ceux venus d'un fournisseur sont conservés.
    pub members: Option<Vec<i64>>,
    pub sla_report: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    pub sla_objective: Option<Option<f64>>,
    pub report_timezone: Option<String>,
}

/// Valide un nom de client.
pub fn validate_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a client name.".into());
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!("The client name must be {MAX_NAME_CHARS} characters or fewer."));
    }
    if name.chars().any(char::is_control) {
        return Err("The client name cannot contain control characters.".into());
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
        return Err(format!("A client maps at most {MAX_GROUPS} identity-provider groups."));
    }
    Ok(out)
}

fn clean_contacts(contacts: &[String]) -> Result<Vec<String>, String> {
    let cleaned: Vec<String> =
        contacts.iter().map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect();
    let out = crate::reports::normalise_recipients(&cleaned)?;
    if out.len() > MAX_CONTACTS {
        return Err(format!("A client has at most {MAX_CONTACTS} contacts."));
    }
    Ok(out)
}

fn row_to_client(row: &sqlx::sqlite::SqliteRow) -> Result<Client> {
    let contacts: String = row.try_get("contacts")?;
    let groups: String = row.try_get("idp_groups")?;
    Ok(Client {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        contacts: serde_json::from_str(&contacts).unwrap_or_default(),
        monitor_quota: row.try_get("monitor_quota")?,
        idp_groups: serde_json::from_str(&groups).unwrap_or_default(),
        members: Vec::new(),
        targets: row.try_get("targets")?,
        has_logo: row.try_get::<Option<String>, _>("logo_type")?.is_some(),
        sla_report: row.try_get::<i64, _>("sla_report")? != 0,
        sla_objective: row.try_get("sla_objective")?,
        report_timezone: row.try_get("report_timezone")?,
        report_last_period: row.try_get("report_last_period")?,
        report_last_error: row.try_get("report_last_error")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

const SELECT_CLIENTS: &str = "SELECT c.id, c.name, c.description, c.contacts, c.monitor_quota,
        c.idp_groups, c.logo_type, c.sla_report, c.sla_objective, c.report_timezone,
        c.report_last_period, c.report_last_error, c.created_at, c.updated_at,
        (SELECT COUNT(*) FROM targets t WHERE t.client_id = c.id) AS targets
     FROM clients c";

pub async fn list(pool: &SqlitePool) -> Result<Vec<Client>> {
    let sql = format!("{SELECT_CLIENTS} ORDER BY c.name COLLATE NOCASE");
    let rows =
        sqlx::query(sqlx::AssertSqlSafe(sql)).fetch_all(pool).await.context("liste des clients")?;
    let mut clients = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut client = row_to_client(row)?;
        client.members = members(pool, client.id).await?;
        clients.push(client);
    }
    Ok(clients)
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<Client>> {
    let sql = format!("{SELECT_CLIENTS} WHERE c.id = ?");
    let Some(row) = sqlx::query(sqlx::AssertSqlSafe(sql)).bind(id).fetch_optional(pool).await?
    else {
        return Ok(None);
    };
    let mut client = row_to_client(&row)?;
    client.members = members(pool, id).await?;
    Ok(Some(client))
}

/// Références courtes des clients, tous ou seulement `only`.
pub async fn refs(pool: &SqlitePool, only: Option<&[i64]>) -> Result<Vec<ClientRef>> {
    let rows: Vec<(i64, String, Option<String>)> =
        sqlx::query_as("SELECT id, name, logo_type FROM clients ORDER BY name COLLATE NOCASE")
            .fetch_all(pool)
            .await
            .context("liste des clients")?;
    Ok(rows
        .into_iter()
        .filter(|(id, _, _)| only.is_none_or(|only| only.contains(id)))
        .map(|(id, name, logo)| ClientRef { id, name, has_logo: logo.is_some() })
        .collect())
}

async fn members(pool: &SqlitePool, client_id: i64) -> Result<Vec<Member>> {
    let rows = sqlx::query(
        "SELECT u.id AS id, u.username AS username, u.display_name AS display_name,
                u.role AS role, m.source AS source
         FROM client_members m JOIN users u ON u.id = m.user_id
         WHERE m.client_id = ? ORDER BY u.username COLLATE NOCASE",
    )
    .bind(client_id)
    .fetch_all(pool)
    .await
    .context("membres d'un client")?;
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

/// Les comptes non administrateurs membres d'aucun client : mode activé, ils ne
/// voient rien.
pub async fn unassigned_users(pool: &SqlitePool) -> Result<Vec<i64>> {
    sqlx::query_scalar(
        "SELECT id FROM users
         WHERE role <> 'admin' AND id NOT IN (SELECT user_id FROM client_members)
         ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .context("comptes sans client")
}

/// Crée un client. Rend son identifiant.
pub async fn create(pool: &SqlitePool, input: &ClientInput) -> Result<i64, ClientError> {
    let name =
        validate_name(input.name.as_deref().unwrap_or_default()).map_err(ClientError::Invalid)?;
    let mut tx = pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO clients (name) VALUES (?) ON CONFLICT(name) DO NOTHING RETURNING id",
    )
    .bind(&name)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = inserted else {
        return Err(ClientError::Conflict(format!("A client named \"{name}\" already exists.")));
    };
    let id: i64 = row.try_get("id")?;
    apply(&mut tx, id, input).await?;
    tx.commit().await?;
    Ok(id)
}

/// Modifie un client.
pub async fn update(pool: &SqlitePool, id: i64, input: &ClientInput) -> Result<(), ClientError> {
    let mut tx = pool.begin().await?;
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM clients WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    if exists.is_none() {
        return Err(ClientError::NotFound);
    }
    if let Some(name) = &input.name {
        let name = validate_name(name).map_err(ClientError::Invalid)?;
        let taken: Option<i64> =
            sqlx::query_scalar("SELECT id FROM clients WHERE name = ? COLLATE NOCASE AND id <> ?")
                .bind(&name)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        if taken.is_some() {
            return Err(ClientError::Conflict(format!(
                "A client named \"{name}\" already exists."
            )));
        }
        sqlx::query("UPDATE clients SET name = ? WHERE id = ?")
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
    input: &ClientInput,
) -> Result<(), ClientError> {
    if let Some(description) = &input.description {
        let description = description.trim();
        if description.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(ClientError::Invalid(format!(
                "The description must be {MAX_DESCRIPTION_CHARS} characters or fewer."
            )));
        }
        sqlx::query("UPDATE clients SET description = ? WHERE id = ?")
            .bind(description)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    if let Some(contacts) = &input.contacts {
        let contacts = clean_contacts(contacts).map_err(ClientError::Invalid)?;
        sqlx::query("UPDATE clients SET contacts = ? WHERE id = ?")
            .bind(serde_json::to_string(&contacts)?)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    if let Some(quota) = input.monitor_quota {
        if quota.is_some_and(|q| !(0..=1_000_000).contains(&q)) {
            return Err(ClientError::Invalid(
                "The target quota must be between 0 and 1000000.".into(),
            ));
        }
        sqlx::query("UPDATE clients SET monitor_quota = ? WHERE id = ?")
            .bind(quota)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    if let Some(groups) = &input.idp_groups {
        let groups = clean_groups(groups).map_err(ClientError::Invalid)?;
        sqlx::query("UPDATE clients SET idp_groups = ? WHERE id = ?")
            .bind(serde_json::to_string(&groups)?)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    if let Some(enabled) = input.sla_report {
        sqlx::query("UPDATE clients SET sla_report = ? WHERE id = ?")
            .bind(i64::from(enabled))
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    if let Some(objective) = input.sla_objective {
        if objective.is_some_and(|v| !(v.is_finite() && v > 0.0 && v <= 100.0)) {
            return Err(ClientError::Invalid(
                "The availability objective must be a percentage above 0 and up to 100.".into(),
            ));
        }
        sqlx::query("UPDATE clients SET sla_objective = ? WHERE id = ?")
            .bind(objective)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    if let Some(tz) = &input.report_timezone {
        let tz = tz.trim();
        if crate::reports::schedule::parse_timezone(tz).is_none() {
            return Err(ClientError::Invalid(
                "The timezone must be an IANA name such as \"Europe/Paris\".".into(),
            ));
        }
        sqlx::query("UPDATE clients SET report_timezone = ? WHERE id = ?")
            .bind(tz)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    if let Some(wanted) = &input.members {
        let wanted: HashSet<i64> = wanted.iter().copied().collect();
        for user_id in &wanted {
            let known: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_optional(&mut **tx)
                .await?;
            if known.is_none() {
                return Err(ClientError::Invalid(format!("Account {user_id} does not exist.")));
            }
        }
        let current: Vec<(i64, String)> =
            sqlx::query_as("SELECT user_id, source FROM client_members WHERE client_id = ?")
                .bind(id)
                .fetch_all(&mut **tx)
                .await?;
        for (user_id, source) in &current {
            if source == "manual" && !wanted.contains(user_id) {
                sqlx::query("DELETE FROM client_members WHERE client_id = ? AND user_id = ?")
                    .bind(id)
                    .bind(user_id)
                    .execute(&mut **tx)
                    .await?;
            }
        }
        for user_id in wanted {
            sqlx::query(
                "INSERT INTO client_members (client_id, user_id, source) VALUES (?, ?, 'manual')
                 ON CONFLICT(client_id, user_id) DO UPDATE SET source = 'manual'",
            )
            .bind(id)
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
        }
    }
    sqlx::query(
        "UPDATE clients SET updated_at = strftime('%Y-%m-%d %H:%M:%S', 'now') WHERE id = ?",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Supprime un client vide. Le client par défaut reste : c'est là que tombe ce
/// qui n'a pas de client désigné.
pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), ClientError> {
    if id == DEFAULT_CLIENT {
        return Err(ClientError::Conflict("The default client cannot be deleted.".into()));
    }
    if !exists(pool, id).await? {
        return Err(ClientError::NotFound);
    }
    for (table, what) in [
        ("targets", "targets"),
        ("status_pages", "status pages"),
        ("agent_tokens", "agent enrollment tokens"),
    ] {
        let sql = format!("SELECT COUNT(*) FROM {table} WHERE client_id = ?");
        let count: i64 =
            sqlx::query_scalar(sqlx::AssertSqlSafe(sql)).bind(id).fetch_one(pool).await?;
        if count > 0 {
            return Err(ClientError::Conflict(format!(
                "This client still has {count} {what}: move or delete them first."
            )));
        }
    }
    sqlx::query("DELETE FROM clients WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

/// Logo d'un client : (type MIME, octets).
pub async fn logo(pool: &SqlitePool, id: i64) -> Result<Option<(String, Vec<u8>)>> {
    let row: Option<(Option<String>, Option<Vec<u8>>)> =
        sqlx::query_as("SELECT logo_type, logo FROM clients WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .context("logo d'un client")?;
    Ok(match row {
        Some((Some(kind), Some(bytes))) => Some((kind, bytes)),
        _ => None,
    })
}

pub async fn set_logo(pool: &SqlitePool, id: i64, logo: Option<(&str, &[u8])>) -> Result<bool> {
    let (kind, bytes) = match logo {
        Some((kind, bytes)) => (Some(kind), Some(bytes)),
        None => (None, None),
    };
    let done = sqlx::query(
        "UPDATE clients SET logo_type = ?, logo = ?,
                updated_at = strftime('%Y-%m-%d %H:%M:%S', 'now')
         WHERE id = ?",
    )
    .bind(kind)
    .bind(bytes)
    .bind(id)
    .execute(pool)
    .await
    .context("logo d'un client")?;
    Ok(done.rows_affected() > 0)
}

/// Le client d'une page de statut quand le mode MSP est activé : ce qui habille
/// la page publique (nom, logo). `None` mode coupé ou pour une page d'instance.
pub async fn branding_of_page(pool: &SqlitePool, page_id: i64) -> Result<Option<ClientRef>> {
    if !enabled(pool).await? {
        return Ok(None);
    }
    let row: Option<(i64, String, Option<String>)> = sqlx::query_as(
        "SELECT c.id, c.name, c.logo_type FROM status_pages p JOIN clients c ON c.id = p.client_id
         WHERE p.id = ?",
    )
    .bind(page_id)
    .fetch_optional(pool)
    .await
    .context("client d'une page de statut")?;
    Ok(row.map(|(id, name, logo)| ClientRef { id, name, has_logo: logo.is_some() }))
}

/// Synchronise les clients d'un compte avec les groupes annoncés par son
/// fournisseur d'identité, à chaque connexion OIDC, LDAP ou SAML (comme
/// [`crate::auth::teams::sync_idp_groups`]).
pub async fn sync_idp_groups(pool: &SqlitePool, user_id: i64, groups: &[String]) -> Result<()> {
    sync_groups(pool, user_id, groups, "idp").await
}

/// Provisionnement SCIM : un groupe SCIM est une équipe ; ses membres
/// rejoignent les clients associés à ce nom de groupe (source `scim`, distincte
/// de celle des connexions SSO pour qu'aucune ne défasse l'autre). Recalculé
/// pour chaque compte concerné après toute écriture d'un groupe. Un
/// administrateur n'est jamais touché, comme partout dans SCIM.
pub async fn sync_scim_clients(pool: &SqlitePool) -> Result<()> {
    let users: Vec<i64> = sqlx::query_scalar(
        "SELECT u.id FROM users u
          WHERE u.role <> 'admin'
            AND (EXISTS (SELECT 1 FROM team_members m JOIN teams t ON t.id = m.team_id
                          WHERE m.user_id = u.id AND t.scim_id IS NOT NULL)
                 OR EXISTS (SELECT 1 FROM client_members c
                             WHERE c.user_id = u.id AND c.source = 'scim'))",
    )
    .fetch_all(pool)
    .await
    .context("comptes provisionnés par SCIM")?;
    for user in users {
        let groups: Vec<String> = sqlx::query_scalar(
            "SELECT t.name FROM team_members m JOIN teams t ON t.id = m.team_id
              WHERE m.user_id = ? AND t.scim_id IS NOT NULL",
        )
        .bind(user)
        .fetch_all(pool)
        .await
        .context("groupes SCIM d'un compte")?;
        sync_groups(pool, user, &groups, "scim").await?;
    }
    Ok(())
}

async fn sync_groups(
    pool: &SqlitePool,
    user_id: i64,
    groups: &[String],
    source: &str,
) -> Result<()> {
    let rows = sqlx::query("SELECT id, idp_groups FROM clients WHERE idp_groups <> '[]'")
        .fetch_all(pool)
        .await
        .context("clients associés à des groupes")?;
    let mut tx = pool.begin().await?;
    for row in rows {
        let client_id: i64 = row.try_get("id")?;
        let raw: String = row.try_get("idp_groups")?;
        let wanted: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        if wanted.is_empty() {
            continue;
        }
        let member = groups.iter().any(|g| wanted.iter().any(|w| group_matches(w, g)));
        if member {
            sqlx::query(
                "INSERT INTO client_members (client_id, user_id, source) VALUES (?, ?, ?)
                 ON CONFLICT(client_id, user_id) DO NOTHING",
            )
            .bind(client_id)
            .bind(user_id)
            .bind(source)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "DELETE FROM client_members WHERE client_id = ? AND user_id = ? AND source = ?",
            )
            .bind(client_id)
            .bind(user_id)
            .bind(source)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await.context("synchronisation des clients")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Vue d'ensemble
// ---------------------------------------------------------------------------

/// État d'un client pour la vue d'ensemble de l'administrateur MSP.
#[derive(Debug, Clone, Serialize)]
pub struct ClientHealth {
    pub id: i64,
    pub name: String,
    pub has_logo: bool,
    pub targets: i64,
    pub monitor_quota: Option<i64>,
    /// Équipements avec au moins une alerte active.
    pub alerting_targets: i64,
    /// Alertes actives par sévérité.
    pub critical: i64,
    pub warning: i64,
    pub info: i64,
    pub paused: i64,
}

pub async fn overview(pool: &SqlitePool) -> Result<Vec<ClientHealth>> {
    let rows = sqlx::query(
        "SELECT c.id, c.name, c.logo_type, c.monitor_quota,
                (SELECT COUNT(*) FROM targets t WHERE t.client_id = c.id) AS targets,
                (SELECT COUNT(*) FROM targets t WHERE t.client_id = c.id AND t.enabled = 0)
                    AS paused,
                (SELECT COUNT(DISTINCT s.target_id) FROM alert_state s
                   JOIN targets t ON t.id = s.target_id
                  WHERE t.client_id = c.id AND s.phase = 'firing') AS alerting_targets,
                (SELECT COUNT(*) FROM alert_state s JOIN targets t ON t.id = s.target_id
                  WHERE t.client_id = c.id AND s.phase = 'firing' AND s.severity = 'critical')
                    AS critical,
                (SELECT COUNT(*) FROM alert_state s JOIN targets t ON t.id = s.target_id
                  WHERE t.client_id = c.id AND s.phase = 'firing' AND s.severity = 'warning')
                    AS warning,
                (SELECT COUNT(*) FROM alert_state s JOIN targets t ON t.id = s.target_id
                  WHERE t.client_id = c.id AND s.phase = 'firing' AND s.severity = 'info')
                    AS info
         FROM clients c ORDER BY c.name COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await
    .context("vue d'ensemble des clients")?;
    rows.iter()
        .map(|row| {
            Ok(ClientHealth {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                has_logo: row.try_get::<Option<String>, _>("logo_type")?.is_some(),
                targets: row.try_get("targets")?,
                monitor_quota: row.try_get("monitor_quota")?,
                alerting_targets: row.try_get("alerting_targets")?,
                critical: row.try_get("critical")?,
                warning: row.try_get("warning")?,
                info: row.try_get("info")?,
                paused: row.try_get("paused")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scoped(clients: Option<Vec<i64>>) -> Access {
        Access::Scoped(Arc::new(Scope {
            ceiling: Level::Read,
            grants: vec![Grant { folder: ALL_FOLDERS.to_string(), level: Level::Read }],
            teams: Vec::new(),
            clients,
            admin_view: false,
        }))
    }

    #[test]
    fn the_client_level_comes_before_the_folder() {
        let Access::Scoped(scope) = scoped(Some(vec![2])) else { unreachable!() };
        assert_eq!(scope.level_for_target(2, "Paris"), Some(Level::Read));
        assert_eq!(scope.level_for_target(3, "Paris"), None);
        assert_eq!(scope.level_for_target(3, ""), None);
    }

    #[test]
    fn the_view_header_never_widens_access() {
        // Un compte restreint ne se donne pas un client étranger.
        let narrowed = narrow(scoped(Some(vec![2])), 3, false);
        assert_eq!(narrowed.clients(), Some(&[2][..]));
        // Ni un administrateur hors mode : `admin` faux, rien ne change.
        assert!(narrow(Access::All, 3, false).clients().is_none());
        // Parmi ses clients, il peut se limiter à un seul.
        let narrowed = narrow(scoped(Some(vec![2, 5])), 5, false);
        assert_eq!(narrowed.clients(), Some(&[5][..]));
        assert!(narrowed.is_scoped());
        // La vue d'un administrateur filtre sans restreindre.
        let view = narrow(Access::All, 4, true);
        assert_eq!(view.clients(), Some(&[4][..]));
        assert!(!view.is_scoped());
    }

    #[test]
    fn shared_objects_follow_the_flag() {
        let access = scoped(Some(vec![2]));
        assert!(visible_owner(&access, Some(2), false));
        assert!(!visible_owner(&access, Some(3), true));
        assert!(!visible_owner(&access, None, false));
        assert!(visible_owner(&access, None, true));
        assert!(visible_owner(&Access::All, Some(3), false));
    }

    #[test]
    fn creation_goes_to_the_only_client() {
        assert_eq!(creation_client(&scoped(Some(vec![7]))), Some(7));
        assert_eq!(creation_client(&scoped(Some(vec![7, 8]))), None);
        assert_eq!(creation_client(&Access::All), None);
    }
}
