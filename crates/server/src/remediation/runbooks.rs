//! Procédures : stockage, rattachements et rapprochement avec une alerte.
//!
//! Le corps est du Markdown brut. Le serveur ne le rend jamais en HTML : c'est
//! l'interface qui l'affiche, en échappant tout balisage (`web/src/lib/markdown.ts`).
//! Les notifications n'en portent que le titre et un lien.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::alerting::notify_policy::Digest;
use crate::notify::Message;

/// Longueur maximale d'un titre.
pub const MAX_TITLE_LEN: usize = 200;

/// Taille maximale d'un corps, en octets.
pub const MAX_BODY_LEN: usize = 64 * 1024;

/// Nombre maximal de rattachements d'une procédure.
pub const MAX_LINKS: usize = 50;

/// Nombre maximal de procédures citées dans une notification.
const MAX_IN_MESSAGE: usize = 3;

/// Un rattachement : une règle (par uid), un dossier ou un type d'équipement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RunbookLink {
    pub scope: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Runbook {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub links: Vec<RunbookLink>,
    pub created_at: String,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

/// Ce qu'on enregistre d'une procédure.
#[derive(Debug, Clone, Deserialize)]
pub struct RunbookDraft {
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub links: Vec<RunbookLink>,
}

impl RunbookDraft {
    /// Valide et normalise : titre non vide et borné, corps borné, liaisons
    /// connues, dédoublonnées.
    pub fn validate(mut self) -> Result<Self, String> {
        self.title = self.title.trim().to_string();
        if self.title.is_empty() || self.title.chars().count() > MAX_TITLE_LEN {
            return Err(format!("The title is required ({MAX_TITLE_LEN} characters max)."));
        }
        if self.title.chars().any(char::is_control) {
            return Err("The title cannot contain control characters.".to_string());
        }
        if self.body.len() > MAX_BODY_LEN {
            return Err(format!("The text is too long ({} KiB max).", MAX_BODY_LEN / 1024));
        }
        if self.links.len() > MAX_LINKS {
            return Err(format!("Too many links ({MAX_LINKS} max)."));
        }
        for link in &mut self.links {
            link.value = link.value.trim().to_string();
            if !matches!(link.scope.as_str(), "rule" | "group" | "kind") {
                return Err(format!("Unknown link scope '{}'.", link.scope));
            }
            if link.value.is_empty() || link.value.chars().count() > 200 {
                return Err("A link needs a value (200 characters max).".to_string());
            }
        }
        self.links.sort();
        self.links.dedup();
        Ok(self)
    }
}

async fn links_of(pool: &SqlitePool, id: i64) -> Result<Vec<RunbookLink>> {
    let rows = sqlx::query(
        "SELECT scope, value FROM runbook_links WHERE runbook_id = ? ORDER BY scope, value",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .context("reading runbook links")?;
    rows.iter()
        .map(|row| Ok(RunbookLink { scope: row.try_get("scope")?, value: row.try_get("value")? }))
        .collect()
}

fn row_to_runbook(row: &sqlx::sqlite::SqliteRow) -> Result<Runbook> {
    Ok(Runbook {
        id: row.try_get("id")?,
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        links: Vec::new(),
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        updated_by: row.try_get("updated_by")?,
    })
}

const SELECT: &str = "SELECT id, title, body, created_at, updated_at, updated_by FROM runbooks";

pub async fn list(pool: &SqlitePool) -> Result<Vec<Runbook>> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!("{SELECT} ORDER BY title COLLATE NOCASE")))
        .fetch_all(pool)
        .await
        .context("listing runbooks")?;
    let mut runbooks = rows.iter().map(row_to_runbook).collect::<Result<Vec<_>>>()?;
    for runbook in &mut runbooks {
        runbook.links = links_of(pool, runbook.id).await?;
    }
    Ok(runbooks)
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<Runbook>> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("{SELECT} WHERE id = ?")))
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("reading the runbook")?;
    let Some(row) = row else { return Ok(None) };
    let mut runbook = row_to_runbook(&row)?;
    runbook.links = links_of(pool, id).await?;
    Ok(Some(runbook))
}

async fn write_links(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: i64,
    links: &[RunbookLink],
) -> Result<()> {
    sqlx::query("DELETE FROM runbook_links WHERE runbook_id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await
        .context("clearing runbook links")?;
    for link in links {
        sqlx::query("INSERT INTO runbook_links (runbook_id, scope, value) VALUES (?, ?, ?)")
            .bind(id)
            .bind(&link.scope)
            .bind(&link.value)
            .execute(&mut **tx)
            .await
            .context("writing a runbook link")?;
    }
    Ok(())
}

pub async fn create(pool: &SqlitePool, draft: &RunbookDraft, by: &str) -> Result<i64> {
    let mut tx = pool.begin().await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO runbooks (title, body, updated_by) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(&draft.title)
    .bind(&draft.body)
    .bind(by)
    .fetch_one(&mut *tx)
    .await
    .context("creating the runbook")?;
    write_links(&mut tx, id, &draft.links).await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn update(pool: &SqlitePool, id: i64, draft: &RunbookDraft, by: &str) -> Result<bool> {
    let mut tx = pool.begin().await?;
    let result = sqlx::query(
        "UPDATE runbooks SET title = ?, body = ?, updated_by = ?,
             updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now')
         WHERE id = ?",
    )
    .bind(&draft.title)
    .bind(&draft.body)
    .bind(by)
    .bind(id)
    .execute(&mut *tx)
    .await
    .context("updating the runbook")?;
    if result.rows_affected() == 0 {
        return Ok(false);
    }
    write_links(&mut tx, id, &draft.links).await?;
    tx.commit().await?;
    Ok(true)
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM runbooks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("deleting the runbook")?;
    Ok(result.rows_affected() > 0)
}

/// Ce qui désigne un contexte d'alerte ou d'équipement.
#[derive(Debug, Clone, Default)]
pub struct MatchContext {
    pub rule_uid: Option<String>,
    pub group: Option<String>,
    pub kind: Option<String>,
}

impl MatchContext {
    /// Dossier et type d'un équipement, lus en base.
    pub async fn for_target(pool: &SqlitePool, target_id: Option<i64>) -> Result<Self> {
        let Some(id) = target_id else { return Ok(Self::default()) };
        let row = sqlx::query("SELECT group_name, kind FROM targets WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .context("reading the target for runbooks")?;
        Ok(match row {
            Some(row) => Self {
                rule_uid: None,
                group: Some(row.try_get::<String, _>("group_name")?).filter(|g| !g.is_empty()),
                kind: Some(row.try_get("kind")?),
            },
            None => Self::default(),
        })
    }
}

/// Procédures qui s'appliquent à ce contexte, sans doublon, par titre.
pub async fn matching(pool: &SqlitePool, context: &MatchContext) -> Result<Vec<Runbook>> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT DISTINCT r.id FROM runbooks r JOIN runbook_links l ON l.runbook_id = r.id
         WHERE (l.scope = 'rule' AND l.value = ?)
            OR (l.scope = 'group' AND l.value = ?)
            OR (l.scope = 'kind' AND l.value = ?)
         ORDER BY r.title COLLATE NOCASE",
    )
    .bind(&context.rule_uid)
    .bind(&context.group)
    .bind(&context.kind)
    .fetch_all(pool)
    .await
    .context("matching runbooks")?;
    let mut runbooks = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(runbook) = get(pool, id).await? {
            runbooks.push(runbook);
        }
    }
    Ok(runbooks)
}

/// Ajoute aux notifications un lien vers les procédures des alertes qui
/// sonnent (jamais pour une résolution). Le titre est débarrassé de ce qui
/// casserait un lien Markdown ; le corps n'est jamais recopié.
pub async fn attach_to_message(
    pool: &SqlitePool,
    message: &mut Message,
    digest: &Digest,
    base_url: &str,
) -> Result<()> {
    let base = base_url.trim_end_matches('/');
    let mut seen = Vec::new();
    for group in &digest.groups {
        let target = MatchContext::for_target(pool, group.target_id).await?;
        for item in group.items.iter().filter(|item| item.reason.is_firing_like()) {
            let context = MatchContext { rule_uid: Some(item.rule_uid.clone()), ..target.clone() };
            for runbook in matching(pool, &context).await? {
                if seen.len() >= MAX_IN_MESSAGE {
                    break;
                }
                if !seen.contains(&runbook.id) {
                    seen.push(runbook.id);
                    let title: String = runbook
                        .title
                        .chars()
                        .filter(|c| !matches!(c, '[' | ']' | '(' | ')' | '<' | '>' | '\n'))
                        .collect();
                    let link = format!("{base}/runbooks/{}", runbook.id);
                    message.text.push_str(&format!("\nRunbook: {title}: {link}"));
                    message.markdown.push_str(&format!("\n[Runbook: {title}]({link})"));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(title: &str, links: Vec<(&str, &str)>) -> RunbookDraft {
        RunbookDraft {
            title: title.into(),
            body: "1. Check the disk".into(),
            links: links
                .into_iter()
                .map(|(scope, value)| RunbookLink { scope: scope.into(), value: value.into() })
                .collect(),
        }
    }

    #[test]
    fn drafts_are_bounded_and_scopes_closed() {
        assert!(draft("  ", vec![]).validate().is_err());
        assert!(draft("ok", vec![("target", "1")]).validate().is_err());
        assert!(draft("ok", vec![("rule", " ")]).validate().is_err());
        let mut long = draft("ok", vec![]);
        long.body = "x".repeat(MAX_BODY_LEN + 1);
        assert!(long.validate().is_err());
        let ok = draft("Disk full", vec![("rule", "disk_full"), ("rule", "disk_full")])
            .validate()
            .unwrap();
        assert_eq!(ok.links.len(), 1, "duplicates removed");
    }
}
