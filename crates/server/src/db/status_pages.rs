//! Accès aux pages de statut, à leurs services et aux incidents annoncés.
//!
//! Le module ne connaît ni la validation ni la présentation : il lit et écrit
//! les tables, et laisse `api::status_pages` décider de ce qui est acceptable et
//! de ce qui est montré au public.

use anyhow::{Context, Result};
use dumbmonit_proto::TargetId;
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

/// Une page telle qu'enregistrée.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct StatusPage {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub published: bool,
    pub theme: String,
    pub show_uptime_days: i64,
    pub created_at: String,
    pub updated_at: String,
    /// Teinte d'accent, parmi le jeu fermé de `api::status_pages`.
    pub accent: String,
    /// Scènes de la bannière, séparées par des virgules (voir la migration) ;
    /// servies en liste JSON.
    #[serde(serialize_with = "serialize_scenes")]
    pub scenes: String,
    /// Rythme de changement de scène, parmi le jeu fermé de `api::status_pages`.
    pub scene_rotation: String,
    /// Mode simple : ni scène, ni pigeon, ni animation (voir la migration 0047).
    pub simple: bool,
    /// Calendrier public de 90 jours (migration 0050).
    pub show_calendar: bool,
    /// Graphique public de temps de réponse par service (migration 0050).
    pub show_latency: bool,
    pub footer_text: String,
    pub homepage_url: String,
    /// Type MIME du logo déposé ; `None` : pas de logo.
    pub logo_type: Option<String>,
    /// Canal SMTP des abonnés ; `None` : pas d'abonnement par courriel.
    pub subscribe_channel_id: Option<i64>,
    /// Base des liens des courriels, vue par l'administrateur (voir la migration).
    #[serde(skip)]
    pub link_origin: String,
    /// Domaine public : la page est servie à la racine de ce nom d'hôte
    /// (`crate::status_host`) ; `None` : seulement sous `/s/<slug>`.
    pub domain: Option<String>,
    /// Réglages de référencement (migration 0065), servis en objet JSON.
    #[serde(serialize_with = "serialize_seo")]
    pub seo: String,
    /// Mode d'accès (migration 0085) : `public`, `password` ou `users`.
    pub access_mode: String,
    /// Un mot de passe d'accès est enregistré (l'empreinte, elle, ne sort jamais).
    pub has_password: bool,
}

impl StatusPage {
    /// Vrai si tout le monde peut lire la page (aucun mot de passe, aucun compte).
    pub fn is_public(&self) -> bool {
        self.access_mode == "public"
    }
}

/// Scènes de ville de la bannière. Le jeu est fermé : chaque identifiant a son
/// dessin dans `web/src/routes/s/`; validé par `api::status_pages`.
/// Scène d'une page créée sans liste de scènes.
pub const DEFAULT_SCENE: &str = "paris";
pub const SCENES: &[&str] = &[
    "venice",
    "paris",
    "tokyo",
    "newyork",
    "london",
    "rome",
    "sydney",
    "dubai",
    "sanfrancisco",
    "barcelona",
    "amsterdam",
    "istanbul",
    "rio",
    "chichenitza",
    "machupicchu",
    "greatwall",
    "petra",
    "tajmahal",
    "giza",
    "babylon",
    "artemis",
    "zeus",
    "halicarnassus",
    "rhodes",
    "alexandria",
    "athens",
    "geneva",
    "matterhorn",
    "bern",
    "lucerne",
    "chillon",
    "hongkong",
    "bosphorus",
    "sugarloaf",
    "marrakech",
    "kyoto",
    "reykjavik",
    "cairo",
    "orbit",
    "aquarium",
    "lighthouse",
    "mars",
    "chalet",
    "train",
    "zen",
    "arcade",
    "forest",
    "desert",
    "hamsters",
    "cats",
    "raclette",
];
/// Rythmes de changement de scène (plusieurs scènes seulement).
pub const SCENE_ROTATIONS: [&str; 4] = ["visit", "1m", "10m", "1h"];
/// Sépare la liste enregistrée (`venice,paris`) en identifiants de scène.
pub fn split_scenes(csv: &str) -> Vec<String> {
    csv.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

fn serialize_seo<S: serde::Serializer>(json: &str, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_some(&crate::status_seo::Seo::parse(json))
}

fn serialize_scenes<S: serde::Serializer>(csv: &str, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(split_scenes(csv))
}

/// Colonnes lues d'une page, dans l'ordre de [`StatusPage`].
macro_rules! select_page {
    ($tail:literal) => {
        concat!(
            "SELECT id, slug, title, description, published, theme, show_uptime_days, ",
            "created_at, updated_at, accent, scenes, scene_rotation, simple, show_calendar, show_latency, footer_text, homepage_url, ",
            "logo_type, subscribe_channel_id, link_origin, domain, seo, access_mode, ",
            "(access_password_hash IS NOT NULL) AS has_password FROM status_pages ",
            $tail
        )
    };
}

/// Champs modifiables d'une page, à la création comme à la mise à jour.
#[derive(Debug, Clone)]
pub struct StatusPageInput {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub published: bool,
    pub theme: String,
    pub show_uptime_days: i64,
    pub accent: String,
    /// Scènes séparées par des virgules, déjà validées.
    pub scenes: String,
    pub scene_rotation: String,
    pub simple: bool,
    pub show_calendar: bool,
    pub show_latency: bool,
    pub footer_text: String,
    pub homepage_url: String,
    pub subscribe_channel_id: Option<i64>,
    pub link_origin: String,
    /// Déjà normalisé et validé (`status_host::normalise_domain`).
    pub domain: Option<String>,
    /// Déjà validé (`status_seo::Seo::validate`).
    pub seo: crate::status_seo::Seo,
    /// Groupes cochés, dans l'ordre ; `None` : inchangés.
    pub groups: Option<Vec<String>>,
    /// Noms publics des membres de groupes ; `None` : inchangés.
    pub group_labels: Option<Vec<GroupLabel>>,
}

/// Nom public d'une surveillance arrivée par un groupe coché.
#[derive(Debug, Clone, Serialize, serde::Deserialize, FromRow)]
pub struct GroupLabel {
    pub target_id: TargetId,
    pub label: String,
}

/// Un service affiché par une page.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PageItem {
    pub id: i64,
    pub page_id: i64,
    pub target_id: TargetId,
    pub label: String,
    pub group_name: String,
    pub position: i64,
}

#[derive(Debug, Clone)]
pub struct PageItemInput {
    pub target_id: TargetId,
    pub label: String,
    pub group_name: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Incident {
    pub id: i64,
    pub page_id: Option<i64>,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub severity: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct IncidentInput {
    pub page_id: Option<i64>,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub severity: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct IncidentUpdate {
    pub id: i64,
    pub incident_id: i64,
    pub status: String,
    pub body: String,
    pub created_at: String,
}

// --------------------------------------------------------------------------
// Pages
// --------------------------------------------------------------------------

pub async fn list_pages(pool: &SqlitePool) -> Result<Vec<StatusPage>> {
    sqlx::query_as(select_page!("ORDER BY title, id"))
        .fetch_all(pool)
        .await
        .context("liste des pages de statut")
}

pub async fn get_page(pool: &SqlitePool, id: i64) -> Result<Option<StatusPage>> {
    sqlx::query_as(select_page!("WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("lecture d'une page de statut")
}

pub async fn get_page_by_slug(pool: &SqlitePool, slug: &str) -> Result<Option<StatusPage>> {
    sqlx::query_as(select_page!("WHERE slug = ?"))
        .bind(slug)
        .fetch_optional(pool)
        .await
        .context("lecture d'une page de statut par slug")
}

/// Crée une page. Une violation d'unicité du slug remonte telle quelle : c'est à
/// l'appelant de la traduire en conflit.
pub async fn create_page(pool: &SqlitePool, input: &StatusPageInput) -> Result<i64> {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO status_pages (slug, title, description, published, theme, show_uptime_days,
             accent, scenes, scene_rotation, simple, show_calendar, show_latency, footer_text, homepage_url,
             subscribe_channel_id, link_origin, domain, seo)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(&input.slug)
    .bind(&input.title)
    .bind(&input.description)
    .bind(i64::from(input.published))
    .bind(&input.theme)
    .bind(input.show_uptime_days)
    .bind(&input.accent)
    .bind(&input.scenes)
    .bind(&input.scene_rotation)
    .bind(i64::from(input.simple))
    .bind(i64::from(input.show_calendar))
    .bind(i64::from(input.show_latency))
    .bind(&input.footer_text)
    .bind(&input.homepage_url)
    .bind(input.subscribe_channel_id)
    .bind(&input.link_origin)
    .bind(&input.domain)
    .bind(input.seo.to_json())
    .fetch_one(pool)
    .await
    .context("création de la page de statut")?;
    if let Some(groups) = &input.groups {
        set_groups(pool, id, groups).await?;
    }
    if let Some(labels) = &input.group_labels {
        set_group_labels(pool, id, labels).await?;
    }
    Ok(id)
}

/// Empreinte Argon2id du mot de passe d'accès d'une page, si elle en a un.
pub async fn access_password_hash(pool: &SqlitePool, id: i64) -> Result<Option<String>> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT access_password_hash FROM status_pages WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .context("lecture du mot de passe d'accès d'une page de statut")?;
    Ok(row.and_then(|(hash,)| hash))
}

/// Fixe le mode d'accès d'une page. `hash` : `Some(Some(h))` remplace
/// l'empreinte, `Some(None)` l'efface, `None` la laisse.
pub async fn set_access(
    pool: &SqlitePool,
    id: i64,
    mode: &str,
    hash: Option<Option<&str>>,
) -> Result<bool> {
    let result = match hash {
        Some(hash) => sqlx::query(
            "UPDATE status_pages SET access_mode = ?, access_password_hash = ?,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
        )
        .bind(mode)
        .bind(hash)
        .bind(id),
        None => sqlx::query(
            "UPDATE status_pages SET access_mode = ?,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
        )
        .bind(mode)
        .bind(id),
    }
    .execute(pool)
    .await
    .context("mode d'accès de la page de statut")?;
    Ok(result.rows_affected() > 0)
}

/// Date de création du plus ancien appareil montré par la page : l'âge de ses
/// données, qui borne la fenêtre d'historique affichée.
pub async fn oldest_target_created_at(pool: &SqlitePool, page_id: i64) -> Result<Option<String>> {
    let (oldest,): (Option<String>,) = sqlx::query_as(
        "SELECT MIN(created_at) FROM targets
         WHERE id IN (SELECT target_id FROM status_page_items WHERE page_id = ?1)
            OR (group_name != ''
                AND group_name IN (SELECT group_name FROM status_page_groups WHERE page_id = ?1))",
    )
    .bind(page_id)
    .fetch_one(pool)
    .await
    .context("âge des données d'une page de statut")?;
    Ok(oldest)
}

pub async fn update_page(pool: &SqlitePool, id: i64, input: &StatusPageInput) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE status_pages SET
             slug = ?, title = ?, description = ?, published = ?, theme = ?,
             show_uptime_days = ?, accent = ?, scenes = ?, scene_rotation = ?, simple = ?,
             show_calendar = ?, show_latency = ?,
             footer_text = ?, homepage_url = ?, subscribe_channel_id = ?, link_origin = ?,
             domain = ?, seo = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ?",
    )
    .bind(&input.slug)
    .bind(&input.title)
    .bind(&input.description)
    .bind(i64::from(input.published))
    .bind(&input.theme)
    .bind(input.show_uptime_days)
    .bind(&input.accent)
    .bind(&input.scenes)
    .bind(&input.scene_rotation)
    .bind(i64::from(input.simple))
    .bind(i64::from(input.show_calendar))
    .bind(i64::from(input.show_latency))
    .bind(&input.footer_text)
    .bind(&input.homepage_url)
    .bind(input.subscribe_channel_id)
    .bind(&input.link_origin)
    .bind(&input.domain)
    .bind(input.seo.to_json())
    .bind(id)
    .execute(pool)
    .await
    .context("mise à jour de la page de statut")?;
    if result.rows_affected() > 0 {
        if let Some(groups) = &input.groups {
            set_groups(pool, id, groups).await?;
        }
        if let Some(labels) = &input.group_labels {
            set_group_labels(pool, id, labels).await?;
        }
    }
    Ok(result.rows_affected() > 0)
}

/// Domaines publics enregistrés, avec le slug de la page qu'ils servent.
pub async fn list_domains(pool: &SqlitePool) -> Result<Vec<(String, String)>> {
    sqlx::query_as("SELECT domain, slug FROM status_pages WHERE domain IS NOT NULL")
        .fetch_all(pool)
        .await
        .context("liste des domaines des pages de statut")
}

pub async fn delete_page(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM status_pages WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("suppression de la page de statut")?;
    Ok(result.rows_affected() > 0)
}

/// Marque la page comme modifiée — appelé quand ses services changent, pour que
/// le public voie une date de mise à jour honnête.
async fn touch_page(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE status_pages SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await
    .context("horodatage de la page de statut")?;
    Ok(())
}

// --------------------------------------------------------------------------
// Services d'une page
// --------------------------------------------------------------------------

pub async fn list_items(pool: &SqlitePool, page_id: i64) -> Result<Vec<PageItem>> {
    sqlx::query_as(
        "SELECT id, page_id, target_id, label, group_name, position FROM status_page_items WHERE page_id = ? ORDER BY position, id"
    )
    .bind(page_id)
    .fetch_all(pool)
    .await
    .context("liste des services d'une page de statut")
}

/// Services de toutes les pages, pour la liste d'administration.
pub async fn list_all_items(pool: &SqlitePool) -> Result<Vec<PageItem>> {
    sqlx::query_as(
        "SELECT id, page_id, target_id, label, group_name, position FROM status_page_items ORDER BY page_id, position, id"
    )
    .fetch_all(pool)
    .await
    .context("liste des services des pages de statut")
}

// --------------------------------------------------------------------------
// Groupes cochés (dynamiques)
// --------------------------------------------------------------------------

/// Groupes cochés d'une page, dans l'ordre.
pub async fn list_groups(pool: &SqlitePool, page_id: i64) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT group_name FROM status_page_groups WHERE page_id = ? ORDER BY position, group_name",
    )
    .bind(page_id)
    .fetch_all(pool)
    .await
    .context("liste des groupes d'une page de statut")?;
    Ok(rows.into_iter().map(|(name,)| name).collect())
}

/// Groupes cochés de toutes les pages, pour la liste d'administration.
pub async fn list_all_groups(pool: &SqlitePool) -> Result<Vec<(i64, String)>> {
    sqlx::query_as(
        "SELECT page_id, group_name FROM status_page_groups ORDER BY page_id, position, group_name",
    )
    .fetch_all(pool)
    .await
    .context("liste des groupes des pages de statut")
}

/// Remplace les groupes cochés d'une page.
pub async fn set_groups(pool: &SqlitePool, page_id: i64, groups: &[String]) -> Result<()> {
    let mut tx = pool.begin().await.context("ouverture de la transaction")?;
    sqlx::query("DELETE FROM status_page_groups WHERE page_id = ?")
        .bind(page_id)
        .execute(&mut *tx)
        .await
        .context("purge des groupes de la page")?;
    for (position, name) in groups.iter().enumerate() {
        sqlx::query(
            "INSERT OR IGNORE INTO status_page_groups (page_id, group_name, position)
             VALUES (?, ?, ?)",
        )
        .bind(page_id)
        .bind(name)
        .bind(position as i64)
        .execute(&mut *tx)
        .await
        .context("insertion d'un groupe de page")?;
    }
    tx.commit().await.context("validation des groupes de la page")?;
    Ok(())
}

/// Noms publics des membres de groupes d'une page.
pub async fn list_group_labels(pool: &SqlitePool, page_id: i64) -> Result<Vec<GroupLabel>> {
    sqlx::query_as(
        "SELECT target_id, label FROM status_page_group_labels WHERE page_id = ? ORDER BY target_id",
    )
    .bind(page_id)
    .fetch_all(pool)
    .await
    .context("noms publics des membres de groupes")
}

/// Noms publics des membres de groupes de toutes les pages.
pub async fn list_all_group_labels(pool: &SqlitePool) -> Result<Vec<(i64, GroupLabel)>> {
    let rows: Vec<(i64, TargetId, String)> = sqlx::query_as(
        "SELECT page_id, target_id, label FROM status_page_group_labels ORDER BY page_id, target_id",
    )
    .fetch_all(pool)
    .await
    .context("noms publics des membres de groupes")?;
    Ok(rows
        .into_iter()
        .map(|(page, target_id, label)| (page, GroupLabel { target_id, label }))
        .collect())
}

/// Remplace les noms publics des membres de groupes ; une surveillance qui
/// n'existe plus est ignorée.
pub async fn set_group_labels(
    pool: &SqlitePool,
    page_id: i64,
    labels: &[GroupLabel],
) -> Result<()> {
    let mut tx = pool.begin().await.context("ouverture de la transaction")?;
    sqlx::query("DELETE FROM status_page_group_labels WHERE page_id = ?")
        .bind(page_id)
        .execute(&mut *tx)
        .await
        .context("purge des noms publics")?;
    for entry in labels {
        sqlx::query(
            "INSERT OR REPLACE INTO status_page_group_labels (page_id, target_id, label)
             SELECT ?, id, ? FROM targets WHERE id = ?",
        )
        .bind(page_id)
        .bind(&entry.label)
        .bind(entry.target_id)
        .execute(&mut *tx)
        .await
        .context("insertion d'un nom public")?;
    }
    tx.commit().await.context("validation des noms publics")?;
    Ok(())
}

/// Une surveillance membre d'un groupe coché.
#[derive(Debug, Clone, FromRow)]
pub struct GroupMember {
    pub target_id: TargetId,
    pub name: String,
    pub group_name: String,
    /// Nom public choisi pour cette page ; vide : le nom de la surveillance.
    pub public_label: String,
}

/// Services effectivement montrés : ceux choisis un à un, puis les membres
/// *actuels* des groupes cochés. Lu à chaque fois : une surveillance ajoutée
/// au groupe apparaît, une retirée disparaît.
pub async fn effective_items(pool: &SqlitePool, page_id: i64) -> Result<Vec<PageItem>> {
    let explicit = list_items(pool, page_id).await?;
    let groups = list_groups(pool, page_id).await?;
    if groups.is_empty() {
        return Ok(explicit);
    }
    let members: Vec<GroupMember> = sqlx::query_as(
        "SELECT t.id AS target_id, t.name, t.group_name,
                COALESCE(l.label, '') AS public_label
         FROM targets t
         LEFT JOIN status_page_group_labels l ON l.page_id = ?1 AND l.target_id = t.id
         WHERE t.group_name != ''
           AND t.group_name IN (SELECT group_name FROM status_page_groups WHERE page_id = ?1)
         ORDER BY t.position, t.id",
    )
    .bind(page_id)
    .fetch_all(pool)
    .await
    .context("membres des groupes d'une page de statut")?;
    Ok(merge_items(page_id, explicit, &groups, &members))
}

/// Fusionne services choisis et membres de groupes, sans doublon.
///
/// Un service choisi un à un garde son libellé ; s'il n'a pas de groupe à lui
/// et appartient à un groupe coché, il s'affiche sous ce groupe. Les membres
/// viennent ensuite, groupe par groupe dans l'ordre des cases cochées.
pub fn merge_items(
    page_id: i64,
    explicit: Vec<PageItem>,
    groups: &[String],
    members: &[GroupMember],
) -> Vec<PageItem> {
    let folder_of: std::collections::HashMap<TargetId, &str> =
        members.iter().map(|member| (member.target_id, member.group_name.as_str())).collect();
    let mut seen: std::collections::HashSet<TargetId> = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(explicit.len() + members.len());
    for mut item in explicit {
        if !seen.insert(item.target_id) {
            continue;
        }
        if item.group_name.is_empty()
            && let Some(folder) = folder_of.get(&item.target_id)
        {
            item.group_name = (*folder).to_string();
        }
        out.push(item);
    }
    for group in groups {
        for member in members.iter().filter(|member| &member.group_name == group) {
            if !seen.insert(member.target_id) {
                continue;
            }
            out.push(PageItem {
                id: 0,
                page_id,
                target_id: member.target_id,
                label: if member.public_label.trim().is_empty() {
                    member.name.clone()
                } else {
                    member.public_label.trim().to_string()
                },
                group_name: member.group_name.clone(),
                position: out.len() as i64,
            });
        }
    }
    out
}

/// Remplace d'un bloc les services d'une page, dans l'ordre donné.
///
/// Tout ou rien : une cible inconnue fait échouer la transaction entière, et la
/// page garde alors sa liste précédente.
pub async fn set_items(pool: &SqlitePool, page_id: i64, items: &[PageItemInput]) -> Result<()> {
    let mut tx = pool.begin().await.context("ouverture de la transaction")?;
    sqlx::query("DELETE FROM status_page_items WHERE page_id = ?")
        .bind(page_id)
        .execute(&mut *tx)
        .await
        .context("purge des services de la page")?;
    for (position, item) in items.iter().enumerate() {
        sqlx::query(
            "INSERT INTO status_page_items (page_id, target_id, label, group_name, position)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(page_id)
        .bind(item.target_id)
        .bind(&item.label)
        .bind(&item.group_name)
        .bind(position as i64)
        .execute(&mut *tx)
        .await
        .context("insertion d'un service de page")?;
    }
    sqlx::query(
        "UPDATE status_pages SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(page_id)
    .execute(&mut *tx)
    .await
    .context("horodatage de la page de statut")?;
    tx.commit().await.context("validation des services de la page")?;
    Ok(())
}

// --------------------------------------------------------------------------
// Incidents
// --------------------------------------------------------------------------

pub async fn list_incidents(pool: &SqlitePool) -> Result<Vec<Incident>> {
    sqlx::query_as(
        "SELECT id, page_id, title, kind, status, severity, starts_at, ends_at, created_at, updated_at FROM incidents ORDER BY starts_at DESC, id DESC"
    )
    .fetch_all(pool)
    .await
    .context("liste des incidents")
}

/// Incidents visibles par une page : les siens et ceux qui valent pour toutes,
/// ouverts ou terminés depuis `since` (horodatage au format de la base).
pub async fn incidents_for_page(
    pool: &SqlitePool,
    page_id: i64,
    since: &str,
) -> Result<Vec<Incident>> {
    sqlx::query_as(
        "SELECT id, page_id, title, kind, status, severity, starts_at, ends_at, created_at, updated_at FROM incidents
         WHERE (page_id IS NULL OR page_id = ?)
           AND (ends_at IS NULL OR ends_at >= ? OR starts_at >= ?)
         ORDER BY starts_at DESC, id DESC"
    )
    .bind(page_id)
    .bind(since)
    .bind(since)
    .fetch_all(pool)
    .await
    .context("incidents d'une page de statut")
}

pub async fn get_incident(pool: &SqlitePool, id: i64) -> Result<Option<Incident>> {
    sqlx::query_as("SELECT id, page_id, title, kind, status, severity, starts_at, ends_at, created_at, updated_at FROM incidents WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("lecture d'un incident")
}

pub async fn create_incident(pool: &SqlitePool, input: &IncidentInput) -> Result<i64> {
    create_incident_with_thread(pool, input, &[]).await
}

/// Un message du fil d'une annonce créée d'un bloc.
#[derive(Debug, Clone)]
pub struct ThreadEntry {
    pub status: String,
    pub body: String,
    /// Date du message, format de la base.
    pub at: String,
}

/// Crée une annonce et son fil dans une même transaction : une saisie
/// d'historique est enregistrée entière ou pas du tout.
pub async fn create_incident_with_thread(
    pool: &SqlitePool,
    input: &IncidentInput,
    thread: &[ThreadEntry],
) -> Result<i64> {
    let mut tx = pool.begin().await.context("ouverture de la transaction")?;
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO incidents (page_id, title, kind, status, severity, starts_at, ends_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(input.page_id)
    .bind(&input.title)
    .bind(&input.kind)
    .bind(&input.status)
    .bind(&input.severity)
    .bind(&input.starts_at)
    .bind(&input.ends_at)
    .fetch_one(&mut *tx)
    .await
    .context("création de l'incident")?;
    for entry in thread {
        sqlx::query(
            "INSERT INTO incident_updates (incident_id, status, body, created_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&entry.status)
        .bind(&entry.body)
        .bind(&entry.at)
        .execute(&mut *tx)
        .await
        .context("ajout au fil de l'incident")?;
    }
    tx.commit().await.context("validation de l'incident")?;
    if let Some(page_id) = input.page_id {
        touch_page(pool, page_id).await?;
    }
    Ok(id)
}

pub async fn update_incident(pool: &SqlitePool, id: i64, input: &IncidentInput) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE incidents SET
             page_id = ?, title = ?, kind = ?, status = ?, severity = ?, starts_at = ?,
             ends_at = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ?",
    )
    .bind(input.page_id)
    .bind(&input.title)
    .bind(&input.kind)
    .bind(&input.status)
    .bind(&input.severity)
    .bind(&input.starts_at)
    .bind(&input.ends_at)
    .bind(id)
    .execute(pool)
    .await
    .context("mise à jour de l'incident")?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_incident(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM incidents WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("suppression de l'incident")?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_updates(pool: &SqlitePool, incident_id: i64) -> Result<Vec<IncidentUpdate>> {
    sqlx::query_as(
        "SELECT id, incident_id, status, body, created_at FROM incident_updates
         WHERE incident_id = ? ORDER BY created_at, id",
    )
    .bind(incident_id)
    .fetch_all(pool)
    .await
    .context("fil d'un incident")
}

/// Fils de tous les incidents donnés.
///
/// Les incidents visibles sur une page se comptent en dizaines au plus : une
/// requête par incident reste dérisoire, et évite d'assembler du SQL à la volée.
pub async fn list_updates_for(
    pool: &SqlitePool,
    incident_ids: &[i64],
) -> Result<Vec<IncidentUpdate>> {
    let mut all = Vec::new();
    for &id in incident_ids {
        all.extend(list_updates(pool, id).await?);
    }
    Ok(all)
}

/// Ajoute un message au fil et fait passer l'incident au statut correspondant.
///
/// `ends_at` n'est posé que pour une fin (résolu, terminé) et seulement s'il
/// était vide : une maintenance garde sa fin prévue.
pub async fn add_update(
    pool: &SqlitePool,
    incident_id: i64,
    status: &str,
    body: &str,
    closes: bool,
) -> Result<IncidentUpdate> {
    add_update_at(pool, incident_id, status, body, closes, None).await
}

/// Comme [`add_update`], à une date donnée (format de la base) : saisie d'un
/// historique. Le statut de l'annonce suit alors le message *le plus récent* du
/// fil, pas le dernier saisi ; la fin, si l'annonce n'en avait pas, est la date
/// du message de clôture.
pub async fn add_update_at(
    pool: &SqlitePool,
    incident_id: i64,
    status: &str,
    body: &str,
    closes: bool,
    at: Option<&str>,
) -> Result<IncidentUpdate> {
    let mut tx = pool.begin().await.context("ouverture de la transaction")?;
    let update: IncidentUpdate = sqlx::query_as(
        "INSERT INTO incident_updates (incident_id, status, body, created_at)
         VALUES (?, ?, ?, COALESCE(?, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')))
         RETURNING id, incident_id, status, body, created_at",
    )
    .bind(incident_id)
    .bind(status)
    .bind(body)
    .bind(at)
    .fetch_one(&mut *tx)
    .await
    .context("ajout au fil de l'incident")?;
    sqlx::query(
        "UPDATE incidents SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
             status = (SELECT status FROM incident_updates WHERE incident_id = ?1
                       ORDER BY created_at DESC, id DESC LIMIT 1)
         WHERE id = ?1",
    )
    .bind(incident_id)
    .execute(&mut *tx)
    .await
    .context("changement de statut de l'incident")?;
    if closes {
        sqlx::query("UPDATE incidents SET ends_at = COALESCE(ends_at, ?2) WHERE id = ?1")
            .bind(incident_id)
            .bind(&update.created_at)
            .execute(&mut *tx)
            .await
            .context("fin de l'incident")?;
    }
    tx.commit().await.context("validation du fil de l'incident")?;
    Ok(update)
}

// --------------------------------------------------------------------------
// Logo
// --------------------------------------------------------------------------

/// Enregistre (ou efface, avec `None`) le type du logo et date la page : le
/// document public porte cette date, qui sert aussi à invalider le cache du logo.
pub async fn set_logo_type(pool: &SqlitePool, id: i64, logo_type: Option<&str>) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE status_pages SET logo_type = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ?",
    )
    .bind(logo_type)
    .bind(id)
    .execute(pool)
    .await
    .context("logo de la page de statut")?;
    Ok(result.rows_affected() > 0)
}

// --------------------------------------------------------------------------
// Abonnés
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Subscriber {
    pub id: i64,
    pub page_id: i64,
    pub email: String,
    /// Jamais renvoyé par l'API : il vaut désabonnement.
    #[serde(skip)]
    pub token: String,
    pub confirmed_at: Option<String>,
    pub created_at: String,
}

/// Colonnes lues d'un abonné, dans l'ordre de [`Subscriber`].
macro_rules! select_subscriber {
    ($tail:literal) => {
        concat!(
            "SELECT id, page_id, email, token, confirmed_at, created_at FROM status_subscribers ",
            $tail
        )
    };
}

pub async fn list_subscribers(pool: &SqlitePool, page_id: i64) -> Result<Vec<Subscriber>> {
    sqlx::query_as(select_subscriber!("WHERE page_id = ? ORDER BY email"))
        .bind(page_id)
        .fetch_all(pool)
        .await
        .context("liste des abonnés")
}

/// Abonnés confirmés d'une page : ceux qui reçoivent les annonces.
pub async fn confirmed_subscribers(pool: &SqlitePool, page_id: i64) -> Result<Vec<Subscriber>> {
    sqlx::query_as(select_subscriber!("WHERE page_id = ? AND confirmed_at IS NOT NULL ORDER BY id"))
        .bind(page_id)
        .fetch_all(pool)
        .await
        .context("abonnés confirmés")
}

pub async fn count_subscribers(pool: &SqlitePool, page_id: i64) -> Result<i64> {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM status_subscribers WHERE page_id = ?")
            .bind(page_id)
            .fetch_one(pool)
            .await
            .context("compte des abonnés")?;
    Ok(count)
}

pub async fn find_subscriber(
    pool: &SqlitePool,
    page_id: i64,
    email: &str,
) -> Result<Option<Subscriber>> {
    sqlx::query_as(select_subscriber!("WHERE page_id = ? AND email = ?"))
        .bind(page_id)
        .bind(email)
        .fetch_optional(pool)
        .await
        .context("recherche d'un abonné")
}

pub async fn subscriber_by_token(
    pool: &SqlitePool,
    page_id: i64,
    token: &str,
) -> Result<Option<Subscriber>> {
    sqlx::query_as(select_subscriber!("WHERE page_id = ? AND token = ?"))
        .bind(page_id)
        .bind(token)
        .fetch_optional(pool)
        .await
        .context("abonné par jeton")
}

/// Crée un abonné non confirmé, ou renouvelle le jeton et la date d'une demande
/// restée sans confirmation.
pub async fn upsert_pending_subscriber(
    pool: &SqlitePool,
    page_id: i64,
    email: &str,
    token: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO status_subscribers (page_id, email, token) VALUES (?, ?, ?)
         ON CONFLICT (page_id, email) DO UPDATE SET
             token = excluded.token,
             created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE status_subscribers.confirmed_at IS NULL",
    )
    .bind(page_id)
    .bind(email)
    .bind(token)
    .execute(pool)
    .await
    .context("enregistrement d'un abonné")?;
    Ok(())
}

pub async fn confirm_subscriber(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE status_subscribers
         SET confirmed_at = COALESCE(confirmed_at, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
         WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await
    .context("confirmation d'un abonné")?;
    Ok(())
}

pub async fn delete_subscriber(pool: &SqlitePool, page_id: i64, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM status_subscribers WHERE page_id = ? AND id = ?")
        .bind(page_id)
        .bind(id)
        .execute(pool)
        .await
        .context("suppression d'un abonné")?;
    Ok(result.rows_affected() > 0)
}

/// Oublie les demandes jamais confirmées plus anciennes que `before` : une
/// adresse saisie par un tiers ne reste pas en base.
pub async fn purge_unconfirmed(pool: &SqlitePool, before: &str) -> Result<u64> {
    let result =
        sqlx::query("DELETE FROM status_subscribers WHERE confirmed_at IS NULL AND created_at < ?")
            .bind(before)
            .execute(pool)
            .await
            .context("purge des abonnés non confirmés")?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(target_id: TargetId, label: &str, group: &str) -> PageItem {
        PageItem {
            id: 1,
            page_id: 7,
            target_id,
            label: label.into(),
            group_name: group.into(),
            position: 0,
        }
    }

    fn member(target_id: TargetId, name: &str, group: &str) -> GroupMember {
        GroupMember {
            target_id,
            name: name.into(),
            group_name: group.into(),
            public_label: String::new(),
        }
    }

    #[test]
    fn group_members_follow_the_group_and_are_not_duplicated() {
        let groups = vec!["Rack".to_string()];
        let members = vec![member(1, "NAS", "Rack"), member(2, "Switch", "Rack")];
        // Le NAS est aussi coché seul, avec son propre libellé.
        let merged = merge_items(7, vec![item(1, "Storage", "")], &groups, &members);
        assert_eq!(merged.len(), 2, "une seule ligne par surveillance");
        assert_eq!((merged[0].target_id, merged[0].label.as_str()), (1, "Storage"));
        assert_eq!(merged[0].group_name, "Rack", "il s'affiche sous le groupe");
        assert_eq!((merged[1].target_id, merged[1].label.as_str()), (2, "Switch"));
        assert_eq!(merged[1].group_name, "Rack");
    }

    #[test]
    fn a_group_member_shows_its_public_name_not_the_private_one() {
        let groups = vec!["Rack".to_string()];
        let mut named = member(1, "SRV-CRM01", "Rack");
        named.public_label = " Customer portal ".into();
        let merged = merge_items(7, vec![], &groups, &[named, member(2, "SRV-DB02", "Rack")]);
        assert_eq!(merged[0].label, "Customer portal");
        assert_eq!(merged[1].label, "SRV-DB02", "no public name: the monitor name");
    }

    #[test]
    fn an_own_group_name_is_kept_and_a_removed_member_disappears() {
        let groups = vec!["Rack".to_string()];
        let members = vec![member(1, "NAS", "Rack")];
        let merged = merge_items(7, vec![item(1, "NAS", "Storage")], &groups, &members);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].group_name, "Storage");
        // Le groupe n'a plus de membres : seul le choix individuel reste.
        assert_eq!(merge_items(7, vec![item(1, "NAS", "")], &groups, &[]).len(), 1);
        assert!(merge_items(7, vec![], &groups, &[]).is_empty());
    }
}
